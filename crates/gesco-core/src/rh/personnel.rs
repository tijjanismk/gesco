//! Registre du personnel (SPEC_PHASE_6 §2) : tout le personnel de l'école, dans des catégories et
//! des fonctions que l'école choisit, sans liste fermée (T-39). Un membre est une personne de la
//! phase 2 ; l'enseignant de la phase 4 qui partage sa personne est le même membre (Q-63).

use serde::{Deserialize, Serialize};

use crate::commun::{DateCivile, Id};
use crate::core::{Etablissement, Utilisateur};
use crate::scolarite::Personne;

/// Catégorie de personnel, propre à l'école ; les livrées ont un code stable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct CategoriePersonnel {
    /// Identifiant technique.
    pub id: Id<CategoriePersonnel>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Code d'une catégorie livrée (`enseignement`) ; absent pour une catégorie de l'école.
    pub code: Option<String>,
    /// Libellé affiché, modifiable.
    pub libelle: String,
    /// Ordre d'affichage.
    pub ordre: i64,
    /// Faux pour une catégorie retirée : les fiches qui la portent la gardent.
    pub active: bool,
}

/// Fonction proposée sur la fiche, rattachée à une catégorie.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct FonctionPersonnel {
    /// Identifiant technique.
    pub id: Id<FonctionPersonnel>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Catégorie.
    pub categorie_id: Id<CategoriePersonnel>,
    /// Code d'une fonction livrée ; absent pour une fonction de l'école.
    pub code: Option<String>,
    /// Libellé affiché, modifiable.
    pub libelle: String,
    /// Ordre d'affichage.
    pub ordre: i64,
    /// Faux pour une fonction retirée de la liste.
    pub active: bool,
}

/// Catégories livrées : code et libellé.
pub const CATEGORIES_LIVREES: [(&str, &str); 3] = [
    ("enseignement", "Enseignement"),
    ("administration", "Direction et administration"),
    ("appui", "Appui"),
];

/// Fonctions livrées : code, catégorie et libellé.
pub const FONCTIONS_LIVREES: [(&str, &str, &str); 13] = [
    ("enseignant", "enseignement", "Enseignant"),
    ("maitre", "enseignement", "Maître"),
    ("directeur", "administration", "Directeur"),
    ("directeur_etudes", "administration", "Directeur des études"),
    ("censeur", "administration", "Censeur"),
    ("secretaire", "administration", "Secrétaire"),
    ("comptable", "administration", "Comptable"),
    ("caissier", "administration", "Caissier"),
    ("surveillant", "administration", "Surveillant"),
    ("gardien", "appui", "Gardien"),
    ("agent_entretien", "appui", "Agent d'entretien"),
    ("chauffeur", "appui", "Chauffeur"),
    ("cuisinier", "appui", "Cuisinière"),
];

/// Comment le membre est payé ; la paie de la phase 7 s'en sert.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum MoyenPaiement {
    /// En espèces, à la caisse.
    Especes,
    /// Par mobile money, sur ce numéro (forme canonique).
    MobileMoney {
        /// Numéro.
        numero: String,
    },
    /// Par virement, sur ce compte.
    Banque {
        /// Banque.
        banque: String,
        /// Numéro de compte, tel qu'écrit par la banque.
        compte: String,
    },
}

/// Diplôme ou qualification du dossier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Diplome {
    /// Intitulé : « Licence de mathématiques », « CAP électricité ».
    pub intitule: String,
    /// École ou organisme qui l'a délivré.
    pub delivre_par: String,
    /// Année d'obtention.
    pub annee: Option<i32>,
}

/// Membre du personnel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct MembrePersonnel {
    /// Identifiant technique.
    pub id: Id<MembrePersonnel>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Personne : nom, prénoms, sexe, date de naissance, téléphones, adresse.
    pub personne_id: Id<Personne>,
    /// Matricule interne : `P-0001`, ou celui de l'école (Q-64).
    pub matricule: String,
    /// Catégorie.
    pub categorie_id: Id<CategoriePersonnel>,
    /// Fonction, telle qu'écrite : choisie dans la liste ou libre (T-39).
    pub fonction: String,
    /// Arrivée dans l'école.
    pub arrive_le: Option<DateCivile>,
    /// Départ ; absent tant qu'il est dans l'école.
    pub parti_le: Option<DateCivile>,
    /// Motif du départ.
    pub motif_depart: String,
    /// Fonctionnaire de l'État (T-40).
    pub fonctionnaire: bool,
    /// Numéro de fonctionnaire, facultatif.
    pub numero_fonctionnaire: String,
    /// Numéro de sécurité sociale (INPS), facultatif.
    pub numero_inps: String,
    /// Personne à prévenir.
    pub prevenir_nom: String,
    /// Téléphone de la personne à prévenir, forme canonique.
    pub prevenir_telephone: String,
    /// Moyen de paiement.
    pub paiement: MoyenPaiement,
    /// Diplômes et qualifications.
    pub diplomes: Vec<Diplome>,
    /// Compte lié : il lit sa propre fiche.
    pub utilisateur_id: Option<Id<Utilisateur>>,
    /// Situation de famille, facultative : elle sert à la réduction d'impôt pour charges de
    /// famille (SPEC_PHASE_7 §3).
    pub situation_famille: Option<SituationFamille>,
    /// Enfants à charge, facultatif.
    pub enfants_a_charge: Option<i64>,
}

