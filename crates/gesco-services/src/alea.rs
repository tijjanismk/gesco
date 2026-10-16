//! Aléa du système : la seule source des clés, nonces, sels et phrases de secours.

use crate::erreur::{ErreurService, Resultat};

/// Tableau d'octets aléatoires tirés du générateur du système d'exploitation.
pub(crate) fn octets<const N: usize>() -> Resultat<[u8; N]> {
    let mut tampon = [0u8; N];
    getrandom::fill(&mut tampon).map_err(|e| ErreurService::Secret(e.to_string()))?;
    Ok(tampon)
}
