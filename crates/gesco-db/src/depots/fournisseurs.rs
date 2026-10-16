//! Fournisseurs et leurs factures (SPEC_PHASE_8 §4).

use gesco_core::commun::{DateCivile, Id, Montant};
use gesco_core::tresorerie::fournisseur::{FactureFournisseur, Fournisseur};
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::depots::booleen;
use crate::erreur::{ErreurBase, Resultat};
use crate::id::lire_id;

#[derive(FromRow)]
struct LigneFournisseur {
    id: String,
    etablissement_id: String,
    nom: String,
    telephone: String,
    fourniture: String,
    actif: i64,
}

impl LigneFournisseur {
    fn vers_entite(self) -> Resultat<Fournisseur> {
        let t = "fournisseur";
        Ok(Fournisseur {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            nom: self.nom,
            telephone: self.telephone,
            fourniture: self.fourniture,
            actif: booleen(self.actif),
        })
    }
}

/// Fournisseurs de l'école, par nom.
pub async fn fournisseurs(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Vec<Fournisseur>> {
    sqlx::query_as::<_, LigneFournisseur>(
        "SELECT id, etablissement_id, nom, telephone, fourniture, actif FROM fournisseur
         WHERE etablissement_id = ?1 ORDER BY nom",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneFournisseur::vers_entite)
    .collect()
}

/// Enregistre un fournisseur.
pub async fn inserer_fournisseur(
    cnx: &mut Connexion,
    f: &Fournisseur,
    contexte: &Contexte,
) -> Resultat<()> {
    if f.etablissement_id != contexte.etablissement_id {
        return Err(ErreurBase::Cloisonnement {
            entite: "fournisseur",
            id: f.id.vers_chaine(),
        });
    }
    sqlx::query(
        "INSERT INTO fournisseur (id, etablissement_id, nom, telephone, fourniture, actif, cree_le,
            cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?7, ?8, ?9)",
    )
    .bind(f.id.vers_chaine())
    .bind(f.etablissement_id.vers_chaine())
    .bind(&f.nom)
    .bind(&f.telephone)
    .bind(&f.fourniture)
    .bind(i64::from(f.actif))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Récrit un fournisseur.
pub async fn ecrire_fournisseur(
    cnx: &mut Connexion,
    f: &Fournisseur,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE fournisseur SET nom = ?1, telephone = ?2, fourniture = ?3, actif = ?4,
            mis_a_jour_le = ?5, mis_a_jour_par = ?6, identifiant_poste = ?7
         WHERE id = ?8 AND etablissement_id = ?9",
    )
    .bind(&f.nom)
    .bind(&f.telephone)
    .bind(&f.fourniture)
    .bind(i64::from(f.actif))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(f.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

#[derive(FromRow)]
struct LigneFacture {
    id: String,
    etablissement_id: String,
    fournisseur_id: String,
    numero: String,
    date: String,
    echeance: Option<String>,
    libelle: String,
    categorie_id: String,
    montant: i64,
    annulee: i64,
    motif_annulation: String,
}

impl LigneFacture {
    fn vers_entite(self) -> Resultat<FactureFournisseur> {
        let t = "facture_fournisseur";
        let jour = |colonne: &'static str, texte: &str| {
            DateCivile::analyser(texte).ok_or_else(|| ErreurBase::illisible(t, colonne, texte))
        };
        Ok(FactureFournisseur {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            fournisseur_id: lire_id(t, "fournisseur_id", &self.fournisseur_id)?,
            numero: self.numero,
            date: jour("date", &self.date)?,
            echeance: self
                .echeance
                .as_deref()
                .map(|e| jour("echeance", e))
                .transpose()?,
            libelle: self.libelle,
            categorie_id: lire_id(t, "categorie_id", &self.categorie_id)?,
            montant: Montant::depuis_francs(self.montant),
            annulee: booleen(self.annulee),
            motif_annulation: self.motif_annulation,
        })
    }
}

/// Factures des fournisseurs, de la plus ancienne à la plus récente.
pub async fn factures(
    cnx: &mut Connexion,
    contexte: &Contexte,
) -> Resultat<Vec<FactureFournisseur>> {
    sqlx::query_as::<_, LigneFacture>(
        "SELECT id, etablissement_id, fournisseur_id, numero, date, echeance, libelle, categorie_id,
            montant, annulee, motif_annulation
         FROM facture_fournisseur WHERE etablissement_id = ?1 ORDER BY date, cree_le",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneFacture::vers_entite)
    .collect()
}

/// Enregistre une facture.
pub async fn inserer_facture(
    cnx: &mut Connexion,
    f: &FactureFournisseur,
    contexte: &Contexte,
) -> Resultat<()> {
    if f.etablissement_id != contexte.etablissement_id {
        return Err(ErreurBase::Cloisonnement {
            entite: "facture_fournisseur",
            id: f.id.vers_chaine(),
        });
    }
    sqlx::query(
        "INSERT INTO facture_fournisseur (id, etablissement_id, fournisseur_id, numero, date,
            echeance, libelle, categorie_id, montant, annulee, motif_annulation, cree_le, cree_par,
            mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?12, ?13, ?14)",
    )
    .bind(f.id.vers_chaine())
    .bind(f.etablissement_id.vers_chaine())
    .bind(f.fournisseur_id.vers_chaine())
    .bind(&f.numero)
    .bind(f.date.to_string())
    .bind(f.echeance.map(|e| e.to_string()))
    .bind(&f.libelle)
    .bind(f.categorie_id.vers_chaine())
    .bind(f.montant.francs())
    .bind(i64::from(f.annulee))
    .bind(&f.motif_annulation)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Annule une facture, avec son motif.
pub async fn annuler_facture(
    cnx: &mut Connexion,
    id: Id<FactureFournisseur>,
    motif: &str,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE facture_fournisseur SET annulee = 1, motif_annulation = ?1, mis_a_jour_le = ?2,
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
