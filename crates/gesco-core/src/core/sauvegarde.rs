//! Manifeste et rotation des sauvegardes (blueprint §18, spécification §11).

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::cle::Enveloppe;
use super::etablissement::Etablissement;
use crate::commun::{DateCivile, Horodatage, Id};

/// Format courant de l'archive de sauvegarde.
pub const FORMAT_ARCHIVE: u16 = 1;

/// Rappel imprimé en clair dans chaque manifeste (spécification §11.2).
pub const RAPPEL_PHRASE: &str = "La restauration de cette sauvegarde sur un autre poste demande la phrase de secours imprimée sur le certificat de clé de secours.";

/// Ce qui a déclenché une sauvegarde (§11.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum MotifSauvegarde {
    /// Demandée par un utilisateur.
    Manuelle,
    /// Intervalle configuré.
    Planifiee,
    /// Fermeture de l'application.
    Fermeture,
    /// Avant une migration de schéma (R-48).
    AvantMigration,
    /// Avant une restauration (R-48).
    AvantRestauration,
    /// Avant l'ouverture d'une année scolaire (SPEC_PHASE_1 §3.3).
    AvantOuvertureAnnee,
    /// Avant la clôture d'une année scolaire (SPEC_PHASE_10 §4).
    AvantClotureAnnee,
    /// Avant l'application d'un fichier d'élèves importé (R-48, SPEC_PHASE_2 §10).
    AvantImport,
    /// Avant la facturation en masse des inscriptions déjà validées (R-48, SPEC_PHASE_3 §0.3).
    AvantFacturation,
}

/// Manifeste d'une archive, lisible sans la clé. Il est lié au contenu chiffré : le modifier
/// rend l'archive illisible.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifeste {
    /// Format de l'archive.
    pub format: u16,
    /// Version de l'application qui l'a produite.
    pub version_applicative: String,
    /// Version du schéma de la base sauvegardée.
    pub version_schema: i64,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Nom de l'établissement, pour l'assistant de restauration.
    pub etablissement_nom: String,
    /// Instant de la sauvegarde.
    pub horodatage: Horodatage,
    /// Déclencheur.
    pub motif: MotifSauvegarde,
    /// Identifiant de la clé de données qui chiffre le contenu.
    pub cle_id: String,
    /// Enveloppe de secours de cette clé : la phrase du certificat l'ouvre (R-57). Jamais
    /// l'enveloppe machine.
    pub enveloppe_secours: Enveloppe,
    /// Empreinte SHA-256 du fichier de base sauvegardé.
    pub empreinte_base: String,
    /// Empreinte du contenu logique de la base, comparée après restauration.
    pub empreinte_contenu: String,
    /// Préfixe des nonces des blocs chiffrés.
    #[serde(with = "hex::serde")]
    pub prefixe_nonce: [u8; 16],
    /// Rappel en clair.
    pub rappel: String,
}

/// Politique de rotation : 7 quotidiennes, 4 hebdomadaires, 12 mensuelles par défaut (§11.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PolitiqueRotation {
    /// Jours distincts conservés.
    pub quotidiennes: u32,
    /// Semaines distinctes conservées.
    pub hebdomadaires: u32,
    /// Mois distincts conservés.
    pub mensuelles: u32,
}

impl Default for PolitiqueRotation {
    fn default() -> Self {
        Self {
            quotidiennes: 7,
            hebdomadaires: 4,
            mensuelles: 12,
        }
    }
}

/// Sorte d'une destination de sauvegarde. Les réglages venus de la 1.2 n'ont qu'une case
/// « amovible » : vraie, elle donne [`SorteDestination::Amovible`], fausse,
/// [`SorteDestination::Dossier`].
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type,
)]
#[serde(rename_all = "snake_case")]
pub enum SorteDestination {
    /// Dossier du poste ou du réseau.
    #[default]
    Dossier,
    /// Clé USB, disque externe : seule protection réelle contre un rançongiciel.
    Amovible,
    /// Dossier que le logiciel d'un fournisseur envoie sur un compte en ligne : la copie hors de
    /// l'école. Il ne reçoit que la copie à la demande et la copie du rythme.
    Synchronise,
}

impl SorteDestination {
    /// Nom rangé dans la table `sauvegarde`.
    pub const fn nom(self) -> &'static str {
        match self {
            Self::Dossier => "dossier",
            Self::Amovible => "amovible",
            Self::Synchronise => "synchronise",
        }
    }

    /// Sorte d'après son nom rangé.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        match nom {
            "dossier" => Some(Self::Dossier),
            "amovible" => Some(Self::Amovible),
            "synchronise" => Some(Self::Synchronise),
            _ => None,
        }
    }
}

