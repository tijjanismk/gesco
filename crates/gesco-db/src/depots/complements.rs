//! Compléments pour les rapports (SPEC_PHASE_9 §8.2, §8.3) : année de recrutement de l'élève,
//! prise en charge par l'État, fonctionnement et pédagogie de la classe, difficultés de l'élève,
//! statut, spécialité et diplôme le plus élevé du personnel. Tout est facultatif.

use std::collections::HashMap;

use gesco_core::commun::Id;
use gesco_core::core::complements::{DegreDifficulte, DifficulteEleve, TypeDifficulte};
use gesco_core::scolarite::{Classe, Inscription};
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::erreur::{ErreurBase, Resultat};
use crate::id::nouvel_id;

/// Un élève d'une classe, pour la grille des compléments.
#[derive(Debug, Clone, FromRow)]
pub struct EleveDeClasse {
    /// Inscription.
    pub inscription_id: String,
    /// Élève.
    pub eleve_id: String,
    /// Nom.
    pub nom: String,
    /// Prénoms.
    pub prenoms: String,
    /// Année de recrutement saisie.
    pub annee_recrutement: Option<i64>,
    /// Pris en charge par l'État cette année.
    pub prise_en_charge_etat: i64,
}

/// Élèves inscrits dans une classe, brouillons exceptés, par nom.
pub async fn eleves_de_la_classe(
    cnx: &mut Connexion,
    classe_id: Id<Classe>,
    contexte: &Contexte,
) -> Resultat<Vec<EleveDeClasse>> {
    Ok(sqlx::query_as::<_, EleveDeClasse>(
        "SELECT i.id AS inscription_id, e.id AS eleve_id, p.nom, p.prenoms, e.annee_recrutement,
                i.prise_en_charge_etat
         FROM inscription i
         JOIN eleve e ON e.id = i.eleve_id
         JOIN personne p ON p.id = e.personne_id
         WHERE i.classe_id = ?1 AND i.etablissement_id = ?2 AND i.statut <> 'brouillon'
           AND e.supprime_le IS NULL AND e.fusionne_dans IS NULL
         ORDER BY p.nom, p.prenoms",
    )
    .bind(classe_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?)
}

/// Première inscription connue d'un élève dans l'école : son type, le code de son niveau, le
/// premier jour de son année (millisecondes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PremiereInscription {
    /// `nouvelle`, `reinscription`, `transfert_entrant`.
    pub type_inscription: String,
    /// Code du niveau (`1` pour la 1re année).
    pub code_niveau: String,
    /// Premier jour de l'année.
    pub debut: i64,
}

