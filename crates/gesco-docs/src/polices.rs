//! Polices embarquées : aucune police du système n'est lue, le rendu est identique sur tous les
//! postes (blueprint §5.3).

use typst::foundations::Bytes;
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;

const CHARIS_REGULIER: &[u8] = include_bytes!("../polices/CharisSIL-Regular.ttf");
const CHARIS_GRAS: &[u8] = include_bytes!("../polices/CharisSIL-Bold.ttf");
/// Police des formules des leçons (ADR « format des leçons »).
const MATHS: &[u8] = include_bytes!("../polices/NewCMMath-Regular.otf");

pub(crate) struct Polices {
    pub(crate) livre: LazyHash<FontBook>,
    pub(crate) polices: Vec<Font>,
}

impl Polices {
    pub(crate) fn embarquees() -> Self {
        let polices: Vec<Font> = [CHARIS_REGULIER, CHARIS_GRAS, MATHS]
            .into_iter()
            .flat_map(|octets| Font::iter(Bytes::new(octets)))
            .collect();
        Self {
            livre: LazyHash::new(FontBook::from_fonts(&polices)),
            polices,
        }
    }
}