/// État d'une copie hors de l'école. Les noms ne parlent pas de dossier : un serveur de
/// l'éditeur reprendra les mêmes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum EtatEnvoi {
    /// Intacte dans le dossier ; Windows ne dit rien de l'envoi.
    Deposee,
    /// Windows dit que le fichier n'est pas encore envoyé.
    EnAttente,
    /// Windows dit que le fichier est à jour sur le compte.
    Envoyee,
    /// Windows dit que le logiciel du fournisseur n'a pas pu l'envoyer.
    EchecEnvoi,
    /// Dossier absent, plein ou refusé ; copie interrompue.
    NonDeposee,
    /// Fichier effacé ou modifié depuis le dépôt.
    Absente,
}

impl EtatEnvoi {
    /// Nom rangé dans la table `sauvegarde`.
    pub const fn nom(self) -> &'static str {
        match self {
            Self::Deposee => "deposee",
            Self::EnAttente => "en_attente",
            Self::Envoyee => "envoyee",
            Self::EchecEnvoi => "echec_envoi",
            Self::NonDeposee => "non_deposee",
            Self::Absente => "absente",
        }
    }

    /// État d'après son nom rangé.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        match nom {
            "deposee" => Some(Self::Deposee),
            "en_attente" => Some(Self::EnAttente),
            "envoyee" => Some(Self::Envoyee),
            "echec_envoi" => Some(Self::EchecEnvoi),
            "non_deposee" => Some(Self::NonDeposee),
            "absente" => Some(Self::Absente),
            _ => None,
        }
    }
}

/// Clé du paramètre du rythme de la copie hors de l'école.
pub const PARAMETRE_RYTHME_HORS_ECOLE: &str = "sauvegarde.hors_ecole.rythme";

/// Rythme de la copie hors de l'école, un seul pour tous les dossiers synchronisés. Jamais
/// chaque jour.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type,
)]
#[serde(rename_all = "snake_case")]
pub enum RythmeHorsEcole {
    /// Rien ne part sans un geste de l'école. Réglage livré.
    #[default]
    Demande,
    /// Une copie quand la dernière bien déposée a sept jours ou plus.
    Semaine,
    /// Une copie quand la dernière bien déposée date du même jour du mois précédent ou d'avant.
    Mois,
}

impl RythmeHorsEcole {
    /// Nom rangé dans le paramètre.
    pub const fn nom(self) -> &'static str {
        match self {
            Self::Demande => "demande",
            Self::Semaine => "semaine",
            Self::Mois => "mois",
        }
    }

    /// Rythme d'après son nom rangé.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        match nom {
            "demande" => Some(Self::Demande),
            "semaine" => Some(Self::Semaine),
            "mois" => Some(Self::Mois),
            _ => None,
        }
    }
}

/// Nettoyage d'un dossier synchronisé : les 4 dernières semaines et les 12 derniers mois, soit
/// 16 copies au plus.
pub const POLITIQUE_HORS_ECOLE: PolitiqueRotation = PolitiqueRotation {
    quotidiennes: 0,
    hebdomadaires: 4,
    mensuelles: 12,
};

/// Après un dépôt manqué (dossier absent, disque plein), la copie du rythme ne se retente qu'un
/// jour plus tard : la tâche périodique ne refait pas une sauvegarde chaque minute.
pub const DELAI_NOUVEL_ESSAI_HORS_ECOLE_MILLIS: i64 = 24 * 3_600_000;

/// Instant à partir duquel la copie suivante est due, d'après la dernière copie bien déposée ;
/// rien au rythme « à la demande ». Au mois, le même jour du mois suivant, ramené au dernier jour
/// d'un mois plus court (le 31 mars donne le 30 avril), à minuit.
pub fn prochaine_copie(rythme: RythmeHorsEcole, derniere: Horodatage) -> Option<Horodatage> {
    match rythme {
        RythmeHorsEcole::Demande => None,
        RythmeHorsEcole::Semaine => derniere.plus_millis(7 * MILLIS_PAR_JOUR).ok(),
        RythmeHorsEcole::Mois => {
            let c = derniere.civil_utc();
            let (annee, mois) = if c.mois == 12 {
                (c.annee + 1, 1)
            } else {
                (c.annee, c.mois + 1)
            };
            let annee = i32::try_from(annee).ok()?;
            let date = (1..=c.jour)
                .rev()
                .find_map(|jour| DateCivile::nouvelle(annee, mois, jour))?;
            Some(Horodatage::depuis_millis_epoque(
                date.jours_depuis_epoque() * MILLIS_PAR_JOUR,
            ))
        }
    }
}

