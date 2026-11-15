//! Bibliothèque de cours : paquets installés, chapitres, contenus, correspondance des
//! disciplines et index de recherche. Le livré se réécrit par sa clé à chaque installation ; ce
//! que l'école a écrit n'est jamais touché ici. Rien ne se supprime, sauf les lignes de l'index,
//! refait d'après le reste.

use std::collections::HashSet;

use gesco_core::bibliotheque::{
    ChapitreBibliotheque, ChapitrePaquet, ContenuBibliotheque, ContenuPaquet, ExamenSujet,
    NatureContenu, OrigineBibliotheque, PaquetInstalle,
};
use gesco_core::commun::Horodatage;
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::depots::booleen;
use crate::erreur::{ErreurBase, Resultat};
use crate::id::{lire_id, nouvel_id};

fn incoherent(table: &'static str, colonne: &'static str, valeur: &str) -> ErreurBase {
    ErreurBase::Integrite(format!("{table}.{colonne} inconnu : {valeur}"))
}

// --- Paquets --------------------------------------------------------------------------------

#[derive(FromRow)]
struct LignePaquet {
    id: String,
    etablissement_id: String,
    code: String,
    version: String,
    date_paquet: String,
    gesco_minimum: String,
    nom_fichier: String,
    taille: i64,
    empreinte_sha256: String,
    disciplines: String,
    actif: i64,
    installe_le: i64,
}

impl LignePaquet {
    fn vers_entite(self) -> Resultat<PaquetInstalle> {
        let t = "bibliotheque_paquet";
        Ok(PaquetInstalle {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            code: self.code,
            version: self.version,
            date_paquet: self.date_paquet,
            gesco_minimum: self.gesco_minimum,
            nom_fichier: self.nom_fichier,
            taille: self.taille,
            empreinte: self.empreinte_sha256,
            disciplines: serde_json::from_str(&self.disciplines)
                .map_err(|_| incoherent(t, "disciplines", &self.disciplines))?,
            actif: booleen(self.actif),
            installe_le: Horodatage::depuis_millis_epoque(self.installe_le),
        })
    }
}

const COLONNES_PAQUET: &str = "id, etablissement_id, code, version, date_paquet, gesco_minimum,
    nom_fichier, taille, empreinte_sha256, disciplines, actif, installe_le";

/// Paquets installés, remplacés compris, du plus ancien au plus récent.
pub async fn paquets(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Vec<PaquetInstalle>> {
    sqlx::query_as::<_, LignePaquet>(sqlx::AssertSqlSafe(format!(
        "SELECT {COLONNES_PAQUET} FROM bibliotheque_paquet WHERE etablissement_id = ?1
         ORDER BY installe_le, id"
    )))
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LignePaquet::vers_entite)
    .collect()
}

