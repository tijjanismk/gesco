//! Frontière de l'application (spécification §7.1, §12.2) : le point de passage unique entre le
//! frontend et les services. Une commande nommée et son entrée JSON arrivent ; la garde vérifie
//! session et permission côté Rust (R-21), puis le gestionnaire de la commande s'exécute.
//!
//! La même frontière sert la commande Tauri du poste et, en phase 11, la route HTTP du mode
//! réseau local (ADR-014) : c'est pourquoi elle vit ici et non dans `gesco-app`. Le contrat des
//! commandes est déclaré une fois, dans la macro `contrat!`, d'où sont tirés l'aiguillage et les
//! types TypeScript (ADR-007, R-37).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use base64::Engine;
use gesco_core::commun::{Horodatage, Id};
use gesco_core::core::{
    AnneeScolaire, CleDonnees, Etablissement, MotifSauvegarde, Operation, Parametre, Permission,
    PhraseSecours, Poste, StatutEtablissement, TypeEvenement, Utilisateur, ValeurParametre,
};
use gesco_core::scolarite::{Classe, Cycle, Niveau, Serie};
use gesco_db::depots::document::Document;
use gesco_db::depots::sauvegarde::Sauvegarde;
use gesco_db::depots::{annee_scolaire, audit, etablissement, parametre, role, utilisateur};
use gesco_db::unite::Durabilite;
use gesco_db::{Base, OptionsBase, nouvel_id};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};

use crate::absences::{
    AbsencesPeriode, BilanReleve, DemandeAbsencesPeriode, DemandeEnregistrementReleve,
    DemandeJustification, DemandeReleve, ReleveAbsences, ServiceAbsences,
};
use crate::absences_seances::{DemandeReleveParSeance, ReleveParSeance, ServiceAbsencesSeances};
use crate::administration::{
    FicheRole, FicheUtilisateur, PageJournal, RechercheJournal, ServiceAdministration,
    VerificationAudit,
};
use crate::annees::ServiceAnnees;
use crate::archive_journal::{DemandeVerificationArchive, VerificationArchive};
use crate::authentification::{NouvelUtilisateur, ServiceAuthentification};
use crate::bulletins::{
    BulletinsClasse, DemandeAppreciation, DemandeBulletins, DemandeCorrection, DemandeIdBulletin,
    DemandeReprise, ReprisesClasse, ServiceBulletins, donnees_impression,
};
use crate::caisse::{
    DemandeCloture, DemandeOuverture, DemandeSession, DemandeVisa, ServiceCaisse, SessionAffichee,
};
use crate::cantine::{Cantine, DemandeCantine, DemandeRepas, ServiceCantine};
use crate::cles::{EspaceCles, EtatCles, Trousseau};
use crate::commandes::{Acces, Commande};
use crate::comptes::ServiceComptes;
use crate::conges::{
    AbsencesEtConges, DemandeAbsencePersonnel, DemandeConge, DemandeIdConge, DemandeMoisConges,
    DemandeMotifAbsence, DemandeMotifConge, DemandeTypeConge, ServiceConges,
};
use crate::departs::{
    DemandeDepart, DemandeExclusion, DemandeLevee, DemandeTransfert, ServiceDeparts,
    TransfertEnregistre,
};
use crate::depenses::{
    DemandeAnnulationDepense, DemandeDecisionDepense, DemandeDepense, DemandeMoisDepenses,
    DemandePayerDepense, DemandePieceDepense, DemandePieceDepenseSous, Depenses, ServiceDepenses,
};
use crate::discipline::{
    DemandeAnnulationSanction, DemandeDossierDiscipline, DemandeSanction, DossierDiscipline,
    RegistreDiscipline, ServiceDiscipline,
};
use crate::documents::{
    AtelierDocuments, EffetsEchoues, ServiceDocuments, date_longue, traiter_effets,
};
use crate::documents_paie::{DemandeBulletinPaie, MonBulletinPaie, ServiceDocumentsPaie};
use crate::effets::FileEffets;
use crate::eleves::{
    DemandeDetachement, DemandeDoublons, DemandeEleve, DemandeModificationEleve,
    DemandeModificationResponsable, DemandePhoto, DemandeRattachement, DemandeRecherche,
    EleveTrouve, FicheEleve, ReglagesScolarite, ResponsableTrouve, SaisieEleve, ServiceEleves,
};
use crate::emploi_du_temps::{
    DemandeEmploiClasse, DemandeEmploiEnseignant, DemandeRetraitSeance, DemandeSeance,
    EmploiDuTemps, ServiceEmploiDuTemps,
};
use crate::encaissement::{
    DemandeAnnulationVersement, DemandeEncaissement, DemandeFamille, DemandePreparation,
    PreparationEncaissement, ServiceEncaissement,
};
use crate::engagements::{
    DemandeClotureEngagement, DemandeEngagement, DemandeEngagementProposee, DemandeIdEngagement,
    DemandeMoisPayes, EngagementAEcheance, EngagementAffiche, ServiceEngagements,
};
use crate::enseignants::{
    AffectationAffichee, DemandeAffectation, DemandeEnseignant, DemandeIdAffectation,
    DemandeMaitre, FicheEnseignant, ServiceEnseignants,
};
use crate::erreur::{ErreurService, MotifRefus, Resultat};
use crate::espace::EspaceDonnees;
use crate::evaluations::{
    ApercuImportNotes, DemandeDeverrouillage, DemandeEvaluation, DemandeEvaluations,
    DemandeIdEvaluation, DemandeImportNotes, DemandeNote, EvaluationAffichee, GrilleNotes,
    ServiceEvaluations,
};
use crate::export::{DemandeExport, FichierEcrit};
use crate::facturation::{
    DemandeChangementPlan, FactureAffichee, LigneRattrapage, RapportRattrapage, ServiceFacturation,
    SituationInscription,
};
use crate::fiche_etablissement::{ChampsEtablissement, FicheEtablissement, ServiceEtablissement};
use crate::fin_annee::{
    BilanReinscriptions, DecisionsClasse, DemandeClasseFinAnnee, DemandeDecision, DemandePassage,
    DemandeReouverture, FinAnnee, ServiceFinAnnee,
};
use crate::fournisseurs::{
    DemandeAnnulationFacture, DemandeFacture, DemandeFournisseur, Fournisseurs, ServiceFournisseurs,
};
use crate::frais::{
    DemandeCreationGrille, DemandeGrille, DemandeGrillesAnnee, DemandeModificationGrille,
    GrilleAffichee, MiseEnVigueur, ServiceFrais,
};
use crate::fusion::{ApercuFusion, DemandeFusion, ServiceFusion};
use crate::garde::{Garde, PorteeTotale};
use crate::heures::{
    DemandeDeverrouillageHeures, DemandeMois, DemandeSaisieHeures, DemandeVisaHeures, EtatHeures,
    ServiceHeures,
};
use crate::horloge::Horloge;
use crate::import_eleves::{
    Analyse, Apercu, DemandeAnalyse, FichierImporte, Rapport, Reglages, ServiceImport,
};
use crate::import_personnel::{
    AnalyseImportPersonnel, BilanImportPersonnel, DemandeImportPersonnel, DemandeModelePersonnel,
    ServiceImportPersonnel,
};
use crate::initialisation::{PremierDemarrage, ServiceInitialisation};
use crate::inscriptions::{
    DemandeAbandon, DemandeChangementClasse, DemandeIdInscription, DemandeInscription,
    DemandeListeInscriptions, DemandeModificationInscription, DemandeValidation, LigneInscription,
    ResultatInscription, ServiceInscriptions,
};
use crate::journal_tresorerie::{
    Budget, DemandeBudget, DemandeJournalTresorerie, DemandeLigneBudget, JournalTresorerie,
    ServiceJournal,
};
use crate::licence::{EtatLicenceAffiche, ServiceLicence};
use crate::livres::{ApercuReleve, DemandeApercuReleve, DemandeImportReleve, ResultatReleve};
use crate::livres::{
    DemandeCompteDuMode, DemandeLivre, DemandePointage, DemandeTransfertCompte, EtatComptes,
    LivreCompte, ServiceLivres,
};
use crate::messages::{
    DemandeConsentement, DemandeMessagesResponsable, DemandeRemiseALEssai, MessagesResponsable,
    ServiceMessages,
};
use crate::noyau::{Noyau, OptionsNoyau};
use crate::paie::{
    DemandeAttribution, DemandeElement, DemandeVersion, ElementsPaie, ReglagesPaie, ServicePaie,
};
use crate::paie_mois::{
    DemandeAbsencePaie, DemandeIdSaisie, DemandeMoisPaie, DemandeSaisiePaie, PaieDuMois,
    ServicePaieMois,
};
use crate::paiements_personnel::{
    DemandeAnnulationAvance, DemandeAnnulationPaiement, DemandeAvance, DemandePaiementSalaire,
    DemandeResteDuMois, PaiementFait, PaiementsPersonnel, ServicePaiementsPersonnel,
};
use crate::pedagogie::{
    DemandeCalendrier, DemandeMatiere, DemandeNombrePeriodes, DemandeProgramme, DemandeRegle,
    Pedagogie, ServicePedagogie,
};
use crate::personnel::{
    DemandeCategoriePersonnel, DemandeDepartMembre, DemandeFonctionPersonnel, DemandeIdMembre,
    DemandeIdPiece, DemandeMembre, DemandeMembreEnseignant, DemandePieceDossier, DemandePieceSous,
    FicheMembre, MaFiche, MembreAffiche, ReferentielPersonnel, ServicePersonnel,
};
use crate::recettes::{
    DemandeAnnulationRecette, DemandeMoisRecettes, DemandeRecette, RecettesDuMois, ServiceRecettes,
};
use crate::recouvrement::{
    DemandeJournal, JournalEncaissements, ListeImpayes, ServiceRecouvrement,
};
use crate::referentiel::{DemandeFiliere, FicheClasse, Referentiel, ServiceReferentiel};
use crate::remises::{
    DemandeDecisionRemise, DemandeIdRemise, DemandeRemise, RemiseEnAttente, ServiceRemises,
};
use crate::restauration::{self, RapportRestauration, Tentatives};
use crate::roles::ServiceRoles;
use crate::sauvegarde::{
    ApercuSauvegarde, Avertissement, Destination, PARAMETRE_CERTIFICAT, RapportTest,
    ServiceSauvegarde,
};
use crate::secrets::{MotDePasse, Pin, mot_de_passe_correspond};
use crate::session::{Jeton, PolitiqueSession, Session};
use crate::tableau::{ServiceTableau, TableauDeBord};
use crate::tresorerie::{
    DemandeCategorieTresorerie, DemandeCompteTresorerie, ReglagesTresorerie, ServiceTresorerie,
    Tresorerie,
};

/// Paramètres de démarrage de l'application sur un poste.
pub struct OptionsApplication {
    /// Racine de l'espace de données.
    pub racine: PathBuf,
    /// Nom du poste.
    pub poste: Poste,
    /// Empreinte de la machine, pour l'audit.
    pub empreinte_machine: String,
    /// Trousseau du système (DPAPI sous Windows).
    pub trousseau: Arc<dyn Trousseau>,
    /// Horloge.
    pub horloge: Arc<dyn Horloge>,
    /// Politique de session.
    pub politique_session: PolitiqueSession,
    /// Version de l'application, portée par les manifestes de sauvegarde.
    pub version_applicative: String,
    /// Clé publique contre laquelle les licences sont vérifiées : celle de l'éditeur en
    /// production ([`crate::licence::cle_publique_editeur`]), une clé de test ailleurs.
    pub cle_licence: [u8; 32],
}

/// Erreur rendue au frontend : une clé de traduction (R-39), jamais un message technique seul.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ErreurCommande {
    /// Clé du catalogue i18n.
    pub cle: String,
    /// Fin d'une attente imposée (compte ou phrase verrouillés), millisecondes depuis l'époque.
    pub jusqua: Option<i64>,
    /// Détail technique pour l'assistance, présent seulement pour les erreurs internes.
    pub detail: Option<String>,
}

impl From<ErreurService> for ErreurCommande {
    fn from(erreur: ErreurService) -> Self {
        let jusqua = match &erreur {
            ErreurService::Authentification(
                MotifRefus::Verrouille { jusqua } | MotifRefus::PhraseVerrouillee { jusqua },
            ) => Some(*jusqua),
            _ => None,
        };
        let detail = matches!(
            erreur,
            ErreurService::Base(_)
                | ErreurService::Fichier(_)
                | ErreurService::Secret(_)
                | ErreurService::Json(_)
                | ErreurService::Document(_)
        )
        .then(|| erreur.to_string());
        Self {
            cle: erreur.cle_traduction().to_owned(),
            jusqua,
            detail,
        }
    }
}

/// Étape du démarrage, qui décide de l'écran affiché.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum PhaseDemarrage {
    /// La clé de données attend la phrase de secours (autre poste, trousseau perdu).
    ClesVerrouillees,
    /// Aucun établissement : création ou restauration d'une sauvegarde.
    PremierDemarrage,
    /// Écran de connexion.
    Pret,
}

/// Établissement servi par le poste.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ResumeEtablissement {
    /// Identifiant.
    pub id: Id<Etablissement>,
    /// Nom.
    pub nom: String,
}

/// État de démarrage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct EtatDemarrage {
    /// Étape.
    pub phase: PhaseDemarrage,
    /// Établissement, une fois créé.
    pub etablissement: Option<ResumeEtablissement>,
    /// Version de l'application.
    pub version_applicative: String,
}

/// Phrase de secours affichée une seule fois, avec son certificat à imprimer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct PhraseAffichee {
    /// Identifiant de l'enveloppe de secours, imprimé sur le certificat.
    pub enveloppe_id: String,
    /// Les douze mots.
    pub mots: Vec<String>,
    /// Certificat de clé de secours, PDF en base64. Jamais écrit sur le disque du poste.
    pub certificat_pdf: String,
}

/// Données du premier démarrage.
#[derive(Deserialize, specta::Type)]
pub struct DemandePremierDemarrage {
    /// Organisation propriétaire.
    pub organisation: String,
    /// Nom de l'établissement.
    pub etablissement_nom: String,
    /// Statut de l'établissement.
    pub statut: StatutEtablissement,
    /// Code de l'académie d'enseignement.
    pub code_academie: String,
    /// Code du centre d'animation pédagogique.
    pub code_cap: String,
    /// Adresse.
    pub adresse: String,
    /// Téléphones.
    pub telephones: Vec<String>,
    /// Identifiant de connexion de l'administrateur.
    pub administrateur_identifiant: String,
    /// Nom affiché de l'administrateur.
    pub administrateur_nom: String,
    /// Mot de passe initial de l'administrateur.
    pub administrateur_mot_de_passe: String,
    /// Cycles enseignés par l'école.
    pub cycles: Vec<Cycle>,
}

/// Résultat du premier démarrage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct PremierDemarrageEffectue {
    /// Établissement créé.
    pub etablissement: ResumeEtablissement,
    /// Phrase de secours, affichée une seule fois.
    pub phrase: PhraseAffichee,
}

/// Identifiants de connexion.
#[derive(Deserialize, specta::Type)]
pub struct DemandeConnexion {
    /// Identifiant.
    pub identifiant: String,
    /// Mot de passe.
    pub mot_de_passe: String,
}

/// Déverrouillage d'une session par code PIN.
#[derive(Deserialize, specta::Type)]
pub struct DemandePin {
    /// Session à rouvrir.
    pub jeton: Jeton,
    /// Code PIN.
    pub pin: String,
}

/// Saisie de la phrase de secours.
#[derive(Deserialize, specta::Type)]
pub struct DemandePhrase {
    /// Les douze mots.
    pub phrase: String,
}

/// Archive désignée par son chemin.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeArchive {
    /// Chemin du fichier.
    pub chemin: String,
}

/// Restauration d'une archive.
#[derive(Deserialize, specta::Type)]
pub struct DemandeRestauration {
    /// Chemin du fichier.
    pub chemin: String,
    /// Phrase de secours, si la clé du poste ne suffit pas.
    pub phrase: Option<String>,
}

/// Définition du code PIN, mot de passe redemandé.
#[derive(Deserialize, specta::Type)]
pub struct DemandeDefinirPin {
    /// Mot de passe actuel.
    pub mot_de_passe: String,
    /// Nouveau code PIN, 4 à 6 chiffres.
    pub pin: String,
}

/// Changement de mot de passe.
#[derive(Deserialize, specta::Type)]
pub struct DemandeChangementMotDePasse {
    /// Mot de passe actuel.
    pub ancien: String,
    /// Nouveau mot de passe.
    pub nouveau: String,
}

/// Session courante et ce qu'elle autorise : le frontend masque les sections sans permission,
/// la garde reste la seule décision (R-01).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct SessionCourante {
    /// Session.
    pub session: Session,
    /// Permissions de l'utilisateur.
    pub permissions: Vec<Permission>,
    /// Changement de mot de passe exigé.
    pub doit_changer_mot_de_passe: bool,
    /// Inactivité avant verrouillage (§6.2) : l'écran se verrouille de lui-même, sans attendre
    /// la prochaine commande refusée.
    pub verrouillage_inactivite_ms: i64,
    /// L'utilisateur a un code PIN ; sinon un avertissement personnel l'invite à en définir un.
    pub pin_defini: bool,
    /// Libellé de l'année scolaire courante, affiché dans le bandeau.
    pub annee_courante: Option<String>,
    /// Jour courant selon l'horloge de GESCO : l'interface ne lit pas celle du poste (R-03).
    pub aujourd_hui: Option<gesco_core::commun::DateCivile>,
}

/// Création d'un utilisateur.
#[derive(Deserialize, specta::Type)]
pub struct DemandeCreationUtilisateur {
    /// Identifiant de connexion.
    pub identifiant: String,
    /// Nom affiché.
    pub nom_affiche: String,
    /// Mot de passe initial.
    pub mot_de_passe_initial: String,
    /// Codes des rôles.
    pub roles: Vec<String>,
}

/// Modification d'un utilisateur.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeModificationUtilisateur {
    /// Compte modifié.
    pub id: Id<Utilisateur>,
    /// Nom affiché.
    pub nom_affiche: String,
    /// Compte actif.
    pub actif: bool,
}

/// Attribution ou retrait d'un rôle.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeAttributionRole {
    /// Utilisateur.
    pub utilisateur_id: Id<Utilisateur>,
    /// Code du rôle.
    pub role: String,
    /// Vrai pour attribuer, faux pour retirer.
    pub attribue: bool,
}

/// Réinitialisation du mot de passe d'un compte par un administrateur.
#[derive(Deserialize, specta::Type)]
pub struct DemandeReinitialisation {
    /// Compte.
    pub id: Id<Utilisateur>,
    /// Mot de passe initial, à changer à la connexion suivante.
    pub mot_de_passe_initial: String,
}

/// Compte désigné par son identifiant technique.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeUtilisateur {
    /// Compte.
    pub id: Id<Utilisateur>,
}

/// Déverrouillage d'une session par le mot de passe, pour un utilisateur sans PIN.
#[derive(Deserialize, specta::Type)]
pub struct DemandeDeverrouillageMotDePasse {
    /// Session à rouvrir.
    pub jeton: Jeton,
    /// Mot de passe de son utilisateur.
    pub mot_de_passe: String,
}

/// Logo à poser, en base64, ou `null` pour le retirer.
#[derive(Deserialize, specta::Type)]
pub struct DemandeLogo {
    /// Contenu du fichier PNG ou JPEG, en base64.
    pub contenu_base64: Option<String>,
}

/// Libellé et bornes d'une année scolaire, en millisecondes UTC.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeAnnee {
    /// « 2026-2027 ».
    pub libelle: String,
    /// Premier jour.
    pub debut: Horodatage,
    /// Dernier jour.
    pub fin: Horodatage,
}

/// Nouvelle version d'une année en préparation.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeModificationAnnee {
    /// Année.
    pub id: Id<AnneeScolaire>,
    /// « 2026-2027 ».
    pub libelle: String,
    /// Premier jour.
    pub debut: Horodatage,
    /// Dernier jour.
    pub fin: Horodatage,
}

/// Année désignée par son identifiant.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeIdAnnee {
    /// Année.
    pub id: Id<AnneeScolaire>,
}

/// Création d'un rôle personnalisé.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeCreationRole {
    /// Libellé affiché.
    pub libelle: String,
    /// Codes des permissions accordées.
    pub permissions: Vec<String>,
}

/// Nouvelle composition d'un rôle.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeModificationRole {
    /// Code du rôle.
    pub code: String,
    /// Libellé ; ignoré pour un rôle livré.
    pub libelle: String,
    /// Codes des permissions accordées.
    pub permissions: Vec<String>,
}

/// Rôle désigné par son code.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeRole {
    /// Code du rôle.
    pub code: String,
}

/// Libellé et état d'un niveau.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeNiveau {
    /// Niveau.
    pub id: Id<Niveau>,
    /// Nouveau libellé.
    pub libelle: String,
    /// Faux pour un niveau que l'école n'enseigne pas.
    pub actif: bool,
}