/// Vrai si la copie du rythme est due : jamais à la demande ; sinon quand il n'y a pas encore
/// de copie bien déposée, ou que la suivante est échue. Une copie à la demande compte comme une
/// copie du rythme.
pub fn copie_due(
    rythme: RythmeHorsEcole,
    derniere_deposee: Option<Horodatage>,
    maintenant: Horodatage,
) -> bool {
    if rythme == RythmeHorsEcole::Demande {
        return false;
    }
    derniere_deposee.is_none_or(|d| prochaine_copie(rythme, d).is_some_and(|p| p <= maintenant))
}

/// Première vérification d'une copie hors de l'école, quelques minutes après son dépôt.
pub const PREMIERE_VERIFICATION_MILLIS: i64 = 5 * 60_000;
/// Deuxième vérification, une heure après le dépôt.
pub const DEUXIEME_VERIFICATION_MILLIS: i64 = 3_600_000;
/// Ensuite, une vérification par jour, tant que la copie n'est pas envoyée.
pub const VERIFICATION_QUOTIDIENNE_MILLIS: i64 = 24 * 3_600_000;
/// Une copie qui attend son envoi plus de sept jours fait parler le bandeau.
pub const ATTENTE_SIGNALEE_MILLIS: i64 = 7 * 24 * 3_600_000;

/// Vrai si l'état d'envoi d'une copie se vérifie maintenant : seulement tant qu'elle peut encore
/// changer (déposée, en attente, en échec), quelques minutes après le dépôt, une heure après, au
/// premier passage depuis le démarrage du poste principal, puis une fois par jour.
pub fn verification_due(
    etat: EtatEnvoi,
    depose_le: Horodatage,
    verifie_le: Option<Horodatage>,
    maintenant: Horodatage,
    premier_passage: bool,
) -> bool {
    if !matches!(
        etat,
        EtatEnvoi::Deposee | EtatEnvoi::EnAttente | EtatEnvoi::EchecEnvoi
    ) {
        return false;
    }
    if premier_passage {
        return true;
    }
    let depot = depose_le.millis_epoque();
    let derniere = verifie_le.unwrap_or(depose_le).millis_epoque();
    let instant = maintenant.millis_epoque();
    let jalon_manque = [PREMIERE_VERIFICATION_MILLIS, DEUXIEME_VERIFICATION_MILLIS]
        .iter()
        .map(|delai| depot.saturating_add(*delai))
        .any(|jalon| instant >= jalon && derniere < jalon);
    jalon_manque || instant.saturating_sub(derniere) >= VERIFICATION_QUOTIDIENNE_MILLIS
}

/// Vrai si la dernière copie d'un dossier synchronisé n'est pas arrivée sur le compte : elle n'a
/// pas pu être déposée, n'est plus dans le dossier, est en échec d'envoi, ou attend son envoi
/// depuis plus de sept jours. « Déposée » ne fait jamais parler le bandeau : Windows ne sait
/// pas tout.
pub fn alerte_hors_ecole(etat: EtatEnvoi, depose_le: Horodatage, maintenant: Horodatage) -> bool {
    match etat {
        EtatEnvoi::NonDeposee | EtatEnvoi::Absente | EtatEnvoi::EchecEnvoi => true,
        EtatEnvoi::EnAttente => depose_le
            .difference_millis(&maintenant)
            .is_ok_and(|d| d > ATTENTE_SIGNALEE_MILLIS),
        EtatEnvoi::Deposee | EtatEnvoi::Envoyee => false,
    }
}

const MILLIS_PAR_JOUR: i64 = 86_400_000;

/// Regroupe les sauvegardes par jour, semaine ou mois.
type CleDePalier = fn(Horodatage) -> i64;

fn jour(h: Horodatage) -> i64 {
    h.millis_epoque().div_euclid(MILLIS_PAR_JOUR)
}

// Le 1er janvier 1970 est un jeudi : décaler de trois jours aligne les semaines sur le lundi.
fn semaine(h: Horodatage) -> i64 {
    (jour(h) + 3).div_euclid(7)
}

fn mois(h: Horodatage) -> i64 {
    let civil = h.civil_utc();
    civil.annee * 12 + i64::from(civil.mois)
}

