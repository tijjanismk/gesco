//! Canal de la clé modem GSM (SPEC_PHASE_5 §4) : la session s'ouvre au premier besoin, se prépare
//! (mode PDU, accusés signalés), et se rouvre après une coupure ; une clé débranchée rend
//! l'envoi impossible sans rien perdre, la file réessaie. Le crédit se lit par USSD.

use std::io;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;

use gesco_core::notification::ErreurEnvoi;

use crate::at::{ErreurAt, NonSollicite, Port, PortSerie, Session, lire_montant};
use crate::pdu::{erreur_cms, lire_accuse, trames};
use crate::{Accuse, Canal};

/// Attente de chaque réponse de la clé : un envoi prend quelques secondes sur le réseau.
const DELAI_REPONSE: Duration = Duration::from_secs(30);

/// Ce que la clé dit d'elle-même.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InfoCle {
    /// Fabricant.
    pub fabricant: String,
    /// Modèle.
    pub modele: String,
    /// Réseau sur lequel elle est inscrite.
    pub operateur: String,
    /// Force du signal, de 0 à 100 ; aucune si la clé ne la connaît pas.
    pub signal: Option<u8>,
}

/// Crédit lu par USSD.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LectureCredit {
    /// Montant lu dans la réponse, s'il s'y trouve.
    pub montant: Option<i64>,
    /// Réponse de l'opérateur, telle quelle.
    pub texte: String,
}

type Ouvreur = Box<dyn Fn() -> io::Result<Box<dyn Port>> + Send + Sync>;

/// Canal d'une clé modem.
pub struct CanalModem {
    ouvrir: Ouvreur,
    session: Mutex<Option<Session>>,
    accuses: Mutex<Vec<Accuse>>,
    concatenation: AtomicU8,
    delai: Duration,
}

/// Référence d'un message chez la clé, telle que la file la garde.
fn reference(mr: u8) -> String {
    format!("modem-{mr}")
}

impl CanalModem {
    /// Canal sur une liaison quelconque, ouverte à la demande.
    pub fn nouveau(ouvrir: Ouvreur) -> Self {
        Self {
            ouvrir,
            session: Mutex::new(None),
            accuses: Mutex::new(Vec::new()),
            concatenation: AtomicU8::new(0),
            delai: DELAI_REPONSE,
        }
    }

    /// Canal sur le port série d'une vraie clé (`COM3`).
    pub fn sur_port_serie(nom: &str) -> Self {
        let nom = nom.to_owned();
        Self::nouveau(Box::new(move || {
            PortSerie::ouvrir(&nom).map(|p| Box::new(p) as Box<dyn Port>)
        }))
    }

    /// Délai de réponse raccourci (le modem simulé répond aussitôt).
    #[must_use]
    pub fn avec_delai(mut self, delai: Duration) -> Self {
        self.delai = delai;
        self
    }

    /// Exécute une opération sur la session, ouverte et préparée au besoin ; une coupure la
    /// referme pour la rouvrir au prochain appel. Les accusés arrivés sont mis de côté.
    fn avec_session<T>(
        &self,
        operation: impl FnOnce(&mut Session) -> Result<T, ErreurAt>,
    ) -> Result<T, ErreurAt> {
        let mut garde = self
            .session
            .lock()
            .map_err(|_| ErreurAt::Port("session verrouillée".to_owned()))?;
        if garde.is_none() {
            let port = (self.ouvrir)().map_err(|e| ErreurAt::Port(e.to_string()))?;
            let mut session = Session::nouvelle(port, self.delai);
            session.preparer()?;
            *garde = Some(session);
        }
        let Some(session) = garde.as_mut() else {
            return Err(ErreurAt::Port("session absente".to_owned()));
        };
        let resultat = operation(session);
        let arrives = session.non_sollicites();
        if matches!(resultat, Err(ErreurAt::Delai | ErreurAt::Port(_))) {
            *garde = None;
        }
        drop(garde);
        let nouveaux: Vec<Accuse> = arrives
            .into_iter()
            .filter_map(|n| match n {
                NonSollicite::Accuse(hexa) => lire_accuse(&hexa),
                NonSollicite::Ussd(_) => None,
            })
            .filter_map(|(mr, definitif)| {
                definitif.map(|remis| Accuse {
                    reference: reference(mr),
                    remis,
                })
            })
            .collect();
        if let Ok(mut accuses) = self.accuses.lock() {
            accuses.extend(nouveaux);
        }
        resultat
    }

    /// Vérifie que la clé répond.
    ///
    /// # Errors
    /// La clé est absente ou ne répond pas.
    pub fn verifier(&self) -> Result<(), ErreurAt> {
        self.avec_session(|s| s.commande("AT").map(|_| ()))
    }

    /// Fabricant, modèle, réseau et signal de la clé.
    ///
    /// # Errors
    /// La clé est absente ou ne répond pas.
    pub fn identifier(&self) -> Result<InfoCle, ErreurAt> {
        self.avec_session(|s| {
            let premiere = |lignes: Vec<String>| lignes.into_iter().next().unwrap_or_default();
            let fabricant = premiere(s.commande("AT+CGMI")?);
            let modele = premiere(s.commande("AT+CGMM")?);
            let signal = s
                .commande("AT+CSQ")?
                .iter()
                .find_map(|l| l.strip_prefix("+CSQ:"))
                .and_then(|v| v.split(',').next())
                .and_then(|v| v.trim().parse::<u16>().ok())
                .filter(|&rssi| rssi <= 31)
                .map(|rssi| u8::try_from(rssi * 100 / 31).unwrap_or(100));
            let operateur = s
                .commande("AT+COPS?")?
                .iter()
                .find_map(|l| l.strip_prefix("+COPS:"))
                .and_then(|v| {
                    let debut = v.find('"')?;
                    let fin = v.rfind('"')?;
                    v.get(debut + 1..fin).map(str::to_owned)
                })
                .unwrap_or_default();
            Ok(InfoCle {
                fabricant,
                modele,
                operateur,
                signal,
            })
        })
    }

    /// Crédit de la puce, lu par le code USSD de l'opérateur.
    ///
    /// # Errors
    /// La clé est absente, ou l'opérateur ne répond pas.
    pub fn lire_credit(&self, code_ussd: &str) -> Result<LectureCredit, ErreurAt> {
        self.avec_session(|s| s.ussd(code_ussd))
            .map(|texte| LectureCredit {
                montant: lire_montant(&texte),
                texte,
            })
    }
}

impl Canal for CanalModem {
    fn envoyer(&self, numero: &str, texte: &str) -> Result<String, ErreurEnvoi> {
        let concatenation = self.concatenation.fetch_add(1, Ordering::Relaxed);
        self.avec_session(|s| {
            let mut derniere = None;
            for trame in trames(numero, texte, concatenation) {
                derniere = Some(s.envoyer_trame(&trame)?);
            }
            derniere.ok_or(ErreurAt::Erreur)
        })
        .map(reference)
        .map_err(|e| match e {
            ErreurAt::Cms(code) => erreur_cms(code),
            _ => ErreurEnvoi::Reseau,
        })
    }

    fn accuses(&self) -> Vec<Accuse> {
        // Une lecture brève ramasse les accusés arrivés entre deux passages.
        let _ = self.avec_session(|s| s.lire_lignes(Duration::from_millis(10)).map(|_| ()));
        self.accuses
            .lock()
            .map(|mut a| std::mem::take(&mut *a))
            .unwrap_or_default()
    }
}
