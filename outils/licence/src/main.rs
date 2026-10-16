//! `gesco-licence` — outil de l'éditeur (blueprint §26.5, spécification §13).
//!
//! ```text
//! gesco-licence cles <fichier-cle-privee>
//! gesco-licence emettre --cle <fichier-cle-privee> --etablissement <nom> --empreinte <empreinte>
//!                       --sortie <fichier> [--id <identifiant>] [--edition essentielle|connectee]
//!                       [--modules a,b] [--echeance AAAA-MM-JJ]
//! gesco-licence lot --cle <fichier-cle-privee> --nombre <n> --dossier <dossier>
//!                   [--premier <numéro>] [--prefixe GESCO] [--edition essentielle|connectee]
//! ```
//!
//! `lot` émet des licences libres numérotées (`GESCO-0001.lic`…) : chacune s'installe sur
//! n'importe quel poste, qui s'y attache. Le registre `registre.csv` du dossier sert à noter
//! l'école qui reçoit chaque numéro.
//!
//! La clé privée reste sur le poste de l'éditeur, hors du dépôt. Seule la clé publique est livrée,
//! dans `crates/gesco-services/cle-publique-licence.txt`.

use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use gesco_core::commun::Horodatage;
use gesco_core::core::licence::{self, EMPREINTE_LIBRE, Edition, Licence};
use zeroize::Zeroizing;

const USAGE: &str = "usage :
  gesco-licence cles <fichier-cle-privee>
  gesco-licence emettre --cle <fichier> --etablissement <nom> --empreinte <empreinte> --sortie <fichier>
                        [--id <identifiant>] [--edition essentielle|connectee] [--modules a,b] [--echeance AAAA-MM-JJ]
  gesco-licence lot --cle <fichier> --nombre <n> --dossier <dossier>
                    [--premier <numéro>] [--prefixe GESCO] [--edition essentielle|connectee]";

/// Taille maximale d'un lot.
const LOT_MAX: u32 = 10_000;

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let issue = match arguments.split_first() {
        Some((commande, reste)) if commande == "cles" => generer_cles(reste),
        Some((commande, reste)) if commande == "emettre" => emettre(reste),
        Some((commande, reste)) if commande == "lot" => emettre_lot(reste),
        _ => Err(USAGE.to_owned()),
    };
    match issue {
        Ok(message) => {
            let _ = writeln!(std::io::stdout(), "{message}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            let _ = writeln!(std::io::stderr(), "{message}");
            ExitCode::FAILURE
        }
    }
}

/// Tire une graine de clé privée, l'écrit dans un fichier neuf et rend la clé publique.
fn generer_cles(arguments: &[String]) -> Result<String, String> {
    let [chemin] = arguments else {
        return Err(USAGE.to_owned());
    };
    let chemin = Path::new(chemin);
    if chemin.exists() {
        return Err(format!(
            "{} existe déjà : une clé de l'éditeur ne s'écrase jamais.",
            chemin.display()
        ));
    }
    let mut graine = Zeroizing::new([0u8; 32]);
    getrandom::fill(graine.as_mut()).map_err(|e| format!("aléa indisponible : {e}"))?;
    if let Some(dossier) = chemin.parent() {
        std::fs::create_dir_all(dossier).map_err(|e| e.to_string())?;
    }
    std::fs::write(chemin, hex::encode(graine.as_ref())).map_err(|e| e.to_string())?;
    Ok(format!(
        "Clé privée écrite dans {} — à sauvegarder hors ligne, jamais dans le dépôt.\nClé publique : {}",
        chemin.display(),
        hex::encode(licence::cle_publique(&graine))
    ))
}