/// Situation de famille d'un membre du personnel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum SituationFamille {
    /// Célibataire.
    Celibataire,
    /// Marié ou mariée.
    Marie,
    /// Veuf ou veuve.
    Veuf,
    /// Divorcé ou divorcée.
    Divorce,
}

impl SituationFamille {
    /// Les quatre situations.
    pub const TOUTES: [Self; 4] = [Self::Celibataire, Self::Marie, Self::Veuf, Self::Divorce];

    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Celibataire => "celibataire",
            Self::Marie => "marie",
            Self::Veuf => "veuf",
            Self::Divorce => "divorce",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::TOUTES.into_iter().find(|s| s.nom() == nom)
    }
}

/// Plus grand nombre d'enfants à charge admis sur une fiche.
pub const ENFANTS_A_CHARGE_MAX: i64 = 30;

impl MembrePersonnel {
    /// Vrai tant que le membre n'est pas parti.
    pub const fn present(&self) -> bool {
        self.parti_le.is_none()
    }
}

/// Nature d'une pièce du dossier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum NaturePiece {
    /// Pièce d'identité.
    Identite,
    /// Diplôme.
    Diplome,
    /// Engagement signé.
    Engagement,
    /// Autre pièce.
    Autre,
}

impl NaturePiece {
    /// Les quatre natures.
    pub const TOUTES: [Self; 4] = [Self::Identite, Self::Diplome, Self::Engagement, Self::Autre];

    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Identite => "identite",
            Self::Diplome => "diplome",
            Self::Engagement => "engagement",
            Self::Autre => "autre",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::TOUTES.into_iter().find(|n| n.nom() == nom)
    }
}

/// Pièce du dossier, sans son contenu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct PieceDossier {
    /// Identifiant technique.
    pub id: Id<PieceDossier>,
    /// Membre.
    pub membre_id: Id<MembrePersonnel>,
    /// Intitulé.
    pub intitule: String,
    /// Nature.
    pub nature: NaturePiece,
    /// Nom du fichier d'origine, pour l'enregistrer de nouveau.
    pub nom_fichier: String,
    /// Taille en octets.
    pub taille: i64,
    /// Faux pour une pièce retirée du dossier : elle reste dans la base (R-14).
    pub active: bool,
}

/// Taille maximale d'une pièce : 2 Mo, de quoi tenir le scan d'un diplôme.
pub const TAILLE_MAX_PIECE: usize = 2 * 1024 * 1024;

/// Extensions admises pour une pièce : images et PDF.
pub const EXTENSIONS_PIECE: [&str; 4] = ["pdf", "png", "jpg", "jpeg"];

/// Matricule suivant `P-0001`, après le plus grand des matricules de cette forme. Un matricule
/// choisi par l'école n'interrompt pas la suite.
pub fn matricule_suivant<'a>(existants: impl IntoIterator<Item = &'a str>) -> String {
    let dernier = existants
        .into_iter()
        .filter_map(|m| m.strip_prefix("P-"))
        .filter(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
        .filter_map(|n| n.parse::<u32>().ok())
        .max()
        .unwrap_or(0);
    format!("P-{:04}", dernier.saturating_add(1))
}

/// Contrôle d'un membre avant écriture ; l'erreur est la clé du message.
pub fn verifier_membre(m: &MembrePersonnel) -> Result<(), &'static str> {
    let matricule = m.matricule.trim();
    if matricule.is_empty() || matricule.chars().count() > 20 {
        return Err("personnel.matricule_invalide");
    }
    if m.fonction.trim().is_empty() {
        return Err("personnel.fonction_obligatoire");
    }
    if let (Some(arrivee), Some(depart)) = (m.arrive_le, m.parti_le)
        && depart < arrivee
    {
        return Err("personnel.depart_avant_arrivee");
    }
    if m.parti_le.is_some() && m.motif_depart.trim().is_empty() {
        return Err("personnel.motif_depart_obligatoire");
    }
    if m.enfants_a_charge
        .is_some_and(|n| !(0..=ENFANTS_A_CHARGE_MAX).contains(&n))
    {
        return Err("personnel.enfants_invalides");
    }
    match &m.paiement {
        MoyenPaiement::Especes => {}
        MoyenPaiement::MobileMoney { numero } => {
            if numero.trim().is_empty() {
                return Err("personnel.numero_paiement_obligatoire");
            }
        }
        MoyenPaiement::Banque { banque, compte } => {
            if banque.trim().is_empty() || compte.trim().is_empty() {
                return Err("personnel.compte_bancaire_incomplet");
            }
        }
    }
    if m.diplomes.iter().any(|d| d.intitule.trim().is_empty()) {
        return Err("personnel.diplome_sans_intitule");
    }
    if m.diplomes
        .iter()
        .filter_map(|d| d.annee)
        .any(|a| !(1950..=2100).contains(&a))
    {
        return Err("personnel.annee_diplome_invalide");
    }
    Ok(())
}