/// Enregistre un paquet installé : le paquet actif de même code passe à remplacé.
pub async fn enregistrer_paquet(
    cnx: &mut Connexion,
    p: &PaquetInstalle,
    contexte: &Contexte,
) -> Resultat<()> {
    let maintenant = contexte.maintenant.millis_epoque();
    sqlx::query(
        "UPDATE bibliotheque_paquet SET actif = 0, remplace_le = ?1, mis_a_jour_le = ?1,
            mis_a_jour_par = ?2, identifiant_poste = ?3
         WHERE etablissement_id = ?4 AND code = ?5 AND actif = 1",
    )
    .bind(maintenant)
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(&p.code)
    .execute(&mut *cnx)
    .await?;
    let disciplines = serde_json::to_string(&p.disciplines)
        .map_err(|e| ErreurBase::Integrite(format!("disciplines du paquet : {e}")))?;
    sqlx::query(
        "INSERT INTO bibliotheque_paquet (id, etablissement_id, code, version, date_paquet,
            gesco_minimum, nom_fichier, taille, empreinte_sha256, disciplines, actif,
            installe_le, installe_par, cree_le, cree_par, mis_a_jour_le, mis_a_jour_par,
            identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 1, ?11, ?12, ?11, ?12, ?11, ?12, ?13)",
    )
    .bind(p.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(&p.code)
    .bind(&p.version)
    .bind(&p.date_paquet)
    .bind(&p.gesco_minimum)
    .bind(&p.nom_fichier)
    .bind(p.taille)
    .bind(&p.empreinte)
    .bind(disciplines)
    .bind(maintenant)
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

// --- Chapitres ------------------------------------------------------------------------------

#[derive(FromRow)]
struct LigneChapitre {
    id: String,
    cle: String,
    origine: String,
    paquet_code: Option<String>,
    niveau_code: String,
    discipline_code: Option<String>,
    niveau_id: Option<String>,
    matiere_id: Option<String>,
    partie: String,
    titre: String,
    ordre: i64,
    apres_cle: Option<String>,
    a_confirmer: i64,
    plus_livre_le: Option<i64>,
    series: Option<String>,
}

impl LigneChapitre {
    fn vers_entite(self) -> Resultat<ChapitreBibliotheque> {
        let t = "chapitre_programme";
        Ok(ChapitreBibliotheque {
            id: lire_id(t, "id", &self.id)?,
            origine: OrigineBibliotheque::depuis_nom(&self.origine)
                .ok_or_else(|| incoherent(t, "origine", &self.origine))?,
            cle: self.cle,
            paquet_code: self.paquet_code,
            niveau_code: self.niveau_code,
            discipline_code: self.discipline_code,
            niveau_id: self.niveau_id,
            matiere_id: self.matiere_id,
            partie: self.partie,
            titre: self.titre,
            ordre: self.ordre,
            apres_cle: self.apres_cle,
            a_confirmer: booleen(self.a_confirmer),
            series: self
                .series
                .map(|s| s.split(',').map(str::to_owned).collect())
                .unwrap_or_default(),
            plus_livre: self.plus_livre_le.is_some(),
        })
    }
}

/// Chapitres de l'établissement, livrés et de l'école, « plus livrés » compris, avec leurs
/// séries en service.
pub async fn chapitres(
    cnx: &mut Connexion,
    contexte: &Contexte,
) -> Resultat<Vec<ChapitreBibliotheque>> {
    sqlx::query_as::<_, LigneChapitre>(
        "SELECT c.id, c.cle, c.origine, c.paquet_code, c.niveau_code, c.discipline_code,
                c.niveau_id, c.matiere_id, c.partie, c.titre, c.ordre, c.apres_cle,
                c.a_confirmer, c.plus_livre_le,
                (SELECT group_concat(s.serie_code, ',') FROM
                    (SELECT serie_code FROM chapitre_serie
                     WHERE chapitre_id = c.id AND retiree_le IS NULL ORDER BY serie_code) s)
                    AS series
         FROM chapitre_programme c WHERE c.etablissement_id = ?1
         ORDER BY c.niveau_code, c.ordre, c.id",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneChapitre::vers_entite)
    .collect()
}

/// Écrit un chapitre livré par sa clé : l'ajoute, ou le récrit s'il existe (titre, partie, ordre,
/// séries) et lève sa marque « plus livré ». Un chapitre de l'école de même clé n'est jamais
/// touché. Rend l'identifiant et vrai si le chapitre est nouveau.
pub async fn ecrire_chapitre_livre(
    cnx: &mut Connexion,
    c: &ChapitrePaquet,
    paquet_code: &str,
    contexte: &Contexte,
) -> Resultat<(String, bool)> {
    let etablissement = contexte.etablissement_id.vers_chaine();
    let maintenant = contexte.maintenant.millis_epoque();
    let existant: Option<(String, String)> = sqlx::query_as(
        "SELECT id, origine FROM chapitre_programme WHERE etablissement_id = ?1 AND cle = ?2",
    )
    .bind(&etablissement)
    .bind(&c.cle)
    .fetch_optional(&mut *cnx)
    .await?;
    let (id, nouveau) = match existant {
        Some((_, origine)) if origine != OrigineBibliotheque::Livre.nom() => {
            return Err(ErreurBase::Integrite(format!(
                "la clé {} est celle d'un chapitre de l'école",
                c.cle
            )));
        }
        Some((id, _)) => {
            sqlx::query(
                "UPDATE chapitre_programme SET paquet_code = ?1, niveau_code = ?2,
                    discipline_code = ?3, partie = ?4, titre = ?5, ordre = ?6, a_confirmer = ?7,
                    plus_livre_le = NULL, mis_a_jour_le = ?8, mis_a_jour_par = ?9,
                    identifiant_poste = ?10
                 WHERE id = ?11",
            )
            .bind(paquet_code)
            .bind(&c.niveau)
            .bind(&c.discipline)
            .bind(&c.partie)
            .bind(&c.titre)
            .bind(i64::from(c.ordre))
            .bind(i64::from(c.a_confirmer))
            .bind(maintenant)
            .bind(contexte.auteur())
            .bind(&contexte.poste.0)
            .bind(&id)
            .execute(&mut *cnx)
            .await?;
            (id, false)
        }
        None => {
            let id = nouvel_id::<ChapitreBibliotheque>().vers_chaine();
            sqlx::query(
                "INSERT INTO chapitre_programme (id, etablissement_id, cle, origine, paquet_code,
                    niveau_code, discipline_code, partie, titre, ordre, a_confirmer, cree_le,
                    cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
                 VALUES (?1, ?2, ?3, 'livre', ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?11, ?12,
                    ?13)",
            )
            .bind(&id)
            .bind(&etablissement)
            .bind(&c.cle)
            .bind(paquet_code)
            .bind(&c.niveau)
            .bind(&c.discipline)
            .bind(&c.partie)
            .bind(&c.titre)
            .bind(i64::from(c.ordre))
            .bind(i64::from(c.a_confirmer))
            .bind(maintenant)
            .bind(contexte.auteur())
            .bind(&contexte.poste.0)
            .execute(&mut *cnx)
            .await?;
            (id, true)
        }
    };
    ecrire_series(cnx, &id, &c.series, contexte).await?;
    Ok((id, nouveau))
}

/// Met les séries d'un chapitre à la liste donnée : une série nouvelle s'ajoute, une série
/// absente est marquée retirée, une série revenue est remise.
async fn ecrire_series(
    cnx: &mut Connexion,
    chapitre_id: &str,
    series: &[String],
    contexte: &Contexte,
) -> Resultat<()> {
    let maintenant = contexte.maintenant.millis_epoque();
    let existantes: Vec<(String, String, Option<i64>)> = sqlx::query_as(
        "SELECT id, serie_code, retiree_le FROM chapitre_serie WHERE chapitre_id = ?1",
    )
    .bind(chapitre_id)
    .fetch_all(&mut *cnx)
    .await?;
    for (id, code, retiree) in &existantes {
        let voulue = series.iter().any(|s| s == code);
        let a_changer = match (voulue, retiree) {
            (true, Some(_)) => Some(None),
            (false, None) => Some(Some(maintenant)),
            _ => None,
        };
        if let Some(retiree_le) = a_changer {
            sqlx::query(
                "UPDATE chapitre_serie SET retiree_le = ?1, mis_a_jour_le = ?2,
                    mis_a_jour_par = ?3, identifiant_poste = ?4 WHERE id = ?5",
            )
            .bind(retiree_le)
            .bind(maintenant)
            .bind(contexte.auteur())
            .bind(&contexte.poste.0)
            .bind(id)
            .execute(&mut *cnx)
            .await?;
        }
    }
    for serie in series {
        if existantes.iter().any(|(_, code, _)| code == serie) {
            continue;
        }
        sqlx::query(
            "INSERT INTO chapitre_serie (id, etablissement_id, chapitre_id, serie_code, cree_le,
                cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?5, ?6, ?7)",
        )
        .bind(nouvel_id::<ChapitreBibliotheque>().vers_chaine())
        .bind(contexte.etablissement_id.vers_chaine())
        .bind(chapitre_id)
        .bind(serie)
        .bind(maintenant)
        .bind(contexte.auteur())
        .bind(&contexte.poste.0)
        .execute(&mut *cnx)
        .await?;
    }
    Ok(())
}

// --- Contenus -------------------------------------------------------------------------------

#[derive(FromRow)]
struct LigneContenu {
    id: String,
    cle: String,
    origine: String,
    nature: String,
    chapitre_id: String,
    titre: String,
    presentation: String,
    ordre: i64,
    apres_cle: Option<String>,
    langue: String,
    lie_cle: Option<String>,
    examen: Option<String>,
    serie_code: Option<String>,
    session: Option<i64>,
    paquet_code: Option<String>,
    fichier: Option<String>,
    empreinte_sha256: Option<String>,
    faits: Option<String>,
    plus_livre_le: Option<i64>,
}

fn examen_depuis_nom(nom: &str) -> Option<ExamenSujet> {
    match nom {
        "def" => Some(ExamenSujet::Def),
        "bac" => Some(ExamenSujet::Bac),
        _ => None,
    }
}

fn nom_examen(examen: ExamenSujet) -> &'static str {
    match examen {
        ExamenSujet::Def => "def",
        ExamenSujet::Bac => "bac",
    }
}

impl LigneContenu {
    fn vers_entite(self) -> Resultat<ContenuBibliotheque> {
        let t = "contenu";
        Ok(ContenuBibliotheque {
            id: lire_id(t, "id", &self.id)?,
            origine: OrigineBibliotheque::depuis_nom(&self.origine)
                .ok_or_else(|| incoherent(t, "origine", &self.origine))?,
            nature: NatureContenu::depuis_nom(&self.nature)
                .ok_or_else(|| incoherent(t, "nature", &self.nature))?,
            chapitre_id: lire_id(t, "chapitre_id", &self.chapitre_id)?,
            examen: match self.examen.as_deref() {
                None => None,
                Some(e) => Some(examen_depuis_nom(e).ok_or_else(|| incoherent(t, "examen", e))?),
            },
            session: self.session.and_then(|s| u16::try_from(s).ok()),
            cle: self.cle,
            titre: self.titre,
            presentation: self.presentation,
            ordre: self.ordre,
            apres_cle: self.apres_cle,
            langue: self.langue,
            lie_cle: self.lie_cle,
            serie_code: self.serie_code,
            paquet_code: self.paquet_code,
            fichier: self.fichier,
            empreinte: self.empreinte_sha256,
            faits: self.faits,
            plus_livre: self.plus_livre_le.is_some(),
        })
    }
}

/// Contenus de l'établissement, livrés et de l'école, « plus livrés » compris.
pub async fn contenus(
    cnx: &mut Connexion,
    contexte: &Contexte,
) -> Resultat<Vec<ContenuBibliotheque>> {
    sqlx::query_as::<_, LigneContenu>(
        "SELECT id, cle, origine, nature, chapitre_id, titre, presentation, ordre, apres_cle,
                langue, lie_cle, examen, serie_code, session, paquet_code, fichier,
                empreinte_sha256, faits, plus_livre_le
         FROM contenu WHERE etablissement_id = ?1 ORDER BY chapitre_id, ordre, id",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneContenu::vers_entite)
    .collect()
}

/// Ce qu'une installation a fait d'un contenu livré.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EcritureContenu {
    /// Il n'existait pas.
    Nouveau,
    /// Son texte livré a changé.
    TexteChange,
    /// Son texte est le même (titre, rang ou chapitre ont pu changer).
    TextePareil,
}

