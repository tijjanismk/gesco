//! Portail des élèves (phase 12, lot 4) : codes des élèves, essais faux comptés, qui peut entrer
//! et les niveaux ouverts à la lecture sans code. Un code ne se supprime pas, il se remplace.
//! Les identifiants voyagent en texte : le portail ne fait que les relire.

use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::erreur::Resultat;

/// Un élève inscrit dans une classe de l'année en cours, tel que le portail le connaît : son
/// prénom et sa classe, rien d'autre de sa fiche.
#[derive(Debug, Clone, FromRow)]
pub struct EleveDuPortail {
    /// Identifiant de l'élève.
    pub eleve_id: String,
    /// Prénom(s).
    pub prenom: String,
    /// Classe de l'année en cours.
    pub classe_id: String,
    /// Libellé de la classe.
    pub classe: String,
    /// Niveau de la classe.
    pub niveau_id: String,
    /// Série de la classe, au lycée.
    pub serie_id: Option<String>,
    /// Année en cours.
    pub annee_id: String,
}

/// L'élève de ce matricule, s'il est inscrit (inscription validée) dans une classe de l'année
/// en cours. Un élève sorti, radié, fusionné ou non réinscrit n'est pas rendu.
pub async fn eleve_pour_entrer(
    cnx: &mut Connexion,
    matricule: &str,
    contexte: &Contexte,
) -> Resultat<Option<EleveDuPortail>> {
    Ok(sqlx::query_as::<_, EleveDuPortail>(
        "SELECT e.id AS eleve_id, p.prenoms AS prenom, c.id AS classe_id, c.libelle AS classe,
            c.niveau_id, c.serie_id, a.id AS annee_id
         FROM eleve e
         JOIN personne p ON p.id = e.personne_id
         JOIN inscription i ON i.eleve_id = e.id AND i.statut = 'validee'
         JOIN annee_scolaire a ON a.id = i.annee_id AND a.est_courante = 1
            AND a.supprime_le IS NULL
         JOIN classe c ON c.id = i.classe_id AND c.supprime_le IS NULL
         WHERE e.etablissement_id = ?1 AND upper(trim(e.matricule_interne)) = ?2
            AND e.supprime_le IS NULL AND e.fusionne_dans IS NULL
         ORDER BY i.date_inscription DESC LIMIT 1",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(matricule)
    .fetch_optional(cnx)
    .await?)
}

/// L'élève de cet identifiant, s'il est inscrit dans une classe de l'année en cours.
pub async fn eleve_inscrit(
    cnx: &mut Connexion,
    eleve_id: &str,
    contexte: &Contexte,
) -> Resultat<Option<EleveDuPortail>> {
    Ok(sqlx::query_as::<_, EleveDuPortail>(
        "SELECT e.id AS eleve_id, p.prenoms AS prenom, c.id AS classe_id, c.libelle AS classe,
            c.niveau_id, c.serie_id, a.id AS annee_id
         FROM eleve e
         JOIN personne p ON p.id = e.personne_id
         JOIN inscription i ON i.eleve_id = e.id AND i.statut = 'validee'
         JOIN annee_scolaire a ON a.id = i.annee_id AND a.est_courante = 1
            AND a.supprime_le IS NULL
         JOIN classe c ON c.id = i.classe_id AND c.supprime_le IS NULL
         WHERE e.etablissement_id = ?1 AND e.id = ?2
            AND e.supprime_le IS NULL AND e.fusionne_dans IS NULL
         ORDER BY i.date_inscription DESC LIMIT 1",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(eleve_id)
    .fetch_optional(cnx)
    .await?)
}

/// Code actif d'un élève.
#[derive(Debug, Clone, FromRow)]
pub struct CodeEleve {
    /// Identifiant de la ligne.
    pub id: String,
    /// Empreinte Argon2id.
    pub empreinte: String,
    /// Remis le (millisecondes).
    pub remis_le: i64,
}

/// Le code actif d'un élève, s'il en a un.
pub async fn code_actif(
    cnx: &mut Connexion,
    eleve_id: &str,
    contexte: &Contexte,
) -> Resultat<Option<CodeEleve>> {
    Ok(sqlx::query_as::<_, CodeEleve>(
        "SELECT id, empreinte, remis_le FROM code_eleve
         WHERE eleve_id = ?1 AND etablissement_id = ?2 AND actif = 1",
    )
    .bind(eleve_id)
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?)
}

/// Remplace le code actif d'un élève par un nouveau : l'ancien est révoqué, la ligne reste.
/// Rend l'identifiant de l'ancien code, s'il y en avait un.
pub async fn remplacer_code(
    cnx: &mut Connexion,
    id: &str,
    eleve_id: &str,
    empreinte: &str,
    contexte: &Contexte,
) -> Resultat<Option<String>> {
    let maintenant = contexte.maintenant.millis_epoque();
    let ancien: Option<String> = sqlx::query_scalar(
        "SELECT id FROM code_eleve WHERE eleve_id = ?1 AND etablissement_id = ?2 AND actif = 1",
    )
    .bind(eleve_id)
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(&mut *cnx)
    .await?;
    if let Some(ancien) = &ancien {
        sqlx::query(
            "UPDATE code_eleve SET actif = 0, revoque_le = ?1, mis_a_jour_le = ?1,
                mis_a_jour_par = ?2, identifiant_poste = ?3
             WHERE id = ?4",
        )
        .bind(maintenant)
        .bind(contexte.auteur())
        .bind(&contexte.poste.0)
        .bind(ancien)
        .execute(&mut *cnx)
        .await?;
    }
    sqlx::query(
        "INSERT INTO code_eleve (id, etablissement_id, eleve_id, empreinte, actif, remis_le,
            remis_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6, ?5, ?6, ?7)",
    )
    .bind(id)
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(eleve_id)
    .bind(empreinte)
    .bind(maintenant)
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(&mut *cnx)
    .await?;
    Ok(ancien)
}

/// Codes faux comptés aujourd'hui pour ce matricule.
pub async fn essais_faux(
    cnx: &mut Connexion,
    matricule: &str,
    jour: &str,
    contexte: &Contexte,
) -> Resultat<i64> {
    Ok(sqlx::query_scalar::<_, i64>(
        "SELECT essais_faux FROM essai_code_eleve
         WHERE etablissement_id = ?1 AND matricule = ?2 AND jour = ?3",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(matricule)
    .bind(jour)
    .fetch_optional(cnx)
    .await?
    .unwrap_or(0))
}

/// Compte un code faux de plus ; `bloque` remplit l'instant du blocage. Rend le nouveau compte.
pub async fn compter_essai_faux(
    cnx: &mut Connexion,
    matricule: &str,
    jour: &str,
    seuil: i64,
    contexte: &Contexte,
) -> Resultat<i64> {
    let maintenant = contexte.maintenant.millis_epoque();
    sqlx::query(
        "INSERT INTO essai_code_eleve (etablissement_id, matricule, jour, essais_faux, bloque_le,
            mis_a_jour_le)
         VALUES (?1, ?2, ?3, 1, CASE WHEN ?5 <= 1 THEN ?4 END, ?4)
         ON CONFLICT (etablissement_id, matricule, jour) DO UPDATE SET
            essais_faux = essais_faux + 1,
            bloque_le = COALESCE(bloque_le, CASE WHEN essais_faux + 1 >= ?5 THEN ?4 END),
            mis_a_jour_le = ?4",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(matricule)
    .bind(jour)
    .bind(maintenant)
    .bind(seuil)
    .execute(&mut *cnx)
    .await?;
    essais_faux(cnx, matricule, jour, contexte).await
}

/// Un élève d'une classe, pour ses billets.
#[derive(Debug, Clone, FromRow)]
pub struct EleveBillet {
    /// Identifiant de l'élève.
    pub eleve_id: String,
    /// Nom.
    pub nom: String,
    /// Prénom(s).
    pub prenoms: String,
    /// Matricule de l'école.
    pub matricule: String,
    /// Libellé de la classe.
    pub classe: String,
    /// Le code actif a été remis le (millisecondes) ; aucun s'il n'a pas de code.
    pub code_remis_le: Option<i64>,
}

/// Élèves inscrits (inscription validée) dans une classe, par ordre alphabétique, avec la date
/// de remise de leur code actif.
pub async fn eleves_de_la_classe(
    cnx: &mut Connexion,
    classe_id: &str,
    contexte: &Contexte,
) -> Resultat<Vec<EleveBillet>> {
    Ok(sqlx::query_as::<_, EleveBillet>(
        "SELECT e.id AS eleve_id, p.nom, p.prenoms, e.matricule_interne AS matricule,
            c.libelle AS classe, k.remis_le AS code_remis_le
         FROM inscription i
         JOIN classe c ON c.id = i.classe_id
         JOIN eleve e ON e.id = i.eleve_id AND e.supprime_le IS NULL AND e.fusionne_dans IS NULL
         JOIN personne p ON p.id = e.personne_id
         LEFT JOIN code_eleve k ON k.eleve_id = e.id AND k.actif = 1
         WHERE i.classe_id = ?1 AND i.etablissement_id = ?2 AND i.statut = 'validee'
         ORDER BY p.nom COLLATE NOCASE, p.prenoms COLLATE NOCASE",
    )
    .bind(classe_id)
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?)
}

/// L'élève d'un billet seul, dans sa classe de l'année en cours.
pub async fn eleve_billet(
    cnx: &mut Connexion,
    eleve_id: &str,
    contexte: &Contexte,
) -> Resultat<Option<EleveBillet>> {
    Ok(sqlx::query_as::<_, EleveBillet>(
        "SELECT e.id AS eleve_id, p.nom, p.prenoms, e.matricule_interne AS matricule,
            c.libelle AS classe, k.remis_le AS code_remis_le
         FROM eleve e
         JOIN personne p ON p.id = e.personne_id
         JOIN inscription i ON i.eleve_id = e.id AND i.statut = 'validee'
         JOIN annee_scolaire a ON a.id = i.annee_id AND a.est_courante = 1
         JOIN classe c ON c.id = i.classe_id
         LEFT JOIN code_eleve k ON k.eleve_id = e.id AND k.actif = 1
         WHERE e.id = ?1 AND e.etablissement_id = ?2
         ORDER BY i.date_inscription DESC LIMIT 1",
    )
    .bind(eleve_id)
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?)
}

/// Date de remise du code actif d'un élève, quelle que soit son inscription.
pub async fn code_remis_le(
    cnx: &mut Connexion,
    eleve_id: &str,
    contexte: &Contexte,
) -> Resultat<Option<i64>> {
    Ok(code_actif(cnx, eleve_id, contexte)
        .await?
        .map(|c| c.remis_le))
}

/// Un niveau ouvert à la lecture sans code, avec une série (lycée) ou sans.
#[derive(Debug, Clone, FromRow)]
pub struct NiveauOuvert {
    /// Niveau.
    pub niveau_id: String,
    /// Libellé du niveau.
    pub niveau: String,
    /// Série, au lycée.
    pub serie_id: Option<String>,
    /// Libellé de la série.
    pub serie: Option<String>,
}

/// Niveaux (et séries) qui ont une classe cette année, hors jardin, dans l'ordre des niveaux.
pub async fn niveaux_ouverts(
    cnx: &mut Connexion,
    contexte: &Contexte,
) -> Resultat<Vec<NiveauOuvert>> {
    Ok(sqlx::query_as::<_, NiveauOuvert>(
        "SELECT DISTINCT n.id AS niveau_id, n.libelle AS niveau, s.id AS serie_id,
            s.libelle AS serie, n.ordre, s.code
         FROM classe c
         JOIN annee_scolaire a ON a.id = c.annee_id AND a.est_courante = 1
         JOIN niveau n ON n.id = c.niveau_id AND n.supprime_le IS NULL AND n.cycle <> 'jardin'
         LEFT JOIN serie s ON s.id = c.serie_id
         WHERE c.etablissement_id = ?1 AND c.supprime_le IS NULL
         ORDER BY n.ordre, s.code",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?)
}

/// Nom de l'école, pour l'en-tête du portail.
pub async fn nom_ecole(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<String> {
    Ok(
        sqlx::query_scalar::<_, String>("SELECT nom FROM etablissement WHERE id = ?1")
            .bind(contexte.etablissement_id.vers_chaine())
            .fetch_optional(cnx)
            .await?
            .unwrap_or_default(),
    )
}
