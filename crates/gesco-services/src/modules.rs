//! Modules de l'école (`ModuleEcole`) : lequel est allumé, d'après son réglage.

use gesco_core::core::ModuleEcole;
use gesco_db::depots::parametre;
use gesco_db::{Connexion, Contexte};

use crate::erreur::Resultat;

/// Vrai si l'école a allumé ce module.
pub(crate) async fn actif(
    cnx: &mut Connexion,
    module: ModuleEcole,
    contexte: &Contexte,
) -> Resultat<bool> {
    Ok(parametre::booleen_param(
        cnx,
        module.cle_reglage(),
        module.actif_par_defaut(),
        contexte,
    )
    .await?)
}

/// Les modules allumés.
pub(crate) async fn actifs(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Vec<ModuleEcole>> {
    let mut actifs = Vec::new();
    for module in ModuleEcole::TOUS {
        if actif(cnx, module, contexte).await? {
            actifs.push(module);
        }
    }
    Ok(actifs)
}