/// Écrit un contenu livré par sa clé, avec l'empreinte de son texte livré ; lève sa marque
/// « plus livré ». Un contenu de l'école de même clé n'est jamais touché.
pub async fn ecrire_contenu_livre(
    cnx: &mut Connexion,
    c: &ContenuPaquet,
    chapitre_id: &str,
    paquet_code: &str,
    empreinte: &str,
    contexte: &Contexte,
) -> Resultat<EcritureContenu> {
    let etablissement = contexte.etablissement_id.vers_chaine();
    let maintenant = contexte.maintenant.millis_epoque();
    let existant: Option<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT id, origine, empreinte_sha256 FROM contenu
         WHERE etablissement_id = ?1 AND cle = ?2",
    )
    .bind(&etablissement)
    .bind(&c.cle)
    .fetch_optional(&mut *cnx)
    .await?;
    let examen = c.examen.map(nom_examen);
    let session = c.session.map(i64::from);
    match existant {
        Some((_, origine, _)) if origine != OrigineBibliotheque::Livre.nom() => {
            Err(ErreurBase::Integrite(format!(
                "la clé {} est celle d'un contenu de l'école",
                c.cle
            )))
        }
        Some((id, _, avant)) => {
            sqlx::query(
                "UPDATE contenu SET nature = ?1, chapitre_id = ?2, titre = ?3, ordre = ?4,
                    lie_cle = ?5, examen = ?6, session = ?7, paquet_code = ?8, fichier = ?9,
                    empreinte_sha256 = ?10, faits = ?11, plus_livre_le = NULL,
                    mis_a_jour_le = ?12, mis_a_jour_par = ?13, identifiant_poste = ?14
                 WHERE id = ?15",
            )
            .bind(c.nature.nom())
            .bind(chapitre_id)
            .bind(&c.titre)
            .bind(i64::from(c.ordre))
            .bind(c.lie.as_deref())
            .bind(examen)
            .bind(session)
            .bind(paquet_code)
            .bind(&c.fichier)
            .bind(empreinte)
            .bind(c.faits.as_deref())
            .bind(maintenant)
            .bind(contexte.auteur())
            .bind(&contexte.poste.0)
            .bind(&id)
            .execute(&mut *cnx)
            .await?;
            Ok(if avant.as_deref() == Some(empreinte) {
                EcritureContenu::TextePareil
            } else {
                EcritureContenu::TexteChange
            })
        }
        None => {
            sqlx::query(
                "INSERT INTO contenu (id, etablissement_id, cle, origine, nature, chapitre_id,
                    titre, ordre, lie_cle, examen, session, paquet_code, fichier,
                    empreinte_sha256, faits, cree_le, cree_par, mis_a_jour_le, mis_a_jour_par,
                    identifiant_poste)
                 VALUES (?1, ?2, ?3, 'livre', ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
                    ?15, ?16, ?15, ?16, ?17)",
            )
            .bind(nouvel_id::<ContenuBibliotheque>().vers_chaine())
            .bind(&etablissement)
            .bind(&c.cle)
            .bind(c.nature.nom())
            .bind(chapitre_id)
            .bind(&c.titre)
            .bind(i64::from(c.ordre))
            .bind(c.lie.as_deref())
            .bind(examen)
            .bind(session)
            .bind(paquet_code)
            .bind(&c.fichier)
            .bind(empreinte)
            .bind(c.faits.as_deref())
            .bind(maintenant)
            .bind(contexte.auteur())
            .bind(&contexte.poste.0)
            .execute(cnx)
            .await?;
            Ok(EcritureContenu::Nouveau)
        }
    }
}

/// Marque « plus livrés » les chapitres et contenus livrés d'un paquet qui n'y sont plus. Rend le
/// nombre de contenus marqués.
pub async fn marquer_plus_livres(
    cnx: &mut Connexion,
    paquet_code: &str,
    chapitres: &HashSet<&str>,
    contenus: &HashSet<&str>,
    contexte: &Contexte,
) -> Resultat<u32> {
    let etablissement = contexte.etablissement_id.vers_chaine();
    let maintenant = contexte.maintenant.millis_epoque();
    let mut marques = 0_u32;
    for (table, presentes) in [("chapitre_programme", chapitres), ("contenu", contenus)] {
        let livres: Vec<(String, String)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "SELECT id, cle FROM {table} WHERE etablissement_id = ?1 AND origine = 'livre'
                 AND paquet_code = ?2 AND plus_livre_le IS NULL"
        )))
        .bind(&etablissement)
        .bind(paquet_code)
        .fetch_all(&mut *cnx)
        .await?;
        for (id, cle) in livres {
            if presentes.contains(cle.as_str()) {
                continue;
            }
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "UPDATE {table} SET plus_livre_le = ?1, mis_a_jour_le = ?1, mis_a_jour_par = ?2,
                    identifiant_poste = ?3 WHERE id = ?4"
            )))
            .bind(maintenant)
            .bind(contexte.auteur())
            .bind(&contexte.poste.0)
            .bind(&id)
            .execute(&mut *cnx)
            .await?;
            if table == "contenu" {
                marques += 1;
            }
        }
    }
    Ok(marques)
}

// --- Correspondance des disciplines ---------------------------------------------------------

/// Pose la correspondance d'une discipline dans un niveau si l'école n'en a pas encore : une
/// correspondance existante, posée ou changée par la direction, n'est jamais réécrite.
pub async fn poser_correspondance_si_absente(
    cnx: &mut Connexion,
    discipline: &str,
    niveau_code: &str,
    matiere_id: Option<&str>,
    contexte: &Contexte,
) -> Resultat<bool> {
    let maintenant = contexte.maintenant.millis_epoque();
    let faite = sqlx::query(
        "INSERT INTO correspondance_discipline (id, etablissement_id, discipline_code,
            niveau_code, matiere_id, cree_le, cree_par, mis_a_jour_le, mis_a_jour_par,
            identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?6, ?7, ?8)
         ON CONFLICT (etablissement_id, discipline_code, niveau_code) DO NOTHING",
    )
    .bind(nouvel_id::<ContenuBibliotheque>().vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(discipline)
    .bind(niveau_code)
    .bind(matiere_id)
    .bind(maintenant)
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(faite.rows_affected() > 0)
}

