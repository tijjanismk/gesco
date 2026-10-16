//! Personnes, élèves, responsables et liens (SPEC_PHASE_2 §4).

use gesco_core::commun::{DateCivile, Id};
use gesco_core::scolarite::{Eleve, Lien, NatureActe, Personne, Qualites, Responsable, Sexe};
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::depots::booleen;
use crate::erreur::{ErreurBase, Resultat};
use crate::id::lire_id;

fn cloisonner<T>(
    entite: &'static str,
    proprietaire: Id<gesco_core::core::Etablissement>,
    id: Id<T>,
    contexte: &Contexte,
) -> Resultat<()> {
    if proprietaire != contexte.etablissement_id {
        return Err(ErreurBase::Cloisonnement {
            entite,
            id: id.vers_chaine(),
        });
    }
    Ok(())
}

fn date(
    table: &'static str,
    colonne: &'static str,
    texte: Option<String>,
) -> Resultat<Option<DateCivile>> {
    texte
        .map(|t| DateCivile::analyser(&t).ok_or_else(|| ErreurBase::illisible(table, colonne, &t)))
        .transpose()
}

// --- Personnes ------------------------------------------------------------------------------

/// Colonnes d'identité, sans la photo.
macro_rules! colonnes_personne {
    ($alias:literal) => {
        concat!(
            $alias,
            "id AS p_id, ",
            $alias,
            "etablissement_id AS p_etablissement_id, ",
            $alias,
            "nom AS p_nom, ",
            $alias,
            "prenoms AS p_prenoms, ",
            $alias,
            "sexe AS p_sexe, ",
            $alias,
            "date_naissance AS p_date_naissance, ",
            $alias,
            "date_naissance_estimee AS p_date_naissance_estimee, ",
            $alias,
            "lieu_naissance AS p_lieu_naissance, ",
            $alias,
            "nationalite AS p_nationalite, ",
            $alias,
            "adresse AS p_adresse, ",
            $alias,
            "telephones AS p_telephones"
        )
    };
}

#[derive(FromRow)]
struct LignePersonne {
    p_id: String,
    p_etablissement_id: String,
    p_nom: String,
    p_prenoms: String,
    p_sexe: Option<String>,
    p_date_naissance: Option<String>,
    p_date_naissance_estimee: i64,
    p_lieu_naissance: String,
    p_nationalite: String,
    p_adresse: String,
    p_telephones: String,
}

impl LignePersonne {
    fn vers_entite(self) -> Resultat<Personne> {
        Ok(Personne {
            id: lire_id("personne", "id", &self.p_id)?,
            etablissement_id: lire_id("personne", "etablissement_id", &self.p_etablissement_id)?,
            sexe: self
                .p_sexe
                .map(|s| {
                    Sexe::depuis_nom(&s)
                        .ok_or_else(|| ErreurBase::illisible("personne", "sexe", &s))
                })
                .transpose()?,
            date_naissance: date("personne", "date_naissance", self.p_date_naissance)?,
            telephones: serde_json::from_str(&self.p_telephones)?,
            nom: self.p_nom,
            prenoms: self.p_prenoms,
            date_naissance_estimee: booleen(self.p_date_naissance_estimee),
            lieu_naissance: self.p_lieu_naissance,
            nationalite: self.p_nationalite,
            adresse: self.p_adresse,
        })
    }
}

/// Crée une personne.
pub async fn inserer_personne(
    cnx: &mut Connexion,
    personne: &Personne,
    contexte: &Contexte,
) -> Resultat<()> {
    cloisonner("personne", personne.etablissement_id, personne.id, contexte)?;
    sqlx::query(
        "INSERT INTO personne (id, etablissement_id, nom, prenoms, sexe, date_naissance, date_naissance_estimee,
            lieu_naissance, nationalite, adresse, telephones,
            cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?12, ?13, ?14)",
    )
    .bind(personne.id.vers_chaine())
    .bind(personne.etablissement_id.vers_chaine())
    .bind(&personne.nom)
    .bind(&personne.prenoms)
    .bind(personne.sexe.map(|s| s.nom()))
    .bind(personne.date_naissance.map(|d| d.to_string()))
    .bind(i64::from(personne.date_naissance_estimee))
    .bind(&personne.lieu_naissance)
    .bind(&personne.nationalite)
    .bind(&personne.adresse)
    .bind(serde_json::to_string(&personne.telephones)?)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Récrit l'identité d'une personne ; la photo a sa propre écriture.
