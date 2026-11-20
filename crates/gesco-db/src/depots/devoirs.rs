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

/// Devoirs donnés (ou clos) à une classe d'une année, par date limite : ce que voient ses élèves.
pub async fn donnes_a_la_classe(
    cnx: &mut Connexion,
    classe_id: &str,
    annee_id: &str,
    contexte: &Contexte,
) -> Resultat<Vec<LigneDevoir>> {
    Ok(
        sqlx::query_as::<_, LigneDevoir>(sqlx::AssertSqlSafe(format!(
            "SELECT {COLONNES} FROM devoir
         WHERE etablissement_id = ?1 AND classe_id = ?2 AND annee_id = ?3
           AND etat IN ('donne', 'clos')
         ORDER BY date_limite, titre"
        )))
        .bind(contexte.etablissement_id.vers_chaine())
        .bind(classe_id)
        .bind(annee_id)
        .fetch_all(cnx)
        .await?,
    )
}

/// Un rendu d'un élève.
#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct LigneRendu {
    /// Identifiant.
    pub id: String,
    /// Devoir.
    pub devoir_id: String,
    /// Élève.
    pub eleve_id: String,
    /// Numéro du rendu pour ce devoir et cet élève ; le plus grand compte.
    pub numero: i64,
    /// `portail` ou `papier`.
    pub origine: String,
    /// Texte rendu.
    pub texte: String,
    /// Rendu le.
    pub rendu_le: i64,
    /// Rendu après la date limite.
    pub en_retard: bool,
    /// Note en centièmes.
    pub note: Option<i64>,
    /// Commentaire du maître.
    pub commentaire: String,
    /// Noté le.
    pub note_le: Option<i64>,
    /// Noté par.
    pub note_par: Option<String>,
}

const COLONNES_RENDU: &str = "r.id, r.devoir_id, r.eleve_id, r.numero, r.origine, r.texte,
    r.rendu_le, r.en_retard, r.note, r.commentaire, r.note_le, r.note_par";

/// Rendus d'un élève (d'un devoir, ou de tous), par devoir puis numéro.
pub async fn rendus_de_l_eleve(
    cnx: &mut Connexion,
    eleve_id: &str,
    devoir_id: Option<&str>,
    contexte: &Contexte,
) -> Resultat<Vec<LigneRendu>> {
    Ok(sqlx::query_as::<_, LigneRendu>(sqlx::AssertSqlSafe(format!(
        "SELECT {COLONNES_RENDU} FROM rendu_devoir r
         WHERE r.etablissement_id = ?1 AND r.eleve_id = ?2 AND (?3 IS NULL OR r.devoir_id = ?3)
         ORDER BY r.devoir_id, r.numero"
    )))
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(eleve_id)
    .bind(devoir_id)
    .fetch_all(cnx)
    .await?)
}

/// Rendus d'un devoir, tous élèves, par élève puis numéro.
pub async fn rendus_du_devoir(
    cnx: &mut Connexion,
    devoir_id: &str,
    contexte: &Contexte,
) -> Resultat<Vec<LigneRendu>> {
    Ok(sqlx::query_as::<_, LigneRendu>(sqlx::AssertSqlSafe(format!(
        "SELECT {COLONNES_RENDU} FROM rendu_devoir r
         WHERE r.etablissement_id = ?1 AND r.devoir_id = ?2
         ORDER BY r.eleve_id, r.numero"
    )))
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(devoir_id)
    .fetch_all(cnx)
    .await?)
}

/// Une photo d'un rendu : (rendu, ordre, empreinte, place libérée).
pub type LignePhoto = (String, i64, String, bool);

/// Photos des rendus d'un devoir (d'un élève, ou de tous), par rendu puis ordre.
pub async fn photos_du_devoir(
    cnx: &mut Connexion,
    devoir_id: &str,
    eleve_id: Option<&str>,
    contexte: &Contexte,
) -> Resultat<Vec<LignePhoto>> {
    Ok(sqlx::query_as(
        "SELECT p.rendu_id, p.ordre, p.empreinte, f.libere_le IS NOT NULL
         FROM photo_rendu p
         JOIN rendu_devoir r ON r.id = p.rendu_id
         JOIN fichier_rendu f ON f.empreinte = p.empreinte
         WHERE r.etablissement_id = ?1 AND r.devoir_id = ?2 AND (?3 IS NULL OR r.eleve_id = ?3)
         ORDER BY p.rendu_id, p.ordre",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(devoir_id)
    .bind(eleve_id)
    .fetch_all(cnx)
    .await?)
}

/// Numéro du prochain rendu d'un élève pour un devoir.
pub async fn prochain_numero(
    cnx: &mut Connexion,
    devoir_id: &str,
    eleve_id: &str,
) -> Resultat<i64> {
    let dernier: Option<i64> = sqlx::query_scalar(
        "SELECT MAX(numero) FROM rendu_devoir WHERE devoir_id = ?1 AND eleve_id = ?2",
    )
    .bind(devoir_id)
    .bind(eleve_id)
    .fetch_one(cnx)
    .await?;
    Ok(dernier.unwrap_or(0).saturating_add(1))
}

/// Une photo à ranger avec un rendu, déjà vérifiée et nettoyée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhotoAEcrire {
    /// Ordre dans le rendu (1 à 5).
    pub ordre: i64,
    /// Empreinte SHA-256 du contenu nettoyé.
    pub empreinte: String,
    /// Contenu nettoyé.
    pub contenu: Vec<u8>,
    /// `jpeg` ou `png`.
    pub sorte: String,
}

