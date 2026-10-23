//! Événements métier de l'`outbox` (ADR-009, §25.2, spécification §3).
//!
//! Un événement est un **fait accompli**, nommé au participe passé, versionné, à charge utile
//! minimale et auto-suffisante. Son type est une donnée du noyau ; aucune chaîne libre.

use serde::{Deserialize, Serialize};

use super::etablissement::Etablissement;
use super::utilisateur::{Poste, Utilisateur};
use crate::commun::{Horodatage, Id};

/// Types d'événements connus. Le catalogue se remplit phase par phase (spécification §3.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TypeEvenement {
    /// Un compte utilisateur a été créé.
    UtilisateurCree,
    /// Un rôle a été attribué à un utilisateur.
    RoleAttribue,
    /// Une sauvegarde a été produite et vérifiée.
    SauvegardeEffectuee,
    /// Un document a été composé et enregistré (étape 10).
    DocumentCompose,
    /// Une sauvegarde a été restaurée sur ce poste (étape 11).
    SauvegardeRestauree,
    /// La phrase de secours a été remplacée ; l'ancienne n'ouvre plus les sauvegardes suivantes.
    PhraseSecoursRegeneree,
    /// Une licence signée a été installée sur le poste (étape 13).
    LicenceInstallee,
    /// Les permissions d'un rôle ont changé (SPEC_PHASE_1 §5).
    RoleRecompose,
    /// Un rôle personnalisé a été créé.
    RoleCree,
    /// Un rôle personnalisé a été supprimé.
    RoleSupprime,
    /// Un rôle a été retiré à un utilisateur.
    RoleRetire,
    /// Un compte a été désactivé ; ses sessions sont fermées.
    CompteDesactive,
    /// Un compte désactivé a été réactivé.
    CompteReactive,
    /// Un administrateur a réinitialisé le mot de passe d'un compte.
    MotDePasseReinitialise,
    /// La fiche de l'établissement a changé.
    FicheEtablissementRectifiee,
    /// Le logo de l'établissement a été posé, remplacé ou retiré.
    LogoEtablissementRemplace,
    /// Une année scolaire a été créée en préparation.
    AnneeScolairePreparee,
    /// Une année scolaire est devenue active et courante.
    AnneeScolaireOuverte,
    /// Une année scolaire est clôturée : elle passe en lecture seule (SPEC_PHASE_10 §4).
    AnneeScolaireCloturee,
    /// Une année clôturée est rouverte, avec un motif.
    AnneeScolaireRouverte,
    /// La décision de fin d'année d'un élève est posée ou changée (SPEC_PHASE_10 §3).
    DecisionFinAnneePosee,
    /// Une classe a été créée dans une année (SPEC_PHASE_2 §3).
    ClasseCreee,
    /// Une fiche élève a été créée, son matricule attribué (SPEC_PHASE_2 §4).
    FicheEleveCreee,
    /// L'identité d'un élève a changé.
    FicheEleveRectifiee,
    /// Un responsable a été lié à un élève.
    ResponsableRattache,
    /// Le lien entre un élève et un responsable a été retiré.
    ResponsableDetache,
    /// Un brouillon d'inscription existe (SPEC_PHASE_2 §5).
    InscriptionPreparee,
    /// Une inscription est validée ; la facturation la consommera en phase 3.
    InscriptionValidee,
    /// Une inscription a été abandonnée, avec son motif.
    InscriptionAbandonnee,
    /// Un élève a changé de classe en cours d'année (Q-35).
    ClasseChangee,
    /// Deux fiches élèves n'en font plus qu'une (SPEC_PHASE_2 §8).
    EleveFusionne,
    /// Un fichier d'élèves a été appliqué : fiches, responsables et brouillons créés (§10).
    ImportElevesApplique,
    /// Une grille de frais est en vigueur pour son année (SPEC_PHASE_3 §3).
    GrilleActivee,
    /// Une facture et son échéancier sont nés de la validation d'une inscription (§4.1).
    FactureEmise,
    /// Une facture est annulée avec son inscription (§4.3).
    FactureAnnulee,
    /// Le parent a changé de plan de paiement (§3.1 bis).
    PlanPaiementChange,
    /// Les échéances d'un élève passent à un autre jour du mois.
    EcheancierDecale,
    /// Une remise est proposée à la direction (§5).
    RemiseProposee,
    /// Une remise est accordée ; elle réduit l'échéancier.
    RemiseAccordee,
    /// Une remise est refusée, avec son motif.
    RemiseRefusee,
    /// Une remise accordée est annulée par contre-passation.
    RemiseAnnulee,
    /// Un caissier a ouvert sa caisse avec son fonds (§6).
    SessionCaisseOuverte,
    /// Un caissier a clôturé sa caisse avec le total compté.
    SessionCaisseCloturee,
    /// Une session clôturée est visée par la direction ou le comptable.
    SessionCaisseRapprochee,
    /// Un versement est encaissé et imputé ; son reçu est numéroté (§7). Servira au SMS de
    /// confirmation de la phase 5.
    VersementEncaisse,
    /// Un versement est annulé avec son motif : ses imputations sont contre-passées ; après la
    /// clôture de sa session, une sortie de régularisation passe dans la caisse du jour (§8).
    VersementAnnule,
    /// Une matière est créée ou modifiée (SPEC_PHASE_4 §2.1).
    MatiereEnregistree,
    /// Le programme d'un niveau est enregistré ou repris de l'année précédente (§2.2).
    ProgrammeEnregistre,
    /// Les périodes d'un niveau sont définies ou corrigées (§2.3).
    CalendrierDefini,
    /// Une nouvelle version de la règle d'évaluation d'un cycle est enregistrée (§2.4, RC-4).
    RegleEvaluationEnregistree,
    /// Un enseignant est créé ou modifié (SPEC_PHASE_4 §3.1).
    EnseignantEnregistre,
    /// Les affectations d'une classe changent : ajout, matière confiée, retrait, maître (§3.2).
    AffectationModifiee,
    /// Une évaluation est créée (SPEC_PHASE_4 §4.1).
    EvaluationCreee,
    /// La note d'un élève est saisie, corrigée ou retirée (§4.3).
    NoteSaisie,
    /// Une évaluation complète est validée par la direction (§4.4).
    EvaluationValidee,
    /// Une évaluation validée est rouverte, avec motif (§4.4).
    EvaluationDeverrouillee,
    /// Une grille de notes remplie par l'enseignant est importée (SPEC_PHASE_4 §4.5).
    GrilleNotesImportee,
    /// Le relevé des absences et retards d'une classe pour un jour est enregistré (SPEC_PHASE_4 §6).
    AbsenceRelevee,
    /// Une absence ou un retard est justifié (§6).
    AbsenceJustifiee,
    /// Les bulletins d'une classe sont calculés ou recalculés pour une période (SPEC_PHASE_4 §7.1).
    ResultatCalcule,
    /// Une appréciation est écrite sur un bulletin (§7.2).
    AppreciationSaisie,
    /// Un bulletin est validé par la direction (§7.3).
    BulletinValide,
    /// Un bulletin est publié : il ne change plus (§7.3) ; la phase 5 en avertira la famille.
    BulletinPublie,
    /// Une version publiée est remplacée par sa correction publiée (§7.3).
    BulletinCorrige,
    /// La moyenne d'une période suivie dans une autre école est reprise de son relevé (Q-28).
    MoyenneReprise,
    /// Un élève part dans une autre école, son certificat de transfert délivré (SPEC_PHASE_4 §9).
    InscriptionTransferee,
    /// Un élève est exclu, temporairement ou définitivement (réponse du 13 octobre 2026).
    EleveExclu,
    /// Une exclusion temporaire est levée avant son terme, ou une exclusion définitive révoquée.
    ExclusionLevee,
    /// Un membre du personnel est enregistré ou sa fiche modifiée (SPEC_PHASE_6 §2).
    MembreEnregistre,
    /// Un membre du personnel quitte l'école, ou y revient.
    MembreParti,
    /// Une catégorie ou une fonction du personnel est ajoutée, renommée ou retirée.
    ReferentielPersonnelModifie,
    /// Une pièce est ajoutée au dossier d'un membre du personnel.
    PieceDossierAjoutee,
    /// Une pièce est retirée du dossier d'un membre du personnel.
    PieceDossierRetiree,
    /// Un engagement du personnel est conclu (SPEC_PHASE_6 §3).
    EngagementConclu,
    /// Un engagement se termine : à sa fin, plus tôt, ou remplacé par un nouveau.
    EngagementTermine,
    /// Un engagement est rompu, avec un motif.
    EngagementRompu,
    /// Des heures de vacation d'un mois sont saisies (SPEC_PHASE_6 §4).
    EtatHeuresSaisi,
    /// Des heures de vacation d'un mois sont visées par la direction.
    EtatHeuresVise,
    /// Des heures visées reviennent en saisie, avec un motif.
    EtatHeuresDeverrouille,
    /// Une absence ou un retard du personnel est noté (SPEC_PHASE_6 §5).
    AbsencePersonnelNotee,
    /// Une absence du personnel est justifiée.
    AbsencePersonnelJustifiee,
    /// Une absence du personnel notée par erreur est annulée.
    AbsencePersonnelAnnulee,
    /// Un congé est demandé (SPEC_PHASE_6 §6).
    CongeDemande,
    /// Un congé est accordé par la direction.
    CongeAccorde,
    /// Un congé est refusé, avec un motif.
    CongeRefuse,
    /// Un congé accordé est annulé avant son début.
    CongeAnnule,
    /// Un élément de paie, une version de son calcul ou une attribution change.
    ElementPaieModifie,
    /// La paie d'un mois est calculée ou refaite (SPEC_PHASE_7 §5).
    PaieCalculee,
    /// La paie d'un mois est validée : ses bulletins sont numérotés et figés.
    PaieValidee,
    /// Un salaire est payé, en tout ou en partie : la phase 8 en fera une dépense (Q-06).
    SalairePaye,
    /// Une avance sur salaire est accordée et versée.
    AvanceAccordee,
    /// Un paiement du personnel est annulé, avec un motif.
    PaiementPersonnelAnnule,
    /// Une catégorie de recette ou de dépense est créée, renommée, retirée ou remise.
    CategorieTresorerieModifiee,
    /// Un compte de trésorerie est ouvert, modifié ou fermé.
    CompteTresorerieModifie,
    /// Une dépense est saisie (SPEC_PHASE_8 §3).
    DepenseSaisie,
    /// Une dépense au-dessus du seuil est accordée ou refusée.
    DepenseDecidee,
    /// Une dépense est payée d'un compte.
    DepensePayee,
    /// Une dépense est annulée, avec un motif.
    DepenseAnnulee,
    /// Un fournisseur est enregistré, modifié ou retiré (SPEC_PHASE_8 §4).
    FournisseurModifie,
    /// Une facture de fournisseur est enregistrée.
    FactureFournisseurEnregistree,
    /// Une facture de fournisseur est annulée, avec un motif.
    FactureFournisseurAnnulee,
    /// De l'argent passe d'un compte de l'école à un autre (SPEC_PHASE_8 §5).
    TransfertTresorerieEnregistre,
    /// Une ligne du livre d'un compte est cochée ou décochée contre le relevé.
    LigneTresoreriePointee,
    /// Le montant prévu d'une catégorie au budget de l'année change (SPEC_PHASE_8 §7).
    BudgetPrevu,
    /// La journée d'un cycle est réglée (SPEC_PHASE_8 §8).
    JourneeCycleReglee,
    /// Une séance de l'emploi du temps est placée, déplacée ou retirée.
    SeancePlacee,
    /// Le repas d'un abonné de la cantine est pointé ou décoché (SPEC_PHASE_8 §9).
    RepasPointe,
    /// Une recette hors des versements des parents est reçue (SPEC_VERSION_1_1 §1).
    RecetteEnregistree,
    /// Une autre recette est annulée, avec un motif.
    RecetteAnnulee,
    /// Une sanction est notée au dossier de discipline d'un élève (SPEC_VERSION_1_1 §1).
    SanctionNotee,
    /// Une sanction est annulée, avec un motif.
    SanctionAnnulee,
    /// Le journal d'audit d'une année est archivé dans un fichier signé (blueprint §12.4).
    JournalArchive,
    /// Le relevé par séance d'une classe est enregistré pour un jour (SPEC_VERSION_1_1 §1, T-71).
    ReleveParSeanceEnregistre,
    /// Le relevé d'un compte est importé ; les lignes retrouvées sont cochées (T-72).
    ReleveImporte,
    /// Une liste du personnel est importée (SPEC_VERSION_1_1 §1, étape 8).
    PersonnelImporte,
    /// Un responsable accepte ou refuse un type de message (SPEC_PHASE_5 §2, blueprint §25.6).
    ConsentementModifie,
    /// Un numéro injoignable ou invalide est remis à l'essai : il reçoit de nouveau.
    JoignabiliteReinitialisee,
    /// Un message est mis dans la file des SMS (SPEC_PHASE_5 §3).
    MessageMisEnFile,
    /// Une personne décide d'envoyer un message préparé, son texte relu.
    MessageDecide,
    /// Un message qui n'est pas parti est annulé, avec un motif.
    MessageAnnule,
    /// Un message échoué ou expiré est remis dans la file.
    MessageRenvoye,
    /// L'école reformule le modèle d'un type de message.
    ModeleMessageModifie,
    /// L'école choisit le port de sa clé modem (SPEC_PHASE_5 §4).
    PortSmsChoisi,
    /// La file des SMS, arrêtée faute de crédit, repart à la main.
    FileSmsReprise,
    /// Un bulletin publié est remis à un responsable, à l'élève ou à un tiers (SPEC_PHASE_5 §6).
    DocumentRemis,
    /// Un bulletin remis est signé.
    DocumentSigne,
    /// L'avis d'un bulletin non retiré est renvoyé.
    AvisBulletinRenvoye,
    /// Un responsable est convoqué au sujet d'un élève (SPEC_PHASE_5 §7).
    ConvocationEmise,
    /// L'école note ce qu'elle sait de la réception d'une convocation.
    ConvocationAccusee,
    /// Le rendez-vous d'une convocation est déplacé.
    ConvocationReportee,
    /// Une convocation a sa suite : honorée ou non honorée.
    ConvocationCloturee,
    /// Une convocation est annulée, avec un motif.
    ConvocationAnnulee,
    /// Un code d'accès est remis à un responsable, l'ancien révoqué (SPEC_PHASE_5 §9).
    CodeAccesRemis,
    /// Le code d'accès d'un responsable est révoqué.
    CodeAccesRevoque,
    /// Un SMS est reçu : consultation servie ou refusée.
    SmsRecu,
}