/// Première inscription de chaque élève de l'école, brouillons exceptés.
pub async fn premieres_inscriptions(
    cnx: &mut Connexion,
    contexte: &Contexte,
) -> Resultat<HashMap<String, PremiereInscription>> {
    let lignes: Vec<(String, String, String, i64)> = sqlx::query_as(
        "SELECT i.eleve_id, i.type, n.code, a.debut
         FROM inscription i
         JOIN classe c ON c.id = i.classe_id
         JOIN niveau n ON n.id = c.niveau_id
         JOIN annee_scolaire a ON a.id = i.annee_id
         WHERE i.etablissement_id = ?1 AND i.statut <> 'brouillon'
         ORDER BY a.debut DESC, i.date_inscription DESC",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?;
    // Du plus récent au plus ancien : la dernière écriture est la première inscription.
    let mut par_eleve = HashMap::new();
    for (eleve_id, type_inscription, code_niveau, debut) in lignes {
        par_eleve.insert(
            eleve_id,
            PremiereInscription {
                type_inscription,
                code_niveau,
                debut,
            },
        );
    }
    Ok(par_eleve)
}

/// Années de recrutement saisies, par élève.
pub async fn annees_recrutement(
    cnx: &mut Connexion,
    contexte: &Contexte,
) -> Resultat<HashMap<String, i64>> {
    let lignes: Vec<(String, i64)> = sqlx::query_as(
        "SELECT id, annee_recrutement FROM eleve
         WHERE etablissement_id = ?1 AND annee_recrutement IS NOT NULL",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?;
    Ok(lignes.into_iter().collect())
}

/// Écrit l'année de recrutement d'un élève.
pub async fn ecrire_annee_recrutement(
    cnx: &mut Connexion,
    eleve_id: &str,
    annee: Option<i64>,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE eleve SET annee_recrutement = ?1, mis_a_jour_le = ?2, mis_a_jour_par = ?3,
            identifiant_poste = ?4 WHERE id = ?5 AND etablissement_id = ?6",
    )
    .bind(annee)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(eleve_id)
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Inscriptions de l'année prises en charge par l'État.
pub async fn prises_en_charge(
    cnx: &mut Connexion,
    annee_id: &str,
    contexte: &Contexte,
) -> Resultat<Vec<String>> {
    let lignes: Vec<(String,)> = sqlx::query_as(
        "SELECT eleve_id FROM inscription
         WHERE annee_id = ?1 AND etablissement_id = ?2 AND prise_en_charge_etat = 1",
    )
    .bind(annee_id)
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?;
    Ok(lignes.into_iter().map(|(e,)| e).collect())
}

/// Écrit la prise en charge par l'État d'une inscription.
pub async fn ecrire_prise_en_charge(
    cnx: &mut Connexion,
    inscription_id: Id<Inscription>,
    prise: bool,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE inscription SET prise_en_charge_etat = ?1, mis_a_jour_le = ?2,
            mis_a_jour_par = ?3, identifiant_poste = ?4 WHERE id = ?5 AND etablissement_id = ?6",
    )
    .bind(i64::from(prise))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(inscription_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Élève et classe d'une inscription.
pub async fn inscription(
    cnx: &mut Connexion,
    inscription_id: Id<Inscription>,
    contexte: &Contexte,
) -> Resultat<(String, String, Option<i64>, bool)> {
    let ligne: Option<(String, String, Option<i64>, i64)> = sqlx::query_as(
        "SELECT i.eleve_id, i.classe_id, e.annee_recrutement, i.prise_en_charge_etat
         FROM inscription i JOIN eleve e ON e.id = i.eleve_id
         WHERE i.id = ?1 AND i.etablissement_id = ?2",
    )
    .bind(inscription_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?;
    let (e, c, a, p) = ligne.ok_or_else(|| ErreurBase::Introuvable {
        entite: "inscription",
        id: inscription_id.vers_chaine(),
    })?;
    Ok((e, c, a, p != 0))
}

/// Compléments d'une classe : libellé, cycle, fonctionnement, pédagogie.
#[derive(Debug, Clone, FromRow)]
pub struct ClasseComplements {
    /// Classe.
    pub id: String,
    /// Libellé.
    pub libelle: String,
    /// Cycle du niveau.
    pub cycle: String,
    /// Fonctionnement saisi.
    pub fonctionnement: Option<String>,
    /// Pédagogie saisie.
    pub pedagogie: Option<String>,
}

/// Compléments des classes d'une année.
pub async fn classes(
    cnx: &mut Connexion,
    annee_id: &str,
    contexte: &Contexte,
) -> Resultat<Vec<ClasseComplements>> {
    Ok(sqlx::query_as::<_, ClasseComplements>(
        "SELECT c.id, c.libelle, n.cycle, c.fonctionnement, c.pedagogie
         FROM classe c JOIN niveau n ON n.id = c.niveau_id
         WHERE c.annee_id = ?1 AND c.etablissement_id = ?2 AND c.supprime_le IS NULL
         ORDER BY n.ordre, c.libelle",
    )
    .bind(annee_id)
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?)
}

/// Classe lue avec son année.
type LigneClasse = (
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    String,
);

/// Compléments d'une classe.
pub async fn classe(
    cnx: &mut Connexion,
    classe_id: Id<Classe>,
    contexte: &Contexte,
) -> Resultat<(ClasseComplements, String)> {
    let ligne: Option<LigneClasse> = sqlx::query_as(
        "SELECT c.id, c.libelle, n.cycle, c.fonctionnement, c.pedagogie, c.annee_id
             FROM classe c JOIN niveau n ON n.id = c.niveau_id
             WHERE c.id = ?1 AND c.etablissement_id = ?2",
    )
    .bind(classe_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?;
    let (id, libelle, cycle, fonctionnement, pedagogie, annee) =
        ligne.ok_or_else(|| ErreurBase::Introuvable {
            entite: "classe",
            id: classe_id.vers_chaine(),
        })?;
    Ok((
        ClasseComplements {
            id,
            libelle,
            cycle,
            fonctionnement,
            pedagogie,
        },
        annee,
    ))
}

/// Écrit le fonctionnement et la pédagogie d'une classe.
pub async fn ecrire_classe(
    cnx: &mut Connexion,
    classe_id: Id<Classe>,
    fonctionnement: Option<&str>,
    pedagogie: Option<&str>,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE classe SET fonctionnement = ?1, pedagogie = ?2, mis_a_jour_le = ?3,
            mis_a_jour_par = ?4, identifiant_poste = ?5 WHERE id = ?6 AND etablissement_id = ?7",
    )
    .bind(fonctionnement)
    .bind(pedagogie)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(classe_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Vrai si l'élève existe dans l'établissement.
pub async fn eleve_existe(
    cnx: &mut Connexion,
    eleve_id: &str,
    contexte: &Contexte,
) -> Resultat<bool> {
    let ligne: Option<(String,)> =
        sqlx::query_as("SELECT id FROM eleve WHERE id = ?1 AND etablissement_id = ?2")
            .bind(eleve_id)
            .bind(contexte.etablissement_id.vers_chaine())
            .fetch_optional(cnx)
            .await?;
    Ok(ligne.is_some())
}

#[derive(FromRow)]
struct LigneDifficulte {
    eleve_id: String,
    type_difficulte: String,
    degre: Option<String>,
    note: String,
}

/// Difficultés notées des élèves de l'école, retirées comprises.
pub async fn difficultes(
    cnx: &mut Connexion,
    contexte: &Contexte,
) -> Resultat<Vec<DifficulteEleve>> {
    let lignes = sqlx::query_as::<_, LigneDifficulte>(
        "SELECT eleve_id, type AS type_difficulte, degre, note FROM difficulte_eleve
         WHERE etablissement_id = ?1",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?;
    Ok(lignes
        .into_iter()
        .filter_map(|l| {
            Some(DifficulteEleve {
                etablissement_id: contexte.etablissement_id,
                eleve_id: l.eleve_id,
                type_difficulte: TypeDifficulte::depuis_nom(&l.type_difficulte)?,
                degre: l.degre.as_deref().and_then(DegreDifficulte::depuis_nom),
                note: l.note,
            })
        })
        .collect())
}

/// Écrit une difficulté : la ligne de l'élève et du type est récrite, sinon créée.
pub async fn ecrire_difficulte(
    cnx: &mut Connexion,
    d: &DifficulteEleve,
    contexte: &Contexte,
) -> Resultat<()> {
    let maj = sqlx::query(
        "UPDATE difficulte_eleve SET degre = ?1, note = ?2, mis_a_jour_le = ?3,
            mis_a_jour_par = ?4, identifiant_poste = ?5
         WHERE eleve_id = ?6 AND type = ?7 AND etablissement_id = ?8",
    )
    .bind(d.degre.map(DegreDifficulte::nom))
    .bind(&d.note)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(&d.eleve_id)
    .bind(d.type_difficulte.nom())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(&mut *cnx)
    .await?;
    if maj.rows_affected() == 0 {
        let id: Id<DifficulteEleve> = nouvel_id();
        sqlx::query(
            "INSERT INTO difficulte_eleve (id, etablissement_id, eleve_id, type, degre, note,
                cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?7, ?8, ?9)",
        )
        .bind(id.vers_chaine())
        .bind(contexte.etablissement_id.vers_chaine())
        .bind(&d.eleve_id)
        .bind(d.type_difficulte.nom())
        .bind(d.degre.map(DegreDifficulte::nom))
        .bind(&d.note)
        .bind(contexte.maintenant.millis_epoque())
        .bind(contexte.auteur())
        .bind(&contexte.poste.0)
        .execute(cnx)
        .await?;
    }
    Ok(())
}

/// Compléments d'un membre du personnel.
#[derive(Debug, Clone, FromRow)]
pub struct MembreComplements {
    /// Membre.
    pub id: String,
    /// Personne.
    pub personne_id: String,
    /// Nom.
    pub nom: String,
    /// Prénoms.
    pub prenoms: String,
    /// Fonctionnaire coché.
    pub fonctionnaire: i64,
    /// Diplômes en texte libre.
    pub diplomes: String,
    /// Départ.
    pub parti_le: Option<String>,
    /// Statut saisi.
    pub statut_rapport: Option<String>,
    /// Spécialité saisie.
    pub specialite: Option<String>,
    /// Diplôme le plus élevé saisi.
    pub diplome_eleve: Option<String>,
}

/// Compléments de tous les membres du personnel, par nom.
pub async fn membres(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Vec<MembreComplements>> {
    Ok(sqlx::query_as::<_, MembreComplements>(
        "SELECT m.id, m.personne_id, p.nom, p.prenoms, m.fonctionnaire, m.diplomes,
                m.parti_le, m.statut_rapport, m.specialite, m.diplome_eleve
         FROM membre_personnel m JOIN personne p ON p.id = m.personne_id
         WHERE m.etablissement_id = ?1 AND p.supprime_le IS NULL
         ORDER BY p.nom, p.prenoms",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?)
}

/// Écrit le statut, la spécialité et le diplôme le plus élevé d'un membre du personnel.
pub async fn ecrire_membre(
    cnx: &mut Connexion,
    membre_id: &str,
    statut: Option<&str>,
    specialite: Option<&str>,
    diplome: Option<&str>,
    contexte: &Contexte,
) -> Resultat<u64> {
    Ok(sqlx::query(
        "UPDATE membre_personnel SET statut_rapport = ?1, specialite = ?2, diplome_eleve = ?3,
            mis_a_jour_le = ?4, mis_a_jour_par = ?5, identifiant_poste = ?6
         WHERE id = ?7 AND etablissement_id = ?8",
    )
    .bind(statut)
    .bind(specialite)
    .bind(diplome)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(membre_id)
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?
    .rows_affected())
}
