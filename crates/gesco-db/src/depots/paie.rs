//! Paie (SPEC_PHASE_7) : éléments de paie de l'école, versions datées de leur calcul,
//! attributions à un membre ; paie du mois, ses bulletins, ses saisies et ses absences retenues.

use gesco_core::commun::{DateCivile, Horodatage, Id, Montant};
use gesco_core::core::{MachineAEtats, Utilisateur};
use gesco_core::finance::ModePaiement;
use gesco_core::paie::{
    AbsenceConfirmee, Attribution, Avance, BulletinPaie, ElementPaie, NatureElement, ObjetPaiement,
    PaieMois, PaiementPersonnel, SaisieMois, SourcePaiement, StatutAvance, StatutPaie,
    VersionElement,
};
use gesco_core::rh::{MembrePersonnel, Mois};
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::depots::{booleen, horodatage_optionnel};
use crate::erreur::{ErreurBase, Resultat};
use crate::id::lire_id;

fn cloisonne<T>(entite: &'static str, id: Id<T>, etablissement: bool) -> Resultat<()> {
    if etablissement {
        Ok(())
    } else {
        Err(ErreurBase::Cloisonnement {
            entite,
            id: id.vers_chaine(),
        })
    }
}

fn mois(table: &'static str, colonne: &'static str, texte: &str) -> Resultat<Mois> {
    Mois::analyser(texte).ok_or_else(|| ErreurBase::illisible(table, colonne, texte))
}

#[derive(FromRow)]
struct LigneElement {
    id: String,
    etablissement_id: String,
    code: Option<String>,
    libelle: String,
    nature: String,
    natures: String,
    fonctionnaires: i64,
    ordre: i64,
    actif: i64,
}

impl LigneElement {
    fn vers_entite(self) -> Resultat<ElementPaie> {
        let t = "element_paie";
        Ok(ElementPaie {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            code: self.code,
            libelle: self.libelle,
            nature: NatureElement::depuis_nom(&self.nature)
                .ok_or_else(|| ErreurBase::illisible(t, "nature", &self.nature))?,
            natures: serde_json::from_str(&self.natures)?,
            fonctionnaires: booleen(self.fonctionnaires),
            ordre: self.ordre,
            actif: booleen(self.actif),
        })
    }
}

