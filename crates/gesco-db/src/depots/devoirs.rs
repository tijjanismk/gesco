//! Devoirs à rendre (phase 12, lot 5) : le devoir que le maître donne à une classe, les rendus
//! des élèves, les photos des copies. Les identifiants voyagent en texte. Rien ne se supprime ;
//! seul le contenu d'une photo d'une année close se vide (`libere_le`).

use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::erreur::Resultat;

/// Un devoir, tel que la base le garde.
#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct LigneDevoir {
    /// Identifiant.
    pub id: String,
    /// Année.
    pub annee_id: String,
    /// Classe.
    pub classe_id: String,
    /// Matière.
    pub matiere_id: String,
    /// Compte qui l'a écrit.
    pub auteur_id: String,
    /// Titre.
    pub titre: String,
    /// Consignes, au format des leçons.
    pub consignes: String,
    /// Leçon liée.
    pub contenu_id: Option<String>,
    /// Date et heure limites (millisecondes).
    pub date_limite: i64,
    /// Barème en centièmes.
    pub bareme: i64,
    /// `texte`, `photos`, `texte_ou_photos` ou `papier`.
    pub forme_rendu: String,
    /// `brouillon`, `donne` ou `clos`.
    pub etat: String,
    /// Donné le.
    pub donne_le: Option<i64>,
    /// Clos le.
    pub clos_le: Option<i64>,
    /// Copies corrigées rendues aux élèves le.
    pub copies_rendues_le: Option<i64>,
    /// Dernier report au bulletin.
    pub reporte_le: Option<i64>,
    /// Devoir dont celui-ci est la copie.
    pub copie_de: Option<String>,
    /// Créé le.
    pub cree_le: i64,
    /// Dernière modification.
    pub mis_a_jour_le: i64,
}

const COLONNES: &str = "id, annee_id, classe_id, matiere_id, auteur_id, titre, consignes,
    contenu_id, date_limite, bareme, forme_rendu, etat, donne_le, clos_le, copies_rendues_le,
    reporte_le, copie_de, cree_le, mis_a_jour_le";

/// Ajoute un devoir.
pub async fn inserer(cnx: &mut Connexion, d: &LigneDevoir, contexte: &Contexte) -> Resultat<()> {
    sqlx::query(
        "INSERT INTO devoir (id, etablissement_id, annee_id, classe_id, matiere_id, auteur_id,
            titre, consignes, contenu_id, date_limite, bareme, forme_rendu, etat, donne_le,
            clos_le, copie_de, cree_le, cree_par, mis_a_jour_le, mis_a_jour_par,
            identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18,
            ?17, ?18, ?19)",
    )
    .bind(&d.id)
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(&d.annee_id)
    .bind(&d.classe_id)
    .bind(&d.matiere_id)
    .bind(&d.auteur_id)
    .bind(&d.titre)
    .bind(&d.consignes)
    .bind(&d.contenu_id)
    .bind(d.date_limite)
    .bind(d.bareme)
    .bind(&d.forme_rendu)
    .bind(&d.etat)
    .bind(d.donne_le)
    .bind(d.clos_le)
    .bind(&d.copie_de)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Récrit ce que le maître écrit d'un devoir (classe, matière, titre, consignes, leçon liée,
/// date limite, barème, façon de rendre).
pub async fn ecrire(cnx: &mut Connexion, d: &LigneDevoir, contexte: &Contexte) -> Resultat<()> {
    sqlx::query(
        "UPDATE devoir SET classe_id = ?3, matiere_id = ?4, titre = ?5, consignes = ?6,
            contenu_id = ?7, date_limite = ?8, bareme = ?9, forme_rendu = ?10,
            mis_a_jour_le = ?11, mis_a_jour_par = ?12, identifiant_poste = ?13
         WHERE etablissement_id = ?1 AND id = ?2",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(&d.id)
    .bind(&d.classe_id)
    .bind(&d.matiere_id)
    .bind(&d.titre)
    .bind(&d.consignes)
    .bind(&d.contenu_id)
    .bind(d.date_limite)
    .bind(d.bareme)
    .bind(&d.forme_rendu)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Change l'état d'un devoir, avec ses instants.
pub async fn changer_etat(
    cnx: &mut Connexion,
    id: &str,
    (etat, donne_le, clos_le): (&str, Option<i64>, Option<i64>),
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE devoir SET etat = ?3, donne_le = ?4, clos_le = ?5, mis_a_jour_le = ?6,
            mis_a_jour_par = ?7, identifiant_poste = ?8
         WHERE etablissement_id = ?1 AND id = ?2",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(id)
    .bind(etat)
    .bind(donne_le)
    .bind(clos_le)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Un devoir de l'établissement.
pub async fn lire(
    cnx: &mut Connexion,
    id: &str,
    contexte: &Contexte,
) -> Resultat<Option<LigneDevoir>> {
    Ok(
        sqlx::query_as::<_, LigneDevoir>(sqlx::AssertSqlSafe(format!(
            "SELECT {COLONNES} FROM devoir WHERE etablissement_id = ?1 AND id = ?2"
        )))
        .bind(contexte.etablissement_id.vers_chaine())
        .bind(id)
        .fetch_optional(cnx)
        .await?,
    )
}

/// Devoirs d'une année, par date limite.
pub async fn de_l_annee(
    cnx: &mut Connexion,
    annee_id: &str,
    contexte: &Contexte,
) -> Resultat<Vec<LigneDevoir>> {
    Ok(
        sqlx::query_as::<_, LigneDevoir>(sqlx::AssertSqlSafe(format!(
            "SELECT {COLONNES} FROM devoir WHERE etablissement_id = ?1 AND annee_id = ?2
         ORDER BY date_limite DESC, titre"
        )))
        .bind(contexte.etablissement_id.vers_chaine())
        .bind(annee_id)
        .fetch_all(cnx)
        .await?,
    )
}

/// Pour chaque devoir de l'année : le nombre d'élèves qui ont rendu (par le portail ou sur
/// papier).
pub async fn rendus_par_devoir(
    cnx: &mut Connexion,
    annee_id: &str,
    contexte: &Contexte,
) -> Resultat<Vec<(String, i64)>> {
    Ok(sqlx::query_as(
        "SELECT r.devoir_id, COUNT(DISTINCT r.eleve_id)
         FROM rendu_devoir r JOIN devoir d ON d.id = r.devoir_id
         WHERE d.etablissement_id = ?1 AND d.annee_id = ?2
         GROUP BY r.devoir_id",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(annee_id)
    .fetch_all(cnx)
    .await?)
}

/// Titre d'un contenu de la bibliothèque (leçon liée), s'il existe.
pub async fn titre_du_contenu(
    cnx: &mut Connexion,
    contenu_id: &str,
    contexte: &Contexte,
) -> Resultat<Option<String>> {
    Ok(
        sqlx::query_scalar("SELECT titre FROM contenu WHERE etablissement_id = ?1 AND id = ?2")
            .bind(contexte.etablissement_id.vers_chaine())
            .bind(contenu_id)
            .fetch_optional(cnx)
            .await?,
    )
}