/// Libellé et état d'une série.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeSerie {
    /// Série.
    pub id: Id<Serie>,
    /// Nouveau libellé.
    pub libelle: String,
    /// Faux pour une série que l'école n'ouvre pas.
    pub actif: bool,
}

/// Cycles que l'école enseigne.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeCycles {
    /// Cycles enseignés ; au moins un.
    pub cycles: Vec<Cycle>,
}

/// Année dont on veut les classes.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeClasses {
    /// Année scolaire.
    pub annee_id: Id<AnneeScolaire>,
}

/// Nouvelle classe.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeCreationClasse {
    /// Année scolaire.
    pub annee_id: Id<AnneeScolaire>,
    /// Niveau.
    pub niveau_id: Id<Niveau>,
    /// Série, exigée en 11e et 12e.
    pub serie_id: Option<Id<Serie>>,
    /// Libellé : « 7e A », « 12TSE ».
    pub libelle: String,
}

/// Nouveau libellé d'une classe.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeModificationClasse {
    /// Classe.
    pub id: Id<Classe>,
    /// Nouveau libellé.
    pub libelle: String,
}

/// Classe désignée.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeClasse {
    /// Classe.
    pub id: Id<Classe>,
}

/// Modification d'un paramètre.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeParametre {
    /// Clé.
    pub cle: String,
    /// Nouvelle valeur, du type du paramètre.
    pub valeur: ValeurParametre,
}

/// Régénération de la phrase de secours, mot de passe redemandé (§6.3).
#[derive(Deserialize, specta::Type)]
pub struct DemandeRegeneration {
    /// Mot de passe de l'administrateur connecté.
    pub mot_de_passe: String,
}

/// Fichier de licence importé.
#[derive(Deserialize, specta::Type)]
pub struct DemandeLicence {
    /// Contenu du fichier, tel qu'émis par l'éditeur.
    pub contenu: String,
}

/// Versement encaissé et son reçu.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct RecuEmis {
    /// Versement.
    pub versement: gesco_core::finance::Versement,
    /// Reçu imprimable ; marqué duplicata pour une clé déjà vue.
    pub recu: DocumentAffiche,
}

/// Versement annulé : son reçu marqué « annulé », et la sortie de régularisation passée si sa
/// session était close.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct AnnulationEmise {
    /// Versement annulé.
    pub versement: gesco_core::finance::Versement,
    /// Reçu marqué annulé, avec le motif.
    pub recu: DocumentAffiche,
    /// Sortie de régularisation.
    pub regularisation: Option<gesco_core::finance::MouvementCaisse>,
}

/// Versement désigné.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeVersement {
    /// Versement.
    pub id: gesco_core::commun::Id<gesco_core::finance::Versement>,
}

/// Document produit, avec son contenu pour l'aperçu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DocumentAffiche {
    /// Ligne enregistrée.
    pub document: Document,
    /// PDF en base64.
    pub pdf: String,
}

fn base64(octets: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(octets)
}

/// Base, clé et tâche des effets d'une installation déverrouillée.
struct Ouverte {
    noyau: Arc<Noyau>,
    cle: CleDonnees,
    tache_effets: tokio::task::JoinHandle<()>,
}

/// Ce que la garde a établi pour un appel.
struct Appel {
    ouverte: Option<Arc<Ouverte>>,
    session: Option<Session>,
    jeton: Option<Jeton>,
}

impl Appel {
    fn ouverte(&self) -> Resultat<&Ouverte> {
        self.ouverte.as_deref().ok_or(ErreurService::Regle {
            cle: "application.cles_verrouillees",
        })
    }

    fn session(&self) -> Resultat<&Session> {
        self.session
            .as_ref()
            .ok_or(ErreurService::Authentification(MotifRefus::SessionInvalide))
    }

    fn jeton(&self) -> Resultat<&Jeton> {
        self.jeton
            .as_ref()
            .ok_or(ErreurService::Authentification(MotifRefus::SessionInvalide))
    }
}

/// L'application sur un poste : espace de données, clés, base ouverte quand la clé l'est.
pub struct Application {
    espace: EspaceDonnees,
    cles: EspaceCles,
    atelier: Arc<AtelierDocuments>,
    poste: Poste,
    empreinte_machine: String,
    horloge: Arc<dyn Horloge>,
    politique_session: PolitiqueSession,
    version_applicative: String,
    cle_licence: [u8; 32],
    etat: RwLock<Option<Arc<Ouverte>>>,
    tentatives: Tentatives,
    echecs: EffetsEchoues,
    /// Fichiers d'élèves déposés, le temps de l'assistant d'import (§10).
    imports: Mutex<HashMap<String, FichierImporte>>,
}

macro_rules! contrat {
    ($( $nom:ident ( $entree:ty ) -> $sortie:ty ; )+) => {
        /// Signature d'une commande : type d'entrée, type de sortie.
        #[derive(specta::Type)]
        pub struct Signature<E, S> {
            /// Entrée de la commande.
            pub entree: E,
            /// Sortie de la commande.
            pub sortie: S,
        }

        /// Contrat de la frontière, une entrée par commande. Jamais instancié : il sert à
        /// générer les types TypeScript du frontend (R-37).
        #[derive(specta::Type)]
        pub struct Contrat {
            $(
                /// Signature de la commande du même nom.
                pub $nom: Signature<$entree, $sortie>,
            )+
        }

        /// Commandes dont la frontière connaît le gestionnaire ; un test les compare au
        /// catalogue des accès.
        pub const COMMANDES_IMPLEMENTEES: &[&str] = &[$(stringify!($nom)),+];

        impl Application {
            async fn aiguiller(
                &self,
                nom: &str,
                appel: Appel,
                entree: serde_json::Value,
            ) -> Resultat<serde_json::Value> {
                match nom {
                    $(
                        stringify!($nom) => {
                            let entree: $entree = serde_json::from_value(entree).map_err(|_| {
                                ErreurService::Regle { cle: "commande.entree_invalide" }
                            })?;
                            // Chaque commande sur le tas : l'aiguillage ne porte qu'un pointeur, et sa
                            // taille ne grandit pas avec la plus grosse commande (débordement de pile
                            // constaté le 13 octobre 2026).
                            let sortie: $sortie = Box::pin(self.$nom(appel, entree)).await?;
                            Ok(serde_json::to_value(sortie)?)
                        }
                    )+
                    _ => Err(ErreurService::Regle { cle: "commande.inconnue" }),
                }
            }
        }
    };
}

contrat! {
    etat_demarrage(()) -> EtatDemarrage;
    premier_demarrage(DemandePremierDemarrage) -> PremierDemarrageEffectue;
    connexion(DemandeConnexion) -> Session;
    deverrouiller_pin(DemandePin) -> Session;
    deverrouiller_mot_de_passe(DemandeDeverrouillageMotDePasse) -> Session;
    sessions_du_poste(()) -> Vec<Session>;
    cles_deverrouiller_phrase(DemandePhrase) -> EtatDemarrage;
    sauvegarde_apercu(DemandeArchive) -> ApercuSauvegarde;
    sauvegarde_restaurer_initiale(DemandeRestauration) -> RapportRestauration;
    deconnexion(()) -> ();
    verrouiller(()) -> Session;
    definir_pin(DemandeDefinirPin) -> ();
    changer_mot_de_passe(DemandeChangementMotDePasse) -> ();
    session_courante(()) -> SessionCourante;
    avertissements(()) -> Vec<Avertissement>;
    utilisateurs_lister(()) -> Vec<FicheUtilisateur>;
    utilisateur_creer(DemandeCreationUtilisateur) -> Utilisateur;
    utilisateur_modifier(DemandeModificationUtilisateur) -> Utilisateur;
    utilisateur_reinitialiser_mot_de_passe(DemandeReinitialisation) -> Utilisateur;
    utilisateur_deverrouiller(DemandeUtilisateur) -> Utilisateur;
    roles_lister(()) -> Vec<FicheRole>;
    role_attribuer(DemandeAttributionRole) -> ();
    permissions_catalogue(()) -> Vec<Permission>;
    role_creer(DemandeCreationRole) -> FicheRole;
    role_modifier(DemandeModificationRole) -> FicheRole;
    role_retablir(DemandeRole) -> FicheRole;
    role_supprimer(DemandeRole) -> ();
    annees_lister(()) -> Vec<AnneeScolaire>;
    annee_creer(DemandeAnnee) -> AnneeScolaire;
    annee_modifier(DemandeModificationAnnee) -> AnneeScolaire;
    annee_activer(DemandeIdAnnee) -> AnneeScolaire;
    fin_annee_lire(()) -> FinAnnee;
    decisions_classe_lire(DemandeClasseFinAnnee) -> DecisionsClasse;
    decision_poser(DemandeDecision) -> DecisionsClasse;
    decisions_valider_classe(DemandeClasseFinAnnee) -> DecisionsClasse;
    passage_regler(DemandePassage) -> FinAnnee;
    annee_cloturer(DemandeIdAnnee) -> AnneeScolaire;
    annee_rouvrir(DemandeReouverture) -> AnneeScolaire;
    classes_reprendre(DemandeIdAnnee) -> FinAnnee;
    reinscriptions_preparer(DemandeIdAnnee) -> BilanReinscriptions;
    annee_supprimer(DemandeIdAnnee) -> ();
    niveaux_lister(()) -> Referentiel;
    cycles_definir(DemandeCycles) -> Referentiel;
    niveau_modifier(DemandeNiveau) -> Niveau;
    serie_modifier(DemandeSerie) -> Serie;
    filiere_creer(DemandeFiliere) -> Referentiel;
    classes_lister(DemandeClasses) -> Vec<FicheClasse>;
    classe_creer(DemandeCreationClasse) -> Classe;
    classe_modifier(DemandeModificationClasse) -> Classe;
    classe_supprimer(DemandeClasse) -> ();
    scolarite_reglages(()) -> ReglagesScolarite;
    eleves_rechercher(DemandeRecherche) -> Vec<EleveTrouve>;
    eleve_lire(DemandeEleve) -> FicheEleve;
    eleve_creer(SaisieEleve) -> FicheEleve;
    eleve_modifier(DemandeModificationEleve) -> FicheEleve;
    eleve_photo_definir(DemandePhoto) -> FicheEleve;
    eleve_doublons_probables(DemandeDoublons) -> Vec<EleveTrouve>;
    eleves_fusion_apercu(DemandeFusion) -> ApercuFusion;
    eleves_fusionner(DemandeFusion) -> FicheEleve;
    responsables_rechercher(DemandeRecherche) -> Vec<ResponsableTrouve>;
    inscriptions_lister(DemandeListeInscriptions) -> Vec<LigneInscription>;
    inscription_creer(DemandeInscription) -> ResultatInscription;
    inscription_modifier(DemandeModificationInscription) -> ResultatInscription;
    inscription_valider(DemandeValidation) -> ResultatInscription;
    inscription_abandonner(DemandeAbandon) -> ResultatInscription;
    inscription_changer_classe(DemandeChangementClasse) -> ResultatInscription;
    inscription_fiche_produire(DemandeIdInscription) -> DocumentAffiche;
    responsable_rattacher(DemandeRattachement) -> FicheEleve;
    responsable_modifier(DemandeModificationResponsable) -> FicheEleve;
    responsable_detacher(DemandeDetachement) -> FicheEleve;
    etablissement_lire(()) -> FicheEtablissement;
    etablissement_modifier(ChampsEtablissement) -> FicheEtablissement;
    etablissement_logo_definir(DemandeLogo) -> FicheEtablissement;
    parametres_lister(()) -> Vec<Parametre>;
    parametre_definir(DemandeParametre) -> Parametre;
    audit_lister(RechercheJournal) -> PageJournal;
    audit_verifier(()) -> VerificationAudit;
    sauvegarde_creer(()) -> Vec<Sauvegarde>;
    sauvegardes_lister(()) -> Vec<Sauvegarde>;
    sauvegarde_tester(()) -> RapportTest;
    sauvegarde_destinations_lister(()) -> Vec<Destination>;
    sauvegarde_destinations_definir(Vec<Destination>) -> Vec<Destination>;
    certificat_confirmer(()) -> ();
    sauvegarde_restaurer(DemandeRestauration) -> RapportRestauration;
    phrase_secours_regenerer(DemandeRegeneration) -> PhraseAffichee;
    document_essai_produire(()) -> DocumentAffiche;
    exporter_xlsx(DemandeExport) -> FichierEcrit;
    import_eleves_analyser(DemandeAnalyse) -> Analyse;
    import_eleves_valider(Reglages) -> Apercu;
    import_eleves_appliquer(Reglages) -> Rapport;
    grilles_lister(DemandeGrillesAnnee) -> Vec<GrilleAffichee>;
    grille_creer(DemandeCreationGrille) -> GrilleAffichee;
    grille_modifier(DemandeModificationGrille) -> GrilleAffichee;
    grille_mettre_en_vigueur(DemandeGrille) -> MiseEnVigueur;
    situation_financiere(DemandeEleve) -> Vec<SituationInscription>;
    echeancier_changer_plan(DemandeChangementPlan) -> FactureAffichee;
    factures_rattraper_apercu(()) -> Vec<LigneRattrapage>;
    factures_rattraper(()) -> RapportRattrapage;
    remise_proposer(DemandeRemise) -> gesco_core::finance::Remise;
    remise_accorder(DemandeIdRemise) -> gesco_core::finance::Remise;
    remise_refuser(DemandeDecisionRemise) -> gesco_core::finance::Remise;
    remise_annuler(DemandeDecisionRemise) -> gesco_core::finance::Remise;
    remises_en_attente(()) -> Vec<RemiseEnAttente>;
    caisse_session_courante(()) -> Option<SessionAffichee>;
    caisse_ouvrir(DemandeOuverture) -> SessionAffichee;
    caisse_cloturer(DemandeCloture) -> SessionAffichee;
    caisse_rapprocher(DemandeVisa) -> SessionAffichee;
    caisse_sessions_lister(()) -> Vec<SessionAffichee>;
    etat_caisse_produire(DemandeSession) -> DocumentAffiche;
    encaissement_preparer(DemandePreparation) -> PreparationEncaissement;
    encaisser(DemandeEncaissement) -> RecuEmis;
    encaisser_famille(DemandeFamille) -> Vec<RecuEmis>;
    recu_produire(DemandeVersement) -> DocumentAffiche;
    versement_annuler(DemandeAnnulationVersement) -> AnnulationEmise;
    releve_compte_produire(DemandeEleve) -> DocumentAffiche;
    impayes_lister(()) -> ListeImpayes;
    pedagogie_lire(DemandeIdAnnee) -> Pedagogie;
    matiere_enregistrer(DemandeMatiere) -> gesco_core::pedagogie::Matiere;
    programme_enregistrer(DemandeProgramme) -> Vec<gesco_core::pedagogie::LigneProgramme>;
    programme_reprendre(DemandeIdAnnee) -> u32;
    periodes_definir(DemandeNombrePeriodes) -> Vec<gesco_core::pedagogie::Periode>;
    calendrier_modifier(DemandeCalendrier) -> Vec<gesco_core::pedagogie::Periode>;
    regle_enregistrer(DemandeRegle) -> gesco_core::pedagogie::RegleEvaluation;
    enseignants_lister(()) -> Vec<FicheEnseignant>;
    enseignant_enregistrer(DemandeEnseignant) -> FicheEnseignant;
    affectations_lister(DemandeIdAnnee) -> Vec<AffectationAffichee>;
    affectation_ajouter(DemandeAffectation) -> gesco_core::pedagogie::AffectationEnseignement;
    affectation_confier(DemandeAffectation) -> gesco_core::pedagogie::AffectationEnseignement;
    affectation_retirer(DemandeIdAffectation) -> ();
    maitre_affecter(DemandeMaitre) -> u32;
    portee_notes_lire(()) -> gesco_core::pedagogie::PorteeNotes;
    evaluations_lister(DemandeEvaluations) -> Vec<EvaluationAffichee>;
    evaluation_creer(DemandeEvaluation) -> gesco_core::pedagogie::Evaluation;
    notes_grille(DemandeIdEvaluation) -> GrilleNotes;
    note_saisir(DemandeNote) -> Option<gesco_core::pedagogie::NoteEleve>;
    evaluation_valider(DemandeIdEvaluation) -> gesco_core::pedagogie::Evaluation;
    evaluation_deverrouiller(DemandeDeverrouillage) -> gesco_core::pedagogie::Evaluation;
    inscriptions_valider_classe(DemandeClasse) -> crate::inscriptions::BilanValidation;
    notes_import_analyser(DemandeImportNotes) -> ApercuImportNotes;
    notes_importer(DemandeImportNotes) -> ApercuImportNotes;
    portee_absences_lire(()) -> gesco_core::pedagogie::PorteeAbsences;
    absences_releve_lire(DemandeReleve) -> ReleveAbsences;
    absences_releve_enregistrer(DemandeEnregistrementReleve) -> BilanReleve;
    absences_justifier(DemandeJustification) -> u32;
    absences_periode_lire(DemandeAbsencesPeriode) -> AbsencesPeriode;
    bulletins_lire(DemandeBulletins) -> BulletinsClasse;
    resultats_calculer(DemandeBulletins) -> BulletinsClasse;
    appreciation_saisir(DemandeAppreciation) -> gesco_core::pedagogie::Bulletin;
    bulletins_valider(DemandeBulletins) -> u32;
    bulletins_publier(DemandeBulletins) -> DocumentAffiche;
    bulletin_corriger(DemandeCorrection) -> gesco_core::pedagogie::Bulletin;
    bulletins_produire(DemandeBulletins) -> DocumentAffiche;
    bulletin_produire(DemandeIdBulletin) -> DocumentAffiche;
    moyennes_reprises_lire(DemandeBulletins) -> ReprisesClasse;
    moyenne_reprise_saisir(DemandeReprise) -> Option<gesco_core::pedagogie::MoyenneReprise>;
    inscription_transferer(DemandeTransfert) -> TransfertEnregistre;
    tableau_de_bord_lire(()) -> TableauDeBord;
    certificat_transfert_produire(DemandeDepart) -> DocumentAffiche;
    releve_moyennes_produire(DemandeDepart) -> DocumentAffiche;
    inscription_exclure(DemandeExclusion) -> gesco_core::scolarite::Exclusion;
    exclusion_lever(DemandeLevee) -> gesco_core::scolarite::Exclusion;
    exclusions_lister(DemandeDepart) -> Vec<gesco_core::scolarite::Exclusion>;
    encaissements_lister(DemandeJournal) -> JournalEncaissements;
    licence_etat(()) -> EtatLicenceAffiche;
    licence_installer(DemandeLicence) -> EtatLicenceAffiche;
    personnel_referentiel_lire(()) -> ReferentielPersonnel;
    categorie_personnel_enregistrer(DemandeCategoriePersonnel) -> gesco_core::rh::CategoriePersonnel;
    fonction_personnel_enregistrer(DemandeFonctionPersonnel) -> gesco_core::rh::FonctionPersonnel;
    personnel_lister(()) -> Vec<MembreAffiche>;
    personnel_import_modele(DemandeModelePersonnel) -> FichierEcrit;
    personnel_import_analyser(DemandeImportPersonnel) -> AnalyseImportPersonnel;
    personnel_import_appliquer(DemandeImportPersonnel) -> BilanImportPersonnel;
    membre_lire(DemandeIdMembre) -> FicheMembre;
    ma_fiche_lire(()) -> Option<MaFiche>;
    membre_enregistrer(DemandeMembre) -> FicheMembre;
    membre_partir(DemandeDepartMembre) -> FicheMembre;
    membre_revenir(DemandeIdMembre) -> FicheMembre;
    membre_enseignant(DemandeMembreEnseignant) -> FicheMembre;
    piece_dossier_ajouter(DemandePieceDossier) -> gesco_core::rh::PieceDossier;
    piece_dossier_enregistrer_sous(DemandePieceSous) -> gesco_core::rh::PieceDossier;
    piece_dossier_retirer(DemandeIdPiece) -> gesco_core::rh::PieceDossier;
    engagements_lister(DemandeIdMembre) -> Vec<EngagementAffiche>;
    engagement_conclure(DemandeEngagement) -> gesco_core::rh::Engagement;
    engagement_terminer(DemandeClotureEngagement) -> gesco_core::rh::Engagement;
    engagement_rompre(DemandeClotureEngagement) -> gesco_core::rh::Engagement;
    engagement_renouvellement(DemandeIdEngagement) -> DemandeEngagementProposee;
    engagements_a_echeance(()) -> Vec<EngagementAEcheance>;
    lettre_engagement_produire(DemandeIdEngagement) -> DocumentAffiche;
    heures_lire(DemandeMois) -> EtatHeures;
    heures_saisir(DemandeSaisieHeures) -> EtatHeures;
    heures_viser(DemandeVisaHeures) -> EtatHeures;
    heures_deverrouiller(DemandeDeverrouillageHeures) -> EtatHeures;
    etat_heures_produire(DemandeMois) -> DocumentAffiche;
    absences_conges_lire(DemandeMoisConges) -> AbsencesEtConges;
    absence_personnel_noter(DemandeAbsencePersonnel) -> gesco_core::rh::AbsencePersonnel;
    absence_personnel_justifier(DemandeMotifAbsence) -> gesco_core::rh::AbsencePersonnel;
    absence_personnel_annuler(DemandeMotifAbsence) -> gesco_core::rh::AbsencePersonnel;
    conge_demander(DemandeConge) -> gesco_core::rh::Conge;
    conge_accorder(DemandeIdConge) -> gesco_core::rh::Conge;
    conge_refuser(DemandeMotifConge) -> gesco_core::rh::Conge;
    conge_annuler(DemandeMotifConge) -> gesco_core::rh::Conge;
    type_conge_enregistrer(DemandeTypeConge) -> gesco_core::rh::TypeConge;
    engagement_mois_payes(DemandeMoisPayes) -> gesco_core::rh::Engagement;
    elements_paie_lire(()) -> ElementsPaie;
    element_paie_enregistrer(DemandeElement) -> ElementsPaie;
    element_paie_version_enregistrer(DemandeVersion) -> ElementsPaie;
    attribution_enregistrer(DemandeAttribution) -> ElementsPaie;
    paie_reglages_enregistrer(ReglagesPaie) -> ElementsPaie;
    paie_mois_lire(DemandeMoisPaie) -> PaieDuMois;
    paie_calculer(DemandeMoisPaie) -> PaieDuMois;
    paie_saisir(DemandeSaisiePaie) -> PaieDuMois;
    paie_saisie_retirer(DemandeIdSaisie) -> PaieDuMois;
    paie_absence_confirmer(DemandeAbsencePaie) -> PaieDuMois;
    paie_valider(DemandeMoisPaie) -> PaieDuMois;
    paiements_personnel_lire(()) -> PaiementsPersonnel;
    salaire_payer(DemandePaiementSalaire) -> PaiementFait;
    paie_payer_reste(DemandeResteDuMois) -> PaiementFait;
    paiement_personnel_annuler(DemandeAnnulationPaiement) -> PaiementFait;
    avance_accorder(DemandeAvance) -> PaiementFait;
    avance_annuler(DemandeAnnulationAvance) -> PaiementFait;
    bulletins_paie_produire(DemandeMoisPaie) -> DocumentAffiche;
    bulletin_paie_produire(DemandeBulletinPaie) -> DocumentAffiche;
    etat_paie_produire(DemandeMoisPaie) -> DocumentAffiche;
    etat_retenues_produire(DemandeMoisPaie) -> DocumentAffiche;
    mes_bulletins_paie_lire(()) -> Vec<MonBulletinPaie>;
    mon_bulletin_paie_produire(DemandeBulletinPaie) -> DocumentAffiche;
    tresorerie_lire(()) -> Tresorerie;
    categorie_tresorerie_enregistrer(DemandeCategorieTresorerie) -> Tresorerie;
    compte_tresorerie_enregistrer(DemandeCompteTresorerie) -> Tresorerie;
    tresorerie_reglages_enregistrer(ReglagesTresorerie) -> Tresorerie;
    depenses_lire(DemandeMoisDepenses) -> Depenses;
    depense_saisir(DemandeDepense) -> Depenses;
    depense_decider(DemandeDecisionDepense) -> Depenses;
    depense_payer(DemandePayerDepense) -> Depenses;
    depense_annuler(DemandeAnnulationDepense) -> Depenses;
    depense_piece_joindre(DemandePieceDepense) -> Depenses;
    depense_piece_enregistrer_sous(DemandePieceDepenseSous) -> ();
    fournisseurs_lire(()) -> Fournisseurs;
    fournisseur_enregistrer(DemandeFournisseur) -> Fournisseurs;
    facture_fournisseur_enregistrer(DemandeFacture) -> Fournisseurs;
    facture_fournisseur_annuler(DemandeAnnulationFacture) -> Fournisseurs;
    comptes_soldes_lire(()) -> EtatComptes;
    livre_compte_lire(DemandeLivre) -> LivreCompte;
    transfert_faire(DemandeTransfertCompte) -> EtatComptes;
    ligne_tresorerie_pointer(DemandePointage) -> LivreCompte;
    compte_du_mode_choisir(DemandeCompteDuMode) -> EtatComptes;
    journal_tresorerie_lire(DemandeJournalTresorerie) -> JournalTresorerie;
    budget_lire(DemandeBudget) -> Budget;
    budget_prevoir(DemandeLigneBudget) -> Budget;
    emploi_du_temps_lire(()) -> EmploiDuTemps;
    journee_regler(gesco_core::pedagogie::emploi_du_temps::JourneeCycle) -> EmploiDuTemps;
    seance_placer(DemandeSeance) -> EmploiDuTemps;
    seance_retirer(DemandeRetraitSeance) -> EmploiDuTemps;
    emploi_classe_produire(DemandeEmploiClasse) -> DocumentAffiche;
    emploi_enseignant_produire(DemandeEmploiEnseignant) -> DocumentAffiche;
    cantine_lire(DemandeCantine) -> Cantine;
    repas_pointer(DemandeRepas) -> Cantine;
    liste_cantine_produire(DemandeCantine) -> DocumentAffiche;
    recettes_lire(DemandeMoisRecettes) -> RecettesDuMois;
    recette_saisir(DemandeRecette) -> RecettesDuMois;
    recette_annuler(DemandeAnnulationRecette) -> RecettesDuMois;
    discipline_registre_lire(()) -> RegistreDiscipline;
    discipline_dossier_lire(DemandeDossierDiscipline) -> DossierDiscipline;
    responsable_messages_lire(DemandeMessagesResponsable) -> MessagesResponsable;
    responsable_consentement_definir(DemandeConsentement) -> MessagesResponsable;
    numero_remettre_a_l_essai(DemandeRemiseALEssai) -> MessagesResponsable;
    sanction_noter(DemandeSanction) -> DossierDiscipline;
    sanction_annuler(DemandeAnnulationSanction) -> DossierDiscipline;
    journal_archive_verifier(DemandeVerificationArchive) -> VerificationArchive;
    absences_seances_lire(DemandeReleve) -> ReleveParSeance;
    absences_seances_enregistrer(DemandeReleveParSeance) -> ReleveParSeance;
    releve_apercu(DemandeApercuReleve) -> ApercuReleve;
    releve_importer(DemandeImportReleve) -> ResultatReleve;
}

