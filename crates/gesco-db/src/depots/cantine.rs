//! Cantine (SPEC_PHASE_8 §9) : les abonnés, lus dans les factures de l'année, et les repas
//! pointés.

use gesco_core::commun::{DateCivile, Id, Montant};
use gesco_core::core::AnneeScolaire;
use gesco_core::scolarite::inscription::Inscription;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::erreur::Resultat;
use crate::id::lire_id;

/// Élève abonné à la cantine : sa facture de l'année porte des frais de cantine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Abonne {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Prénoms et nom.
    pub eleve: String,
    /// Classe.
    pub classe: String,
    /// Cantine échue au jour dite et pas encore payée.
    pub reste_echu: Montant,
}

/// Abonnés d'une année au jour dit, par classe et par nom.
pub async fn abonnes(
    cnx: &mut Connexion,
    annee_id: Id<AnneeScolaire>,
    jour: DateCivile,
    contexte: &Contexte,
) -> Resultat<Vec<Abonne>> {
    let lignes: Vec<(String, String, String, String, i64, i64)> = sqlx::query_as(
        "SELECT i.id, p.prenoms, p.nom, c.libelle,
            (SELECT COALESCE(SUM(e.montant), 0) FROM echeance e
                JOIN ligne_facture l ON l.id = e.ligne_facture_id
             WHERE l.facture_id = f.id AND l.type_frais = 'cantine' AND e.statut = 'due'
                AND e.date_echeance <= ?3),
            (SELECT COALESCE(SUM(im.montant), 0) FROM imputation_versement im
                JOIN echeance e ON e.id = im.echeance_id
                JOIN ligne_facture l ON l.id = e.ligne_facture_id
             WHERE l.facture_id = f.id AND l.type_frais = 'cantine' AND im.statut = 'active'
                AND e.date_echeance <= ?3)
         FROM inscription i
            JOIN eleve el ON el.id = i.eleve_id
            JOIN personne p ON p.id = el.personne_id
            JOIN classe c ON c.id = i.classe_id
            JOIN facture f ON f.inscription_id = i.id AND f.statut <> 'annulee'
         WHERE i.etablissement_id = ?1 AND i.annee_id = ?2 AND i.statut = 'validee'
            AND EXISTS (SELECT 1 FROM ligne_facture lf WHERE lf.facture_id = f.id
                AND lf.type_frais = 'cantine' AND lf.montant > 0)
         ORDER BY c.libelle, p.nom, p.prenoms",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(annee_id.vers_chaine())
    .bind(jour.to_string())
    .fetch_all(cnx)
    .await?;
    lignes
        .into_iter()
        .map(|(id, prenoms, nom, classe, du, paye)| {
            Ok(Abonne {
                inscription_id: lire_id("inscription", "id", &id)?,
                eleve: format!("{prenoms} {nom}"),
                classe,
                reste_echu: Montant::depuis_francs((du - paye).max(0)),
            })
        })
        .collect()
}

/// Inscriptions dont le repas est pointé un jour.
pub async fn repas_du_jour(
    cnx: &mut Connexion,
    jour: DateCivile,
    contexte: &Contexte,
) -> Resultat<Vec<Id<Inscription>>> {
    let lignes: Vec<(String,)> = sqlx::query_as(
        "SELECT inscription_id FROM repas_cantine
         WHERE etablissement_id = ?1 AND date = ?2 AND pris = 1",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(jour.to_string())
    .fetch_all(cnx)
    .await?;
    lignes
        .into_iter()
        .map(|(id,)| lire_id("repas_cantine", "inscription_id", &id))
        .collect()
}

/// Repas pris entre deux jours inclus.
pub async fn repas_entre(
    cnx: &mut Connexion,
    debut: DateCivile,
    fin: DateCivile,
    contexte: &Contexte,
) -> Resultat<i64> {
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM repas_cantine
         WHERE etablissement_id = ?1 AND date BETWEEN ?2 AND ?3 AND pris = 1",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(debut.to_string())
    .bind(fin.to_string())
    .fetch_one(cnx)
    .await?;
    Ok(n)
}

/// Pointe ou décoche le repas d'un élève un jour.
pub async fn pointer(
    cnx: &mut Connexion,
    id_si_nouveau: &str,
    inscription_id: Id<Inscription>,
    jour: DateCivile,
    pris: bool,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "INSERT INTO repas_cantine (id, etablissement_id, inscription_id, date, pris, cree_le,
            cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?6, ?7, ?8)
         ON CONFLICT (inscription_id, date) DO UPDATE SET pris = excluded.pris,
            mis_a_jour_le = excluded.mis_a_jour_le, mis_a_jour_par = excluded.mis_a_jour_par,
            identifiant_poste = excluded.identifiant_poste",
    )
    .bind(id_si_nouveau)
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(inscription_id.vers_chaine())
    .bind(jour.to_string())
    .bind(i64::from(pris))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}