/// Correspondances de l'établissement : (discipline, code du niveau, matière de l'école).
pub async fn correspondances(
    cnx: &mut Connexion,
    contexte: &Contexte,
) -> Resultat<Vec<(String, String, Option<String>)>> {
    Ok(sqlx::query_as(
        "SELECT discipline_code, niveau_code, matiere_id FROM correspondance_discipline
         WHERE etablissement_id = ?1 ORDER BY niveau_code, discipline_code",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?)
}

/// Matières de l'école : (code, identifiant, active).
pub async fn matieres_par_code(
    cnx: &mut Connexion,
    contexte: &Contexte,
) -> Resultat<Vec<(String, String, bool)>> {
    let lignes: Vec<(String, String, i64)> = sqlx::query_as(
        "SELECT code, id, active FROM matiere WHERE etablissement_id = ?1 ORDER BY ordre, code",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|(code, id, active)| (code, id, booleen(active)))
        .collect())
}

// --- Versions publiées et index -------------------------------------------------------------

/// Version publiée de chaque contenu qui en a une : (contenu, texte, retour à la version livrée).
pub async fn versions_publiees(
    cnx: &mut Connexion,
    contexte: &Contexte,
) -> Resultat<Vec<(String, String, bool)>> {
    let lignes: Vec<(String, String, i64)> = sqlx::query_as(
        "SELECT contenu_id, texte, retour_livre FROM version_contenu
         WHERE etablissement_id = ?1 AND etat = 'publiee'",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|(id, texte, retour)| (id, texte, booleen(retour)))
        .collect())
}

/// Vide l'index de recherche des contenus de l'établissement.
pub async fn vider_index(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<()> {
    sqlx::query(
        "DELETE FROM recherche_bibliotheque WHERE contenu_id IN
            (SELECT id FROM contenu WHERE etablissement_id = ?1)",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Écrit (ou réécrit) l'entrée d'un contenu dans l'index de recherche.
pub async fn indexer(
    cnx: &mut Connexion,
    contenu_id: &str,
    titre: &str,
    texte: &str,
) -> Resultat<()> {
    sqlx::query("DELETE FROM recherche_bibliotheque WHERE contenu_id = ?1")
        .bind(contenu_id)
        .execute(&mut *cnx)
        .await?;
    sqlx::query(
        "INSERT INTO recherche_bibliotheque (contenu_id, titre, texte) VALUES (?1, ?2, ?3)",
    )
    .bind(contenu_id)
    .bind(titre)
    .bind(texte)
    .execute(cnx)
    .await?;
    Ok(())
}

// --- Lecture --------------------------------------------------------------------------------

/// Versions de l'école qui comptent pour le catalogue : (contenu, état, retour à la version
/// livrée), pour les états publiée, soumise et brouillon.
pub async fn etats_versions(
    cnx: &mut Connexion,
    contexte: &Contexte,
) -> Resultat<Vec<(String, String, bool)>> {
    let lignes: Vec<(String, String, i64)> = sqlx::query_as(
        "SELECT contenu_id, etat, retour_livre FROM version_contenu
         WHERE etablissement_id = ?1 AND etat IN ('publiee', 'soumise', 'brouillon')",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|(id, etat, retour)| (id, etat, booleen(retour)))
        .collect())
}

/// Matières de l'utilisateur : (matière, code du niveau) des affectations de l'année courante
/// de l'enseignant lié à son compte.
pub async fn matieres_du_compte(
    cnx: &mut Connexion,
    utilisateur_id: &str,
    contexte: &Contexte,
) -> Resultat<Vec<(String, String)>> {
    Ok(sqlx::query_as(
        "SELECT DISTINCT a.matiere_id, n.code
         FROM affectation_enseignement a
         JOIN enseignant e ON e.id = a.enseignant_id
         JOIN classe c ON c.id = a.classe_id
         JOIN niveau n ON n.id = c.niveau_id
         JOIN annee_scolaire y ON y.id = a.annee_id
         WHERE e.utilisateur_id = ?1 AND e.actif = 1 AND a.etablissement_id = ?2
           AND y.est_courante = 1 AND c.supprime_le IS NULL",
    )
    .bind(utilisateur_id)
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?)
}

/// Un contenu de l'établissement.
pub async fn contenu(
    cnx: &mut Connexion,
    contenu_id: &str,
    contexte: &Contexte,
) -> Resultat<Option<ContenuBibliotheque>> {
    sqlx::query_as::<_, LigneContenu>(
        "SELECT id, cle, origine, nature, chapitre_id, titre, presentation, ordre, apres_cle,
                langue, lie_cle, examen, serie_code, session, paquet_code, fichier,
                empreinte_sha256, faits, plus_livre_le
         FROM contenu WHERE etablissement_id = ?1 AND id = ?2",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(contenu_id)
    .fetch_optional(cnx)
    .await?
    .map(LigneContenu::vers_entite)
    .transpose()
}

/// Version d'un contenu dans un état unique (`publiee` ou `brouillon`) : (texte, retour à la
/// version livrée).
pub async fn version_dans_etat(
    cnx: &mut Connexion,
    contenu_id: &str,
    etat: &str,
    contexte: &Contexte,
) -> Resultat<Option<(String, bool)>> {
    let ligne: Option<(String, i64)> = sqlx::query_as(
        "SELECT texte, retour_livre FROM version_contenu
         WHERE etablissement_id = ?1 AND contenu_id = ?2 AND etat = ?3",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(contenu_id)
    .bind(etat)
    .fetch_optional(cnx)
    .await?;
    Ok(ligne.map(|(texte, retour)| (texte, booleen(retour))))
}

/// Images de l'école appelées par leur empreinte : (empreinte, type MIME, contenu).
pub async fn images_par_empreinte(
    cnx: &mut Connexion,
    empreintes: &[String],
    contexte: &Contexte,
) -> Resultat<Vec<(String, String, Vec<u8>)>> {
    let mut images = Vec::new();
    for e in empreintes {
        let ligne: Option<(String, String, Vec<u8>)> = sqlx::query_as(
            "SELECT empreinte_sha256, type_mime, contenu FROM fichier_bibliotheque
             WHERE etablissement_id = ?1 AND empreinte_sha256 = ?2 AND nature = 'image'",
        )
        .bind(contexte.etablissement_id.vers_chaine())
        .bind(e)
        .fetch_optional(&mut *cnx)
        .await?;
        images.extend(ligne);
    }
    Ok(images)
}

/// Cherche dans l'index : (contenu, extrait), les meilleurs d'abord. L'extrait marque les mots
/// trouvés entre les caractères `\u{2}` et `\u{3}`. `requete` est déjà une requête FTS5 sûre.
pub async fn rechercher(
    cnx: &mut Connexion,
    requete: &str,
    limite: u32,
) -> Resultat<Vec<(String, String)>> {
    Ok(sqlx::query_as(
        "SELECT contenu_id,
                snippet(recherche_bibliotheque, 2, char(2), char(3), '…', 14)
         FROM recherche_bibliotheque WHERE recherche_bibliotheque MATCH ?1
         ORDER BY bm25(recherche_bibliotheque, 0.0, 6.0, 1.0) LIMIT ?2",
    )
    .bind(requete)
    .bind(i64::from(limite))
    .fetch_all(cnx)
    .await?)
}

// --- Retraits et correspondance -------------------------------------------------------------

/// Un retrait lu dans la base.
#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct LigneRetrait {
    /// Identifiant.
    pub id: String,
    /// Grain (nom persisté).
    pub grain: String,
    /// Cible.
    pub cible: String,
    /// Niveau d'une matière.
    pub niveau_code: Option<String>,
    /// Série d'une matière.
    pub serie_code: Option<String>,
    /// Retiré le.
    pub retire_le: i64,
    /// Nom de qui a retiré.
    pub retire_par: String,
    /// Remis le.
    pub remis_le: Option<i64>,
    /// Nom de qui a remis.
    pub remis_par: String,
}

/// Retraits de l'établissement ; `en_cours` : ceux qui ne sont pas remis.
pub async fn retraits(
    cnx: &mut Connexion,
    en_cours: bool,
    contexte: &Contexte,
) -> Resultat<Vec<LigneRetrait>> {
    Ok(sqlx::query_as::<_, LigneRetrait>(
        "SELECT r.id, r.grain, r.cible, r.niveau_code, r.serie_code, r.retire_le,
                COALESCE(u.nom_affiche, '') AS retire_par, r.remis_le,
                COALESCE(m.nom_affiche, '') AS remis_par
         FROM retrait_bibliotheque r LEFT JOIN utilisateur u ON u.id = r.retire_par
         LEFT JOIN utilisateur m ON m.id = r.remis_par
         WHERE r.etablissement_id = ?1 AND (?2 = 0 OR r.remis_le IS NULL)
         ORDER BY r.retire_le DESC, r.id DESC",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(i64::from(en_cours))
    .fetch_all(cnx)
    .await?)
}

/// Écrit un retrait.
pub async fn inserer_retrait(
    cnx: &mut Connexion,
    grain: &str,
    cible: &str,
    niveau_code: Option<&str>,
    serie_code: Option<&str>,
    contexte: &Contexte,
) -> Resultat<String> {
    let id = nouvel_id::<ContenuBibliotheque>().vers_chaine();
    let maintenant = contexte.maintenant.millis_epoque();
    sqlx::query(
        "INSERT INTO retrait_bibliotheque (id, etablissement_id, grain, cible, niveau_code,
            serie_code, retire_le, retire_par, cree_le, cree_par, mis_a_jour_le, mis_a_jour_par,
            identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?7, ?8, ?7, ?8, ?9)",
    )
    .bind(&id)
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(grain)
    .bind(cible)
    .bind(niveau_code)
    .bind(serie_code)
    .bind(maintenant)
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(id)
}

/// Remet un retrait en cours ; faux s'il n'existe pas ou s'il est déjà remis.
pub async fn remettre_retrait(
    cnx: &mut Connexion,
    id: &str,
    contexte: &Contexte,
) -> Resultat<bool> {
    let maintenant = contexte.maintenant.millis_epoque();
    let faite = sqlx::query(
        "UPDATE retrait_bibliotheque SET remis_le = ?1, remis_par = ?2, mis_a_jour_le = ?1,
            mis_a_jour_par = ?2, identifiant_poste = ?3
         WHERE id = ?4 AND etablissement_id = ?5 AND remis_le IS NULL",
    )
    .bind(maintenant)
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(id)
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(faite.rows_affected() > 0)
}

/// Règle la correspondance d'une discipline dans un niveau (la crée si elle manque). Rend la
/// matière d'avant.
pub async fn regler_correspondance(
    cnx: &mut Connexion,
    discipline: &str,
    niveau_code: &str,
    matiere_id: Option<&str>,
    contexte: &Contexte,
) -> Resultat<Option<String>> {
    let avant: Option<(Option<String>,)> = sqlx::query_as(
        "SELECT matiere_id FROM correspondance_discipline
         WHERE etablissement_id = ?1 AND discipline_code = ?2 AND niveau_code = ?3",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(discipline)
    .bind(niveau_code)
    .fetch_optional(&mut *cnx)
    .await?;
    let maintenant = contexte.maintenant.millis_epoque();
    sqlx::query(
        "INSERT INTO correspondance_discipline (id, etablissement_id, discipline_code,
            niveau_code, matiere_id, cree_le, cree_par, mis_a_jour_le, mis_a_jour_par,
            identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?6, ?7, ?8)
         ON CONFLICT (etablissement_id, discipline_code, niveau_code) DO UPDATE SET
            matiere_id = excluded.matiere_id, mis_a_jour_le = excluded.mis_a_jour_le,
            mis_a_jour_par = excluded.mis_a_jour_par,
            identifiant_poste = excluded.identifiant_poste",
    )
    .bind(nouvel_id::<ContenuBibliotheque>().vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(discipline)
    .bind(niveau_code)
    .bind(matiere_id)
    .bind(maintenant)
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(avant.and_then(|(m,)| m))
}

// --- Écriture : chapitres, contenus et versions de l'école ----------------------------------

/// Un chapitre ajouté par l'école.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NouveauChapitre<'a> {
    /// Clé stable (`ecole/<identifiant>`).
    pub cle: &'a str,
    /// Niveau de l'école.
    pub niveau_id: &'a str,
    /// Code du niveau.
    pub niveau_code: &'a str,
    /// Matière de l'école.
    pub matiere_id: &'a str,
    /// Partie (« Algèbre »…), vide si aucune.
    pub partie: &'a str,
    /// Titre.
    pub titre: &'a str,
    /// Clé du chapitre après lequel il se range ; aucune : en tête.
    pub apres_cle: Option<&'a str>,
    /// Séries du lycée ; aucune : toutes.
    pub series: &'a [String],
}

/// Écrit un chapitre de l'école ; rend son identifiant.
pub async fn inserer_chapitre_ecole(
    cnx: &mut Connexion,
    c: &NouveauChapitre<'_>,
    contexte: &Contexte,
) -> Resultat<String> {
    let id = nouvel_id::<ChapitreBibliotheque>().vers_chaine();
    let etablissement = contexte.etablissement_id.vers_chaine();
    let maintenant = contexte.maintenant.millis_epoque();
    let (ordre,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(MAX(ordre), 0) + 1 FROM chapitre_programme
         WHERE etablissement_id = ?1 AND origine = 'ecole'",
    )
    .bind(&etablissement)
    .fetch_one(&mut *cnx)
    .await?;
    sqlx::query(
        "INSERT INTO chapitre_programme (id, etablissement_id, cle, origine, niveau_code,
            niveau_id, matiere_id, partie, titre, ordre, apres_cle, cree_le, cree_par,
            mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, 'ecole', ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?11, ?12, ?13)",
    )
    .bind(&id)
    .bind(&etablissement)
    .bind(c.cle)
    .bind(c.niveau_code)
    .bind(c.niveau_id)
    .bind(c.matiere_id)
    .bind(c.partie)
    .bind(c.titre)
    .bind(ordre)
    .bind(c.apres_cle)
    .bind(maintenant)
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(&mut *cnx)
    .await?;
    ecrire_series(cnx, &id, c.series, contexte).await?;
    Ok(id)
}

/// Un contenu ajouté par l'école.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NouveauContenu<'a> {
    /// Clé stable (`ecole/<identifiant>`).
    pub cle: &'a str,
    /// Nature.
    pub nature: NatureContenu,
    /// Chapitre.
    pub chapitre_id: &'a str,
    /// Titre.
    pub titre: &'a str,
    /// Phrase de présentation d'un document.
    pub presentation: &'a str,
    /// Clé du contenu après lequel il se range ; aucune : en tête.
    pub apres_cle: Option<&'a str>,
    /// Clé du contenu lié (le sujet d'un corrigé).
    pub lie_cle: Option<&'a str>,
    /// Examen d'un sujet ou d'un corrigé.
    pub examen: Option<ExamenSujet>,
    /// Série au lycée.
    pub serie_code: Option<&'a str>,
    /// Session.
    pub session: Option<u16>,
}

/// Écrit un contenu de l'école ; rend son identifiant.
pub async fn inserer_contenu_ecole(
    cnx: &mut Connexion,
    c: &NouveauContenu<'_>,
    contexte: &Contexte,
) -> Resultat<String> {
    let id = nouvel_id::<ContenuBibliotheque>().vers_chaine();
    let etablissement = contexte.etablissement_id.vers_chaine();
    let maintenant = contexte.maintenant.millis_epoque();
    let (ordre,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(MAX(ordre), 0) + 1 FROM contenu
         WHERE etablissement_id = ?1 AND origine = 'ecole'",
    )
    .bind(&etablissement)
    .fetch_one(&mut *cnx)
    .await?;
    sqlx::query(
        "INSERT INTO contenu (id, etablissement_id, cle, origine, nature, chapitre_id, titre,
            presentation, ordre, apres_cle, lie_cle, examen, serie_code, session, cree_le,
            cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, 'ecole', ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?14,
            ?15, ?16)",
    )
    .bind(&id)
    .bind(&etablissement)
    .bind(c.cle)
    .bind(c.nature.nom())
    .bind(c.chapitre_id)
    .bind(c.titre)
    .bind(c.presentation)
    .bind(ordre)
    .bind(c.apres_cle)
    .bind(c.lie_cle)
    .bind(c.examen.map(nom_examen))
    .bind(c.serie_code)
    .bind(c.session.map(i64::from))
    .bind(maintenant)
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(id)
}

/// Lie un contenu de l'école à un autre par sa clé (le sujet à son corrigé). Un contenu livré
/// n'est jamais touché ; faux si le contenu n'est pas de l'école.
pub async fn lier_contenu_ecole(
    cnx: &mut Connexion,
    contenu_id: &str,
    lie_cle: &str,
    contexte: &Contexte,
) -> Resultat<bool> {
    let faite = sqlx::query(
        "UPDATE contenu SET lie_cle = ?1, mis_a_jour_le = ?2, mis_a_jour_par = ?3,
            identifiant_poste = ?4
         WHERE id = ?5 AND etablissement_id = ?6 AND origine = 'ecole'",
    )
    .bind(lie_cle)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(contenu_id)
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(faite.rows_affected() > 0)
}

/// Une version d'un contenu, avec le nom de son auteur et de qui a décidé.
#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct LigneVersion {
    /// Identifiant.
    pub id: String,
    /// Contenu.
    pub contenu_id: String,
    /// Numéro, à partir de 1.
    pub numero: i64,
    /// Texte Markdown (vide pour un retour à la version livrée).
    pub texte: String,
    /// Empreinte SHA-256 du texte.
    pub empreinte_sha256: String,
    /// État (nom persisté).
    pub etat: String,
    /// Retour à la version livrée (0 ou 1).
    pub retour_livre: i64,
    /// Version de l'école dont elle est partie.
    pub depart_version_id: Option<String>,
    /// Empreinte du texte livré dont elle est partie.
    pub depart_livre_sha256: Option<String>,
    /// Compteur de révision du brouillon.
    pub revision: i64,
    /// Auteur.
    pub auteur_id: String,
    /// Nom de l'auteur.
    pub auteur: String,
    /// Soumise le.
    pub soumise_le: Option<i64>,
    /// Décidée le.
    pub decidee_le: Option<i64>,
    /// Nom de qui a décidé.
    pub decidee_par: Option<String>,
    /// Commentaire d'un renvoi.
    pub commentaire: String,
    /// Créée le.
    pub cree_le: i64,
    /// Dernier enregistrement.
    pub mis_a_jour_le: i64,
}

const COLONNES_VERSION: &str = "v.id, v.contenu_id, v.numero, v.texte, v.empreinte_sha256,
    v.etat, v.retour_livre, v.depart_version_id, v.depart_livre_sha256, v.revision, v.auteur_id,
    COALESCE(a.nom_affiche, '') AS auteur, v.soumise_le, v.decidee_le,
    d.nom_affiche AS decidee_par, v.commentaire, v.cree_le, v.mis_a_jour_le
    FROM version_contenu v
    LEFT JOIN utilisateur a ON a.id = v.auteur_id
    LEFT JOIN utilisateur d ON d.id = v.decidee_par";

/// Versions d'un contenu, de la plus ancienne à la plus récente.
pub async fn versions(
    cnx: &mut Connexion,
    contenu_id: &str,
    contexte: &Contexte,
) -> Resultat<Vec<LigneVersion>> {
    Ok(
        sqlx::query_as::<_, LigneVersion>(sqlx::AssertSqlSafe(format!(
            "SELECT {COLONNES_VERSION}
         WHERE v.etablissement_id = ?1 AND v.contenu_id = ?2 ORDER BY v.numero"
        )))
        .bind(contexte.etablissement_id.vers_chaine())
        .bind(contenu_id)
        .fetch_all(cnx)
        .await?,
    )
}

/// Une version nouvelle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NouvelleVersion<'a> {
    /// Contenu.
    pub contenu_id: &'a str,
    /// Texte Markdown.
    pub texte: &'a str,
    /// Empreinte SHA-256 du texte.
    pub empreinte: &'a str,
    /// Retour à la version livrée (texte vide).
    pub retour_livre: bool,
    /// Version de l'école dont elle part.
    pub depart_version_id: Option<&'a str>,
    /// Empreinte du texte livré dont elle part.
    pub depart_livre_sha256: Option<&'a str>,
    /// Auteur.
    pub auteur_id: &'a str,
}