pub async fn modifier_personne(
    cnx: &mut Connexion,
    personne: &Personne,
    contexte: &Contexte,
) -> Resultat<()> {
    cloisonner("personne", personne.etablissement_id, personne.id, contexte)?;
    sqlx::query(
        "UPDATE personne SET nom = ?1, prenoms = ?2, sexe = ?3, date_naissance = ?4, date_naissance_estimee = ?5,
            lieu_naissance = ?6, nationalite = ?7, adresse = ?8, telephones = ?9,
            mis_a_jour_le = ?10, mis_a_jour_par = ?11, identifiant_poste = ?12
         WHERE id = ?13 AND etablissement_id = ?14 AND supprime_le IS NULL",
    )
    .bind(&personne.nom)
    .bind(&personne.prenoms)
    .bind(personne.sexe.map(|s| s.nom()))
    .bind(personne.date_naissance.map(|d| d.to_string()))
    .bind(i64::from(personne.date_naissance_estimee))
    .bind(&personne.lieu_naissance)
    .bind(&personne.nationalite)
    .bind(&personne.adresse)
    .bind(serde_json::to_string(&personne.telephones)?)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(personne.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Lit une personne.
pub async fn personne(
    cnx: &mut Connexion,
    id: Id<Personne>,
    contexte: &Contexte,
) -> Resultat<Personne> {
    sqlx::query_as::<_, LignePersonne>(concat!(
        "SELECT ",
        colonnes_personne!(""),
        " FROM personne WHERE id = ?1 AND etablissement_id = ?2 AND supprime_le IS NULL"
    ))
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .ok_or_else(|| ErreurBase::Introuvable {
        entite: "personne",
        id: id.vers_chaine(),
    })?
    .vers_entite()
}

/// Photo d'une personne : octets et format.
pub async fn photo(
    cnx: &mut Connexion,
    id: Id<Personne>,
    contexte: &Contexte,
) -> Resultat<Option<(Vec<u8>, String)>> {
    let ligne: Option<(Option<Vec<u8>>, Option<String>)> = sqlx::query_as(
        "SELECT photo, photo_format FROM personne WHERE id = ?1 AND etablissement_id = ?2 AND supprime_le IS NULL",
    )
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?;
    Ok(match ligne {
        Some((Some(octets), Some(format))) => Some((octets, format)),
        _ => None,
    })
}

/// Pose ou retire la photo d'une personne.
pub async fn definir_photo(
    cnx: &mut Connexion,
    id: Id<Personne>,
    photo: Option<(&[u8], &str)>,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE personne SET photo = ?1, photo_format = ?2, mis_a_jour_le = ?3, mis_a_jour_par = ?4, identifiant_poste = ?5
         WHERE id = ?6 AND etablissement_id = ?7 AND supprime_le IS NULL",
    )
    .bind(photo.map(|(octets, _)| octets))
    .bind(photo.map(|(_, format)| format))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

// --- Élèves ---------------------------------------------------------------------------------

#[derive(FromRow)]
struct LigneEleve {
    e_id: String,
    e_etablissement_id: String,
    e_personne_id: String,
    e_matricule_interne: String,
    e_matricule_national: Option<String>,
    e_acte_nature: Option<String>,
    e_acte_numero: String,
    e_acte_date: Option<String>,
    e_ecole_origine: String,
    e_fusionne_dans: Option<String>,
    #[sqlx(flatten)]
    personne: LignePersonne,
}

impl LigneEleve {
    fn vers_entite(self) -> Resultat<(Eleve, Personne)> {
        let eleve = Eleve {
            id: lire_id("eleve", "id", &self.e_id)?,
            etablissement_id: lire_id("eleve", "etablissement_id", &self.e_etablissement_id)?,
            personne_id: lire_id("eleve", "personne_id", &self.e_personne_id)?,
            acte_nature: self
                .e_acte_nature
                .map(|n| {
                    NatureActe::depuis_nom(&n)
                        .ok_or_else(|| ErreurBase::illisible("eleve", "acte_nature", &n))
                })
                .transpose()?,
            acte_date: date("eleve", "acte_date", self.e_acte_date)?,
            fusionne_dans: self
                .e_fusionne_dans
                .as_deref()
                .map(|f| lire_id("eleve", "fusionne_dans", f))
                .transpose()?,
            matricule_interne: self.e_matricule_interne,
            matricule_national: self.e_matricule_national,
            acte_numero: self.e_acte_numero,
            ecole_origine: self.e_ecole_origine,
        };
        Ok((eleve, self.personne.vers_entite()?))
    }
}

macro_rules! selectionner_eleve {
    ($suite:literal) => {
        concat!(
            "SELECT e.id AS e_id, e.etablissement_id AS e_etablissement_id, e.personne_id AS e_personne_id,
                e.matricule_interne AS e_matricule_interne, e.matricule_national AS e_matricule_national,
                e.acte_nature AS e_acte_nature, e.acte_numero AS e_acte_numero, e.acte_date AS e_acte_date,
                e.ecole_origine AS e_ecole_origine, e.fusionne_dans AS e_fusionne_dans, ",
            colonnes_personne!("p."),
            " FROM eleve e JOIN personne p ON p.id = e.personne_id ",
            $suite
        )
    };
}

/// Crée un élève ; sa personne existe déjà.
pub async fn inserer_eleve(
    cnx: &mut Connexion,
    eleve: &Eleve,
    contexte: &Contexte,
) -> Resultat<()> {
    cloisonner("eleve", eleve.etablissement_id, eleve.id, contexte)?;
    sqlx::query(
        "INSERT INTO eleve (id, etablissement_id, personne_id, matricule_interne, matricule_national,
            acte_nature, acte_numero, acte_date, ecole_origine,
            cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?10, ?11, ?12)",
    )
    .bind(eleve.id.vers_chaine())
    .bind(eleve.etablissement_id.vers_chaine())
    .bind(eleve.personne_id.vers_chaine())
    .bind(&eleve.matricule_interne)
    .bind(eleve.matricule_national.as_deref())
    .bind(eleve.acte_nature.map(|n| n.nom()))
    .bind(&eleve.acte_numero)
    .bind(eleve.acte_date.map(|d| d.to_string()))
    .bind(&eleve.ecole_origine)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Récrit les champs propres à l'élève ; le matricule interne ne change jamais.
pub async fn modifier_eleve(
    cnx: &mut Connexion,
    eleve: &Eleve,
    contexte: &Contexte,
) -> Resultat<()> {
    cloisonner("eleve", eleve.etablissement_id, eleve.id, contexte)?;
    sqlx::query(
        "UPDATE eleve SET matricule_national = ?1, acte_nature = ?2, acte_numero = ?3, acte_date = ?4,
            ecole_origine = ?5, mis_a_jour_le = ?6, mis_a_jour_par = ?7, identifiant_poste = ?8
         WHERE id = ?9 AND etablissement_id = ?10 AND supprime_le IS NULL",
    )
    .bind(eleve.matricule_national.as_deref())
    .bind(eleve.acte_nature.map(|n| n.nom()))
    .bind(&eleve.acte_numero)
    .bind(eleve.acte_date.map(|d| d.to_string()))
    .bind(&eleve.ecole_origine)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(eleve.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Lit un élève et son identité, fusionné ou non.
pub async fn eleve(
    cnx: &mut Connexion,
    id: Id<Eleve>,
    contexte: &Contexte,
) -> Resultat<(Eleve, Personne)> {
    sqlx::query_as::<_, LigneEleve>(selectionner_eleve!(
        "WHERE e.id = ?1 AND e.etablissement_id = ?2 AND e.supprime_le IS NULL"
    ))
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .ok_or_else(|| ErreurBase::Introuvable {
        entite: "eleve",
        id: id.vers_chaine(),
    })?
    .vers_entite()
}

/// Élèves de l'établissement et leur identité, fiches absorbées exclues : le socle de la
/// recherche, qui se calcule en mémoire (SPEC_PHASE_2 §6.2).
pub async fn eleves(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Vec<(Eleve, Personne)>> {
    sqlx::query_as::<_, LigneEleve>(selectionner_eleve!(
        "WHERE e.etablissement_id = ?1 AND e.supprime_le IS NULL AND e.fusionne_dans IS NULL
         ORDER BY p.nom, p.prenoms"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneEleve::vers_entite)
    .collect()
}

/// Fiches absorbées par une fusion, et leur identité : leur matricule mène à la fiche conservée.
pub async fn eleves_absorbes(
    cnx: &mut Connexion,
    contexte: &Contexte,
) -> Resultat<Vec<(Eleve, Personne)>> {
    sqlx::query_as::<_, LigneEleve>(selectionner_eleve!(
        "WHERE e.etablissement_id = ?1 AND e.supprime_le IS NULL AND e.fusionne_dans IS NOT NULL"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneEleve::vers_entite)
    .collect()
}

/// Désigne la fiche qui en absorbe une autre (SPEC_PHASE_2 §8) ; l'absorbée n'est pas supprimée.
pub async fn absorber(
    cnx: &mut Connexion,
    absorbee: Id<Eleve>,
    conservee: Id<Eleve>,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE eleve SET fusionne_dans = ?1, mis_a_jour_le = ?2, mis_a_jour_par = ?3, identifiant_poste = ?4
         WHERE id = ?5 AND etablissement_id = ?6 AND supprime_le IS NULL AND fusionne_dans IS NULL",
    )
    .bind(conservee.vers_chaine())
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(absorbee.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Vrai si un autre élève porte ce matricule interne, fiches absorbées comprises : un matricule
/// n'est jamais réemployé.
pub async fn matricule_interne_pris(
    cnx: &mut Connexion,
    matricule: &str,
    sauf: Option<Id<Eleve>>,
    contexte: &Contexte,
) -> Resultat<bool> {
    let trouve: Option<String> = sqlx::query_scalar(
        "SELECT id FROM eleve WHERE etablissement_id = ?1 AND matricule_interne = ?2
           AND (?3 IS NULL OR id <> ?3) LIMIT 1",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(matricule)
    .bind(sauf.map(|s| s.vers_chaine()))
    .fetch_optional(cnx)
    .await?;
    Ok(trouve.is_some())
}

/// Récrit le matricule interne d'un élève, quand l'école le saisit elle-même.
pub async fn renumeroter(
    cnx: &mut Connexion,
    id: Id<Eleve>,
    matricule: &str,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE eleve SET matricule_interne = ?1, mis_a_jour_le = ?2, mis_a_jour_par = ?3, identifiant_poste = ?4
         WHERE id = ?5 AND etablissement_id = ?6 AND supprime_le IS NULL",
    )
    .bind(matricule)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Vrai si un autre élève vivant porte ce matricule national.
pub async fn matricule_national_pris(
    cnx: &mut Connexion,
    matricule: &str,
    sauf: Option<Id<Eleve>>,
    contexte: &Contexte,
) -> Resultat<bool> {
    let trouve: Option<String> = sqlx::query_scalar(
        "SELECT id FROM eleve WHERE etablissement_id = ?1 AND matricule_national = ?2
           AND fusionne_dans IS NULL AND supprime_le IS NULL AND (?3 IS NULL OR id <> ?3) LIMIT 1",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(matricule)
    .bind(sauf.map(|s| s.vers_chaine()))
    .fetch_optional(cnx)
    .await?;
    Ok(trouve.is_some())
}

// --- Responsables ---------------------------------------------------------------------------

#[derive(FromRow)]
struct LigneResponsable {
    r_id: String,
    r_etablissement_id: String,
    r_personne_id: String,
    r_profession: String,
    r_employeur: String,
    #[sqlx(flatten)]
    personne: LignePersonne,
}

impl LigneResponsable {
    fn vers_entite(self) -> Resultat<(Responsable, Personne)> {
        Ok((
            Responsable {
                id: lire_id("responsable", "id", &self.r_id)?,
                etablissement_id: lire_id(
                    "responsable",
                    "etablissement_id",
                    &self.r_etablissement_id,
                )?,
                personne_id: lire_id("responsable", "personne_id", &self.r_personne_id)?,
                profession: self.r_profession,
                employeur: self.r_employeur,
            },
            self.personne.vers_entite()?,
        ))
    }
}

macro_rules! selectionner_responsable {
    ($suite:literal) => {
        concat!(
            "SELECT r.id AS r_id, r.etablissement_id AS r_etablissement_id, r.personne_id AS r_personne_id,
                r.profession AS r_profession, r.employeur AS r_employeur, ",
            colonnes_personne!("p."),
            " FROM responsable r JOIN personne p ON p.id = r.personne_id ",
            $suite
        )
    };
}

/// Crée un responsable ; sa personne existe déjà.
pub async fn inserer_responsable(
    cnx: &mut Connexion,
    responsable: &Responsable,
    contexte: &Contexte,
) -> Resultat<()> {
    cloisonner(
        "responsable",
        responsable.etablissement_id,
        responsable.id,
        contexte,
    )?;
    sqlx::query(
        "INSERT INTO responsable (id, etablissement_id, personne_id, profession, employeur,
            cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?6, ?7, ?8)",
    )
    .bind(responsable.id.vers_chaine())
    .bind(responsable.etablissement_id.vers_chaine())
    .bind(responsable.personne_id.vers_chaine())
    .bind(&responsable.profession)
    .bind(&responsable.employeur)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Récrit la profession et l'employeur d'un responsable.
pub async fn modifier_responsable(
    cnx: &mut Connexion,
    responsable: &Responsable,
    contexte: &Contexte,
) -> Resultat<()> {
    cloisonner(
        "responsable",
        responsable.etablissement_id,
        responsable.id,
        contexte,
    )?;
    sqlx::query(
        "UPDATE responsable SET profession = ?1, employeur = ?2, mis_a_jour_le = ?3, mis_a_jour_par = ?4, identifiant_poste = ?5
         WHERE id = ?6 AND etablissement_id = ?7 AND supprime_le IS NULL",
    )
    .bind(&responsable.profession)
    .bind(&responsable.employeur)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(responsable.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Lit un responsable et son identité.
pub async fn responsable(
    cnx: &mut Connexion,
    id: Id<Responsable>,
    contexte: &Contexte,
) -> Resultat<(Responsable, Personne)> {
    sqlx::query_as::<_, LigneResponsable>(selectionner_responsable!(
        "WHERE r.id = ?1 AND r.etablissement_id = ?2 AND r.supprime_le IS NULL"
    ))
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .ok_or_else(|| ErreurBase::Introuvable {
        entite: "responsable",
        id: id.vers_chaine(),
    })?
    .vers_entite()
}

/// Responsables de l'établissement et leur identité.
pub async fn responsables(
    cnx: &mut Connexion,
    contexte: &Contexte,
) -> Resultat<Vec<(Responsable, Personne)>> {
    sqlx::query_as::<_, LigneResponsable>(selectionner_responsable!(
        "WHERE r.etablissement_id = ?1 AND r.supprime_le IS NULL ORDER BY p.nom, p.prenoms"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneResponsable::vers_entite)
    .collect()
}

// --- Liens ----------------------------------------------------------------------------------

#[derive(FromRow)]
struct LigneLien {
    eleve_id: String,
    responsable_id: String,
    lien: String,
    est_responsable_legal: i64,
    est_payeur: i64,
    recoit_notifications: i64,
    vient_chercher: i64,
    ordre_priorite: i64,
}

impl LigneLien {
    fn vers_entite(self) -> Resultat<(Id<Eleve>, Id<Responsable>, Qualites)> {
        Ok((
            lire_id("eleve_responsable", "eleve_id", &self.eleve_id)?,
            lire_id("eleve_responsable", "responsable_id", &self.responsable_id)?,
            Qualites {
                lien: Lien::depuis_nom(&self.lien).ok_or_else(|| {
                    ErreurBase::illisible("eleve_responsable", "lien", &self.lien)
                })?,
                est_responsable_legal: booleen(self.est_responsable_legal),
                est_payeur: booleen(self.est_payeur),
                recoit_notifications: booleen(self.recoit_notifications),
                vient_chercher: booleen(self.vient_chercher),
                ordre_priorite: self.ordre_priorite,
            },
        ))
    }
}

/// Lie un responsable à un élève.
pub async fn lier(
    cnx: &mut Connexion,
    eleve_id: Id<Eleve>,
    responsable_id: Id<Responsable>,
    qualites: &Qualites,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "INSERT INTO eleve_responsable (etablissement_id, eleve_id, responsable_id, lien, est_responsable_legal,
            est_payeur, recoit_notifications, vient_chercher, ordre_priorite,
            cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?10, ?11, ?12)",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(eleve_id.vers_chaine())
    .bind(responsable_id.vers_chaine())
    .bind(qualites.lien.nom())
    .bind(i64::from(qualites.est_responsable_legal))
    .bind(i64::from(qualites.est_payeur))
    .bind(i64::from(qualites.recoit_notifications))
    .bind(i64::from(qualites.vient_chercher))
    .bind(qualites.ordre_priorite)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Récrit les qualités d'un lien vivant.
pub async fn modifier_lien(
    cnx: &mut Connexion,
    eleve_id: Id<Eleve>,
    responsable_id: Id<Responsable>,
    qualites: &Qualites,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE eleve_responsable SET lien = ?1, est_responsable_legal = ?2, est_payeur = ?3,
            recoit_notifications = ?4, vient_chercher = ?5, ordre_priorite = ?6,
            mis_a_jour_le = ?7, mis_a_jour_par = ?8, identifiant_poste = ?9
         WHERE eleve_id = ?10 AND responsable_id = ?11 AND etablissement_id = ?12 AND supprime_le IS NULL",
    )
    .bind(qualites.lien.nom())
    .bind(i64::from(qualites.est_responsable_legal))
    .bind(i64::from(qualites.est_payeur))
    .bind(i64::from(qualites.recoit_notifications))
    .bind(i64::from(qualites.vient_chercher))
    .bind(qualites.ordre_priorite)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(eleve_id.vers_chaine())
    .bind(responsable_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Retire un lien (suppression logique).
pub async fn detacher(
    cnx: &mut Connexion,
    eleve_id: Id<Eleve>,
    responsable_id: Id<Responsable>,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE eleve_responsable SET supprime_le = ?1, supprime_par = ?2, mis_a_jour_le = ?1, mis_a_jour_par = ?2, identifiant_poste = ?3
         WHERE eleve_id = ?4 AND responsable_id = ?5 AND etablissement_id = ?6 AND supprime_le IS NULL",
    )
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(eleve_id.vers_chaine())
    .bind(responsable_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Liens vivants d'un élève, dans l'ordre d'appel.
pub async fn liens_de_l_eleve(
    cnx: &mut Connexion,
    eleve_id: Id<Eleve>,
    contexte: &Contexte,
) -> Resultat<Vec<(Id<Eleve>, Id<Responsable>, Qualites)>> {
    sqlx::query_as::<_, LigneLien>(
        "SELECT eleve_id, responsable_id, lien, est_responsable_legal, est_payeur, recoit_notifications,
            vient_chercher, ordre_priorite
         FROM eleve_responsable WHERE eleve_id = ?1 AND etablissement_id = ?2 AND supprime_le IS NULL
         ORDER BY ordre_priorite, rowid",
    )
    .bind(eleve_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneLien::vers_entite)
    .collect()
}

/// Liens vivants de tous les responsables, pour nommer la fratrie d'un responsable trouvé.
pub async fn liens(
    cnx: &mut Connexion,
    contexte: &Contexte,
) -> Resultat<Vec<(Id<Eleve>, Id<Responsable>, Qualites)>> {
    sqlx::query_as::<_, LigneLien>(
        "SELECT eleve_id, responsable_id, lien, est_responsable_legal, est_payeur, recoit_notifications,
            vient_chercher, ordre_priorite
         FROM eleve_responsable WHERE etablissement_id = ?1 AND supprime_le IS NULL",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneLien::vers_entite)
    .collect()
}

/// Payeur d'un élève, pour la liste des impayés (SPEC_PHASE_3 §9.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayeurEleve {
    /// Élève.
    pub eleve_id: Id<Eleve>,
    /// Prénoms et nom.
    pub nom: String,
    /// Téléphones, forme canonique.
    pub telephones: Vec<String>,
}

#[derive(FromRow)]
struct LignePayeur {
    eleve_id: String,
    prenoms: String,
    nom: String,
    telephones: String,
}

/// Payeurs de tous les élèves de l'établissement, par élève puis par ordre de priorité.
pub async fn payeurs(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Vec<PayeurEleve>> {
    sqlx::query_as::<_, LignePayeur>(
        "SELECT er.eleve_id, p.prenoms, p.nom, p.telephones FROM eleve_responsable er
         JOIN responsable r ON r.id = er.responsable_id
         JOIN personne p ON p.id = r.personne_id
         WHERE er.etablissement_id = ?1 AND er.est_payeur = 1 AND er.supprime_le IS NULL
           AND r.supprime_le IS NULL
         ORDER BY er.eleve_id, er.ordre_priorite",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(|l| {
        Ok(PayeurEleve {
            eleve_id: lire_id("eleve_responsable", "eleve_id", &l.eleve_id)?,
            nom: format!("{} {}", l.prenoms, l.nom),
            telephones: serde_json::from_str(&l.telephones)?,
        })
    })
    .collect()
}
