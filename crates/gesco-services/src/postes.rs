//! Postes de l'école (SPEC_PHASE_11_A §4) : activer le réseau local, relier un poste par un
//! code d'appairage, le reconnaître à chaque appel par son secret, le révoquer, renouveler la clé
//! du réseau. Le certificat lui-même est fabriqué par `gesco-reseau`, qui le dépose ici ; ce
//! service ne garde que ce qui doit survivre dans la base, et le code d'appairage en mémoire.

use std::sync::{Arc, Mutex};

use gesco_core::commun::{Horodatage, Id};
use gesco_core::core::reseau as regles;
use gesco_core::core::reseau::{CodeAppairage, IssueCode};
use gesco_core::core::{
    Etablissement, Operation, Poste, PosteRelie, PostesCouverts, TypeEvenement, Utilisateur,
};
use gesco_db::depots::reseau as depot;
use gesco_db::nouvel_id;
use gesco_db::unite::Durabilite;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::watch;

use crate::erreur::{ErreurService, Resultat};
use crate::licence::ServiceLicence;
use crate::noyau::Noyau;
use crate::session::Session;

/// Écart minimal entre deux notes de la dernière connexion d'un poste : une minute. Chaque appel
/// d'un poste relié n'a pas à écrire dans la base.
const ECART_NOTE_CONNEXION_MILLIS: i64 = 60_000;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Appairage ouvert sur le poste principal : le code en attente, et qui l'a demandé.
struct AppairageOuvert {
    code: CodeAppairage,
    etablissement_id: Id<Etablissement>,
    ouvert_par: Id<Utilisateur>,
}

/// Ce que le réseau local garde en mémoire sur le poste principal : le code d'appairage en
/// attente, qui ne s'écrit jamais dans la base, et le signal qui réveille le serveur quand le
/// réseau s'allume, s'éteint ou change de clé.
pub struct ReseauVivant {
    appairage: Mutex<Option<AppairageOuvert>>,
    signal: watch::Sender<u64>,
    adresses: Mutex<Option<SourceAdresses>>,
}

/// Donne les adresses de ce poste sur le réseau de l'école, à l'instant où on les demande : elles
/// changent quand l'ordinateur change de réseau. Fournie par le binaire du poste.
pub type SourceAdresses = Arc<dyn Fn() -> Vec<String> + Send + Sync>;

impl Default for ReseauVivant {
    fn default() -> Self {
        let (signal, _) = watch::channel(0);
        Self {
            appairage: Mutex::new(None),
            signal,
            adresses: Mutex::new(None),
        }
    }
}

/// Ce que donne un code présenté par un poste qui veut se relier.
enum Presentation {
    /// Aucun appairage n'est ouvert pour cet établissement.
    Aucun,
    /// Code juste : qui a ouvert l'appairage. Le code reste en attente jusqu'à la liaison.
    Accepte(Id<Utilisateur>),
    /// Code refusé ; faux une fois de trop ou expiré, il est oublié.
    Refuse(IssueCode),
}

impl ReseauVivant {
    /// Suit les changements du réseau : la valeur ne dit rien, son changement dit de relire
    /// l'état du réseau.
    pub fn suivre(&self) -> watch::Receiver<u64> {
        self.signal.subscribe()
    }

    /// Dit au serveur de relire l'état du réseau.
    pub fn signaler(&self) {
        self.signal.send_modify(|n| *n = n.wrapping_add(1));
    }

    /// Retient qui donne les adresses de ce poste.
    pub fn declarer_adresses(&self, source: SourceAdresses) {
        if let Ok(mut a) = self.adresses.lock() {
            *a = Some(source);
        }
    }

    fn source_adresses(&self) -> Option<SourceAdresses> {
        self.adresses.lock().ok()?.clone()
    }

