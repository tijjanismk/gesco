//! QR code de vérification des documents (blueprint §25.9) : il encode la référence du document,
//! que l'école retrouve dans ses registres. Produit en SVG par le noyau, jamais par le webview
//! (R-44).

use qrcode::render::svg;
use qrcode::{EcLevel, QrCode};

use crate::erreur::ErreurDocument;
use crate::moteur::Piece;

/// Texte encodé pour un document : `GESCO:DOC:<identifiant>`.
pub fn reference(document_id: &str) -> String {
    format!("GESCO:DOC:{document_id}")
}

/// QR code d'un texte, en SVG, prêt à joindre à une composition sous le nom `qr.svg`.
pub fn piece(texte: &str) -> Result<Piece, ErreurDocument> {
    let code = QrCode::with_error_correction_level(texte.as_bytes(), EcLevel::M).map_err(|e| {
        ErreurDocument::Composition {
            modele: "qr".to_owned(),
            detail: e.to_string(),
        }
    })?;
    let image = code
        .render::<svg::Color<'_>>()
        .min_dimensions(200, 200)
        .quiet_zone(true)
        .build();
    Ok(Piece {
        nom: "qr.svg".to_owned(),
        octets: image.into_bytes(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_qr_est_un_svg() {
        let Ok(piece) = piece(&reference("01J9ZQ3W5N6K8T0V2X4Y6Z8A0B")) else {
            unreachable!("une référence courte tient dans un QR code");
        };
        assert_eq!(piece.nom, "qr.svg");
        let texte = String::from_utf8(piece.octets).unwrap_or_default();
        assert!(texte.contains("<svg"), "{texte:.80}");
    }
}