/// Écrit un brouillon nouveau, numéroté après la dernière version du contenu ; rend son
/// identifiant et son numéro. L'index unique refuse un second brouillon.
pub async fn inserer_brouillon(
    cnx: &mut Connexion,
    v: &NouvelleVersion<'_>,
    contexte: &Contexte,
) -> Resultat<(String, i64)> {
    let id = nouvel_id::<ContenuBibliotheque>().vers_chaine();
    let maintenant = contexte.maintenant.millis_epoque();
    let (numero,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(MAX(numero), 0) + 1 FROM version_contenu WHERE contenu_id = ?1",
    )
    .bind(v.contenu_id)
    .fetch_one(&mut *cnx)
    .await?;
    sqlx::query(
        "INSERT INTO version_contenu (id, etablissement_id, contenu_id, numero, texte,
            empreinte_sha256, etat, retour_livre, depart_version_id, depart_livre_sha256,
            revision, auteur_id, cree_le, cree_par, mis_a_jour_le, mis_a_jour_par,
            identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'brouillon', ?7, ?8, ?9, 1, ?10, ?11, ?12, ?11, ?12, ?13)",
    )
    .bind(&id)
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(v.contenu_id)
    .bind(numero)
    .bind(v.texte)
    .bind(v.empreinte)
    .bind(i64::from(v.retour_livre))
    .bind(v.depart_version_id)
    .bind(v.depart_livre_sha256)
    .bind(v.auteur_id)
    .bind(maintenant)
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok((id, numero))
}

