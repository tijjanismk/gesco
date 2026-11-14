//! Lectures des statistiques de l'école (SPEC_PHASE_9 §9.1) : les inscriptions d'une année, le
//! personnel, les affectations et l'emploi du temps, tels qu'ils sont en base. Aucun calcul ici :
//! le service compte, à la date qu'on lui donne.

use gesco_core::commun::Id;
use gesco_core::core::AnneeScolaire;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::erreur::Resultat;

/// Une classe de l'année.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClasseStat {
    /// Identifiant.
    pub id: String,
    /// Libellé de la classe.
    pub libelle: String,
    /// Identifiant du niveau.
    pub niveau_id: String,
    /// Libellé du niveau.
    pub niveau: String,
    /// Ordre du niveau.
    pub ordre: i64,
    /// Cycle, nom persisté.
    pub cycle: String,
    /// Classe retirée.
    pub supprimee: bool,
}

/// Une inscription de l'année, avec ce qu'il faut savoir de l'élève.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InscriptionStat {
    /// Identifiant de l'élève.
    pub eleve_id: String,
    /// Nom et prénoms.
    pub nom: String,
    /// Prénoms.
    pub prenoms: String,
    /// Sexe, nom persisté, s'il est connu.
    pub sexe: Option<String>,
    /// Date de naissance, `AAAA-MM-JJ`, si elle est connue.
    pub date_naissance: Option<String>,
    /// Classe actuelle.
    pub classe_id: String,
    /// Classe d'avant un changement de classe en cours d'année.
    pub classe_precedente_id: Option<String>,
    /// Jour d'arrivée dans la classe actuelle, après un changement en cours d'année.
    pub classe_depuis: Option<String>,
    /// Jour de l'inscription.
    pub date_inscription: String,
    /// Type, nom persisté.
    pub type_inscription: String,
    /// État, nom persisté.
    pub statut: String,
    /// Redoublant.
    pub redoublant: bool,
    /// Jour du départ, s'il est noté.
    pub date_depart: Option<String>,
    /// Motif du départ ou de l'abandon.
    pub motif: String,
    /// Dernière modification, millisecondes depuis l'époque.
    pub mis_a_jour_le: i64,
}

/// Un membre du personnel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembreStat {
    /// Identifiant du membre.
    pub id: String,
    /// Personne.
    pub personne_id: String,
    /// Nom.
    pub nom: String,
    /// Prénoms.
    pub prenoms: String,
    /// Sexe, nom persisté, s'il est connu.
    pub sexe: Option<String>,
    /// Matricule dans l'école.
    pub matricule: String,
    /// Catégorie.
    pub categorie: String,
    /// Ordre de la catégorie.
    pub categorie_ordre: i64,
    /// Fonction.
    pub fonction: String,
    /// Jour d'arrivée, s'il est noté.
    pub arrive_le: Option<String>,
    /// Jour du départ, s'il est noté.
    pub parti_le: Option<String>,
    /// Fonctionnaire.
    pub fonctionnaire: bool,
    /// Situation de famille, nom persisté.
    pub situation_famille: Option<String>,
    /// Diplômes, en JSON.
    pub diplomes: String,
}

/// Un engagement du personnel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngagementStat {
    /// Membre.
    pub membre_id: String,
    /// Nature, nom persistée.
    pub nature: String,
    /// Premier jour.
    pub debut: String,
    /// Dernier jour prévu.
    pub fin: Option<String>,
    /// Dernier jour effectif d'un engagement clos.
    pub arrete_le: Option<String>,
}

/// Une matière confiée à un enseignant dans une classe de l'année.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AffectationStat {
    /// Personne de l'enseignant.
    pub personne_id: String,
    /// Classe.
    pub classe_id: String,
    /// Matière.
    pub matiere_id: String,
    /// Libellé de la matière.
    pub matiere: String,
    /// Ordre de la matière.
    pub matiere_ordre: i64,
}

