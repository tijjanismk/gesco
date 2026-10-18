//! Trames PDU des SMS (3GPP TS 23.040) : ce que la clé modem reçoit en mode PDU. Le texte, déjà
//! translittéré en GSM-7 par la file, est codé sur sept bits ; au-delà de 160 caractères, il part
//! en plusieurs parties qui se recollent sur le téléphone ; l'accusé de remise est demandé sur la
//! dernière. Les accusés (SMS-STATUS-REPORT) se décodent ici.

use std::fmt::Write as _;

use gesco_core::notification::ErreurEnvoi;
use gesco_core::notification::gsm7::{CARACTERES_PAR_PARTIE, CARACTERES_SMS_SEUL};

/// Code d'échappement vers la table d'extension.
const ECHAPPEMENT: u8 = 0x1B;

/// Ajoute le ou les septets GSM-7 d'un caractère : un pour l'alphabet de base, deux pour la
/// table d'extension. Un caractère hors alphabet devient « ? » (la file translittère avant, ce
/// cas ne se présente pas).
fn ajouter_septets(c: char, septets: &mut Vec<u8>) {
    let extension = match c {
        '^' => Some(0x14),
        '{' => Some(0x28),
        '}' => Some(0x29),
        '\\' => Some(0x2F),
        '[' => Some(0x3C),
        '~' => Some(0x3D),
        ']' => Some(0x3E),
        '|' => Some(0x40),
        '€' => Some(0x65),
        _ => None,
    };
    if let Some(code) = extension {
        septets.extend([ECHAPPEMENT, code]);
        return;
    }
    let base = match c {
        '@' => 0x00,
        '$' => 0x02,
        '_' => 0x11,
        '\n' => 0x0A,
        '\r' => 0x0D,
        ' '..='#' | '%'..='?' | 'A'..='Z' | 'a'..='z' => u8::try_from(c).unwrap_or(0x3F),
        _ => 0x3F,
    };
    septets.push(base);
}

/// Texte en septets GSM-7.
pub fn en_septets(texte: &str) -> Vec<u8> {
    let mut septets = Vec::with_capacity(texte.len());
    for c in texte.chars() {
        ajouter_septets(c, &mut septets);
    }
    septets
}

/// Septets découpés en parties : une seule jusqu'à 160, sinon des parties de 153 sans couper une
/// séquence d'échappement.
pub fn decouper(septets: &[u8]) -> Vec<Vec<u8>> {
    if septets.len() <= CARACTERES_SMS_SEUL {
        return vec![septets.to_vec()];
    }
    let mut parties = Vec::new();
    let mut debut = 0;
    while debut < septets.len() {
        let mut fin = (debut + CARACTERES_PAR_PARTIE).min(septets.len());
        if fin < septets.len() && septets.get(fin - 1) == Some(&ECHAPPEMENT) {
            fin -= 1;
        }
        parties.push(septets.get(debut..fin).unwrap_or_default().to_vec());
        debut = fin;
    }
    parties
}

/// Septets tassés sur huit bits, précédés de `bits_avant` bits vides (le remplissage après
/// l'en-tête de concaténation).
fn tasser(septets: &[u8], bits_avant: usize) -> Vec<u8> {
    let total = bits_avant + septets.len() * 7;
    let mut octets = vec![0u8; total.div_ceil(8)];
    for (i, s) in septets.iter().enumerate() {
        let bit = bits_avant + i * 7;
        let (index, decalage) = (bit / 8, bit % 8);
        let valeur = u16::from(s & 0x7F) << decalage;
        if let Some(o) = octets.get_mut(index) {
            *o |= (valeur & 0xFF) as u8;
        }
        if let Some(o) = octets.get_mut(index + 1) {
            *o |= (valeur >> 8) as u8;
        }
    }
    octets
}

/// Septets lus depuis des octets tassés.
fn detasser(octets: &[u8], nombre: usize, bits_avant: usize) -> Vec<u8> {
    (0..nombre)
        .map(|i| {
            let bit = bits_avant + i * 7;
            let (index, decalage) = (bit / 8, bit % 8);
            let bas = u16::from(octets.get(index).copied().unwrap_or(0));
            let haut = u16::from(octets.get(index + 1).copied().unwrap_or(0));
            (((haut << 8 | bas) >> decalage) & 0x7F) as u8
        })
        .collect()
}