/// Code de la fonction donnée à un enseignant devenu membre à la mise à niveau : *maître* s'il
/// enseigne au 1er cycle, *enseignant* sinon.
pub const fn fonction_d_enseignant(au_premier_cycle: bool) -> &'static str {
    if au_premier_cycle {
        "maitre"
    } else {
        "enseignant"
    }
}

/// Extension admise d'un nom de fichier, en minuscules.
pub fn extension_piece(nom_fichier: &str) -> Option<&'static str> {
    let extension = nom_fichier.rsplit_once('.')?.1.to_ascii_lowercase();
    EXTENSIONS_PIECE.into_iter().find(|e| *e == extension)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn membre() -> MembrePersonnel {
        MembrePersonnel {
            id: Id::depuis_octets([1; 16]),
            etablissement_id: Id::depuis_octets([2; 16]),
            personne_id: Id::depuis_octets([3; 16]),
            matricule: "P-0001".to_owned(),
            categorie_id: Id::depuis_octets([4; 16]),
            fonction: "Gardien".to_owned(),
            arrive_le: DateCivile::nouvelle(2024, 10, 1),
            parti_le: None,
            motif_depart: String::new(),
            fonctionnaire: false,
            numero_fonctionnaire: String::new(),
            numero_inps: String::new(),
            prevenir_nom: String::new(),
            prevenir_telephone: String::new(),
            paiement: MoyenPaiement::Especes,
            diplomes: Vec::new(),
            utilisateur_id: None,
            situation_famille: None,
            enfants_a_charge: None,
        }
    }

    #[test]
    fn le_matricule_suit_le_plus_grand_de_la_forme_p() {
        assert_eq!(matricule_suivant([]), "P-0001");
        assert_eq!(
            matricule_suivant(["P-0001", "P-0007", "ENS-12", "P-12a"]),
            "P-0008"
        );
        assert_eq!(matricule_suivant(["P-9999"]), "P-10000");
    }

    #[test]
    fn un_membre_se_controle_avant_ecriture() {
        assert_eq!(verifier_membre(&membre()), Ok(()));
        let mut m = membre();
        m.fonction = "  ".to_owned();
        assert_eq!(verifier_membre(&m), Err("personnel.fonction_obligatoire"));
        let mut m = membre();
        m.parti_le = DateCivile::nouvelle(2024, 9, 1);
        m.motif_depart = "Démission".to_owned();
        assert_eq!(verifier_membre(&m), Err("personnel.depart_avant_arrivee"));
        let mut m = membre();
        m.parti_le = DateCivile::nouvelle(2025, 7, 1);
        assert_eq!(
            verifier_membre(&m),
            Err("personnel.motif_depart_obligatoire")
        );
        let mut m = membre();
        m.paiement = MoyenPaiement::Banque {
            banque: "BDM".to_owned(),
            compte: String::new(),
        };
        assert_eq!(
            verifier_membre(&m),
            Err("personnel.compte_bancaire_incomplet")
        );
        let mut m = membre();
        m.diplomes.push(Diplome {
            intitule: "Licence".to_owned(),
            delivre_par: String::new(),
            annee: Some(1890),
        });
        assert_eq!(verifier_membre(&m), Err("personnel.annee_diplome_invalide"));
    }

    #[test]
    fn seules_les_images_et_les_pdf_sont_des_pieces() {
        assert_eq!(extension_piece("diplome.PDF"), Some("pdf"));
        assert_eq!(extension_piece("cni.jpeg"), Some("jpeg"));
        assert_eq!(extension_piece("lettre.docx"), None);
        assert_eq!(extension_piece("sans-extension"), None);
    }

    #[test]
    fn les_fonctions_livrees_sont_dans_les_categories_livrees() {
        for (_, categorie, _) in FONCTIONS_LIVREES {
            assert!(CATEGORIES_LIVREES.iter().any(|(c, _)| *c == categorie));
        }
        assert!(
            FONCTIONS_LIVREES
                .iter()
                .any(|(c, ..)| *c == fonction_d_enseignant(true))
        );
        assert!(
            FONCTIONS_LIVREES
                .iter()
                .any(|(c, ..)| *c == fonction_d_enseignant(false))
        );
    }
}
