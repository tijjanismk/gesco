//! Livres des comptes de trésorerie (SPEC_PHASE_8 §5) : les versements qui entrent, les
//! transferts, les coches du rapprochement, le compte choisi pour chaque mode.

use gesco_core::commun::{DateCivile, Horodatage, Id, Montant};
use gesco_core::finance::ModePaiement;
use gesco_core::tresorerie::CompteTresorerie;
use gesco_core::tresorerie::livre::{OrigineLigne, Transfert};
use gesco_core::tresorerie::releve::FormatReleve;
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::erreur::{ErreurBase, Resultat};
use crate::id::lire_id;

/// Versement d'un parent qui compte dans un livre : valide, ou annulé après coup dans une autre
/// session (la régularisation le sort alors de la caisse).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersementEntrant {
    /// Identifiant.
    pub id: String,
    /// Instant de l'encaissement.
    pub le: Horodatage,
    /// Montant.
    pub montant: Montant,
    /// Mode.
    pub mode: ModePaiement,
    /// Référence de la transaction.
    pub reference: String,
    /// Numéro du reçu.
    pub numero_recu: String,
    /// Qui a payé.
    pub payeur: String,
}

/// Versements qui comptent dans les livres.
pub async fn versements_entrants(
    cnx: &mut Connexion,
    contexte: &Contexte,
) -> Resultat<Vec<VersementEntrant>> {
    let lignes: Vec<(String, i64, i64, String, String, String, String)> = sqlx::query_as(
        "SELECT v.id, v.encaisse_le, v.montant, v.mode, v.reference, v.numero_recu, v.payeur_nom
         FROM versement v
         WHERE v.etablissement_id = ?1 AND (v.statut = 'valide'
            OR EXISTS (SELECT 1 FROM mouvement_caisse m WHERE m.versement_id = v.id))
         ORDER BY v.encaisse_le",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?;
    lignes
        .into_iter()
        .map(|(id, le, montant, mode, reference, numero_recu, payeur)| {
            Ok(VersementEntrant {
                le: Horodatage::depuis_millis_epoque(le),
                montant: Montant::depuis_francs(montant),
                mode: ModePaiement::depuis_nom(&mode)
                    .ok_or_else(|| ErreurBase::illisible("versement", "mode", &mode))?,
                id,
                reference,
                numero_recu,
                payeur,
            })
        })
        .collect()
}

#[derive(FromRow)]
struct LigneTransfert {
    id: String,
    etablissement_id: String,
    date: String,
    source_id: String,
    destination_id: String,
    montant: i64,
    reference: String,
    motif: String,
    session_id: Option<String>,
    fait_par: String,
}

impl LigneTransfert {
    fn vers_entite(self) -> Resultat<Transfert> {
        let t = "transfert_tresorerie";
        Ok(Transfert {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            date: DateCivile::analyser(&self.date)
                .ok_or_else(|| ErreurBase::illisible(t, "date", &self.date))?,
            source_id: lire_id(t, "source_id", &self.source_id)?,
            destination_id: lire_id(t, "destination_id", &self.destination_id)?,
            montant: Montant::depuis_francs(self.montant),
            reference: self.reference,
            motif: self.motif,
            session_id: self
                .session_id
                .as_deref()
                .map(|s| lire_id(t, "session_id", s))
                .transpose()?,
            fait_par: lire_id(t, "fait_par", &self.fait_par)?,
        })
    }
}

/// Transferts de l'école, du plus ancien au plus récent.
pub async fn transferts(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Vec<Transfert>> {
    sqlx::query_as::<_, LigneTransfert>(
        "SELECT id, etablissement_id, date, source_id, destination_id, montant, reference, motif,
            session_id, fait_par
         FROM transfert_tresorerie WHERE etablissement_id = ?1 ORDER BY date, cree_le",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneTransfert::vers_entite)
    .collect()
}

/// Enregistre un transfert.
pub async fn inserer_transfert(
    cnx: &mut Connexion,
    t: &Transfert,
    contexte: &Contexte,
) -> Resultat<()> {
    if t.etablissement_id != contexte.etablissement_id {
        return Err(ErreurBase::Cloisonnement {
            entite: "transfert_tresorerie",
            id: t.id.vers_chaine(),
        });
    }
    sqlx::query(
        "INSERT INTO transfert_tresorerie (id, etablissement_id, date, source_id, destination_id,
            montant, reference, motif, session_id, fait_par, cree_le, cree_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
    )
    .bind(t.id.vers_chaine())
    .bind(t.etablissement_id.vers_chaine())
    .bind(t.date.to_string())
    .bind(t.source_id.vers_chaine())
    .bind(t.destination_id.vers_chaine())
    .bind(t.montant.francs())
    .bind(&t.reference)
    .bind(&t.motif)
    .bind(t.session_id.map(|s| s.vers_chaine()))
    .bind(t.fait_par.vers_chaine())
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Lignes cochées d'un compte : origine et identifiant.
pub async fn pointees(
    cnx: &mut Connexion,
    compte_id: Id<CompteTresorerie>,
    contexte: &Contexte,
) -> Resultat<Vec<(OrigineLigne, String)>> {
    let lignes: Vec<(String, String)> = sqlx::query_as(
        "SELECT origine, origine_id FROM pointage_tresorerie
         WHERE etablissement_id = ?1 AND compte_id = ?2 AND pointee = 1",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(compte_id.vers_chaine())
    .fetch_all(cnx)
    .await?;
    lignes
        .into_iter()
        .map(|(origine, id)| {
            OrigineLigne::depuis_nom(&origine)
                .map(|o| (o, id))
                .ok_or_else(|| ErreurBase::illisible("pointage_tresorerie", "origine", &origine))
        })
        .collect()
}

/// Coche ou décoche une ligne du livre d'un compte.
pub async fn pointer(
    cnx: &mut Connexion,
    id_si_nouveau: &str,
    compte_id: Id<CompteTresorerie>,
    origine: OrigineLigne,
    origine_id: &str,
    pointee: bool,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "INSERT INTO pointage_tresorerie (id, etablissement_id, compte_id, origine, origine_id,
            pointee, cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?7, ?8, ?9)
         ON CONFLICT (compte_id, origine, origine_id) DO UPDATE SET pointee = excluded.pointee,
            mis_a_jour_le = excluded.mis_a_jour_le, mis_a_jour_par = excluded.mis_a_jour_par,
            identifiant_poste = excluded.identifiant_poste",
    )
    .bind(id_si_nouveau)
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(compte_id.vers_chaine())
    .bind(origine.nom())
    .bind(origine_id)
    .bind(i64::from(pointee))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Compte choisi pour chaque mode.
pub async fn comptes_des_modes(
    cnx: &mut Connexion,
    contexte: &Contexte,
) -> Resultat<Vec<(ModePaiement, Id<CompteTresorerie>)>> {
    let lignes: Vec<(String, String)> =
        sqlx::query_as("SELECT mode, compte_id FROM compte_du_mode WHERE etablissement_id = ?1")
            .bind(contexte.etablissement_id.vers_chaine())
            .fetch_all(cnx)
            .await?;
    lignes
        .into_iter()
        .map(|(mode, compte)| {
            Ok((
                ModePaiement::depuis_nom(&mode)
                    .ok_or_else(|| ErreurBase::illisible("compte_du_mode", "mode", &mode))?,
                lire_id("compte_du_mode", "compte_id", &compte)?,
            ))
        })
        .collect()
}

/// Choisit le compte d'un mode.
pub async fn choisir_compte_du_mode(
    cnx: &mut Connexion,
    mode: ModePaiement,
    compte_id: Id<CompteTresorerie>,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "INSERT INTO compte_du_mode (etablissement_id, mode, compte_id, mis_a_jour_le,
            mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT (etablissement_id, mode) DO UPDATE SET compte_id = excluded.compte_id,
            mis_a_jour_le = excluded.mis_a_jour_le, mis_a_jour_par = excluded.mis_a_jour_par,
            identifiant_poste = excluded.identifiant_poste",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(mode.nom())
    .bind(compte_id.vers_chaine())
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Format du relevé d'un compte, s'il a été dit.
pub async fn format_releve(
    cnx: &mut Connexion,
    compte_id: Id<CompteTresorerie>,
    contexte: &Contexte,
) -> Resultat<Option<FormatReleve>> {
    let ligne: Option<(String,)> = sqlx::query_as(
        "SELECT contenu FROM format_releve WHERE compte_id = ?1 AND etablissement_id = ?2",
    )
    .bind(compte_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?;
    ligne
        .map(|(contenu,)| {
            serde_json::from_str(&contenu)
                .map_err(|_| ErreurBase::illisible("format_releve", "contenu", &contenu))
        })
        .transpose()
}

/// Garde le format du relevé d'un compte.
pub async fn garder_format_releve(
    cnx: &mut Connexion,
    compte_id: Id<CompteTresorerie>,
    format: &FormatReleve,
    contexte: &Contexte,
) -> Resultat<()> {
    let contenu = serde_json::to_string(format)
        .map_err(|_| ErreurBase::illisible("format_releve", "contenu", "format"))?;
    sqlx::query(
        "INSERT INTO format_releve (compte_id, etablissement_id, contenu, mis_a_jour_le,
            mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT (compte_id) DO UPDATE SET contenu = excluded.contenu,
            mis_a_jour_le = excluded.mis_a_jour_le, mis_a_jour_par = excluded.mis_a_jour_par,
            identifiant_poste = excluded.identifiant_poste",
    )
    .bind(compte_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(contenu)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}
