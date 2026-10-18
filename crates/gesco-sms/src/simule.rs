//! Modem simulé (décision du 17 octobre 2026 : le matériel est différé) : il répond aux commandes
//! AT comme une clé, reçoit les trames PDU, rend leur référence et l'accusé de remise, débite un
//! crédit et le dit par USSD. Les essais lui font jouer un numéro refusé, une remise manquée, une
//! coupure du réseau, une clé débranchée, une puce vide.

use std::collections::HashSet;
use std::io;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::at::Port;
use crate::pdu::{depuis_septets, lire_soumission, trame_accuse};

/// Prix d'un SMS sur la puce simulée, en francs.
pub const COUT_SMS: i64 = 25;

#[derive(Debug, Default)]
struct Etat {
    echo: bool,
    entree: Vec<u8>,
    sortie: Vec<u8>,
    attente_trame: bool,
    credit: i64,
    reference: u8,
    non_remis: HashSet<String>,
    refuses: HashSet<String>,
    sans_reseau: bool,
    debranchee: bool,
    envoyes: Vec<(String, String)>,
}

/// Montant écrit à la française : `1 250`.
fn montant(n: i64) -> String {
    let chiffres = n.abs().to_string();
    let mut groupes: Vec<&str> = Vec::new();
    let mut fin = chiffres.len();
    while fin > 0 {
        let debut = fin.saturating_sub(3);
        groupes.push(chiffres.get(debut..fin).unwrap_or_default());
        fin = debut;
    }
    groupes.reverse();
    format!("{}{}", if n < 0 { "-" } else { "" }, groupes.join(" "))
}

impl Etat {
    fn repondre(&mut self, texte: &str) {
        self.sortie.extend_from_slice(texte.as_bytes());
    }

    fn traiter(&mut self) {
        loop {
            if self.attente_trame {
                let Some(fin) = self.entree.iter().position(|&o| o == 0x1A) else {
                    return;
                };
                let trame: Vec<u8> = self.entree.drain(..=fin).collect();
                self.attente_trame = false;
                let hexa =
                    String::from_utf8_lossy(trame.get(..trame.len() - 1).unwrap_or_default())
                        .into_owned();
                self.recevoir_trame(&hexa);
            } else {
                let Some(fin) = self.entree.iter().position(|&o| o == b'\r') else {
                    return;
                };
                let ligne: Vec<u8> = self.entree.drain(..=fin).collect();
                let commande = String::from_utf8_lossy(&ligne).trim().to_owned();
                if self.echo {
                    self.repondre(&format!("{commande}\r\n"));
                }
                self.commande(&commande);
            }
        }
    }

    fn commande(&mut self, commande: &str) {
        match commande {
            "AT" | "AT+CMGF=0" => self.repondre("\r\nOK\r\n"),
            "ATE0" => {
                self.echo = false;
                self.repondre("\r\nOK\r\n");
            }
            c if c.starts_with("AT+CNMI=") => self.repondre("\r\nOK\r\n"),
            "AT+CGMI" => self.repondre("\r\nSIMULE\r\n\r\nOK\r\n"),
            "AT+CGMM" => self.repondre("\r\nCle GSM simulee\r\n\r\nOK\r\n"),
            "AT+CSQ" => {
                let rssi = if self.sans_reseau { 99 } else { 20 };
                self.repondre(&format!("\r\n+CSQ: {rssi},99\r\n\r\nOK\r\n"));
            }
            "AT+COPS?" => self.repondre("\r\n+COPS: 0,0,\"Reseau simule\",2\r\n\r\nOK\r\n"),
            c if c.starts_with("AT+CMGS=") => {
                self.attente_trame = true;
                self.repondre("\r\n> ");
            }
            c if c.starts_with("AT+CUSD=1,") => {
                let solde = montant(self.credit);
                self.repondre(&format!(
                    "\r\nOK\r\n\r\n+CUSD: 0,\"Votre solde est de {solde} FCFA.\",15\r\n"
                ));
            }
            _ => self.repondre("\r\nERROR\r\n"),
        }
    }

