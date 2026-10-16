//! Empreinte de la machine (spécification §13) : dérivée du `MachineGuid` de Windows, stable tant
//! que Windows n'est pas réinstallé. Lue par `reg.exe` plutôt que par l'API du registre, ce qui
//! évite tout code `unsafe` (interdit dans l'espace de travail).

use gesco_core::core::licence::empreinte_machine;

/// Empreinte du poste ; à défaut d'identifiant lisible, celle du nom de l'ordinateur.
pub fn du_poste(nom_du_poste: &str) -> String {
    empreinte_machine(&identifiant_machine().unwrap_or_else(|| nom_du_poste.to_owned()))
}

#[cfg(windows)]
fn identifiant_machine() -> Option<String> {
    use std::os::windows::process::CommandExt;
    // CREATE_NO_WINDOW : pas de console qui clignote au démarrage.
    const SANS_FENETRE: u32 = 0x0800_0000;
    let sortie = std::process::Command::new("reg")
        .args([
            "query",
            r"HKLM\SOFTWARE\Microsoft\Cryptography",
            "/v",
            "MachineGuid",
        ])
        .creation_flags(SANS_FENETRE)
        .output()
        .ok()?;
    String::from_utf8_lossy(&sortie.stdout)
        .lines()
        .find(|l| l.contains("MachineGuid"))
        .and_then(|l| l.split_whitespace().last())
        .map(str::to_owned)
}

#[cfg(not(windows))]
fn identifiant_machine() -> Option<String> {
    std::fs::read_to_string("/etc/machine-id")
        .ok()
        .map(|s| s.trim().to_owned())
}
