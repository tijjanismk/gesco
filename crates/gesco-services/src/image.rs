//! Images gardées dans la base : logo de l'établissement, photo d'un élève. PNG ou JPEG,
//! reconnus à leurs premiers octets, jamais à l'extension ou au type annoncé (R-02).

use base64::Engine;
use sha2::{Digest, Sha256};

use crate::erreur::{ErreurService, Resultat};

/// Image décodée et reconnue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    /// Octets du fichier.
    pub octets: Vec<u8>,
    /// `png` ou `jpeg`.
    pub format: &'static str,
}

/// Clés des messages d'erreur propres à chaque usage.
pub struct Refus {
    /// Base64 illisible.
    pub illisible: &'static str,
    /// Taille dépassée.
    pub trop_lourde: &'static str,
    /// Ni PNG ni JPEG.
    pub format: &'static str,
}

/// Format d'image reconnu à ses premiers octets.
pub fn format_image(octets: &[u8]) -> Option<&'static str> {
    if octets.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("png")
    } else if octets.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("jpeg")
    } else {
        None
    }
}

/// Décode une image transmise en base64 et vérifie son format et sa taille.
pub fn decoder(base64: &str, taille_max: usize, refus: &Refus) -> Resultat<Image> {
    let regle = |cle| ErreurService::Regle { cle };
    let octets = base64::engine::general_purpose::STANDARD
        .decode(base64.trim())
        .map_err(|_| regle(refus.illisible))?;
    if octets.len() > taille_max {
        return Err(regle(refus.trop_lourde));
    }
    let format = format_image(&octets).ok_or(regle(refus.format))?;
    Ok(Image { octets, format })
}

/// URL de données pour l'aperçu à l'écran : `data:image/png;base64,…`.
pub fn url_de_donnees(octets: &[u8], format: &str) -> String {
    let mime = if format == "png" {
        "image/png"
    } else {
        "image/jpeg"
    };
    format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(octets)
    )
}

/// Ce que l'audit garde d'une image : son empreinte et sa taille, jamais son contenu.
pub fn trace(octets: &[u8], format: &str) -> serde_json::Value {
    serde_json::json!({
        "empreinte": hex::encode(Sha256::digest(octets)),
        "taille": octets.len(),
        "format": format,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_format_se_lit_aux_premiers_octets() {
        assert_eq!(format_image(b"\x89PNG\r\n\x1a\nreste"), Some("png"));
        assert_eq!(format_image(&[0xFF, 0xD8, 0xFF, 0xE0]), Some("jpeg"));
        assert_eq!(format_image(b"GIF89a"), None);
    }
}