impl Application {
    /// Démarre l'application : prépare l'espace et les clés, ouvre la base si l'enveloppe machine
    /// s'ouvre. Sinon l'application attend la phrase de secours.
    pub async fn demarrer(options: OptionsApplication) -> Resultat<Self> {
        let espace = EspaceDonnees::nouveau(options.racine);
        espace.preparer()?;
        let cles = EspaceCles::nouveau(espace.cles(), options.trousseau);
        let atelier = Arc::new(AtelierDocuments::nouveau(&espace)?);
        let application = Self {
            espace,
            cles,
            atelier,
            poste: options.poste,
            empreinte_machine: options.empreinte_machine,
            horloge: options.horloge,
            politique_session: options.politique_session,
            version_applicative: options.version_applicative,
            cle_licence: options.cle_licence,
            etat: RwLock::new(None),
            tentatives: Tentatives::default(),
            echecs: EffetsEchoues::default(),
            imports: Mutex::new(HashMap::new()),
        };
        if let EtatCles::Ouvertes(cle) = application.cles.preparer()? {
            let ouverte = application.ouvrir(cle).await?;
            *application.etat.write().await = Some(Arc::new(ouverte));
        }
        Ok(application)
    }

    /// Exécute une commande du frontend : garde, puis gestionnaire.
    pub async fn executer(
        &self,
        nom: &str,
        jeton: Option<Jeton>,
        entree: serde_json::Value,
    ) -> Result<serde_json::Value, ErreurCommande> {
        self.executer_garde(nom, jeton, entree)
            .await
            .map_err(ErreurCommande::from)
    }

    async fn executer_garde(
        &self,
        nom: &str,
        jeton: Option<Jeton>,
        entree: serde_json::Value,
    ) -> Resultat<serde_json::Value> {
        let commande = Commande::par_nom(nom).ok_or(ErreurService::Regle {
            cle: "commande.inconnue",
        })?;
        let ouverte = self.etat.read().await.clone();
        let session = match (&ouverte, commande.acces) {
            (_, Acces::Public) => None,
            (Some(o), _) => {
                Garde::nouvelle(&o.noyau, Arc::new(PorteeTotale))
                    .autoriser(commande, jeton.as_ref(), None)
                    .await?
            }
            (None, _) => {
                return Err(ErreurService::Regle {
                    cle: "application.cles_verrouillees",
                });
            }
        };
        self.aiguiller(
            nom,
            Appel {
                ouverte,
                session,
                jeton,
            },
            entree,
        )
        .await
    }

    /// Espace de données.
    pub fn espace(&self) -> &EspaceDonnees {
        &self.espace
    }

    async fn ouvrir(&self, cle: CleDonnees) -> Resultat<Ouverte> {
        let base = Base::ouvrir_sans_migrer(
            OptionsBase::fichier(self.espace.base()).avec_cle_hex(cle.en_hexadecimal().as_str()),
        )
        .await?;
        let (file, recepteur) = FileEffets::nouvelle();
        let noyau = Arc::new(Noyau::assembler(OptionsNoyau {
            base,
            poste: self.poste.clone(),
            empreinte_machine: self.empreinte_machine.clone(),
            horloge: Arc::clone(&self.horloge),
            effets: Arc::new(file),
            politique_session: self.politique_session,
        }));
        if noyau.base().migrations_en_attente().await? {
            // R-48 : une sauvegarde précède toute migration de schéma.
            for fiche in Self::installes(&noyau).await? {
                ServiceSauvegarde::nouveau(
                    &noyau,
                    &self.espace,
                    &self.cles,
                    &cle,
                    &self.version_applicative,
                )
                .creer(fiche.id, None, MotifSauvegarde::AvantMigration)
                .await?;
            }
        }
        noyau.base().migrer().await?;
        noyau.base().verifier_integrite().await?;
        for fiche in Self::installes(&noyau).await? {
            crate::mise_a_niveau::completer_roles(&noyau, fiche.id).await?;
            crate::mise_a_niveau::completer_parametres(&noyau, fiche.id).await?;
            crate::mise_a_niveau::completer_referentiel(&noyau, fiche.id).await?;
            crate::mise_a_niveau::completer_pedagogie(&noyau, fiche.id).await?;
            crate::mise_a_niveau::completer_personnel(&noyau, fiche.id).await?;
        }
        for fiche in Self::installes(&noyau).await? {
            // §5.2 : les cent dernières lignes de la chaîne sont vérifiées à chaque démarrage.
            let contexte = noyau.contexte(fiche.id, None);
            let mut cnx = noyau
                .base()
                .pool()
                .acquire()
                .await
                .map_err(gesco_db::ErreurBase::from)?;
            if audit::verifier_chaine(&mut cnx, Some(100), &contexte)
                .await
                .is_err()
                && let Ok(mut echecs) = self.echecs.lock()
            {
                echecs.push(crate::documents::EffetEchoue {
                    cle: "avertissement.chaine_audit_rompue".to_owned(),
                    entite_id: fiche.id.vers_chaine(),
                });
            }
        }
        let tache_effets = tokio::spawn(traiter_effets(
            Arc::clone(&noyau),
            Arc::clone(&self.atelier),
            Arc::clone(&self.echecs),
            recepteur,
        ));
        Ok(Ouverte {
            noyau,
            cle,
            tache_effets,
        })
    }

    async fn fermer_base(&self) {
        if let Some(ouverte) = self.etat.write().await.take() {
            ouverte.tache_effets.abort();
            ouverte.noyau.base().clone().fermer().await;
        }
    }

    async fn installes(noyau: &Noyau) -> Resultat<Vec<Etablissement>> {
        let mut cnx = noyau
            .base()
            .pool()
            .acquire()
            .await
            .map_err(gesco_db::ErreurBase::from)?;
        Ok(etablissement::installes(&mut cnx).await?)
    }

    async fn installe(noyau: &Noyau) -> Resultat<Option<Etablissement>> {
        Ok(Self::installes(noyau).await?.into_iter().next())
    }

    fn sauvegardes<'a>(&'a self, ouverte: &'a Ouverte) -> ServiceSauvegarde<'a> {
        ServiceSauvegarde::nouveau(
            &ouverte.noyau,
            &self.espace,
            &self.cles,
            &ouverte.cle,
            &self.version_applicative,
        )
    }

    /// Fermeture de l'application : sauvegarde (§11.1), puis fermeture de la base.
    pub async fn fermer(&self) -> Resultat<()> {
        let ouverte = self.etat.read().await.clone();
        if let Some(o) = ouverte
            && let Some(fiche) = Self::installe(&o.noyau).await?
            && self.cles.enveloppe_secours()?.is_some()
        {
            self.sauvegardes(&o)
                .creer(fiche.id, None, MotifSauvegarde::Fermeture)
                .await?;
        }
        self.fermer_base().await;
        Ok(())
    }

    /// Tâches périodiques, appelées par le binaire chaque minute : sauvegarde à l'intervalle
    /// configuré (§11.1) et test de restauration mensuel (§11.5). Un échec n'arrête rien : il
    /// rejoint les avertissements affichés dans le bandeau.
    pub async fn taches_periodiques(&self) {
        if let Err(erreur) = self.executer_taches_periodiques().await
            && let Ok(mut echecs) = self.echecs.lock()
        {
            echecs.push(crate::documents::EffetEchoue {
                cle: erreur.cle_traduction().to_owned(),
                entite_id: "taches_periodiques".to_owned(),
            });
        }
    }

    async fn executer_taches_periodiques(&self) -> Resultat<()> {
        let Some(o) = self.etat.read().await.clone() else {
            return Ok(());
        };
        let Some(fiche) = Self::installe(&o.noyau).await? else {
            return Ok(());
        };
        if self.cles.enveloppe_secours()?.is_none() {
            return Ok(());
        }
        let service = self.sauvegardes(&o);
        let intervalle_min = {
            let contexte = o.noyau.contexte(fiche.id, None);
            let mut cnx = o
                .noyau
                .base()
                .pool()
                .acquire()
                .await
                .map_err(gesco_db::ErreurBase::from)?;
            parametre::entier(&mut cnx, "sauvegarde.intervalle_min", 60, &contexte).await?
        };
        let maintenant = o.noyau.horloge().maintenant();
        let derniere = service.lister(fiche.id, 1).await?.into_iter().next();
        let due = derniere.is_none_or(|s| {
            s.horodatage
                .difference_millis(&maintenant)
                .is_ok_and(|d| d >= intervalle_min.saturating_mul(60_000))
        });
        if due {
            service
                .creer(fiche.id, None, MotifSauvegarde::Planifiee)
                .await?;
        }
        if service.test_du(fiche.id).await? {
            service.tester(fiche.id, None).await?;
        }
        Ok(())
    }

    async fn etat_courant(&self) -> Resultat<EtatDemarrage> {
        let ouverte = self.etat.read().await.clone();
        let (phase, etablissement) = match ouverte {
            None => (PhaseDemarrage::ClesVerrouillees, None),
            Some(o) => match Self::installe(&o.noyau).await? {
                None => (PhaseDemarrage::PremierDemarrage, None),
                Some(e) => (
                    PhaseDemarrage::Pret,
                    Some(ResumeEtablissement {
                        id: e.id,
                        nom: e.nom,
                    }),
                ),
            },
        };
        Ok(EtatDemarrage {
            phase,
            etablissement,
            version_applicative: self.version_applicative.clone(),
        })
    }

    async fn phrase_et_certificat(
        &self,
        ouverte: &Ouverte,
        etablissement_id: Id<Etablissement>,
        emis_par: &str,
    ) -> Resultat<(PhraseAffichee, String, i64)> {
        let (phrase, enveloppe): (PhraseSecours, _) = self.cles.regenerer_secours(&ouverte.cle)?;
        let certificat = ServiceDocuments::nouveau(&ouverte.noyau, &self.atelier)
            .composer_certificat(
                etablissement_id,
                &enveloppe.id,
                &phrase,
                emis_par,
                ouverte.noyau.horloge().maintenant(),
            )
            .await?;
        Ok((
            PhraseAffichee {
                enveloppe_id: enveloppe.id.clone(),
                mots: phrase.mots().iter().map(|m| (*m).to_owned()).collect(),
                certificat_pdf: base64(&certificat.pdf),
            },
            certificat.modele_id,
            i64::from(certificat.modele_version),
        ))
    }

    /// Restaure une archive : base fermée, restauration, réouverture, trace dans la base
    /// restaurée. Les sessions ouvertes disparaissent avec l'ancienne base.
    async fn restaurer_et_rouvrir(
        &self,
        chemin: &Path,
        phrase: Option<&str>,
        auteur: Option<&Session>,
    ) -> Resultat<RapportRestauration> {
        let cle_locale = self.etat.read().await.as_ref().map(|o| o.cle.clone());
        self.fermer_base().await;
        let issue = restauration::restaurer(
            &self.espace,
            &self.cles,
            cle_locale.as_ref(),
            &self.tentatives,
            self.horloge.as_ref(),
            chemin,
            phrase,
        )
        .await;
        let (rapport, cle) = match issue {
            Ok(r) => r,
            Err(e) => {
                // La base d'origine est intacte : on la rouvre telle quelle.
                if let Some(cle) = cle_locale {
                    let ouverte = self.ouvrir(cle).await?;
                    *self.etat.write().await = Some(Arc::new(ouverte));
                }
                return Err(e);
            }
        };
        let ouverte = Arc::new(self.ouvrir(cle).await?);
        *self.etat.write().await = Some(Arc::clone(&ouverte));
        let contexte = ouverte
            .noyau
            .contexte(rapport.apercu.etablissement_id, auteur);
        let mut unite = ouverte
            .noyau
            .commencer(contexte, Durabilite::Critique)
            .await?;
        let id = rapport.apercu.etablissement_id.vers_chaine();
        unite.auditer(
            "sauvegarde",
            &id,
            Operation::Creation,
            None,
            Some(serde_json::to_value(&rapport)?),
            "sauvegarde.restaurer",
        );
        unite.emettre(
            TypeEvenement::SauvegardeRestauree,
            "etablissement",
            &id,
            serde_json::json!({
                "horodatage_sauvegarde": rapport.apercu.horodatage,
                "version_schema": rapport.version_schema,
                "par_phrase": rapport.par_phrase,
            }),
        );
        ouverte.noyau.terminer(unite).await?;
        Ok(rapport)
    }

    // --- Gestionnaires des commandes publiques ---------------------------------------------

    async fn etat_demarrage(&self, _appel: Appel, _entree: ()) -> Resultat<EtatDemarrage> {
        self.etat_courant().await
    }

    async fn premier_demarrage(
        &self,
        appel: Appel,
        entree: DemandePremierDemarrage,
    ) -> Resultat<PremierDemarrageEffectue> {
        let ouverte = appel.ouverte()?;
        if Self::installe(&ouverte.noyau).await?.is_some() {
            return Err(ErreurService::Regle {
                cle: "premier_demarrage.deja_effectue",
            });
        }
        let fiche = Etablissement {
            id: nouvel_id(),
            organisation_id: nouvel_id(),
            nom: entree.etablissement_nom.trim().to_owned(),
            statut: entree.statut,
            code_academie: entree.code_academie,
            code_cap: entree.code_cap,
            adresse: entree.adresse,
            telephones: entree.telephones,
            mentions_legales: String::new(),
            multiple_arrondi: 5,
        };
        if fiche.nom.is_empty() {
            return Err(ErreurService::Regle {
                cle: "premier_demarrage.nom_obligatoire",
            });
        }
        let resume = ResumeEtablissement {
            id: fiche.id,
            nom: fiche.nom.clone(),
        };
        let initialise = ServiceInitialisation::nouveau(&ouverte.noyau)
            .executer(PremierDemarrage {
                organisation: &entree.organisation,
                etablissement: fiche,
                administrateur_identifiant: &entree.administrateur_identifiant,
                administrateur_nom: &entree.administrateur_nom,
                administrateur_mot_de_passe: MotDePasse(&entree.administrateur_mot_de_passe),
                cycles: &entree.cycles,
            })
            .await?;
        let (phrase, modele_id, modele_version) = self
            .phrase_et_certificat(
                ouverte,
                initialise.etablissement_id,
                &entree.administrateur_nom,
            )
            .await?;
        let contexte = ouverte.noyau.contexte(initialise.etablissement_id, None);
        let mut unite = ouverte
            .noyau
            .commencer(contexte, Durabilite::Critique)
            .await?;
        unite.auditer(
            "enveloppe_secours",
            &phrase.enveloppe_id,
            Operation::Creation,
            None,
            Some(serde_json::json!({
                "certificat": { "modele_id": modele_id, "modele_version": modele_version },
            })),
            "premier_demarrage",
        );
        ouverte.noyau.terminer(unite).await?;
        Ok(PremierDemarrageEffectue {
            etablissement: resume,
            phrase,
        })
    }