impl TypeEvenement {
    /// Tous les types connus.
    pub const TOUS: &'static [Self] = &[
        Self::UtilisateurCree,
        Self::RoleAttribue,
        Self::SauvegardeEffectuee,
        Self::DocumentCompose,
        Self::SauvegardeRestauree,
        Self::PhraseSecoursRegeneree,
        Self::LicenceInstallee,
        Self::RoleRecompose,
        Self::RoleCree,
        Self::RoleSupprime,
        Self::RoleRetire,
        Self::CompteDesactive,
        Self::CompteReactive,
        Self::MotDePasseReinitialise,
        Self::FicheEtablissementRectifiee,
        Self::LogoEtablissementRemplace,
        Self::AnneeScolairePreparee,
        Self::AnneeScolaireOuverte,
        Self::AnneeScolaireCloturee,
        Self::AnneeScolaireRouverte,
        Self::DecisionFinAnneePosee,
        Self::ClasseCreee,
        Self::FicheEleveCreee,
        Self::FicheEleveRectifiee,
        Self::ResponsableRattache,
        Self::ResponsableDetache,
        Self::InscriptionPreparee,
        Self::InscriptionValidee,
        Self::InscriptionAbandonnee,
        Self::ClasseChangee,
        Self::EleveFusionne,
        Self::ImportElevesApplique,
        Self::GrilleActivee,
        Self::FactureEmise,
        Self::FactureAnnulee,
        Self::PlanPaiementChange,
        Self::EcheancierDecale,
        Self::RemiseProposee,
        Self::RemiseAccordee,
        Self::RemiseRefusee,
        Self::RemiseAnnulee,
        Self::SessionCaisseOuverte,
        Self::SessionCaisseCloturee,
        Self::SessionCaisseRapprochee,
        Self::VersementEncaisse,
        Self::VersementAnnule,
        Self::MatiereEnregistree,
        Self::ProgrammeEnregistre,
        Self::CalendrierDefini,
        Self::RegleEvaluationEnregistree,
        Self::EnseignantEnregistre,
        Self::AffectationModifiee,
        Self::EvaluationCreee,
        Self::NoteSaisie,
        Self::EvaluationValidee,
        Self::EvaluationDeverrouillee,
        Self::GrilleNotesImportee,
        Self::AbsenceRelevee,
        Self::AbsenceJustifiee,
        Self::ResultatCalcule,
        Self::AppreciationSaisie,
        Self::BulletinValide,
        Self::BulletinPublie,
        Self::BulletinCorrige,
        Self::MoyenneReprise,
        Self::InscriptionTransferee,
        Self::EleveExclu,
        Self::ExclusionLevee,
        Self::MembreEnregistre,
        Self::MembreParti,
        Self::ReferentielPersonnelModifie,
        Self::PieceDossierAjoutee,
        Self::PieceDossierRetiree,
        Self::EngagementConclu,
        Self::EngagementTermine,
        Self::EngagementRompu,
        Self::EtatHeuresSaisi,
        Self::EtatHeuresVise,
        Self::EtatHeuresDeverrouille,
        Self::AbsencePersonnelNotee,
        Self::AbsencePersonnelJustifiee,
        Self::AbsencePersonnelAnnulee,
        Self::CongeDemande,
        Self::CongeAccorde,
        Self::CongeRefuse,
        Self::CongeAnnule,
        Self::ElementPaieModifie,
        Self::PaieCalculee,
        Self::PaieValidee,
        Self::SalairePaye,
        Self::AvanceAccordee,
        Self::PaiementPersonnelAnnule,
        Self::CategorieTresorerieModifiee,
        Self::CompteTresorerieModifie,
        Self::DepenseSaisie,
        Self::DepenseDecidee,
        Self::DepensePayee,
        Self::DepenseAnnulee,
        Self::FournisseurModifie,
        Self::FactureFournisseurEnregistree,
        Self::FactureFournisseurAnnulee,
        Self::TransfertTresorerieEnregistre,
        Self::LigneTresoreriePointee,
        Self::BudgetPrevu,
        Self::JourneeCycleReglee,
        Self::SeancePlacee,
        Self::RepasPointe,
        Self::RecetteEnregistree,
        Self::RecetteAnnulee,
        Self::SanctionNotee,
        Self::SanctionAnnulee,
        Self::JournalArchive,
        Self::ReleveParSeanceEnregistre,
        Self::ReleveImporte,
        Self::PersonnelImporte,
        Self::ConsentementModifie,
        Self::JoignabiliteReinitialisee,
        Self::MessageMisEnFile,
        Self::MessageDecide,
        Self::MessageAnnule,
        Self::MessageRenvoye,
        Self::ModeleMessageModifie,
        Self::PortSmsChoisi,
        Self::FileSmsReprise,
        Self::DocumentRemis,
        Self::DocumentSigne,
        Self::AvisBulletinRenvoye,
        Self::ConvocationEmise,
        Self::ConvocationAccusee,
        Self::ConvocationReportee,
        Self::ConvocationCloturee,
        Self::ConvocationAnnulee,
        Self::CodeAccesRemis,
        Self::CodeAccesRevoque,
        Self::SmsRecu,
    ];

    /// Nom persisté, identique au nom Rust.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::UtilisateurCree => "UtilisateurCree",
            Self::RoleAttribue => "RoleAttribue",
            Self::SauvegardeEffectuee => "SauvegardeEffectuee",
            Self::DocumentCompose => "DocumentCompose",
            Self::SauvegardeRestauree => "SauvegardeRestauree",
            Self::PhraseSecoursRegeneree => "PhraseSecoursRegeneree",
            Self::LicenceInstallee => "LicenceInstallee",
            Self::RoleRecompose => "RoleRecompose",
            Self::RoleCree => "RoleCree",
            Self::RoleSupprime => "RoleSupprime",
            Self::RoleRetire => "RoleRetire",
            Self::CompteDesactive => "CompteDesactive",
            Self::CompteReactive => "CompteReactive",
            Self::MotDePasseReinitialise => "MotDePasseReinitialise",
            Self::FicheEtablissementRectifiee => "FicheEtablissementRectifiee",
            Self::LogoEtablissementRemplace => "LogoEtablissementRemplace",
            Self::AnneeScolairePreparee => "AnneeScolairePreparee",
            Self::AnneeScolaireOuverte => "AnneeScolaireOuverte",
            Self::AnneeScolaireCloturee => "AnneeScolaireCloturee",
            Self::AnneeScolaireRouverte => "AnneeScolaireRouverte",
            Self::DecisionFinAnneePosee => "DecisionFinAnneePosee",
            Self::ClasseCreee => "ClasseCreee",
            Self::FicheEleveCreee => "FicheEleveCreee",
            Self::FicheEleveRectifiee => "FicheEleveRectifiee",
            Self::ResponsableRattache => "ResponsableRattache",
            Self::ResponsableDetache => "ResponsableDetache",
            Self::InscriptionPreparee => "InscriptionPreparee",
            Self::InscriptionValidee => "InscriptionValidee",
            Self::InscriptionAbandonnee => "InscriptionAbandonnee",
            Self::ClasseChangee => "ClasseChangee",
            Self::EleveFusionne => "EleveFusionne",
            Self::ImportElevesApplique => "ImportElevesApplique",
            Self::GrilleActivee => "GrilleActivee",
            Self::FactureEmise => "FactureEmise",
            Self::FactureAnnulee => "FactureAnnulee",
            Self::PlanPaiementChange => "PlanPaiementChange",
            Self::EcheancierDecale => "EcheancierDecale",
            Self::RemiseProposee => "RemiseProposee",
            Self::RemiseAccordee => "RemiseAccordee",
            Self::RemiseRefusee => "RemiseRefusee",
            Self::RemiseAnnulee => "RemiseAnnulee",
            Self::SessionCaisseOuverte => "SessionCaisseOuverte",
            Self::SessionCaisseCloturee => "SessionCaisseCloturee",
            Self::SessionCaisseRapprochee => "SessionCaisseRapprochee",
            Self::VersementEncaisse => "VersementEncaisse",
            Self::VersementAnnule => "VersementAnnule",
            Self::MatiereEnregistree => "MatiereEnregistree",
            Self::ProgrammeEnregistre => "ProgrammeEnregistre",
            Self::CalendrierDefini => "CalendrierDefini",
            Self::RegleEvaluationEnregistree => "RegleEvaluationEnregistree",
            Self::EnseignantEnregistre => "EnseignantEnregistre",
            Self::AffectationModifiee => "AffectationModifiee",
            Self::EvaluationCreee => "EvaluationCreee",
            Self::NoteSaisie => "NoteSaisie",
            Self::EvaluationValidee => "EvaluationValidee",
            Self::EvaluationDeverrouillee => "EvaluationDeverrouillee",
            Self::GrilleNotesImportee => "GrilleNotesImportee",
            Self::AbsenceRelevee => "AbsenceRelevee",
            Self::AbsenceJustifiee => "AbsenceJustifiee",
            Self::ResultatCalcule => "ResultatCalcule",
            Self::AppreciationSaisie => "AppreciationSaisie",
            Self::BulletinValide => "BulletinValide",
            Self::BulletinPublie => "BulletinPublie",
            Self::BulletinCorrige => "BulletinCorrige",
            Self::MoyenneReprise => "MoyenneReprise",
            Self::InscriptionTransferee => "InscriptionTransferee",
            Self::EleveExclu => "EleveExclu",
            Self::ExclusionLevee => "ExclusionLevee",
            Self::MembreEnregistre => "MembreEnregistre",
            Self::MembreParti => "MembreParti",
            Self::ReferentielPersonnelModifie => "ReferentielPersonnelModifie",
            Self::PieceDossierAjoutee => "PieceDossierAjoutee",
            Self::PieceDossierRetiree => "PieceDossierRetiree",
            Self::EngagementConclu => "EngagementConclu",
            Self::EngagementTermine => "EngagementTermine",
            Self::EngagementRompu => "EngagementRompu",
            Self::EtatHeuresSaisi => "EtatHeuresSaisi",
            Self::EtatHeuresVise => "EtatHeuresVise",
            Self::EtatHeuresDeverrouille => "EtatHeuresDeverrouille",
            Self::AbsencePersonnelNotee => "AbsencePersonnelNotee",
            Self::AbsencePersonnelJustifiee => "AbsencePersonnelJustifiee",
            Self::AbsencePersonnelAnnulee => "AbsencePersonnelAnnulee",
            Self::CongeDemande => "CongeDemande",
            Self::CongeAccorde => "CongeAccorde",
            Self::CongeRefuse => "CongeRefuse",
            Self::CongeAnnule => "CongeAnnule",
            Self::ElementPaieModifie => "ElementPaieModifie",
            Self::PaieCalculee => "PaieCalculee",
            Self::PaieValidee => "PaieValidee",
            Self::SalairePaye => "SalairePaye",
            Self::AvanceAccordee => "AvanceAccordee",
            Self::PaiementPersonnelAnnule => "PaiementPersonnelAnnule",
            Self::CategorieTresorerieModifiee => "CategorieTresorerieModifiee",
            Self::CompteTresorerieModifie => "CompteTresorerieModifie",
            Self::DepenseSaisie => "DepenseSaisie",
            Self::DepenseDecidee => "DepenseDecidee",
            Self::DepensePayee => "DepensePayee",
            Self::DepenseAnnulee => "DepenseAnnulee",
            Self::FournisseurModifie => "FournisseurModifie",
            Self::FactureFournisseurEnregistree => "FactureFournisseurEnregistree",
            Self::FactureFournisseurAnnulee => "FactureFournisseurAnnulee",
            Self::TransfertTresorerieEnregistre => "TransfertTresorerieEnregistre",
            Self::LigneTresoreriePointee => "LigneTresoreriePointee",
            Self::BudgetPrevu => "BudgetPrevu",
            Self::JourneeCycleReglee => "JourneeCycleReglee",
            Self::SeancePlacee => "SeancePlacee",
            Self::RepasPointe => "RepasPointe",
            Self::RecetteEnregistree => "RecetteEnregistree",
            Self::RecetteAnnulee => "RecetteAnnulee",
            Self::SanctionNotee => "SanctionNotee",
            Self::SanctionAnnulee => "SanctionAnnulee",
            Self::JournalArchive => "JournalArchive",
            Self::ReleveParSeanceEnregistre => "ReleveParSeanceEnregistre",
            Self::ReleveImporte => "ReleveImporte",
            Self::PersonnelImporte => "PersonnelImporte",
            Self::ConsentementModifie => "ConsentementModifie",
            Self::JoignabiliteReinitialisee => "JoignabiliteReinitialisee",
            Self::MessageMisEnFile => "MessageMisEnFile",
            Self::MessageDecide => "MessageDecide",
            Self::MessageAnnule => "MessageAnnule",
            Self::MessageRenvoye => "MessageRenvoye",
            Self::ModeleMessageModifie => "ModeleMessageModifie",
            Self::PortSmsChoisi => "PortSmsChoisi",
            Self::FileSmsReprise => "FileSmsReprise",
            Self::DocumentRemis => "DocumentRemis",
            Self::DocumentSigne => "DocumentSigne",
            Self::AvisBulletinRenvoye => "AvisBulletinRenvoye",
            Self::ConvocationEmise => "ConvocationEmise",
            Self::ConvocationAccusee => "ConvocationAccusee",
            Self::ConvocationReportee => "ConvocationReportee",
            Self::ConvocationCloturee => "ConvocationCloturee",
            Self::ConvocationAnnulee => "ConvocationAnnulee",
            Self::CodeAccesRemis => "CodeAccesRemis",
            Self::CodeAccesRevoque => "CodeAccesRevoque",
            Self::SmsRecu => "SmsRecu",
        }
    }

    /// Version courante du schéma de la charge utile. Un événement publié ne change jamais de
    /// forme : une évolution incrémente la version et conserve la lecture des anciennes.
    pub const fn version(&self) -> u16 {
        match self {
            Self::UtilisateurCree
            | Self::RoleAttribue
            | Self::SauvegardeEffectuee
            | Self::DocumentCompose
            | Self::SauvegardeRestauree
            | Self::PhraseSecoursRegeneree
            | Self::LicenceInstallee
            | Self::RoleRecompose
            | Self::RoleCree
            | Self::RoleSupprime
            | Self::RoleRetire
            | Self::CompteDesactive
            | Self::CompteReactive
            | Self::MotDePasseReinitialise
            | Self::FicheEtablissementRectifiee
            | Self::LogoEtablissementRemplace
            | Self::AnneeScolairePreparee
            | Self::AnneeScolaireOuverte
            | Self::AnneeScolaireCloturee
            | Self::AnneeScolaireRouverte
            | Self::DecisionFinAnneePosee
            | Self::ClasseCreee
            | Self::FicheEleveCreee
            | Self::FicheEleveRectifiee
            | Self::ResponsableRattache
            | Self::ResponsableDetache
            | Self::InscriptionPreparee
            | Self::InscriptionValidee
            | Self::InscriptionAbandonnee
            | Self::ClasseChangee
            | Self::EleveFusionne
            | Self::ImportElevesApplique
            | Self::GrilleActivee
            | Self::FactureEmise
            | Self::FactureAnnulee
            | Self::PlanPaiementChange
            | Self::EcheancierDecale
            | Self::RemiseProposee
            | Self::RemiseAccordee
            | Self::RemiseRefusee
            | Self::RemiseAnnulee
            | Self::SessionCaisseOuverte
            | Self::SessionCaisseCloturee
            | Self::SessionCaisseRapprochee
            | Self::VersementEncaisse
            | Self::VersementAnnule
            | Self::MatiereEnregistree
            | Self::ProgrammeEnregistre
            | Self::CalendrierDefini
            | Self::RegleEvaluationEnregistree
            | Self::EnseignantEnregistre
            | Self::AffectationModifiee
            | Self::EvaluationCreee
            | Self::NoteSaisie
            | Self::EvaluationValidee
            | Self::EvaluationDeverrouillee
            | Self::GrilleNotesImportee
            | Self::AbsenceRelevee
            | Self::AbsenceJustifiee
            | Self::ResultatCalcule
            | Self::AppreciationSaisie
            | Self::BulletinValide
            | Self::BulletinPublie
            | Self::BulletinCorrige
            | Self::MoyenneReprise
            | Self::InscriptionTransferee
            | Self::EleveExclu
            | Self::ExclusionLevee
            | Self::MembreEnregistre
            | Self::MembreParti
            | Self::ReferentielPersonnelModifie
            | Self::PieceDossierAjoutee
            | Self::PieceDossierRetiree
            | Self::EngagementConclu
            | Self::EngagementTermine
            | Self::EngagementRompu
            | Self::EtatHeuresSaisi
            | Self::EtatHeuresVise
            | Self::EtatHeuresDeverrouille
            | Self::AbsencePersonnelNotee
            | Self::AbsencePersonnelJustifiee
            | Self::AbsencePersonnelAnnulee
            | Self::CongeDemande
            | Self::CongeAccorde
            | Self::CongeRefuse
            | Self::CongeAnnule
            | Self::ElementPaieModifie
            | Self::PaieCalculee
            | Self::PaieValidee
            | Self::SalairePaye
            | Self::AvanceAccordee
            | Self::PaiementPersonnelAnnule
            | Self::CategorieTresorerieModifiee
            | Self::CompteTresorerieModifie
            | Self::DepenseSaisie
            | Self::DepenseDecidee
            | Self::DepensePayee
            | Self::DepenseAnnulee
            | Self::FournisseurModifie
            | Self::FactureFournisseurEnregistree
            | Self::FactureFournisseurAnnulee
            | Self::TransfertTresorerieEnregistre
            | Self::LigneTresoreriePointee
            | Self::BudgetPrevu
            | Self::JourneeCycleReglee
            | Self::SeancePlacee
            | Self::RepasPointe
            | Self::RecetteEnregistree
            | Self::RecetteAnnulee
            | Self::SanctionNotee
            | Self::SanctionAnnulee
            | Self::JournalArchive
            | Self::ReleveParSeanceEnregistre
            | Self::ReleveImporte
            | Self::PersonnelImporte
            | Self::ConsentementModifie
            | Self::JoignabiliteReinitialisee
            | Self::MessageMisEnFile
            | Self::MessageDecide
            | Self::MessageAnnule
            | Self::MessageRenvoye
            | Self::ModeleMessageModifie
            | Self::PortSmsChoisi
            | Self::FileSmsReprise
            | Self::DocumentRemis
            | Self::DocumentSigne
            | Self::AvisBulletinRenvoye
            | Self::ConvocationEmise
            | Self::ConvocationAccusee
            | Self::ConvocationReportee
            | Self::ConvocationCloturee
            | Self::ConvocationAnnulee
            | Self::CodeAccesRemis
            | Self::CodeAccesRevoque
            | Self::SmsRecu => 1,
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::TOUS.iter().copied().find(|t| t.nom() == nom)
    }
}

