//! Dossiers synchronisés du poste (copie hors de l'école) : les dossiers que Google Drive,
//! OneDrive, Dropbox ou un autre logiciel envoient sur un compte en ligne. GESCO les cherche pour
//! le compte Windows qui l'utilise, et propose leur sous-dossier `GESCO`. Trouver un dossier n'est
//! jamais une condition : l'école peut toujours en choisir un autre à la main.
//!
//! Ce qui se lit dans le système (variables, fichiers, registre, lecteurs) passe par un trait :
//! le binaire branche la lecture de Windows (registre par `reg.exe`, sans code `unsafe`) ; les
//! essais disent ce que Windows répondrait.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::Serialize;

/// Nom du sous-dossier proposé dans un dossier trouvé.
pub const SOUS_DOSSIER: &str = "GESCO";

/// Clé du registre où OneDrive déclare ses comptes.
pub const CLE_ONEDRIVE: &str = r"HKCU\Software\Microsoft\OneDrive\Accounts";
/// Clé du registre où les logiciels de synchronisation déclarent leurs racines à Windows.
pub const CLE_RACINES: &str =
    r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\SyncRootManager";

/// Logiciel qui envoie le dossier sur un compte en ligne.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum FournisseurEnLigne {
    /// Google Drive pour ordinateur.
    GoogleDrive,
    /// OneDrive, compte personnel ou professionnel.
    OneDrive,
    /// Dropbox.
    Dropbox,
    /// Un autre logiciel qui déclare sa racine à Windows.
    Autre,
}

impl FournisseurEnLigne {
    /// Nom affiché du fournisseur.
    pub fn nom(self) -> &'static str {
        match self {
            Self::GoogleDrive => "Google Drive",
            Self::OneDrive => "OneDrive",
            Self::Dropbox => "Dropbox",
            Self::Autre => "",
        }
    }

    fn rang(self) -> u8 {
        match self {
            Self::GoogleDrive => 0,
            Self::OneDrive => 1,
            Self::Dropbox => 2,
            Self::Autre => 3,
        }
    }
}

/// Un dossier synchronisé trouvé sur le poste.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct DossierSynchronise {
    /// Logiciel qui l'envoie.
    pub fournisseur: FournisseurEnLigne,
    /// Nom affiché : celui du fournisseur, ou celui qu'il donne à Windows.
    pub nom: String,
    /// Racine du compte sur le poste.
    pub racine: String,
    /// Dossier proposé : le sous-dossier `GESCO` de la racine.
    pub propose: String,
}

/// Ce que GESCO lit du système pour trouver les dossiers.
pub trait SystemeDossiers: Send + Sync {
    /// Variable d'environnement du compte Windows.
    fn variable(&self, nom: &str) -> Option<String>;
    /// Vrai si le chemin est un dossier qui existe.
    fn est_dossier(&self, chemin: &Path) -> bool;
    /// Contenu d'un petit fichier texte (réglages d'un fournisseur).
    fn lire_texte(&self, chemin: &Path) -> Option<String>;
    /// Sortie de `reg query <clé> /s` ; rien si la clé n'existe pas ou ne se lit pas.
    fn registre(&self, cle: &str) -> Option<String>;
    /// Identifiant (SID) du compte Windows qui fait tourner GESCO.
    fn compte(&self) -> Option<String>;
    /// Lettres des lecteurs présents sur le poste.
    fn lecteurs(&self) -> Vec<char>;
}

/// Lecture simulée : variables, registre, compte et lecteurs posés par les essais ; les dossiers
/// et les fichiers sont ceux du disque. Sans rien de posé, rien n'est trouvé.
#[derive(Debug, Default)]
pub struct SystemeSimule {
    variables: Mutex<HashMap<String, String>>,
    registre: Mutex<HashMap<String, String>>,
    compte: Mutex<Option<String>>,
    lecteurs: Mutex<Vec<char>>,
}

impl SystemeSimule {
    /// Pose une variable d'environnement ; rien l'efface.
    pub fn variable_posee(&self, nom: &str, valeur: Option<&str>) {
        if let Ok(mut variables) = self.variables.lock() {
            match valeur {
                Some(v) => {
                    variables.insert(nom.to_owned(), v.to_owned());
                }
                None => {
                    variables.remove(nom);
                }
            }
        }
    }

