//! Ce que doit l'État (SPEC_PRISE_EN_CHARGE_ETAT §6.3, §9, §10) : par inscription, par classe et
//! pour l'école, l'attendu, le reçu, le reste dû et l'échu, plus les versements de l'État de
//! l'année et ce qui n'en est pas réparti. Les chiffres de chaque inscription sont ceux de la
//! liste des élèves (`chiffres_etat`, sur le partage de `facturation::etat_facture`) : un seul
//! calcul pour l'écran, l'état imprimé, l'export Excel, la tuile du tableau de bord et la ligne
//! « À recevoir de l'État » du budget. Rien ne s'écrit ici, sauf le document produit.

use std::collections::HashMap;

use gesco_core::commun::{DateCivile, Id, Montant};
use gesco_core::core::{AnneeScolaire, Permission};
use gesco_core::finance::prise_en_charge::{
    StatutPriseEnCharge, StatutVersementEtat, VersementEtat,
};
use gesco_core::scolarite::{Classe, Eleve, Inscription, Serie, Sexe};
use gesco_db::depots::{academique, personnes, prise_en_charge, rapports, tresorerie};
use gesco_db::{Connexion, Contexte};
use serde::{Deserialize, Serialize};

use super::{
    ChiffresEtat, ClassePriseEnCharge, ServicePriseEnCharge, annee_close, annee_demandee,
    chiffres_etat, lire_cible, nombre, somme, versements::affiche,
};
use crate::erreur::Resultat;
use crate::export::Feuille;
use crate::inscriptions::jour;
use crate::session::Session;

/// Titre du document et de la feuille Excel.
pub const TITRE_DU_ETAT: &str = "État des sommes dues par l'État";

// ---------------------------------------------------------------------------------------------
// Demandes et réponses
// ---------------------------------------------------------------------------------------------

/// Filtres de l'état de ce que doit l'État (§6.3, §9.2).
#[derive(Debug, Clone, Default, Deserialize, specta::Type)]
pub struct DemandeDuEtat {
    /// Année ; absente, l'année courante. Une année close se lit et s'imprime.
    #[serde(default)]
    pub annee_id: Option<Id<AnneeScolaire>>,
    /// Classe ; absente, toutes.
    #[serde(default)]
    pub classe_id: Option<Id<Classe>>,
    /// Série ; absente, toutes.
    #[serde(default)]
    pub serie_id: Option<Id<Serie>>,
    /// Seulement les élèves dont l'État doit encore quelque chose.
    #[serde(default)]
    pub seulement_dus: bool,
}

/// Production de l'état : le PDF, ou, avec un chemin, le classeur Excel des mêmes colonnes.
#[derive(Debug, Clone, Default, Deserialize, specta::Type)]
pub struct DemandeProductionDuEtat {
    /// Année ; absente, l'année courante.
    #[serde(default)]
    pub annee_id: Option<Id<AnneeScolaire>>,
    /// Classe ; absente, toutes.
    #[serde(default)]
    pub classe_id: Option<Id<Classe>>,
    /// Série ; absente, toutes.
    #[serde(default)]
    pub serie_id: Option<Id<Serie>>,
    /// Seulement les élèves dont l'État doit encore quelque chose.
    #[serde(default)]
    pub seulement_dus: bool,
    /// Chemin du classeur `.xlsx` à écrire ; absent, le PDF.
    #[serde(default)]
    pub chemin: Option<String>,
}

impl DemandeProductionDuEtat {
    /// Les filtres de la production.
    pub const fn filtres(&self) -> DemandeDuEtat {
        DemandeDuEtat {
            annee_id: self.annee_id,
            classe_id: self.classe_id,
            serie_id: self.serie_id,
            seulement_dus: self.seulement_dus,
        }
    }
}

/// Une inscription dans l'état.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct LigneDuEtat {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Élève.
    pub eleve_id: Id<Eleve>,
    /// Nom et prénoms.
    pub eleve: String,
    /// Matricule interne.
    pub matricule: String,
    /// Sexe, « F » ou « M » (vide s'il n'est pas connu).
    pub sexe: String,
    /// Classe.
    pub classe_id: Id<Classe>,
    /// Libellé de la classe.
    pub classe: String,
    /// Série de la classe.
    pub serie_id: Option<Id<Serie>>,
    /// Libellé de la série (vide sans série).
    pub serie: String,
    /// Référence d'orientation.
    pub reference: String,
    /// Case cochée (version en cours) ; sinon prise en charge terminée, encore suivie pour ce
    /// que l'État a payé.
    pub pris_en_charge: bool,
    /// Attendu : le montant attendu d'un élève coché ; pour une prise en charge terminée, ce que
    /// l'État doit, c'est-à-dire ce qu'il a payé sur la facture.
    pub attendu: Montant,
    /// Abandonné par l'école (« Ne plus attendre »).
    pub abandonne: Montant,
    /// Dû par l'État : sur la facture et en complément.
    pub du: Montant,
    /// Reçu de l'État.
    pub recu: Montant,
    /// Reste dû par l'État.
    pub reste: Montant,
    /// Dont échu.
    pub echu: Montant,
    /// Reçu au-delà de ce que l'État doit pour l'élève.
    pub recu_en_plus: Montant,
}

