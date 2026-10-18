//! File des SMS (SPEC_PHASE_5 §3, blueprint §25.3) : les messages mis en file partent à leur tour,
//! dans les heures d'envoi de l'école, au débit réglé, sous le plafond du mois ; les accusés de
//! remise reviennent et disent si le numéro est joignable. Une personne décide des messages
//! préparés, annule ce qui ne doit pas partir, renvoie ce qui a échoué. L'utilisateur n'attend
//! jamais : le traitement tourne à part, toutes les quelques secondes.

use gesco_core::commun::{DateCivile, Horodatage, Id};
use gesco_core::core::{Etablissement, Operation, Permission, TypeEvenement};
use gesco_core::notification::canal::texte_alerte;
use gesco_core::notification::message::apres_erreur;
use gesco_core::notification::{
    ErreurEnvoi, EtatCanal, EtatMessage, Joignabilite, Message, NatureMessage, gsm7, modele,
    reglages,
};
use gesco_core::scolarite::{Eleve, Responsable, telephone};
use gesco_db::depots::{etablissement, file_sms, messages, parametre, personnes, role};
use gesco_db::unite::Durabilite;
use gesco_db::{Connexion, Contexte, nouvel_id};
use gesco_sms::{Canal, CanalModem};
use serde::{Deserialize, Serialize};

use crate::erreur::{ErreurService, Resultat};
use crate::noyau::Noyau;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Réglage du canal d'envoi.
pub const PARAMETRE_CANAL: &str = "sms.canal";
/// Réglage de la signature, le nom de l'école en tête des SMS.
pub const PARAMETRE_SIGNATURE: &str = "sms.signature";

/// Réglage du port de la clé modem.
pub const PARAMETRE_PORT: &str = "sms.port";
/// Réglage du code USSD du solde.
pub const PARAMETRE_USSD: &str = "sms.ussd_solde";
/// Réglage du seuil de crédit sous lequel la file s'arrête.
pub const PARAMETRE_SEUIL_CREDIT: &str = "sms.seuil_credit";
/// Réglage du numéro prévenu quand la file s'arrête.
pub const PARAMETRE_NUMERO_ALERTE: &str = "sms.numero_alerte";

/// Les canaux que le poste sait ouvrir : le canal simulé, et la clé modem d'un port.
pub trait SourceCanaux: Sync {
    /// Le canal simulé.
    fn simule(&self) -> &dyn Canal;
    /// La clé modem de ce port ; aucune pour un port vide.
    fn modem(&self, port: &str) -> Option<std::sync::Arc<CanalModem>>;
}

/// Port où brancher la clé.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct PortDisponible {
    /// Nom du port : `COM3`, ou `simule`.
    pub nom: String,
    /// Vrai pour la clé simulée des compilations de développement.
    pub simule: bool,
}

/// Port de la clé simulée : il répond aux commandes AT comme une clé, sans rien envoyer.
pub const PORT_SIMULE: &str = "simule";

/// Ports série du poste ; la clé simulée en plus dans une compilation de développement.
pub fn ports_disponibles() -> Vec<PortDisponible> {
    let mut ports: Vec<PortDisponible> = gesco_sms::at::ports_du_poste()
        .into_iter()
        .map(|nom| PortDisponible { nom, simule: false })
        .collect();
    if cfg!(debug_assertions) {
        ports.push(PortDisponible {
            nom: PORT_SIMULE.to_owned(),
            simule: true,
        });
    }
    ports
}

/// Messages listés au plus.
const LISTE_MAX: i64 = 200;

/// Message tel que l'écran le montre.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct MessageAffiche {
    /// Message.
    pub message: Message,
    /// Prénoms et nom du responsable destinataire ; vide pour un essai.
    pub destinataire: String,
    /// Prénoms et nom de l'élève dont il parle.
    pub eleve: String,
}

/// Nombre de messages d'un état.
#[derive(Debug, Clone, Copy, Serialize, specta::Type)]
pub struct CompteEtat {
    /// État.
    pub etat: EtatMessage,
    /// Nombre de messages.
    pub nombre: u32,
}

/// Modèle d'une nature de message, avec son exemple formulé.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ModeleSms {
    /// Nature.
    pub nature: NatureMessage,
    /// Modèle, avec ses variables entre accolades.
    pub texte: String,
    /// Variables que le modèle peut employer.
    pub variables: Vec<String>,
    /// Exemple formulé avec des valeurs longues, tel qu'il partirait.
    pub exemple: String,
    /// Nombre de SMS de l'exemple.
    pub sms: u8,
}

/// Ce que peut faire le compte connecté.
#[derive(Debug, Clone, Copy, Serialize, specta::Type)]
pub struct DroitsMessages {
    /// Régler les modèles, envoyer un SMS d'essai.
    pub configurer: bool,
    /// Renvoyer un message échoué.
    pub renvoyer: bool,
    /// Décider d'un message préparé, annuler un message.
    pub decider: bool,
}