/// Ajoute un rendu et ses photos ; une photo déjà rangée (même empreinte) n'est pas recopiée.
pub async fn inserer_rendu(
    cnx: &mut Connexion,
    r: &LigneRendu,
    photos: &[PhotoAEcrire],
    contexte: &Contexte,
) -> Resultat<()> {
    let etablissement = contexte.etablissement_id.vers_chaine();
    sqlx::query(
        "INSERT INTO rendu_devoir (id, etablissement_id, devoir_id, eleve_id, numero, origine,
            texte, rendu_le, en_retard, note, commentaire, note_le, note_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
    )
    .bind(&r.id)
    .bind(&etablissement)
    .bind(&r.devoir_id)
    .bind(&r.eleve_id)
    .bind(r.numero)
    .bind(&r.origine)
    .bind(&r.texte)
    .bind(r.rendu_le)
    .bind(r.en_retard)
    .bind(r.note)
    .bind(&r.commentaire)
    .bind(r.note_le)
    .bind(&r.note_par)
    .bind(&contexte.poste.0)
    .execute(&mut *cnx)
    .await?;
    for p in photos {
        sqlx::query(
            "INSERT INTO fichier_rendu (empreinte, etablissement_id, contenu, taille, sorte, cree_le)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT (empreinte) DO NOTHING",
        )
        .bind(&p.empreinte)
        .bind(&etablissement)
        .bind(&p.contenu)
        .bind(i64::try_from(p.contenu.len()).unwrap_or(i64::MAX))
        .bind(&p.sorte)
        .bind(r.rendu_le)
        .execute(&mut *cnx)
        .await?;
        sqlx::query(
            "INSERT INTO photo_rendu (id, rendu_id, ordre, empreinte) VALUES (?1, ?2, ?3, ?4)",
        )
        .bind(crate::nouvel_id::<()>().vers_chaine())
        .bind(&r.id)
        .bind(p.ordre)
        .bind(&p.empreinte)
        .execute(&mut *cnx)
        .await?;
    }
    Ok(())
}