/// Réécrit le texte d'un brouillon lu à la révision `revision` ; faux s'il n'est plus brouillon
/// ou s'il a été enregistré depuis. La révision avance d'un.
pub async fn reecrire_brouillon(
    cnx: &mut Connexion,
    version_id: &str,
    revision: i64,
    texte: &str,
    empreinte: &str,
    contexte: &Contexte,
) -> Resultat<bool> {
    let faite = sqlx::query(
        "UPDATE version_contenu SET texte = ?1, empreinte_sha256 = ?2, revision = revision + 1,
            mis_a_jour_le = ?3, mis_a_jour_par = ?4, identifiant_poste = ?5
         WHERE id = ?6 AND etablissement_id = ?7 AND etat = 'brouillon' AND revision = ?8",
    )
    .bind(texte)
    .bind(empreinte)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(version_id)
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(revision)
    .execute(cnx)
    .await?;
    Ok(faite.rows_affected() > 0)
}

/// Donne un brouillon à un autre auteur (« Reprendre ce brouillon ») ; la révision avance, pour
/// que l'éditeur de l'ancien auteur ne puisse plus enregistrer par-dessus.
pub async fn reprendre_brouillon(
    cnx: &mut Connexion,
    version_id: &str,
    auteur_id: &str,
    contexte: &Contexte,
) -> Resultat<bool> {
    let faite = sqlx::query(
        "UPDATE version_contenu SET auteur_id = ?1, revision = revision + 1,
            mis_a_jour_le = ?2, mis_a_jour_par = ?3, identifiant_poste = ?4
         WHERE id = ?5 AND etablissement_id = ?6 AND etat = 'brouillon'",
    )
    .bind(auteur_id)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(version_id)
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(faite.rows_affected() > 0)
}

/// Passe un brouillon à « abandonnée » ; faux s'il n'est plus brouillon.
pub async fn abandonner_brouillon(
    cnx: &mut Connexion,
    version_id: &str,
    contexte: &Contexte,
) -> Resultat<bool> {
    let maintenant = contexte.maintenant.millis_epoque();
    let faite = sqlx::query(
        "UPDATE version_contenu SET etat = 'abandonnee', decidee_le = ?1, decidee_par = ?2,
            mis_a_jour_le = ?1, mis_a_jour_par = ?2, identifiant_poste = ?3
         WHERE id = ?4 AND etablissement_id = ?5 AND etat = 'brouillon'",
    )
    .bind(maintenant)
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(version_id)
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(faite.rows_affected() > 0)
}

