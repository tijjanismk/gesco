//! Import des élèves depuis un tableur (SPEC_PHASE_2 §10) : reconnaissance des colonnes par
//! leur titre, lecture des valeurs telles que les écoles les écrivent, règle de découpe d'une
//! colonne « nom et prénoms ». Pur : ni fichier ni base ; les motifs sont des clés du catalogue.

use serde::{Deserialize, Serialize};

use crate::commun::DateCivile;
use crate::scolarite::Sexe;
use crate::scolarite::recherche::normaliser;

/// Ce qu'une colonne du fichier apporte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum Champ {
    /// Colonne laissée de côté.
    Ignorer,
    /// Nom de famille.
    Nom,
    /// Prénoms.
    Prenoms,
    /// Nom et prénoms dans une seule colonne, découpée selon une règle affichée.
    NomEtPrenoms,
    /// Sexe.
    Sexe,
    /// Date de naissance.
    DateNaissance,
    /// Lieu de naissance.
    LieuNaissance,
    /// Classe de l'année.
    Classe,
    /// Matricule interne de l'école.
    MatriculeInterne,
    /// Matricule national.
    MatriculeNational,
    /// Adresse.
    Adresse,
    /// Nom et prénoms du responsable.
    ResponsableNom,
    /// Téléphone du responsable.
    ResponsableTelephone,
}

impl Champ {
    /// Tous les champs, dans l'ordre proposé à l'écran.
    pub const TOUS: &'static [Self] = &[
        Self::Ignorer,
        Self::Nom,
        Self::Prenoms,
        Self::NomEtPrenoms,
        Self::Sexe,
        Self::DateNaissance,
        Self::LieuNaissance,
        Self::Classe,
        Self::MatriculeInterne,
        Self::MatriculeNational,
        Self::Adresse,
        Self::ResponsableNom,
        Self::ResponsableTelephone,
    ];
}

/// Où le fichier porte la classe (T-06).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    /// Une seule feuille, une colonne « classe ».
    ColonneClasse,
    /// Une feuille par classe : le nom de la feuille désigne la classe.
    FeuilleParClasse,
}

/// Règle de découpe d'une colonne « nom et prénoms », affichée et validée à l'écran.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum Decoupe {
    /// Le premier mot est le nom : « DIALLO Moussa Abdoulaye ».
    NomDAbord,
    /// Le dernier mot est le nom : « Moussa Abdoulaye DIALLO ».
    NomEnDernier,
}

/// Texte normalisé en minuscules : accents ôtés, ponctuation réduite à des espaces.
fn simple(texte: &str) -> String {
    normaliser(texte).to_lowercase()
}

/// Reconnaît ce qu'une colonne apporte d'après son titre, tel que les écoles l'écrivent.
pub fn reconnaitre(entete: &str) -> Champ {
    let n = simple(entete);
    let contient = |mots: &[&str]| mots.iter().any(|m| n.contains(m));
    if n.is_empty() {
        return Champ::Ignorer;
    }
    if contient(&["national"]) && contient(&["matricule", "numero", "identifiant", "n "]) {
        return Champ::MatriculeNational;
    }
    if contient(&["matricule", "identifiant"]) {
        return Champ::MatriculeInterne;
    }
    if contient(&["lieu"]) {
        return Champ::LieuNaissance;
    }
    if contient(&["naissance", "ne le", "nee le", "ne e le", "date"]) {
        return Champ::DateNaissance;
    }
    if contient(&["sexe", "genre"]) {
        return Champ::Sexe;
    }
    if contient(&["classe", "niveau"]) {
        return Champ::Classe;
    }
    if contient(&["adresse", "quartier", "domicile"]) {
        return Champ::Adresse;
    }
    let responsable = contient(&["parent", "tuteur", "responsable", "pere", "mere", "contact"]);
    if contient(&["tel", "portable", "mobile", "phone"]) {
        return Champ::ResponsableTelephone;
    }
    if responsable {
        return Champ::ResponsableNom;
    }
    if contient(&["prenom"]) {
        return if contient(&["nom et", "noms et", "nom prenom", "noms prenom"]) {
            Champ::NomEtPrenoms
        } else {
            Champ::Prenoms
        };
    }
    if contient(&["nom complet", "nom et", "noms et"]) {
        return Champ::NomEtPrenoms;
    }
    if contient(&["nom", "eleve"]) {
        return Champ::Nom;
    }
    Champ::Ignorer
}