/// Une photo d'un rendu : (sorte, contenu) ; contenu `None` quand la place a été libérée.
/// `eleve_id` borne aux rendus de cet élève (portail).
pub async fn photo_du_rendu(
    cnx: &mut Connexion,
    rendu_id: &str,
    ordre: i64,
    eleve_id: Option<&str>,
    contexte: &Contexte,
) -> Resultat<Option<(String, Option<Vec<u8>>)>> {
    Ok(sqlx::query_as(
        "SELECT f.sorte, f.contenu
         FROM photo_rendu p
         JOIN rendu_devoir r ON r.id = p.rendu_id
         JOIN fichier_rendu f ON f.empreinte = p.empreinte
         WHERE r.etablissement_id = ?1 AND p.rendu_id = ?2 AND p.ordre = ?3
           AND (?4 IS NULL OR r.eleve_id = ?4)",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(rendu_id)
    .bind(ordre)
    .bind(eleve_id)
    .fetch_optional(cnx)
    .await?)
}

/// Écrit la note et le commentaire d'un rendu (note `None` : retirée).
pub async fn noter_rendu(
    cnx: &mut Connexion,
    rendu_id: &str,
    (note, commentaire): (Option<i64>, &str),
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE rendu_devoir SET note = ?3, commentaire = ?4, note_le = ?5, note_par = ?6,
            identifiant_poste = ?7
         WHERE etablissement_id = ?1 AND id = ?2",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(rendu_id)
    .bind(note)
    .bind(commentaire)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// « Rendre les copies corrigées » : notes et commentaires visibles des élèves.
pub async fn marquer_copies_rendues(
    cnx: &mut Connexion,
    devoir_id: &str,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE devoir SET copies_rendues_le = ?3, mis_a_jour_le = ?3, mis_a_jour_par = ?4,
            identifiant_poste = ?5
         WHERE etablissement_id = ?1 AND id = ?2",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(devoir_id)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Évaluations nées du report d'un devoir, pour une classe : (évaluation, devoir).
pub async fn evaluations_reportees(
    cnx: &mut Connexion,
    classe_id: &str,
    contexte: &Contexte,
) -> Resultat<Vec<(String, String)>> {
    Ok(sqlx::query_as(
        "SELECT id, devoir_id FROM evaluation
         WHERE etablissement_id = ?1 AND classe_id = ?2 AND devoir_id IS NOT NULL",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(classe_id)
    .fetch_all(cnx)
    .await?)
}

/// L'évaluation née du report d'un devoir, s'il a été reporté.
pub async fn evaluation_du_devoir(
    cnx: &mut Connexion,
    devoir_id: &str,
    contexte: &Contexte,
) -> Resultat<Option<String>> {
    Ok(sqlx::query_scalar(
        "SELECT id FROM evaluation WHERE etablissement_id = ?1 AND devoir_id = ?2",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(devoir_id)
    .fetch_optional(cnx)
    .await?)
}

/// Marque une évaluation comme le report d'un devoir (`origine` « devoir en ligne »).
pub async fn lier_evaluation(
    cnx: &mut Connexion,
    evaluation_id: &str,
    devoir_id: &str,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE evaluation SET origine = 'devoir_en_ligne', devoir_id = ?3
         WHERE etablissement_id = ?1 AND id = ?2",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(evaluation_id)
    .bind(devoir_id)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Ce qu'un nouveau report récrit d'une évaluation en saisie : période, jour, libellé, barème,
/// pondération.
pub struct EvaluationRecrite<'a> {
    /// Évaluation.
    pub id: &'a str,
    /// Période.
    pub periode_id: &'a str,
    /// Jour (AAAA-MM-JJ).
    pub date: String,
    /// Libellé.
    pub libelle: &'a str,
    /// Barème en centièmes.
    pub bareme: i32,
    /// Pondération.
    pub ponderation: i32,
}

/// Récrit une évaluation née d'un report, encore en saisie.
pub async fn recrire_evaluation_reportee(
    cnx: &mut Connexion,
    e: &EvaluationRecrite<'_>,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE evaluation SET periode_id = ?3, date = ?4, libelle = ?5, bareme = ?6,
            ponderation = ?7, mis_a_jour_le = ?8, mis_a_jour_par = ?9, identifiant_poste = ?10
         WHERE etablissement_id = ?1 AND id = ?2 AND statut = 'en_saisie'",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(e.id)
    .bind(e.periode_id)
    .bind(&e.date)
    .bind(e.libelle)
    .bind(e.bareme)
    .bind(e.ponderation)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Note l'instant du dernier report au bulletin.
pub async fn marquer_reporte(
    cnx: &mut Connexion,
    devoir_id: &str,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE devoir SET reporte_le = ?3, mis_a_jour_le = ?3, mis_a_jour_par = ?4,
            identifiant_poste = ?5
         WHERE etablissement_id = ?1 AND id = ?2",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(devoir_id)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Place prise par les photos des copies d'une année : (nombre de photos gardées, octets gardés,
/// nombre de photos déjà retirées). Une photo rangée une fois compte une fois.
pub async fn place_des_photos(
    cnx: &mut Connexion,
    annee_id: &str,
    contexte: &Contexte,
) -> Resultat<(i64, i64, i64)> {
    Ok(sqlx::query_as(
        "SELECT COALESCE(SUM(f.libere_le IS NULL), 0),
                COALESCE(SUM(CASE WHEN f.libere_le IS NULL THEN f.taille ELSE 0 END), 0),
                COALESCE(SUM(f.libere_le IS NOT NULL), 0)
         FROM fichier_rendu f
         WHERE f.etablissement_id = ?1 AND f.empreinte IN (
            SELECT p.empreinte FROM photo_rendu p
            JOIN rendu_devoir r ON r.id = p.rendu_id
            JOIN devoir d ON d.id = r.devoir_id
            WHERE d.etablissement_id = ?1 AND d.annee_id = ?2)",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(annee_id)
    .fetch_one(cnx)
    .await?)
}

/// Vide le contenu des photos des copies d'une année ; une photo aussi rendue pour un devoir
/// d'une autre année encore ouverte est gardée. Rend le nombre de photos retirées.
pub async fn liberer_les_photos(
    cnx: &mut Connexion,
    annee_id: &str,
    contexte: &Contexte,
) -> Resultat<u64> {
    let fait = sqlx::query(
        "UPDATE fichier_rendu SET contenu = NULL, libere_le = ?3
         WHERE etablissement_id = ?1 AND libere_le IS NULL
           AND empreinte IN (
            SELECT p.empreinte FROM photo_rendu p
            JOIN rendu_devoir r ON r.id = p.rendu_id
            JOIN devoir d ON d.id = r.devoir_id
            WHERE d.etablissement_id = ?1 AND d.annee_id = ?2)
           AND empreinte NOT IN (
            SELECT p.empreinte FROM photo_rendu p
            JOIN rendu_devoir r ON r.id = p.rendu_id
            JOIN devoir d ON d.id = r.devoir_id
            JOIN annee_scolaire a ON a.id = d.annee_id
            WHERE d.etablissement_id = ?1 AND d.annee_id <> ?2
              AND a.statut NOT IN ('cloturee', 'archivee'))",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(annee_id)
    .bind(contexte.maintenant.millis_epoque())
    .execute(cnx)
    .await?;
    Ok(fait.rows_affected())
}