    async fn connexion(&self, appel: Appel, entree: DemandeConnexion) -> Resultat<Session> {
        let ouverte = appel.ouverte()?;
        let fiche = Self::installe(&ouverte.noyau)
            .await?
            .ok_or(ErreurService::Regle {
                cle: "premier_demarrage.requis",
            })?;
        ServiceAuthentification::nouveau(&ouverte.noyau)
            .connecter(
                fiche.id,
                &entree.identifiant,
                MotDePasse(&entree.mot_de_passe),
            )
            .await
    }

    async fn deverrouiller_pin(&self, appel: Appel, entree: DemandePin) -> Resultat<Session> {
        ServiceAuthentification::nouveau(&appel.ouverte()?.noyau)
            .deverrouiller_par_pin(&entree.jeton, Pin(&entree.pin))
            .await
    }

    async fn deverrouiller_mot_de_passe(
        &self,
        appel: Appel,
        entree: DemandeDeverrouillageMotDePasse,
    ) -> Resultat<Session> {
        ServiceAuthentification::nouveau(&appel.ouverte()?.noyau)
            .deverrouiller_par_mot_de_passe(&entree.jeton, MotDePasse(&entree.mot_de_passe))
            .await
    }

    async fn sessions_du_poste(&self, appel: Appel, _entree: ()) -> Resultat<Vec<Session>> {
        Ok(appel
            .ouverte
            .as_ref()
            .map(|o| o.noyau.sessions().lister())
            .unwrap_or_default())
    }

    async fn cles_deverrouiller_phrase(
        &self,
        appel: Appel,
        entree: DemandePhrase,
    ) -> Resultat<EtatDemarrage> {
        if appel.ouverte.is_some() {
            return self.etat_courant().await;
        }
        let maintenant = self.horloge.maintenant();
        self.tentatives_verifier(maintenant)?;
        let issue = PhraseSecours::analyser(&entree.phrase)
            .map_err(ErreurService::from)
            .and_then(|p| self.cles.ouvrir_par_phrase(&p));
        let cle = self.tentatives_conclure(issue, maintenant)?;
        let ouverte = self.ouvrir(cle).await?;
        *self.etat.write().await = Some(Arc::new(ouverte));
        self.etat_courant().await
    }

    fn tentatives_verifier(&self, maintenant: gesco_core::commun::Horodatage) -> Resultat<()> {
        self.tentatives.verifier_public(maintenant)
    }

    fn tentatives_conclure<T>(
        &self,
        issue: Resultat<T>,
        maintenant: gesco_core::commun::Horodatage,
    ) -> Resultat<T> {
        self.tentatives.conclure(issue, maintenant)
    }

    async fn sauvegarde_apercu(
        &self,
        _appel: Appel,
        entree: DemandeArchive,
    ) -> Resultat<ApercuSauvegarde> {
        restauration::apercu(Path::new(&entree.chemin))
    }

    async fn sauvegarde_restaurer_initiale(
        &self,
        appel: Appel,
        entree: DemandeRestauration,
    ) -> Resultat<RapportRestauration> {
        if let Some(o) = &appel.ouverte
            && Self::installe(&o.noyau).await?.is_some()
        {
            return Err(ErreurService::Regle {
                cle: "sauvegarde.restauration_initiale_refusee",
            });
        }
        self.restaurer_et_rouvrir(Path::new(&entree.chemin), entree.phrase.as_deref(), None)
            .await
    }

    // --- Gestionnaires des commandes connectées --------------------------------------------

    async fn deconnexion(&self, appel: Appel, _entree: ()) -> Resultat<()> {
        ServiceAuthentification::nouveau(&appel.ouverte()?.noyau)
            .deconnecter(appel.jeton()?)
            .await
    }

    async fn verrouiller(&self, appel: Appel, _entree: ()) -> Resultat<Session> {
        ServiceAuthentification::nouveau(&appel.ouverte()?.noyau).verrouiller(appel.jeton()?)
    }

    async fn definir_pin(&self, appel: Appel, entree: DemandeDefinirPin) -> Resultat<()> {
        ServiceAuthentification::nouveau(&appel.ouverte()?.noyau)
            .definir_pin(
                appel.session()?,
                MotDePasse(&entree.mot_de_passe),
                Pin(&entree.pin),
            )
            .await
    }

    async fn changer_mot_de_passe(
        &self,
        appel: Appel,
        entree: DemandeChangementMotDePasse,
    ) -> Resultat<()> {
        ServiceAuthentification::nouveau(&appel.ouverte()?.noyau)
            .changer_mot_de_passe(
                appel.session()?,
                MotDePasse(&entree.ancien),
                MotDePasse(&entree.nouveau),
            )
            .await
    }

    async fn session_courante(&self, appel: Appel, _entree: ()) -> Resultat<SessionCourante> {
        let ouverte = appel.ouverte()?;
        let session = appel.session()?.clone();
        let contexte = ouverte
            .noyau
            .contexte(session.etablissement_id, Some(&session));
        let mut cnx = ouverte
            .noyau
            .base()
            .pool()
            .acquire()
            .await
            .map_err(gesco_db::ErreurBase::from)?;
        let mut permissions =
            role::permissions_de(&mut cnx, session.utilisateur_id, &contexte).await?;
        permissions.sort_by_key(Permission::code);
        permissions.dedup();
        let compte = utilisateur::lire(&mut cnx, session.utilisateur_id, &contexte).await?;
        let pin_defini = utilisateur::lire_secrets(&mut cnx, session.utilisateur_id, &contexte)
            .await?
            .empreinte_pin
            .is_some();
        let annee_courante = annee_scolaire::courante(&mut cnx, &contexte)
            .await?
            .map(|a| a.libelle);
        Ok(SessionCourante {
            session,
            permissions,
            doit_changer_mot_de_passe: compte.doit_changer_mot_de_passe,
            pin_defini,
            annee_courante,
            verrouillage_inactivite_ms: ouverte.noyau.politique_session().verrouillage_apres_millis,
            aujourd_hui: gesco_core::commun::DateCivile::du_jour(
                ouverte.noyau.horloge().maintenant(),
            ),
        })
    }

    async fn avertissements(&self, appel: Appel, _entree: ()) -> Resultat<Vec<Avertissement>> {
        let ouverte = appel.ouverte()?;
        let session = appel.session()?;
        let mut liste = self
            .sauvegardes(ouverte)
            .avertissements(session.etablissement_id)
            .await?;
        if let Some(cle) = ServiceLicence::nouveau(&ouverte.noyau, self.cle_licence)
            .avertissement(session.etablissement_id)
            .await?
        {
            liste.push(Avertissement {
                cle: cle.to_owned(),
            });
        }
        if ServiceAnnees::nouveau(&ouverte.noyau)
            .aucune_annee_active(session)
            .await?
        {
            liste.push(Avertissement {
                cle: "avertissement.annee_absente".to_owned(),
            });
        }
        if ServiceFacturation::nouveau(&ouverte.noyau)
            .rattrapage_en_attente(session)
            .await?
        {
            liste.push(Avertissement {
                cle: "avertissement.inscriptions_sans_facture".to_owned(),
            });
        }
        // La paie du mois écoulé, pas validée après le 5, chez qui fait la paie (SPEC_PHASE_7 §10).
        let paie = {
            let contexte = ouverte
                .noyau
                .contexte(session.etablissement_id, Some(session));
            let mut cnx = ouverte
                .noyau
                .base()
                .pool()
                .acquire()
                .await
                .map_err(gesco_db::ErreurBase::from)?;
            let permissions =
                role::permissions_de(&mut cnx, session.utilisateur_id, &contexte).await?;
            permissions.iter().any(|p| p.domaine() == "payroll")
        };
        if paie
            && ServicePaieMois::nouveau(&ouverte.noyau)
                .bloc(session)
                .await?
                .a_valider
        {
            liste.push(Avertissement {
                cle: "avertissement.paie_a_valider".to_owned(),
            });
        }
        if ServiceCaisse::nouveau(&ouverte.noyau)
            .regularisations_a_viser(session)
            .await?
        {
            liste.push(Avertissement {
                cle: "avertissement.regularisations_a_viser".to_owned(),
            });
        }
        // La trésorerie (SPEC_PHASE_8) : des dépenses à accorder chez qui accorde, des factures
        // échues chez qui saisit ou paie les dépenses, un budget dépassé chez qui lit la trésorerie.
        let droits = {
            let contexte = ouverte
                .noyau
                .contexte(session.etablissement_id, Some(session));
            let mut cnx = ouverte
                .noyau
                .base()
                .pool()
                .acquire()
                .await
                .map_err(gesco_db::ErreurBase::from)?;
            role::permissions_de(&mut cnx, session.utilisateur_id, &contexte).await?
        };
        let a = |p: Permission| droits.contains(&p);
        let saisit = a(Permission::EXPENSES_CREATE) || a(Permission::EXPENSES_PAY);
        if a(Permission::TREASURY_READ) || saisit || a(Permission::EXPENSES_APPROVE) {
            let bloc = crate::livres::bloc(&ouverte.noyau, session).await?;
            for (vrai, cle) in [
                (
                    a(Permission::EXPENSES_APPROVE) && bloc.a_approuver > 0,
                    "avertissement.depenses_a_approuver",
                ),
                (
                    saisit && bloc.factures_echues > 0,
                    "avertissement.factures_fournisseurs_echues",
                ),
                (
                    a(Permission::TREASURY_READ) && bloc.budget_depasse > 0,
                    "avertissement.budget_depasse",
                ),
            ] {
                if vrai {
                    liste.push(Avertissement {
                        cle: cle.to_owned(),
                    });
                }
            }
        }
        if let Ok(echecs) = self.echecs.lock() {
            liste.extend(echecs.iter().map(|e| Avertissement { cle: e.cle.clone() }));
        }
        liste.dedup();
        Ok(liste)
    }

    // --- Gestionnaires des commandes à permission ------------------------------------------

    async fn utilisateurs_lister(
        &self,
        appel: Appel,
        _entree: (),
    ) -> Resultat<Vec<FicheUtilisateur>> {
        ServiceAdministration::nouveau(&appel.ouverte()?.noyau)
            .utilisateurs(appel.session()?)
            .await
    }

    async fn utilisateur_creer(
        &self,
        appel: Appel,
        entree: DemandeCreationUtilisateur,
    ) -> Resultat<Utilisateur> {
        let ouverte = appel.ouverte()?;
        let session = appel.session()?;
        let contexte = ouverte
            .noyau
            .contexte(session.etablissement_id, Some(session));
        let roles: Vec<&str> = entree.roles.iter().map(String::as_str).collect();
        ServiceAuthentification::nouveau(&ouverte.noyau)
            .creer_utilisateur(
                &contexte,
                NouvelUtilisateur {
                    identifiant: &entree.identifiant,
                    nom_affiche: &entree.nom_affiche,
                    mot_de_passe_initial: MotDePasse(&entree.mot_de_passe_initial),
                    roles: &roles,
                },
            )
            .await
    }

    async fn utilisateur_modifier(
        &self,
        appel: Appel,
        entree: DemandeModificationUtilisateur,
    ) -> Resultat<Utilisateur> {
        ServiceAdministration::nouveau(&appel.ouverte()?.noyau)
            .modifier_utilisateur(
                appel.session()?,
                entree.id,
                &entree.nom_affiche,
                entree.actif,
            )
            .await
    }

    async fn utilisateur_reinitialiser_mot_de_passe(
        &self,
        appel: Appel,
        entree: DemandeReinitialisation,
    ) -> Resultat<Utilisateur> {
        ServiceComptes::nouveau(&appel.ouverte()?.noyau)
            .reinitialiser_mot_de_passe(
                appel.session()?,
                entree.id,
                MotDePasse(&entree.mot_de_passe_initial),
            )
            .await
    }

    async fn utilisateur_deverrouiller(
        &self,
        appel: Appel,
        entree: DemandeUtilisateur,
    ) -> Resultat<Utilisateur> {
        ServiceComptes::nouveau(&appel.ouverte()?.noyau)
            .deverrouiller(appel.session()?, entree.id)
            .await
    }

    async fn roles_lister(&self, appel: Appel, _entree: ()) -> Resultat<Vec<FicheRole>> {
        ServiceAdministration::nouveau(&appel.ouverte()?.noyau)
            .roles(appel.session()?)
            .await
    }

    async fn role_attribuer(&self, appel: Appel, entree: DemandeAttributionRole) -> Resultat<()> {
        ServiceAdministration::nouveau(&appel.ouverte()?.noyau)
            .attribuer_role(
                appel.session()?,
                entree.utilisateur_id,
                &entree.role,
                entree.attribue,
            )
            .await
    }

    async fn permissions_catalogue(&self, _appel: Appel, _entree: ()) -> Resultat<Vec<Permission>> {
        Ok(ServiceRoles::catalogue())
    }

    async fn role_creer(&self, appel: Appel, entree: DemandeCreationRole) -> Resultat<FicheRole> {
        ServiceRoles::nouveau(&appel.ouverte()?.noyau)
            .creer(appel.session()?, &entree.libelle, &entree.permissions)
            .await
    }

    async fn role_modifier(
        &self,
        appel: Appel,
        entree: DemandeModificationRole,
    ) -> Resultat<FicheRole> {
        ServiceRoles::nouveau(&appel.ouverte()?.noyau)
            .modifier(
                appel.session()?,
                &entree.code,
                &entree.libelle,
                &entree.permissions,
            )
            .await
    }

    async fn role_retablir(&self, appel: Appel, entree: DemandeRole) -> Resultat<FicheRole> {
        ServiceRoles::nouveau(&appel.ouverte()?.noyau)
            .retablir(appel.session()?, &entree.code)
            .await
    }

    async fn role_supprimer(&self, appel: Appel, entree: DemandeRole) -> Resultat<()> {
        ServiceRoles::nouveau(&appel.ouverte()?.noyau)
            .supprimer(appel.session()?, &entree.code)
            .await
    }

    async fn annees_lister(&self, appel: Appel, _entree: ()) -> Resultat<Vec<AnneeScolaire>> {
        ServiceAnnees::nouveau(&appel.ouverte()?.noyau)
            .lister(appel.session()?)
            .await
    }

    async fn annee_creer(&self, appel: Appel, entree: DemandeAnnee) -> Resultat<AnneeScolaire> {
        ServiceAnnees::nouveau(&appel.ouverte()?.noyau)
            .creer(appel.session()?, &entree.libelle, entree.debut, entree.fin)
            .await
    }

    async fn annee_modifier(
        &self,
        appel: Appel,
        entree: DemandeModificationAnnee,
    ) -> Resultat<AnneeScolaire> {
        ServiceAnnees::nouveau(&appel.ouverte()?.noyau)
            .modifier(
                appel.session()?,
                entree.id,
                &entree.libelle,
                entree.debut,
                entree.fin,
            )
            .await
    }

    async fn annee_activer(&self, appel: Appel, entree: DemandeIdAnnee) -> Resultat<AnneeScolaire> {
        let ouverte = appel.ouverte()?;
        let session = appel.session()?;
        let service = ServiceAnnees::nouveau(&ouverte.noyau);
        service.verifier_activation(session, entree.id).await?;
        // Ouvrir une année engage toute l'organisation de l'école : une sauvegarde la précède,
        // comme pour les opérations à risque du §11.1 de la spécification 0.5.
        self.sauvegardes(ouverte)
            .creer(
                session.etablissement_id,
                Some(session),
                MotifSauvegarde::AvantOuvertureAnnee,
            )
            .await?;
        service.activer(session, entree.id).await
    }

    async fn fin_annee_lire(&self, appel: Appel, _entree: ()) -> Resultat<FinAnnee> {
        ServiceFinAnnee::nouveau(&appel.ouverte()?.noyau)
            .lire(appel.session()?)
            .await
    }

    async fn decisions_classe_lire(
        &self,
        appel: Appel,
        entree: DemandeClasseFinAnnee,
    ) -> Resultat<DecisionsClasse> {
        ServiceFinAnnee::nouveau(&appel.ouverte()?.noyau)
            .decisions(appel.session()?, entree.classe_id)
            .await
    }

    async fn decision_poser(
        &self,
        appel: Appel,
        entree: DemandeDecision,
    ) -> Resultat<DecisionsClasse> {
        ServiceFinAnnee::nouveau(&appel.ouverte()?.noyau)
            .poser(appel.session()?, &entree)
            .await
    }

    async fn decisions_valider_classe(
        &self,
        appel: Appel,
        entree: DemandeClasseFinAnnee,
    ) -> Resultat<DecisionsClasse> {
        ServiceFinAnnee::nouveau(&appel.ouverte()?.noyau)
            .valider_propositions(appel.session()?, entree.classe_id)
            .await
    }

    async fn passage_regler(&self, appel: Appel, entree: DemandePassage) -> Resultat<FinAnnee> {
        ServiceFinAnnee::nouveau(&appel.ouverte()?.noyau)
            .regler_passage(appel.session()?, &entree)
            .await
    }

    async fn annee_cloturer(
        &self,
        appel: Appel,
        entree: DemandeIdAnnee,
    ) -> Resultat<AnneeScolaire> {
        let ouverte = appel.ouverte()?;
        let session = appel.session()?;
        let service = ServiceFinAnnee::nouveau(&ouverte.noyau);
        service.verifier_cloture(session, entree.id).await?;
        // La clôture fige toute l'année : une sauvegarde la précède, comme son ouverture.
        self.sauvegardes(ouverte)
            .creer(
                session.etablissement_id,
                Some(session),
                MotifSauvegarde::AvantClotureAnnee,
            )
            .await?;
        let annee = service.cloturer(session, entree.id).await?;
        // Le journal de l'année s'archive, signé, avec la sauvegarde de clôture (§12.4, T-73) :
        // dans le dossier du poste et sur les destinations. Un échec n'annule pas la clôture ; il
        // s'affiche en avertissement.
        let dossiers: Vec<PathBuf> = self
            .sauvegardes(ouverte)
            .destinations(session.etablissement_id)
            .await?
            .into_iter()
            .map(|d| PathBuf::from(d.chemin))
            .collect();
        if crate::archive_journal::archiver(
            &ouverte.noyau,
            session,
            &ouverte.cle,
            &annee,
            &dossiers,
        )
        .await
        .is_err()
            && let Ok(mut echecs) = self.echecs.lock()
        {
            echecs.push(crate::documents::EffetEchoue {
                cle: "avertissement.archive_journal_echouee".to_owned(),
                entite_id: annee.id.vers_chaine(),
            });
        }
        Ok(annee)
    }

    async fn annee_rouvrir(
        &self,
        appel: Appel,
        entree: DemandeReouverture,
    ) -> Resultat<AnneeScolaire> {
        ServiceFinAnnee::nouveau(&appel.ouverte()?.noyau)
            .rouvrir(appel.session()?, &entree)
            .await
    }

    async fn classes_reprendre(&self, appel: Appel, entree: DemandeIdAnnee) -> Resultat<FinAnnee> {
        ServiceFinAnnee::nouveau(&appel.ouverte()?.noyau)
            .reprendre_classes(appel.session()?, entree.id)
            .await
    }

    async fn reinscriptions_preparer(
        &self,
        appel: Appel,
        entree: DemandeIdAnnee,
    ) -> Resultat<BilanReinscriptions> {
        ServiceFinAnnee::nouveau(&appel.ouverte()?.noyau)
            .preparer_reinscriptions(appel.session()?, entree.id)
            .await
    }

    async fn annee_supprimer(&self, appel: Appel, entree: DemandeIdAnnee) -> Resultat<()> {
        ServiceAnnees::nouveau(&appel.ouverte()?.noyau)
            .supprimer(appel.session()?, entree.id)
            .await
    }

    async fn niveaux_lister(&self, appel: Appel, _entree: ()) -> Resultat<Referentiel> {
        ServiceReferentiel::nouveau(&appel.ouverte()?.noyau)
            .referentiel(appel.session()?)
            .await
    }

    async fn cycles_definir(&self, appel: Appel, entree: DemandeCycles) -> Resultat<Referentiel> {
        ServiceReferentiel::nouveau(&appel.ouverte()?.noyau)
            .definir_cycles(appel.session()?, &entree.cycles)
            .await
    }

    async fn niveau_modifier(&self, appel: Appel, entree: DemandeNiveau) -> Resultat<Niveau> {
        ServiceReferentiel::nouveau(&appel.ouverte()?.noyau)
            .modifier_niveau(appel.session()?, entree.id, &entree.libelle, entree.actif)
            .await
    }