/// Éléments de paie de l'établissement, dans l'ordre du bulletin.
pub async fn elements(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Vec<ElementPaie>> {
    sqlx::query_as::<_, LigneElement>(
        "SELECT id, etablissement_id, code, libelle, nature, natures, fonctionnaires, ordre, actif
         FROM element_paie WHERE etablissement_id = ?1 ORDER BY ordre, libelle",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneElement::vers_entite)
    .collect()
}

/// Enregistre un élément.
pub async fn inserer_element(
    cnx: &mut Connexion,
    e: &ElementPaie,
    contexte: &Contexte,
) -> Resultat<()> {
    cloisonne(
        "element_paie",
        e.id,
        e.etablissement_id == contexte.etablissement_id,
    )?;
    sqlx::query(
        "INSERT INTO element_paie (id, etablissement_id, code, libelle, nature, natures,
            fonctionnaires, ordre, actif, cree_le, cree_par, mis_a_jour_le, mis_a_jour_par,
            identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?10, ?11, ?12)",
    )
    .bind(e.id.vers_chaine())
    .bind(e.etablissement_id.vers_chaine())
    .bind(e.code.as_deref())
    .bind(&e.libelle)
    .bind(e.nature.nom())
    .bind(serde_json::to_string(&e.natures)?)
    .bind(i64::from(e.fonctionnaires))
    .bind(e.ordre)
    .bind(i64::from(e.actif))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Récrit le libellé, les natures concernées, l'ordre et l'état d'un élément ; sa nature et son
/// code ne changent pas.
pub async fn ecrire_element(
    cnx: &mut Connexion,
    e: &ElementPaie,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE element_paie SET libelle = ?1, natures = ?2, fonctionnaires = ?3, ordre = ?4,
            actif = ?5, mis_a_jour_le = ?6, mis_a_jour_par = ?7, identifiant_poste = ?8
         WHERE id = ?9 AND etablissement_id = ?10",
    )
    .bind(&e.libelle)
    .bind(serde_json::to_string(&e.natures)?)
    .bind(i64::from(e.fonctionnaires))
    .bind(e.ordre)
    .bind(i64::from(e.actif))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(e.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

#[derive(FromRow)]
struct LigneVersion {
    id: String,
    etablissement_id: String,
    element_id: String,
    effet: String,
    calcul: String,
    assiette_deduit: String,
}

impl LigneVersion {
    fn vers_entite(self) -> Resultat<VersionElement> {
        let t = "version_element";
        Ok(VersionElement {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            element_id: lire_id(t, "element_id", &self.element_id)?,
            effet: mois(t, "effet", &self.effet)?,
            calcul: serde_json::from_str(&self.calcul)?,
            assiette_deduit: serde_json::from_str(&self.assiette_deduit)?,
        })
    }
}

/// Versions de tous les éléments, par élément puis par effet.
pub async fn versions(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Vec<VersionElement>> {
    sqlx::query_as::<_, LigneVersion>(
        "SELECT id, etablissement_id, element_id, effet, calcul, assiette_deduit
         FROM version_element WHERE etablissement_id = ?1 ORDER BY element_id, effet",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneVersion::vers_entite)
    .collect()
}

/// Enregistre une version.
pub async fn inserer_version(
    cnx: &mut Connexion,
    v: &VersionElement,
    contexte: &Contexte,
) -> Resultat<()> {
    cloisonne(
        "version_element",
        v.id,
        v.etablissement_id == contexte.etablissement_id,
    )?;
    sqlx::query(
        "INSERT INTO version_element (id, etablissement_id, element_id, effet, calcul,
            assiette_deduit, cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?7, ?8, ?9)",
    )
    .bind(v.id.vers_chaine())
    .bind(v.etablissement_id.vers_chaine())
    .bind(v.element_id.vers_chaine())
    .bind(v.effet.to_string())
    .bind(serde_json::to_string(&v.calcul)?)
    .bind(serde_json::to_string(&v.assiette_deduit)?)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Récrit le calcul d'une version qu'aucune paie validée n'a appliquée.
pub async fn ecrire_version(
    cnx: &mut Connexion,
    v: &VersionElement,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE version_element SET calcul = ?1, assiette_deduit = ?2,
            mis_a_jour_le = ?3, mis_a_jour_par = ?4, identifiant_poste = ?5
         WHERE id = ?6 AND etablissement_id = ?7",
    )
    .bind(serde_json::to_string(&v.calcul)?)
    .bind(serde_json::to_string(&v.assiette_deduit)?)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(v.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

#[derive(FromRow)]
struct LigneAttribution {
    id: String,
    etablissement_id: String,
    membre_id: String,
    element_id: String,
    montant: i64,
    debut: String,
    fin: Option<String>,
}

impl LigneAttribution {
    fn vers_entite(self) -> Resultat<Attribution> {
        let t = "attribution_element";
        Ok(Attribution {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            membre_id: lire_id(t, "membre_id", &self.membre_id)?,
            element_id: lire_id(t, "element_id", &self.element_id)?,
            montant: Montant::depuis_francs(self.montant),
            debut: mois(t, "debut", &self.debut)?,
            fin: self.fin.as_deref().map(|f| mois(t, "fin", f)).transpose()?,
        })
    }
}

/// Attributions de l'établissement, par membre puis par début.
pub async fn attributions(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Vec<Attribution>> {
    sqlx::query_as::<_, LigneAttribution>(
        "SELECT id, etablissement_id, membre_id, element_id, montant, debut, fin
         FROM attribution_element WHERE etablissement_id = ?1 ORDER BY membre_id, debut",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneAttribution::vers_entite)
    .collect()
}

/// Enregistre une attribution.
pub async fn inserer_attribution(
    cnx: &mut Connexion,
    a: &Attribution,
    contexte: &Contexte,
) -> Resultat<()> {
    cloisonne(
        "attribution_element",
        a.id,
        a.etablissement_id == contexte.etablissement_id,
    )?;
    sqlx::query(
        "INSERT INTO attribution_element (id, etablissement_id, membre_id, element_id, montant,
            debut, fin, cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?8, ?9, ?10)",
    )
    .bind(a.id.vers_chaine())
    .bind(a.etablissement_id.vers_chaine())
    .bind(a.membre_id.vers_chaine())
    .bind(a.element_id.vers_chaine())
    .bind(a.montant.francs())
    .bind(a.debut.to_string())
    .bind(a.fin.map(|f| f.to_string()))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Récrit le montant et la période d'une attribution.
pub async fn ecrire_attribution(
    cnx: &mut Connexion,
    a: &Attribution,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE attribution_element SET montant = ?1, debut = ?2, fin = ?3,
            mis_a_jour_le = ?4, mis_a_jour_par = ?5, identifiant_poste = ?6
         WHERE id = ?7 AND etablissement_id = ?8",
    )
    .bind(a.montant.francs())
    .bind(a.debut.to_string())
    .bind(a.fin.map(|f| f.to_string()))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(a.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

// --- Paie du mois (SPEC_PHASE_7 §5) ---

#[derive(FromRow)]
struct LignePaie {
    id: String,
    etablissement_id: String,
    mois: String,
    statut: String,
    calculee_le: i64,
    calculee_par: Option<String>,
    validee_le: Option<i64>,
    validee_par: Option<String>,
}

impl LignePaie {
    fn vers_entite(self) -> Resultat<PaieMois> {
        let t = "paie_mois";
        Ok(PaieMois {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            mois: mois(t, "mois", &self.mois)?,
            statut: StatutPaie::depuis_nom(&self.statut)
                .ok_or_else(|| ErreurBase::illisible(t, "statut", &self.statut))?,
            calculee_le: Horodatage::depuis_millis_epoque(self.calculee_le),
            calculee_par: self
                .calculee_par
                .as_deref()
                .map(|u| lire_id::<Utilisateur>(t, "calculee_par", u))
                .transpose()?,
            validee_le: horodatage_optionnel(self.validee_le),
            validee_par: self
                .validee_par
                .as_deref()
                .map(|u| lire_id::<Utilisateur>(t, "validee_par", u))
                .transpose()?,
        })
    }
}

/// Paies de l'école, de la plus ancienne à la plus récente.
pub async fn paies(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Vec<PaieMois>> {
    sqlx::query_as::<_, LignePaie>(
        "SELECT id, etablissement_id, mois, statut, calculee_le, calculee_par, validee_le, validee_par
         FROM paie_mois WHERE etablissement_id = ?1 ORDER BY mois",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LignePaie::vers_entite)
    .collect()
}

/// Paie d'un mois, s'il y en a une.
pub async fn paie_du_mois(
    cnx: &mut Connexion,
    m: Mois,
    contexte: &Contexte,
) -> Resultat<Option<PaieMois>> {
    sqlx::query_as::<_, LignePaie>(
        "SELECT id, etablissement_id, mois, statut, calculee_le, calculee_par, validee_le, validee_par
         FROM paie_mois WHERE etablissement_id = ?1 AND mois = ?2",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(m.to_string())
    .fetch_optional(cnx)
    .await?
    .map(LignePaie::vers_entite)
    .transpose()
}

/// Enregistre la paie d'un mois.
pub async fn inserer_paie(cnx: &mut Connexion, p: &PaieMois, contexte: &Contexte) -> Resultat<()> {
    cloisonne(
        "paie_mois",
        p.id,
        p.etablissement_id == contexte.etablissement_id,
    )?;
    sqlx::query(
        "INSERT INTO paie_mois (id, etablissement_id, mois, statut, calculee_le, calculee_par,
            validee_le, validee_par, cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?9, ?10, ?11)",
    )
    .bind(p.id.vers_chaine())
    .bind(p.etablissement_id.vers_chaine())
    .bind(p.mois.to_string())
    .bind(p.statut.nom())
    .bind(p.calculee_le.millis_epoque())
    .bind(p.calculee_par.map(|u| u.vers_chaine()))
    .bind(p.validee_le.map(|h| h.millis_epoque()))
    .bind(p.validee_par.map(|u| u.vers_chaine()))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Récrit l'état et les dates de calcul et de validation d'une paie.
pub async fn ecrire_paie(cnx: &mut Connexion, p: &PaieMois, contexte: &Contexte) -> Resultat<()> {
    sqlx::query(
        "UPDATE paie_mois SET statut = ?1, calculee_le = ?2, calculee_par = ?3, validee_le = ?4,
            validee_par = ?5, mis_a_jour_le = ?6, mis_a_jour_par = ?7, identifiant_poste = ?8
         WHERE id = ?9 AND etablissement_id = ?10",
    )
    .bind(p.statut.nom())
    .bind(p.calculee_le.millis_epoque())
    .bind(p.calculee_par.map(|u| u.vers_chaine()))
    .bind(p.validee_le.map(|h| h.millis_epoque()))
    .bind(p.validee_par.map(|u| u.vers_chaine()))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(p.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

#[derive(FromRow)]
struct LigneBulletinPaie {
    id: String,
    etablissement_id: String,
    paie_id: String,
    membre_id: String,
    numero: Option<String>,
    contenu: String,
    exclu: i64,
}

impl LigneBulletinPaie {
    fn vers_entite(self) -> Resultat<BulletinPaie> {
        let t = "bulletin_paie";
        Ok(BulletinPaie {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            paie_id: lire_id(t, "paie_id", &self.paie_id)?,
            membre_id: lire_id(t, "membre_id", &self.membre_id)?,
            numero: self.numero,
            resultat: serde_json::from_str(&self.contenu)?,
            exclu: booleen(self.exclu),
        })
    }
}

/// Bulletins d'une paie.
pub async fn bulletins_de_la_paie(
    cnx: &mut Connexion,
    paie_id: Id<PaieMois>,
    contexte: &Contexte,
) -> Resultat<Vec<BulletinPaie>> {
    sqlx::query_as::<_, LigneBulletinPaie>(
        "SELECT id, etablissement_id, paie_id, membre_id, numero, contenu, exclu
         FROM bulletin_paie WHERE paie_id = ?1 AND etablissement_id = ?2 ORDER BY cree_le, id",
    )
    .bind(paie_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneBulletinPaie::vers_entite)
    .collect()
}

/// Bulletins validés d'un membre, avec le mois de leur paie, du plus ancien au plus récent.
pub async fn bulletins_valides_du_membre(
    cnx: &mut Connexion,
    membre_id: Id<MembrePersonnel>,
    contexte: &Contexte,
) -> Resultat<Vec<(Mois, BulletinPaie)>> {
    #[derive(FromRow)]
    struct Ligne {
        mois: String,
        #[sqlx(flatten)]
        bulletin: LigneBulletinPaie,
    }
    sqlx::query_as::<_, Ligne>(
        "SELECT p.mois, b.id, b.etablissement_id, b.paie_id, b.membre_id, b.numero, b.contenu, b.exclu
         FROM bulletin_paie b JOIN paie_mois p ON p.id = b.paie_id
         WHERE b.membre_id = ?1 AND b.etablissement_id = ?2 AND p.statut = 'validee' AND b.exclu = 0
         ORDER BY p.mois",
    )
    .bind(membre_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(|l| Ok((mois("paie_mois", "mois", &l.mois)?, l.bulletin.vers_entite()?)))
    .collect()
}

/// Enregistre un bulletin.
pub async fn inserer_bulletin(
    cnx: &mut Connexion,
    b: &BulletinPaie,
    contexte: &Contexte,
) -> Resultat<()> {
    cloisonne(
        "bulletin_paie",
        b.id,
        b.etablissement_id == contexte.etablissement_id,
    )?;
    sqlx::query(
        "INSERT INTO bulletin_paie (id, etablissement_id, paie_id, membre_id, numero, contenu, brut,
            net, exclu, cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?10, ?11, ?12)",
    )
    .bind(b.id.vers_chaine())
    .bind(b.etablissement_id.vers_chaine())
    .bind(b.paie_id.vers_chaine())
    .bind(b.membre_id.vers_chaine())
    .bind(&b.numero)
    .bind(serde_json::to_string(&b.resultat)?)
    .bind(b.resultat.brut.francs())
    .bind(b.resultat.net.francs())
    .bind(i64::from(b.exclu))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Récrit le calcul d'un bulletin, son exclusion et son numéro.
pub async fn ecrire_bulletin(
    cnx: &mut Connexion,
    b: &BulletinPaie,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE bulletin_paie SET numero = ?1, contenu = ?2, brut = ?3, net = ?4, exclu = ?5,
            mis_a_jour_le = ?6, mis_a_jour_par = ?7, identifiant_poste = ?8
         WHERE id = ?9 AND etablissement_id = ?10",
    )
    .bind(&b.numero)
    .bind(serde_json::to_string(&b.resultat)?)
    .bind(b.resultat.brut.francs())
    .bind(b.resultat.net.francs())
    .bind(i64::from(b.exclu))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(b.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Donne son numéro au bulletin d'une paie que l'on valide.
pub async fn numeroter_bulletin(
    cnx: &mut Connexion,
    id: Id<BulletinPaie>,
    numero: &str,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE bulletin_paie SET numero = ?1, mis_a_jour_le = ?2, mis_a_jour_par = ?3,
            identifiant_poste = ?4
         WHERE id = ?5 AND etablissement_id = ?6",
    )
    .bind(numero)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

#[derive(FromRow)]
struct LigneSaisie {
    id: String,
    etablissement_id: String,
    paie_id: String,
    membre_id: String,
    element_id: String,
    montant: i64,
    libelle: String,
    retiree: i64,
}

impl LigneSaisie {
    fn vers_entite(self) -> Resultat<SaisieMois> {
        let t = "saisie_paie";
        Ok(SaisieMois {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            paie_id: lire_id(t, "paie_id", &self.paie_id)?,
            membre_id: lire_id(t, "membre_id", &self.membre_id)?,
            element_id: lire_id(t, "element_id", &self.element_id)?,
            montant: Montant::depuis_francs(self.montant),
            libelle: self.libelle,
            retiree: booleen(self.retiree),
        })
    }
}

/// Montants saisis dans une paie, retirés compris.
pub async fn saisies_de_la_paie(
    cnx: &mut Connexion,
    paie_id: Id<PaieMois>,
    contexte: &Contexte,
) -> Resultat<Vec<SaisieMois>> {
    sqlx::query_as::<_, LigneSaisie>(
        "SELECT id, etablissement_id, paie_id, membre_id, element_id, montant, libelle, retiree
         FROM saisie_paie WHERE paie_id = ?1 AND etablissement_id = ?2 ORDER BY cree_le, id",
    )
    .bind(paie_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneSaisie::vers_entite)
    .collect()
}

/// Un montant saisi.
pub async fn saisie(
    cnx: &mut Connexion,
    id: Id<SaisieMois>,
    contexte: &Contexte,
) -> Resultat<SaisieMois> {
    sqlx::query_as::<_, LigneSaisie>(
        "SELECT id, etablissement_id, paie_id, membre_id, element_id, montant, libelle, retiree
         FROM saisie_paie WHERE id = ?1 AND etablissement_id = ?2",
    )
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .ok_or_else(|| ErreurBase::Introuvable {
        entite: "saisie_paie",
        id: id.vers_chaine(),
    })?
    .vers_entite()
}

/// Enregistre un montant saisi.
pub async fn inserer_saisie(
    cnx: &mut Connexion,
    s: &SaisieMois,
    contexte: &Contexte,
) -> Resultat<()> {
    cloisonne(
        "saisie_paie",
        s.id,
        s.etablissement_id == contexte.etablissement_id,
    )?;
    sqlx::query(
        "INSERT INTO saisie_paie (id, etablissement_id, paie_id, membre_id, element_id, montant,
            libelle, retiree, cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?9, ?10, ?11)",
    )
    .bind(s.id.vers_chaine())
    .bind(s.etablissement_id.vers_chaine())
    .bind(s.paie_id.vers_chaine())
    .bind(s.membre_id.vers_chaine())
    .bind(s.element_id.vers_chaine())
    .bind(s.montant.francs())
    .bind(&s.libelle)
    .bind(i64::from(s.retiree))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Retire un montant saisi : il reste, sans compter.
pub async fn retirer_saisie(
    cnx: &mut Connexion,
    id: Id<SaisieMois>,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE saisie_paie SET retiree = 1, mis_a_jour_le = ?1, mis_a_jour_par = ?2,
            identifiant_poste = ?3
         WHERE id = ?4 AND etablissement_id = ?5",
    )
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

#[derive(FromRow)]
struct LigneConfirmation {
    id: String,
    etablissement_id: String,
    paie_id: String,
    absence_id: String,
    retenue: i64,
}

impl LigneConfirmation {
    fn vers_entite(self) -> Resultat<AbsenceConfirmee> {
        let t = "absence_confirmee";
        Ok(AbsenceConfirmee {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            paie_id: lire_id(t, "paie_id", &self.paie_id)?,
            absence_id: lire_id(t, "absence_id", &self.absence_id)?,
            retenue: booleen(self.retenue),
        })
    }
}

/// Décisions de la direction sur les absences d'une paie.
pub async fn confirmations(
    cnx: &mut Connexion,
    paie_id: Id<PaieMois>,
    contexte: &Contexte,
) -> Resultat<Vec<AbsenceConfirmee>> {
    sqlx::query_as::<_, LigneConfirmation>(
        "SELECT id, etablissement_id, paie_id, absence_id, retenue
         FROM absence_confirmee WHERE paie_id = ?1 AND etablissement_id = ?2",
    )
    .bind(paie_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneConfirmation::vers_entite)
    .collect()
}

/// Pose ou change la décision sur une absence.
pub async fn poser_confirmation(
    cnx: &mut Connexion,
    c: &AbsenceConfirmee,
    contexte: &Contexte,
) -> Resultat<()> {
    cloisonne(
        "absence_confirmee",
        c.id,
        c.etablissement_id == contexte.etablissement_id,
    )?;
    sqlx::query(
        "INSERT INTO absence_confirmee (id, etablissement_id, paie_id, absence_id, retenue, cree_le,
            cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?6, ?7, ?8)
         ON CONFLICT (paie_id, absence_id) DO UPDATE SET retenue = excluded.retenue,
            mis_a_jour_le = excluded.mis_a_jour_le, mis_a_jour_par = excluded.mis_a_jour_par,
            identifiant_poste = excluded.identifiant_poste",
    )
    .bind(c.id.vers_chaine())
    .bind(c.etablissement_id.vers_chaine())
    .bind(c.paie_id.vers_chaine())
    .bind(c.absence_id.vers_chaine())
    .bind(i64::from(c.retenue))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

// --- Avances et paiements du personnel (SPEC_PHASE_7 §4 et §7) ---

#[derive(FromRow)]
struct LigneAvance {
    id: String,
    etablissement_id: String,
    membre_id: String,
    montant: i64,
    mensualites: i64,
    premier_mois: String,
    accordee_le: String,
    motif: String,
    statut: String,
    motif_annulation: String,
}

impl LigneAvance {
    fn vers_entite(self) -> Resultat<Avance> {
        let t = "avance_salaire";
        Ok(Avance {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            membre_id: lire_id(t, "membre_id", &self.membre_id)?,
            montant: Montant::depuis_francs(self.montant),
            mensualites: self.mensualites,
            premier_mois: mois(t, "premier_mois", &self.premier_mois)?,
            accordee_le: DateCivile::analyser(&self.accordee_le)
                .ok_or_else(|| ErreurBase::illisible(t, "accordee_le", &self.accordee_le))?,
            motif: self.motif,
            statut: StatutAvance::depuis_nom(&self.statut)
                .ok_or_else(|| ErreurBase::illisible(t, "statut", &self.statut))?,
            motif_annulation: self.motif_annulation,
        })
    }
}

/// Avances de l'école, de la plus ancienne à la plus récente.
pub async fn avances(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Vec<Avance>> {
    sqlx::query_as::<_, LigneAvance>(
        "SELECT id, etablissement_id, membre_id, montant, mensualites, premier_mois, accordee_le,
            motif, statut, motif_annulation
         FROM avance_salaire WHERE etablissement_id = ?1 ORDER BY accordee_le, cree_le, id",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneAvance::vers_entite)
    .collect()
}

/// Une avance.
pub async fn avance(cnx: &mut Connexion, id: Id<Avance>, contexte: &Contexte) -> Resultat<Avance> {
    sqlx::query_as::<_, LigneAvance>(
        "SELECT id, etablissement_id, membre_id, montant, mensualites, premier_mois, accordee_le,
            motif, statut, motif_annulation
         FROM avance_salaire WHERE id = ?1 AND etablissement_id = ?2",
    )
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .ok_or_else(|| ErreurBase::Introuvable {
        entite: "avance_salaire",
        id: id.vers_chaine(),
    })?
    .vers_entite()
}

/// Enregistre une avance.
pub async fn inserer_avance(cnx: &mut Connexion, a: &Avance, contexte: &Contexte) -> Resultat<()> {
    cloisonne(
        "avance_salaire",
        a.id,
        a.etablissement_id == contexte.etablissement_id,
    )?;
    sqlx::query(
        "INSERT INTO avance_salaire (id, etablissement_id, membre_id, montant, mensualites,
            premier_mois, accordee_le, motif, statut, motif_annulation, cree_le, cree_par,
            mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?11, ?12, ?13)",
    )
    .bind(a.id.vers_chaine())
    .bind(a.etablissement_id.vers_chaine())
    .bind(a.membre_id.vers_chaine())
    .bind(a.montant.francs())
    .bind(a.mensualites)
    .bind(a.premier_mois.to_string())
    .bind(a.accordee_le.to_string())
    .bind(&a.motif)
    .bind(a.statut.nom())
    .bind(&a.motif_annulation)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Récrit l'état d'une avance.
pub async fn ecrire_statut_avance(
    cnx: &mut Connexion,
    a: &Avance,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE avance_salaire SET statut = ?1, motif_annulation = ?2, mis_a_jour_le = ?3,
            mis_a_jour_par = ?4, identifiant_poste = ?5
         WHERE id = ?6 AND etablissement_id = ?7",
    )
    .bind(a.statut.nom())
    .bind(&a.motif_annulation)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(a.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Remboursements déjà retenus sur chaque avance, par les paies validées.
pub async fn rembourse_par_avance(
    cnx: &mut Connexion,
    contexte: &Contexte,
) -> Resultat<Vec<(Id<Avance>, Montant)>> {
    let lignes: Vec<(String, i64)> = sqlx::query_as(
        "SELECT avance_id, SUM(montant) FROM remboursement_avance WHERE etablissement_id = ?1
         GROUP BY avance_id",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?;
    lignes
        .into_iter()
        .map(|(id, total)| {
            Ok((
                lire_id("remboursement_avance", "avance_id", &id)?,
                Montant::depuis_francs(total),
            ))
        })
        .collect()
}

/// Note ce qu'une paie validée retient sur une avance.
pub async fn inserer_remboursement(
    cnx: &mut Connexion,
    id: &str,
    avance_id: Id<Avance>,
    bulletin_id: Id<BulletinPaie>,
    montant: Montant,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "INSERT INTO remboursement_avance (id, etablissement_id, avance_id, bulletin_id, montant,
            cree_le, cree_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
    )
    .bind(id)
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(avance_id.vers_chaine())
    .bind(bulletin_id.vers_chaine())
    .bind(montant.francs())
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

#[derive(FromRow)]
struct LignePaiement {
    id: String,
    etablissement_id: String,
    membre_id: String,
    objet: String,
    bulletin_id: Option<String>,
    avance_id: Option<String>,
    montant: i64,
    le: i64,
    mode: String,
    reference: String,
    source: String,
    session_id: Option<String>,
    par: String,
    annule: i64,
    motif_annulation: String,
}

impl LignePaiement {
    fn vers_entite(self) -> Resultat<PaiementPersonnel> {
        let t = "paiement_personnel";
        Ok(PaiementPersonnel {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            membre_id: lire_id(t, "membre_id", &self.membre_id)?,
            objet: ObjetPaiement::depuis_nom(&self.objet)
                .ok_or_else(|| ErreurBase::illisible(t, "objet", &self.objet))?,
            bulletin_id: self
                .bulletin_id
                .as_deref()
                .map(|b| lire_id(t, "bulletin_id", b))
                .transpose()?,
            avance_id: self
                .avance_id
                .as_deref()
                .map(|a| lire_id(t, "avance_id", a))
                .transpose()?,
            montant: Montant::depuis_francs(self.montant),
            le: Horodatage::depuis_millis_epoque(self.le),
            mode: ModePaiement::depuis_nom(&self.mode)
                .ok_or_else(|| ErreurBase::illisible(t, "mode", &self.mode))?,
            reference: self.reference,
            source: SourcePaiement::depuis_nom(&self.source)
                .ok_or_else(|| ErreurBase::illisible(t, "source", &self.source))?,
            session_id: self
                .session_id
                .as_deref()
                .map(|s| lire_id(t, "session_id", s))
                .transpose()?,
            par: lire_id(t, "par", &self.par)?,
            annule: booleen(self.annule),
            motif_annulation: self.motif_annulation,
        })
    }
}

macro_rules! selectionner_paiement {
    ($suite:literal) => {
        concat!(
            "SELECT id, etablissement_id, membre_id, objet, bulletin_id, avance_id, montant, le, mode,
                reference, source, session_id, par, annule, motif_annulation
             FROM paiement_personnel ",
            $suite
        )
    };
}

/// Paiements du personnel, annulés compris, du plus ancien au plus récent.
pub async fn paiements(
    cnx: &mut Connexion,
    contexte: &Contexte,
) -> Resultat<Vec<PaiementPersonnel>> {
    sqlx::query_as::<_, LignePaiement>(selectionner_paiement!(
        "WHERE etablissement_id = ?1 ORDER BY le, cree_le, id"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LignePaiement::vers_entite)
    .collect()
}

/// Un paiement.
pub async fn paiement(
    cnx: &mut Connexion,
    id: Id<PaiementPersonnel>,
    contexte: &Contexte,
) -> Resultat<PaiementPersonnel> {
    sqlx::query_as::<_, LignePaiement>(selectionner_paiement!(
        "WHERE id = ?1 AND etablissement_id = ?2"
    ))
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .ok_or_else(|| ErreurBase::Introuvable {
        entite: "paiement_personnel",
        id: id.vers_chaine(),
    })?
    .vers_entite()
}

/// Enregistre un paiement.
pub async fn inserer_paiement(
    cnx: &mut Connexion,
    p: &PaiementPersonnel,
    contexte: &Contexte,
) -> Resultat<()> {
    cloisonne(
        "paiement_personnel",
        p.id,
        p.etablissement_id == contexte.etablissement_id,
    )?;
    sqlx::query(
        "INSERT INTO paiement_personnel (id, etablissement_id, membre_id, objet, bulletin_id,
            avance_id, montant, le, mode, reference, source, session_id, par, annule,
            motif_annulation, cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?16,
            ?17, ?18)",
    )
    .bind(p.id.vers_chaine())
    .bind(p.etablissement_id.vers_chaine())
    .bind(p.membre_id.vers_chaine())
    .bind(p.objet.nom())
    .bind(p.bulletin_id.map(|b| b.vers_chaine()))
    .bind(p.avance_id.map(|a| a.vers_chaine()))
    .bind(p.montant.francs())
    .bind(p.le.millis_epoque())
    .bind(p.mode.nom())
    .bind(&p.reference)
    .bind(p.source.nom())
    .bind(p.session_id.map(|s| s.vers_chaine()))
    .bind(p.par.vers_chaine())
    .bind(i64::from(p.annule))
    .bind(&p.motif_annulation)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Annule un paiement, avec son motif.
pub async fn annuler_paiement(
    cnx: &mut Connexion,
    id: Id<PaiementPersonnel>,
    motif: &str,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE paiement_personnel SET annule = 1, motif_annulation = ?1, mis_a_jour_le = ?2,
            mis_a_jour_par = ?3, identifiant_poste = ?4
         WHERE id = ?5 AND etablissement_id = ?6",
    )
    .bind(motif)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}
