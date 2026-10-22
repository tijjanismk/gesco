//! Journal technique du poste : chaque erreur rendue à l'écran, et chaque panne de l'interface,
//! notée avec l'heure dans `journal-technique.txt` du dossier de données, pour l'assistance.
//! Jamais une donnée saisie : le nom de la commande, la clé de l'erreur et son détail technique.
//! Au-delà d'un mégaoctet, le fichier devient `journal-technique.ancien.txt` et repart vide.

use std::io::Write;
use std::panic::AssertUnwindSafe;
use std::path::Path;
use std::pin::Pin;
use std::task::{Context, Poll};

use gesco_core::commun::Horodatage;

/// Nom du fichier, dans le dossier de données.
pub const FICHIER: &str = "journal-technique.txt";
const ANCIEN: &str = "journal-technique.ancien.txt";
const TAILLE_MAX: u64 = 1024 * 1024;
/// Longueur maximale d'une ligne venue de l'interface.
const LIGNE_MAX: usize = 2_000;

/// Instant écrit en temps universel : `2026-10-19 16:50:03 UTC`.
fn instant(h: Horodatage) -> String {
    let c = h.civil_utc();
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
        c.annee, c.mois, c.jour, c.heure, c.minute, c.seconde
    )
}

/// Ajoute une ligne au journal. Un échec d'écriture est ignoré : le journal ne doit jamais
/// empêcher GESCO de servir.
pub fn noter(racine: &Path, maintenant: Horodatage, ligne: &str) {
    let chemin = racine.join(FICHIER);
    if std::fs::metadata(&chemin).is_ok_and(|m| m.len() > TAILLE_MAX) {
        let _ = std::fs::rename(&chemin, racine.join(ANCIEN));
    }
    let propre: String = ligne
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(LIGNE_MAX)
        .collect();
    if let Ok(mut fichier) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&chemin)
    {
        let _ = writeln!(fichier, "{} {propre}", instant(maintenant));
    }
}

/// Message d'une panique, tel que `panic!` l'a reçu.
pub fn message_de_panique(charge: &(dyn std::any::Any + Send)) -> String {
    charge
        .downcast_ref::<&str>()
        .map(|m| (*m).to_owned())
        .or_else(|| charge.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "panique sans message".to_owned())
}

/// Calcul dont une panique devient une erreur : sans elle, l'interface attendrait une réponse
/// qui ne vient jamais, bouton grisé et sans message.
pub struct SansPanique<F: Future>(Pin<Box<F>>);

impl<F: Future> SansPanique<F> {
    /// Enveloppe le calcul.
    pub fn nouveau(calcul: F) -> Self {
        Self(Box::pin(calcul))
    }
}

impl<F: Future> Future for SansPanique<F> {
    type Output = Result<F::Output, String>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let calcul = &mut self.0;
        match std::panic::catch_unwind(AssertUnwindSafe(|| calcul.as_mut().poll(cx))) {
            Ok(Poll::Ready(valeur)) => Poll::Ready(Ok(valeur)),
            Ok(Poll::Pending) => Poll::Pending,
            Err(charge) => Poll::Ready(Err(message_de_panique(charge.as_ref()))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[allow(clippy::panic)]
    async fn une_panique_devient_une_erreur() {
        let rendu = SansPanique::nouveau(async {
            tokio::task::yield_now().await;
            if std::hint::black_box(true) {
                panic!("index hors limites");
            }
            1
        })
        .await;
        assert_eq!(rendu, Err("index hors limites".to_owned()));
        assert_eq!(SansPanique::nouveau(async { 2 }).await, Ok(2));
    }

    #[test]
    fn une_ligne_par_erreur_sans_retour_a_la_ligne() {
        let dossier = std::env::temp_dir().join(format!("gesco-jt-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dossier);
        let _ = std::fs::remove_file(dossier.join(FICHIER));
        let h = Horodatage::depuis_millis_epoque(1_791_101_730_250);
        noter(
            &dossier,
            h,
            "commande=premier_demarrage cle=secret.trousseau",
        );
        noter(&dossier, h, "interface\nplusieurs lignes");
        let contenu = std::fs::read_to_string(dossier.join(FICHIER)).unwrap_or_default();
        let lignes: Vec<&str> = contenu.lines().collect();
        assert_eq!(lignes.len(), 2);
        assert!(
            lignes.first().is_some_and(
                |l| l.starts_with("2026-10-04 08:15:30 UTC commande=premier_demarrage")
            )
        );
        assert!(
            lignes
                .get(1)
                .is_some_and(|l| l.ends_with("interface plusieurs lignes"))
        );
        let _ = std::fs::remove_dir_all(&dossier);
    }
}