/// La file, telle que l'écran « Messages » la montre.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct FileMessages {
    /// Messages de l'état demandé, les plus récents d'abord.
    pub messages: Vec<MessageAffiche>,
    /// Nombre de messages de chaque état.
    pub compteurs: Vec<CompteEtat>,
    /// Canal choisi : `aucun`, `simule` ou `modem`.
    pub canal: String,
    /// Port de la clé modem.
    pub port: String,
    /// État du canal : crédit, arrêt, clé absente.
    pub etat_canal: EtatCanal,
    /// Seuil de crédit sous lequel la file s'arrête ; 0 : jamais.
    pub seuil_credit: i64,
    /// Vrai si l'école a donné le code USSD du solde.
    pub credit_lisible: bool,
    /// Heures d'envoi : « 7-20 ».
    pub fenetre: String,
    /// SMS partis depuis le début du mois.
    pub sms_du_mois: u32,
    /// Plafond du mois ; 0 : sans limite.
    pub plafond: u32,
    /// Modèles des messages.
    pub modeles: Vec<ModeleSms>,
    /// Droits de qui consulte.
    pub droits: DroitsMessages,
}

/// Messages d'un état à lire ; aucun : tous.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeFile {
    /// État.
    pub etat: Option<EtatMessage>,
}

/// Décision d'envoyer un message préparé, son texte relu.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeDecisionMessage {
    /// Message.
    pub id: Id<Message>,
    /// Texte relu, corrigé au besoin.
    pub texte: String,
}

/// Annulation motivée d'un message qui n'est pas parti.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeAnnulationMessage {
    /// Message.
    pub id: Id<Message>,
    /// Motif.
    pub motif: String,
}

/// Message échoué ou expiré à renvoyer.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeRenvoi {
    /// Message.
    pub id: Id<Message>,
}

/// Modèle reformulé par l'école.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeModele {
    /// Nature.
    pub nature: NatureMessage,
    /// Nouveau modèle ; vide : le modèle livré.
    pub texte: String,
}

/// SMS d'essai vers un numéro saisi.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeEssai {
    /// Numéro, sous une forme quelconque.
    pub numero: String,
}

/// Texte dont on veut l'aperçu.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeApercu {
    /// Texte saisi.
    pub texte: String,
}

/// Un texte tel qu'il partirait.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ApercuSms {
    /// Texte translittéré en GSM-7.
    pub texte: String,
    /// Caractères comptés comme un téléphone.
    pub caracteres: u32,
    /// Nombre de SMS.
    pub sms: u8,
}

/// Ce qu'un passage de la file a fait.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct BilanFile {
    /// Messages expirés.
    pub expires: u64,
    /// Messages partis.
    pub envoyes: u32,
    /// Accusés de remise reçus.
    pub remis: u32,
    /// Messages échoués.
    pub echoues: u32,
    /// Vrai si la clé ne répondait pas.
    pub cle_absente: bool,
    /// Vrai si la file est arrêtée faute de crédit.
    pub arretee: bool,
}

/// Port de la clé modem.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandePort {
    /// Nom du port : `COM3`.
    pub port: String,
}

/// Ce que l'essai de la clé a appris.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct EssaiCle {
    /// Fabricant.
    pub fabricant: String,
    /// Modèle.
    pub modele: String,
    /// Réseau sur lequel la clé est inscrite.
    pub operateur: String,
    /// Force du signal, de 0 à 100.
    pub signal: Option<u8>,
    /// Crédit lu, si l'école a donné le code USSD.
    pub credit: Option<i64>,
    /// Réponse de l'opérateur au code USSD.
    pub credit_texte: String,
}

/// Message à mettre en file, formulé.
pub(crate) struct MessageAFaire {
    pub nature: NatureMessage,
    pub cle_fait: String,
    pub responsable_id: Option<Id<Responsable>>,
    pub eleve_id: Option<Id<Eleve>>,
    pub numero: String,
    pub texte: String,
    /// Attente avant le départ, en millisecondes (délai de grâce).
    pub attente_ms: i64,
    /// Expiration ; aucune : la durée de vie de sa nature.
    pub expire_le: Option<Horodatage>,
}