/// Numéro canonique `+223…` en adresse : nombre de chiffres, type international, chiffres
/// inversés deux à deux.
fn adresse(numero: &str) -> Vec<u8> {
    let chiffres: Vec<u8> = numero
        .bytes()
        .filter(u8::is_ascii_digit)
        .map(|b| b - b'0')
        .collect();
    let mut octets = vec![u8::try_from(chiffres.len()).unwrap_or(0), 0x91];
    for paire in chiffres.chunks(2) {
        let premier = paire.first().copied().unwrap_or(0xF);
        let second = paire.get(1).copied().unwrap_or(0xF);
        octets.push(second << 4 | premier);
    }
    octets
}

/// Numéro lu d'une adresse, `+` devant s'il est international.
fn lire_adresse(octets: &[u8]) -> Option<(String, usize)> {
    let chiffres = usize::from(*octets.first()?);
    let type_adresse = *octets.get(1)?;
    let longueur = 2 + chiffres.div_ceil(2);
    let mut numero = String::new();
    if type_adresse & 0x70 == 0x10 {
        numero.push('+');
    }
    for o in octets.get(2..longueur)? {
        for demi in [o & 0x0F, o >> 4] {
            if demi < 10 {
                numero.push(char::from(b'0' + demi));
            }
        }
    }
    Some((numero, longueur))
}

fn en_hexa(octets: &[u8]) -> String {
    octets.iter().fold(String::new(), |mut s, o| {
        let _ = write!(s, "{o:02X}");
        s
    })
}

fn depuis_hexa(texte: &str) -> Option<Vec<u8>> {
    let texte = texte.trim();
    if !texte.len().is_multiple_of(2) {
        return None;
    }
    (0..texte.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(texte.get(i..i + 2)?, 16).ok())
        .collect()
}

/// Trame d'une partie, prête pour `AT+CMGS` : son texte hexadécimal et sa longueur en octets sans
/// le centre de messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trame {
    /// Trame en hexadécimal, centre de messages de la puce compris (`00`).
    pub hexa: String,
    /// Longueur annoncée à `AT+CMGS`.
    pub longueur: usize,
}

/// Trames d'un SMS vers un numéro : une par partie, l'accusé de remise demandé sur la dernière.
/// `reference` relie les parties d'un même message.
pub fn trames(numero: &str, texte: &str, reference: u8) -> Vec<Trame> {
    let parties = decouper(&en_septets(texte));
    let nombre = u8::try_from(parties.len()).unwrap_or(u8::MAX);
    parties
        .iter()
        .enumerate()
        .map(|(i, septets)| {
            let derniere = i + 1 == parties.len();
            let concatene = nombre > 1;
            // SMS-SUBMIT, validité relative ; accusé demandé ; en-tête si plusieurs parties.
            let mut premier = 0x01 | 0x10;
            if derniere {
                premier |= 0x20;
            }
            if concatene {
                premier |= 0x40;
            }
            let mut tpdu = vec![premier, 0x00];
            tpdu.extend(adresse(numero));
            // Protocole 0, codage GSM-7, validité de 24 heures.
            tpdu.extend([0x00, 0x00, 0xA7]);
            if concatene {
                let numero_partie = u8::try_from(i + 1).unwrap_or(u8::MAX);
                let entete = [0x05, 0x00, 0x03, reference, nombre, numero_partie];
                // Six octets d'en-tête, soit sept septets avec le bit de remplissage.
                tpdu.push(u8::try_from(7 + septets.len()).unwrap_or(u8::MAX));
                tpdu.extend(entete);
                tpdu.extend(tasser(septets, 1));
            } else {
                tpdu.push(u8::try_from(septets.len()).unwrap_or(u8::MAX));
                tpdu.extend(tasser(septets, 0));
            }
            Trame {
                hexa: format!("00{}", en_hexa(&tpdu)),
                longueur: tpdu.len(),
            }
        })
        .collect()
}