    /// Adresses de ce poste à donner aux postes à relier ; aucune tant que le binaire n'a pas dit
    /// comment les trouver. La recherche peut interroger le système : elle se fait à part.
    async fn adresses(&self) -> Vec<String> {
        match self.source_adresses() {
            Some(source) => tokio::task::spawn_blocking(move || source())
                .await
                .unwrap_or_default(),
            None => Vec::new(),
        }
    }

    fn ouvrir(&self, appairage: AppairageOuvert) {
        if let Ok(mut a) = self.appairage.lock() {
            *a = Some(appairage);
        }
    }

    fn fermer(&self) {
        if let Ok(mut a) = self.appairage.lock() {
            *a = None;
        }
    }

    /// Le code en attente tel qu'on l'affiche, s'il est encore valable.
    fn affiche(
        &self,
        etablissement_id: Id<Etablissement>,
        maintenant: Horodatage,
    ) -> Option<AppairageAffiche> {
        let mut garde = self.appairage.lock().ok()?;
        let ouvert = garde.as_ref()?;
        if ouvert.etablissement_id != etablissement_id {
            return None;
        }
        if !maintenant.avant(&ouvert.code.expire_le) {
            *garde = None;
            return None;
        }
        Some(AppairageAffiche {
            code: regles::code_appairage_lisible(&ouvert.code.code),
            expire_le: ouvert.code.expire_le,
        })
    }

    fn presenter(
        &self,
        etablissement_id: Id<Etablissement>,
        saisie: &str,
        maintenant: Horodatage,
    ) -> Presentation {
        let Ok(mut garde) = self.appairage.lock() else {
            return Presentation::Aucun;
        };
        let Some(ouvert) = garde
            .as_mut()
            .filter(|a| a.etablissement_id == etablissement_id)
        else {
            return Presentation::Aucun;
        };
        match ouvert.code.presenter(saisie, maintenant) {
            IssueCode::Accepte => Presentation::Accepte(ouvert.ouvert_par),
            IssueCode::Faux => Presentation::Refuse(IssueCode::Faux),
            issue => {
                *garde = None;
                Presentation::Refuse(issue)
            }
        }
    }
}

/// Code d'appairage affiché sur le poste principal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct AppairageAffiche {
    /// Le code, en deux groupes de quatre.
    pub code: String,
    /// Fin de validité.
    pub expire_le: Horodatage,
}

/// État du réseau local, pour l'écran « Postes de l'école ».
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct EtatReseau {
    /// Le poste principal écoute les postes reliés.
    pub actif: bool,
    /// Port d'écoute.
    pub port: u16,
    /// Nom du poste principal, à donner aux postes à relier.
    pub nom_poste_principal: String,
    /// Adresses du poste principal sur le réseau de l'école ; vide si le réseau est éteint ou si
    /// l'ordinateur n'est sur aucun réseau.
    pub adresses: Vec<String>,
    /// Empreinte courte du certificat, à comparer à l'œil lors d'un appairage ; absente tant que
    /// le serveur ne l'a pas fabriqué.
    pub empreinte_courte: Option<String>,
    /// Postes reliés, en service ou révoqués, du plus récent au plus ancien.
    pub postes: Vec<PosteRelie>,
    /// Postes en service.
    pub en_service: u32,
    /// Postes en service d'où une session est ouverte en ce moment.
    pub connectes: Vec<Id<PosteRelie>>,
    /// Postes que la licence couvre.
    pub couverts: PostesCouverts,
    /// Code d'appairage en attente.
    pub appairage: Option<AppairageAffiche>,
}

/// Poste désigné par son identifiant.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandePoste {
    /// Poste relié.
    pub poste_id: Id<PosteRelie>,
}

/// Ce que reçoit un poste qui vient d'être relié : son identifiant et son secret, qu'il garde
/// dans son trousseau. Le secret n'est rendu que cette fois.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppairageConclu {
    /// Identifiant du poste, présenté à chaque appel.
    pub poste_id: String,
    /// Nom retenu pour le poste.
    pub nom: String,
    /// Secret du poste.
    pub secret: String,
}