/// Met un message en file, à décider ou en attente selon sa nature ; rend le message, ou rien s'il
/// y était déjà pour ce fait et ce numéro.
pub(crate) async fn mettre_en_file(
    cnx: &mut Connexion,
    a_faire: MessageAFaire,
    contexte: &Contexte,
) -> Resultat<Option<Message>> {
    let texte = gsm7::translitterer(a_faire.texte.trim());
    let sms = sms_du_texte(&texte)?;
    let maintenant = contexte.maintenant;
    let plus = |ms: i64| Horodatage::depuis_millis_epoque(maintenant.millis_epoque() + ms);
    let message = Message {
        id: nouvel_id(),
        etablissement_id: contexte.etablissement_id,
        nature: a_faire.nature,
        cle_fait: a_faire.cle_fait,
        responsable_id: a_faire.responsable_id,
        eleve_id: a_faire.eleve_id,
        numero: a_faire.numero,
        texte,
        sms,
        etat: if a_faire.nature.a_decider() {
            EtatMessage::ADecider
        } else {
            EtatMessage::EnAttente
        },
        essais: 0,
        pas_avant: plus(a_faire.attente_ms.max(0)),
        expire_le: a_faire
            .expire_le
            .or_else(|| a_faire.nature.duree_de_vie_ms().map(plus)),
        reference: None,
        envoye_le: None,
        remis_le: None,
        erreur: None,
        motif_annulation: String::new(),
        cree_le: maintenant,
    };
    Ok(file_sms::inserer(cnx, &message, contexte)
        .await?
        .then_some(message))
}

/// Nombre de SMS d'un texte déjà translittéré : un à trois.
fn sms_du_texte(texte: &str) -> Resultat<u8> {
    match gsm7::nombre_sms(texte) {
        0 => Err(regle("sms.message_vide")),
        n if n > gsm7::SMS_MAX => Err(regle("sms.message_trop_long")),
        n => Ok(u8::try_from(n).unwrap_or(u8::MAX)),
    }
}

