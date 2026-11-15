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
}

/// Retraits de l'établissement ; `en_cours` : ceux qui ne sont pas remis.
pub async fn retraits(
    cnx: &mut Connexion,
    en_cours: bool,
    contexte: &Contexte,
) -> Resultat<Vec<LigneRetrait>> {
    Ok(sqlx::query_as::<_, LigneRetrait>(
        "SELECT r.id, r.grain, r.cible, r.niveau_code, r.serie_code, r.retire_le,
                COALESCE(u.nom_affiche, '') AS retire_par, r.remis_le
         FROM retrait_bibliotheque r LEFT JOIN utilisateur u ON u.id = r.retire_par
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