/// Ce que le serveur du réseau local doit servir.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReseauAServir {
    /// Port d'écoute.
    pub port: u16,
    /// Certificat (DER) et clé privée (PKCS#8, DER) ; absents tant qu'ils ne sont pas fabriqués.
    pub certificat: Option<(Vec<u8>, Vec<u8>)>,
}

/// Service des postes de l'école.
pub struct ServicePostes<'n> {
    noyau: &'n Noyau,
    vivant: &'n ReseauVivant,
    cle_licence: [u8; 32],
}

impl<'n> ServicePostes<'n> {
    /// Service adossé au noyau.
    pub fn nouveau(noyau: &'n Noyau, vivant: &'n ReseauVivant, cle_licence: [u8; 32]) -> Self {
        Self {
            noyau,
            vivant,
            cle_licence,
        }
    }

    async fn connexion(&self) -> Resultat<gesco_db::ConnexionEmpruntee> {
        Ok(self
            .noyau
            .base()
            .pool()
            .acquire()
            .await
            .map_err(gesco_db::ErreurBase::from)?)
    }

    /// Postes que la licence installée couvre ; un poste à l'essai sans licence ni mention.
    async fn couverts(&self, etablissement_id: Id<Etablissement>) -> Resultat<PostesCouverts> {
        let etat = ServiceLicence::nouveau(self.noyau, self.cle_licence)
            .etat(etablissement_id)
            .await?;
        Ok(PostesCouverts::depuis_modules(
            etat.licence
                .as_ref()
                .map(|l| l.modules.as_slice())
                .unwrap_or_default(),
        ))
    }

    /// État du réseau local.
    pub async fn etat(&self, etablissement_id: Id<Etablissement>) -> Resultat<EtatReseau> {
        let contexte = self.noyau.contexte(etablissement_id, None);
        let (local, postes) = {
            let mut cnx = self.connexion().await?;
            (
                depot::lire(&mut cnx, &contexte).await?,
                depot::postes(&mut cnx, &contexte).await?,
            )
        };
        let en_service = postes.iter().filter(|p| p.en_service()).count();
        let en_session = self.noyau.postes_en_session();
        let connectes = postes
            .iter()
            .filter(|p| p.en_service() && en_session.iter().any(|s| s.0 == p.nom))
            .map(|p| p.id)
            .collect();
        let adresses = if local.actif {
            self.vivant.adresses().await
        } else {
            Vec::new()
        };
        Ok(EtatReseau {
            actif: local.actif,
            port: local.port,
            nom_poste_principal: self.noyau.poste().0.clone(),
            adresses,
            connectes,
            empreinte_courte: local
                .certificat
                .as_ref()
                .map(|(der, _)| regles::empreinte_courte(&regles::empreinte_certificat(der))),
            en_service: u32::try_from(en_service).unwrap_or(u32::MAX),
            postes,
            couverts: self.couverts(etablissement_id).await?,
            appairage: self.vivant.affiche(etablissement_id, contexte.maintenant),
        })
    }