    /// Pose la sortie de `reg query <clé> /s`.
    pub fn registre_pose(&self, cle: &str, sortie: &str) {
        if let Ok(mut registre) = self.registre.lock() {
            registre.insert(cle.to_owned(), sortie.to_owned());
        }
    }

    /// Pose le compte Windows.
    pub fn compte_pose(&self, compte: &str) {
        if let Ok(mut c) = self.compte.lock() {
            *c = Some(compte.to_owned());
        }
    }

    /// Pose les lettres des lecteurs.
    pub fn lecteurs_poses(&self, lettres: &[char]) {
        if let Ok(mut l) = self.lecteurs.lock() {
            *l = lettres.to_vec();
        }
    }
}

impl SystemeDossiers for SystemeSimule {
    fn variable(&self, nom: &str) -> Option<String> {
        self.variables.lock().ok()?.get(nom).cloned()
    }

    fn est_dossier(&self, chemin: &Path) -> bool {
        chemin.is_dir()
    }

    fn lire_texte(&self, chemin: &Path) -> Option<String> {
        std::fs::read_to_string(chemin).ok()
    }

    fn registre(&self, cle: &str) -> Option<String> {
        self.registre.lock().ok()?.get(cle).cloned()
    }

    fn compte(&self) -> Option<String> {
        self.compte.lock().ok()?.clone()
    }

    fn lecteurs(&self) -> Vec<char> {
        self.lecteurs.lock().map(|l| l.clone()).unwrap_or_default()
    }
}

/// Une clé lue dans la sortie de `reg query /s`, avec ses valeurs (nom, donnée).
#[derive(Debug, Clone, PartialEq, Eq)]
struct CleRegistre {
    chemin: String,
    valeurs: Vec<(String, String)>,
}

/// Lit la sortie de `reg query <clé> /s` : une ligne `HKEY_…` par clé, puis ses valeurs, une par
/// ligne en retrait, « nom    REG_SZ    donnée ». Les valeurs qui ne sont pas du texte sont
/// ignorées.
fn lire_registre(sortie: &str) -> Vec<CleRegistre> {
    let mut cles: Vec<CleRegistre> = Vec::new();
    for ligne in sortie.lines() {
        if ligne.starts_with("HKEY_") {
            cles.push(CleRegistre {
                chemin: ligne.trim().to_owned(),
                valeurs: Vec::new(),
            });
            continue;
        }
        if !ligne.starts_with(' ') {
            continue;
        }
        let mut parties = ligne.trim().splitn(3, "    ");
        let (Some(nom), Some(sorte), Some(donnee)) =
            (parties.next(), parties.next(), parties.next())
        else {
            continue;
        };
        if !matches!(sorte, "REG_SZ" | "REG_EXPAND_SZ") {
            continue;
        }
        if let Some(cle) = cles.last_mut() {
            cle.valeurs
                .push((nom.trim().to_owned(), donnee.trim().to_owned()));
        }
    }
    cles
}

/// Forme d'un chemin pour reconnaître deux fois le même dossier.
fn forme(chemin: &str) -> String {
    chemin
        .trim()
        .trim_end_matches(['\\', '/'])
        .replace('/', "\\")
        .to_lowercase()
}

/// Les dossiers trouvés, dans l'ordre du guide : Google Drive, OneDrive, Dropbox, puis les
/// autres ; chacun une seule fois, et seulement s'il existe.
pub fn chercher(systeme: &dyn SystemeDossiers) -> Vec<DossierSynchronise> {
    let mut trouves: Vec<(FournisseurEnLigne, String, String)> = Vec::new();
    google_drive(systeme, &mut trouves);
    onedrive(systeme, &mut trouves);
    dropbox(systeme, &mut trouves);
    racines_windows(systeme, &mut trouves);
    let mut vus = HashSet::new();
    let mut dossiers: Vec<DossierSynchronise> = trouves
        .into_iter()
        .filter(|(_, _, racine)| !racine.trim().is_empty())
        .filter(|(_, _, racine)| systeme.est_dossier(Path::new(racine)))
        .filter(|(_, _, racine)| vus.insert(forme(racine)))
        .map(|(fournisseur, nom, racine)| {
            let racine = racine.trim().trim_end_matches(['\\', '/']).to_owned();
            DossierSynchronise {
                fournisseur,
                nom,
                propose: Path::new(&racine)
                    .join(SOUS_DOSSIER)
                    .to_string_lossy()
                    .into_owned(),
                racine,
            }
        })
        .collect();
    dossiers.sort_by_key(|d| d.fournisseur.rang());
    dossiers
}

