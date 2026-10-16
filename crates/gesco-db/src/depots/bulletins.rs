//! Appréciations et bulletins (SPEC_PHASE_4 §7). Un bulletin ne se supprime jamais (R-14).

use gesco_core::commun::{Horodatage, Id};
use gesco_core::core::MachineAEtats;
use gesco_core::pedagogie::calcul::Rang;
use gesco_core::pedagogie::{Bulletin, ContenuBulletin, MoyenneReprise, Periode, StatutBulletin};
use gesco_core::scolarite::{Classe, Inscription};
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::depots::horodatage_optionnel;
use crate::erreur::{ErreurBase, Resultat};
use crate::id::lire_id;

/// Clé de l'appréciation générale, à la place d'une matière.
pub const APPRECIATION_GENERALE: &str = "generale";

#[derive(FromRow)]
struct LigneBulletinBase {
    id: String,
    etablissement_id: String,
    inscription_id: String,
    periode_id: String,
    classe_id: String,
    version: i64,
    statut: String,
    contenu: String,
    regle_version: i64,
    motif_correction: String,
    calcule_le: i64,
    publie_le: Option<i64>,
}

impl LigneBulletinBase {
    fn vers_entite(self) -> Resultat<Bulletin> {
        let t = "bulletin";
        Ok(Bulletin {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            inscription_id: lire_id(t, "inscription_id", &self.inscription_id)?,
            periode_id: lire_id(t, "periode_id", &self.periode_id)?,
            classe_id: lire_id(t, "classe_id", &self.classe_id)?,
            version: self.version,
            statut: StatutBulletin::depuis_nom(&self.statut)
                .ok_or_else(|| ErreurBase::illisible(t, "statut", &self.statut))?,
            contenu: serde_json::from_str::<ContenuBulletin>(&self.contenu)?,
            regle_version: self.regle_version,
            motif_correction: self.motif_correction,
            calcule_le: Horodatage::depuis_millis_epoque(self.calcule_le),
            publie_le: horodatage_optionnel(self.publie_le),
        })
    }
}

macro_rules! selectionner_bulletin {
    ($suite:literal) => {
        concat!(
            "SELECT id, etablissement_id, inscription_id, periode_id, classe_id, version, statut, contenu,
                regle_version, motif_correction, calcule_le, publie_le FROM bulletin ",
            $suite
        )
    };
}

fn rang_en_colonnes(rang: Option<Rang>) -> (Option<i64>, i64) {
    rang.map_or((None, 0), |r| {
        (Some(i64::from(r.rang)), i64::from(r.ex_aequo))
    })
}

/// Bulletins d'une classe pour une période, toutes versions, par élève puis par version.
pub async fn de_la_classe(
    cnx: &mut Connexion,
    classe_id: Id<Classe>,
    periode_id: Id<Periode>,
    contexte: &Contexte,
) -> Resultat<Vec<Bulletin>> {
    sqlx::query_as::<_, LigneBulletinBase>(selectionner_bulletin!(
        "WHERE classe_id = ?1 AND periode_id = ?2 AND etablissement_id = ?3
         ORDER BY inscription_id, version"
    ))
    .bind(classe_id.vers_chaine())
    .bind(periode_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneBulletinBase::vers_entite)
    .collect()
}

/// Bulletins d'un élève, toutes périodes et versions.
pub async fn de_l_inscription(
    cnx: &mut Connexion,
    inscription_id: Id<Inscription>,
    contexte: &Contexte,
) -> Resultat<Vec<Bulletin>> {
    sqlx::query_as::<_, LigneBulletinBase>(selectionner_bulletin!(
        "WHERE inscription_id = ?1 AND etablissement_id = ?2 ORDER BY periode_id, version"
    ))
    .bind(inscription_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneBulletinBase::vers_entite)
    .collect()
}

/// Lit un bulletin.
pub async fn lire(
    cnx: &mut Connexion,
    id: Id<Bulletin>,
    contexte: &Contexte,
) -> Resultat<Bulletin> {
    sqlx::query_as::<_, LigneBulletinBase>(selectionner_bulletin!(
        "WHERE id = ?1 AND etablissement_id = ?2"
    ))
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .ok_or_else(|| ErreurBase::Introuvable {
        entite: "bulletin",
        id: id.vers_chaine(),
    })?
    .vers_entite()
}