/// Vérifie la correspondance choisie : chaque champ au plus une fois, le nom d'une seule façon,
/// une classe quand elle ne vient pas du nom de la feuille.
pub fn verifier_correspondance(
    champs: &[Champ],
    disposition: Disposition,
) -> Result<(), &'static str> {
    for (i, champ) in champs.iter().enumerate() {
        if *champ != Champ::Ignorer && champs.iter().skip(i + 1).any(|c| c == champ) {
            return Err("import.champ_en_double");
        }
    }
    let a = |c: Champ| champs.contains(&c);
    match (a(Champ::NomEtPrenoms), a(Champ::Nom), a(Champ::Prenoms)) {
        (true, false, false) | (false, true, true) => {}
        (true, _, _) => return Err("import.nom_deux_facons"),
        _ => return Err("import.nom_absent"),
    }
    if disposition == Disposition::ColonneClasse && !a(Champ::Classe) {
        return Err("import.classe_absente_correspondance");
    }
    Ok(())
}

/// Découpe « nom et prénoms » selon la règle : (nom, prénoms).
pub fn decouper(complet: &str, decoupe: Decoupe) -> (String, String) {
    let mots: Vec<&str> = complet.split_whitespace().collect();
    match (mots.split_first(), mots.split_last(), decoupe) {
        (Some((nom, prenoms)), _, Decoupe::NomDAbord)
        | (_, Some((nom, prenoms)), Decoupe::NomEnDernier) => {
            ((*nom).to_owned(), prenoms.join(" "))
        }
        _ => (String::new(), String::new()),
    }
}

/// Sexe tel que les fichiers l'écrivent : M, F, masculin, féminin, garçon, fille, G, H.
pub fn lire_sexe(texte: &str) -> Option<Sexe> {
    match simple(texte).as_str() {
        "m" | "g" | "h" | "masculin" | "garcon" | "homme" | "male" => Some(Sexe::Masculin),
        "f" | "feminin" | "fille" | "femme" | "femelle" => Some(Sexe::Feminin),
        _ => None,
    }
}

/// Date lue, estimée quand le fichier ne donne que l'année (« vers 2014 »).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateLue {
    /// Date.
    pub date: DateCivile,
    /// Seule l'année était connue : premier janvier, marquée estimée.
    pub estimee: bool,
}

/// Date de naissance : `AAAA-MM-JJ`, `JJ/MM/AAAA`, `JJ-MM-AAAA`, `JJ.MM.AAAA`, `AAAA/MM/JJ`,
/// ou une année seule, éventuellement précédée de « vers ».
pub fn lire_date(texte: &str) -> Option<DateLue> {
    let brut = texte.trim();
    let t = brut.split(['T', ' ']).next().unwrap_or(brut);
    if let Some(date) = DateCivile::analyser(t) {
        return Some(DateLue {
            date,
            estimee: false,
        });
    }
    let morceaux: Vec<&str> = t.split(['/', '-', '.']).collect();
    if let [a, b, c] = morceaux.as_slice() {
        let (annee, mois, jour) = if a.len() == 4 { (a, b, c) } else { (c, b, a) };
        if annee.len() == 4 {
            let date =
                DateCivile::nouvelle(annee.parse().ok()?, mois.parse().ok()?, jour.parse().ok()?)?;
            return Some(DateLue {
                date,
                estimee: false,
            });
        }
        return None;
    }
    let annee = simple(brut);
    let annee = annee.strip_prefix("vers ").unwrap_or(&annee).trim();
    if annee.len() == 4 && annee.bytes().all(|b| b.is_ascii_digit()) {
        return DateCivile::nouvelle(annee.parse().ok()?, 1, 1).map(|date| DateLue {
            date,
            estimee: true,
        });
    }
    None
}