// --- Validation et droit de publier ---------------------------------------------------------

/// Soumet un brouillon à la direction ; faux s'il n'est plus brouillon.
pub async fn soumettre_brouillon(
    cnx: &mut Connexion,
    version_id: &str,
    contexte: &Contexte,
) -> Resultat<bool> {
    let maintenant = contexte.maintenant.millis_epoque();
    let faite = sqlx::query(
        "UPDATE version_contenu SET etat = 'soumise', soumise_le = ?1,
            mis_a_jour_le = ?1, mis_a_jour_par = ?2, identifiant_poste = ?3
         WHERE id = ?4 AND etablissement_id = ?5 AND etat = 'brouillon'",
    )
    .bind(maintenant)
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(version_id)
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(faite.rows_affected() > 0)
}

/// Ce que la publication d'une version a fait.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Publication {
    /// Publiée ; la version publiée précédente, s'il y en avait une, est passée à « remplacée ».
    Faite {
        /// Identifiant de la version remplacée.
        remplacee: Option<String>,
    },
    /// La version n'était plus brouillon ni soumise.
    Impossible,
}

/// Publie une version brouillon ou soumise : la version publiée précédente passe d'abord à
/// « remplacée » (une seule version publiée par contenu). L'appelant annule l'unité de travail
/// si la publication est impossible.
pub async fn publier_version(
    cnx: &mut Connexion,
    contenu_id: &str,
    version_id: &str,
    contexte: &Contexte,
) -> Resultat<Publication> {
    let maintenant = contexte.maintenant.millis_epoque();
    let etablissement = contexte.etablissement_id.vers_chaine();
    let precedente: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM version_contenu
         WHERE etablissement_id = ?1 AND contenu_id = ?2 AND etat = 'publiee'",
    )
    .bind(&etablissement)
    .bind(contenu_id)
    .fetch_optional(&mut *cnx)
    .await?;
    if let Some((id,)) = &precedente {
        sqlx::query(
            "UPDATE version_contenu SET etat = 'remplacee', mis_a_jour_le = ?1,
                mis_a_jour_par = ?2, identifiant_poste = ?3
             WHERE id = ?4 AND etat = 'publiee'",
        )
        .bind(maintenant)
        .bind(contexte.auteur())
        .bind(&contexte.poste.0)
        .bind(id)
        .execute(&mut *cnx)
        .await?;
    }
    let faite = sqlx::query(
        "UPDATE version_contenu SET etat = 'publiee', decidee_le = ?1, decidee_par = ?2,
            mis_a_jour_le = ?1, mis_a_jour_par = ?2, identifiant_poste = ?3
         WHERE id = ?4 AND etablissement_id = ?5 AND contenu_id = ?6
           AND etat IN ('brouillon', 'soumise')",
    )
    .bind(maintenant)
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(version_id)
    .bind(&etablissement)
    .bind(contenu_id)
    .execute(cnx)
    .await?;
    if faite.rows_affected() == 0 {
        return Ok(Publication::Impossible);
    }
    Ok(Publication::Faite {
        remplacee: precedente.map(|(id,)| id),
    })
}

/// Renvoie une version soumise avec un commentaire ; faux si elle n'est plus soumise.
pub async fn renvoyer_version(
    cnx: &mut Connexion,
    version_id: &str,
    commentaire: &str,
    contexte: &Contexte,
) -> Resultat<bool> {
    let maintenant = contexte.maintenant.millis_epoque();
    let faite = sqlx::query(
        "UPDATE version_contenu SET etat = 'renvoyee', commentaire = ?1, decidee_le = ?2,
            decidee_par = ?3, mis_a_jour_le = ?2, mis_a_jour_par = ?3, identifiant_poste = ?4
         WHERE id = ?5 AND etablissement_id = ?6 AND etat = 'soumise'",
    )
    .bind(commentaire)
    .bind(maintenant)
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(version_id)
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(faite.rows_affected() > 0)
}

/// Versions soumises de l'établissement, les plus anciennes d'abord.
pub async fn versions_soumises(
    cnx: &mut Connexion,
    contexte: &Contexte,
) -> Resultat<Vec<LigneVersion>> {
    Ok(
        sqlx::query_as::<_, LigneVersion>(sqlx::AssertSqlSafe(format!(
            "SELECT {COLONNES_VERSION}
         WHERE v.etablissement_id = ?1 AND v.etat = 'soumise' ORDER BY v.soumise_le, v.numero"
        )))
        .bind(contexte.etablissement_id.vers_chaine())
        .fetch_all(cnx)
        .await?,
    )
}

/// Enlève l'entrée d'un contenu de l'index de recherche (il n'a plus de texte visible).
pub async fn desindexer(cnx: &mut Connexion, contenu_id: &str) -> Resultat<()> {
    sqlx::query("DELETE FROM recherche_bibliotheque WHERE contenu_id = ?1")
        .bind(contenu_id)
        .execute(cnx)
        .await?;
    Ok(())
}

/// Un droit de publier en cours, avec les noms.
#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct LigneDroitPublication {
    /// Compte du maître.
    pub utilisateur_id: String,
    /// Nom du maître.
    pub nom: String,
    /// Donné le.
    pub donne_le: i64,
    /// Nom de qui l'a donné.
    pub donne_par: String,
}

