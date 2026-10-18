//! Conversation en commandes AT avec une clé modem (3GPP TS 27.005 et 27.007) : une commande, ses
//! lignes de réponse jusqu'à `OK` ou une erreur ; l'envoi d'une trame après l'invite `>` ; les
//! lignes que la clé envoie d'elle-même (accusés de remise `+CDS`, réponses USSD `+CUSD`, annonce
//! d'un SMS reçu `+CMTI`) ; la lecture et l'effacement des SMS reçus (`AT+CMGL`, `AT+CMGD`).

use std::io::{self, Read, Write};
use std::time::{Duration, Instant};

use crate::pdu::Trame;

/// Liaison avec la clé : un port série, ou le modem simulé.
pub trait Port: Send {
    /// Écrit des octets vers la clé.
    ///
    /// # Errors
    /// La clé est débranchée ou le port fermé.
    fn ecrire(&mut self, octets: &[u8]) -> io::Result<()>;

    /// Lit ce que la clé a envoyé, en attendant au plus `delai` ; vide si rien n'est venu.
    ///
    /// # Errors
    /// La clé est débranchée ou le port fermé.
    fn lire(&mut self, delai: Duration) -> io::Result<Vec<u8>>;
}

/// Port série d'une vraie clé.
pub struct PortSerie {
    port: Box<dyn serialport::SerialPort>,
}

impl PortSerie {
    /// Ouvre un port série (`COM3`) à 115 200 bauds.
    ///
    /// # Errors
    /// Le port n'existe pas ou est déjà pris.
    pub fn ouvrir(nom: &str) -> io::Result<Self> {
        let port = serialport::new(nom, 115_200)
            .timeout(Duration::from_millis(200))
            .open()
            .map_err(io::Error::other)?;
        Ok(Self { port })
    }
}

impl Port for PortSerie {
    fn ecrire(&mut self, octets: &[u8]) -> io::Result<()> {
        self.port.write_all(octets)?;
        self.port.flush()
    }

    fn lire(&mut self, delai: Duration) -> io::Result<Vec<u8>> {
        self.port
            .set_timeout(delai.max(Duration::from_millis(1)))
            .map_err(io::Error::other)?;
        let mut tampon = [0u8; 1024];
        match self.port.read(&mut tampon) {
            Ok(n) => Ok(tampon.get(..n).unwrap_or_default().to_vec()),
            Err(e) if e.kind() == io::ErrorKind::TimedOut => Ok(Vec::new()),
            Err(e) => Err(e),
        }
    }
}

/// Noms des ports série du poste.
pub fn ports_du_poste() -> Vec<String> {
    serialport::available_ports()
        .map(|ports| ports.into_iter().map(|p| p.port_name).collect())
        .unwrap_or_default()
}

/// Échec d'une commande.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErreurAt {
    /// La clé n'a pas répondu à temps.
    Delai,
    /// `ERROR`.
    Erreur,
    /// `+CMS ERROR: n`, erreur du service des SMS.
    Cms(u16),
    /// `+CME ERROR: n`, erreur de l'équipement.
    Cme(u16),
    /// Le port ne s'ouvre pas, ou s'est fermé.
    Port(String),
}

/// Ligne que la clé envoie d'elle-même.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NonSollicite {
    /// Trame d'un accusé de remise, en hexadécimal.
    Accuse(String),
    /// Ligne `+CUSD:` d'une réponse USSD.
    Ussd(String),
}

/// Session ouverte avec une clé.
pub struct Session {
    port: Box<dyn Port>,
    tampon: String,
    attente_accuse: bool,
    non_sollicites: Vec<NonSollicite>,
    delai: Duration,
    sms_a_lire: bool,
}

fn erreur_port(e: &io::Error) -> ErreurAt {
    ErreurAt::Port(e.to_string())
}

/// Code d'une ligne `+CMS ERROR: 21`.
fn code(ligne: &str, prefixe: &str) -> Option<u16> {
    ligne.strip_prefix(prefixe)?.trim().parse().ok()
}

impl Session {
    /// Session sur un port ; `delai` borne l'attente de chaque réponse.
    pub fn nouvelle(port: Box<dyn Port>, delai: Duration) -> Self {
        Self {
            port,
            tampon: String::new(),
            attente_accuse: false,
            non_sollicites: Vec::new(),
            delai,
            // À l'ouverture, la puce peut garder des SMS reçus pendant que GESCO était fermé.
            sms_a_lire: true,
        }
    }