/// Totaux d'une classe ou de l'école.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, specta::Type)]
pub struct TotauxDuEtat {
    /// Élèves.
    pub nombre: u32,
    /// Attendu.
    pub attendu: Montant,
    /// Abandonné.
    pub abandonne: Montant,
    /// Dû.
    pub du: Montant,
    /// Reçu.
    pub recu: Montant,
    /// Reste dû.
    pub reste: Montant,
    /// Dont échu.
    pub echu: Montant,
    /// Reçu en plus.
    pub recu_en_plus: Montant,
}

impl TotauxDuEtat {
    fn de(lignes: &[LigneDuEtat]) -> Resultat<Self> {
        let total = |f: fn(&LigneDuEtat) -> Montant| somme(lignes.iter().map(f));
        Ok(Self {
            nombre: nombre(lignes.len()),
            attendu: total(|l| l.attendu)?,
            abandonne: total(|l| l.abandonne)?,
            du: total(|l| l.du)?,
            recu: total(|l| l.recu)?,
            reste: total(|l| l.reste)?,
            echu: total(|l| l.echu)?,
            recu_en_plus: total(|l| l.recu_en_plus)?,
        })
    }
}

/// Une classe de l'état, avec son sous-total.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ClasseDuEtat {
    /// Classe.
    pub classe_id: Id<Classe>,
    /// Libellé.
    pub classe: String,
    /// Élèves, par nom.
    pub lignes: Vec<LigneDuEtat>,
    /// Sous-total.
    pub totaux: TotauxDuEtat,
}

/// Série proposée au filtre.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct SerieDuEtat {
    /// Série.
    pub id: Id<Serie>,
    /// Libellé.
    pub libelle: String,
}

/// Un versement de l'État de l'année, dans l'état.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct VersementDuEtat {
    /// Versement.
    pub versement_etat_id: Id<VersementEtat>,
    /// Date affichée : la réception, ou l'arrêt avant GESCO pour un versement reçu avant GESCO.
    pub date: DateCivile,
    /// Référence.
    pub reference: String,
    /// Libellé.
    pub libelle: String,
    /// Reçu avant GESCO (déjà dans le solde d'ouverture).
    pub avant_gesco: bool,
    /// Montant reçu.
    pub montant: Montant,
    /// Réparti sur des élèves.
    pub reparti: Montant,
    /// Reçu, pas encore réparti.
    pub non_reparti: Montant,
}

/// Ce que doit l'État pour une année (§6.3, §9.2).
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct DuEtat {
    /// Année.
    pub annee_id: Id<AnneeScolaire>,
    /// Libellé de l'année.
    pub annee: String,
    /// Année close : l'État lui doit encore, l'état se produit.
    pub annee_close: bool,
    /// Jour d'arrêt des chiffres (aujourd'hui).
    pub arrete_au: DateCivile,
    /// Classes qui ont une prise en charge, pour le filtre.
    pub classes_filtre: Vec<ClassePriseEnCharge>,
    /// Séries qui ont une prise en charge, pour le filtre.
    pub series_filtre: Vec<SerieDuEtat>,
    /// Classes montrées, par libellé, avec leurs élèves.
    pub classes: Vec<ClasseDuEtat>,
    /// Total général des élèves montrés.
    pub totaux: TotauxDuEtat,
    /// Versements valides de l'État pour l'année, par date.
    pub versements: Vec<VersementDuEtat>,
    /// Total reçu de l'État pour l'année.
    pub total_verse: Montant,
    /// Reçu de l'État pour l'année, pas encore réparti sur des élèves.
    pub non_reparti: Montant,
}

// ---------------------------------------------------------------------------------------------
// Le calcul
// ---------------------------------------------------------------------------------------------

fn lettre_sexe(sexe: Option<Sexe>) -> &'static str {
    match sexe {
        Some(Sexe::Feminin) => "F",
        Some(Sexe::Masculin) => "M",
        None => "",
    }
}