fn ajouter(
    trouves: &mut Vec<(FournisseurEnLigne, String, String)>,
    fournisseur: FournisseurEnLigne,
    racine: String,
) {
    trouves.push((fournisseur, fournisseur.nom().to_owned(), racine));
}

/// Google Drive : « Mon Drive » ou « My Drive » sur le lecteur de Google Drive, ou le dossier en
/// miroir sous le profil.
fn google_drive(
    systeme: &dyn SystemeDossiers,
    trouves: &mut Vec<(FournisseurEnLigne, String, String)>,
) {
    const NOMS: [&str; 2] = ["Mon Drive", "My Drive"];
    for lettre in systeme.lecteurs() {
        for nom in NOMS {
            let chemin = format!("{lettre}:\\{nom}");
            if systeme.est_dossier(Path::new(&chemin)) {
                ajouter(trouves, FournisseurEnLigne::GoogleDrive, chemin);
            }
        }
    }
    if let Some(profil) = systeme.variable("USERPROFILE") {
        for nom in NOMS.into_iter().chain(["Google Drive"]) {
            let chemin = Path::new(&profil).join(nom);
            ajouter(
                trouves,
                FournisseurEnLigne::GoogleDrive,
                chemin.to_string_lossy().into_owned(),
            );
        }
    }
}

/// OneDrive : les variables d'environnement, puis les comptes déclarés dans son registre.
fn onedrive(
    systeme: &dyn SystemeDossiers,
    trouves: &mut Vec<(FournisseurEnLigne, String, String)>,
) {
    for variable in ["OneDriveConsumer", "OneDriveCommercial", "OneDrive"] {
        if let Some(chemin) = systeme.variable(variable) {
            ajouter(trouves, FournisseurEnLigne::OneDrive, chemin);
        }
    }
    if let Some(sortie) = systeme.registre(CLE_ONEDRIVE) {
        for cle in lire_registre(&sortie) {
            for (nom, donnee) in cle.valeurs {
                if nom.eq_ignore_ascii_case("UserFolder") {
                    ajouter(trouves, FournisseurEnLigne::OneDrive, donnee);
                }
            }
        }
    }
}

/// Dropbox : le fichier `info.json` qu'il tient dans le profil, un compte personnel et un compte
/// professionnel au plus.
fn dropbox(systeme: &dyn SystemeDossiers, trouves: &mut Vec<(FournisseurEnLigne, String, String)>) {
    for variable in ["APPDATA", "LOCALAPPDATA"] {
        let Some(base) = systeme.variable(variable) else {
            continue;
        };
        let fichier = Path::new(&base).join("Dropbox").join("info.json");
        let Some(texte) = systeme.lire_texte(&fichier) else {
            continue;
        };
        let Ok(serde_json::Value::Object(comptes)) = serde_json::from_str(&texte) else {
            continue;
        };
        for compte in comptes.values() {
            if let Some(chemin) = compte.get("path").and_then(serde_json::Value::as_str) {
                ajouter(trouves, FournisseurEnLigne::Dropbox, chemin.to_owned());
            }
        }
    }
}