/// Ce qu'une trame SMS-SUBMIT contient : le numéro, le texte de la partie, l'accusé demandé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Soumission {
    /// Numéro du destinataire.
    pub numero: String,
    /// Septets du texte de cette partie.
    pub septets: Vec<u8>,
    /// Vrai si l'accusé de remise est demandé.
    pub accuse_demande: bool,
}

/// Lecture d'une trame SMS-SUBMIT (le modem simulé s'en sert pour savoir à qui l'on écrit).
pub fn lire_soumission(hexa: &str) -> Option<Soumission> {
    let octets = depuis_hexa(hexa)?;
    let centre = usize::from(*octets.first()?);
    let tpdu = octets.get(1 + centre..)?;
    let premier = *tpdu.first()?;
    if premier & 0x03 != 0x01 {
        return None;
    }
    let (numero, longueur) = lire_adresse(tpdu.get(2..)?)?;
    let mut position = 2 + longueur + 2;
    if premier & 0x18 == 0x10 {
        position += 1;
    }
    let udl = usize::from(*tpdu.get(position)?);
    let ud = tpdu.get(position + 1..)?;
    let septets = if premier & 0x40 != 0 {
        let entete = usize::from(*ud.first()?) + 1;
        let septets_entete = (entete * 8).div_ceil(7);
        let bits_avant = septets_entete * 7 - entete * 8;
        detasser(
            ud.get(entete..)?,
            udl.checked_sub(septets_entete)?,
            bits_avant,
        )
    } else {
        detasser(ud, udl, 0)
    };
    Some(Soumission {
        numero,
        septets,
        accuse_demande: premier & 0x20 != 0,
    })
}

/// Septets GSM-7 relus en texte (base et extension que la file emploie).
pub fn depuis_septets(septets: &[u8]) -> String {
    let mut texte = String::new();
    let mut iter = septets.iter();
    while let Some(&s) = iter.next() {
        let c = match s {
            ECHAPPEMENT => match iter.next() {
                Some(0x14) => '^',
                Some(0x28) => '{',
                Some(0x29) => '}',
                Some(0x2F) => '\\',
                Some(0x3C) => '[',
                Some(0x3D) => '~',
                Some(0x3E) => ']',
                Some(0x40) => '|',
                Some(0x65) => '€',
                _ => '?',
            },
            0x00 => '@',
            0x02 => '$',
            0x11 => '_',
            0x0A => '\n',
            0x0D => '\r',
            0x20..=0x23 | 0x25..=0x3F | 0x41..=0x5A | 0x61..=0x7A => char::from(s),
            _ => '?',
        };
        texte.push(c);
    }
    texte
}

/// Accusé de remise lu d'une trame SMS-STATUS-REPORT : la référence du message chez la clé, et
/// le statut s'il est définitif (vrai : remis ; faux : ne sera pas remis). Un statut provisoire
/// (le réseau réessaie) ne dit encore rien.
pub fn lire_accuse(hexa: &str) -> Option<(u8, Option<bool>)> {
    let octets = depuis_hexa(hexa)?;
    let centre = usize::from(*octets.first()?);
    let tpdu = octets.get(1 + centre..)?;
    if tpdu.first()? & 0x03 != 0x02 {
        return None;
    }
    let reference = *tpdu.get(1)?;
    let (_, longueur) = lire_adresse(tpdu.get(2..)?)?;
    // Après l'adresse : horodatage du centre (7 octets), de la remise (7), puis le statut.
    let statut = *tpdu.get(2 + longueur + 14)?;
    let definitif = match statut {
        0x00..=0x1F => Some(true),
        0x20..=0x3F => None,
        _ => Some(false),
    };
    Some((reference, definitif))
}

/// Trame d'un accusé de remise (le modem simulé en produit).
pub fn trame_accuse(reference: u8, numero: &str, remis: bool) -> (String, usize) {
    let mut tpdu = vec![0x06, reference];
    tpdu.extend(adresse(numero));
    tpdu.extend([0u8; 14]);
    tpdu.push(if remis { 0x00 } else { 0x41 });
    (format!("00{}", en_hexa(&tpdu)), tpdu.len())
}