    async fn serie_modifier(&self, appel: Appel, entree: DemandeSerie) -> Resultat<Serie> {
        ServiceReferentiel::nouveau(&appel.ouverte()?.noyau)
            .modifier_serie(appel.session()?, entree.id, &entree.libelle, entree.actif)
            .await
    }

    async fn filiere_creer(&self, appel: Appel, entree: DemandeFiliere) -> Resultat<Referentiel> {
        ServiceReferentiel::nouveau(&appel.ouverte()?.noyau)
            .creer_filiere(appel.session()?, &entree)
            .await
    }

    async fn classes_lister(
        &self,
        appel: Appel,
        entree: DemandeClasses,
    ) -> Resultat<Vec<FicheClasse>> {
        ServiceReferentiel::nouveau(&appel.ouverte()?.noyau)
            .classes(appel.session()?, entree.annee_id)
            .await
    }

    async fn classe_creer(&self, appel: Appel, entree: DemandeCreationClasse) -> Resultat<Classe> {
        ServiceReferentiel::nouveau(&appel.ouverte()?.noyau)
            .creer_classe(
                appel.session()?,
                entree.annee_id,
                entree.niveau_id,
                entree.serie_id,
                &entree.libelle,
            )
            .await
    }

    async fn classe_modifier(
        &self,
        appel: Appel,
        entree: DemandeModificationClasse,
    ) -> Resultat<Classe> {
        ServiceReferentiel::nouveau(&appel.ouverte()?.noyau)
            .renommer_classe(appel.session()?, entree.id, &entree.libelle)
            .await
    }

    async fn classe_supprimer(&self, appel: Appel, entree: DemandeClasse) -> Resultat<()> {
        ServiceReferentiel::nouveau(&appel.ouverte()?.noyau)
            .supprimer_classe(appel.session()?, entree.id)
            .await
    }

    async fn scolarite_reglages(&self, appel: Appel, _entree: ()) -> Resultat<ReglagesScolarite> {
        ServiceEleves::nouveau(&appel.ouverte()?.noyau)
            .reglages(appel.session()?)
            .await
    }

    async fn eleves_rechercher(
        &self,
        appel: Appel,
        entree: DemandeRecherche,
    ) -> Resultat<Vec<EleveTrouve>> {
        ServiceEleves::nouveau(&appel.ouverte()?.noyau)
            .rechercher(appel.session()?, &entree.texte)
            .await
    }

    async fn eleve_lire(&self, appel: Appel, entree: DemandeEleve) -> Resultat<FicheEleve> {
        ServiceEleves::nouveau(&appel.ouverte()?.noyau)
            .fiche(appel.session()?, entree.id)
            .await
    }

    async fn eleve_creer(&self, appel: Appel, entree: SaisieEleve) -> Resultat<FicheEleve> {
        ServiceEleves::nouveau(&appel.ouverte()?.noyau)
            .creer(appel.session()?, &entree)
            .await
    }

    async fn eleve_modifier(
        &self,
        appel: Appel,
        entree: DemandeModificationEleve,
    ) -> Resultat<FicheEleve> {
        ServiceEleves::nouveau(&appel.ouverte()?.noyau)
            .modifier(appel.session()?, entree.id, &entree.saisie)
            .await
    }

    async fn eleve_photo_definir(
        &self,
        appel: Appel,
        entree: DemandePhoto,
    ) -> Resultat<FicheEleve> {
        ServiceEleves::nouveau(&appel.ouverte()?.noyau)
            .definir_photo(
                appel.session()?,
                entree.id,
                entree.contenu_base64.as_deref(),
            )
            .await
    }

    async fn eleve_doublons_probables(
        &self,
        appel: Appel,
        entree: DemandeDoublons,
    ) -> Resultat<Vec<EleveTrouve>> {
        ServiceEleves::nouveau(&appel.ouverte()?.noyau)
            .doublons_probables(appel.session()?, &entree)
            .await
    }

    async fn inscriptions_lister(
        &self,
        appel: Appel,
        entree: DemandeListeInscriptions,
    ) -> Resultat<Vec<LigneInscription>> {
        ServiceInscriptions::nouveau(&appel.ouverte()?.noyau)
            .lister(appel.session()?, entree.annee_id)
            .await
    }

    async fn inscription_creer(
        &self,
        appel: Appel,
        entree: DemandeInscription,
    ) -> Resultat<ResultatInscription> {
        ServiceInscriptions::nouveau(&appel.ouverte()?.noyau)
            .creer(appel.session()?, &entree)
            .await
    }

    async fn inscription_modifier(
        &self,
        appel: Appel,
        entree: DemandeModificationInscription,
    ) -> Resultat<ResultatInscription> {
        ServiceInscriptions::nouveau(&appel.ouverte()?.noyau)
            .modifier(appel.session()?, entree.id, &entree.champs)
            .await
    }

    async fn inscription_valider(
        &self,
        appel: Appel,
        entree: DemandeValidation,
    ) -> Resultat<ResultatInscription> {
        ServiceInscriptions::nouveau(&appel.ouverte()?.noyau)
            .valider(appel.session()?, entree.id, &entree.choix)
            .await
    }

    async fn inscription_abandonner(
        &self,
        appel: Appel,
        entree: DemandeAbandon,
    ) -> Resultat<ResultatInscription> {
        ServiceInscriptions::nouveau(&appel.ouverte()?.noyau)
            .abandonner(appel.session()?, entree.id, &entree.motif)
            .await
    }

    async fn inscription_changer_classe(
        &self,
        appel: Appel,
        entree: DemandeChangementClasse,
    ) -> Resultat<ResultatInscription> {
        ServiceInscriptions::nouveau(&appel.ouverte()?.noyau)
            .changer_classe(appel.session()?, entree.id, entree.classe_id, &entree.motif)
            .await
    }

    async fn inscription_fiche_produire(
        &self,
        appel: Appel,
        entree: DemandeIdInscription,
    ) -> Resultat<DocumentAffiche> {
        let ouverte = appel.ouverte()?;
        let session = appel.session()?;
        let fiche = ServiceInscriptions::nouveau(&ouverte.noyau)
            .donnees_fiche(session, entree.id)
            .await?;
        let contexte = ouverte
            .noyau
            .contexte(session.etablissement_id, Some(session));
        let produit = ServiceDocuments::nouveau(&ouverte.noyau, &self.atelier)
            .produire_avec(
                contexte,
                "fiche_inscription",
                "inscription",
                &entree.id.vers_chaine(),
                fiche.donnees,
                fiche.pieces,
            )
            .await?;
        Ok(DocumentAffiche {
            pdf: base64(&produit.pdf),
            document: produit.document,
        })
    }

    async fn eleves_fusion_apercu(
        &self,
        appel: Appel,
        entree: DemandeFusion,
    ) -> Resultat<ApercuFusion> {
        ServiceFusion::nouveau(&appel.ouverte()?.noyau)
            .apercu(appel.session()?, &entree)
            .await
    }

    async fn eleves_fusionner(&self, appel: Appel, entree: DemandeFusion) -> Resultat<FicheEleve> {
        ServiceFusion::nouveau(&appel.ouverte()?.noyau)
            .fusionner(appel.session()?, &entree)
            .await
    }

    async fn responsables_rechercher(
        &self,
        appel: Appel,
        entree: DemandeRecherche,
    ) -> Resultat<Vec<ResponsableTrouve>> {
        ServiceEleves::nouveau(&appel.ouverte()?.noyau)
            .rechercher_responsables(appel.session()?, &entree.texte)
            .await
    }

    async fn responsable_rattacher(
        &self,
        appel: Appel,
        entree: DemandeRattachement,
    ) -> Resultat<FicheEleve> {
        ServiceEleves::nouveau(&appel.ouverte()?.noyau)
            .rattacher(
                appel.session()?,
                entree.eleve_id,
                entree.responsable_id,
                entree.saisie.as_ref(),
                entree.qualites,
            )
            .await
    }

    async fn responsable_modifier(
        &self,
        appel: Appel,
        entree: DemandeModificationResponsable,
    ) -> Resultat<FicheEleve> {
        ServiceEleves::nouveau(&appel.ouverte()?.noyau)
            .modifier_responsable(
                appel.session()?,
                entree.eleve_id,
                entree.responsable_id,
                &entree.saisie,
                entree.qualites,
            )
            .await
    }

    async fn responsable_detacher(
        &self,
        appel: Appel,
        entree: DemandeDetachement,
    ) -> Resultat<FicheEleve> {
        ServiceEleves::nouveau(&appel.ouverte()?.noyau)
            .detacher(appel.session()?, entree.eleve_id, entree.responsable_id)
            .await
    }

    async fn etablissement_lire(&self, appel: Appel, _entree: ()) -> Resultat<FicheEtablissement> {
        ServiceEtablissement::nouveau(&appel.ouverte()?.noyau)
            .fiche(appel.session()?)
            .await
    }

    async fn etablissement_modifier(
        &self,
        appel: Appel,
        entree: ChampsEtablissement,
    ) -> Resultat<FicheEtablissement> {
        ServiceEtablissement::nouveau(&appel.ouverte()?.noyau)
            .modifier(appel.session()?, entree)
            .await
    }

    async fn etablissement_logo_definir(
        &self,
        appel: Appel,
        entree: DemandeLogo,
    ) -> Resultat<FicheEtablissement> {
        ServiceEtablissement::nouveau(&appel.ouverte()?.noyau)
            .definir_logo(appel.session()?, entree.contenu_base64.as_deref())
            .await
    }

    async fn parametres_lister(&self, appel: Appel, _entree: ()) -> Resultat<Vec<Parametre>> {
        ServiceAdministration::nouveau(&appel.ouverte()?.noyau)
            .parametres(appel.session()?)
            .await
    }

    async fn parametre_definir(
        &self,
        appel: Appel,
        entree: DemandeParametre,
    ) -> Resultat<Parametre> {
        ServiceAdministration::nouveau(&appel.ouverte()?.noyau)
            .definir_parametre(appel.session()?, &entree.cle, entree.valeur)
            .await
    }

    async fn audit_lister(&self, appel: Appel, entree: RechercheJournal) -> Resultat<PageJournal> {
        ServiceAdministration::nouveau(&appel.ouverte()?.noyau)
            .journal(appel.session()?, &entree)
            .await
    }

    async fn audit_verifier(&self, appel: Appel, _entree: ()) -> Resultat<VerificationAudit> {
        ServiceAdministration::nouveau(&appel.ouverte()?.noyau)
            .verifier_journal(appel.session()?)
            .await
    }

    async fn sauvegarde_creer(&self, appel: Appel, _entree: ()) -> Resultat<Vec<Sauvegarde>> {
        let session = appel.session()?;
        self.sauvegardes(appel.ouverte()?)
            .creer(
                session.etablissement_id,
                Some(session),
                MotifSauvegarde::Manuelle,
            )
            .await
    }

    async fn sauvegardes_lister(&self, appel: Appel, _entree: ()) -> Resultat<Vec<Sauvegarde>> {
        self.sauvegardes(appel.ouverte()?)
            .lister(appel.session()?.etablissement_id, 100)
            .await
    }

    async fn sauvegarde_tester(&self, appel: Appel, _entree: ()) -> Resultat<RapportTest> {
        let session = appel.session()?;
        self.sauvegardes(appel.ouverte()?)
            .tester(session.etablissement_id, Some(session))
            .await
    }

    async fn sauvegarde_destinations_lister(
        &self,
        appel: Appel,
        _entree: (),
    ) -> Resultat<Vec<Destination>> {
        self.sauvegardes(appel.ouverte()?)
            .destinations(appel.session()?.etablissement_id)
            .await
    }

    async fn sauvegarde_destinations_definir(
        &self,
        appel: Appel,
        entree: Vec<Destination>,
    ) -> Resultat<Vec<Destination>> {
        let locale = self
            .espace
            .sauvegardes_locales()
            .to_string_lossy()
            .into_owned();
        let configurees = entree.into_iter().filter(|d| d.chemin != locale).collect();
        self.sauvegardes(appel.ouverte()?)
            .definir_destinations(appel.session()?, configurees)
            .await
    }

    async fn certificat_confirmer(&self, appel: Appel, _entree: ()) -> Resultat<()> {
        self.sauvegardes(appel.ouverte()?)
            .confirmer_certificat(appel.session()?)
            .await
    }

    async fn sauvegarde_restaurer(
        &self,
        appel: Appel,
        entree: DemandeRestauration,
    ) -> Resultat<RapportRestauration> {
        let session = appel.session()?.clone();
        // R-48 : l'état courant est sauvegardé avant d'être remplacé.
        self.sauvegardes(appel.ouverte()?)
            .creer(
                session.etablissement_id,
                Some(&session),
                MotifSauvegarde::AvantRestauration,
            )
            .await?;
        drop(appel);
        self.restaurer_et_rouvrir(
            Path::new(&entree.chemin),
            entree.phrase.as_deref(),
            Some(&session),
        )
        .await
    }

    async fn phrase_secours_regenerer(
        &self,
        appel: Appel,
        entree: DemandeRegeneration,
    ) -> Resultat<PhraseAffichee> {
        let ouverte = appel.ouverte()?;
        let session = appel.session()?;
        let contexte = ouverte
            .noyau
            .contexte(session.etablissement_id, Some(session));
        {
            let mut cnx = ouverte
                .noyau
                .base()
                .pool()
                .acquire()
                .await
                .map_err(gesco_db::ErreurBase::from)?;
            let secrets =
                utilisateur::lire_secrets(&mut cnx, session.utilisateur_id, &contexte).await?;
            if !mot_de_passe_correspond(
                &MotDePasse(&entree.mot_de_passe),
                &secrets.empreinte_mot_de_passe,
            ) {
                return Err(ErreurService::Authentification(
                    MotifRefus::IdentifiantsInvalides,
                ));
            }
        }
        let ancienne = self.cles.enveloppe_secours()?.map(|e| e.id);
        let (phrase, modele_id, modele_version) = self
            .phrase_et_certificat(ouverte, session.etablissement_id, &session.nom_affiche)
            .await?;
        let mut unite = ouverte
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        parametre::definir(
            unite.connexion(),
            &Parametre {
                etablissement_id: session.etablissement_id,
                cle: PARAMETRE_CERTIFICAT.to_owned(),
                valeur: ValeurParametre::Booleen(false),
                modifiable_par_utilisateur: false,
            },
            &contexte,
        )
        .await?;
        unite.auditer(
            "enveloppe_secours",
            &phrase.enveloppe_id,
            Operation::Modification,
            Some(serde_json::json!({ "enveloppe_id": ancienne })),
            Some(serde_json::json!({
                "enveloppe_id": phrase.enveloppe_id,
                "certificat": { "modele_id": modele_id, "modele_version": modele_version },
            })),
            "sauvegarde.phrase_secours_regenerer",
        );
        unite.emettre(
            TypeEvenement::PhraseSecoursRegeneree,
            "enveloppe_secours",
            &phrase.enveloppe_id,
            serde_json::json!({ "ancienne": ancienne, "nouvelle": phrase.enveloppe_id }),
        );
        ouverte.noyau.terminer(unite).await?;
        Ok(phrase)
    }

    async fn exporter_xlsx(&self, appel: Appel, entree: DemandeExport) -> Resultat<FichierEcrit> {
        // Connecté, rien de plus : la liste vient de l'écran, rien n'est lu en base (§11).
        appel.session()?;
        crate::export::exporter_xlsx(entree).await
    }

    async fn grilles_lister(
        &self,
        appel: Appel,
        entree: DemandeGrillesAnnee,
    ) -> Resultat<Vec<GrilleAffichee>> {
        ServiceFrais::nouveau(&appel.ouverte()?.noyau)
            .lister(appel.session()?, entree.annee_id)
            .await
    }

    async fn grille_creer(
        &self,
        appel: Appel,
        entree: DemandeCreationGrille,
    ) -> Resultat<GrilleAffichee> {
        ServiceFrais::nouveau(&appel.ouverte()?.noyau)
            .creer(appel.session()?, &entree)
            .await
    }

    async fn grille_modifier(
        &self,
        appel: Appel,
        entree: DemandeModificationGrille,
    ) -> Resultat<GrilleAffichee> {
        ServiceFrais::nouveau(&appel.ouverte()?.noyau)
            .modifier(appel.session()?, &entree)
            .await
    }

    async fn grille_mettre_en_vigueur(
        &self,
        appel: Appel,
        entree: DemandeGrille,
    ) -> Resultat<MiseEnVigueur> {
        ServiceFrais::nouveau(&appel.ouverte()?.noyau)
            .mettre_en_vigueur(appel.session()?, entree.id)
            .await
    }

    async fn situation_financiere(
        &self,
        appel: Appel,
        entree: DemandeEleve,
    ) -> Resultat<Vec<SituationInscription>> {
        ServiceFacturation::nouveau(&appel.ouverte()?.noyau)
            .situation(appel.session()?, entree.id)
            .await
    }

    async fn echeancier_changer_plan(
        &self,
        appel: Appel,
        entree: DemandeChangementPlan,
    ) -> Resultat<FactureAffichee> {
        ServiceFacturation::nouveau(&appel.ouverte()?.noyau)
            .changer_plan(appel.session()?, &entree)
            .await
    }

    async fn factures_rattraper_apercu(
        &self,
        appel: Appel,
        _entree: (),
    ) -> Resultat<Vec<LigneRattrapage>> {
        ServiceFacturation::nouveau(&appel.ouverte()?.noyau)
            .rattrapage_apercu(appel.session()?)
            .await
    }

    async fn factures_rattraper(&self, appel: Appel, _entree: ()) -> Resultat<RapportRattrapage> {
        let ouverte = appel.ouverte()?;
        let session = appel.session()?;
        // R-48 : la sauvegarde précède toute écriture en masse.
        self.sauvegardes(ouverte)
            .creer(
                session.etablissement_id,
                Some(session),
                MotifSauvegarde::AvantFacturation,
            )
            .await?;
        ServiceFacturation::nouveau(&ouverte.noyau)
            .rattraper(session)
            .await
    }

    async fn remise_proposer(
        &self,
        appel: Appel,
        entree: DemandeRemise,
    ) -> Resultat<gesco_core::finance::Remise> {
        ServiceRemises::nouveau(&appel.ouverte()?.noyau)
            .proposer(appel.session()?, &entree)
            .await
    }

    async fn remise_accorder(
        &self,
        appel: Appel,
        entree: DemandeIdRemise,
    ) -> Resultat<gesco_core::finance::Remise> {
        ServiceRemises::nouveau(&appel.ouverte()?.noyau)
            .accorder(appel.session()?, entree.id)
            .await
    }

    async fn remise_refuser(
        &self,
        appel: Appel,
        entree: DemandeDecisionRemise,
    ) -> Resultat<gesco_core::finance::Remise> {
        ServiceRemises::nouveau(&appel.ouverte()?.noyau)
            .refuser(appel.session()?, &entree)
            .await
    }

    async fn remise_annuler(
        &self,
        appel: Appel,
        entree: DemandeDecisionRemise,
    ) -> Resultat<gesco_core::finance::Remise> {
        ServiceRemises::nouveau(&appel.ouverte()?.noyau)
            .annuler(appel.session()?, &entree)
            .await
    }

    async fn remises_en_attente(
        &self,
        appel: Appel,
        _entree: (),
    ) -> Resultat<Vec<RemiseEnAttente>> {
        ServiceRemises::nouveau(&appel.ouverte()?.noyau)
            .en_attente(appel.session()?)
            .await
    }

    async fn caisse_session_courante(
        &self,
        appel: Appel,
        _entree: (),
    ) -> Resultat<Option<SessionAffichee>> {
        ServiceCaisse::nouveau(&appel.ouverte()?.noyau)
            .courante(appel.session()?)
            .await
    }

    async fn caisse_ouvrir(
        &self,
        appel: Appel,
        entree: DemandeOuverture,
    ) -> Resultat<SessionAffichee> {
        ServiceCaisse::nouveau(&appel.ouverte()?.noyau)
            .ouvrir(appel.session()?, entree.fonds_initial)
            .await
    }

    async fn caisse_cloturer(
        &self,
        appel: Appel,
        entree: DemandeCloture,
    ) -> Resultat<SessionAffichee> {
        ServiceCaisse::nouveau(&appel.ouverte()?.noyau)
            .cloturer(appel.session()?, &entree)
            .await
    }

    async fn caisse_rapprocher(
        &self,
        appel: Appel,
        entree: DemandeVisa,
    ) -> Resultat<SessionAffichee> {
        ServiceCaisse::nouveau(&appel.ouverte()?.noyau)
            .rapprocher(appel.session()?, &entree)
            .await
    }

    async fn caisse_sessions_lister(
        &self,
        appel: Appel,
        _entree: (),
    ) -> Resultat<Vec<SessionAffichee>> {
        ServiceCaisse::nouveau(&appel.ouverte()?.noyau)
            .lister(appel.session()?)
            .await
    }