/// Une ligne par inscription de l'année qui a une prise en charge en cours, ou terminée mais à
/// qui l'État a payé ou doit quelque chose ; sans filtre.
async fn lignes_de_l_annee(
    cnx: &mut Connexion,
    annee: &AnneeScolaire,
    aujourd_hui: DateCivile,
    contexte: &Contexte,
) -> Resultat<Vec<LigneDuEtat>> {
    let versions = prise_en_charge::courantes_de_l_annee(cnx, annee.id, contexte).await?;
    if versions.is_empty() {
        return Ok(Vec::new());
    }
    let eleves: HashMap<Id<Eleve>, (String, String, &'static str)> =
        personnes::eleves(cnx, contexte)
            .await?
            .into_iter()
            .map(|(e, p)| {
                (
                    e.id,
                    (p.nom_complet(), e.matricule_interne, lettre_sexe(p.sexe)),
                )
            })
            .collect();
    let series: HashMap<Id<Serie>, String> = academique::series(cnx, contexte)
        .await?
        .into_iter()
        .map(|s| (s.id, s.libelle))
        .collect();
    let mut lignes = Vec::with_capacity(versions.len());
    for v in versions {
        let cible = lire_cible(cnx, v.inscription_id, contexte).await?;
        let (_, etat): (_, ChiffresEtat) =
            chiffres_etat(cnx, &cible, aujourd_hui, contexte).await?;
        let en_cours = cible.en_cours();
        if en_cours.is_none() && etat.recu.est_nul() && etat.reste.est_nul() {
            continue;
        }
        let (eleve, matricule, sexe) = eleves
            .get(&cible.inscription.eleve_id)
            .cloned()
            .unwrap_or_default();
        let serie_id = cible.classe.serie_id;
        lignes.push(LigneDuEtat {
            inscription_id: cible.inscription.id,
            eleve_id: cible.inscription.eleve_id,
            eleve,
            matricule,
            sexe: sexe.to_owned(),
            classe_id: cible.classe.id,
            classe: cible.classe.libelle.clone(),
            serie_id,
            serie: serie_id
                .and_then(|s| series.get(&s).cloned())
                .unwrap_or_default(),
            reference: v.reference.clone(),
            pris_en_charge: v.statut == StatutPriseEnCharge::EnCours,
            attendu: en_cours.map_or(etat.du, |p| p.montant_attendu),
            abandonne: en_cours.map_or(Montant::ZERO, |p| p.abandonne),
            du: etat.du,
            recu: etat.recu,
            reste: etat.reste,
            echu: etat.echu,
            recu_en_plus: etat.recu_en_plus,
        });
    }
    Ok(lignes)
}

/// Ce que l'État doit encore pour une année, et dont échu (§9.3, §10) : la somme des restes de
/// l'État de ses inscriptions. Rien si l'année n'a aucune prise en charge.
pub(crate) async fn a_recevoir_de_l_etat(
    cnx: &mut Connexion,
    annee: &AnneeScolaire,
    aujourd_hui: DateCivile,
    contexte: &Contexte,
) -> Resultat<Option<(Montant, Montant)>> {
    if prise_en_charge::courantes_de_l_annee(cnx, annee.id, contexte)
        .await?
        .is_empty()
    {
        return Ok(None);
    }
    let lignes = lignes_de_l_annee(cnx, annee, aujourd_hui, contexte).await?;
    Ok(Some((
        somme(lignes.iter().map(|l| l.reste))?,
        somme(lignes.iter().map(|l| l.echu))?,
    )))
}

/// L'état d'une année, filtré.
async fn du_de_l_annee(
    cnx: &mut Connexion,
    demande: &DemandeDuEtat,
    contexte: &Contexte,
) -> Resultat<DuEtat> {
    let annee = annee_demandee(cnx, demande.annee_id, contexte).await?;
    let aujourd_hui = jour(contexte.maintenant)?;
    let toutes = lignes_de_l_annee(cnx, &annee, aujourd_hui, contexte).await?;

    // Filtres proposés : les classes et séries qui ont une prise en charge.
    let mut classes_filtre: Vec<ClassePriseEnCharge> = Vec::new();
    let mut series_filtre: Vec<SerieDuEtat> = Vec::new();
    for l in &toutes {
        if !classes_filtre.iter().any(|c| c.id == l.classe_id) {
            classes_filtre.push(ClassePriseEnCharge {
                id: l.classe_id,
                libelle: l.classe.clone(),
            });
        }
        if let Some(id) = l.serie_id
            && !series_filtre.iter().any(|s| s.id == id)
        {
            series_filtre.push(SerieDuEtat {
                id,
                libelle: l.serie.clone(),
            });
        }
    }
    classes_filtre.sort_by(|a, b| a.libelle.cmp(&b.libelle));
    series_filtre.sort_by(|a, b| a.libelle.cmp(&b.libelle));

    let mut montrees: Vec<LigneDuEtat> = toutes
        .into_iter()
        .filter(|l| demande.classe_id.is_none_or(|c| c == l.classe_id))
        .filter(|l| demande.serie_id.is_none_or(|s| Some(s) == l.serie_id))
        .filter(|l| !demande.seulement_dus || !l.reste.est_nul())
        .collect();
    montrees.sort_by(|a, b| (&a.classe, &a.eleve).cmp(&(&b.classe, &b.eleve)));
    let totaux = TotauxDuEtat::de(&montrees)?;
    let mut classes: Vec<ClasseDuEtat> = Vec::new();
    for l in montrees {
        match classes.iter_mut().find(|c| c.classe_id == l.classe_id) {
            Some(c) => c.lignes.push(l),
            None => classes.push(ClasseDuEtat {
                classe_id: l.classe_id,
                classe: l.classe.clone(),
                lignes: vec![l],
                totaux: TotauxDuEtat::default(),
            }),
        }
    }
    for c in &mut classes {
        c.totaux = TotauxDuEtat::de(&c.lignes)?;
    }

    // Versements valides de l'année, avec ce qui n'en est pas réparti.
    let valides: Vec<VersementEtat> =
        prise_en_charge::versements_de_l_annee(cnx, annee.id, contexte)
            .await?
            .into_iter()
            .filter(|v| v.statut == StatutVersementEtat::Valide)
            .collect();
    let arret = if valides.iter().any(|v| v.avant_gesco) {
        Some(
            crate::reprises::arret_de_l_annee(cnx, &annee, contexte)
                .await?
                .0,
        )
    } else {
        None
    };
    let comptes = tresorerie::comptes(cnx, contexte).await?;
    let mut versements = Vec::with_capacity(valides.len());
    for v in valides {
        let (a, _) = affiche(cnx, v, &comptes, arret, contexte).await?;
        versements.push(VersementDuEtat {
            versement_etat_id: a.versement.id,
            date: a.date_affichee,
            reference: a.versement.reference.clone(),
            libelle: a.versement.libelle.clone(),
            avant_gesco: a.versement.avant_gesco,
            montant: a.versement.montant,
            reparti: a.reparti,
            non_reparti: a.non_reparti,
        });
    }
    Ok(DuEtat {
        annee_id: annee.id,
        annee: annee.libelle.clone(),
        annee_close: annee_close(&annee),
        arrete_au: aujourd_hui,
        classes_filtre,
        series_filtre,
        classes,
        totaux,
        total_verse: somme(versements.iter().map(|v| v.montant))?,
        non_reparti: somme(versements.iter().map(|v| v.non_reparti))?,
        versements,
    })
}

// ---------------------------------------------------------------------------------------------
// Le document et le classeur
// ---------------------------------------------------------------------------------------------

/// Données du modèle `sommes_dues_etat` (R-45). Les montants sont mis en forme ici.
fn donnees(etat: &DuEtat, demande: &DemandeDuEtat, fait_a: &str) -> serde_json::Value {
    let totaux = |t: &TotauxDuEtat| {
        serde_json::json!({
            "nombre": t.nombre,
            "attendu": t.attendu.formater(),
            "recu": t.recu.formater(),
            "reste": t.reste.formater(),
            "echu": t.echu.formater(),
        })
    };
    let mut precisions = Vec::new();
    match demande.classe_id {
        Some(_) => {
            if let Some(c) = etat.classes.first() {
                precisions.push(format!("Classe : {}", c.classe));
            }
        }
        None => precisions.push("Toutes les classes".to_owned()),
    }
    if let Some(s) = demande
        .serie_id
        .and_then(|id| etat.series_filtre.iter().find(|s| s.id == id))
    {
        precisions.push(format!("Série : {}", s.libelle));
    }
    if demande.seulement_dus {
        precisions.push("Seulement les élèves dont l'État doit encore".to_owned());
    }
    if etat.annee_close {
        precisions.push("Année close".to_owned());
    }
    serde_json::json!({
        "titre": TITRE_DU_ETAT,
        "annee": etat.annee,
        "arrete_au": crate::documents::date_civile_longue(etat.arrete_au),
        "precisions": precisions.join(" · "),
        "classes": etat.classes.iter().map(|c| serde_json::json!({
            "classe": c.classe,
            "lignes": c.lignes.iter().map(|l| serde_json::json!({
                "eleve": l.eleve,
                "matricule": l.matricule,
                "sexe": l.sexe,
                "serie": l.serie,
                "reference": l.reference,
                "attendu": l.attendu.formater(),
                "recu": l.recu.formater(),
                "reste": l.reste.formater(),
                "echu": l.echu.formater(),
            })).collect::<Vec<_>>(),
            "sous_total": totaux(&c.totaux),
        })).collect::<Vec<_>>(),
        "total": totaux(&etat.totaux),
        "recu_en_plus": (!etat.totaux.recu_en_plus.est_nul())
            .then(|| etat.totaux.recu_en_plus.formater()),
        "versements": etat.versements.iter().map(|v| serde_json::json!({
            "date": if v.avant_gesco {
                format!("{} (reçu avant GESCO)", crate::documents::date_civile_longue(v.date))
            } else {
                crate::documents::date_civile_longue(v.date)
            },
            "reference": v.reference,
            "libelle": v.libelle,
            "montant": v.montant.formater(),
        })).collect::<Vec<_>>(),
        "total_verse": etat.total_verse.formater(),
        "non_reparti": etat.non_reparti.formater(),
        "fait_a": fait_a,
    })
}

/// La feuille Excel de l'état : les colonnes du document, une ligne par élève, puis le
/// sous-total de chaque classe et le total général.
pub(crate) fn feuille_du_etat(etat: &DuEtat) -> Feuille {
    let entetes = [
        "Classe",
        "Élève",
        "Matricule",
        "Sexe",
        "Série",
        "Référence",
        "Attendu",
        "Reçu",
        "Reste dû",
        "Dont échu",
    ]
    .map(str::to_owned)
    .to_vec();
    let chiffre = |m: Montant| m.francs().to_string();
    let total = |libelle: String, t: &TotauxDuEtat| {
        vec![
            libelle,
            format!("{} élève(s)", t.nombre),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            chiffre(t.attendu),
            chiffre(t.recu),
            chiffre(t.reste),
            chiffre(t.echu),
        ]
    };
    let mut lignes = Vec::new();
    for c in &etat.classes {
        for l in &c.lignes {
            lignes.push(vec![
                l.classe.clone(),
                l.eleve.clone(),
                l.matricule.clone(),
                l.sexe.clone(),
                l.serie.clone(),
                l.reference.clone(),
                chiffre(l.attendu),
                chiffre(l.recu),
                chiffre(l.reste),
                chiffre(l.echu),
            ]);
        }
        lignes.push(total(format!("Sous-total {}", c.classe), &c.totaux));
    }
    lignes.push(total("Total général".to_owned(), &etat.totaux));
    Feuille {
        titre: TITRE_DU_ETAT.to_owned(),
        entetes,
        lignes,
    }
}

// ---------------------------------------------------------------------------------------------
// Le service
// ---------------------------------------------------------------------------------------------

impl ServicePriseEnCharge<'_> {
    /// Ce que doit l'État pour une année (§6.3, §9) : par classe, avec sous-totaux, total
    /// général, versements de l'année et non réparti. Lu avec `invoices:issue`, ou avec
    /// `invoices:discount` comme la liste des élèves (le Directeur n'a pas `invoices:issue`).
    pub async fn du_lister(&self, session: &Session, demande: &DemandeDuEtat) -> Resultat<DuEtat> {
        self.exiger_l_une(
            session,
            Permission::INVOICES_ISSUE,
            &[Permission::INVOICES_DISCOUNT],
        )
        .await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        du_de_l_annee(&mut cnx, demande, &contexte).await
    }

    /// L'état et les données de son document (§9.2) : « Fait à », le lieu des rapports de l'école
    /// s'il est réglé, et la date du jour.
    pub async fn du_donnees(
        &self,
        session: &Session,
        demande: &DemandeDuEtat,
    ) -> Resultat<(DuEtat, serde_json::Value)> {
        let etat = self.du_lister(session, demande).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let lieu = rapports::dernier_regle(&mut cnx, &contexte)
            .await?
            .map(|r| r.lieu)
            .filter(|l| !l.trim().is_empty());
        let le = crate::documents::date_civile_longue(etat.arrete_au);
        let fait_a = match lieu {
            Some(lieu) => format!("Fait à {}, le {le}", lieu.trim()),
            None => format!("Fait à …………………, le {le}"),
        };
        let donnees = donnees(&etat, demande, &fait_a);
        Ok((etat, donnees))
    }
}