/// Le nom de l'école en tête des SMS : la signature réglée, ou le nom de l'établissement.
pub(crate) async fn signature(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<String> {
    let reglee = parametre::texte(cnx, PARAMETRE_SIGNATURE, "", contexte).await?;
    if !reglee.trim().is_empty() {
        return Ok(reglee.trim().to_owned());
    }
    Ok(etablissement::lire(cnx, contexte.etablissement_id)
        .await
        .map(|e| e.nom)
        .unwrap_or_default())
}

/// Le modèle d'une nature : celui de l'école, ou le modèle livré.
pub(crate) async fn modele_de(
    cnx: &mut Connexion,
    nature: NatureMessage,
    contexte: &Contexte,
) -> Resultat<String> {
    Ok(parametre::texte(
        cnx,
        &modele::cle_reglage(nature),
        modele::modele_par_defaut(nature),
        contexte,
    )
    .await?)
}

/// Aperçu d'un texte tel qu'il partirait.
pub fn apercu(demande: &DemandeApercu) -> ApercuSms {
    let texte = gsm7::translitterer(demande.texte.trim());
    ApercuSms {
        caracteres: u32::try_from(gsm7::longueur(&texte)).unwrap_or(u32::MAX),
        sms: u8::try_from(gsm7::nombre_sms(&texte)).unwrap_or(u8::MAX),
        texte,
    }
}

/// Début du mois civil d'un instant.
fn debut_du_mois(maintenant: Horodatage) -> Horodatage {
    let civil = maintenant.civil_utc();
    let jours = i32::try_from(civil.annee)
        .ok()
        .and_then(|a| DateCivile::nouvelle(a, civil.mois, 1))
        .map_or(0, |d| d.jours_depuis_epoque());
    Horodatage::depuis_millis_epoque(jours * 86_400_000)
}

/// Service de la file des SMS.
pub struct ServiceFile<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceFile<'n> {
    /// Service sur le noyau.
    pub fn nouveau(noyau: &'n Noyau) -> Self {
        Self { noyau }
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

    async fn afficher(
        cnx: &mut Connexion,
        message: Message,
        contexte: &Contexte,
    ) -> Resultat<MessageAffiche> {
        let destinataire = match message.responsable_id {
            Some(id) => personnes::responsable(cnx, id, contexte)
                .await
                .map(|(_, p)| p.nom_complet())
                .unwrap_or_default(),
            None => String::new(),
        };
        let eleve = match message.eleve_id {
            Some(id) => personnes::eleve(cnx, id, contexte)
                .await
                .map(|(_, p)| p.nom_complet())
                .unwrap_or_default(),
            None => String::new(),
        };
        Ok(MessageAffiche {
            message,
            destinataire,
            eleve,
        })
    }

    async fn modeles(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Vec<ModeleSms>> {
        let ecole = signature(cnx, contexte).await?;
        let mut modeles = Vec::new();
        for nature in modele::NATURES_A_MODELE {
            let texte = modele_de(cnx, nature, contexte).await?;
            let mut valeurs: Vec<(&str, &str)> = modele::variables(nature).to_vec();
            if let Some(v) = valeurs.iter_mut().find(|(n, _)| *n == "ecole") {
                v.1 = &ecole;
            }
            let exemple = modele::formuler(&texte, &valeurs);
            modeles.push(ModeleSms {
                nature,
                variables: modele::variables(nature)
                    .iter()
                    .map(|(n, _)| (*n).to_owned())
                    .collect(),
                sms: u8::try_from(gsm7::nombre_sms(&exemple)).unwrap_or(u8::MAX),
                exemple,
                texte,
            });
        }
        Ok(modeles)
    }

    /// La file : les messages d'un état, les compteurs, le canal, la consommation, les modèles.
    pub async fn lire(&self, session: &Session, demande: &DemandeFile) -> Resultat<FileMessages> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let mut affiches = Vec::new();
        for m in file_sms::lister(&mut cnx, demande.etat, LISTE_MAX, &contexte).await? {
            affiches.push(Self::afficher(&mut cnx, m, &contexte).await?);
        }
        let compteurs = file_sms::compter_par_etat(&mut cnx, &contexte)
            .await?
            .into_iter()
            .map(|(etat, n)| CompteEtat {
                etat,
                nombre: u32::try_from(n).unwrap_or(u32::MAX),
            })
            .collect();
        let (_, sms_du_mois) =
            file_sms::partis_depuis(&mut cnx, debut_du_mois(contexte.maintenant), &contexte)
                .await?;
        let p = role::permissions_de(&mut cnx, session.utilisateur_id, &contexte).await?;
        Ok(FileMessages {
            messages: affiches,
            compteurs,
            canal: parametre::texte(&mut cnx, PARAMETRE_CANAL, "aucun", &contexte).await?,
            port: parametre::texte(&mut cnx, PARAMETRE_PORT, "", &contexte).await?,
            etat_canal: Self::etat_canal(&mut cnx, &contexte).await?,
            seuil_credit: parametre::entier(&mut cnx, PARAMETRE_SEUIL_CREDIT, 500, &contexte)
                .await?,
            credit_lisible: !parametre::texte(&mut cnx, PARAMETRE_USSD, "", &contexte)
                .await?
                .trim()
                .is_empty(),
            fenetre: parametre::texte(
                &mut cnx,
                "sms.fenetre_envoi",
                reglages::FENETRE_DEFAUT,
                &contexte,
            )
            .await?,
            sms_du_mois: u32::try_from(sms_du_mois).unwrap_or(u32::MAX),
            plafond: u32::try_from(
                parametre::entier(&mut cnx, "sms.plafond_mensuel", 0, &contexte).await?,
            )
            .unwrap_or(0),
            modeles: Self::modeles(&mut cnx, &contexte).await?,
            droits: DroitsMessages {
                configurer: p.contains(&Permission::NOTIFICATIONS_CONFIGURE),
                renvoyer: p.contains(&Permission::NOTIFICATIONS_SEND),
                decider: p.contains(&Permission::NOTIFICATIONS_SEND_MANUAL),
            },
        })
    }

    /// Un message modifié par une personne, audité avec son événement, puis la file relue.
    async fn ecrire(
        &self,
        session: &Session,
        avant: &Message,
        apres: &Message,
        evenement: TypeEvenement,
        operation: &'static str,
    ) -> Resultat<FileMessages> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        file_sms::mettre_a_jour(unite.connexion(), apres, &contexte).await?;
        let id = apres.id.vers_chaine();
        unite.auditer(
            "message_sms",
            &id,
            Operation::Modification,
            Some(serde_json::json!({ "etat": avant.etat, "texte": avant.texte })),
            Some(serde_json::json!({
                "etat": apres.etat, "texte": apres.texte, "motif": apres.motif_annulation,
            })),
            operation,
        );
        unite.emettre(
            evenement,
            "message_sms",
            &id,
            serde_json::json!({ "nature": apres.nature, "etat": apres.etat }),
        );
        self.noyau.terminer(unite).await?;
        self.lire(session, &DemandeFile { etat: None }).await
    }

    async fn message(&self, session: &Session, id: Id<Message>) -> Resultat<Message> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        Ok(file_sms::message(&mut cnx, id, &contexte).await?)
    }

    /// Décide d'envoyer un message préparé, avec son texte relu : il entre dans la file.
    pub async fn decider(
        &self,
        session: &Session,
        demande: &DemandeDecisionMessage,
    ) -> Resultat<FileMessages> {
        let avant = self.message(session, demande.id).await?;
        if avant.etat != EtatMessage::ADecider {
            return Err(regle("sms.message_deja_decide"));
        }
        let texte = gsm7::translitterer(demande.texte.trim());
        let sms = sms_du_texte(&texte)?;
        let maintenant = self.noyau.horloge().maintenant();
        let apres = Message {
            texte,
            sms,
            etat: EtatMessage::EnAttente,
            pas_avant: maintenant,
            ..avant.clone()
        };
        self.ecrire(
            session,
            &avant,
            &apres,
            TypeEvenement::MessageDecide,
            "file_sms.decider",
        )
        .await
    }

    /// Annule un message qui n'est pas parti, avec un motif.
    pub async fn annuler(
        &self,
        session: &Session,
        demande: &DemandeAnnulationMessage,
    ) -> Resultat<FileMessages> {
        let motif = demande.motif.trim();
        if motif.is_empty() {
            return Err(regle("sms.motif_obligatoire"));
        }
        let avant = self.message(session, demande.id).await?;
        if !avant.etat.annulable() {
            return Err(regle("sms.message_parti"));
        }
        let apres = Message {
            etat: EtatMessage::Annule,
            motif_annulation: motif.to_owned(),
            ..avant.clone()
        };
        self.ecrire(
            session,
            &avant,
            &apres,
            TypeEvenement::MessageAnnule,
            "file_sms.annuler",
        )
        .await
    }

    /// Remet dans la file un message échoué ou expiré.
    pub async fn renvoyer(
        &self,
        session: &Session,
        demande: &DemandeRenvoi,
    ) -> Resultat<FileMessages> {
        let avant = self.message(session, demande.id).await?;
        if !avant.etat.renvoyable() {
            return Err(regle("sms.message_non_renvoyable"));
        }
        let maintenant = self.noyau.horloge().maintenant();
        let vie = avant.nature.duree_de_vie_ms().unwrap_or(24 * 3_600 * 1_000);
        let apres = Message {
            etat: EtatMessage::EnAttente,
            essais: 0,
            pas_avant: maintenant,
            expire_le: Some(Horodatage::depuis_millis_epoque(
                maintenant.millis_epoque() + vie,
            )),
            reference: None,
            envoye_le: None,
            remis_le: None,
            erreur: None,
            ..avant.clone()
        };
        self.ecrire(
            session,
            &avant,
            &apres,
            TypeEvenement::MessageRenvoye,
            "file_sms.renvoyer",
        )
        .await
    }

    /// Reformule le modèle d'une nature ; un texte vide remet le modèle livré.
    pub async fn definir_modele(
        &self,
        session: &Session,
        demande: &DemandeModele,
    ) -> Resultat<FileMessages> {
        if !modele::NATURES_A_MODELE.contains(&demande.nature) {
            return Err(regle("sms.modele_vide"));
        }
        let texte = if demande.texte.trim().is_empty() {
            modele::modele_par_defaut(demande.nature).to_owned()
        } else {
            demande.texte.trim().to_owned()
        };
        modele::verifier(demande.nature, &texte).map_err(regle)?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let cle = modele::cle_reglage(demande.nature);
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        let avant = parametre::texte(
            unite.connexion(),
            &cle,
            modele::modele_par_defaut(demande.nature),
            &contexte,
        )
        .await?;
        if avant != texte {
            parametre::definir(
                unite.connexion(),
                &gesco_core::core::parametre::Parametre {
                    etablissement_id: session.etablissement_id,
                    cle: cle.clone(),
                    valeur: gesco_core::core::parametre::ValeurParametre::Texte(texte.clone()),
                    modifiable_par_utilisateur: true,
                },
                &contexte,
            )
            .await?;
            unite.auditer(
                "parametre",
                &cle,
                Operation::Modification,
                Some(serde_json::json!({ "texte": avant })),
                Some(serde_json::json!({ "texte": texte })),
                "file_sms.definir_modele",
            );
            unite.emettre(
                TypeEvenement::ModeleMessageModifie,
                "parametre",
                &cle,
                serde_json::json!({ "nature": demande.nature }),
            );
        }
        self.noyau.terminer(unite).await?;
        self.lire(session, &DemandeFile { etat: None }).await
    }

    /// Met en file un SMS d'essai vers un numéro saisi.
    pub async fn essai(&self, session: &Session, demande: &DemandeEssai) -> Resultat<FileMessages> {
        let numero = telephone::normaliser(&demande.numero).map_err(regle)?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        let ecole = signature(unite.connexion(), &contexte).await?;
        let texte = modele::formuler(
            modele::modele_par_defaut(NatureMessage::Essai),
            &[("ecole", &ecole)],
        );
        let id_essai: Id<Message> = nouvel_id();
        let mis = mettre_en_file(
            unite.connexion(),
            MessageAFaire {
                nature: NatureMessage::Essai,
                cle_fait: format!("essai:{}", id_essai.vers_chaine()),
                responsable_id: None,
                eleve_id: None,
                numero,
                texte,
                attente_ms: 0,
                expire_le: None,
            },
            &contexte,
        )
        .await?;
        if let Some(m) = mis {
            let id = m.id.vers_chaine();
            unite.auditer(
                "message_sms",
                &id,
                Operation::Creation,
                None,
                Some(serde_json::json!({ "nature": m.nature, "numero": m.numero })),
                "file_sms.essai",
            );
            unite.emettre(
                TypeEvenement::MessageMisEnFile,
                "message_sms",
                &id,
                serde_json::json!({ "nature": m.nature }),
            );
        }
        self.noyau.terminer(unite).await?;
        self.lire(session, &DemandeFile { etat: None }).await
    }

    /// Accusés de remise revenus : remis ou échoué, et la joignabilité du numéro.
    async fn recevoir_accuses(
        cnx: &mut Connexion,
        canal: &dyn Canal,
        bilan: &mut BilanFile,
        contexte: &Contexte,
    ) -> Resultat<()> {
        for accuse in canal.accuses() {
            let Some(mut m) = file_sms::par_reference(cnx, &accuse.reference, contexte).await?
            else {
                continue;
            };
            if m.etat != EtatMessage::Envoye {
                continue;
            }
            let joignabilite = messages::joignabilite(cnx, &m.numero, contexte)
                .await?
                .unwrap_or_else(|| Joignabilite::nouvelle(&m.numero));
            let joignabilite = if accuse.remis {
                m.etat = EtatMessage::Remis;
                m.remis_le = Some(contexte.maintenant);
                bilan.remis += 1;
                joignabilite.apres_remise()
            } else {
                m.etat = EtatMessage::Echoue;
                m.erreur = Some(ErreurEnvoi::NonRemis);
                bilan.echoues += 1;
                joignabilite.apres_echec()
            };
            file_sms::mettre_a_jour(cnx, &m, contexte).await?;
            messages::enregistrer_joignabilite(cnx, &joignabilite, contexte).await?;
        }
        Ok(())
    }

    /// État du canal, l'état neuf s'il n'a jamais été enregistré.
    async fn etat_canal(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<EtatCanal> {
        Ok(file_sms::etat_canal(cnx, contexte).await?)
    }

    /// Prévient le numéro d'alerte que la file s'arrête, directement par la clé, s'il reste de
    /// quoi envoyer ; sans numéro d'alerte, l'avertissement du tableau de bord suffit.
    async fn alerter(
        cnx: &mut Connexion,
        canal: &dyn Canal,
        etat: &EtatCanal,
        contexte: &Contexte,
    ) -> Resultat<()> {
        let saisi = parametre::texte(cnx, PARAMETRE_NUMERO_ALERTE, "", contexte).await?;
        let (Ok(numero), Some(raison)) = (telephone::normaliser(&saisi), etat.raison) else {
            return Ok(());
        };
        let ecole = signature(cnx, contexte).await?;
        let _ = canal.envoyer(&numero, &texte_alerte(&ecole, raison, etat.credit));
        Ok(())
    }

    /// Lit le crédit de la clé par USSD, et en tire l'arrêt ou la reprise de la file ; vrai si la
    /// lecture a abouti.
    async fn lire_credit(
        cnx: &mut Connexion,
        modem: &CanalModem,
        etat: EtatCanal,
        contexte: &Contexte,
    ) -> Resultat<(EtatCanal, bool)> {
        let code = parametre::texte(cnx, PARAMETRE_USSD, "", contexte).await?;
        if code.trim().is_empty() {
            return Ok((etat, false));
        }
        let Ok(lecture) = modem.lire_credit(code.trim()) else {
            return Ok((etat, false));
        };
        let seuil = parametre::entier(cnx, PARAMETRE_SEUIL_CREDIT, 500, contexte).await?;
        let (etat, alerte) =
            etat.apres_lecture(lecture.montant, &lecture.texte, seuil, contexte.maintenant);
        if alerte {
            Self::alerter(cnx, modem, &etat, contexte).await?;
        }
        Ok((etat, true))
    }

    /// Un passage de la file pour un établissement : expire ce qui a trop attendu ; avec la clé,
    /// vérifie qu'elle répond et relit le crédit chaque heure ; reçoit les accusés ; envoie ce dont
    /// le tour est venu, dans les heures d'envoi, au débit et sous le plafond réglés, sauf si la
    /// file est arrêtée faute de crédit. Sans canal (réglage « aucun »), rien ne part.
    pub async fn traiter(
        &self,
        etablissement_id: Id<Etablissement>,
        source: &dyn SourceCanaux,
    ) -> Resultat<BilanFile> {
        let contexte = self.noyau.contexte(etablissement_id, None);
        let maintenant = contexte.maintenant;
        let mut cnx = self.connexion().await?;
        let mut bilan = BilanFile {
            expires: file_sms::expirer(&mut cnx, maintenant, &contexte).await?,
            ..BilanFile::default()
        };
        let nom_canal = parametre::texte(&mut cnx, PARAMETRE_CANAL, "aucun", &contexte).await?;
        let initial = Self::etat_canal(&mut cnx, &contexte).await?;
        let mut etat = initial.clone();
        let modem = if nom_canal == "modem" {
            let port = parametre::texte(&mut cnx, PARAMETRE_PORT, "", &contexte).await?;
            source.modem(&port).filter(|m| m.verifier().is_ok())
        } else {
            None
        };
        let canal: &dyn Canal = match (nom_canal.as_str(), &modem) {
            ("simule", _) => source.simule(),
            ("modem", Some(m)) => m.as_ref(),
            ("modem", None) => {
                // Clé débranchée ou muette : la file attend, sans rien perdre.
                etat.cle_absente = true;
                bilan.cle_absente = true;
                if etat != initial {
                    file_sms::enregistrer_etat_canal(&mut cnx, &etat, &contexte).await?;
                }
                return Ok(bilan);
            }
            _ => return Ok(bilan),
        };
        etat.cle_absente = false;
        if let Some(m) = &modem
            && etat.lecture_due(maintenant)
        {
            etat = Self::lire_credit(&mut cnx, m, etat, &contexte).await?.0;
        }
        Self::recevoir_accuses(&mut cnx, canal, &mut bilan, &contexte).await?;
        let fenetre = reglages::lire_fenetre(
            &parametre::texte(
                &mut cnx,
                "sms.fenetre_envoi",
                reglages::FENETRE_DEFAUT,
                &contexte,
            )
            .await?,
        )
        .unwrap_or((7, 20));
        let debit = parametre::entier(&mut cnx, "sms.debit_par_minute", 6, &contexte).await?;
        let (dans_la_minute, _) = file_sms::partis_depuis(
            &mut cnx,
            Horodatage::depuis_millis_epoque(maintenant.millis_epoque() - 60_000),
            &contexte,
        )
        .await?;
        let place = debit - dans_la_minute;
        let plafond = parametre::entier(&mut cnx, "sms.plafond_mensuel", 0, &contexte).await?;
        let (_, mut sms_du_mois) =
            file_sms::partis_depuis(&mut cnx, debut_du_mois(maintenant), &contexte).await?;
        let a_envoyer = if etat.arretee
            || place <= 0
            || !reglages::dans_la_fenetre(fenetre, maintenant.civil_utc().heure)
        {
            Vec::new()
        } else {
            file_sms::a_envoyer(&mut cnx, maintenant, place, &contexte).await?
        };
        for mut m in a_envoyer {
            if plafond > 0 && sms_du_mois + i64::from(m.sms) > plafond {
                break;
            }
            let joignabilite = messages::joignabilite(&mut cnx, &m.numero, &contexte).await?;
            if joignabilite.as_ref().is_some_and(|j| !j.peut_recevoir()) {
                m.etat = EtatMessage::Echoue;
                m.erreur = Some(ErreurEnvoi::NumeroInjoignable);
                bilan.echoues += 1;
                file_sms::mettre_a_jour(&mut cnx, &m, &contexte).await?;
                continue;
            }
            m.essais = m.essais.saturating_add(1);
            match canal.envoyer(&m.numero, &m.texte) {
                Ok(reference) => {
                    m.etat = EtatMessage::Envoye;
                    m.reference = Some(reference);
                    m.envoye_le = Some(maintenant);
                    m.erreur = None;
                    sms_du_mois += i64::from(m.sms);
                    bilan.envoyes += 1;
                    etat = etat.apres_envoi();
                }
                Err(erreur) => {
                    let (etat_message, pas_avant) = apres_erreur(m.essais, erreur, maintenant);
                    m.etat = etat_message;
                    m.pas_avant = pas_avant;
                    m.erreur = Some(erreur);
                    if etat_message == EtatMessage::Echoue {
                        bilan.echoues += 1;
                    }
                    if erreur == ErreurEnvoi::NumeroRefuse {
                        let refuse = joignabilite
                            .unwrap_or_else(|| Joignabilite::nouvelle(&m.numero))
                            .apres_refus();
                        messages::enregistrer_joignabilite(&mut cnx, &refuse, &contexte).await?;
                    }
                    if erreur == ErreurEnvoi::Credit {
                        let (suivant, alerte) = etat.apres_erreur_credit();
                        etat = suivant;
                        if alerte {
                            Self::alerter(&mut cnx, canal, &etat, &contexte).await?;
                        }
                    }
                }
            }
            file_sms::mettre_a_jour(&mut cnx, &m, &contexte).await?;
            if etat.arretee {
                break;
            }
        }
        Self::recevoir_accuses(&mut cnx, canal, &mut bilan, &contexte).await?;
        bilan.arretee = etat.arretee;
        if etat != initial {
            file_sms::enregistrer_etat_canal(&mut cnx, &etat, &contexte).await?;
        }
        Ok(bilan)
    }

    /// Essaie la clé d'un port : ce qu'elle dit d'elle-même et, si l'école a donné le code USSD
    /// de son opérateur, le crédit de la puce.
    pub async fn essayer_cle(
        &self,
        session: &Session,
        demande: &DemandePort,
        source: &dyn SourceCanaux,
    ) -> Resultat<EssaiCle> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let modem = source
            .modem(demande.port.trim())
            .ok_or(regle("sms.port_absent"))?;
        let info = modem.identifier().map_err(|_| regle("sms.cle_muette"))?;
        let code = parametre::texte(&mut cnx, PARAMETRE_USSD, "", &contexte).await?;
        let credit = if code.trim().is_empty() {
            None
        } else {
            modem.lire_credit(code.trim()).ok()
        };
        Ok(EssaiCle {
            fabricant: info.fabricant,
            modele: info.modele,
            operateur: info.operateur,
            signal: info.signal,
            credit: credit.as_ref().and_then(|c| c.montant),
            credit_texte: credit.map(|c| c.texte).unwrap_or_default(),
        })
    }

    /// Choisit le port de la clé.
    pub async fn choisir_port(
        &self,
        session: &Session,
        demande: &DemandePort,
    ) -> Resultat<FileMessages> {
        let port = demande.port.trim().to_owned();
        if port.chars().count() > 40 {
            return Err(regle("sms.port_absent"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        let avant = parametre::texte(unite.connexion(), PARAMETRE_PORT, "", &contexte).await?;
        if avant != port {
            parametre::definir(
                unite.connexion(),
                &gesco_core::core::parametre::Parametre {
                    etablissement_id: session.etablissement_id,
                    cle: PARAMETRE_PORT.to_owned(),
                    valeur: gesco_core::core::parametre::ValeurParametre::Texte(port.clone()),
                    modifiable_par_utilisateur: true,
                },
                &contexte,
            )
            .await?;
            unite.auditer(
                "parametre",
                PARAMETRE_PORT,
                Operation::Modification,
                Some(serde_json::json!({ "port": avant })),
                Some(serde_json::json!({ "port": port })),
                "file_sms.choisir_port",
            );
            unite.emettre(
                TypeEvenement::PortSmsChoisi,
                "parametre",
                PARAMETRE_PORT,
                serde_json::json!({ "port": port }),
            );
        }
        self.noyau.terminer(unite).await?;
        self.lire(session, &DemandeFile { etat: None }).await
    }

    /// Relit le crédit de la clé tout de suite : la file s'arrête ou repart selon le seuil.
    pub async fn relire_credit(
        &self,
        session: &Session,
        source: &dyn SourceCanaux,
    ) -> Resultat<FileMessages> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        {
            let mut cnx = self.connexion().await?;
            let port = parametre::texte(&mut cnx, PARAMETRE_PORT, "", &contexte).await?;
            let modem = source.modem(&port).ok_or(regle("sms.port_absent"))?;
            if parametre::texte(&mut cnx, PARAMETRE_USSD, "", &contexte)
                .await?
                .trim()
                .is_empty()
            {
                return Err(regle("sms.ussd_absent"));
            }
            let avant = Self::etat_canal(&mut cnx, &contexte).await?;
            let (apres, lu) = Self::lire_credit(&mut cnx, &modem, avant, &contexte).await?;
            if !lu {
                return Err(regle("sms.cle_muette"));
            }
            file_sms::enregistrer_etat_canal(&mut cnx, &apres, &contexte).await?;
        }
        self.lire(session, &DemandeFile { etat: None }).await
    }

    /// Fait repartir la file arrêtée, après une recharge que le code USSD ne peut pas lire.
    pub async fn reprendre(&self, session: &Session) -> Resultat<FileMessages> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        let avant = Self::etat_canal(unite.connexion(), &contexte).await?;
        if !avant.arretee {
            return Err(regle("sms.file_non_arretee"));
        }
        let apres = avant.reprise();
        file_sms::enregistrer_etat_canal(unite.connexion(), &apres, &contexte).await?;
        let id = session.etablissement_id.vers_chaine();
        unite.auditer(
            "etat_canal_sms",
            &id,
            Operation::Modification,
            Some(serde_json::json!({ "arretee": true, "raison": avant.raison })),
            Some(serde_json::json!({ "arretee": false })),
            "file_sms.reprendre",
        );
        unite.emettre(
            TypeEvenement::FileSmsReprise,
            "etat_canal_sms",
            &id,
            serde_json::json!({ "raison": avant.raison }),
        );
        self.noyau.terminer(unite).await?;
        self.lire(session, &DemandeFile { etat: None }).await
    }

    /// Avertissements du tableau de bord : la file arrêtée faute de crédit, la clé qui ne répond
    /// pas. Rien quand l'école n'envoie pas de SMS.
    pub async fn avertissements(&self, session: &Session) -> Resultat<Vec<&'static str>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let canal = parametre::texte(&mut cnx, PARAMETRE_CANAL, "aucun", &contexte).await?;
        if canal == "aucun" {
            return Ok(Vec::new());
        }
        let etat = Self::etat_canal(&mut cnx, &contexte).await?;
        let mut liste = Vec::new();
        if etat.arretee {
            liste.push("avertissement.sms_arretes");
        }
        if canal == "modem" && etat.cle_absente {
            liste.push("avertissement.sms_cle_absente");
        }
        Ok(liste)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debut_du_mois_civil() {
        // 14 octobre 2026, 10 h UTC.
        let t = Horodatage::depuis_millis_epoque(1_791_972_000_000);
        let debut = debut_du_mois(t).civil_utc();
        assert_eq!(
            (debut.annee, debut.mois, debut.jour, debut.heure),
            (2026, 10, 1, 0)
        );
    }

    #[test]
    fn apercu_d_un_texte() {
        let a = apercu(&DemandeApercu {
            texte: " Élève absent ".to_owned(),
        });
        assert_eq!(a.texte, "Eleve absent");
        assert_eq!((a.caracteres, a.sms), (12, 1));
    }
}