/// Droits de publier en cours, par nom.
pub async fn droits_publication(
    cnx: &mut Connexion,
    contexte: &Contexte,
) -> Resultat<Vec<LigneDroitPublication>> {
    Ok(sqlx::query_as(
        "SELECT d.utilisateur_id, COALESCE(u.nom_affiche, '') AS nom, d.donne_le,
            COALESCE(p.nom_affiche, '') AS donne_par
         FROM droit_publication d
         LEFT JOIN utilisateur u ON u.id = d.utilisateur_id
         LEFT JOIN utilisateur p ON p.id = d.donne_par
         WHERE d.etablissement_id = ?1 AND d.retire_le IS NULL
         ORDER BY u.nom_affiche",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?)
}

/// Vrai si le compte a le droit de publier, donné nommément et pas retiré.
pub async fn a_le_droit_de_publier(
    cnx: &mut Connexion,
    utilisateur_id: &str,
    contexte: &Contexte,
) -> Resultat<bool> {
    let (n,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM droit_publication
         WHERE etablissement_id = ?1 AND utilisateur_id = ?2 AND retire_le IS NULL",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(utilisateur_id)
    .fetch_one(cnx)
    .await?;
    Ok(n > 0)
}

/// Comptes actifs liés à un enseignant actif : (compte, nom), par nom.
pub async fn maitres_avec_compte(
    cnx: &mut Connexion,
    contexte: &Contexte,
) -> Resultat<Vec<(String, String)>> {
    Ok(sqlx::query_as(
        "SELECT u.id, u.nom_affiche FROM enseignant e
         JOIN utilisateur u ON u.id = e.utilisateur_id
         WHERE e.etablissement_id = ?1 AND e.actif = 1 AND u.actif = 1
           AND u.supprime_le IS NULL
         ORDER BY u.nom_affiche",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?)
}

/// Donne le droit de publier à un compte ; rend l'identifiant de la ligne. L'index unique
/// refuse un second droit en cours.
pub async fn donner_droit_publication(
    cnx: &mut Connexion,
    utilisateur_id: &str,
    contexte: &Contexte,
) -> Resultat<String> {
    let id = nouvel_id::<ContenuBibliotheque>().vers_chaine();
    let maintenant = contexte.maintenant.millis_epoque();
    sqlx::query(
        "INSERT INTO droit_publication (id, etablissement_id, utilisateur_id, donne_le,
            donne_par, cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?4, ?5, ?4, ?5, ?6)",
    )
    .bind(&id)
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(utilisateur_id)
    .bind(maintenant)
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(id)
}

/// Retire le droit de publier d'un compte ; rend l'identifiant de la ligne, ou `None` s'il n'en
/// avait pas.
pub async fn retirer_droit_publication(
    cnx: &mut Connexion,
    utilisateur_id: &str,
    contexte: &Contexte,
) -> Resultat<Option<String>> {
    let maintenant = contexte.maintenant.millis_epoque();
    let ligne: Option<(String,)> = sqlx::query_as(
        "UPDATE droit_publication SET retire_le = ?1, retire_par = ?2, mis_a_jour_le = ?1,
            mis_a_jour_par = ?2, identifiant_poste = ?3
         WHERE etablissement_id = ?4 AND utilisateur_id = ?5 AND retire_le IS NULL
         RETURNING id",
    )
    .bind(maintenant)
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(utilisateur_id)
    .fetch_optional(cnx)
    .await?;
    Ok(ligne.map(|(id,)| id))
}

// --- Fichiers et vidéos de l'école ----------------------------------------------------------

/// Un fichier joint à un document de l'école.
#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct FichierJoint {
    /// Ligne qui joint le fichier au document.
    pub lien_id: String,
    /// Empreinte SHA-256 du fichier.
    pub empreinte_sha256: String,
    /// Nom du fichier.
    pub nom_fichier: String,
    /// Type MIME.
    pub type_mime: String,
    /// Taille en octets.
    pub taille: i64,
}

/// Une vidéo de l'école.
#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct VideoEcole {
    /// Empreinte SHA-256, nom du fichier dans `videos`.
    pub empreinte_sha256: String,
    /// Nom du fichier d'origine.
    pub nom_fichier: String,
    /// Taille en octets.
    pub taille: i64,
}

/// Range un fichier dans la base par son empreinte, une seule fois ; rend son identifiant et
/// vrai s'il est nouveau.
#[allow(clippy::too_many_arguments)]
pub async fn enregistrer_fichier(
    cnx: &mut Connexion,
    nature: &str,
    type_mime: &str,
    nom_fichier: &str,
    empreinte: &str,
    octets: &[u8],
    contexte: &Contexte,
) -> Resultat<(String, bool)> {
    let deja: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM fichier_bibliotheque
         WHERE etablissement_id = ?1 AND empreinte_sha256 = ?2",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(empreinte)
    .fetch_optional(&mut *cnx)
    .await?;
    if let Some((id,)) = deja {
        return Ok((id, false));
    }
    let id = nouvel_id::<ContenuBibliotheque>().vers_chaine();
    let maintenant = contexte.maintenant.millis_epoque();
    sqlx::query(
        "INSERT INTO fichier_bibliotheque (id, etablissement_id, empreinte_sha256, nature,
            type_mime, nom_fichier, taille, contenu, cree_le, cree_par, mis_a_jour_le,
            mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?9, ?10, ?11)",
    )
    .bind(&id)
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(empreinte)
    .bind(nature)
    .bind(type_mime)
    .bind(nom_fichier)
    .bind(i64::try_from(octets.len()).unwrap_or(i64::MAX))
    .bind(octets)
    .bind(maintenant)
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok((id, true))
}

/// Un fichier de l'école par son empreinte : (nom, type MIME, contenu).
pub async fn fichier_par_empreinte(
    cnx: &mut Connexion,
    empreinte: &str,
    contexte: &Contexte,
) -> Resultat<Option<(String, String, Vec<u8>)>> {
    Ok(sqlx::query_as(
        "SELECT nom_fichier, type_mime, contenu FROM fichier_bibliotheque
         WHERE etablissement_id = ?1 AND empreinte_sha256 = ?2",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(empreinte)
    .fetch_optional(cnx)
    .await?)
}

/// Fichiers d'un document, dans l'ordre, sans ceux qui en ont été retirés.
pub async fn fichiers_du_contenu(
    cnx: &mut Connexion,
    contenu_id: &str,
    contexte: &Contexte,
) -> Resultat<Vec<FichierJoint>> {
    Ok(sqlx::query_as(
        "SELECT fc.id AS lien_id, f.empreinte_sha256, f.nom_fichier, f.type_mime, f.taille
         FROM fichier_contenu fc JOIN fichier_bibliotheque f ON f.id = fc.fichier_id
         WHERE fc.etablissement_id = ?1 AND fc.contenu_id = ?2 AND fc.retire_le IS NULL
         ORDER BY fc.ordre, fc.cree_le",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(contenu_id)
    .fetch_all(cnx)
    .await?)
}

/// Joint un fichier à un document, en dernier ; rend l'identifiant de la ligne.
pub async fn joindre_fichier(
    cnx: &mut Connexion,
    contenu_id: &str,
    fichier_id: &str,
    contexte: &Contexte,
) -> Resultat<String> {
    let id = nouvel_id::<ContenuBibliotheque>().vers_chaine();
    let maintenant = contexte.maintenant.millis_epoque();
    sqlx::query(
        "INSERT INTO fichier_contenu (id, etablissement_id, contenu_id, fichier_id, ordre,
            cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4,
            (SELECT COALESCE(MAX(ordre), 0) + 1 FROM fichier_contenu
             WHERE etablissement_id = ?2 AND contenu_id = ?3),
            ?5, ?6, ?5, ?6, ?7)",
    )
    .bind(&id)
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(contenu_id)
    .bind(fichier_id)
    .bind(maintenant)
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(id)
}

/// Retire un fichier d'un document (la ligne reste) ; faux s'il n'y était plus.
pub async fn retirer_fichier_joint(
    cnx: &mut Connexion,
    contenu_id: &str,
    lien_id: &str,
    contexte: &Contexte,
) -> Resultat<bool> {
    let maintenant = contexte.maintenant.millis_epoque();
    let fait = sqlx::query(
        "UPDATE fichier_contenu SET retire_le = ?1, mis_a_jour_le = ?1, mis_a_jour_par = ?2,
            identifiant_poste = ?3
         WHERE etablissement_id = ?4 AND contenu_id = ?5 AND id = ?6 AND retire_le IS NULL",
    )
    .bind(maintenant)
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(contenu_id)
    .bind(lien_id)
    .execute(cnx)
    .await?;
    Ok(fait.rows_affected() > 0)
}

/// Écrit une vidéo connue de l'école, une seule fois par empreinte ; vrai si elle est nouvelle.
pub async fn enregistrer_video(
    cnx: &mut Connexion,
    empreinte: &str,
    nom_fichier: &str,
    taille: i64,
    contexte: &Contexte,
) -> Resultat<bool> {
    let id = nouvel_id::<ContenuBibliotheque>().vers_chaine();
    let maintenant = contexte.maintenant.millis_epoque();
    let fait = sqlx::query(
        "INSERT INTO video_bibliotheque (id, etablissement_id, empreinte_sha256, nom_fichier,
            taille, cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?6, ?7, ?8)
         ON CONFLICT (etablissement_id, empreinte_sha256) DO NOTHING",
    )
    .bind(&id)
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(empreinte)
    .bind(nom_fichier)
    .bind(taille)
    .bind(maintenant)
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(fait.rows_affected() > 0)
}

/// Vidéos connues de l'école.
pub async fn videos(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Vec<VideoEcole>> {
    Ok(sqlx::query_as(
        "SELECT empreinte_sha256, nom_fichier, taille FROM video_bibliotheque
         WHERE etablissement_id = ?1 ORDER BY cree_le",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?)
}