    async fn etat_caisse_produire(
        &self,
        appel: Appel,
        entree: DemandeSession,
    ) -> Resultat<DocumentAffiche> {
        let ouverte = appel.ouverte()?;
        let session = appel.session()?;
        let donnees = ServiceCaisse::nouveau(&ouverte.noyau)
            .donnees_etat(session, entree.id)
            .await?;
        let contexte = ouverte
            .noyau
            .contexte(session.etablissement_id, Some(session));
        let produit = ServiceDocuments::nouveau(&ouverte.noyau, &self.atelier)
            .produire(
                contexte,
                "etat_caisse",
                "session_caisse",
                &entree.id.vers_chaine(),
                donnees,
            )
            .await?;
        Ok(DocumentAffiche {
            pdf: base64(&produit.pdf),
            document: produit.document,
        })
    }

    /// Compose le reçu d'un versement ; `duplicata` pour une réimpression.
    async fn produire_recu(
        &self,
        appel: &Appel,
        versement: gesco_core::finance::Versement,
        duplicata: bool,
    ) -> Resultat<RecuEmis> {
        let ouverte = appel.ouverte()?;
        let session = appel.session()?;
        let donnees = ServiceEncaissement::nouveau(&ouverte.noyau)
            .donnees_recu(session, versement.id, duplicata)
            .await?;
        let contexte = ouverte
            .noyau
            .contexte(session.etablissement_id, Some(session));
        let produit = ServiceDocuments::nouveau(&ouverte.noyau, &self.atelier)
            .produire(
                contexte,
                "recu",
                "versement",
                &versement.id.vers_chaine(),
                donnees,
            )
            .await?;
        Ok(RecuEmis {
            versement,
            recu: DocumentAffiche {
                pdf: base64(&produit.pdf),
                document: produit.document,
            },
        })
    }

    async fn encaissement_preparer(
        &self,
        appel: Appel,
        entree: DemandePreparation,
    ) -> Resultat<PreparationEncaissement> {
        ServiceEncaissement::nouveau(&appel.ouverte()?.noyau)
            .preparer(appel.session()?, &entree)
            .await
    }

    async fn encaisser(&self, appel: Appel, entree: DemandeEncaissement) -> Resultat<RecuEmis> {
        let fait = ServiceEncaissement::nouveau(&appel.ouverte()?.noyau)
            .encaisser(appel.session()?, entree)
            .await?;
        self.produire_recu(&appel, fait.versement, !fait.nouveau)
            .await
    }

    async fn encaisser_famille(
        &self,
        appel: Appel,
        entree: DemandeFamille,
    ) -> Resultat<Vec<RecuEmis>> {
        let faits = ServiceEncaissement::nouveau(&appel.ouverte()?.noyau)
            .encaisser_famille(appel.session()?, entree)
            .await?;
        let mut recus = Vec::with_capacity(faits.len());
        for fait in faits {
            recus.push(
                self.produire_recu(&appel, fait.versement, !fait.nouveau)
                    .await?,
            );
        }
        Ok(recus)
    }

    async fn versement_annuler(
        &self,
        appel: Appel,
        entree: DemandeAnnulationVersement,
    ) -> Resultat<AnnulationEmise> {
        let fait = ServiceEncaissement::nouveau(&appel.ouverte()?.noyau)
            .annuler(appel.session()?, &entree)
            .await?;
        let emis = self.produire_recu(&appel, fait.versement, false).await?;
        Ok(AnnulationEmise {
            versement: emis.versement,
            recu: emis.recu,
            regularisation: fait.regularisation,
        })
    }

    async fn recu_produire(
        &self,
        appel: Appel,
        entree: DemandeVersement,
    ) -> Resultat<DocumentAffiche> {
        let versement = {
            let ouverte = appel.ouverte()?;
            let session = appel.session()?;
            let contexte = ouverte
                .noyau
                .contexte(session.etablissement_id, Some(session));
            let mut cnx = ouverte
                .noyau
                .base()
                .pool()
                .acquire()
                .await
                .map_err(gesco_db::ErreurBase::from)?;
            gesco_db::depots::versements::lire(&mut cnx, entree.id, &contexte).await?
        };
        Ok(self.produire_recu(&appel, versement, true).await?.recu)
    }

    async fn pedagogie_lire(&self, appel: Appel, entree: DemandeIdAnnee) -> Resultat<Pedagogie> {
        ServicePedagogie::nouveau(&appel.ouverte()?.noyau)
            .lire(appel.session()?, entree.id)
            .await
    }

    async fn matiere_enregistrer(
        &self,
        appel: Appel,
        entree: DemandeMatiere,
    ) -> Resultat<gesco_core::pedagogie::Matiere> {
        ServicePedagogie::nouveau(&appel.ouverte()?.noyau)
            .enregistrer_matiere(appel.session()?, &entree)
            .await
    }

    async fn programme_enregistrer(
        &self,
        appel: Appel,
        entree: DemandeProgramme,
    ) -> Resultat<Vec<gesco_core::pedagogie::LigneProgramme>> {
        ServicePedagogie::nouveau(&appel.ouverte()?.noyau)
            .enregistrer_programme(appel.session()?, &entree)
            .await
    }

    async fn programme_reprendre(&self, appel: Appel, entree: DemandeIdAnnee) -> Resultat<u32> {
        let n = ServicePedagogie::nouveau(&appel.ouverte()?.noyau)
            .reprendre_programme(appel.session()?, entree.id)
            .await?;
        Ok(u32::try_from(n).unwrap_or(u32::MAX))
    }

    async fn periodes_definir(
        &self,
        appel: Appel,
        entree: DemandeNombrePeriodes,
    ) -> Resultat<Vec<gesco_core::pedagogie::Periode>> {
        ServicePedagogie::nouveau(&appel.ouverte()?.noyau)
            .definir_periodes(appel.session()?, &entree)
            .await
    }

    async fn calendrier_modifier(
        &self,
        appel: Appel,
        entree: DemandeCalendrier,
    ) -> Resultat<Vec<gesco_core::pedagogie::Periode>> {
        ServicePedagogie::nouveau(&appel.ouverte()?.noyau)
            .modifier_calendrier(appel.session()?, &entree)
            .await
    }

    async fn regle_enregistrer(
        &self,
        appel: Appel,
        entree: DemandeRegle,
    ) -> Resultat<gesco_core::pedagogie::RegleEvaluation> {
        ServicePedagogie::nouveau(&appel.ouverte()?.noyau)
            .enregistrer_regle(appel.session()?, &entree)
            .await
    }

    async fn enseignants_lister(
        &self,
        appel: Appel,
        _entree: (),
    ) -> Resultat<Vec<FicheEnseignant>> {
        ServiceEnseignants::nouveau(&appel.ouverte()?.noyau)
            .lister(appel.session()?)
            .await
    }

    async fn enseignant_enregistrer(
        &self,
        appel: Appel,
        entree: DemandeEnseignant,
    ) -> Resultat<FicheEnseignant> {
        ServiceEnseignants::nouveau(&appel.ouverte()?.noyau)
            .enregistrer(appel.session()?, &entree)
            .await
    }

    async fn affectations_lister(
        &self,
        appel: Appel,
        entree: DemandeIdAnnee,
    ) -> Resultat<Vec<AffectationAffichee>> {
        ServiceEnseignants::nouveau(&appel.ouverte()?.noyau)
            .affectations(appel.session()?, entree.id)
            .await
    }

    async fn affectation_ajouter(
        &self,
        appel: Appel,
        entree: DemandeAffectation,
    ) -> Resultat<gesco_core::pedagogie::AffectationEnseignement> {
        ServiceEnseignants::nouveau(&appel.ouverte()?.noyau)
            .affecter(appel.session()?, &entree, false)
            .await
    }

    async fn affectation_confier(
        &self,
        appel: Appel,
        entree: DemandeAffectation,
    ) -> Resultat<gesco_core::pedagogie::AffectationEnseignement> {
        ServiceEnseignants::nouveau(&appel.ouverte()?.noyau)
            .affecter(appel.session()?, &entree, true)
            .await
    }

    async fn affectation_retirer(
        &self,
        appel: Appel,
        entree: DemandeIdAffectation,
    ) -> Resultat<()> {
        ServiceEnseignants::nouveau(&appel.ouverte()?.noyau)
            .retirer(appel.session()?, entree.id)
            .await
    }

    async fn maitre_affecter(&self, appel: Appel, entree: DemandeMaitre) -> Resultat<u32> {
        let n = ServiceEnseignants::nouveau(&appel.ouverte()?.noyau)
            .affecter_maitre(appel.session()?, &entree)
            .await?;
        Ok(u32::try_from(n).unwrap_or(u32::MAX))
    }

    async fn bulletins_lire(
        &self,
        appel: Appel,
        entree: DemandeBulletins,
    ) -> Resultat<BulletinsClasse> {
        ServiceBulletins::nouveau(&appel.ouverte()?.noyau)
            .lire(appel.session()?, &entree)
            .await
    }

    async fn resultats_calculer(
        &self,
        appel: Appel,
        entree: DemandeBulletins,
    ) -> Resultat<BulletinsClasse> {
        ServiceBulletins::nouveau(&appel.ouverte()?.noyau)
            .calculer(appel.session()?, &entree)
            .await
    }

    async fn appreciation_saisir(
        &self,
        appel: Appel,
        entree: DemandeAppreciation,
    ) -> Resultat<gesco_core::pedagogie::Bulletin> {
        ServiceBulletins::nouveau(&appel.ouverte()?.noyau)
            .apprecier(appel.session()?, &entree)
            .await
    }

    async fn bulletins_valider(&self, appel: Appel, entree: DemandeBulletins) -> Resultat<u32> {
        ServiceBulletins::nouveau(&appel.ouverte()?.noyau)
            .valider(appel.session()?, &entree)
            .await
    }

    async fn bulletins_publier(
        &self,
        appel: Appel,
        entree: DemandeBulletins,
    ) -> Resultat<DocumentAffiche> {
        let publies = ServiceBulletins::nouveau(&appel.ouverte()?.noyau)
            .publier(appel.session()?, &entree)
            .await?;
        self.imprimer_bulletins(&appel, &publies, "classe", &entree.classe_id.vers_chaine())
            .await
    }

    async fn bulletin_corriger(
        &self,
        appel: Appel,
        entree: DemandeCorrection,
    ) -> Resultat<gesco_core::pedagogie::Bulletin> {
        ServiceBulletins::nouveau(&appel.ouverte()?.noyau)
            .corriger(appel.session()?, &entree)
            .await
    }

    async fn bulletins_produire(
        &self,
        appel: Appel,
        entree: DemandeBulletins,
    ) -> Resultat<DocumentAffiche> {
        let liste = ServiceBulletins::nouveau(&appel.ouverte()?.noyau)
            .a_imprimer(appel.session()?, &entree)
            .await?;
        self.imprimer_bulletins(&appel, &liste, "classe", &entree.classe_id.vers_chaine())
            .await
    }

    async fn bulletin_produire(
        &self,
        appel: Appel,
        entree: DemandeIdBulletin,
    ) -> Resultat<DocumentAffiche> {
        let b = ServiceBulletins::nouveau(&appel.ouverte()?.noyau)
            .un(appel.session()?, entree.id)
            .await?;
        self.imprimer_bulletins(
            &appel,
            std::slice::from_ref(&b),
            "bulletin",
            &b.id.vers_chaine(),
        )
        .await
    }

    async fn tableau_de_bord_lire(&self, appel: Appel, _entree: ()) -> Resultat<TableauDeBord> {
        ServiceTableau::nouveau(&appel.ouverte()?.noyau)
            .lire(appel.session()?)
            .await
    }

    async fn inscription_transferer(
        &self,
        appel: Appel,
        entree: DemandeTransfert,
    ) -> Resultat<TransfertEnregistre> {
        ServiceDeparts::nouveau(&appel.ouverte()?.noyau)
            .transferer(appel.session()?, &entree)
            .await
    }

    async fn certificat_transfert_produire(
        &self,
        appel: Appel,
        entree: DemandeDepart,
    ) -> Resultat<DocumentAffiche> {
        let donnees = ServiceDeparts::nouveau(&appel.ouverte()?.noyau)
            .donnees_certificat(appel.session()?, entree.inscription_id)
            .await?;
        self.produire_document(&appel, "certificat_transfert", &entree, donnees)
            .await
    }

    async fn releve_moyennes_produire(
        &self,
        appel: Appel,
        entree: DemandeDepart,
    ) -> Resultat<DocumentAffiche> {
        let donnees = ServiceDeparts::nouveau(&appel.ouverte()?.noyau)
            .donnees_releve(appel.session()?, entree.inscription_id)
            .await?;
        self.produire_document(&appel, "releve_moyennes", &entree, donnees)
            .await
    }

    async fn inscription_exclure(
        &self,
        appel: Appel,
        entree: DemandeExclusion,
    ) -> Resultat<gesco_core::scolarite::Exclusion> {
        ServiceDeparts::nouveau(&appel.ouverte()?.noyau)
            .exclure(appel.session()?, &entree)
            .await
    }

    async fn exclusion_lever(
        &self,
        appel: Appel,
        entree: DemandeLevee,
    ) -> Resultat<gesco_core::scolarite::Exclusion> {
        ServiceDeparts::nouveau(&appel.ouverte()?.noyau)
            .lever(appel.session()?, &entree)
            .await
    }

    async fn exclusions_lister(
        &self,
        appel: Appel,
        entree: DemandeDepart,
    ) -> Resultat<Vec<gesco_core::scolarite::Exclusion>> {
        ServiceDeparts::nouveau(&appel.ouverte()?.noyau)
            .exclusions(appel.session()?, entree.inscription_id)
            .await
    }

    async fn moyennes_reprises_lire(
        &self,
        appel: Appel,
        entree: DemandeBulletins,
    ) -> Resultat<ReprisesClasse> {
        ServiceBulletins::nouveau(&appel.ouverte()?.noyau)
            .reprises(appel.session()?, &entree)
            .await
    }

    async fn moyenne_reprise_saisir(
        &self,
        appel: Appel,
        entree: DemandeReprise,
    ) -> Resultat<Option<gesco_core::pedagogie::MoyenneReprise>> {
        ServiceBulletins::nouveau(&appel.ouverte()?.noyau)
            .reprendre(appel.session()?, &entree)
            .await
    }

    async fn portee_absences_lire(
        &self,
        appel: Appel,
        _entree: (),
    ) -> Resultat<gesco_core::pedagogie::PorteeAbsences> {
        ServiceAbsences::nouveau(&appel.ouverte()?.noyau)
            .portee(appel.session()?)
            .await
    }

    async fn absences_releve_lire(
        &self,
        appel: Appel,
        entree: DemandeReleve,
    ) -> Resultat<ReleveAbsences> {
        ServiceAbsences::nouveau(&appel.ouverte()?.noyau)
            .releve(appel.session()?, &entree)
            .await
    }

    async fn absences_releve_enregistrer(
        &self,
        appel: Appel,
        entree: DemandeEnregistrementReleve,
    ) -> Resultat<BilanReleve> {
        ServiceAbsences::nouveau(&appel.ouverte()?.noyau)
            .enregistrer(appel.session()?, &entree)
            .await
    }

    async fn absences_justifier(
        &self,
        appel: Appel,
        entree: DemandeJustification,
    ) -> Resultat<u32> {
        ServiceAbsences::nouveau(&appel.ouverte()?.noyau)
            .justifier(appel.session()?, &entree)
            .await
    }

    async fn absences_periode_lire(
        &self,
        appel: Appel,
        entree: DemandeAbsencesPeriode,
    ) -> Resultat<AbsencesPeriode> {
        ServiceAbsences::nouveau(&appel.ouverte()?.noyau)
            .de_la_periode(appel.session()?, &entree)
            .await
    }

    async fn portee_notes_lire(
        &self,
        appel: Appel,
        _entree: (),
    ) -> Resultat<gesco_core::pedagogie::PorteeNotes> {
        ServiceEnseignants::nouveau(&appel.ouverte()?.noyau)
            .portee_notes(appel.session()?)
            .await
    }

    async fn evaluations_lister(
        &self,
        appel: Appel,
        entree: DemandeEvaluations,
    ) -> Resultat<Vec<EvaluationAffichee>> {
        ServiceEvaluations::nouveau(&appel.ouverte()?.noyau)
            .lister(appel.session()?, &entree)
            .await
    }

    async fn evaluation_creer(
        &self,
        appel: Appel,
        entree: DemandeEvaluation,
    ) -> Resultat<gesco_core::pedagogie::Evaluation> {
        ServiceEvaluations::nouveau(&appel.ouverte()?.noyau)
            .creer(appel.session()?, &entree)
            .await
    }

    async fn notes_grille(
        &self,
        appel: Appel,
        entree: DemandeIdEvaluation,
    ) -> Resultat<GrilleNotes> {
        ServiceEvaluations::nouveau(&appel.ouverte()?.noyau)
            .grille(appel.session()?, entree.id)
            .await
    }

    async fn note_saisir(
        &self,
        appel: Appel,
        entree: DemandeNote,
    ) -> Resultat<Option<gesco_core::pedagogie::NoteEleve>> {
        ServiceEvaluations::nouveau(&appel.ouverte()?.noyau)
            .saisir(appel.session()?, &entree)
            .await
    }

    async fn evaluation_valider(
        &self,
        appel: Appel,
        entree: DemandeIdEvaluation,
    ) -> Resultat<gesco_core::pedagogie::Evaluation> {
        ServiceEvaluations::nouveau(&appel.ouverte()?.noyau)
            .valider(appel.session()?, entree.id)
            .await
    }

    async fn evaluation_deverrouiller(
        &self,
        appel: Appel,
        entree: DemandeDeverrouillage,
    ) -> Resultat<gesco_core::pedagogie::Evaluation> {
        ServiceEvaluations::nouveau(&appel.ouverte()?.noyau)
            .deverrouiller(appel.session()?, &entree)
            .await
    }

    async fn inscriptions_valider_classe(
        &self,
        appel: Appel,
        entree: DemandeClasse,
    ) -> Resultat<crate::inscriptions::BilanValidation> {
        ServiceInscriptions::nouveau(&appel.ouverte()?.noyau)
            .valider_classe(appel.session()?, entree.id)
            .await
    }

    async fn notes_import_analyser(
        &self,
        appel: Appel,
        entree: DemandeImportNotes,
    ) -> Resultat<ApercuImportNotes> {
        ServiceEvaluations::nouveau(&appel.ouverte()?.noyau)
            .analyser_import(appel.session()?, &entree)
            .await
    }

    async fn notes_importer(
        &self,
        appel: Appel,
        entree: DemandeImportNotes,
    ) -> Resultat<ApercuImportNotes> {
        ServiceEvaluations::nouveau(&appel.ouverte()?.noyau)
            .importer(appel.session()?, &entree)
            .await
    }

    async fn impayes_lister(&self, appel: Appel, _entree: ()) -> Resultat<ListeImpayes> {
        ServiceRecouvrement::nouveau(&appel.ouverte()?.noyau)
            .impayes(appel.session()?)
            .await
    }

    async fn encaissements_lister(
        &self,
        appel: Appel,
        entree: DemandeJournal,
    ) -> Resultat<JournalEncaissements> {
        ServiceRecouvrement::nouveau(&appel.ouverte()?.noyau)
            .journal(appel.session()?, &entree)
            .await
    }

    /// Compose un document d'un élève parti (certificat de transfert, relevé des moyennes).
    async fn produire_document(
        &self,
        appel: &Appel,
        modele: &str,
        entree: &DemandeDepart,
        donnees: serde_json::Value,
    ) -> Resultat<DocumentAffiche> {
        let ouverte = appel.ouverte()?;
        let session = appel.session()?;
        let contexte = ouverte
            .noyau
            .contexte(session.etablissement_id, Some(session));
        let produit = ServiceDocuments::nouveau(&ouverte.noyau, &self.atelier)
            .produire(
                contexte,
                modele,
                "inscription",
                &entree.inscription_id.vers_chaine(),
                donnees,
            )
            .await?;
        Ok(DocumentAffiche {
            pdf: base64(&produit.pdf),
            document: produit.document,
        })
    }

    /// Compose les bulletins en un seul PDF, chacun avec son QR de vérification.
    async fn imprimer_bulletins(
        &self,
        appel: &Appel,
        liste: &[gesco_core::pedagogie::Bulletin],
        entite: &str,
        entite_id: &str,
    ) -> Resultat<DocumentAffiche> {
        let ouverte = appel.ouverte()?;
        let session = appel.session()?;
        // Les bulletins d'un lot sont d'une même classe, donc d'un même cycle.
        let cycle = liste
            .first()
            .map_or(gesco_core::scolarite::Cycle::SecondCycle, |b| {
                b.contenu.cycle
            });
        let reglages = ServiceBulletins::nouveau(&ouverte.noyau)
            .reglages_impression(session, cycle)
            .await?;
        let (donnees, pieces) = donnees_impression(liste, reglages)?;
        let contexte = ouverte
            .noyau
            .contexte(session.etablissement_id, Some(session));
        let produit = ServiceDocuments::nouveau(&ouverte.noyau, &self.atelier)
            .produire_avec(contexte, "bulletin", entite, entite_id, donnees, pieces)
            .await?;
        Ok(DocumentAffiche {
            pdf: base64(&produit.pdf),
            document: produit.document,
        })
    }