/// Élève lu sur une ligne, avec ce qui l'empêche d'être importé (erreurs) et ce qu'il faudra
/// vérifier après (avertissements).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EleveLu {
    /// Nom.
    pub nom: String,
    /// Prénoms.
    pub prenoms: String,
    /// Sexe.
    pub sexe: Option<Sexe>,
    /// Date de naissance.
    pub date_naissance: Option<DateCivile>,
    /// Date estimée.
    pub date_estimee: bool,
    /// Lieu de naissance.
    pub lieu_naissance: String,
    /// Libellé de la classe, tel que le fichier l'écrit.
    pub classe: String,
    /// Matricule interne, s'il est fourni.
    pub matricule_interne: String,
    /// Matricule national, s'il est fourni.
    pub matricule_national: String,
    /// Adresse.
    pub adresse: String,
    /// Nom et prénoms du responsable.
    pub responsable_nom: String,
    /// Téléphone du responsable, tel qu'écrit.
    pub responsable_telephone: String,
    /// Clés des erreurs : la ligne ne s'importe pas.
    pub erreurs: Vec<&'static str>,
    /// Clés des avertissements : la ligne s'importe, à vérifier ensuite.
    pub avertissements: Vec<&'static str>,
}

/// Vrai si la ligne n'a aucune cellule remplie : elle est passée sans bruit.
pub fn ligne_vide(cellules: &[String]) -> bool {
    cellules.iter().all(|c| c.trim().is_empty())
}

