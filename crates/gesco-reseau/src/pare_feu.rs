//! Règle du pare-feu de Windows pour le poste principal (SPEC_PHASE_11_A §10) : les postes
//! reliés entrent par le port du réseau de l'école, vers GESCO seulement, et seulement sur les
//! réseaux que Windows range parmi les réseaux privés.
//!
//! Ce module ne lance rien : il compose les commandes à passer à Windows et lit leurs réponses.
//! `gesco-app` les exécute. GESCO s'installe sans droits d'administrateur ; poser la règle en
//! demande, d'où la question que Windows pose à la personne au moment d'ouvrir le pare-feu.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;

/// Nom de la règle dans le pare-feu de Windows. Sans accent ni apostrophe : il traverse des
/// lignes de commande et se lit dans la console de Windows.
pub const NOM_REGLE: &str = "GESCO postes relies";

/// Programme qui lit et modifie le pare-feu de Windows.
pub const NETSH: &str = "netsh";

/// Interpréteur de Windows, pour demander les droits d'administrateur et lire le réseau.
pub const POWERSHELL: &str = "powershell";

/// Arguments de `netsh` pour lire la règle. Se lit sans droits d'administrateur ; `netsh`
/// répond par un code de sortie non nul quand la règle n'existe pas.
pub fn lecture() -> Vec<String> {
    [
        "advfirewall",
        "firewall",
        "show",
        "rule",
        &format!("name={NOM_REGLE}"),
        "verbose",
    ]
    .map(str::to_owned)
    .to_vec()
}

/// Chaîne entre apostrophes de PowerShell : rien n'y est interprété, l'apostrophe se double.
fn litteral(texte: &str) -> String {
    format!("'{}'", texte.replace('\'', "''"))
}

/// Script joué avec les droits d'administrateur : il retire l'ancienne règle de ce nom (un
/// GESCO installé ailleurs, un autre port), puis pose la règle pour ce programme et ce port.
pub fn script_ouverture(programme: &str, port: u16) -> String {
    let nom = litteral(&format!("name={NOM_REGLE}"));
    let programme = litteral(&format!("program={programme}"));
    format!(
        "& {NETSH} advfirewall firewall delete rule {nom} | Out-Null\n\
         & {NETSH} advfirewall firewall add rule {nom} dir=in action=allow protocol=TCP \
         localport={port} profile=private {programme} | Out-Null\n\
         exit $LASTEXITCODE\n"
    )
}

/// Arguments de PowerShell pour jouer [`script_ouverture`] avec les droits d'administrateur :
/// Windows demande alors son accord à la personne. Le script voyage codé (UTF-16, base64) : le
/// chemin du programme, avec ses espaces et ses accents, ne passe par aucune ligne de commande.
pub fn ouverture(programme: &str, port: u16) -> Vec<String> {
    let octets: Vec<u8> = script_ouverture(programme, port)
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    let code = STANDARD.encode(octets);
    let commande = format!(
        "$p = Start-Process {POWERSHELL} -Verb RunAs -WindowStyle Hidden -Wait -PassThru \
         -ArgumentList '-NoProfile','-NonInteractive','-EncodedCommand','{code}'; exit $p.ExitCode"
    );
    ["-NoProfile", "-NonInteractive", "-Command", &commande]
        .map(str::to_owned)
        .to_vec()
}

/// Arguments de PowerShell pour lire comment Windows range chaque réseau où l'ordinateur est
/// branché : `Public`, `Private` ou `DomainAuthenticated`, un par ligne, jamais traduits.
pub fn lecture_des_reseaux() -> Vec<String> {
    [
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        "(Get-NetConnectionProfile).NetworkCategory",
    ]
    .map(str::to_owned)
    .to_vec()
}