    fn recevoir_trame(&mut self, hexa: &str) {
        let Some(soumission) = lire_soumission(hexa) else {
            self.repondre("\r\n+CMS ERROR: 304\r\n");
            return;
        };
        if self.sans_reseau {
            self.repondre("\r\n+CMS ERROR: 38\r\n");
            return;
        }
        if self.refuses.contains(&soumission.numero) {
            self.repondre("\r\n+CMS ERROR: 1\r\n");
            return;
        }
        if self.credit < COUT_SMS {
            self.repondre("\r\n+CMS ERROR: 21\r\n");
            return;
        }
        self.credit -= COUT_SMS;
        self.reference = self.reference.wrapping_add(1);
        let reference = self.reference;
        self.envoyes.push((
            soumission.numero.clone(),
            depuis_septets(&soumission.septets),
        ));
        self.repondre(&format!("\r\n+CMGS: {reference}\r\n\r\nOK\r\n"));
        if soumission.accuse_demande {
            let remis = !self.non_remis.contains(&soumission.numero);
            let (trame, longueur) = trame_accuse(reference, &soumission.numero, remis);
            self.repondre(&format!("\r\n+CDS: {longueur}\r\n{trame}\r\n"));
        }
    }
}

/// Modem simulé, partagé entre ses ports et les essais qui le pilotent.
#[derive(Debug, Clone, Default)]
pub struct ModemSimule {
    etat: Arc<Mutex<Etat>>,
}

struct PortSimule {
    etat: Arc<Mutex<Etat>>,
}

fn debranchee() -> io::Error {
    io::Error::new(io::ErrorKind::NotConnected, "clé débranchée")
}

impl Port for PortSimule {
    fn ecrire(&mut self, octets: &[u8]) -> io::Result<()> {
        let mut etat = self.etat.lock().map_err(|_| debranchee())?;
        if etat.debranchee {
            return Err(debranchee());
        }
        etat.entree.extend_from_slice(octets);
        etat.traiter();
        Ok(())
    }

    fn lire(&mut self, _delai: Duration) -> io::Result<Vec<u8>> {
        let mut etat = self.etat.lock().map_err(|_| debranchee())?;
        if etat.debranchee {
            return Err(debranchee());
        }
        Ok(std::mem::take(&mut etat.sortie))
    }
}

impl ModemSimule {
    /// Modem simulé avec ce crédit, écho allumé comme une clé neuve.
    pub fn nouveau(credit: i64) -> Self {
        let modem = Self::default();
        modem.modifier(|e| {
            e.credit = credit;
            e.echo = true;
        });
        modem
    }

    fn modifier(&self, f: impl FnOnce(&mut Etat)) {
        if let Ok(mut etat) = self.etat.lock() {
            f(&mut etat);
        }
    }

    /// Un port vers ce modem ; une clé débranchée ne s'ouvre pas.
    ///
    /// # Errors
    /// La clé est débranchée.
    pub fn port(&self) -> io::Result<Box<dyn Port>> {
        let etat = self.etat.lock().map_err(|_| debranchee())?;
        if etat.debranchee {
            return Err(debranchee());
        }
        drop(etat);
        Ok(Box::new(PortSimule {
            etat: Arc::clone(&self.etat),
        }))
    }

    /// Crédit restant.
    pub fn credit(&self) -> i64 {
        self.etat.lock().map(|e| e.credit).unwrap_or(0)
    }

    /// Recharge la puce.
    pub fn recharger(&self, montant: i64) {
        self.modifier(|e| e.credit += montant);
    }

    /// Fixe le crédit.
    pub fn fixer_credit(&self, credit: i64) {
        self.modifier(|e| e.credit = credit);
    }

    /// Le réseau refusera ce numéro.
    pub fn refuser(&self, numero: &str) {
        self.modifier(|e| {
            e.refuses.insert(numero.to_owned());
        });
    }

    /// Les SMS à ce numéro partiront sans être remis.
    pub fn ne_pas_remettre(&self, numero: &str) {
        self.modifier(|e| {
            e.non_remis.insert(numero.to_owned());
        });
    }

    /// Coupe ou rétablit le réseau.
    pub fn couper_reseau(&self, coupe: bool) {
        self.modifier(|e| e.sans_reseau = coupe);
    }

    /// Débranche ou rebranche la clé ; rebranchée, elle repart écho allumé.
    pub fn debrancher(&self, debranchee: bool) {
        self.modifier(|e| {
            e.debranchee = debranchee;
            if !debranchee {
                e.echo = true;
                e.entree.clear();
                e.sortie.clear();
                e.attente_trame = false;
            }
        });
    }