    async fn releve_compte_produire(
        &self,
        appel: Appel,
        entree: DemandeEleve,
    ) -> Resultat<DocumentAffiche> {
        let ouverte = appel.ouverte()?;
        let session = appel.session()?;
        let donnees = ServiceEncaissement::nouveau(&ouverte.noyau)
            .donnees_releve(session, entree.id)
            .await?;
        let contexte = ouverte
            .noyau
            .contexte(session.etablissement_id, Some(session));
        let produit = ServiceDocuments::nouveau(&ouverte.noyau, &self.atelier)
            .produire(
                contexte,
                "releve_compte",
                "eleve",
                &entree.id.vers_chaine(),
                donnees,
            )
            .await?;
        Ok(DocumentAffiche {
            pdf: base64(&produit.pdf),
            document: produit.document,
        })
    }

    /// Fichier déposé par cet utilisateur, sous cet identifiant.
    async fn fichier_importe(&self, session: &Session, id: &str) -> Resultat<FichierImporte> {
        self.imports
            .lock()
            .await
            .get(id)
            .filter(|f| f.utilisateur_id == session.utilisateur_id)
            .cloned()
            .ok_or(ErreurService::Regle {
                cle: "import.fichier_inconnu",
            })
    }

    async fn import_eleves_analyser(
        &self,
        appel: Appel,
        entree: DemandeAnalyse,
    ) -> Resultat<Analyse> {
        let ouverte = appel.ouverte()?;
        let session = appel.session()?;
        let chemin = entree.chemin.clone();
        let feuilles =
            tokio::task::spawn_blocking(move || crate::import_eleves::lire_fichier(&chemin))
                .await
                .map_err(|e| ErreurService::Fichier(std::io::Error::other(e)))??;
        let fichier = FichierImporte {
            utilisateur_id: session.utilisateur_id,
            nom: Path::new(&entree.chemin)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_owned(),
            feuilles,
        };
        let id = nouvel_id::<FichierImporte>().vers_chaine();
        let analyse = ServiceImport::nouveau(&ouverte.noyau)
            .analyser(session, &id, &fichier)
            .await?;
        // Un seul fichier en cours par utilisateur : le précédent est oublié.
        let mut imports = self.imports.lock().await;
        imports.retain(|_, f| f.utilisateur_id != session.utilisateur_id);
        imports.insert(id, fichier);
        Ok(analyse)
    }

    async fn import_eleves_valider(&self, appel: Appel, entree: Reglages) -> Resultat<Apercu> {
        let ouverte = appel.ouverte()?;
        let session = appel.session()?;
        let fichier = self.fichier_importe(session, &entree.fichier_id).await?;
        ServiceImport::nouveau(&ouverte.noyau)
            .valider(session, &fichier, &entree)
            .await
    }

    async fn import_eleves_appliquer(&self, appel: Appel, entree: Reglages) -> Resultat<Rapport> {
        let ouverte = appel.ouverte()?;
        let session = appel.session()?;
        let fichier = self.fichier_importe(session, &entree.fichier_id).await?;
        // R-48 : la sauvegarde précède toute écriture en masse.
        self.sauvegardes(ouverte)
            .creer(
                session.etablissement_id,
                Some(session),
                MotifSauvegarde::AvantImport,
            )
            .await?;
        let rapport = ServiceImport::nouveau(&ouverte.noyau)
            .appliquer(session, &entree.fichier_id, &fichier, &entree)
            .await?;
        self.imports.lock().await.remove(&entree.fichier_id);
        Ok(rapport)
    }

    async fn document_essai_produire(
        &self,
        appel: Appel,
        _entree: (),
    ) -> Resultat<DocumentAffiche> {
        let ouverte = appel.ouverte()?;
        let session = appel.session()?;
        let contexte = ouverte
            .noyau
            .contexte(session.etablissement_id, Some(session));
        let version_schema = ouverte.noyau.base().version_schema().await?;
        let produit = ServiceDocuments::nouveau(&ouverte.noyau, &self.atelier)
            .produire(
                contexte.clone(),
                "essai",
                "etablissement",
                &session.etablissement_id.vers_chaine(),
                serde_json::json!({
                    "titre": "Document d'essai",
                    "date": date_longue(contexte.maintenant),
                    "auteur": session.nom_affiche,
                    "lignes": [
                        { "libelle": "Version de l'application", "valeur": self.version_applicative },
                        { "libelle": "Version du schéma", "valeur": version_schema.to_string() },
                        { "libelle": "Poste", "valeur": ouverte.noyau.poste().0 },
                    ],
                }),
            )
            .await?;
        Ok(DocumentAffiche {
            pdf: base64(&produit.pdf),
            document: produit.document,
        })
    }

    async fn licence_etat(&self, appel: Appel, _entree: ()) -> Resultat<EtatLicenceAffiche> {
        ServiceLicence::nouveau(&appel.ouverte()?.noyau, self.cle_licence)
            .etat(appel.session()?.etablissement_id)
            .await
    }

    async fn licence_installer(
        &self,
        appel: Appel,
        entree: DemandeLicence,
    ) -> Resultat<EtatLicenceAffiche> {
        ServiceLicence::nouveau(&appel.ouverte()?.noyau, self.cle_licence)
            .installer(appel.session()?, &entree.contenu)
            .await
    }

    async fn personnel_referentiel_lire(
        &self,
        appel: Appel,
        _entree: (),
    ) -> Resultat<ReferentielPersonnel> {
        ServicePersonnel::nouveau(&appel.ouverte()?.noyau)
            .referentiel(appel.session()?)
            .await
    }

    async fn categorie_personnel_enregistrer(
        &self,
        appel: Appel,
        entree: DemandeCategoriePersonnel,
    ) -> Resultat<gesco_core::rh::CategoriePersonnel> {
        ServicePersonnel::nouveau(&appel.ouverte()?.noyau)
            .enregistrer_categorie(appel.session()?, &entree)
            .await
    }

    async fn fonction_personnel_enregistrer(
        &self,
        appel: Appel,
        entree: DemandeFonctionPersonnel,
    ) -> Resultat<gesco_core::rh::FonctionPersonnel> {
        ServicePersonnel::nouveau(&appel.ouverte()?.noyau)
            .enregistrer_fonction(appel.session()?, &entree)
            .await
    }

    async fn personnel_lister(&self, appel: Appel, _entree: ()) -> Resultat<Vec<MembreAffiche>> {
        ServicePersonnel::nouveau(&appel.ouverte()?.noyau)
            .lister(appel.session()?)
            .await
    }

    async fn personnel_import_modele(
        &self,
        appel: Appel,
        entree: DemandeModelePersonnel,
    ) -> Resultat<FichierEcrit> {
        appel.session()?;
        crate::import_personnel::ecrire_modele(entree).await
    }

    async fn personnel_import_analyser(
        &self,
        appel: Appel,
        entree: DemandeImportPersonnel,
    ) -> Resultat<AnalyseImportPersonnel> {
        ServiceImportPersonnel::nouveau(&appel.ouverte()?.noyau)
            .analyser(appel.session()?, &entree)
            .await
    }

    async fn personnel_import_appliquer(
        &self,
        appel: Appel,
        entree: DemandeImportPersonnel,
    ) -> Resultat<BilanImportPersonnel> {
        let ouverte = appel.ouverte()?;
        let session = appel.session()?;
        // R-48 : la sauvegarde précède toute écriture en masse.
        self.sauvegardes(ouverte)
            .creer(
                session.etablissement_id,
                Some(session),
                MotifSauvegarde::AvantImport,
            )
            .await?;
        ServiceImportPersonnel::nouveau(&ouverte.noyau)
            .appliquer(session, &entree)
            .await
    }

    async fn membre_lire(&self, appel: Appel, entree: DemandeIdMembre) -> Resultat<FicheMembre> {
        ServicePersonnel::nouveau(&appel.ouverte()?.noyau)
            .lire(appel.session()?, entree.id)
            .await
    }

    async fn ma_fiche_lire(&self, appel: Appel, _entree: ()) -> Resultat<Option<MaFiche>> {
        ServicePersonnel::nouveau(&appel.ouverte()?.noyau)
            .ma_fiche(appel.session()?)
            .await
    }

    async fn membre_enregistrer(
        &self,
        appel: Appel,
        entree: DemandeMembre,
    ) -> Resultat<FicheMembre> {
        ServicePersonnel::nouveau(&appel.ouverte()?.noyau)
            .enregistrer(appel.session()?, &entree)
            .await
    }

    async fn membre_partir(
        &self,
        appel: Appel,
        entree: DemandeDepartMembre,
    ) -> Resultat<FicheMembre> {
        ServicePersonnel::nouveau(&appel.ouverte()?.noyau)
            .partir(appel.session()?, &entree)
            .await
    }

    async fn membre_revenir(&self, appel: Appel, entree: DemandeIdMembre) -> Resultat<FicheMembre> {
        ServicePersonnel::nouveau(&appel.ouverte()?.noyau)
            .revenir(appel.session()?, entree.id)
            .await
    }

    async fn membre_enseignant(
        &self,
        appel: Appel,
        entree: DemandeMembreEnseignant,
    ) -> Resultat<FicheMembre> {
        ServicePersonnel::nouveau(&appel.ouverte()?.noyau)
            .devenir_enseignant(appel.session()?, &entree)
            .await
    }

    async fn piece_dossier_ajouter(
        &self,
        appel: Appel,
        entree: DemandePieceDossier,
    ) -> Resultat<gesco_core::rh::PieceDossier> {
        ServicePersonnel::nouveau(&appel.ouverte()?.noyau)
            .ajouter_piece(appel.session()?, &entree)
            .await
    }

    async fn piece_dossier_enregistrer_sous(
        &self,
        appel: Appel,
        entree: DemandePieceSous,
    ) -> Resultat<gesco_core::rh::PieceDossier> {
        ServicePersonnel::nouveau(&appel.ouverte()?.noyau)
            .enregistrer_piece_sous(appel.session()?, &entree)
            .await
    }

    async fn piece_dossier_retirer(
        &self,
        appel: Appel,
        entree: DemandeIdPiece,
    ) -> Resultat<gesco_core::rh::PieceDossier> {
        ServicePersonnel::nouveau(&appel.ouverte()?.noyau)
            .retirer_piece(appel.session()?, entree.id)
            .await
    }

    async fn engagements_lister(
        &self,
        appel: Appel,
        entree: DemandeIdMembre,
    ) -> Resultat<Vec<EngagementAffiche>> {
        ServiceEngagements::nouveau(&appel.ouverte()?.noyau)
            .lister(appel.session()?, entree.id)
            .await
    }

    async fn engagement_conclure(
        &self,
        appel: Appel,
        entree: DemandeEngagement,
    ) -> Resultat<gesco_core::rh::Engagement> {
        ServiceEngagements::nouveau(&appel.ouverte()?.noyau)
            .conclure(appel.session()?, &entree)
            .await
    }

    async fn engagement_terminer(
        &self,
        appel: Appel,
        entree: DemandeClotureEngagement,
    ) -> Resultat<gesco_core::rh::Engagement> {
        ServiceEngagements::nouveau(&appel.ouverte()?.noyau)
            .terminer(appel.session()?, &entree)
            .await
    }

    async fn engagement_rompre(
        &self,
        appel: Appel,
        entree: DemandeClotureEngagement,
    ) -> Resultat<gesco_core::rh::Engagement> {
        ServiceEngagements::nouveau(&appel.ouverte()?.noyau)
            .rompre(appel.session()?, &entree)
            .await
    }

    async fn engagement_renouvellement(
        &self,
        appel: Appel,
        entree: DemandeIdEngagement,
    ) -> Resultat<DemandeEngagementProposee> {
        ServiceEngagements::nouveau(&appel.ouverte()?.noyau)
            .proposer_renouvellement(appel.session()?, entree.id)
            .await
    }

    async fn engagements_a_echeance(
        &self,
        appel: Appel,
        _entree: (),
    ) -> Resultat<Vec<EngagementAEcheance>> {
        ServiceEngagements::nouveau(&appel.ouverte()?.noyau)
            .a_echeance(appel.session()?)
            .await
    }

    async fn lettre_engagement_produire(
        &self,
        appel: Appel,
        entree: DemandeIdEngagement,
    ) -> Resultat<DocumentAffiche> {
        let ouverte = appel.ouverte()?;
        let session = appel.session()?;
        let donnees = ServiceEngagements::nouveau(&ouverte.noyau)
            .donnees_lettre(session, entree.id)
            .await?;
        let contexte = ouverte
            .noyau
            .contexte(session.etablissement_id, Some(session));
        let produit = ServiceDocuments::nouveau(&ouverte.noyau, &self.atelier)
            .produire(
                contexte,
                "engagement",
                "engagement",
                &entree.id.vers_chaine(),
                donnees,
            )
            .await?;
        Ok(DocumentAffiche {
            pdf: base64(&produit.pdf),
            document: produit.document,
        })
    }

    async fn heures_lire(&self, appel: Appel, entree: DemandeMois) -> Resultat<EtatHeures> {
        ServiceHeures::nouveau(&appel.ouverte()?.noyau)
            .lire(appel.session()?, entree.mois)
            .await
    }

    async fn heures_saisir(
        &self,
        appel: Appel,
        entree: DemandeSaisieHeures,
    ) -> Resultat<EtatHeures> {
        ServiceHeures::nouveau(&appel.ouverte()?.noyau)
            .saisir(appel.session()?, &entree)
            .await
    }

    async fn heures_viser(&self, appel: Appel, entree: DemandeVisaHeures) -> Resultat<EtatHeures> {
        ServiceHeures::nouveau(&appel.ouverte()?.noyau)
            .viser(appel.session()?, &entree)
            .await
    }

    async fn heures_deverrouiller(
        &self,
        appel: Appel,
        entree: DemandeDeverrouillageHeures,
    ) -> Resultat<EtatHeures> {
        ServiceHeures::nouveau(&appel.ouverte()?.noyau)
            .deverrouiller(appel.session()?, &entree)
            .await
    }

    async fn etat_heures_produire(
        &self,
        appel: Appel,
        entree: DemandeMois,
    ) -> Resultat<DocumentAffiche> {
        let ouverte = appel.ouverte()?;
        let session = appel.session()?;
        let donnees = ServiceHeures::nouveau(&ouverte.noyau)
            .donnees_etat(session, entree.mois)
            .await?;
        let contexte = ouverte
            .noyau
            .contexte(session.etablissement_id, Some(session));
        let produit = ServiceDocuments::nouveau(&ouverte.noyau, &self.atelier)
            .produire(
                contexte,
                "etat_heures",
                "heures_vacation",
                &entree.mois.to_string(),
                donnees,
            )
            .await?;
        Ok(DocumentAffiche {
            pdf: base64(&produit.pdf),
            document: produit.document,
        })
    }

    async fn absences_conges_lire(
        &self,
        appel: Appel,
        entree: DemandeMoisConges,
    ) -> Resultat<AbsencesEtConges> {
        ServiceConges::nouveau(&appel.ouverte()?.noyau)
            .lire(appel.session()?, entree.mois)
            .await
    }

    async fn absence_personnel_noter(
        &self,
        appel: Appel,
        entree: DemandeAbsencePersonnel,
    ) -> Resultat<gesco_core::rh::AbsencePersonnel> {
        ServiceConges::nouveau(&appel.ouverte()?.noyau)
            .noter_absence(appel.session()?, &entree)
            .await
    }

    async fn absence_personnel_justifier(
        &self,
        appel: Appel,
        entree: DemandeMotifAbsence,
    ) -> Resultat<gesco_core::rh::AbsencePersonnel> {
        ServiceConges::nouveau(&appel.ouverte()?.noyau)
            .justifier_absence(appel.session()?, &entree)
            .await
    }

    async fn absence_personnel_annuler(
        &self,
        appel: Appel,
        entree: DemandeMotifAbsence,
    ) -> Resultat<gesco_core::rh::AbsencePersonnel> {
        ServiceConges::nouveau(&appel.ouverte()?.noyau)
            .annuler_absence(appel.session()?, &entree)
            .await
    }

    async fn conge_demander(
        &self,
        appel: Appel,
        entree: DemandeConge,
    ) -> Resultat<gesco_core::rh::Conge> {
        ServiceConges::nouveau(&appel.ouverte()?.noyau)
            .demander(appel.session()?, &entree)
            .await
    }

    async fn conge_accorder(
        &self,
        appel: Appel,
        entree: DemandeIdConge,
    ) -> Resultat<gesco_core::rh::Conge> {
        ServiceConges::nouveau(&appel.ouverte()?.noyau)
            .accorder(appel.session()?, &entree)
            .await
    }

    async fn conge_refuser(
        &self,
        appel: Appel,
        entree: DemandeMotifConge,
    ) -> Resultat<gesco_core::rh::Conge> {
        ServiceConges::nouveau(&appel.ouverte()?.noyau)
            .refuser(appel.session()?, &entree)
            .await
    }

    async fn conge_annuler(
        &self,
        appel: Appel,
        entree: DemandeMotifConge,
    ) -> Resultat<gesco_core::rh::Conge> {
        ServiceConges::nouveau(&appel.ouverte()?.noyau)
            .annuler(appel.session()?, &entree)
            .await
    }

    async fn type_conge_enregistrer(
        &self,
        appel: Appel,
        entree: DemandeTypeConge,
    ) -> Resultat<gesco_core::rh::TypeConge> {
        ServiceConges::nouveau(&appel.ouverte()?.noyau)
            .enregistrer_type(appel.session()?, &entree)
            .await
    }

    async fn engagement_mois_payes(
        &self,
        appel: Appel,
        entree: DemandeMoisPayes,
    ) -> Resultat<gesco_core::rh::Engagement> {
        ServiceEngagements::nouveau(&appel.ouverte()?.noyau)
            .changer_mois_payes(appel.session()?, &entree)
            .await
    }

    async fn elements_paie_lire(&self, appel: Appel, _entree: ()) -> Resultat<ElementsPaie> {
        ServicePaie::nouveau(&appel.ouverte()?.noyau)
            .lire(appel.session()?)
            .await
    }

    async fn element_paie_enregistrer(
        &self,
        appel: Appel,
        entree: DemandeElement,
    ) -> Resultat<ElementsPaie> {
        ServicePaie::nouveau(&appel.ouverte()?.noyau)
            .enregistrer_element(appel.session()?, &entree)
            .await
    }

    async fn element_paie_version_enregistrer(
        &self,
        appel: Appel,
        entree: DemandeVersion,
    ) -> Resultat<ElementsPaie> {
        ServicePaie::nouveau(&appel.ouverte()?.noyau)
            .enregistrer_version(appel.session()?, &entree)
            .await
    }

    async fn attribution_enregistrer(
        &self,
        appel: Appel,
        entree: DemandeAttribution,
    ) -> Resultat<ElementsPaie> {
        ServicePaie::nouveau(&appel.ouverte()?.noyau)
            .enregistrer_attribution(appel.session()?, &entree)
            .await
    }

    async fn paie_reglages_enregistrer(
        &self,
        appel: Appel,
        entree: ReglagesPaie,
    ) -> Resultat<ElementsPaie> {
        ServicePaie::nouveau(&appel.ouverte()?.noyau)
            .regler(appel.session()?, &entree)
            .await
    }

    async fn paie_mois_lire(&self, appel: Appel, entree: DemandeMoisPaie) -> Resultat<PaieDuMois> {
        ServicePaieMois::nouveau(&appel.ouverte()?.noyau)
            .lire(appel.session()?, entree.mois)
            .await
    }

    async fn paie_calculer(&self, appel: Appel, entree: DemandeMoisPaie) -> Resultat<PaieDuMois> {
        ServicePaieMois::nouveau(&appel.ouverte()?.noyau)
            .calculer(appel.session()?, entree.mois)
            .await
    }

    async fn paie_saisir(&self, appel: Appel, entree: DemandeSaisiePaie) -> Resultat<PaieDuMois> {
        ServicePaieMois::nouveau(&appel.ouverte()?.noyau)
            .saisir(appel.session()?, &entree)
            .await
    }

    async fn paie_saisie_retirer(
        &self,
        appel: Appel,
        entree: DemandeIdSaisie,
    ) -> Resultat<PaieDuMois> {
        ServicePaieMois::nouveau(&appel.ouverte()?.noyau)
            .retirer_saisie(appel.session()?, &entree)
            .await
    }

    async fn paie_absence_confirmer(
        &self,
        appel: Appel,
        entree: DemandeAbsencePaie,
    ) -> Resultat<PaieDuMois> {
        ServicePaieMois::nouveau(&appel.ouverte()?.noyau)
            .confirmer_absence(appel.session()?, &entree)
            .await
    }

