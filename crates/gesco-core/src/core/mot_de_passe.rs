//! Politique de mot de passe d'un établissement (SPEC_PHASE_1 §6, blueprint §17.1). Les règles
//! sont des paramètres ; la longueur ne descend jamais sous le plancher du produit.

/// Plancher absolu de longueur, que le paramètre d'établissement ne peut pas abaisser : six
/// caractères (porteur du projet, 23 octobre 2026).
pub const LONGUEUR_PLANCHER: usize = 6;

/// Mots de passe parmi les plus essayés, en minuscules : suites de chiffres et de touches, mots
/// de passe génériques, et mots du contexte d'une école malienne. Refusés seulement si l'école
/// le demande : par défaut, un mot de passe simple comme `000000` est accepté (porteur du
/// projet, 23 octobre 2026).
const COURANTS: &[&str] = &[
    "12345678",
    "123456789",
    "1234567890",
    "0123456789",
    "87654321",
    "987654321",
    "11111111",
    "00000000",
    "12121212",
    "11223344",
    "12341234",
    "123123123",
    "112233445566",
    "1q2w3e4r",
    "1qaz2wsx",
    "qwertyuiop",
    "qwerty123",
    "azertyuiop",
    "azerty123",
    "azerty12",
    "aqwzsxedc",
    "qazwsxedc",
    "zaq12wsx",
    "abcd1234",
    "abc12345",
    "abcdefgh",
    "password",
    "password1",
    "password123",
    "passw0rd",
    "motdepasse",
    "motdepasse1",
    "motdepasse123",
    "mot2passe",
    "iloveyou",
    "jetaime1",
    "jetaime123",
    "bonjour1",
    "bonjour123",
    "bonsoir1",
    "soleil123",
    "doudou123",
    "loulou123",
    "chouchou",
    "princesse",
    "football",
    "football1",
    "baseball",
    "sunshine",
    "superman",
    "starwars",
    "whatever",
    "trustno1",
    "letmein1",
    "welcome1",
    "bienvenue",
    "bienvenue1",
    "administrateur",
    "administrator",
    "admin123",
    "admin1234",
    "adminadmin",
    "changeme",
    "secret123",
    "utilisateur",
    "directeur",
    "directrice",
    "secretaire",
    "secretariat",
    "comptable",
    "caissier",
    "enseignant",
    "professeur",
    "surveillant",
    "personnel",
    "ecole123",
    "ecole2026",
    "ecolemali",
    "lycee123",
    "college1",
    "scolarite",
    "inscription",
    "gesco123",
    "gesco2026",
    "bamako123",
    "bamako2026",
    "malimali",
    "mali2026",
    "mali1234",
    "republique",
    "kayes123",
    "sikasso1",
    "segou123",
    "mopti123",
    "tombouctou",
    "koulikoro",
    "badalabougou",
    "hamdallaye",
    "allahakbar",
    "bismillah",
    "inchallah",
    "alhamdoulillah",
    "mohamed1",
    "mamadou1",
    "moussa123",
    "fatoumata",
    "aminata1",
    "mariam123",
    "oumou123",
    "coulibaly",
    "traore123",
    "diarra123",
    "keita123",
    "konate123",
    "toure123",
    "sangare1",
    "doumbia1",
    "dembele1",
    "septembre",
    "octobre1",
    "janvier1",
    "fevrier1",
    "decembre",
    "rentree1",
    "vacances",
];

/// Règles applicables aux mots de passe de l'établissement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PolitiqueMotDePasse {
    /// Longueur minimale, jamais sous [`LONGUEUR_PLANCHER`].
    pub longueur_min: usize,
    /// Refuse un mot de passe qui contient l'identifiant de connexion.
    pub refuser_identifiant: bool,
    /// Refuse un mot de passe de la liste des plus courants.
    pub refuser_courants: bool,
}

impl Default for PolitiqueMotDePasse {
    fn default() -> Self {
        Self {
            longueur_min: LONGUEUR_PLANCHER,
            refuser_identifiant: true,
            refuser_courants: false,
        }
    }
}

impl PolitiqueMotDePasse {
    /// Vérifie un mot de passe ; l'erreur est la clé du message à afficher.
    pub fn verifier(&self, mot_de_passe: &str, identifiant: &str) -> Result<(), &'static str> {
        if mot_de_passe.chars().count() < self.longueur_min.max(LONGUEUR_PLANCHER) {
            return Err("mot_de_passe.trop_court");
        }
        let minuscules = mot_de_passe.to_lowercase();
        let identifiant = identifiant.trim().to_lowercase();
        if self.refuser_identifiant && !identifiant.is_empty() && minuscules.contains(&identifiant)
        {
            return Err("mot_de_passe.contient_identifiant");
        }
        if self.refuser_courants && COURANTS.contains(&minuscules.as_str()) {
            return Err("mot_de_passe.trop_courant");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_liste_ne_contient_que_des_mots_de_passe_au_plancher() {
        for courant in COURANTS {
            assert!(courant.chars().count() >= LONGUEUR_PLANCHER, "{courant}");
            assert_eq!(*courant, courant.to_lowercase(), "{courant}");
        }
    }

    #[test]
    fn chaque_regle_refuse_ce_qu_elle_vise() {
        let p = PolitiqueMotDePasse {
            refuser_courants: true,
            ..PolitiqueMotDePasse::default()
        };
        assert_eq!(p.verifier("court", "awa"), Err("mot_de_passe.trop_court"));
        // Par défaut, six chiffres simples suffisent.
        assert_eq!(
            PolitiqueMotDePasse::default().verifier("000000", "awa"),
            Ok(())
        );
        assert_eq!(
            p.verifier("awa-traore-2026", "AWA"),
            Err("mot_de_passe.contient_identifiant")
        );
        assert_eq!(
            p.verifier("Bamako2026", "fanta"),
            Err("mot_de_passe.trop_courant")
        );
        assert_eq!(p.verifier("cour-du-lycee-2026", "fanta"), Ok(()));
        let longue = PolitiqueMotDePasse {
            longueur_min: 12,
            ..p
        };
        assert_eq!(
            longue.verifier("onze-signes", "x"),
            Err("mot_de_passe.trop_court")
        );
        let laxiste = PolitiqueMotDePasse {
            longueur_min: 4,
            refuser_identifiant: false,
            refuser_courants: false,
        };
        assert_eq!(laxiste.verifier("abc", "x"), Err("mot_de_passe.trop_court"));
        assert_eq!(laxiste.verifier("password", "pass"), Ok(()));
    }
}
