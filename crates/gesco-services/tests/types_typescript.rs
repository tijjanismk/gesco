//! Types TypeScript générés depuis Rust (ADR-007, R-37, spécification §12.3). Le test échoue si
//! les fichiers versionnés de `frontend/src/types/` diffèrent de la génération : modifier une
//! structure Rust casse alors la CI, puis la compilation du frontend une fois régénérés.
//!
//! Régénération : `GESCO_REGENERER_TYPES=1 cargo test -p gesco-services --test types_typescript`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::borrow::Cow;
use std::path::PathBuf;

use gesco_services::commandes::{Acces, COMMANDES};
use gesco_services::frontiere::{Contrat, ErreurCommande};
use specta::datatype::{DataType, Fields, NamedReferenceType, Primitive, Reference};
use specta::{Format, FormatError, Types};

const ENTETE: &str = "// Fichier généré depuis Rust par `cargo test -p gesco-services --test types_typescript`.\n// Ne pas modifier à la main (R-37) : toute modification est écrasée à la régénération.\n";

const VALEUR_JSON: &str = "\n/** Valeur JSON libre : valeurs avant et après d'une ligne d'audit. */\nexport type ValeurJson = null | boolean | number | string | ValeurJson[] | { [cle: string]: ValeurJson };\n";

/// Format serde de specta, et entiers 64 bits rendus en `number` : identifiants et montants
/// voyagent en texte ; horodatages en millisecondes et tailles de fichiers restent sous 2^53.
struct FormatGesco;

static SERDE: specta_serde::Format = specta_serde::Format;

fn convertir_champs(champs: &mut Fields) {
    match champs {
        Fields::Unit => {}
        Fields::Unnamed(c) => c
            .fields
            .iter_mut()
            .filter_map(|f| f.ty.as_mut())
            .for_each(convertir),
        Fields::Named(c) => c
            .fields
            .iter_mut()
            .filter_map(|(_, f)| f.ty.as_mut())
            .for_each(convertir),
    }
}

/// Vrai pour la définition que specta donne de `serde_json::Value` (valeurs avant et après de
/// l'audit). Elle est récursive, ce que l'export en ligne refuse : elle devient `ValeurJson`.
fn est_valeur_json(dt: &DataType) -> bool {
    matches!(dt, DataType::Enum(e) if e.variants.iter().map(|(nom, _)| nom.as_ref()).eq(
        ["Null", "Bool", "Number", "String", "Array", "Object"]
    ))
}

fn convertir(dt: &mut DataType) {
    if let DataType::Reference(Reference::Named(r)) = dt
        && let NamedReferenceType::Inline { dt: interne, .. } = &r.inner
        && est_valeur_json(interne)
    {
        *dt = DataType::Reference(specta_typescript::define("ValeurJson"));
        return;
    }
    match dt {
        DataType::Primitive(
            Primitive::i64 | Primitive::u64 | Primitive::isize | Primitive::usize,
        ) => {
            *dt = DataType::Reference(specta_typescript::define("number"));
        }
        DataType::List(l) => convertir(&mut l.ty),
        DataType::Map(m) => {
            convertir(m.key_ty_mut());
            convertir(m.value_ty_mut());
        }
        DataType::Struct(s) => convertir_champs(&mut s.fields),
        DataType::Enum(e) => e
            .variants
            .iter_mut()
            .for_each(|(_, v)| convertir_champs(&mut v.fields)),
        DataType::Tuple(t) => t.elements.iter_mut().for_each(convertir),
        DataType::Nullable(n) => convertir(n),
        DataType::Intersection(v) => v.iter_mut().for_each(convertir),
        DataType::Reference(Reference::Named(r)) => match &mut r.inner {
            NamedReferenceType::Inline { dt, .. } => convertir(dt),
            NamedReferenceType::Reference { generics, .. } => {
                generics.iter_mut().for_each(|(_, dt)| convertir(dt));
            }
            NamedReferenceType::Recursive(_) => {}
        },
        DataType::Primitive(_)
        | DataType::Generic(_)
        | DataType::Reference(Reference::Opaque(_)) => {}
    }
}

impl Format for FormatGesco {
    fn map_types(&'_ self, types: &Types) -> Result<Cow<'_, Types>, FormatError> {
        let mut types = SERDE.map_types(types)?.into_owned();
        types.iter_mut(|t| {
            if let Some(dt) = t.ty.as_mut() {
                convertir(dt);
            }
        });
        Ok(Cow::Owned(types))
    }

    fn map_type(&'_ self, types: &Types, dt: &DataType) -> Result<Cow<'_, DataType>, FormatError> {
        let mut dt = SERDE.map_type(types, dt)?.into_owned();
        convertir(&mut dt);
        Ok(Cow::Owned(dt))
    }
}

fn dossier_types() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../frontend/src/types")
}

fn verifier_ou_ecrire(nom: &str, attendu: &str) {
    let chemin = dossier_types().join(nom);
    if std::env::var_os("GESCO_REGENERER_TYPES").is_some() {
        std::fs::create_dir_all(dossier_types()).unwrap();
        std::fs::write(&chemin, attendu).unwrap();
        return;
    }
    let versionne = std::fs::read_to_string(&chemin).unwrap_or_default();
    assert!(
        versionne == attendu,
        "{nom} est périmé : relancer avec GESCO_REGENERER_TYPES=1 et versionner le résultat"
    );
}

#[test]
fn les_types_du_contrat_sont_a_jour() {
    let types = Types::default()
        .register::<Contrat>()
        .register::<ErreurCommande>();
    let genere = specta_typescript::Typescript::default()
        .header(format!("{ENTETE}{VALEUR_JSON}"))
        .export(&types, FormatGesco)
        .unwrap();
    verifier_ou_ecrire("gesco.ts", &genere);
}

#[test]
fn la_table_des_acces_est_a_jour() {
    let mut texte = String::from(ENTETE);
    texte.push_str(
        "\n/** Accès exigé par chaque commande ; la garde Rust reste la seule décision (R-01, R-21). */\n",
    );
    texte.push_str("export const ACCES = {\n");
    for commande in COMMANDES {
        let acces = match commande.acces {
            Acces::Public => "{ type: \"public\" }".to_owned(),
            Acces::Connecte => "{ type: \"connecte\" }".to_owned(),
            Acces::Permission(p) => {
                format!("{{ type: \"permission\", permission: \"{}\" }}", p.code())
            }
        };
        texte.push_str(&format!("  {}: {acces},\n", commande.nom));
    }
    texte.push_str("} as const;\n\nexport type NomCommande = keyof typeof ACCES;\n");
    verifier_ou_ecrire("acces.ts", &texte);
}