    /// Prépare la clé : écho coupé, mode PDU, accusés de remise signalés aussitôt (`+CDS`).
    ///
    /// # Errors
    /// La clé ne répond pas ou refuse une commande.
    pub fn preparer(&mut self) -> Result<(), ErreurAt> {
        for commande in ["AT", "ATE0", "AT+CMGF=0", "AT+CNMI=2,1,0,1,0"] {
            self.commande(commande)?;
        }
        Ok(())
    }

    /// Lignes que la clé a envoyées d'elle-même depuis le dernier appel.
    pub fn non_sollicites(&mut self) -> Vec<NonSollicite> {
        std::mem::take(&mut self.non_sollicites)
    }

    /// Lit ce qui est arrivé et rend les lignes complètes qui répondent à une commande ; les
    /// lignes non sollicitées sont mises de côté.
    ///
    /// # Errors
    /// Le port s'est fermé.
    pub fn lire_lignes(&mut self, delai: Duration) -> Result<Vec<String>, ErreurAt> {
        let octets = self.port.lire(delai).map_err(|e| erreur_port(&e))?;
        self.tampon.push_str(&String::from_utf8_lossy(&octets));
        let mut lignes = Vec::new();
        while let Some(fin) = self.tampon.find('\n') {
            let ligne: String = self.tampon.drain(..=fin).collect();
            let ligne = ligne.trim().to_owned();
            if ligne.is_empty() {
                continue;
            }
            if self.attente_accuse {
                self.attente_accuse = false;
                self.non_sollicites.push(NonSollicite::Accuse(ligne));
            } else if ligne.starts_with("+CDS:") {
                self.attente_accuse = true;
            } else if ligne.starts_with("+CUSD:") {
                self.non_sollicites.push(NonSollicite::Ussd(ligne));
            } else if ligne.starts_with("+CMTI:") {
                self.sms_a_lire = true;
            } else if ligne == "RING" {
                // Un appel : la clé ne décroche pas.
            } else {
                lignes.push(ligne);
            }
        }
        Ok(lignes)
    }

    /// Attend la fin d'une réponse : ses lignes jusqu'à `OK`, ou l'erreur.
    fn reponse(&mut self, commande: &str) -> Result<Vec<String>, ErreurAt> {
        let fin = Instant::now() + self.delai;
        let mut reponse = Vec::new();
        loop {
            for ligne in self.lire_lignes(Duration::from_millis(50))? {
                if ligne == "OK" {
                    return Ok(reponse);
                }
                if ligne == "ERROR" {
                    return Err(ErreurAt::Erreur);
                }
                if let Some(n) = code(&ligne, "+CMS ERROR:") {
                    return Err(ErreurAt::Cms(n));
                }
                if let Some(n) = code(&ligne, "+CME ERROR:") {
                    return Err(ErreurAt::Cme(n));
                }
                if ligne != commande {
                    reponse.push(ligne);
                }
            }
            if Instant::now() > fin {
                return Err(ErreurAt::Delai);
            }
        }
    }

    /// Envoie une commande et rend ses lignes de réponse.
    ///
    /// # Errors
    /// Pas de réponse à temps, ou une erreur de la clé.
    pub fn commande(&mut self, commande: &str) -> Result<Vec<String>, ErreurAt> {
        self.port
            .ecrire(format!("{commande}\r").as_bytes())
            .map_err(|e| erreur_port(&e))?;
        self.reponse(commande)
    }

    /// Envoie une trame et rend la référence que la clé lui donne (`+CMGS: n`).
    ///
    /// # Errors
    /// Pas d'invite ni de réponse à temps, ou une erreur du réseau (`+CMS ERROR`).
    pub fn envoyer_trame(&mut self, trame: &Trame) -> Result<u8, ErreurAt> {
        let commande = format!("AT+CMGS={}", trame.longueur);
        self.port
            .ecrire(format!("{commande}\r").as_bytes())
            .map_err(|e| erreur_port(&e))?;
        let fin = Instant::now() + self.delai;
        loop {
            for ligne in self.lire_lignes(Duration::from_millis(50))? {
                if ligne == "ERROR" {
                    return Err(ErreurAt::Erreur);
                }
                if let Some(n) = code(&ligne, "+CMS ERROR:") {
                    return Err(ErreurAt::Cms(n));
                }
            }
            if let Some(invite) = self.tampon.find('>') {
                self.tampon.drain(..=invite);
                break;
            }
            if Instant::now() > fin {
                return Err(ErreurAt::Delai);
            }
        }
        self.port
            .ecrire(format!("{}\u{1a}", trame.hexa).as_bytes())
            .map_err(|e| erreur_port(&e))?;
        let reponse = self.reponse(&commande)?;
        reponse
            .iter()
            .find_map(|l| code(l, "+CMGS:"))
            .and_then(|n| u8::try_from(n).ok())
            .ok_or(ErreurAt::Erreur)
    }