/// Erreur d'envoi d'après le code `+CMS ERROR` de la clé. Les codes du crédit épuisé varient d'un
/// opérateur à l'autre : ceux-ci sont à confirmer sur une vraie clé (essai différé).
pub const fn erreur_cms(code: u16) -> ErreurEnvoi {
    match code {
        // Numéro non attribué, abonné inconnu, adresse invalide.
        1 | 28 | 304 => ErreurEnvoi::NumeroRefuse,
        // Envoi refusé par le réseau, service non souscrit : le plus souvent, plus de crédit.
        21 | 50 => ErreurEnvoi::Credit,
        _ => ErreurEnvoi::Reseau,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tassement_de_l_exemple_classique() {
        // « hellohello » tassé : vecteur des manuels de codage PDU.
        assert_eq!(
            en_hexa(&tasser(&en_septets("hellohello"), 0)),
            "E8329BFD4697D9EC37"
        );
        assert_eq!(
            depuis_septets(&detasser(&tasser(&en_septets("hellohello"), 0), 10, 0)),
            "hellohello"
        );
    }

    #[test]
    fn alphabet_et_extension() {
        assert_eq!(en_septets("@$_"), [0x00, 0x02, 0x11]);
        assert_eq!(en_septets("[€]"), [0x1B, 0x3C, 0x1B, 0x65, 0x1B, 0x3E]);
        assert_eq!(depuis_septets(&en_septets("A{b}~1 000 F")), "A{b}~1 000 F");
    }

    #[test]
    fn une_trame_simple_se_relit() {
        let t = trames("+22376123456", "Bulletin disponible", 7);
        assert_eq!(t.len(), 1);
        let trame = t.first().cloned().unwrap_or(Trame {
            hexa: String::new(),
            longueur: 0,
        });
        assert!(trame.hexa.starts_with("0031000B91"), "{}", trame.hexa);
        assert_eq!(trame.longueur * 2 + 2, trame.hexa.len());
        let s = lire_soumission(&trame.hexa);
        assert_eq!(
            s,
            Some(Soumission {
                numero: "+22376123456".to_owned(),
                septets: en_septets("Bulletin disponible"),
                accuse_demande: true,
            })
        );
    }

    #[test]
    fn un_long_texte_part_en_parties_recollables() {
        let texte = "a".repeat(200) + &"b".repeat(150);
        let t = trames("+22376123456", &texte, 42);
        assert_eq!(t.len(), 3);
        let lues: Vec<Soumission> = t.iter().filter_map(|p| lire_soumission(&p.hexa)).collect();
        assert_eq!(lues.len(), 3);
        let recolle: String = lues.iter().map(|s| depuis_septets(&s.septets)).collect();
        assert_eq!(recolle, texte);
        // L'accusé n'est demandé que sur la dernière partie.
        let demandes: Vec<bool> = lues.iter().map(|s| s.accuse_demande).collect();
        assert_eq!(demandes, [false, false, true]);
    }

    #[test]
    fn une_sequence_d_echappement_ne_se_coupe_pas() {
        let mut septets = vec![0x61; 152];
        septets.extend([ECHAPPEMENT, 0x65]);
        septets.extend([0x61; 10]);
        let parties = decouper(&septets);
        assert_eq!(parties.first().map(Vec::len), Some(152));
        assert_eq!(parties.get(1).and_then(|p| p.first()), Some(&ECHAPPEMENT));
    }

    #[test]
    fn accuses_de_remise() {
        let (remis, _) = trame_accuse(9, "+22376123456", true);
        assert_eq!(lire_accuse(&remis), Some((9, Some(true))));
        let (perdu, _) = trame_accuse(10, "+22376123456", false);
        assert_eq!(lire_accuse(&perdu), Some((10, Some(false))));
        assert_eq!(lire_accuse("zz"), None);
    }

    #[test]
    fn codes_d_erreur() {
        assert_eq!(erreur_cms(1), ErreurEnvoi::NumeroRefuse);
        assert_eq!(erreur_cms(21), ErreurEnvoi::Credit);
        assert_eq!(erreur_cms(38), ErreurEnvoi::Reseau);
    }
}
