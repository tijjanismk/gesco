//! Fusion de deux fiches élèves en doublon (SPEC_PHASE_2 §8) : la fiche conservée garde ses
//! valeurs et reçoit celles de l'absorbée là où elle est vide. Rien ne se fusionne sans un geste
//! humain (T-07) : ces règles ne s'appliquent qu'à la demande.

use super::personne::{Eleve, Personne};

fn completer(conserve: &mut String, absorbe: &str) {
    if conserve.is_empty() {
        absorbe.clone_into(conserve);
    }
}

/// Identité et fiche de la conservée, complétées par l'absorbée. Les champs remplis des deux
/// côtés gardent la valeur de la conservée ; les téléphones s'ajoutent sans doublon.
pub fn fiche_fusionnee(
    conservee: (&Eleve, &Personne),
    absorbee: (&Eleve, &Personne),
) -> (Eleve, Personne) {
    let (ec, pc) = conservee;
    let (ea, pa) = absorbee;
    let mut eleve = ec.clone();
    let mut personne = pc.clone();
    if eleve.matricule_national.is_none() {
        eleve.matricule_national.clone_from(&ea.matricule_national);
    }
    if eleve.acte_nature.is_none() {
        eleve.acte_nature = ea.acte_nature;
    }
    completer(&mut eleve.acte_numero, &ea.acte_numero);
    if eleve.acte_date.is_none() {
        eleve.acte_date = ea.acte_date;
    }
    completer(&mut eleve.ecole_origine, &ea.ecole_origine);
    if personne.sexe.is_none() {
        personne.sexe = pa.sexe;
    }
    if personne.date_naissance.is_none() {
        personne.date_naissance = pa.date_naissance;
        personne.date_naissance_estimee = pa.date_naissance_estimee;
    }
    completer(&mut personne.lieu_naissance, &pa.lieu_naissance);
    completer(&mut personne.nationalite, &pa.nationalite);
    completer(&mut personne.adresse, &pa.adresse);
    for telephone in &pa.telephones {
        if !personne.telephones.contains(telephone) {
            personne.telephones.push(telephone.clone());
        }
    }
    (eleve, personne)
}

/// Vérifie que deux fiches peuvent fusionner : distinctes et toutes deux vivantes.
pub fn verifier_fusion(conservee: &Eleve, absorbee: &Eleve) -> Result<(), &'static str> {
    if conservee.id == absorbee.id {
        return Err("fusion.meme_fiche");
    }
    if conservee.fusionne_dans.is_some() || absorbee.fusionne_dans.is_some() {
        return Err("eleve.absorbe");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commun::{DateCivile, Id};
    use crate::scolarite::personne::{NatureActe, Sexe};

    fn id<T>(x: u128) -> Id<T> {
        Id::depuis_ulid(ulid::Ulid::from(x))
    }

    fn fiche(n: u128) -> (Eleve, Personne) {
        (
            Eleve {
                id: id(n),
                etablissement_id: id(0),
                personne_id: id(n + 100),
                matricule_interne: format!("2026-0000{n}"),
                matricule_national: None,
                acte_nature: None,
                acte_numero: String::new(),
                acte_date: None,
                ecole_origine: String::new(),
                fusionne_dans: None,
            },
            Personne {
                id: id(n + 100),
                etablissement_id: id(0),
                nom: "Traoré".to_owned(),
                prenoms: "Awa".to_owned(),
                sexe: Some(Sexe::Feminin),
                date_naissance: DateCivile::analyser("2014-03-02"),
                date_naissance_estimee: false,
                lieu_naissance: String::new(),
                nationalite: "Malienne".to_owned(),
                adresse: String::new(),
                telephones: vec!["+22376123456".to_owned()],
            },
        )
    }

    #[test]
    fn les_vides_se_remplissent_les_valeurs_restent() {
        let (ec, pc) = fiche(1);
        let (mut ea, mut pa) = fiche(2);
        pa.lieu_naissance = "Kati".to_owned();
        pa.nationalite = "Burkinabè".to_owned();
        pa.telephones = vec!["+22376123456".to_owned(), "+22366001122".to_owned()];
        ea.matricule_national = Some("ML42".to_owned());
        ea.acte_nature = Some(NatureActe::JugementSuppletif);
        ea.acte_numero = "JS-12".to_owned();
        let (eleve, personne) = fiche_fusionnee((&ec, &pc), (&ea, &pa));
        assert_eq!(eleve.id, ec.id);
        assert_eq!(eleve.matricule_interne, ec.matricule_interne);
        assert_eq!(eleve.matricule_national.as_deref(), Some("ML42"));
        assert_eq!(eleve.acte_numero, "JS-12");
        assert_eq!(personne.lieu_naissance, "Kati");
        assert_eq!(
            personne.nationalite, "Malienne",
            "la conservée garde sa valeur"
        );
        assert_eq!(personne.telephones, ["+22376123456", "+22366001122"]);
    }

    #[test]
    fn deux_fiches_vivantes_et_distinctes() {
        let (a, _) = fiche(1);
        let (mut b, _) = fiche(2);
        assert_eq!(verifier_fusion(&a, &a), Err("fusion.meme_fiche"));
        assert_eq!(verifier_fusion(&a, &b), Ok(()));
        b.fusionne_dans = Some(a.id);
        assert_eq!(verifier_fusion(&a, &b), Err("eleve.absorbe"));
    }
}
