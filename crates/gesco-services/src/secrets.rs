//! Empreintes Argon2id des mots de passe et des codes PIN (§6.1 de la spécification).
//!
//! Les paramètres par défaut d'`argon2` (19 Mio, deux passes) correspondent à la cible d'environ
//! 250 ms sur le matériel visé. Aucune valeur en clair n'est conservée ni journalisée (R-25).

use argon2::password_hash::{PasswordHasher, PasswordVerifier};
use argon2::{Argon2, PasswordHash};

use crate::erreur::{ErreurService, Resultat};

/// Longueur minimale d'un mot de passe. La politique complète est un paramètre d'établissement.
pub const LONGUEUR_MINIMALE_MOT_DE_PASSE: usize = gesco_core::core::mot_de_passe::LONGUEUR_PLANCHER;

/// Mot de passe saisi, jamais affiché.
pub struct MotDePasse<'a>(pub &'a str);

impl std::fmt::Debug for MotDePasse<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MotDePasse(****)")
    }
}

/// Code PIN saisi, jamais affiché.
pub struct Pin<'a>(pub &'a str);

impl std::fmt::Debug for Pin<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Pin(****)")
    }
}

impl Pin<'_> {
    /// Un PIN compte de 4 à 6 chiffres (§6.2).
    pub fn est_valide(&self) -> bool {
        (4..=6).contains(&self.0.chars().count()) && self.0.chars().all(|c| c.is_ascii_digit())
    }
}

fn hacher(secret: &[u8]) -> Resultat<String> {
    let empreinte: PasswordHash = Argon2::default()
        .hash_password(secret)
        .map_err(|e| ErreurService::Secret(e.to_string()))?;
    Ok(empreinte.to_string())
}

fn verifier(secret: &[u8], empreinte: &str) -> bool {
    Argon2::default().verify_password(secret, empreinte).is_ok()
}

/// Empreinte PHC d'un mot de passe.
pub fn empreinte_mot_de_passe(mot_de_passe: &MotDePasse<'_>) -> Resultat<String> {
    if mot_de_passe.0.chars().count() < LONGUEUR_MINIMALE_MOT_DE_PASSE {
        return Err(ErreurService::Regle {
            cle: "mot_de_passe.trop_court",
        });
    }
    hacher(mot_de_passe.0.as_bytes())
}

/// Vérifie un mot de passe contre son empreinte PHC.
pub fn mot_de_passe_correspond(mot_de_passe: &MotDePasse<'_>, empreinte: &str) -> bool {
    verifier(mot_de_passe.0.as_bytes(), empreinte)
}

/// Empreinte PHC d'un code PIN.
pub fn empreinte_pin(pin: &Pin<'_>) -> Resultat<String> {
    if !pin.est_valide() {
        return Err(ErreurService::Regle {
            cle: "pin.format_invalide",
        });
    }
    hacher(pin.0.as_bytes())
}

/// Vérifie un PIN contre son empreinte PHC.
pub fn pin_correspond(pin: &Pin<'_>, empreinte: &str) -> bool {
    verifier(pin.0.as_bytes(), empreinte)
}

/// Empreinte PHC d'un code d'accès de responsable (SPEC_PHASE_5 §9) : le code ne se garde pas.
pub fn empreinte_code_acces(code: &str) -> Resultat<String> {
    hacher(code.as_bytes())
}

/// Vérifie un code d'accès contre son empreinte PHC.
pub fn code_acces_correspond(code: &str, empreinte: &str) -> bool {
    verifier(code.as_bytes(), empreinte)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empreinte_verifiee_et_sel_different_a_chaque_calcul() -> Resultat<()> {
        let mdp = MotDePasse("correct-horse-battery");
        let a = empreinte_mot_de_passe(&mdp)?;
        let b = empreinte_mot_de_passe(&mdp)?;
        assert_ne!(a, b, "le sel doit être tiré au hasard");
        assert!(mot_de_passe_correspond(&mdp, &a));
        assert!(!mot_de_passe_correspond(
            &MotDePasse("autre-mot-de-passe"),
            &a
        ));
        Ok(())
    }

    #[test]
    fn mot_de_passe_trop_court_refuse() {
        let erreur = empreinte_mot_de_passe(&MotDePasse("court"));
        assert!(matches!(
            erreur,
            Err(ErreurService::Regle {
                cle: "mot_de_passe.trop_court"
            })
        ));
    }

    #[test]
    fn pin_de_quatre_a_six_chiffres() -> Resultat<()> {
        assert!(Pin("1234").est_valide());
        assert!(Pin("123456").est_valide());
        assert!(!Pin("123").est_valide());
        assert!(!Pin("1234567").est_valide());
        assert!(!Pin("12a4").est_valide());
        let e = empreinte_pin(&Pin("4321"))?;
        assert!(pin_correspond(&Pin("4321"), &e));
        assert!(!pin_correspond(&Pin("1234"), &e));
        Ok(())
    }

    #[test]
    fn les_secrets_ne_s_affichent_pas() {
        assert_eq!(
            format!("{:?}", MotDePasse("secret-1234")),
            "MotDePasse(****)"
        );
        assert_eq!(format!("{:?}", Pin("1234")), "Pin(****)");
    }
}