    /// SMS partis, numéro et texte de chaque partie, dans l'ordre.
    pub fn envoyes(&self) -> Vec<(String, String)> {
        self.etat
            .lock()
            .map(|e| e.envoyes.clone())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Canal;
    use crate::modem::CanalModem;
    use gesco_core::notification::ErreurEnvoi;

    fn canal(modem: &ModemSimule) -> CanalModem {
        let m = modem.clone();
        CanalModem::nouveau(Box::new(move || m.port())).avec_delai(Duration::from_millis(200))
    }

    #[test]
    fn un_sms_part_et_son_accuse_revient() {
        let modem = ModemSimule::nouveau(1_000);
        let c = canal(&modem);
        let r = c.envoyer("+22376123456", "Bulletin disponible");
        assert_eq!(r, Ok("modem-1".to_owned()));
        assert_eq!(
            modem.envoyes(),
            [("+22376123456".to_owned(), "Bulletin disponible".to_owned())]
        );
        assert_eq!(modem.credit(), 1_000 - COUT_SMS);
        let accuses = c.accuses();
        assert_eq!(accuses.len(), 1);
        assert!(accuses.iter().all(|a| a.remis && a.reference == "modem-1"));
    }

    #[test]
    fn un_long_sms_part_en_trois_parties_et_un_seul_accuse() {
        let modem = ModemSimule::nouveau(1_000);
        let c = canal(&modem);
        let texte = "x".repeat(400);
        assert_eq!(c.envoyer("+22376123456", &texte), Ok("modem-3".to_owned()));
        assert_eq!(modem.envoyes().len(), 3);
        assert_eq!(c.accuses().len(), 1);
        assert_eq!(modem.credit(), 1_000 - 3 * COUT_SMS);
    }

    #[test]
    fn erreurs_du_reseau_et_de_la_puce() {
        let modem = ModemSimule::nouveau(30);
        let c = canal(&modem);
        modem.refuser("+22370000000");
        assert_eq!(
            c.envoyer("+22370000000", "x"),
            Err(ErreurEnvoi::NumeroRefuse)
        );
        modem.ne_pas_remettre("+22366000000");
        assert!(c.envoyer("+22366000000", "x").is_ok());
        assert!(c.accuses().iter().all(|a| !a.remis));
        // 30 F moins 25 : la puce n'a plus de quoi envoyer.
        assert_eq!(c.envoyer("+22376123456", "x"), Err(ErreurEnvoi::Credit));
        modem.recharger(500);
        modem.couper_reseau(true);
        assert_eq!(c.envoyer("+22376123456", "x"), Err(ErreurEnvoi::Reseau));
        modem.couper_reseau(false);
        assert!(c.envoyer("+22376123456", "x").is_ok());
    }

    #[test]
    fn une_cle_debranchee_puis_rebranchee() {
        let modem = ModemSimule::nouveau(1_000);
        let c = canal(&modem);
        assert!(c.verifier().is_ok());
        modem.debrancher(true);
        assert_eq!(c.envoyer("+22376123456", "x"), Err(ErreurEnvoi::Reseau));
        assert!(c.verifier().is_err());
        modem.debrancher(false);
        assert!(c.verifier().is_ok());
        assert!(c.envoyer("+22376123456", "x").is_ok());
    }

    #[test]
    fn identite_et_credit_par_ussd() {
        let modem = ModemSimule::nouveau(1_250);
        let c = canal(&modem);
        let info = c.identifier();
        assert_eq!(
            info.map(|i| (i.fabricant, i.modele, i.operateur, i.signal)),
            Ok((
                "SIMULE".to_owned(),
                "Cle GSM simulee".to_owned(),
                "Reseau simule".to_owned(),
                Some(64)
            ))
        );
        let credit = c.lire_credit("#111#");
        assert_eq!(credit.as_ref().map(|l| l.montant), Ok(Some(1_250)));
        assert_eq!(
            credit.map(|l| l.texte),
            Ok("Votre solde est de 1 250 FCFA.".to_owned())
        );
        assert_eq!(montant(1_234_567), "1 234 567");
    }
}