/// Lit une ligne selon la correspondance. `classe_de_feuille` porte le nom de la feuille quand
/// la disposition est une feuille par classe.
pub fn lire_ligne(
    champs: &[Champ],
    cellules: &[String],
    decoupe: Decoupe,
    classe_de_feuille: Option<&str>,
) -> EleveLu {
    let mut lu = EleveLu::default();
    let cellule = |i: usize| cellules.get(i).map(|c| c.trim()).unwrap_or("");
    for (i, champ) in champs.iter().enumerate() {
        let valeur = cellule(i);
        match champ {
            Champ::Ignorer => {}
            Champ::Nom => lu.nom = valeur.to_owned(),
            Champ::Prenoms => lu.prenoms = valeur.to_owned(),
            Champ::NomEtPrenoms => (lu.nom, lu.prenoms) = decouper(valeur, decoupe),
            Champ::Sexe => {
                if valeur.is_empty() {
                    lu.erreurs.push("import.sexe_manquant");
                } else {
                    match lire_sexe(valeur) {
                        Some(s) => lu.sexe = Some(s),
                        None => lu.erreurs.push("import.sexe_illisible"),
                    }
                }
            }
            Champ::DateNaissance => {
                if valeur.is_empty() {
                    lu.erreurs.push("import.date_manquante");
                } else {
                    match lire_date(valeur) {
                        Some(d) => {
                            lu.date_naissance = Some(d.date);
                            lu.date_estimee = d.estimee;
                            if d.estimee {
                                lu.avertissements.push("import.date_estimee");
                            }
                        }
                        None => lu.erreurs.push("import.date_illisible"),
                    }
                }
            }
            Champ::LieuNaissance => lu.lieu_naissance = valeur.to_owned(),
            Champ::Classe => lu.classe = valeur.to_owned(),
            Champ::MatriculeInterne => lu.matricule_interne = valeur.to_owned(),
            Champ::MatriculeNational => lu.matricule_national = valeur.to_owned(),
            Champ::Adresse => lu.adresse = valeur.to_owned(),
            Champ::ResponsableNom => lu.responsable_nom = valeur.to_owned(),
            Champ::ResponsableTelephone => lu.responsable_telephone = valeur.to_owned(),
        }
    }
    if let Some(feuille) = classe_de_feuille {
        lu.classe = feuille.trim().to_owned();
    }
    if lu.nom.is_empty() || lu.prenoms.is_empty() {
        lu.erreurs.push("import.nom_manquant");
    }
    if !champs.contains(&Champ::Sexe) {
        lu.erreurs.push("import.sexe_manquant");
    }
    if !champs.contains(&Champ::DateNaissance) {
        lu.erreurs.push("import.date_manquante");
    }
    if lu.classe.is_empty() {
        lu.erreurs.push("import.classe_manquante");
    }
    if lu.lieu_naissance.is_empty() {
        lu.avertissements.push("import.lieu_manquant");
    }
    lu
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_titres_des_ecoles_sont_reconnus() {
        assert_eq!(reconnaitre("Nom"), Champ::Nom);
        assert_eq!(reconnaitre("NOM DE L'ÉLÈVE"), Champ::Nom);
        assert_eq!(reconnaitre("Prénom(s)"), Champ::Prenoms);
        assert_eq!(reconnaitre("Nom et prénoms"), Champ::NomEtPrenoms);
        assert_eq!(reconnaitre("Noms et Prénoms"), Champ::NomEtPrenoms);
        assert_eq!(reconnaitre("Sexe"), Champ::Sexe);
        assert_eq!(reconnaitre("Né(e) le"), Champ::DateNaissance);
        assert_eq!(reconnaitre("Date de naissance"), Champ::DateNaissance);
        assert_eq!(reconnaitre("Lieu de naissance"), Champ::LieuNaissance);
        assert_eq!(reconnaitre("Classe"), Champ::Classe);
        assert_eq!(reconnaitre("Matricule"), Champ::MatriculeInterne);
        assert_eq!(
            reconnaitre("N° matricule national"),
            Champ::MatriculeNational
        );
        assert_eq!(
            reconnaitre("Téléphone du parent"),
            Champ::ResponsableTelephone
        );
        assert_eq!(reconnaitre("Tuteur"), Champ::ResponsableNom);
        assert_eq!(reconnaitre("Adresse"), Champ::Adresse);
        assert_eq!(reconnaitre("Observations"), Champ::Ignorer);
        assert_eq!(reconnaitre(""), Champ::Ignorer);
    }

    #[test]
    fn la_correspondance_est_verifiee() {
        use Champ::*;
        assert_eq!(
            verifier_correspondance(
                &[Nom, Prenoms, Sexe, DateNaissance, Classe],
                Disposition::ColonneClasse
            ),
            Ok(())
        );
        assert_eq!(
            verifier_correspondance(&[NomEtPrenoms, Sexe], Disposition::FeuilleParClasse),
            Ok(())
        );
        assert_eq!(
            verifier_correspondance(&[Nom, Nom, Prenoms, Classe], Disposition::ColonneClasse),
            Err("import.champ_en_double")
        );
        assert_eq!(
            verifier_correspondance(
                &[Ignorer, Ignorer, Nom, Prenoms, Classe],
                Disposition::ColonneClasse
            ),
            Ok(())
        );
        assert_eq!(
            verifier_correspondance(&[Nom, NomEtPrenoms, Classe], Disposition::ColonneClasse),
            Err("import.nom_deux_facons")
        );
        assert_eq!(
            verifier_correspondance(&[Nom, Classe], Disposition::ColonneClasse),
            Err("import.nom_absent")
        );
        assert_eq!(
            verifier_correspondance(&[Nom, Prenoms], Disposition::ColonneClasse),
            Err("import.classe_absente_correspondance")
        );
    }

    #[test]
    fn le_nom_complet_se_decoupe_selon_la_regle() {
        assert_eq!(
            decouper("DIALLO Moussa Abdoulaye", Decoupe::NomDAbord),
            ("DIALLO".to_owned(), "Moussa Abdoulaye".to_owned())
        );
        assert_eq!(
            decouper("Moussa Abdoulaye DIALLO", Decoupe::NomEnDernier),
            ("DIALLO".to_owned(), "Moussa Abdoulaye".to_owned())
        );
        assert_eq!(
            decouper("Awa", Decoupe::NomDAbord),
            ("Awa".to_owned(), String::new())
        );
        assert_eq!(
            decouper("  ", Decoupe::NomDAbord),
            (String::new(), String::new())
        );
    }

    #[test]
    fn les_dates_des_fichiers_se_lisent() {
        let jour = |a, m, j| DateCivile::nouvelle(a, m, j);
        assert_eq!(lire_date("2014-05-10").map(|d| d.date), jour(2014, 5, 10));
        assert_eq!(lire_date("10/05/2014").map(|d| d.date), jour(2014, 5, 10));
        assert_eq!(lire_date("10-05-2014").map(|d| d.date), jour(2014, 5, 10));
        assert_eq!(lire_date("10.5.2014").map(|d| d.date), jour(2014, 5, 10));
        assert_eq!(lire_date("2014/05/10").map(|d| d.date), jour(2014, 5, 10));
        assert_eq!(
            lire_date("2014-05-10T00:00:00").map(|d| d.date),
            jour(2014, 5, 10)
        );
        assert_eq!(
            lire_date("vers 2014"),
            jour(2014, 1, 1).map(|date| DateLue {
                date,
                estimee: true
            })
        );
        assert_eq!(lire_date("2014").map(|d| d.estimee), Some(true));
        assert_eq!(lire_date("10/05/14"), None);
        assert_eq!(lire_date("31/02/2014"), None);
        assert_eq!(lire_date("inconnue"), None);
    }

    #[test]
    fn une_ligne_se_lit_avec_ses_erreurs_et_ses_avertissements() {
        use Champ::*;
        let champs = [
            NomEtPrenoms,
            Sexe,
            DateNaissance,
            LieuNaissance,
            Classe,
            ResponsableTelephone,
        ];
        let cellules = |v: &[&str]| v.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        let lu = lire_ligne(
            &champs,
            &cellules(&["TRAORE Awa", "F", "vers 2014", "", "7e A", "76 12 34 56"]),
            Decoupe::NomDAbord,
            None,
        );
        assert_eq!(lu.nom, "TRAORE");
        assert_eq!(lu.prenoms, "Awa");
        assert_eq!(lu.sexe, Some(crate::scolarite::Sexe::Feminin));
        assert!(lu.date_estimee);
        assert_eq!(lu.classe, "7e A");
        assert_eq!(lu.erreurs, Vec::<&str>::new());
        assert_eq!(
            lu.avertissements,
            vec!["import.date_estimee", "import.lieu_manquant"]
        );

        let faute = lire_ligne(
            &champs,
            &cellules(&["Awa", "X", "hier", "Kati", "", ""]),
            Decoupe::NomDAbord,
            Some("8e B"),
        );
        assert_eq!(faute.classe, "8e B");
        assert_eq!(
            faute.erreurs,
            vec![
                "import.sexe_illisible",
                "import.date_illisible",
                "import.nom_manquant"
            ]
        );

        let sans_colonnes = lire_ligne(
            &[Nom, Prenoms, Classe],
            &cellules(&["A", "B", "7e A"]),
            Decoupe::NomDAbord,
            None,
        );
        assert_eq!(
            sans_colonnes.erreurs,
            vec!["import.sexe_manquant", "import.date_manquante"]
        );
        assert!(ligne_vide(&cellules(&["", "  ", ""])));
    }
}