/// Lettres et chiffres sans accent, en minuscules : la console de Windows rend les lettres
/// accentuées d'un chemin dans son propre codage, qu'on ne cherche pas à relire.
fn reduit(texte: &str) -> String {
    texte
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// Vrai si la réponse de `netsh` ([`lecture`]) décrit une règle pour ce programme et ce port.
/// Les libellés `Program` et `LocalPort` ne sont pas traduits par Windows.
pub fn regle_posee(reponse: &str, programme: &str, port: u16) -> bool {
    let attendu = reduit(programme);
    let port = port.to_string();
    !attendu.is_empty()
        && valeurs(reponse, "Program").any(|v| reduit(v) == attendu)
        && valeurs(reponse, "LocalPort").any(|v| v == port)
}

/// Valeurs des lignes « libellé : valeur » de la réponse de `netsh` qui portent ce libellé.
fn valeurs<'a>(reponse: &'a str, libelle: &'a str) -> impl Iterator<Item = &'a str> {
    reponse.lines().filter_map(move |ligne| {
        let (nom, valeur) = ligne.split_once(':')?;
        (nom.trim() == libelle).then_some(valeur.trim())
    })
}

/// Vrai si l'un des réseaux de l'ordinateur est rangé parmi les réseaux publics : la règle,
/// limitée aux réseaux privés, n'y laisse entrer personne.
pub fn reseau_public(reponse: &str) -> bool {
    reponse
        .lines()
        .any(|ligne| ligne.trim().eq_ignore_ascii_case("Public"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROGRAMME: &str = r"C:\Users\Awa Traoré\AppData\Local\GESCO\gesco-app.exe";

    /// Réponse de `netsh … show rule … verbose` sur un Windows en français, les lettres
    /// accentuées rendues comme la console les rend à un programme qui lit de l'UTF-8.
    const REPONSE: &str = "\r\n\
        Nom de la r\u{fffd}gle :                     GESCO postes relies\r\n\
        ----------------------------------------------------------------------\r\n\
        Activ\u{fffd} :                              Oui\r\n\
        Direction :                           Actif\r\n\
        Profiles :                            Priv\u{fffd}\r\n\
        LocalIP :                             Tout\r\n\
        Protocole :                           TCP\r\n\
        LocalPort :                           47800\r\n\
        RemotePort :                          Tout\r\n\
        Program :                             C:\\Users\\Awa Traor\u{fffd}\\AppData\\Local\\GESCO\\gesco-app.exe\r\n\
        Action :                              Autoriser\r\n\
        Ok.\r\n";

    #[test]
    fn la_regle_lue_est_reconnue_pour_ce_programme_et_ce_port() {
        assert!(regle_posee(REPONSE, PROGRAMME, 47_800));
        // La casse du chemin ne compte pas.
        assert!(regle_posee(REPONSE, &PROGRAMME.to_uppercase(), 47_800));
    }

    #[test]
    fn un_autre_port_ou_un_autre_programme_ne_vaut_pas() {
        assert!(!regle_posee(REPONSE, PROGRAMME, 47_801));
        assert!(!regle_posee(REPONSE, r"D:\GESCO\gesco-app.exe", 47_800));
        assert!(!regle_posee(REPONSE, "", 47_800));
    }

    #[test]
    fn sans_regle_netsh_ne_decrit_rien() {
        let absente = "\r\nAucune r\u{fffd}gle ne correspond aux crit\u{fffd}res sp\u{fffd}cifi\u{fffd}s.\r\n";
        assert!(!regle_posee(absente, PROGRAMME, 47_800));
        assert!(!regle_posee("", PROGRAMME, 47_800));
    }

    #[test]
    fn le_script_pose_la_regle_sur_les_reseaux_prives_seulement() {
        let script = script_ouverture(r"C:\Users\N'Golo\GESCO\gesco-app.exe", 47_800);
        let lignes: Vec<&str> = script.lines().collect();
        assert_eq!(
            lignes,
            [
                "& netsh advfirewall firewall delete rule 'name=GESCO postes relies' | Out-Null",
                "& netsh advfirewall firewall add rule 'name=GESCO postes relies' dir=in \
                 action=allow protocol=TCP localport=47800 profile=private \
                 'program=C:\\Users\\N''Golo\\GESCO\\gesco-app.exe' | Out-Null",
                "exit $LASTEXITCODE",
            ]
        );
        assert!(!script.contains("public"));
    }

    #[test]
    fn l_ouverture_porte_le_script_code_sans_rien_perdre() {
        let arguments = ouverture(PROGRAMME, 47_800);
        let commande = arguments.last().map(String::as_str).unwrap_or_default();
        assert!(commande.contains("-Verb RunAs"));
        let code = commande.split('\'').rev().nth(1).unwrap_or_default();
        let octets = STANDARD.decode(code).unwrap_or_default();
        let unites: Vec<u16> = octets
            .as_chunks::<2>()
            .0
            .iter()
            .map(|paire| u16::from_le_bytes(*paire))
            .collect();
        assert_eq!(
            String::from_utf16(&unites).unwrap_or_default(),
            script_ouverture(PROGRAMME, 47_800)
        );
    }

    #[test]
    fn la_lecture_nomme_la_regle_que_l_ouverture_pose() {
        let nom = format!("name={NOM_REGLE}");
        assert!(lecture().contains(&nom));
        assert!(script_ouverture(PROGRAMME, 47_800).contains(&nom));
    }

    #[test]
    fn un_reseau_public_se_reconnait_parmi_plusieurs() {
        assert!(reseau_public("Public\r\n"));
        assert!(reseau_public("Private\r\nPublic\r\n"));
        assert!(!reseau_public("Private\r\n"));
        assert!(!reseau_public("DomainAuthenticated\r\nPrivate\r\n"));
        assert!(!reseau_public(""));
    }
}