/// Les racines de synchronisation déclarées à Windows, pour ce compte Windows : la clé
/// `<fournisseur>!<compte>!<nom>` porte le nom affiché, sa sous-clé `UserSyncRoots` le dossier
/// de chaque compte Windows.
fn racines_windows(
    systeme: &dyn SystemeDossiers,
    trouves: &mut Vec<(FournisseurEnLigne, String, String)>,
) {
    let Some(sortie) = systeme.registre(CLE_RACINES) else {
        return;
    };
    let compte = systeme.compte();
    let profil = systeme.variable("USERPROFILE");
    let cles = lire_registre(&sortie);
    let mut noms: HashMap<String, String> = HashMap::new();
    for cle in &cles {
        if let Some((_, nom)) = cle
            .valeurs
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case("DisplayNameResource"))
        {
            noms.insert(cle.chemin.to_lowercase(), nom.clone());
        }
    }
    for cle in &cles {
        let minuscule = cle.chemin.to_lowercase();
        let Some(racine) = minuscule.strip_suffix("\\usersyncroots") else {
            continue;
        };
        let identifiant = cle
            .chemin
            .get(..racine.len())
            .and_then(|c| c.rsplit('\\').next())
            .unwrap_or_default();
        let prefixe = identifiant.split('!').next().unwrap_or_default();
        let fournisseur = fournisseur_de(prefixe);
        let nom = match fournisseur {
            FournisseurEnLigne::Autre => noms
                .get(racine)
                .filter(|n| !n.is_empty() && !n.starts_with('@'))
                .cloned()
                .unwrap_or_else(|| prefixe.to_owned()),
            connu => connu.nom().to_owned(),
        };
        for (sid, chemin) in &cle.valeurs {
            if du_compte(sid, chemin, compte.as_deref(), profil.as_deref()) {
                trouves.push((fournisseur, nom.clone(), chemin.clone()));
            }
        }
    }
}

fn fournisseur_de(prefixe: &str) -> FournisseurEnLigne {
    let p = prefixe.to_lowercase();
    if p.contains("onedrive") {
        FournisseurEnLigne::OneDrive
    } else if p.contains("google") {
        FournisseurEnLigne::GoogleDrive
    } else if p.contains("dropbox") {
        FournisseurEnLigne::Dropbox
    } else {
        FournisseurEnLigne::Autre
    }
}

