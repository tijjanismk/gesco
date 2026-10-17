//! Autres recettes (SPEC_VERSION_1_1 §1, étape 1).

use gesco_core::commun::{DateCivile, Id, Montant};
use gesco_core::finance::ModePaiement;
use gesco_core::tresorerie::CategorieTresorerie;
use gesco_core::tresorerie::recette::Recette;
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::depots::booleen;
use crate::erreur::{ErreurBase, Resultat};
use crate::id::lire_id;

#[derive(FromRow)]
struct LigneRecette {
    id: String,
    etablissement_id: String,
    date: String,
    categorie_id: String,
    libelle: String,
    payeur: String,
    montant: i64,
    compte_id: String,
    mode: String,
    reference: String,
    session_id: Option<String>,
    saisie_par: String,
    annulee: i64,
    motif_annulation: String,
}

impl LigneRecette {
    fn vers_entite(self) -> Resultat<Recette> {
        let t = "recette_tresorerie";
        Ok(Recette {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            date: DateCivile::analyser(&self.date)
                .ok_or_else(|| ErreurBase::illisible(t, "date", &self.date))?,
            categorie_id: lire_id(t, "categorie_id", &self.categorie_id)?,
            libelle: self.libelle,
            payeur: self.payeur,
            montant: Montant::depuis_francs(self.montant),
            compte_id: lire_id(t, "compte_id", &self.compte_id)?,
            mode: ModePaiement::depuis_nom(&self.mode)
                .ok_or_else(|| ErreurBase::illisible(t, "mode", &self.mode))?,
            reference: self.reference,
            session_id: self
                .session_id
                .as_deref()
                .map(|s| lire_id(t, "session_id", s))
                .transpose()?,
            saisie_par: lire_id(t, "saisie_par", &self.saisie_par)?,
            annulee: booleen(self.annulee),
            motif_annulation: self.motif_annulation,
        })
    }
}

macro_rules! selectionner_recette {
    ($suite:literal) => {
        concat!(
            "SELECT id, etablissement_id, date, categorie_id, libelle, payeur, montant, compte_id,
                mode, reference, session_id, saisie_par, annulee, motif_annulation
             FROM recette_tresorerie ",
            $suite
        )
    };
}

/// Recettes de l'école, annulées comprises, de la plus ancienne à la plus récente.
pub async fn recettes(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Vec<Recette>> {
    sqlx::query_as::<_, LigneRecette>(selectionner_recette!(
        "WHERE etablissement_id = ?1 ORDER BY date, cree_le"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneRecette::vers_entite)
    .collect()
}

/// Une recette.
pub async fn recette(
    cnx: &mut Connexion,
    id: Id<Recette>,
    contexte: &Contexte,
) -> Resultat<Recette> {
    sqlx::query_as::<_, LigneRecette>(selectionner_recette!(
        "WHERE id = ?1 AND etablissement_id = ?2"
    ))
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .ok_or_else(|| ErreurBase::Introuvable {
        entite: "recette_tresorerie",
        id: id.vers_chaine(),
    })?
    .vers_entite()
}

/// Enregistre une recette.
pub async fn inserer(cnx: &mut Connexion, r: &Recette, contexte: &Contexte) -> Resultat<()> {
    if r.etablissement_id != contexte.etablissement_id {
        return Err(ErreurBase::Cloisonnement {
            entite: "recette_tresorerie",
            id: r.id.vers_chaine(),
        });
    }
    sqlx::query(
        "INSERT INTO recette_tresorerie (id, etablissement_id, date, categorie_id, libelle, payeur,
            montant, compte_id, mode, reference, session_id, saisie_par, annulee,
            motif_annulation, cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?15, ?16,
            ?17)",
    )
    .bind(r.id.vers_chaine())
    .bind(r.etablissement_id.vers_chaine())
    .bind(r.date.to_string())
    .bind(r.categorie_id.vers_chaine())
    .bind(&r.libelle)
    .bind(&r.payeur)
    .bind(r.montant.francs())
    .bind(r.compte_id.vers_chaine())
    .bind(r.mode.nom())
    .bind(&r.reference)
    .bind(r.session_id.map(|s| s.vers_chaine()))
    .bind(r.saisie_par.vers_chaine())
    .bind(i64::from(r.annulee))
    .bind(&r.motif_annulation)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Annule une recette, avec son motif.
pub async fn annuler(
    cnx: &mut Connexion,
    id: Id<Recette>,
    motif: &str,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE recette_tresorerie SET annulee = 1, motif_annulation = ?1, mis_a_jour_le = ?2,
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

/// Recettes non annulées entre deux jours (le second exclu), par catégorie.
pub async fn par_categorie_entre(
    cnx: &mut Connexion,
    debut: DateCivile,
    fin: DateCivile,
    contexte: &Contexte,
) -> Resultat<Vec<(Id<CategorieTresorerie>, Montant)>> {
    let lignes: Vec<(String, i64)> = sqlx::query_as(
        "SELECT categorie_id, SUM(montant) FROM recette_tresorerie
         WHERE etablissement_id = ?1 AND annulee = 0 AND date >= ?2 AND date < ?3
         GROUP BY categorie_id",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(debut.to_string())
    .bind(fin.to_string())
    .fetch_all(cnx)
    .await?;
    lignes
        .into_iter()
        .map(|(c, m)| {
            Ok((
                lire_id("recette_tresorerie", "categorie_id", &c)?,
                Montant::depuis_francs(m),
            ))
        })
        .collect()
}