    async fn paie_valider(&self, appel: Appel, entree: DemandeMoisPaie) -> Resultat<PaieDuMois> {
        ServicePaieMois::nouveau(&appel.ouverte()?.noyau)
            .valider(appel.session()?, entree.mois)
            .await
    }

    async fn paiements_personnel_lire(
        &self,
        appel: Appel,
        _entree: (),
    ) -> Resultat<PaiementsPersonnel> {
        ServicePaiementsPersonnel::nouveau(&appel.ouverte()?.noyau)
            .lire(appel.session()?)
            .await
    }

    async fn salaire_payer(
        &self,
        appel: Appel,
        entree: DemandePaiementSalaire,
    ) -> Resultat<PaiementFait> {
        ServicePaiementsPersonnel::nouveau(&appel.ouverte()?.noyau)
            .payer_salaire(appel.session()?, &entree)
            .await
    }

    async fn paie_payer_reste(
        &self,
        appel: Appel,
        entree: DemandeResteDuMois,
    ) -> Resultat<PaiementFait> {
        ServicePaiementsPersonnel::nouveau(&appel.ouverte()?.noyau)
            .payer_reste_du_mois(appel.session()?, &entree)
            .await
    }

    async fn paiement_personnel_annuler(
        &self,
        appel: Appel,
        entree: DemandeAnnulationPaiement,
    ) -> Resultat<PaiementFait> {
        ServicePaiementsPersonnel::nouveau(&appel.ouverte()?.noyau)
            .annuler_paiement(appel.session()?, &entree)
            .await
    }

    async fn avance_accorder(&self, appel: Appel, entree: DemandeAvance) -> Resultat<PaiementFait> {
        ServicePaiementsPersonnel::nouveau(&appel.ouverte()?.noyau)
            .accorder_avance(appel.session()?, &entree)
            .await
    }

    async fn avance_annuler(
        &self,
        appel: Appel,
        entree: DemandeAnnulationAvance,
    ) -> Resultat<PaiementFait> {
        ServicePaiementsPersonnel::nouveau(&appel.ouverte()?.noyau)
            .annuler_avance(appel.session()?, &entree)
            .await
    }

    /// Compose un document de la paie et le rend à l'écran.
    async fn document_paie(
        &self,
        appel: Appel,
        modele: &str,
        entite: &str,
        entite_id: String,
        donnees: serde_json::Value,
    ) -> Resultat<DocumentAffiche> {
        let ouverte = appel.ouverte()?;
        let session = appel.session()?;
        let contexte = ouverte
            .noyau
            .contexte(session.etablissement_id, Some(session));
        let produit = ServiceDocuments::nouveau(&ouverte.noyau, &self.atelier)
            .produire(contexte, modele, entite, &entite_id, donnees)
            .await?;
        Ok(DocumentAffiche {
            pdf: base64(&produit.pdf),
            document: produit.document,
        })
    }

    async fn bulletins_paie_produire(
        &self,
        appel: Appel,
        entree: DemandeMoisPaie,
    ) -> Resultat<DocumentAffiche> {
        let donnees = ServiceDocumentsPaie::nouveau(&appel.ouverte()?.noyau)
            .donnees_bulletins(appel.session()?, entree.mois, None)
            .await?;
        self.document_paie(
            appel,
            "bulletin_paie",
            "paie_mois",
            entree.mois.to_string(),
            donnees,
        )
        .await
    }

    async fn bulletin_paie_produire(
        &self,
        appel: Appel,
        entree: DemandeBulletinPaie,
    ) -> Resultat<DocumentAffiche> {
        let service = ServiceDocumentsPaie::nouveau(&appel.ouverte()?.noyau);
        let (mois, _) = service
            .mois_du_bulletin(appel.session()?, entree.id)
            .await?;
        let donnees = service
            .donnees_bulletins(appel.session()?, mois, Some(entree.id))
            .await?;
        self.document_paie(
            appel,
            "bulletin_paie",
            "bulletin_paie",
            entree.id.vers_chaine(),
            donnees,
        )
        .await
    }

    async fn etat_paie_produire(
        &self,
        appel: Appel,
        entree: DemandeMoisPaie,
    ) -> Resultat<DocumentAffiche> {
        let donnees = ServiceDocumentsPaie::nouveau(&appel.ouverte()?.noyau)
            .donnees_etat_paie(appel.session()?, entree.mois)
            .await?;
        self.document_paie(
            appel,
            "etat_paie",
            "paie_mois",
            entree.mois.to_string(),
            donnees,
        )
        .await
    }

    async fn etat_retenues_produire(
        &self,
        appel: Appel,
        entree: DemandeMoisPaie,
    ) -> Resultat<DocumentAffiche> {
        let donnees = ServiceDocumentsPaie::nouveau(&appel.ouverte()?.noyau)
            .donnees_etat_retenues(appel.session()?, entree.mois)
            .await?;
        self.document_paie(
            appel,
            "etat_retenues",
            "paie_mois",
            entree.mois.to_string(),
            donnees,
        )
        .await
    }

    async fn mes_bulletins_paie_lire(
        &self,
        appel: Appel,
        _entree: (),
    ) -> Resultat<Vec<MonBulletinPaie>> {
        ServiceDocumentsPaie::nouveau(&appel.ouverte()?.noyau)
            .mes_bulletins(appel.session()?)
            .await
    }

    async fn depenses_lire(&self, appel: Appel, entree: DemandeMoisDepenses) -> Resultat<Depenses> {
        ServiceDepenses::nouveau(&appel.ouverte()?.noyau)
            .lire(appel.session()?, entree.mois)
            .await
    }

    async fn depense_saisir(&self, appel: Appel, entree: DemandeDepense) -> Resultat<Depenses> {
        ServiceDepenses::nouveau(&appel.ouverte()?.noyau)
            .saisir(appel.session()?, &entree)
            .await
    }

    async fn depense_decider(
        &self,
        appel: Appel,
        entree: DemandeDecisionDepense,
    ) -> Resultat<Depenses> {
        ServiceDepenses::nouveau(&appel.ouverte()?.noyau)
            .decider(appel.session()?, &entree)
            .await
    }

    async fn depense_payer(&self, appel: Appel, entree: DemandePayerDepense) -> Resultat<Depenses> {
        ServiceDepenses::nouveau(&appel.ouverte()?.noyau)
            .payer(appel.session()?, &entree)
            .await
    }

    async fn depense_annuler(
        &self,
        appel: Appel,
        entree: DemandeAnnulationDepense,
    ) -> Resultat<Depenses> {
        ServiceDepenses::nouveau(&appel.ouverte()?.noyau)
            .annuler(appel.session()?, &entree)
            .await
    }

    async fn depense_piece_joindre(
        &self,
        appel: Appel,
        entree: DemandePieceDepense,
    ) -> Resultat<Depenses> {
        ServiceDepenses::nouveau(&appel.ouverte()?.noyau)
            .joindre(appel.session()?, &entree)
            .await
    }

    async fn depense_piece_enregistrer_sous(
        &self,
        appel: Appel,
        entree: DemandePieceDepenseSous,
    ) -> Resultat<()> {
        ServiceDepenses::nouveau(&appel.ouverte()?.noyau)
            .enregistrer_piece_sous(appel.session()?, &entree)
            .await
    }

    async fn fournisseurs_lire(&self, appel: Appel, _entree: ()) -> Resultat<Fournisseurs> {
        ServiceFournisseurs::nouveau(&appel.ouverte()?.noyau)
            .lire(appel.session()?)
            .await
    }

    async fn fournisseur_enregistrer(
        &self,
        appel: Appel,
        entree: DemandeFournisseur,
    ) -> Resultat<Fournisseurs> {
        ServiceFournisseurs::nouveau(&appel.ouverte()?.noyau)
            .enregistrer_fournisseur(appel.session()?, &entree)
            .await
    }

    async fn facture_fournisseur_enregistrer(
        &self,
        appel: Appel,
        entree: DemandeFacture,
    ) -> Resultat<Fournisseurs> {
        ServiceFournisseurs::nouveau(&appel.ouverte()?.noyau)
            .enregistrer_facture(appel.session()?, &entree)
            .await
    }

    async fn facture_fournisseur_annuler(
        &self,
        appel: Appel,
        entree: DemandeAnnulationFacture,
    ) -> Resultat<Fournisseurs> {
        ServiceFournisseurs::nouveau(&appel.ouverte()?.noyau)
            .annuler_facture(appel.session()?, &entree)
            .await
    }

    async fn comptes_soldes_lire(&self, appel: Appel, _entree: ()) -> Resultat<EtatComptes> {
        ServiceLivres::nouveau(&appel.ouverte()?.noyau)
            .etat(appel.session()?)
            .await
    }

    async fn livre_compte_lire(&self, appel: Appel, entree: DemandeLivre) -> Resultat<LivreCompte> {
        ServiceLivres::nouveau(&appel.ouverte()?.noyau)
            .livre(appel.session()?, &entree)
            .await
    }

    async fn transfert_faire(
        &self,
        appel: Appel,
        entree: DemandeTransfertCompte,
    ) -> Resultat<EtatComptes> {
        ServiceLivres::nouveau(&appel.ouverte()?.noyau)
            .transferer(appel.session()?, &entree)
            .await
    }

    async fn ligne_tresorerie_pointer(
        &self,
        appel: Appel,
        entree: DemandePointage,
    ) -> Resultat<LivreCompte> {
        ServiceLivres::nouveau(&appel.ouverte()?.noyau)
            .pointer(appel.session()?, &entree)
            .await
    }

    async fn compte_du_mode_choisir(
        &self,
        appel: Appel,
        entree: DemandeCompteDuMode,
    ) -> Resultat<EtatComptes> {
        ServiceLivres::nouveau(&appel.ouverte()?.noyau)
            .choisir_compte_du_mode(appel.session()?, &entree)
            .await
    }

    async fn journal_tresorerie_lire(
        &self,
        appel: Appel,
        entree: DemandeJournalTresorerie,
    ) -> Resultat<JournalTresorerie> {
        ServiceJournal::nouveau(&appel.ouverte()?.noyau)
            .journal(appel.session()?, &entree)
            .await
    }

    async fn budget_lire(&self, appel: Appel, entree: DemandeBudget) -> Resultat<Budget> {
        ServiceJournal::nouveau(&appel.ouverte()?.noyau)
            .budget(appel.session()?, &entree)
            .await
    }

    async fn budget_prevoir(&self, appel: Appel, entree: DemandeLigneBudget) -> Resultat<Budget> {
        ServiceJournal::nouveau(&appel.ouverte()?.noyau)
            .prevoir(appel.session()?, &entree)
            .await
    }

    async fn releve_apercu(
        &self,
        appel: Appel,
        entree: DemandeApercuReleve,
    ) -> Resultat<ApercuReleve> {
        ServiceLivres::nouveau(&appel.ouverte()?.noyau)
            .apercu_releve(appel.session()?, &entree)
            .await
    }

    async fn releve_importer(
        &self,
        appel: Appel,
        entree: DemandeImportReleve,
    ) -> Resultat<ResultatReleve> {
        ServiceLivres::nouveau(&appel.ouverte()?.noyau)
            .importer_releve(appel.session()?, &entree)
            .await
    }

    async fn absences_seances_lire(
        &self,
        appel: Appel,
        entree: DemandeReleve,
    ) -> Resultat<ReleveParSeance> {
        ServiceAbsencesSeances::nouveau(&appel.ouverte()?.noyau)
            .lire(appel.session()?, &entree)
            .await
    }

    async fn absences_seances_enregistrer(
        &self,
        appel: Appel,
        entree: DemandeReleveParSeance,
    ) -> Resultat<ReleveParSeance> {
        ServiceAbsencesSeances::nouveau(&appel.ouverte()?.noyau)
            .enregistrer(appel.session()?, &entree)
            .await
    }

    async fn journal_archive_verifier(
        &self,
        appel: Appel,
        entree: DemandeVerificationArchive,
    ) -> Resultat<VerificationArchive> {
        let ouverte = appel.ouverte()?;
        appel.session()?;
        crate::archive_journal::verifier_fichier(&ouverte.cle, Path::new(&entree.chemin))
    }

    async fn responsable_messages_lire(
        &self,
        appel: Appel,
        entree: DemandeMessagesResponsable,
    ) -> Resultat<MessagesResponsable> {
        ServiceMessages::nouveau(&appel.ouverte()?.noyau)
            .lire(appel.session()?, &entree)
            .await
    }

    async fn responsable_consentement_definir(
        &self,
        appel: Appel,
        entree: DemandeConsentement,
    ) -> Resultat<MessagesResponsable> {
        ServiceMessages::nouveau(&appel.ouverte()?.noyau)
            .definir_consentement(appel.session()?, &entree)
            .await
    }

    async fn numero_remettre_a_l_essai(
        &self,
        appel: Appel,
        entree: DemandeRemiseALEssai,
    ) -> Resultat<MessagesResponsable> {
        ServiceMessages::nouveau(&appel.ouverte()?.noyau)
            .remettre_a_l_essai(appel.session()?, &entree)
            .await
    }

    async fn discipline_registre_lire(
        &self,
        appel: Appel,
        _entree: (),
    ) -> Resultat<RegistreDiscipline> {
        ServiceDiscipline::nouveau(&appel.ouverte()?.noyau)
            .registre(appel.session()?)
            .await
    }

    async fn discipline_dossier_lire(
        &self,
        appel: Appel,
        entree: DemandeDossierDiscipline,
    ) -> Resultat<DossierDiscipline> {
        ServiceDiscipline::nouveau(&appel.ouverte()?.noyau)
            .dossier(appel.session()?, &entree)
            .await
    }

    async fn sanction_noter(
        &self,
        appel: Appel,
        entree: DemandeSanction,
    ) -> Resultat<DossierDiscipline> {
        ServiceDiscipline::nouveau(&appel.ouverte()?.noyau)
            .sanctionner(appel.session()?, &entree)
            .await
    }

    async fn sanction_annuler(
        &self,
        appel: Appel,
        entree: DemandeAnnulationSanction,
    ) -> Resultat<DossierDiscipline> {
        ServiceDiscipline::nouveau(&appel.ouverte()?.noyau)
            .annuler(appel.session()?, &entree)
            .await
    }

    async fn recettes_lire(
        &self,
        appel: Appel,
        entree: DemandeMoisRecettes,
    ) -> Resultat<RecettesDuMois> {
        ServiceRecettes::nouveau(&appel.ouverte()?.noyau)
            .lire(appel.session()?, entree.mois)
            .await
    }

    async fn recette_saisir(
        &self,
        appel: Appel,
        entree: DemandeRecette,
    ) -> Resultat<RecettesDuMois> {
        ServiceRecettes::nouveau(&appel.ouverte()?.noyau)
            .saisir(appel.session()?, &entree)
            .await
    }

    async fn recette_annuler(
        &self,
        appel: Appel,
        entree: DemandeAnnulationRecette,
    ) -> Resultat<RecettesDuMois> {
        ServiceRecettes::nouveau(&appel.ouverte()?.noyau)
            .annuler(appel.session()?, &entree)
            .await
    }

    async fn cantine_lire(&self, appel: Appel, entree: DemandeCantine) -> Resultat<Cantine> {
        ServiceCantine::nouveau(&appel.ouverte()?.noyau)
            .lire(appel.session()?, &entree)
            .await
    }

    async fn repas_pointer(&self, appel: Appel, entree: DemandeRepas) -> Resultat<Cantine> {
        ServiceCantine::nouveau(&appel.ouverte()?.noyau)
            .pointer(appel.session()?, &entree)
            .await
    }

    async fn liste_cantine_produire(
        &self,
        appel: Appel,
        entree: DemandeCantine,
    ) -> Resultat<DocumentAffiche> {
        let ouverte = appel.ouverte()?;
        let session = appel.session()?;
        let donnees = ServiceCantine::nouveau(&ouverte.noyau)
            .donnees_liste(session, &entree)
            .await?;
        let contexte = ouverte
            .noyau
            .contexte(session.etablissement_id, Some(session));
        let produit = ServiceDocuments::nouveau(&ouverte.noyau, &self.atelier)
            .produire(contexte, "liste_cantine", "cantine", &entree.date, donnees)
            .await?;
        Ok(DocumentAffiche {
            pdf: base64(&produit.pdf),
            document: produit.document,
        })
    }

    async fn emploi_du_temps_lire(&self, appel: Appel, _entree: ()) -> Resultat<EmploiDuTemps> {
        ServiceEmploiDuTemps::nouveau(&appel.ouverte()?.noyau)
            .lire(appel.session()?)
            .await
    }

    async fn journee_regler(
        &self,
        appel: Appel,
        entree: gesco_core::pedagogie::emploi_du_temps::JourneeCycle,
    ) -> Resultat<EmploiDuTemps> {
        ServiceEmploiDuTemps::nouveau(&appel.ouverte()?.noyau)
            .regler_journee(appel.session()?, &entree)
            .await
    }

    async fn seance_placer(&self, appel: Appel, entree: DemandeSeance) -> Resultat<EmploiDuTemps> {
        ServiceEmploiDuTemps::nouveau(&appel.ouverte()?.noyau)
            .placer(appel.session()?, &entree)
            .await
    }

    async fn seance_retirer(
        &self,
        appel: Appel,
        entree: DemandeRetraitSeance,
    ) -> Resultat<EmploiDuTemps> {
        ServiceEmploiDuTemps::nouveau(&appel.ouverte()?.noyau)
            .retirer(appel.session()?, &entree)
            .await
    }

    async fn emploi_classe_produire(
        &self,
        appel: Appel,
        entree: DemandeEmploiClasse,
    ) -> Resultat<DocumentAffiche> {
        let ouverte = appel.ouverte()?;
        let session = appel.session()?;
        let donnees = ServiceEmploiDuTemps::nouveau(&ouverte.noyau)
            .donnees_classe(session, &entree)
            .await?;
        let contexte = ouverte
            .noyau
            .contexte(session.etablissement_id, Some(session));
        let produit = ServiceDocuments::nouveau(&ouverte.noyau, &self.atelier)
            .produire(
                contexte,
                "emploi_du_temps",
                "classe",
                &entree.classe_id.vers_chaine(),
                donnees,
            )
            .await?;
        Ok(DocumentAffiche {
            pdf: base64(&produit.pdf),
            document: produit.document,
        })
    }

    async fn emploi_enseignant_produire(
        &self,
        appel: Appel,
        entree: DemandeEmploiEnseignant,
    ) -> Resultat<DocumentAffiche> {
        let ouverte = appel.ouverte()?;
        let session = appel.session()?;
        let donnees = ServiceEmploiDuTemps::nouveau(&ouverte.noyau)
            .donnees_enseignant(session, &entree)
            .await?;
        let contexte = ouverte
            .noyau
            .contexte(session.etablissement_id, Some(session));
        let produit = ServiceDocuments::nouveau(&ouverte.noyau, &self.atelier)
            .produire(
                contexte,
                "emploi_du_temps",
                "enseignant",
                &entree.enseignant_id.vers_chaine(),
                donnees,
            )
            .await?;
        Ok(DocumentAffiche {
            pdf: base64(&produit.pdf),
            document: produit.document,
        })
    }

    async fn tresorerie_lire(&self, appel: Appel, _entree: ()) -> Resultat<Tresorerie> {
        ServiceTresorerie::nouveau(&appel.ouverte()?.noyau)
            .lire(appel.session()?)
            .await
    }

    async fn categorie_tresorerie_enregistrer(
        &self,
        appel: Appel,
        entree: DemandeCategorieTresorerie,
    ) -> Resultat<Tresorerie> {
        ServiceTresorerie::nouveau(&appel.ouverte()?.noyau)
            .enregistrer_categorie(appel.session()?, &entree)
            .await
    }

    async fn compte_tresorerie_enregistrer(
        &self,
        appel: Appel,
        entree: DemandeCompteTresorerie,
    ) -> Resultat<Tresorerie> {
        ServiceTresorerie::nouveau(&appel.ouverte()?.noyau)
            .enregistrer_compte(appel.session()?, &entree)
            .await
    }

    async fn tresorerie_reglages_enregistrer(
        &self,
        appel: Appel,
        entree: ReglagesTresorerie,
    ) -> Resultat<Tresorerie> {
        ServiceTresorerie::nouveau(&appel.ouverte()?.noyau)
            .regler(appel.session()?, &entree)
            .await
    }

    async fn mon_bulletin_paie_produire(
        &self,
        appel: Appel,
        entree: DemandeBulletinPaie,
    ) -> Resultat<DocumentAffiche> {
        let (_, donnees) = ServiceDocumentsPaie::nouveau(&appel.ouverte()?.noyau)
            .donnees_mon_bulletin(appel.session()?, entree.id)
            .await?;
        self.document_paie(
            appel,
            "bulletin_paie",
            "bulletin_paie",
            entree.id.vers_chaine(),
            donnees,
        )
        .await
    }
}