/// Vrai si la racine appartient au compte Windows qui fait tourner GESCO. Sans identifiant de
/// compte, une racine sous le profil d'un autre compte n'est pas proposée.
fn du_compte(sid: &str, chemin: &str, compte: Option<&str>, profil: Option<&str>) -> bool {
    if let Some(compte) = compte {
        return sid.eq_ignore_ascii_case(compte);
    }
    let Some(profil) = profil else {
        return true;
    };
    let chemin = forme(chemin);
    let profil = forme(profil);
    if chemin.starts_with(&profil) {
        return true;
    }
    let profils = PathBuf::from(&profil)
        .parent()
        .map(|p| forme(&p.to_string_lossy()))
        .unwrap_or_default();
    profils.is_empty() || !chemin.starts_with(&profils)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn dossier(racine: &Path, nom: &str) -> String {
        let chemin = racine.join(nom);
        std::fs::create_dir_all(&chemin).unwrap();
        chemin.to_string_lossy().into_owned()
    }

    #[test]
    fn rien_de_pose_ne_donne_rien() {
        assert!(chercher(&SystemeSimule::default()).is_empty());
    }

    #[test]
    fn onedrive_se_trouve_par_sa_variable_et_son_registre_une_seule_fois() {
        let base = tempfile::tempdir().unwrap();
        let perso = dossier(base.path(), "OneDrive");
        let pro = dossier(base.path(), "OneDrive - Ecole");
        let systeme = SystemeSimule::default();
        systeme.variable_posee("OneDrive", Some(&perso));
        systeme.registre_pose(
            CLE_ONEDRIVE,
            &format!(
                "\r\nHKEY_CURRENT_USER\\Software\\Microsoft\\OneDrive\\Accounts\\Personal\r\n    UserFolder    REG_SZ    {perso}\\\r\n    cid    REG_SZ    abc\r\n\r\nHKEY_CURRENT_USER\\Software\\Microsoft\\OneDrive\\Accounts\\Business1\r\n    UserFolder    REG_SZ    {pro}\r\n    Timer    REG_DWORD    0x1\r\n"
            ),
        );
        let trouves = chercher(&systeme);
        assert_eq!(trouves.len(), 2);
        assert!(
            trouves
                .iter()
                .all(|d| d.fournisseur == FournisseurEnLigne::OneDrive)
        );
        assert_eq!(trouves[0].racine, perso);
        assert_eq!(
            trouves[0].propose,
            Path::new(&perso).join("GESCO").to_string_lossy()
        );
        assert_eq!(trouves[1].racine, pro);
    }

    #[test]
    fn un_dossier_qui_n_existe_pas_n_est_pas_propose() {
        let base = tempfile::tempdir().unwrap();
        let systeme = SystemeSimule::default();
        systeme.variable_posee(
            "OneDrive",
            Some(&base.path().join("absent").to_string_lossy()),
        );
        assert!(chercher(&systeme).is_empty());
    }

    #[test]
    fn dropbox_se_lit_dans_son_info_json() {
        let base = tempfile::tempdir().unwrap();
        let perso = dossier(base.path(), "Dropbox");
        let reglages = base.path().join("Roaming");
        std::fs::create_dir_all(reglages.join("Dropbox")).unwrap();
        let info = serde_json::json!({ "personal": { "path": perso, "host": 1 } });
        std::fs::write(reglages.join("Dropbox").join("info.json"), info.to_string()).unwrap();
        let systeme = SystemeSimule::default();
        systeme.variable_posee("APPDATA", Some(&reglages.to_string_lossy()));
        let trouves = chercher(&systeme);
        assert_eq!(trouves.len(), 1);
        assert_eq!(trouves[0].fournisseur, FournisseurEnLigne::Dropbox);
        assert_eq!(trouves[0].nom, "Dropbox");
        assert_eq!(trouves[0].racine, perso);
    }

    #[test]
    fn google_drive_en_miroir_passe_avant_onedrive() {
        let base = tempfile::tempdir().unwrap();
        let onedrive = dossier(base.path(), "OneDrive");
        let drive = dossier(base.path(), "Mon Drive");
        let systeme = SystemeSimule::default();
        systeme.variable_posee("OneDrive", Some(&onedrive));
        systeme.variable_posee("USERPROFILE", Some(&base.path().to_string_lossy()));
        let trouves = chercher(&systeme);
        assert_eq!(trouves.len(), 2);
        assert_eq!(trouves[0].fournisseur, FournisseurEnLigne::GoogleDrive);
        assert_eq!(trouves[0].racine, drive);
        assert_eq!(trouves[1].fournisseur, FournisseurEnLigne::OneDrive);
    }

    #[test]
    fn les_racines_de_windows_ne_gardent_que_ce_compte() {
        let base = tempfile::tempdir().unwrap();
        let mien = dossier(base.path(), "Nuage");
        let autre = dossier(base.path(), "Nuage-autre");
        let systeme = SystemeSimule::default();
        systeme.compte_pose("S-1-5-21-1");
        systeme.registre_pose(
            CLE_RACINES,
            &format!(
                "\r\nHKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\SyncRootManager\\Nuage!S-1-5-21-1!compte\r\n    DisplayNameResource    REG_SZ    Nuage de l'école\r\n\r\nHKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\SyncRootManager\\Nuage!S-1-5-21-1!compte\\UserSyncRoots\r\n    S-1-5-21-1    REG_SZ    {mien}\r\n    S-1-5-21-2    REG_SZ    {autre}\r\n"
            ),
        );
        let trouves = chercher(&systeme);
        assert_eq!(trouves.len(), 1);
        assert_eq!(trouves[0].fournisseur, FournisseurEnLigne::Autre);
        assert_eq!(trouves[0].nom, "Nuage de l'école");
        assert_eq!(trouves[0].racine, mien);
    }

    #[test]
    fn une_racine_de_windows_deja_trouvee_ne_revient_pas() {
        let base = tempfile::tempdir().unwrap();
        let onedrive = dossier(base.path(), "OneDrive");
        let systeme = SystemeSimule::default();
        systeme.variable_posee("OneDrive", Some(&onedrive));
        systeme.registre_pose(
            CLE_RACINES,
            &format!(
                "HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\SyncRootManager\\OneDrive!S-1-5-21-1!Personal|1\\UserSyncRoots\r\n    S-1-5-21-1    REG_SZ    {}\r\n",
                onedrive.to_uppercase()
            ),
        );
        let trouves = chercher(&systeme);
        assert_eq!(trouves.len(), 1);
        assert_eq!(trouves[0].racine, onedrive);
    }
}
