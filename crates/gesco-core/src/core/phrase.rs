//! Phrase de secours (spécification §6.3) : douze mots tirés d'une liste française de 2 048 mots,
//! soit 132 bits d'entropie. La liste est celle de BIP-39 (licence MIT) ; ses mots restent
//! distincts une fois les accents retirés, ce qui permet de saisir la phrase sans accents.

use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

use super::cle::{CleEnveloppe, ErreurCle, ParametresDerivation};

/// Nombre de mots d'une phrase.
pub const NOMBRE_DE_MOTS: usize = 12;

/// Octets d'aléa consommés pour tirer une phrase : 132 bits utiles sur 136.
pub const OCTETS_ENTROPIE: usize = 17;

const LISTE: &str = include_str!("mots_fr.txt");

/// Les 2 048 mots, dans l'ordre de la liste.
pub fn liste_de_mots() -> impl Iterator<Item = &'static str> {
    LISTE.lines().filter(|l| !l.is_empty())
}

fn mot(index: u16) -> &'static str {
    liste_de_mots().nth(usize::from(index)).unwrap_or_default()
}

/// Forme de comparaison : minuscules, sans accents (composés ou combinants).
fn replier(texte: &str) -> String {
    texte
        .chars()
        .filter(|c| !('\u{300}'..='\u{36f}').contains(c))
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            autre => autre,
        })
        .collect()
}

/// Phrase de secours, sous forme d'indices dans la liste. Jamais affichée par `Debug`, jamais
/// journalisée (R-25) ; effacée de la mémoire à la libération.
#[derive(Clone, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
pub struct PhraseSecours {
    indices: [u16; NOMBRE_DE_MOTS],
}

impl std::fmt::Debug for PhraseSecours {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PhraseSecours(****)")
    }
}

impl PhraseSecours {
    /// Tire une phrase d'un aléa de 136 bits fourni par l'appelant, onze bits par mot.
    pub fn depuis_entropie(entropie: &[u8; OCTETS_ENTROPIE]) -> Self {
        let mut indices = [0u16; NOMBRE_DE_MOTS];
        for (rang, index) in indices.iter_mut().enumerate() {
            let mut valeur = 0u16;
            for bit in rang * 11..rang * 11 + 11 {
                let octet = entropie.get(bit / 8).copied().unwrap_or(0);
                valeur = (valeur << 1) | u16::from((octet >> (7 - bit % 8)) & 1);
            }
            *index = valeur;
        }
        Self { indices }
    }

    /// Lit une phrase saisie : douze mots séparés par des espaces, accents et majuscules
    /// facultatifs.
    pub fn analyser(saisie: &str) -> Result<Self, ErreurCle> {
        let mots: Vec<String> = saisie.split_whitespace().map(replier).collect();
        if mots.len() != NOMBRE_DE_MOTS {
            return Err(ErreurCle::Phrase("douze mots attendus"));
        }
        let mut indices = [0u16; NOMBRE_DE_MOTS];
        for (index, saisi) in indices.iter_mut().zip(&mots) {
            let trouve = liste_de_mots()
                .position(|m| replier(m) == *saisi)
                .ok_or(ErreurCle::Phrase("mot absent de la liste"))?;
            *index = u16::try_from(trouve).map_err(|_| ErreurCle::Phrase("liste invalide"))?;
        }
        Ok(Self { indices })
    }

    /// Les douze mots, accentués, pour l'affichage unique et le certificat.
    pub fn mots(&self) -> [&'static str; NOMBRE_DE_MOTS] {
        self.indices.map(mot)
    }

    /// Forme canonique dérivée : mots repliés séparés par une espace. Elle ne dépend ni des
    /// accents ni de la casse saisis.
    fn canonique(&self) -> Zeroizing<String> {
        Zeroizing::new(
            self.indices
                .iter()
                .map(|&i| replier(mot(i)))
                .collect::<Vec<_>>()
                .join(" "),
        )
    }

    /// Clé d'enveloppe dérivée de la phrase.
    pub fn deriver(&self, parametres: &ParametresDerivation) -> Result<CleEnveloppe, ErreurCle> {
        parametres.deriver(self.canonique().as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn la_liste_compte_2048_mots_distincts_meme_sans_accents() {
        let mots: Vec<&str> = liste_de_mots().collect();
        assert_eq!(mots.len(), 2048);
        let replies: HashSet<String> = mots.iter().map(|m| replier(m)).collect();
        assert_eq!(replies.len(), 2048);
    }

    #[test]
    fn onze_bits_par_mot() {
        let mut entropie = [0u8; OCTETS_ENTROPIE];
        entropie[0] = 0b1000_0000;
        let p = PhraseSecours::depuis_entropie(&entropie);
        assert_eq!(p.indices[0], 1024);
        assert!(p.indices[1..].iter().all(|&i| i == 0));
        let p = PhraseSecours::depuis_entropie(&[0xff; OCTETS_ENTROPIE]);
        assert!(p.indices.iter().all(|&i| i == 2047));
    }

    #[test]
    fn la_saisie_tolere_accents_casse_et_espaces() -> Result<(), ErreurCle> {
        let p = PhraseSecours::depuis_entropie(&[0x5a; OCTETS_ENTROPIE]);
        let saisie = p
            .mots()
            .iter()
            .map(|m| replier(m).to_uppercase())
            .collect::<Vec<_>>()
            .join("   ");
        assert_eq!(PhraseSecours::analyser(&saisie)?, p);
        assert_eq!(PhraseSecours::analyser(&p.mots().join(" "))?, p);
        Ok(())
    }

    #[test]
    fn une_saisie_incomplete_ou_fausse_est_refusee() {
        assert!(PhraseSecours::analyser("abaisser abandon").is_err());
        let douze = ["abaisser"; 11].join(" ") + " zzzz";
        assert!(PhraseSecours::analyser(&douze).is_err());
    }

    #[test]
    fn la_phrase_ne_s_affiche_pas() {
        let p = PhraseSecours::depuis_entropie(&[1; OCTETS_ENTROPIE]);
        assert_eq!(format!("{p:?}"), "PhraseSecours(****)");
    }
}