/// Sauvegardes à conserver : la plus récente de chacun des derniers jours, semaines et mois, dans
/// la limite de la politique. La plus récente de toutes est toujours conservée. Le Mali vit à
/// l'heure UTC : le jour civil UTC est le jour de l'école.
pub fn a_conserver<T: Clone + Eq + std::hash::Hash>(
    sauvegardes: &[(T, Horodatage)],
    politique: PolitiqueRotation,
) -> Vec<T> {
    let mut triees: Vec<&(T, Horodatage)> = sauvegardes.iter().collect();
    triees.sort_by_key(|(_, h)| std::cmp::Reverse(*h));
    let mut gardees: Vec<T> = Vec::new();
    let mut garder = |id: &T| {
        if !gardees.contains(id) {
            gardees.push(id.clone());
        }
    };
    if let Some((id, _)) = triees.first() {
        garder(id);
    }
    let paliers: [(u32, CleDePalier); 3] = [
        (politique.quotidiennes, jour),
        (politique.hebdomadaires, semaine),
        (politique.mensuelles, mois),
    ];
    for (limite, cle) in paliers {
        let mut vues = HashSet::new();
        for (id, h) in &triees {
            if vues.len() >= limite as usize {
                break;
            }
            if vues.insert(cle(*h)) {
                garder(id);
            }
        }
    }
    gardees
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEURE: i64 = 3_600_000;

    fn h(jours: i64, heures: i64) -> Horodatage {
        // 2026-10-05 est un lundi : jour 20_731 depuis l'époque.
        Horodatage::depuis_millis_epoque((20_731 + jours) * MILLIS_PAR_JOUR + heures * HEURE)
    }

    #[test]
    fn une_par_jour_sur_sept_jours() {
        let mut sauvegardes = Vec::new();
        for j in 0..10 {
            sauvegardes.push((format!("{j}-matin"), h(j, 8)));
            sauvegardes.push((format!("{j}-soir"), h(j, 18)));
        }
        let politique = PolitiqueRotation {
            quotidiennes: 7,
            hebdomadaires: 0,
            mensuelles: 0,
        };
        let mut gardees = a_conserver(&sauvegardes, politique);
        gardees.sort();
        let attendues: Vec<String> = (3..10).map(|j| format!("{j}-soir")).collect();
        assert_eq!(gardees, attendues);
    }

    #[test]
    fn semaines_et_mois_s_ajoutent_aux_jours() {
        let sauvegardes: Vec<(i64, Horodatage)> = (0..400).map(|j| (j, h(j, 12))).collect();
        let mut gardees = a_conserver(&sauvegardes, PolitiqueRotation::default());
        gardees.sort_unstable();
        // Sept derniers jours (393 à 399) ; deux semaines de plus (391, 384), la semaine du
        // jour 399 et la précédente étant couvertes ; puis le dernier jour de chacun des dix mois
        // précédents.
        assert_eq!(
            gardees,
            vec![
                87, 118, 146, 177, 207, 238, 268, 299, 330, 360, 384, 391, 393, 394, 395, 396, 397,
                398, 399
            ]
        );
    }

    #[test]
    fn la_plus_recente_est_toujours_gardee() {
        let sauvegardes = vec![("a", h(0, 1)), ("b", h(0, 2))];
        let politique = PolitiqueRotation {
            quotidiennes: 0,
            hebdomadaires: 0,
            mensuelles: 0,
        };
        assert_eq!(a_conserver(&sauvegardes, politique), vec!["b"]);
        assert!(a_conserver::<&str>(&[], PolitiqueRotation::default()).is_empty());
    }

    #[test]
    fn la_copie_de_la_semaine_est_due_apres_sept_jours() {
        let rythme = RythmeHorsEcole::Semaine;
        assert!(copie_due(rythme, None, h(0, 0)));
        assert!(!copie_due(rythme, Some(h(0, 10)), h(7, 9)));
        assert!(copie_due(rythme, Some(h(0, 10)), h(7, 10)));
        assert!(!copie_due(RythmeHorsEcole::Demande, None, h(400, 0)));
    }

    fn instant(annee: i32, mois: u8, jour: u8, heure: i64) -> Horodatage {
        let jours = DateCivile::nouvelle(annee, mois, jour).map_or(0, |d| d.jours_depuis_epoque());
        Horodatage::depuis_millis_epoque(jours * MILLIS_PAR_JOUR + heure * HEURE)
    }

    #[test]
    fn la_copie_du_mois_garde_le_jour_ou_prend_le_dernier_du_mois() {
        let rythme = RythmeHorsEcole::Mois;
        assert_eq!(
            prochaine_copie(rythme, instant(2027, 3, 31, 15)),
            Some(instant(2027, 4, 30, 0))
        );
        assert_eq!(
            prochaine_copie(rythme, instant(2027, 1, 31, 8)),
            Some(instant(2027, 2, 28, 0))
        );
        assert_eq!(
            prochaine_copie(rythme, instant(2026, 12, 15, 8)),
            Some(instant(2027, 1, 15, 0))
        );
        assert!(!copie_due(
            rythme,
            Some(instant(2027, 3, 31, 15)),
            instant(2027, 4, 29, 23)
        ));
        assert!(copie_due(
            rythme,
            Some(instant(2027, 3, 31, 15)),
            instant(2027, 4, 30, 7)
        ));
        assert_eq!(
            prochaine_copie(RythmeHorsEcole::Demande, instant(2027, 1, 1, 0)),
            None
        );
    }

    #[test]
    fn le_dossier_synchronise_garde_seize_copies_au_plus() {
        // Une copie par jour pendant deux ans : 4 semaines et 12 mois, la plus récente comprise.
        let sauvegardes: Vec<(i64, Horodatage)> = (0..730).map(|j| (j, h(j, 12))).collect();
        let gardees = a_conserver(&sauvegardes, POLITIQUE_HORS_ECOLE);
        assert!(gardees.len() <= 16, "{}", gardees.len());
        assert!(gardees.contains(&729));
    }

    #[test]
    fn les_noms_se_relisent() {
        for s in [
            SorteDestination::Dossier,
            SorteDestination::Amovible,
            SorteDestination::Synchronise,
        ] {
            assert_eq!(SorteDestination::depuis_nom(s.nom()), Some(s));
        }
        for e in [
            EtatEnvoi::Deposee,
            EtatEnvoi::EnAttente,
            EtatEnvoi::Envoyee,
            EtatEnvoi::EchecEnvoi,
            EtatEnvoi::NonDeposee,
            EtatEnvoi::Absente,
        ] {
            assert_eq!(EtatEnvoi::depuis_nom(e.nom()), Some(e));
        }
        for r in [
            RythmeHorsEcole::Demande,
            RythmeHorsEcole::Semaine,
            RythmeHorsEcole::Mois,
        ] {
            assert_eq!(RythmeHorsEcole::depuis_nom(r.nom()), Some(r));
        }
        assert_eq!(RythmeHorsEcole::depuis_nom("jour"), None);
    }

    #[test]
    fn la_copie_se_verifie_a_cinq_minutes_une_heure_puis_chaque_jour() {
        let depot = h(0, 10);
        let minute = 60_000;
        let a = |m: i64| Horodatage::depuis_millis_epoque(depot.millis_epoque() + m * minute);
        let e = EtatEnvoi::Deposee;
        assert!(!verification_due(e, depot, Some(depot), a(4), false));
        assert!(verification_due(e, depot, Some(depot), a(5), false));
        assert!(!verification_due(e, depot, Some(a(5)), a(30), false));
        assert!(verification_due(e, depot, Some(a(5)), a(60), false));
        assert!(!verification_due(e, depot, Some(a(60)), a(60 * 24), false));
        assert!(verification_due(e, depot, Some(a(60)), a(60 * 25), false));
        assert!(verification_due(e, depot, Some(a(60)), a(61), true));
        // Envoyée, absente, jamais déposée : plus rien à vérifier, même au démarrage.
        for fini in [
            EtatEnvoi::Envoyee,
            EtatEnvoi::Absente,
            EtatEnvoi::NonDeposee,
        ] {
            assert!(!verification_due(fini, depot, None, a(60 * 48), true));
        }
    }

    #[test]
    fn le_bandeau_ne_parle_que_d_une_copie_qui_n_arrive_pas() {
        let depot = h(0, 10);
        assert!(alerte_hors_ecole(EtatEnvoi::NonDeposee, depot, depot));
        assert!(alerte_hors_ecole(EtatEnvoi::Absente, depot, depot));
        assert!(alerte_hors_ecole(EtatEnvoi::EchecEnvoi, depot, depot));
        assert!(!alerte_hors_ecole(EtatEnvoi::Deposee, depot, h(30, 0)));
        assert!(!alerte_hors_ecole(EtatEnvoi::Envoyee, depot, h(30, 0)));
        assert!(!alerte_hors_ecole(EtatEnvoi::EnAttente, depot, h(7, 10)));
        assert!(alerte_hors_ecole(EtatEnvoi::EnAttente, depot, h(7, 11)));
    }

    #[test]
    fn semaine_alignee_sur_le_lundi() {
        assert_eq!(semaine(h(0, 0)), semaine(h(6, 23)));
        assert_ne!(semaine(h(0, 0)), semaine(h(-1, 23)));
    }
}