    /// Allume ou éteint le réseau local. Éteint, les postes reliés sont refusés ; leurs liaisons
    /// restent pour une réactivation.
    pub async fn regler_actif(&self, session: &Session, actif: bool) -> Resultat<EtatReseau> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let mut local = depot::lire(unite.connexion(), &contexte).await?;
        if local.actif != actif {
            local.actif = actif;
            depot::ecrire(unite.connexion(), &local, &contexte).await?;
            let id = contexte.etablissement_id.vers_chaine();
            unite.auditer(
                "reseau_local",
                &id,
                Operation::Modification,
                Some(json!({ "actif": !actif })),
                Some(json!({ "actif": actif, "port": local.port })),
                "postes.reseau",
            );
            unite.emettre(
                if actif {
                    TypeEvenement::ReseauActive
                } else {
                    TypeEvenement::ReseauDesactive
                },
                "reseau_local",
                &id,
                json!({ "port": local.port }),
            );
        }
        self.noyau.terminer(unite).await?;
        if !actif {
            self.vivant.fermer();
        }
        self.vivant.signaler();
        self.etat(session.etablissement_id).await
    }

    /// Ouvre un appairage : un code neuf, valable dix minutes, remplace le précédent. Refusé si le
    /// réseau est éteint ou si la licence ne couvre pas un poste de plus.
    pub async fn ouvrir_appairage(&self, session: &Session) -> Resultat<EtatReseau> {
        let etat = self.etat(session.etablissement_id).await?;
        if !etat.actif {
            return Err(regle("reseau.eteint"));
        }
        if !etat.couverts.admet_un_de_plus(etat.en_service) {
            return Err(regle("reseau.postes_tous_pris"));
        }
        let code =
            regles::code_appairage(&crate::alea::octets::<{ regles::LONGUEUR_CODE_APPAIRAGE }>()?);
        self.vivant.ouvrir(AppairageOuvert {
            code: CodeAppairage::nouveau(code, self.noyau.horloge().maintenant()),
            etablissement_id: session.etablissement_id,
            ouvert_par: session.utilisateur_id,
        });
        self.etat(session.etablissement_id).await
    }

    /// Referme l'appairage en attente : le code affiché ne sert plus.
    pub async fn fermer_appairage(&self, session: &Session) -> Resultat<EtatReseau> {
        self.vivant.fermer();
        self.etat(session.etablissement_id).await
    }

    /// Note au journal d'audit un appairage refusé : le code faux n'y figure pas.
    async fn noter_refus(
        &self,
        etablissement_id: Id<Etablissement>,
        motif: &'static str,
        nom: &str,
    ) -> Resultat<()> {
        let contexte = self.noyau.contexte(etablissement_id, None);
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        unite.auditer(
            "reseau_local",
            &etablissement_id.vers_chaine(),
            Operation::Acces,
            None,
            Some(json!({ "evenement": "appairage_refuse", "motif": motif, "nom": nom })),
            "postes.relier",
        );
        self.noyau.terminer(unite).await?;
        Ok(())
    }

    /// Relie le poste qui présente le code d'appairage : il reçoit son identifiant et son
    /// secret. Le code n'est consommé que si la liaison aboutit.
    pub async fn conclure_appairage(
        &self,
        etablissement_id: Id<Etablissement>,
        code: &str,
        nom: &str,
    ) -> Resultat<AppairageConclu> {
        let nom = regles::nom_de_poste(nom).map_err(regle)?;
        let maintenant = self.noyau.horloge().maintenant();
        let ouvert_par = match self.vivant.presenter(etablissement_id, code, maintenant) {
            Presentation::Aucun => return Err(regle("reseau.aucun_appairage")),
            Presentation::Accepte(ouvert_par) => ouvert_par,
            Presentation::Refuse(issue) => {
                let (motif, cle) = match issue {
                    IssueCode::Expire => ("expire", "reseau.code_expire"),
                    IssueCode::Annule => ("annule", "reseau.code_annule"),
                    IssueCode::Faux | IssueCode::Accepte => ("faux", "reseau.code_faux"),
                };
                self.noter_refus(etablissement_id, motif, &nom).await?;
                return Err(regle(cle));
            }
        };
        let couverts = self.couverts(etablissement_id).await?;
        let mut contexte = self.noyau.contexte(etablissement_id, None);
        contexte.utilisateur_id = Some(ouvert_par);
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        if !depot::lire(unite.connexion(), &contexte).await?.actif {
            return Err(regle("reseau.eteint"));
        }
        let postes = depot::postes(unite.connexion(), &contexte).await?;
        let en_service: Vec<&PosteRelie> = postes.iter().filter(|p| p.en_service()).collect();
        let pris = nom.eq_ignore_ascii_case(&self.noyau.poste().0)
            || en_service.iter().any(|p| p.nom.eq_ignore_ascii_case(&nom));
        if pris {
            return Err(regle("reseau.nom_poste_pris"));
        }
        if !couverts.admet_un_de_plus(u32::try_from(en_service.len()).unwrap_or(u32::MAX)) {
            return Err(regle("reseau.postes_tous_pris"));
        }
        let secret =
            regles::secret_en_hexadecimal(&crate::alea::octets::<{ regles::LONGUEUR_SECRET }>()?);
        let poste = PosteRelie {
            id: nouvel_id(),
            etablissement_id,
            nom,
            empreinte_secret: regles::empreinte_secret(&secret),
            relie_le: contexte.maintenant,
            relie_par: Some(ouvert_par),
            derniere_connexion: None,
            revoque_le: None,
            revoque_par: None,
        };
        depot::inserer_poste(unite.connexion(), &poste, &contexte).await?;
        let id = poste.id.vers_chaine();
        unite.auditer(
            "poste_relie",
            &id,
            Operation::Creation,
            None,
            Some(serde_json::to_value(&poste)?),
            "postes.relier",
        );
        unite.emettre(
            TypeEvenement::PosteRelie,
            "poste_relie",
            &id,
            json!({ "nom": poste.nom }),
        );
        self.noyau.terminer(unite).await?;
        self.vivant.fermer();
        Ok(AppairageConclu {
            poste_id: id,
            nom: poste.nom,
            secret,
        })
    }

    /// Révoque un poste : il est refusé à l'appel suivant et ses sessions sont fermées.
    pub async fn revoquer(&self, session: &Session, id: Id<PosteRelie>) -> Resultat<EtatReseau> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let Some(poste) = depot::poste(unite.connexion(), id, &contexte).await? else {
            return Err(regle("reseau.poste_inconnu"));
        };
        if !depot::revoquer_poste(unite.connexion(), id, &contexte).await? {
            return Err(regle("reseau.poste_deja_revoque"));
        }
        Self::noter_revocation(&mut unite, &poste, "revocation");
        self.noyau.terminer(unite).await?;
        self.noyau.fermer_sessions_du_poste(&Poste(poste.nom));
        self.etat(session.etablissement_id).await
    }

    fn noter_revocation(
        unite: &mut gesco_db::UniteDeTravail<crate::effets::Effet>,
        poste: &PosteRelie,
        motif: &'static str,
    ) {
        let maintenant = unite.contexte().maintenant;
        let id = poste.id.vers_chaine();
        unite.auditer(
            "poste_relie",
            &id,
            Operation::Modification,
            Some(json!({ "revoque_le": null })),
            Some(json!({ "revoque_le": maintenant, "motif": motif })),
            "postes.revoquer",
        );
        unite.emettre(
            TypeEvenement::PosteRevoque,
            "poste_relie",
            &id,
            json!({ "nom": poste.nom, "motif": motif }),
        );
    }

    /// Renouvelle la clé du réseau : le certificat est oublié, le serveur en fabriquera un autre,
    /// et chaque poste en service est révoqué — il devra être relié de nouveau.
    pub async fn renouveler_cle(&self, session: &Session) -> Resultat<EtatReseau> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let mut local = depot::lire(unite.connexion(), &contexte).await?;
        let ancienne = local
            .certificat
            .take()
            .map(|(der, _)| regles::empreinte_courte(&regles::empreinte_certificat(&der)));
        local.fabrique_le = None;
        depot::ecrire(unite.connexion(), &local, &contexte).await?;
        let postes = depot::postes(unite.connexion(), &contexte).await?;
        let mut revoques = Vec::new();
        for poste in postes.into_iter().filter(PosteRelie::en_service) {
            if depot::revoquer_poste(unite.connexion(), poste.id, &contexte).await? {
                Self::noter_revocation(&mut unite, &poste, "cle_renouvelee");
                revoques.push(Poste(poste.nom));
            }
        }
        let id = contexte.etablissement_id.vers_chaine();
        unite.auditer(
            "reseau_local",
            &id,
            Operation::Modification,
            Some(json!({ "empreinte_courte": ancienne })),
            Some(json!({ "empreinte_courte": null, "postes_revoques": revoques.len() })),
            "postes.cle",
        );
        unite.emettre(
            TypeEvenement::CleReseauRenouvelee,
            "reseau_local",
            &id,
            json!({ "postes_revoques": revoques.len() }),
        );
        self.noyau.terminer(unite).await?;
        for poste in &revoques {
            self.noyau.fermer_sessions_du_poste(poste);
        }
        self.vivant.fermer();
        self.vivant.signaler();
        self.etat(session.etablissement_id).await
    }

    /// Reconnaît le poste qui présente cet identifiant et ce secret : son nom, s'il est en
    /// service et que le réseau est allumé. Tout le reste rend `None`, sans dire pourquoi.
    pub async fn reconnaitre(
        &self,
        etablissement_id: Id<Etablissement>,
        poste_id: &str,
        secret: &str,
    ) -> Resultat<Option<Poste>> {
        let Ok(id) = Id::<PosteRelie>::analyser(poste_id) else {
            return Ok(None);
        };
        let contexte = self.noyau.contexte(etablissement_id, None);
        let mut cnx = self.connexion().await?;
        if !depot::lire(&mut cnx, &contexte).await?.actif {
            return Ok(None);
        }
        let Some(poste) = depot::poste(&mut cnx, id, &contexte).await? else {
            return Ok(None);
        };
        if !poste.en_service() || !regles::secret_correspond(secret, &poste.empreinte_secret) {
            return Ok(None);
        }
        let a_noter = poste
            .derniere_connexion
            .and_then(|derniere| derniere.difference_millis(&contexte.maintenant).ok())
            .is_none_or(|ecart| ecart >= ECART_NOTE_CONNEXION_MILLIS);
        if a_noter {
            depot::noter_connexion(&mut cnx, id, &contexte).await?;
        }
        Ok(Some(Poste(poste.nom)))
    }

    /// Ce que le serveur doit servir, ou `None` quand le réseau est éteint.
    pub async fn a_servir(
        &self,
        etablissement_id: Id<Etablissement>,
    ) -> Resultat<Option<ReseauAServir>> {
        let contexte = self.noyau.contexte(etablissement_id, None);
        let mut cnx = self.connexion().await?;
        let local = depot::lire(&mut cnx, &contexte).await?;
        Ok(local.actif.then_some(ReseauAServir {
            port: local.port,
            certificat: local.certificat,
        }))
    }

    /// Garde le certificat que le serveur vient de fabriquer. Sans effet s'il en existe déjà un :
    /// on ne remplace un certificat qu'en renouvelant la clé du réseau.
    pub async fn enregistrer_certificat(
        &self,
        etablissement_id: Id<Etablissement>,
        certificat: Vec<u8>,
        cle: Vec<u8>,
    ) -> Resultat<()> {
        let contexte = self.noyau.contexte(etablissement_id, None);
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let mut local = depot::lire(unite.connexion(), &contexte).await?;
        if local.certificat.is_none() {
            let empreinte = regles::empreinte_courte(&regles::empreinte_certificat(&certificat));
            local.certificat = Some((certificat, cle));
            local.fabrique_le = Some(contexte.maintenant);
            depot::ecrire(unite.connexion(), &local, &contexte).await?;
            unite.auditer(
                "reseau_local",
                &etablissement_id.vers_chaine(),
                Operation::Modification,
                Some(json!({ "empreinte_courte": null })),
                Some(json!({ "empreinte_courte": empreinte })),
                "postes.cle",
            );
        }
        self.noyau.terminer(unite).await?;
        Ok(())
    }
}