/// Une séance active de l'emploi du temps de l'année.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeanceStat {
    /// Classe.
    pub classe_id: String,
    /// Matière.
    pub matiere_id: String,
    /// Libellé de la matière.
    pub matiere: String,
    /// Ordre de la matière.
    pub matiere_ordre: i64,
    /// Durée, en minutes.
    pub minutes: i64,
}

/// Classes de l'année, supprimées comprises : une inscription peut encore y renvoyer.
pub async fn classes(
    cnx: &mut Connexion,
    annee_id: Id<AnneeScolaire>,
    contexte: &Contexte,
) -> Resultat<Vec<ClasseStat>> {
    let lignes: Vec<(String, String, String, String, i64, String, i64)> = sqlx::query_as(
        "SELECT c.id, c.libelle, n.id, n.libelle, n.ordre, n.cycle, c.supprime_le IS NOT NULL
         FROM classe c JOIN niveau n ON n.id = c.niveau_id
         WHERE c.annee_id = ?1 AND c.etablissement_id = ?2
         ORDER BY n.ordre, c.libelle",
    )
    .bind(annee_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?;
    Ok(lignes
        .into_iter()
        .map(
            |(id, libelle, niveau_id, niveau, ordre, cycle, supprimee)| ClasseStat {
                id,
                libelle,
                niveau_id,
                niveau,
                ordre,
                cycle,
                supprimee: supprimee != 0,
            },
        )
        .collect())
}

/// Ligne lue d'une inscription.
type LigneInscription = (
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    String,
    Option<String>,
    Option<String>,
    String,
    String,
    String,
    i64,
    Option<String>,
    String,
    i64,
);

/// Inscriptions de l'année, hors des fiches fusionnées ou supprimées.
pub async fn inscriptions(
    cnx: &mut Connexion,
    annee_id: Id<AnneeScolaire>,
    contexte: &Contexte,
) -> Resultat<Vec<InscriptionStat>> {
    let lignes: Vec<LigneInscription> = sqlx::query_as(
        "SELECT e.id, p.nom, p.prenoms, p.sexe, p.date_naissance, i.classe_id,
                i.classe_precedente_id, i.classe_depuis, i.date_inscription, i.type, i.statut,
                i.redoublant, i.date_depart,
                CASE WHEN i.motif_depart <> '' THEN i.motif_depart ELSE i.motif_abandon END,
                i.mis_a_jour_le
         FROM inscription i
         JOIN eleve e ON e.id = i.eleve_id
         JOIN personne p ON p.id = e.personne_id
         WHERE i.annee_id = ?1 AND i.etablissement_id = ?2
           AND e.supprime_le IS NULL AND e.fusionne_dans IS NULL AND p.supprime_le IS NULL
         ORDER BY p.nom, p.prenoms, i.date_inscription",
    )
    .bind(annee_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?;
    Ok(lignes
        .into_iter()
        .map(
            |(
                eleve_id,
                nom,
                prenoms,
                sexe,
                date_naissance,
                classe_id,
                classe_precedente_id,
                classe_depuis,
                date_inscription,
                type_inscription,
                statut,
                redoublant,
                date_depart,
                motif,
                mis_a_jour_le,
            )| InscriptionStat {
                eleve_id,
                nom,
                prenoms,
                sexe,
                date_naissance,
                classe_id,
                classe_precedente_id,
                classe_depuis,
                date_inscription,
                type_inscription,
                statut,
                redoublant: redoublant != 0,
                date_depart,
                motif,
                mis_a_jour_le,
            },
        )
        .collect())
}

/// Ligne lue d'un membre du personnel.
type LigneMembre = (
    String,
    String,
    String,
    String,
    Option<String>,
    String,
    String,
    i64,
    String,
    Option<String>,
    Option<String>,
    i64,
    Option<String>,
    String,
);

/// Membres du personnel, partis compris : le service les compte à la date.
pub async fn membres(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Vec<MembreStat>> {
    let lignes: Vec<LigneMembre> = sqlx::query_as(
        "SELECT m.id, p.id, p.nom, p.prenoms, p.sexe, m.matricule, c.libelle, c.ordre, m.fonction,
                m.arrive_le, m.parti_le, m.fonctionnaire, m.situation_famille, m.diplomes
         FROM membre_personnel m
         JOIN personne p ON p.id = m.personne_id
         JOIN categorie_personnel c ON c.id = m.categorie_id
         WHERE m.etablissement_id = ?1 AND p.supprime_le IS NULL
         ORDER BY p.nom, p.prenoms",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?;
    Ok(lignes
        .into_iter()
        .map(
            |(
                id,
                personne_id,
                nom,
                prenoms,
                sexe,
                matricule,
                categorie,
                categorie_ordre,
                fonction,
                arrive_le,
                parti_le,
                fonctionnaire,
                situation_famille,
                diplomes,
            )| MembreStat {
                id,
                personne_id,
                nom,
                prenoms,
                sexe,
                matricule,
                categorie,
                categorie_ordre,
                fonction,
                arrive_le,
                parti_le,
                fonctionnaire: fonctionnaire != 0,
                situation_famille,
                diplomes,
            },
        )
        .collect())
}

/// Ligne lue d'un engagement : membre, nature, début, fin, arrêt.
type LigneEngagement = (String, String, String, Option<String>, Option<String>);

/// Engagements du personnel, clos compris.
pub async fn engagements(
    cnx: &mut Connexion,
    contexte: &Contexte,
) -> Resultat<Vec<EngagementStat>> {
    let lignes: Vec<LigneEngagement> = sqlx::query_as(
        "SELECT membre_id, nature, debut, fin, arrete_le FROM engagement
         WHERE etablissement_id = ?1 ORDER BY debut",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?;
    Ok(lignes
        .into_iter()
        .map(
            |(membre_id, nature, debut, fin, arrete_le)| EngagementStat {
                membre_id,
                nature,
                debut,
                fin,
                arrete_le,
            },
        )
        .collect())
}

/// Matières confiées aux enseignants dans les classes de l'année.
pub async fn affectations(
    cnx: &mut Connexion,
    annee_id: Id<AnneeScolaire>,
    contexte: &Contexte,
) -> Resultat<Vec<AffectationStat>> {
    let lignes: Vec<(String, String, String, String, i64)> = sqlx::query_as(
        "SELECT e.personne_id, a.classe_id, m.id, m.libelle, m.ordre
         FROM affectation_enseignement a
         JOIN enseignant e ON e.id = a.enseignant_id
         JOIN matiere m ON m.id = a.matiere_id
         WHERE a.annee_id = ?1 AND a.etablissement_id = ?2",
    )
    .bind(annee_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?;
    Ok(lignes
        .into_iter()
        .map(
            |(personne_id, classe_id, matiere_id, matiere, matiere_ordre)| AffectationStat {
                personne_id,
                classe_id,
                matiere_id,
                matiere,
                matiere_ordre,
            },
        )
        .collect())
}

/// Séances actives de l'emploi du temps de l'année.
pub async fn seances(
    cnx: &mut Connexion,
    annee_id: Id<AnneeScolaire>,
    contexte: &Contexte,
) -> Resultat<Vec<SeanceStat>> {
    let lignes: Vec<(String, String, String, i64, i64)> = sqlx::query_as(
        "SELECT s.classe_id, m.id, m.libelle, m.ordre, s.fin - s.debut
         FROM seance s JOIN matiere m ON m.id = s.matiere_id
         WHERE s.annee_id = ?1 AND s.etablissement_id = ?2 AND s.active = 1",
    )
    .bind(annee_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?;
    Ok(lignes
        .into_iter()
        .map(
            |(classe_id, matiere_id, matiere, matiere_ordre, minutes)| SeanceStat {
                classe_id,
                matiere_id,
                matiere,
                matiere_ordre,
                minutes,
            },
        )
        .collect())
}
