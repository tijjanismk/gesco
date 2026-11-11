//! Opérations qui demandent d'être seul (SPEC_PHASE_11_A §6.3) : clôture de l'année,
//! restauration, imports. Le temps que dure une telle commande, celles des autres postes sont
//! refusées avec un message qui nomme l'opération ; le poste qui l'a lancée reste servi.

use std::sync::{Mutex, PoisonError};

use crate::commandes::Commande;
use crate::erreur::{ErreurService, Resultat};
use crate::origine::Origine;

/// Opération en cours qui écarte les autres postes.
#[derive(Debug, Clone)]
struct OperationSeule {
    /// Clé du message qui la nomme.
    cle: &'static str,
    /// Poste qui l'a lancée : lui seul continue d'être servi.
    origine: Origine,
}

/// L'opération en cours, s'il y en a une.
#[derive(Debug, Default)]
pub(crate) struct Exclusivite(Mutex<Option<OperationSeule>>);

/// Tient l'opération le temps de sa commande ; la rend en tombant, même après une erreur.
#[derive(Debug)]
pub(crate) struct Seul<'a>(&'a Exclusivite);

impl Drop for Seul<'_> {
    fn drop(&mut self) {
        *self.0.0.lock().unwrap_or_else(PoisonError::into_inner) = None;
    }
}

impl Exclusivite {
    /// Refuse la commande si une opération qui demande d'être seul est en cours depuis un autre
    /// poste, ou si elle en lancerait une seconde ; si la commande en est une, la retient
    /// jusqu'à ce que la garde rendue tombe.
    pub(crate) fn entrer(
        &self,
        commande: &Commande,
        origine: &Origine,
    ) -> Resultat<Option<Seul<'_>>> {
        let mut en_cours = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(operation) = en_cours.as_ref()
            && (operation.origine != *origine || commande.seule().is_some())
        {
            return Err(ErreurService::OperationEnCours { cle: operation.cle });
        }
        Ok(commande.seule().map(|cle| {
            *en_cours = Some(OperationSeule {
                cle,
                origine: origine.clone(),
            });
            Seul(self)
        }))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use gesco_core::core::Poste;

    use super::*;

    fn commande(nom: &str) -> &'static Commande {
        Commande::par_nom(nom).unwrap()
    }

    fn cle(refus: Resultat<Option<Seul<'_>>>) -> &'static str {
        refus.expect_err("refus attendu").cle_traduction()
    }

    #[test]
    fn une_operation_seule_ecarte_les_autres_postes_puis_les_rend() {
        let exclusivite = Exclusivite::default();
        let principal = Origine::Locale;
        let caisse = Origine::Relie(Poste("Caisse".to_owned()));
        let accueil = Origine::Relie(Poste("Accueil".to_owned()));

        // Rien en cours : tout le monde passe, sans rien retenir.
        assert!(
            exclusivite
                .entrer(commande("eleve_lire"), &caisse)
                .unwrap()
                .is_none()
        );

        // La clôture de l'année, lancée du poste principal.
        let cloture = exclusivite
            .entrer(commande("annee_cloturer"), &principal)
            .unwrap();
        assert!(cloture.is_some());
        // Les autres postes sont refusés, en lecture comme en écriture, et le message nomme
        // l'opération.
        for nom in ["eleve_lire", "versement_enregistrer", "session_courante"] {
            if Commande::par_nom(nom).is_some() {
                assert_eq!(
                    cle(exclusivite.entrer(commande(nom), &caisse)),
                    "poste.occupe.cloture"
                );
            }
        }
        assert_eq!(
            cle(exclusivite.entrer(commande("eleve_lire"), &accueil)),
            "poste.occupe.cloture"
        );
        // Le poste qui l'a lancée reste servi, mais n'en lance pas une seconde.
        assert!(
            exclusivite
                .entrer(commande("eleve_lire"), &principal)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            cle(exclusivite.entrer(commande("sauvegarde_restaurer"), &principal)),
            "poste.occupe.cloture"
        );

        // L'opération finie, même par une erreur, les autres postes reprennent.
        drop(cloture);
        assert!(
            exclusivite
                .entrer(commande("eleve_lire"), &caisse)
                .unwrap()
                .is_none()
        );

        // Un import lancé d'un poste relié écarte aussi le poste principal.
        let import = exclusivite
            .entrer(commande("import_eleves_appliquer"), &caisse)
            .unwrap();
        assert_eq!(
            cle(exclusivite.entrer(commande("eleve_lire"), &principal)),
            "poste.occupe.import"
        );
        drop(import);
        assert!(
            exclusivite
                .entrer(commande("eleve_lire"), &principal)
                .unwrap()
                .is_none()
        );
    }
}