/// Émet un fichier de licence signé.
fn emettre(arguments: &[String]) -> Result<String, String> {
    let options = options(arguments)?;
    let exiger = |nom: &str| {
        options
            .get(nom)
            .cloned()
            .ok_or_else(|| format!("--{nom} manquant\n{USAGE}"))
    };
    let graine = lire_graine(Path::new(&exiger("cle")?))?;
    let emise_le = maintenant()?;
    let edition = edition(&options)?;
    let licence = Licence {
        licence_id: options
            .get("id")
            .cloned()
            .unwrap_or_else(|| format!("LIC-{}", emise_le.millis_epoque())),
        etablissement: exiger("etablissement")?,
        edition,
        modules: options
            .get("modules")
            .map(|m| {
                m.split(',')
                    .map(str::trim)
                    .filter(|m| !m.is_empty())
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default(),
        empreinte_machine: exiger("empreinte")?.trim().to_uppercase(),
        emise_le,
        echeance: options.get("echeance").map(|d| date(d)).transpose()?,
    };
    let fichier = licence::signer(&licence, &graine).map_err(|e| e.to_string())?;
    // Contrôle avant remise : le fichier se vérifie avec la clé publique correspondante.
    licence::verifier(&fichier, &licence::cle_publique(&graine)).map_err(|e| e.to_string())?;
    let sortie = exiger("sortie")?;
    std::fs::write(&sortie, &fichier).map_err(|e| e.to_string())?;
    Ok(format!(
        "Licence {} émise pour « {} » ({}) : {sortie}",
        licence.licence_id, licence.etablissement, licence.empreinte_machine
    ))
}

/// Émet un lot de licences libres numérotées, chacune dans son fichier, et leur registre.
fn emettre_lot(arguments: &[String]) -> Result<String, String> {
    let options = options(arguments)?;
    let exiger = |nom: &str| {
        options
            .get(nom)
            .cloned()
            .ok_or_else(|| format!("--{nom} manquant\n{USAGE}"))
    };
    let graine = lire_graine(Path::new(&exiger("cle")?))?;
    let nombre: u32 = exiger("nombre")?
        .parse()
        .map_err(|_| "--nombre attend un entier".to_owned())?;
    let premier: u32 = options
        .get("premier")
        .map(|p| p.parse())
        .transpose()
        .map_err(|_| "--premier attend un entier".to_owned())?
        .unwrap_or(1);
    if nombre == 0 || nombre > LOT_MAX || premier == 0 {
        return Err(format!(
            "un lot compte de 1 à {LOT_MAX} licences, numérotées dès 1"
        ));
    }
    let prefixe = options
        .get("prefixe")
        .cloned()
        .unwrap_or_else(|| "GESCO".to_owned());
    let edition = edition(&options)?;
    let dossier = Path::new(&exiger("dossier")?).to_path_buf();
    std::fs::create_dir_all(&dossier).map_err(|e| e.to_string())?;
    let registre = dossier.join("registre.csv");
    let numeros: Vec<String> = (premier..premier.saturating_add(nombre))
        .map(|n| format!("{prefixe}-{n:04}"))
        .collect();
    // Rien n'est écrit si un seul fichier existe déjà : un numéro ne s'émet jamais deux fois.
    for chemin in numeros
        .iter()
        .map(|n| dossier.join(format!("{n}.lic")))
        .chain([registre.clone()])
    {
        if chemin.exists() {
            return Err(format!(
                "{} existe déjà : choisissez un autre dossier ou un autre --premier.",
                chemin.display()
            ));
        }
    }
    let emise_le = maintenant()?;
    let cle_publique = licence::cle_publique(&graine);
    let mut lignes = vec!["numero;fichier;edition;emise_le;ecole;remise_le".to_owned()];
    for numero in &numeros {
        let licence = Licence {
            licence_id: numero.clone(),
            etablissement: String::new(),
            edition,
            modules: Vec::new(),
            empreinte_machine: EMPREINTE_LIBRE.to_owned(),
            emise_le,
            echeance: None,
        };
        let fichier = licence::signer(&licence, &graine).map_err(|e| e.to_string())?;
        licence::verifier(&fichier, &cle_publique).map_err(|e| e.to_string())?;
        std::fs::write(dossier.join(format!("{numero}.lic")), &fichier)
            .map_err(|e| e.to_string())?;
        lignes.push(format!(
            "{numero};{numero}.lic;{};{};;",
            match edition {
                Edition::Essentielle => "essentielle",
                Edition::Connectee => "connectee",
            },
            emise_le.vers_rfc3339()
        ));
    }
    std::fs::write(&registre, lignes.join("\r\n") + "\r\n").map_err(|e| e.to_string())?;
    Ok(format!(
        "{} licences libres émises ({} à {}) dans {}
Clé publique : {}",
        numeros.len(),
        numeros.first().map(String::as_str).unwrap_or_default(),
        numeros.last().map(String::as_str).unwrap_or_default(),
        dossier.display(),
        hex::encode(cle_publique)
    ))
}

fn edition(options: &HashMap<String, String>) -> Result<Edition, String> {
    match options.get("edition").map(String::as_str) {
        None | Some("essentielle") => Ok(Edition::Essentielle),
        Some("connectee") => Ok(Edition::Connectee),
        Some(autre) => Err(format!("édition inconnue : {autre}")),
    }
}

fn options(arguments: &[String]) -> Result<HashMap<String, String>, String> {
    let mut options = HashMap::new();
    let mut suite = arguments.iter();
    while let Some(nom) = suite.next() {
        let nom = nom
            .strip_prefix("--")
            .ok_or_else(|| format!("argument inattendu : {nom}\n{USAGE}"))?;
        let valeur = suite
            .next()
            .ok_or_else(|| format!("--{nom} attend une valeur"))?;
        options.insert(nom.to_owned(), valeur.clone());
    }
    Ok(options)
}

fn lire_graine(chemin: &Path) -> Result<Zeroizing<[u8; 32]>, String> {
    let texte = Zeroizing::new(
        std::fs::read_to_string(chemin).map_err(|e| format!("{} : {e}", chemin.display()))?,
    );
    let octets = Zeroizing::new(hex::decode(texte.trim()).map_err(|e| e.to_string())?);
    let graine: [u8; 32] = octets
        .as_slice()
        .try_into()
        .map_err(|_| "clé privée mal formée".to_owned())?;
    Ok(Zeroizing::new(graine))
}

fn maintenant() -> Result<Horodatage, String> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_millis();
    Ok(Horodatage::depuis_millis_epoque(
        i64::try_from(millis).map_err(|e| e.to_string())?,
    ))
}

/// `AAAA-MM-JJ` → minuit UTC de ce jour.
fn date(texte: &str) -> Result<Horodatage, String> {
    let invalide = || format!("date invalide : {texte} (attendu AAAA-MM-JJ)");
    let parties: Vec<i64> = texte
        .split('-')
        .map(|p| p.parse().map_err(|_| invalide()))
        .collect::<Result<_, _>>()?;
    let [annee, mois, jour] = parties.as_slice() else {
        return Err(invalide());
    };
    if !(1..=12).contains(mois) || !(1..=31).contains(jour) {
        return Err(invalide());
    }
    // Jours depuis l'époque d'une date grégorienne (algorithme de H. Hinnant).
    let a = if *mois <= 2 { annee - 1 } else { *annee };
    let ere = a.div_euclid(400);
    let annee_ere = a - ere * 400;
    let m = if *mois > 2 { mois - 3 } else { mois + 9 };
    let jour_annee = (153 * m + 2) / 5 + jour - 1;
    let jour_ere = annee_ere * 365 + annee_ere / 4 - annee_ere / 100 + jour_annee;
    let jours = ere * 146_097 + jour_ere - 719_468;
    Ok(Horodatage::depuis_millis_epoque(jours * 86_400_000))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_dates_tombent_a_minuit_utc() -> Result<(), String> {
        assert_eq!(date("1970-01-01")?.millis_epoque(), 0);
        assert_eq!(
            date("2000-03-01")?.vers_rfc3339(),
            "2000-03-01T00:00:00.000Z"
        );
        assert_eq!(
            date("2027-10-04")?.vers_rfc3339(),
            "2027-10-04T00:00:00.000Z"
        );
        assert!(date("2027-13-01").is_err());
        assert!(date("demain").is_err());
        Ok(())
    }
}