/// Enregistre une nouvelle version de bulletin.
pub async fn inserer(cnx: &mut Connexion, b: &Bulletin, contexte: &Contexte) -> Resultat<()> {
    if b.etablissement_id != contexte.etablissement_id {
        return Err(ErreurBase::Cloisonnement {
            entite: "bulletin",
            id: b.id.vers_chaine(),
        });
    }
    let (rang, ex_aequo) = rang_en_colonnes(b.contenu.rang);
    sqlx::query(
        "INSERT INTO bulletin (id, etablissement_id, inscription_id, periode_id, classe_id, version, statut, contenu,
            moyenne, rang, ex_aequo, regle_version, motif_correction, calcule_le, calcule_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
    )
    .bind(b.id.vers_chaine())
    .bind(b.etablissement_id.vers_chaine())
    .bind(b.inscription_id.vers_chaine())
    .bind(b.periode_id.vers_chaine())
    .bind(b.classe_id.vers_chaine())
    .bind(b.version)
    .bind(b.statut.nom())
    .bind(serde_json::to_string(&b.contenu)?)
    .bind(b.contenu.moyenne)
    .bind(rang)
    .bind(ex_aequo)
    .bind(b.regle_version)
    .bind(&b.motif_correction)
    .bind(b.calcule_le.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Récrit le contenu d'un bulletin non publié (recalcul, appréciation), et son état.
pub async fn recrire(cnx: &mut Connexion, b: &Bulletin, contexte: &Contexte) -> Resultat<()> {
    let (rang, ex_aequo) = rang_en_colonnes(b.contenu.rang);
    sqlx::query(
        "UPDATE bulletin SET statut = ?1, contenu = ?2, moyenne = ?3, rang = ?4, ex_aequo = ?5, regle_version = ?6,
            calcule_le = ?7, calcule_par = ?8, identifiant_poste = ?9
         WHERE id = ?10 AND etablissement_id = ?11 AND statut IN ('calcule', 'valide')",
    )
    .bind(b.statut.nom())
    .bind(serde_json::to_string(&b.contenu)?)
    .bind(b.contenu.moyenne)
    .bind(rang)
    .bind(ex_aequo)
    .bind(b.regle_version)
    .bind(b.calcule_le.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(b.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Change l'état d'un bulletin et note qui l'a validé ou publié.
pub async fn ecrire_statut(
    cnx: &mut Connexion,
    id: Id<Bulletin>,
    statut: StatutBulletin,
    contexte: &Contexte,
) -> Resultat<()> {
    let requete = match statut {
        StatutBulletin::Valide => sqlx::query(
            "UPDATE bulletin SET statut = ?1, valide_le = ?2, valide_par = ?3, identifiant_poste = ?4
             WHERE id = ?5 AND etablissement_id = ?6",
        ),
        StatutBulletin::Publie => sqlx::query(
            "UPDATE bulletin SET statut = ?1, publie_le = ?2, publie_par = ?3, identifiant_poste = ?4
             WHERE id = ?5 AND etablissement_id = ?6",
        ),
        // Revenu au calcul ou remplacé : seul l'état change, l'horodatage reste dans l'audit.
        StatutBulletin::Calcule | StatutBulletin::Remplace => sqlx::query(
            "UPDATE bulletin SET statut = ?1, mis_a_jour_le = ?2, mis_a_jour_par = ?3, identifiant_poste = ?4
             WHERE id = ?5 AND etablissement_id = ?6",
        ),
    };
    requete
        .bind(statut.nom())
        .bind(contexte.maintenant.millis_epoque())
        .bind(contexte.auteur())
        .bind(&contexte.poste.0)
        .bind(id.vers_chaine())
        .bind(contexte.etablissement_id.vers_chaine())
        .execute(cnx)
        .await?;
    Ok(())
}

/// Appréciations d'une période : (inscription, matière ou « generale », texte).
pub async fn appreciations(
    cnx: &mut Connexion,
    periode_id: Id<Periode>,
    contexte: &Contexte,
) -> Resultat<Vec<(Id<Inscription>, String, String)>> {
    let lignes: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT inscription_id, matiere_cle, texte FROM appreciation
         WHERE periode_id = ?1 AND etablissement_id = ?2",
    )
    .bind(periode_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?;
    lignes
        .into_iter()
        .map(|(i, m, t)| Ok((lire_id("appreciation", "inscription_id", &i)?, m, t)))
        .collect()
}

/// Écrit une appréciation ; un texte vide la vide.
pub async fn ecrire_appreciation(
    cnx: &mut Connexion,
    inscription_id: Id<Inscription>,
    periode_id: Id<Periode>,
    matiere_cle: &str,
    texte: &str,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "INSERT INTO appreciation (inscription_id, periode_id, matiere_cle, etablissement_id, texte, saisie_le,
            saisie_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT (inscription_id, periode_id, matiere_cle) DO UPDATE SET texte = excluded.texte,
            saisie_le = excluded.saisie_le, saisie_par = excluded.saisie_par,
            identifiant_poste = excluded.identifiant_poste",
    )
    .bind(inscription_id.vers_chaine())
    .bind(periode_id.vers_chaine())
    .bind(matiere_cle)
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(texte)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Moyennes reprises en vigueur des élèves d'une liste d'inscriptions, toutes périodes.
pub async fn moyennes_reprises(
    cnx: &mut Connexion,
    inscriptions: &[Id<Inscription>],
    contexte: &Contexte,
) -> Resultat<Vec<MoyenneReprise>> {
    let mut reprises = Vec::new();
    for inscription_id in inscriptions {
        let lignes: Vec<(String, i64, String, String)> = sqlx::query_as(
            "SELECT periode_id, moyenne, etablissement_origine, piece FROM moyenne_reprise
             WHERE inscription_id = ?1 AND etablissement_id = ?2 AND retiree = 0",
        )
        .bind(inscription_id.vers_chaine())
        .bind(contexte.etablissement_id.vers_chaine())
        .fetch_all(&mut *cnx)
        .await?;
        for (periode_id, moyenne, etablissement_origine, piece) in lignes {
            reprises.push(MoyenneReprise {
                inscription_id: *inscription_id,
                periode_id: lire_id("moyenne_reprise", "periode_id", &periode_id)?,
                moyenne: i32::try_from(moyenne).map_err(|_| {
                    ErreurBase::illisible("moyenne_reprise", "moyenne", moyenne.to_string())
                })?,
                etablissement_origine,
                piece,
            });
        }
    }
    Ok(reprises)
}

/// Écrit la moyenne reprise d'un élève pour une période, ou la retire (`None`) sans la supprimer.
pub async fn ecrire_moyenne_reprise(
    cnx: &mut Connexion,
    inscription_id: Id<Inscription>,
    periode_id: Id<Periode>,
    reprise: Option<&MoyenneReprise>,
    contexte: &Contexte,
) -> Resultat<()> {
    let (moyenne, origine, piece, retiree) = match reprise {
        Some(r) => (
            r.moyenne,
            r.etablissement_origine.as_str(),
            r.piece.as_str(),
            0_i64,
        ),
        None => (0, "", "", 1),
    };
    sqlx::query(
        "INSERT INTO moyenne_reprise (inscription_id, periode_id, etablissement_id, moyenne, etablissement_origine,
            piece, retiree, saisie_le, saisie_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT (inscription_id, periode_id) DO UPDATE SET moyenne = excluded.moyenne,
            etablissement_origine = excluded.etablissement_origine, piece = excluded.piece,
            retiree = excluded.retiree, saisie_le = excluded.saisie_le, saisie_par = excluded.saisie_par,
            identifiant_poste = excluded.identifiant_poste",
    )
    .bind(inscription_id.vers_chaine())
    .bind(periode_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(moyenne)
    .bind(origine)
    .bind(piece)
    .bind(retiree)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}