/// Une ligne de l'`outbox` (spécification §3.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evenement {
    /// Identifiant technique.
    pub id: Id<Evenement>,
    /// Type.
    pub type_evenement: TypeEvenement,
    /// Version du schéma au moment de l'émission.
    pub version: u16,
    /// Entité concernée.
    pub entite: String,
    /// Identifiant de l'entité, en texte.
    pub entite_id: String,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Acteur ; absent pour le système.
    pub acteur_id: Option<Id<Utilisateur>>,
    /// Instant du fait.
    pub horodatage: Horodatage,
    /// Poste émetteur.
    pub poste: Poste,
    /// Charge utile minimale et auto-suffisante, sans secret.
    pub charge_utile: serde_json::Value,
    /// Corrélation avec la ligne d'audit de la même transaction.
    pub correlation_id: String,
    /// Marqueur de consommation par la synchronisation ; nul tant que non consommé.
    pub consomme_le: Option<Horodatage>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_noms_sont_des_faits_accomplis_en_pascal_case() {
        for t in TypeEvenement::TOUS {
            let nom = t.nom();
            assert!(nom.chars().next().is_some_and(char::is_uppercase), "{nom}");
            assert!(nom.chars().all(char::is_alphanumeric), "{nom}");
            // Participe passé français : terminaison en « e » / « ee » / « ue » / « i » / « is » / « u ».
            assert!(
                nom.ends_with('e')
                    || nom.ends_with('i')
                    || nom.ends_with("is")
                    || nom.ends_with('u'),
                "{nom}"
            );
            assert!(
                !nom.starts_with("Creer")
                    && !nom.starts_with("Mettre")
                    && !nom.starts_with("Insert"),
                "{nom}"
            );
            assert_eq!(TypeEvenement::depuis_nom(nom), Some(*t));
            assert!(t.version() >= 1);
        }
    }
}
