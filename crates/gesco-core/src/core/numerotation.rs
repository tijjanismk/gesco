//! Numérotation métier des documents officiels (ADR-004, R-46) : séquentielle, sans trou,
//! réinitialisable par exercice. Le compteur est tenu en base sous verrou ; le format est ici.

use serde::{Deserialize, Serialize};

/// Documents dont le numéro est séquentiel et contrôlé.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TypeDocumentNumerote {
    /// Reçu de versement.
    Recu,
    /// Facture de scolarité.
    Facture,
    /// Certificat de transfert (Q-32).
    CertificatTransfert,
    /// Certificat de scolarité.
    CertificatScolarite,
    /// Bulletin de paie.
    BulletinPaie,
    /// Document de test du moteur documentaire (phase 0.5).
    Test,
    /// Matricule interne d'un élève : seul le compteur sert, la forme suit les règles de l'école
    /// (`scolarite::matricule`).
    MatriculeEleve,
}

impl TypeDocumentNumerote {
    /// Code persisté et préfixe par défaut.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Recu => "REC",
            Self::Facture => "FAC",
            Self::CertificatTransfert => "CTR",
            Self::CertificatScolarite => "CSC",
            Self::BulletinPaie => "PAI",
            Self::Test => "TST",
            Self::MatriculeEleve => "MAT",
        }
    }

    /// Lecture d'un code persisté.
    pub fn depuis_code(code: &str) -> Option<Self> {
        [
            Self::Recu,
            Self::Facture,
            Self::CertificatTransfert,
            Self::CertificatScolarite,
            Self::BulletinPaie,
            Self::Test,
            Self::MatriculeEleve,
        ]
        .into_iter()
        .find(|t| t.code() == code)
    }
}

/// Format d'un numéro : `PREFIXE-EXERCICE-COMPTEUR`, par exemple `REC-2026-000147`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormatNumero {
    /// Préfixe littéral.
    pub prefixe: String,
    /// Largeur minimale du compteur, complétée par des zéros.
    pub largeur: usize,
    /// Insère l'exercice entre le préfixe et le compteur.
    pub avec_exercice: bool,
}

impl FormatNumero {
    /// Format par défaut d'un type de document.
    pub fn par_defaut(type_document: TypeDocumentNumerote) -> Self {
        Self {
            prefixe: type_document.code().to_owned(),
            largeur: 6,
            avec_exercice: true,
        }
    }

    /// Compose le numéro.
    pub fn formater(&self, exercice: &str, compteur: u64) -> String {
        let compteur = format!("{compteur:0largeur$}", largeur = self.largeur);
        if self.avec_exercice {
            format!("{}-{exercice}-{compteur}", self.prefixe)
        } else {
            format!("{}-{compteur}", self.prefixe)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_par_defaut() {
        let f = FormatNumero::par_defaut(TypeDocumentNumerote::Recu);
        assert_eq!(f.formater("2026", 147), "REC-2026-000147");
        assert_eq!(f.formater("2026", 1_234_567), "REC-2026-1234567");
    }

    #[test]
    fn sans_exercice() {
        let f = FormatNumero {
            prefixe: "DOC".to_owned(),
            largeur: 3,
            avec_exercice: false,
        };
        assert_eq!(f.formater("ignoré", 7), "DOC-007");
        assert_eq!(
            TypeDocumentNumerote::depuis_code("FAC"),
            Some(TypeDocumentNumerote::Facture)
        );
    }
}
