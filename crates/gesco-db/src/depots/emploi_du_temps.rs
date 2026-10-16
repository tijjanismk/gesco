//! Journée de l'école et emploi du temps (SPEC_PHASE_8 §8).

use gesco_core::commun::Id;
use gesco_core::core::AnneeScolaire;
use gesco_core::pedagogie::emploi_du_temps::{JourneeCycle, Plage, Seance};
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::depots::booleen;
use crate::erreur::{ErreurBase, Resultat};
use crate::id::lire_id;

/// Journées réglées par l'école, cycle par cycle.
pub async fn journees(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Vec<JourneeCycle>> {
    let lignes: Vec<(String,)> =
        sqlx::query_as("SELECT contenu FROM journee_cycle WHERE etablissement_id = ?1")
            .bind(contexte.etablissement_id.vers_chaine())
            .fetch_all(cnx)
            .await?;
    lignes
        .into_iter()
        .map(|(contenu,)| {
            serde_json::from_str(&contenu)
                .map_err(|_| ErreurBase::illisible("journee_cycle", "contenu", &contenu))
        })
        .collect()
}

/// Règle la journée d'un cycle.
pub async fn ecrire_journee(
    cnx: &mut Connexion,
    journee: &JourneeCycle,
    contexte: &Contexte,
) -> Resultat<()> {
    let contenu = serde_json::to_string(journee)
        .map_err(|_| ErreurBase::illisible("journee_cycle", "contenu", journee.cycle.nom()))?;
    sqlx::query(
        "INSERT INTO journee_cycle (etablissement_id, cycle, contenu, mis_a_jour_le,
            mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT (etablissement_id, cycle) DO UPDATE SET contenu = excluded.contenu,
            mis_a_jour_le = excluded.mis_a_jour_le, mis_a_jour_par = excluded.mis_a_jour_par,
            identifiant_poste = excluded.identifiant_poste",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(journee.cycle.nom())
    .bind(contenu)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

#[derive(FromRow)]
struct LigneSeance {
    id: String,
    etablissement_id: String,
    annee_id: String,
    classe_id: String,
    jour: i64,
    debut: i64,
    fin: i64,
    matiere_id: String,
    enseignant_id: Option<String>,
    salle: String,
    active: i64,
}

impl LigneSeance {
    fn vers_entite(self) -> Resultat<Seance> {
        let t = "seance";
        let minutes = |colonne: &'static str, valeur: i64| {
            u16::try_from(valeur).map_err(|_| ErreurBase::illisible(t, colonne, valeur.to_string()))
        };
        Ok(Seance {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            annee_id: lire_id(t, "annee_id", &self.annee_id)?,
            classe_id: lire_id(t, "classe_id", &self.classe_id)?,
            jour: u8::try_from(self.jour)
                .map_err(|_| ErreurBase::illisible(t, "jour", self.jour.to_string()))?,
            plage: Plage {
                debut: minutes("debut", self.debut)?,
                fin: minutes("fin", self.fin)?,
            },
            matiere_id: lire_id(t, "matiere_id", &self.matiere_id)?,
            enseignant_id: self
                .enseignant_id
                .as_deref()
                .map(|e| lire_id(t, "enseignant_id", e))
                .transpose()?,
            salle: self.salle,
            active: booleen(self.active),
        })
    }
}

/// Séances d'une année, retirées comprises, par jour et par heure.
pub async fn seances(
    cnx: &mut Connexion,
    annee_id: Id<AnneeScolaire>,
    contexte: &Contexte,
) -> Resultat<Vec<Seance>> {
    sqlx::query_as::<_, LigneSeance>(
        "SELECT id, etablissement_id, annee_id, classe_id, jour, debut, fin, matiere_id,
            enseignant_id, salle, active
         FROM seance WHERE etablissement_id = ?1 AND annee_id = ?2 ORDER BY jour, debut",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(annee_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneSeance::vers_entite)
    .collect()
}

/// Enregistre une séance.
pub async fn inserer_seance(cnx: &mut Connexion, s: &Seance, contexte: &Contexte) -> Resultat<()> {
    if s.etablissement_id != contexte.etablissement_id {
        return Err(ErreurBase::Cloisonnement {
            entite: "seance",
            id: s.id.vers_chaine(),
        });
    }
    sqlx::query(
        "INSERT INTO seance (id, etablissement_id, annee_id, classe_id, jour, debut, fin,
            matiere_id, enseignant_id, salle, active, cree_le, cree_par, mis_a_jour_le,
            mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?12, ?13, ?14)",
    )
    .bind(s.id.vers_chaine())
    .bind(s.etablissement_id.vers_chaine())
    .bind(s.annee_id.vers_chaine())
    .bind(s.classe_id.vers_chaine())
    .bind(i64::from(s.jour))
    .bind(i64::from(s.plage.debut))
    .bind(i64::from(s.plage.fin))
    .bind(s.matiere_id.vers_chaine())
    .bind(s.enseignant_id.map(|e| e.vers_chaine()))
    .bind(&s.salle)
    .bind(i64::from(s.active))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Récrit une séance : son horaire, sa matière, son enseignant, sa salle, son retrait.
pub async fn ecrire_seance(cnx: &mut Connexion, s: &Seance, contexte: &Contexte) -> Resultat<()> {
    sqlx::query(
        "UPDATE seance SET jour = ?1, debut = ?2, fin = ?3, matiere_id = ?4, enseignant_id = ?5,
            salle = ?6, active = ?7, mis_a_jour_le = ?8, mis_a_jour_par = ?9,
            identifiant_poste = ?10
         WHERE id = ?11 AND etablissement_id = ?12",
    )
    .bind(i64::from(s.jour))
    .bind(i64::from(s.plage.debut))
    .bind(i64::from(s.plage.fin))
    .bind(s.matiere_id.vers_chaine())
    .bind(s.enseignant_id.map(|e| e.vers_chaine()))
    .bind(&s.salle)
    .bind(i64::from(s.active))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(s.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}