    /// SMS reçus que la puce garde, s'il en est arrivé depuis la dernière lecture : chacun est lu
    /// en mode PDU puis effacé de la puce, qui n'en garde qu'une vingtaine. Rend les trames.
    ///
    /// # Errors
    /// La clé ne répond pas ou refuse la lecture.
    pub fn lire_sms_recus(&mut self) -> Result<Vec<String>, ErreurAt> {
        // Une lecture brève ramasse les annonces arrivées entre deux passages.
        self.lire_lignes(Duration::from_millis(10))?;
        if !self.sms_a_lire {
            return Ok(Vec::new());
        }
        let lignes = self.commande("AT+CMGL=4")?;
        let mut trames = Vec::new();
        let mut index = Vec::new();
        let mut iter = lignes.iter();
        while let Some(ligne) = iter.next() {
            let Some(reste) = ligne.strip_prefix("+CMGL:") else {
                continue;
            };
            if let Some(n) = reste
                .split(',')
                .next()
                .and_then(|n| n.trim().parse::<u32>().ok())
            {
                index.push(n);
            }
            if let Some(trame) = iter.next() {
                trames.push(trame.clone());
            }
        }
        for n in index {
            self.commande(&format!("AT+CMGD={n}"))?;
        }
        self.sms_a_lire = false;
        Ok(trames)
    }

    /// Envoie un code USSD (`#111#`) et rend le texte de la réponse de l'opérateur.
    ///
    /// # Errors
    /// La clé refuse la demande ou l'opérateur ne répond pas à temps.
    pub fn ussd(&mut self, code_ussd: &str) -> Result<String, ErreurAt> {
        self.commande(&format!("AT+CUSD=1,\"{code_ussd}\",15"))?;
        let fin = Instant::now() + self.delai;
        loop {
            let position = self
                .non_sollicites
                .iter()
                .position(|n| matches!(n, NonSollicite::Ussd(_)));
            if let Some(i) = position {
                let NonSollicite::Ussd(ligne) = self.non_sollicites.remove(i) else {
                    continue;
                };
                return Ok(texte_ussd(&ligne));
            }
            if Instant::now() > fin {
                return Err(ErreurAt::Delai);
            }
            self.lire_lignes(Duration::from_millis(50))?;
        }
    }
}

/// Texte d'une ligne `+CUSD: 0,"Votre solde est de 1 250 FCFA.",15`.
pub fn texte_ussd(ligne: &str) -> String {
    match (ligne.find('"'), ligne.rfind('"')) {
        (Some(debut), Some(fin)) if fin > debut => {
            ligne.get(debut + 1..fin).unwrap_or_default().to_owned()
        }
        _ => ligne.to_owned(),
    }
}

/// Montant lu dans la réponse de l'opérateur : le premier nombre, espaces et points de milliers
/// ignorés (« 1 250 FCFA », « 1.250F »).
pub fn lire_montant(texte: &str) -> Option<i64> {
    let debut = texte.find(|c: char| c.is_ascii_digit())?;
    let mut chiffres = String::new();
    let mut caracteres = texte.get(debut..)?.chars().peekable();
    while let Some(c) = caracteres.next() {
        if c.is_ascii_digit() {
            chiffres.push(c);
        } else if matches!(c, ' ' | '.' | '\u{a0}' | '\u{202f}')
            && caracteres.peek().is_some_and(char::is_ascii_digit)
        {
            continue;
        } else {
            break;
        }
    }
    chiffres.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn montants_lus_dans_les_reponses_usd() {
        assert_eq!(lire_montant("Votre solde est de 1 250 FCFA."), Some(1_250));
        assert_eq!(lire_montant("Solde: 1.250F valable"), Some(1_250));
        assert_eq!(lire_montant("Credit 75 F, bonus 10"), Some(75));
        assert_eq!(lire_montant("Service indisponible"), None);
        assert_eq!(
            texte_ussd("+CUSD: 0,\"Votre solde est de 1 250 FCFA.\",15"),
            "Votre solde est de 1 250 FCFA."
        );
    }
}
