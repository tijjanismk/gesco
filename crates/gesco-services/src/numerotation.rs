//! Numérotation des documents officiels (R-46, §9 de la spécification) : séquence sans trou,
//! attribution sous verrou dans la transaction de l'opération, abandon tracé.

use gesco_core::core::{Operation, TypeDocumentNumerote};
use gesco_db::UniteDeTravail;
use gesco_db::depots::sequence::{self, NumeroAbandonne};

use crate::documents::date_longue;
use crate::effets::Effet;
use crate::erreur::{ErreurService, Resultat};

/// Numéro attribué dans la transaction courante.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumeroAttribue {
    /// Numéro formaté (`REC-2026-000147`).
    pub numero: String,
    /// Type de document.
    pub type_document: TypeDocumentNumerote,
    /// Exercice.
    pub exercice: String,
}

/// Attribue le prochain numéro de la séquence, dans la transaction de l'unité de travail. Si la
/// transaction est annulée, le compteur n'avance pas : la séquence reste sans trou.
pub async fn attribuer(
    unite: &mut UniteDeTravail<Effet>,
    type_document: TypeDocumentNumerote,
    exercice: &str,
) -> Resultat<NumeroAttribue> {
    let contexte = unite.contexte().clone();
    let numero =
        sequence::prochain_numero(unite.connexion(), type_document, exercice, &contexte).await?;
    Ok(NumeroAttribue {
        numero,
        type_document,
        exercice: exercice.to_owned(),
    })
}

/// Avance le compteur d'une séquence dans la transaction courante, sans composer de numéro : la
/// forme revient à l'appelant (matricule d'un élève).
pub async fn attribuer_compteur(
    unite: &mut UniteDeTravail<Effet>,
    type_document: TypeDocumentNumerote,
    exercice: &str,
) -> Resultat<u64> {
    let contexte = unite.contexte().clone();
    Ok(
        sequence::prochain_compteur(unite.connexion(), type_document, exercice, &contexte)
            .await?
            .0,
    )
}

/// Abandonne un numéro déjà attribué dans une transaction validée : la trace « annulé » est
/// écrite et auditée, un document annulé est demandé au moteur documentaire.
pub async fn abandonner(
    unite: &mut UniteDeTravail<Effet>,
    numero: NumeroAttribue,
    motif: &str,
) -> Resultat<()> {
    if motif.trim().is_empty() {
        return Err(ErreurService::Regle {
            cle: "numerotation.motif_obligatoire",
        });
    }
    let contexte = unite.contexte().clone();
    let abandon = NumeroAbandonne {
        numero: numero.numero.clone(),
        type_document: numero.type_document,
        exercice: numero.exercice.clone(),
        motif: motif.to_owned(),
    };
    sequence::abandonner(unite.connexion(), &abandon, &contexte).await?;
    unite.auditer(
        "numero",
        &numero.numero,
        Operation::Annulation,
        None,
        Some(serde_json::json!({
            "type_document": numero.type_document.code(),
            "exercice": numero.exercice,
            "motif": motif,
        })),
        "numerotation.abandonner",
    );
    unite.planifier(Effet::ComposerDocument {
        type_document: "numero_annule".to_owned(),
        entite: "numero".to_owned(),
        entite_id: numero.numero.clone(),
        donnees: serde_json::json!({
            "numero": numero.numero,
            "type_document": numero.type_document.code(),
            "exercice": numero.exercice,
            "motif": motif,
            "date": date_longue(contexte.maintenant),
        }),
    });
    Ok(())
}
