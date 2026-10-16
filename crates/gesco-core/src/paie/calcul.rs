//! Calcul d'un bulletin de paie (SPEC_PHASE_7 §2, §6) : gains, brut, retenues salariales,
//! absences retenues, avances, net, cotisations patronales et cumuls. Chaque ligne est tronquée
//! au franc (R-11). Le moteur applique les éléments de l'école ; il ne connaît aucune règle d'un
//! pays (§8.7 du blueprint).

use serde::{Deserialize, Serialize};

use crate::commun::{DateCivile, Id, Montant};
use crate::paie::element::{TAUX_MAX, version_en_vigueur};
use crate::paie::reglages::mois_paye;
use crate::paie::{Attribution, Calcul, ElementPaie, NatureElement, Prorata, VersionElement};
use crate::rh::heures::montant_des_heures;
use crate::rh::{
    Engagement, HeuresVacation, Mois, NatureEngagement, Remuneration, SituationFamille,
};

/// Erreur d'un calcul qui déborde : la clé du message.
const IMPOSSIBLE: &str = "paie.calcul_impossible";

/// Heures visées d'une vacation, à payer dans ce bulletin : celles du mois, ou un rappel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct HeuresAPayer {
    /// Ligne d'heures.
    pub heures_id: Id<HeuresVacation>,
    /// Engagement de vacation.
    pub engagement_id: Id<Engagement>,
    /// Mois des heures.
    pub mois: Mois,
    /// Heures faites, en minutes.
    pub minutes: i64,
    /// Taux de l'heure.
    pub taux: Montant,
}

/// Montant saisi dans la paie du mois pour un élément « saisi » : une prime, une régularisation
/// (négative pour reprendre un trop-perçu), une retenue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct SaisiePaie {
    /// Élément.
    pub element_id: Id<ElementPaie>,
    /// Montant ; négatif seulement pour un gain qui reprend un trop-perçu.
    pub montant: Montant,
    /// Libellé imprimé : le motif d'une régularisation, le nom d'une prime.
    pub libelle: String,
}

/// Absence non justifiée confirmée par la direction, à retenir (T-56).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct AbsenceRetenue {
    /// Jour.
    pub date: DateCivile,
    /// Une demi-journée seulement.
    pub demi_journee: bool,
}

/// Avance en cours de remboursement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct AvanceARembourser {
    /// Avance remboursée.
    pub avance_id: Option<Id<crate::paie::Avance>>,
    /// Libellé imprimé.
    pub libelle: String,
    /// Mensualité prévue.
    pub mensualite: Montant,
    /// Reste dû avant ce bulletin.
    pub reste: Montant,
}

/// Cumuls de l'année civile.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Cumuls {
    /// Brut.
    pub brut: Montant,
    /// Retenues salariales, absences comprises.
    pub retenues: Montant,
    /// Net à payer.
    pub net: Montant,
}

/// Ce que le calcul d'un bulletin reçoit pour un membre et un mois.
#[derive(Debug, Clone)]
pub struct DonneesBulletin<'a> {
    /// Mois de la paie.
    pub mois: Mois,
    /// Règle du prorata.
    pub prorata: Prorata,
    /// Mois de vacances de l'école.
    pub mois_vacances: &'a [u8],
    /// Le samedi compte parmi les jours ouvrables.
    pub samedi_compte: bool,
    /// Le membre est fonctionnaire de l'État.
    pub fonctionnaire: bool,
    /// Situation de famille.
    pub situation_famille: Option<SituationFamille>,
    /// Enfants à charge.
    pub enfants_a_charge: Option<i64>,
    /// Engagements du membre.
    pub engagements: &'a [Engagement],
    /// Heures visées à payer.
    pub heures: &'a [HeuresAPayer],
    /// Éléments de l'école, dans l'ordre.
    pub elements: &'a [ElementPaie],
    /// Versions de leur calcul.
    pub versions: &'a [VersionElement],
    /// Attributions du membre.
    pub attributions: &'a [Attribution],
    /// Montants saisis dans la paie du mois.
    pub saisies: &'a [SaisiePaie],
    /// Absences confirmées.
    pub absences: &'a [AbsenceRetenue],
    /// Avances en cours, dans l'ordre de leur remboursement.
    pub avances: &'a [AvanceARembourser],
    /// Cumuls de l'année avant ce mois.
    pub cumuls_anterieurs: Cumuls,
}

/// Sorte d'une ligne du bulletin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum SorteLigne {
    /// Salaire de base ou indemnité de stage.
    Base,
    /// Heures de vacation.
    Heures,
    /// Prime, indemnité, rappel, régularisation.
    Gain,
    /// Retenue salariale.
    Retenue,
    /// Absence retenue.
    Absence,
    /// Remboursement d'une avance.
    Avance,
    /// Cotisation patronale, hors du net.
    Patronale,
}

/// Ligne calculée du bulletin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct LigneCalculee {
    /// Sorte.
    pub sorte: SorteLigne,
    /// Élément de paie, pour les gains, retenues et cotisations.
    pub element_id: Option<Id<ElementPaie>>,
    /// Engagement, pour le salaire de base et les heures.
    pub engagement_id: Option<Id<Engagement>>,
    /// Libellé.
    pub libelle: String,
    /// Base : salaire mensuel, taux de l'heure, assiette d'un taux ou d'un barème.
    pub base: Option<Montant>,
    /// Taux appliqué, en centièmes de pour cent.
    pub taux: Option<i64>,
    /// Jours payés sur jours du mois, pour le salaire de base.
    pub jours: Option<(i64, i64)>,
    /// Minutes, pour les heures.
    pub minutes: Option<i64>,
    /// Mois d'origine d'un rappel d'heures.
    pub mois_rappel: Option<Mois>,
    /// Ligne d'heures payée.
    pub heures_id: Option<Id<HeuresVacation>>,
    /// Jour d'une absence retenue.
    pub date: Option<DateCivile>,
    /// Reste dû d'une avance après ce bulletin.
    pub reste: Option<Montant>,
    /// Avance que la ligne rembourse.
    #[serde(default)]
    pub avance_id: Option<Id<crate::paie::Avance>>,
    /// Montant, tronqué au franc.
    pub montant: Montant,
}

impl LigneCalculee {
    fn simple(sorte: SorteLigne, libelle: &str, montant: Montant) -> Self {
        Self {
            sorte,
            element_id: None,
            engagement_id: None,
            libelle: libelle.to_owned(),
            base: None,
            taux: None,
            jours: None,
            minutes: None,
            mois_rappel: None,
            heures_id: None,
            date: None,
            reste: None,
            avance_id: None,
            montant,
        }
    }
}

/// Bulletin calculé.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ResultatBulletin {
    /// Lignes, dans l'ordre du bulletin.
    pub lignes: Vec<LigneCalculee>,
    /// Brut.
    pub brut: Montant,
    /// Retenues salariales, absences comprises.
    pub retenues: Montant,
    /// Remboursements d'avances.
    pub avances: Montant,
    /// Net à payer.
    pub net: Montant,
    /// Cotisations patronales.
    pub patronales: Montant,
    /// Cumuls de l'année, ce mois compris.
    pub cumuls: Cumuls,
    /// Avertissements : clés des messages.
    pub avertissements: Vec<String>,
}

fn somme<'a>(mut montants: impl Iterator<Item = &'a Montant>) -> Result<Montant, &'static str> {
    montants.try_fold(Montant::ZERO, |s, m| s.ajouter(*m).map_err(|_| IMPOSSIBLE))
}

fn part(m: Montant, numerateur: i64, denominateur: i64) -> Result<Montant, &'static str> {
    m.part(numerateur, denominateur)
        .map(|d| d.quotient)
        .map_err(|_| IMPOSSIBLE)
}

/// Jours d'un mois, et l'un de ses jours.
fn jours_du_mois(mois: Mois) -> Option<Vec<DateCivile>> {
    let premier = mois.premier_jour()?;
    let dernier = mois.dernier_jour()?;
    (0..=premier.jours_jusqua(dernier))
        .map(|i| premier.decaler(i))
        .collect()
}

fn ouvrable(jour: DateCivile, samedi_compte: bool) -> bool {
    match jour.jour_de_semaine() {
        0..=4 => true,
        5 => samedi_compte,
        _ => false,
    }
}

/// Fraction d'un mois payée par un engagement selon le prorata, `(payés, sur)` : un mois entier
/// vaut toujours tout. Au trentième, chaque jour compte un trentième et le dernier jour du mois
/// complète à trente : la fraction de deux engagements successifs fait bien trente trentièmes.
pub fn fraction_payee(
    prorata: Prorata,
    mois: Mois,
    couvre: impl Fn(DateCivile) -> bool,
    samedi_compte: bool,
) -> Option<(i64, i64)> {
    let jours = jours_du_mois(mois)?;
    let n = i64::try_from(jours.len()).ok()?;
    Some(match prorata {
        Prorata::Trentieme => {
            let payes: i64 = jours
                .iter()
                .enumerate()
                .filter(|(_, j)| couvre(**j))
                .map(|(i, _)| {
                    let rang = i64::try_from(i).unwrap_or(0) + 1;
                    if rang < n { 1 } else { 30 - (n - 1) }
                })
                .sum();
            (payes.clamp(0, 30), 30)
        }
        Prorata::Calendrier => {
            let couverts = jours.iter().filter(|j| couvre(**j)).count();
            (i64::try_from(couverts).ok()?, n)
        }
        Prorata::Ouvrables => {
            let ouvrables: Vec<&DateCivile> = jours
                .iter()
                .filter(|j| ouvrable(**j, samedi_compte))
                .collect();
            let couverts = ouvrables.iter().filter(|j| couvre(***j)).count();
            (
                i64::try_from(couverts).ok()?,
                i64::try_from(ouvrables.len()).ok()?.max(1),
            )
        }
    })
}

/// Part d'une journée d'absence dans un salaire mensuel, selon le prorata : `(1, 30)`, un jour du
/// mois, ou un jour ouvrable du mois ; une demi-journée en vaut la moitié.
fn part_journee(
    prorata: Prorata,
    mois: Mois,
    samedi_compte: bool,
    demi: bool,
) -> Option<(i64, i64)> {
    let (_, sur) = fraction_payee(prorata, mois, |_| true, samedi_compte)?;
    Some((1, if demi { sur * 2 } else { sur }))
}

/// Impôt d'un barème par tranches sur une assiette : chaque tranche à son taux, cumulées.
fn impot_progressif(
    tranches: &[crate::paie::TrancheBareme],
    assiette: Montant,
) -> Result<Montant, &'static str> {
    let mut impot = Montant::ZERO;
    let mut bas = 0_i64;
    for t in tranches {
        let haut = t.jusqua.map_or(i64::MAX, |b| b.francs());
        if assiette.francs() <= bas {
            break;
        }
        let dans = assiette.francs().min(haut) - bas;
        impot = impot
            .ajouter(part(Montant::depuis_francs(dans), t.taux, TAUX_MAX)?)
            .map_err(|_| IMPOSSIBLE)?;
        bas = haut;
    }
    Ok(impot)
}

struct Contexte<'a> {
    d: &'a DonneesBulletin<'a>,
}

impl Contexte<'_> {
    /// Natures des engagements payés ce mois : un engagement mensuel qui couvre le mois et le
    /// paie, ou une vacation qui a des heures dans ce bulletin.
    fn natures_payees(&self) -> Vec<NatureEngagement> {
        let mut natures = Vec::new();
        for e in self.d.engagements {
            let paye = match e.nature {
                NatureEngagement::Vacation => self.d.heures.iter().any(|h| h.engagement_id == e.id),
                NatureEngagement::Benevole => false,
                _ => self.base_de(e).is_some(),
            };
            if paye && !natures.contains(&e.nature) {
                natures.push(e.nature);
            }
        }
        natures
    }

    /// Salaire mensuel d'un engagement mensuel payé ce mois.
    fn base_de(&self, e: &Engagement) -> Option<Montant> {
        let Remuneration::Mensuelle { montant } = e.remuneration else {
            return None;
        };
        let (premier, dernier) = (self.d.mois.premier_jour()?, self.d.mois.dernier_jour()?);
        let couvre_le_mois = e.debut <= dernier && e.dernier_jour().is_none_or(|f| f >= premier);
        (couvre_le_mois
            && mois_paye(e.paye_douze_mois, self.d.mois_vacances, self.d.mois)
            && montant.francs() > 0)
            .then_some(montant)
    }

    fn s_applique(&self, element: &ElementPaie, natures: &[NatureEngagement]) -> bool {
        natures
            .iter()
            .any(|n| element.s_applique(*n, self.d.fonctionnaire))
    }

    fn attribution(&self, element: &ElementPaie) -> Option<&Attribution> {
        self.d
            .attributions
            .iter()
            .find(|a| a.element_id == element.id && a.couvre(self.d.mois))
    }

    fn version(&self, element: &ElementPaie) -> Option<&VersionElement> {
        let siennes: Vec<VersionElement> = self
            .d
            .versions
            .iter()
            .filter(|v| v.element_id == element.id)
            .cloned()
            .collect();
        let effet = version_en_vigueur(&siennes, self.d.mois)?.effet;
        self.d
            .versions
            .iter()
            .find(|v| v.element_id == element.id && v.effet == effet)
    }

    /// Assiette d'un taux ou d'un barème : le brut moins les retenues désignées déjà calculées.
    fn assiette(
        &self,
        brut: Montant,
        version: &VersionElement,
        calculees: &[LigneCalculee],
    ) -> Result<Montant, &'static str> {
        let deduit = somme(
            calculees
                .iter()
                .filter(|l| {
                    l.element_id
                        .is_some_and(|id| version.assiette_deduit.contains(&id))
                })
                .map(|l| &l.montant),
        )?;
        let reste = brut.soustraire(deduit).map_err(|_| IMPOSSIBLE)?;
        Ok(if reste.est_negatif() {
            Montant::ZERO
        } else {
            reste
        })
    }

    /// Une retenue ou une cotisation calculée sur son assiette.
    fn calculer_sur_assiette(
        &self,
        element: &ElementPaie,
        version: &VersionElement,
        assiette: Montant,
    ) -> Result<Option<LigneCalculee>, &'static str> {
        let sorte = if element.nature == NatureElement::CotisationPatronale {
            SorteLigne::Patronale
        } else {
            SorteLigne::Retenue
        };
        let mut ligne = LigneCalculee::simple(sorte, &element.libelle, Montant::ZERO);
        ligne.element_id = Some(element.id);
        match &version.calcul {
            Calcul::Fixe { montant } => ligne.montant = *montant,
            Calcul::Taux { taux, plafond } => {
                let base = plafond.map_or(assiette, |p| {
                    if assiette.francs() > p.francs() {
                        p
                    } else {
                        assiette
                    }
                });
                ligne.base = Some(base);
                ligne.taux = Some(*taux);
                ligne.montant = part(base, *taux, TAUX_MAX)?;
            }
            Calcul::Bareme {
                tranches,
                annuel,
                reduction_points,
                famille,
            } => {
                let periodes = if *annuel { 12 } else { 1 };
                let base = assiette.multiplier(periodes).map_err(|_| IMPOSSIBLE)?;
                let mut impot = impot_progressif(tranches, base)?;
                if *reduction_points > 0 {
                    let reduction = part(base, *reduction_points, TAUX_MAX)?;
                    impot = impot.soustraire(reduction).map_err(|_| IMPOSSIBLE)?;
                    if impot.est_negatif() {
                        impot = Montant::ZERO;
                    }
                }
                if let Some(f) = famille {
                    let mut taux = 0;
                    if self.d.situation_famille == Some(SituationFamille::Marie) {
                        taux += f.marie;
                    }
                    taux += f.par_enfant * self.d.enfants_a_charge.unwrap_or(0);
                    let taux = taux.min(f.plafond);
                    impot = impot
                        .soustraire(part(impot, taux, TAUX_MAX)?)
                        .map_err(|_| IMPOSSIBLE)?;
                }
                ligne.base = Some(assiette);
                ligne.montant = part(impot, 1, periodes)?;
            }
            Calcul::Saisi => return Ok(None),
        }
        Ok(Some(ligne))
    }
}

/// Calcule le bulletin d'un membre pour un mois ; vide s'il n'a rien de payé ce mois-là.
pub fn calculer(d: &DonneesBulletin<'_>) -> Result<ResultatBulletin, &'static str> {
    let c = Contexte { d };
    let natures = c.natures_payees();
    let mut gains: Vec<LigneCalculee> = Vec::new();
    let mut avertissements: Vec<String> = Vec::new();

    // 1. Salaire de base de chaque engagement mensuel, au prorata de ses jours.
    for e in d.engagements {
        let Some(base) = c.base_de(e) else {
            continue;
        };
        let (payes, sur) = fraction_payee(d.prorata, d.mois, |j| e.couvre(j), d.samedi_compte)
            .ok_or(IMPOSSIBLE)?;
        if payes == 0 {
            continue;
        }
        let mut ligne = LigneCalculee::simple(
            SorteLigne::Base,
            if e.nature == NatureEngagement::Stage {
                "Indemnité de stage"
            } else {
                "Salaire de base"
            },
            part(base, payes, sur)?,
        );
        ligne.engagement_id = Some(e.id);
        ligne.base = Some(base);
        ligne.jours = Some((payes, sur));
        gains.push(ligne);
    }
    // 2. Heures visées, du mois et en rappel.
    for h in d.heures {
        let mut ligne = LigneCalculee::simple(
            SorteLigne::Heures,
            "Heures de vacation",
            montant_des_heures(h.taux, h.minutes).ok_or(IMPOSSIBLE)?,
        );
        ligne.engagement_id = Some(h.engagement_id);
        ligne.heures_id = Some(h.heures_id);
        ligne.base = Some(h.taux);
        ligne.minutes = Some(h.minutes);
        ligne.mois_rappel = (h.mois != d.mois).then_some(h.mois);
        gains.push(ligne);
    }
    if gains.is_empty() && d.saisies.is_empty() {
        return Ok(ResultatBulletin {
            lignes: Vec::new(),
            brut: Montant::ZERO,
            retenues: Montant::ZERO,
            avances: Montant::ZERO,
            net: Montant::ZERO,
            patronales: Montant::ZERO,
            cumuls: d.cumuls_anterieurs,
            avertissements,
        });
    }
    let salaire_de_base = somme(
        gains
            .iter()
            .filter(|l| l.sorte == SorteLigne::Base)
            .map(|l| &l.montant),
    )?;
    // 3. Gains de l'école : un montant fixe, ou celui attribué au membre ; un taux du salaire de
    //    base ; un gain saisi attribué.
    for element in d
        .elements
        .iter()
        .filter(|e| e.actif && e.nature == NatureElement::Gain)
    {
        let attribution = c.attribution(element);
        let Some(version) = c.version(element) else {
            continue;
        };
        let montant = match (&version.calcul, attribution) {
            (_, Some(a)) => a.montant,
            (Calcul::Fixe { montant }, None) if c.s_applique(element, &natures) => *montant,
            (Calcul::Taux { taux, .. }, None) if c.s_applique(element, &natures) => {
                part(salaire_de_base, *taux, TAUX_MAX)?
            }
            _ => continue,
        };
        let mut ligne = LigneCalculee::simple(SorteLigne::Gain, &element.libelle, montant);
        ligne.element_id = Some(element.id);
        gains.push(ligne);
    }
    for s in d.saisies {
        let Some(element) = d.elements.iter().find(|e| e.id == s.element_id) else {
            continue;
        };
        if element.nature == NatureElement::Gain {
            let mut ligne = LigneCalculee::simple(SorteLigne::Gain, &s.libelle, s.montant);
            ligne.element_id = Some(element.id);
            gains.push(ligne);
        }
    }
    let brut = somme(gains.iter().map(|l| &l.montant))?;

    // 4. Retenues salariales, dans l'ordre des éléments, chacune sur son assiette.
    let mut retenues: Vec<LigneCalculee> = Vec::new();
    for element in d
        .elements
        .iter()
        .filter(|e| e.actif && e.nature == NatureElement::Retenue)
    {
        let Some(version) = c.version(element) else {
            continue;
        };
        if let Some(a) = c.attribution(element) {
            let mut ligne = LigneCalculee::simple(SorteLigne::Retenue, &element.libelle, a.montant);
            ligne.element_id = Some(element.id);
            retenues.push(ligne);
            continue;
        }
        if !c.s_applique(element, &natures) {
            continue;
        }
        let assiette = c.assiette(brut, version, &retenues)?;
        if let Some(ligne) = c.calculer_sur_assiette(element, version, assiette)? {
            retenues.push(ligne);
        }
    }
    for s in d.saisies {
        let Some(element) = d.elements.iter().find(|e| e.id == s.element_id) else {
            continue;
        };
        if element.nature == NatureElement::Retenue {
            let mut ligne = LigneCalculee::simple(SorteLigne::Retenue, &s.libelle, s.montant);
            ligne.element_id = Some(element.id);
            retenues.push(ligne);
        }
    }
    // 5. Absences confirmées, sur le salaire de base de l'engagement qui couvre le jour.
    for a in d.absences {
        let Some(base) = d
            .engagements
            .iter()
            .filter(|e| e.couvre(a.date))
            .find_map(|e| c.base_de(e))
        else {
            continue;
        };
        let (n, sur) =
            part_journee(d.prorata, d.mois, d.samedi_compte, a.demi_journee).ok_or(IMPOSSIBLE)?;
        let mut ligne = LigneCalculee::simple(
            SorteLigne::Absence,
            if a.demi_journee {
                "Absence d'une demi-journée"
            } else {
                "Absence d'une journée"
            },
            part(base, n, sur)?,
        );
        ligne.base = Some(base);
        ligne.date = Some(a.date);
        retenues.push(ligne);
    }
    let total_retenues = somme(retenues.iter().map(|l| &l.montant))?;

    // 6. Cotisations patronales, hors du net.
    let mut patronales: Vec<LigneCalculee> = Vec::new();
    for element in d
        .elements
        .iter()
        .filter(|e| e.actif && e.nature == NatureElement::CotisationPatronale)
    {
        if !c.s_applique(element, &natures) {
            continue;
        }
        let Some(version) = c.version(element) else {
            continue;
        };
        let assiette = c.assiette(brut, version, &retenues)?;
        if let Some(ligne) = c.calculer_sur_assiette(element, version, assiette)? {
            patronales.push(ligne);
        }
    }

    // 7. Avances : la mensualité, sans dépasser le reste dû ni ce qui reste à payer.
    let mut disponible = brut.soustraire(total_retenues).map_err(|_| IMPOSSIBLE)?;
    if disponible.est_negatif() {
        avertissements.push("paie.net_negatif".to_owned());
    }
    let mut avances: Vec<LigneCalculee> = Vec::new();
    for a in d.avances {
        let mut montant = a.mensualite;
        for borne in [a.reste, disponible] {
            if montant.francs() > borne.francs() {
                montant = if borne.est_negatif() {
                    Montant::ZERO
                } else {
                    borne
                };
            }
        }
        if montant.francs() < a.mensualite.francs() && montant.francs() < a.reste.francs() {
            avertissements.push("paie.avance_reportee".to_owned());
        }
        disponible = disponible.soustraire(montant).map_err(|_| IMPOSSIBLE)?;
        let mut ligne = LigneCalculee::simple(SorteLigne::Avance, &a.libelle, montant);
        ligne.reste = Some(a.reste.soustraire(montant).map_err(|_| IMPOSSIBLE)?);
        ligne.avance_id = a.avance_id;
        avances.push(ligne);
    }
    let total_avances = somme(avances.iter().map(|l| &l.montant))?;
    let net = brut
        .soustraire(total_retenues)
        .and_then(|m| m.soustraire(total_avances))
        .map_err(|_| IMPOSSIBLE)?;
    let total_patronales = somme(patronales.iter().map(|l| &l.montant))?;
    let cumuls = Cumuls {
        brut: d
            .cumuls_anterieurs
            .brut
            .ajouter(brut)
            .map_err(|_| IMPOSSIBLE)?,
        retenues: d
            .cumuls_anterieurs
            .retenues
            .ajouter(total_retenues)
            .map_err(|_| IMPOSSIBLE)?,
        net: d
            .cumuls_anterieurs
            .net
            .ajouter(net)
            .map_err(|_| IMPOSSIBLE)?,
    };
    let mut lignes = gains;
    lignes.extend(retenues);
    lignes.extend(avances);
    lignes.extend(patronales);
    Ok(ResultatBulletin {
        lignes,
        brut,
        retenues: total_retenues,
        avances: total_avances,
        net,
        patronales: total_patronales,
        cumuls,
        avertissements,
    })
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::paie::element::{ELEMENTS_LIVRES, NATURES_PAYEES};
    use crate::rh::StatutEngagement;

    fn f(v: i64) -> Montant {
        Montant::depuis_francs(v)
    }

    fn jour(t: &str) -> DateCivile {
        DateCivile::analyser(t).unwrap_or_else(|| unreachable!())
    }

    fn mois(t: &str) -> Mois {
        Mois::analyser(t).unwrap_or_else(|| unreachable!())
    }

    fn id<T>(n: u8) -> Id<T> {
        Id::depuis_octets([n; 16])
    }

    /// Les éléments livrés, ceux dont le code est donné activés, et une prime de transport fixe
    /// de 10 000 F aux permanents.
    fn ecole(actifs: &[&str]) -> (Vec<ElementPaie>, Vec<VersionElement>) {
        let mut elements = Vec::new();
        let mut versions = Vec::new();
        for (i, l) in ELEMENTS_LIVRES.iter().enumerate() {
            let rang = u8::try_from(i).unwrap_or(0) + 10;
            elements.push(ElementPaie {
                id: id(rang),
                etablissement_id: id(1),
                code: Some(l.code.to_owned()),
                libelle: l.libelle.to_owned(),
                nature: l.nature,
                natures: NATURES_PAYEES.to_vec(),
                fonctionnaires: l.fonctionnaires,
                ordre: i64::from(rang),
                actif: l.actif || actifs.contains(&l.code),
            });
        }
        for (i, l) in ELEMENTS_LIVRES.iter().enumerate() {
            let rang = u8::try_from(i).unwrap_or(0) + 10;
            versions.push(VersionElement {
                id: id(rang + 100),
                etablissement_id: id(1),
                element_id: id(rang),
                effet: mois(l.effet),
                calcul: l.calcul.calcul(),
                assiette_deduit: l
                    .assiette_deduit
                    .iter()
                    .filter_map(|c| {
                        elements
                            .iter()
                            .find(|e| e.code.as_deref() == Some(*c))
                            .map(|e| e.id)
                    })
                    .collect(),
            });
        }
        elements.push(ElementPaie {
            id: id(90),
            etablissement_id: id(1),
            code: None,
            libelle: "Prime de transport".to_owned(),
            nature: NatureElement::Gain,
            natures: vec![
                NatureEngagement::DureeIndeterminee,
                NatureEngagement::DureeDeterminee,
            ],
            fonctionnaires: true,
            ordre: 50,
            actif: actifs.contains(&"TRANSPORT"),
        });
        versions.push(VersionElement {
            id: id(190),
            etablissement_id: id(1),
            element_id: id(90),
            effet: mois("2026-10"),
            calcul: Calcul::Fixe { montant: f(10_000) },
            assiette_deduit: vec![],
        });
        (elements, versions)
    }

    fn engagement(
        n: u8,
        nature: NatureEngagement,
        debut: &str,
        fin: Option<&str>,
        remuneration: Remuneration,
    ) -> Engagement {
        Engagement {
            id: id(n),
            etablissement_id: id(1),
            membre_id: id(2),
            nature,
            debut: jour(debut),
            fin: fin.map(jour),
            fonction: "Secrétaire".to_owned(),
            remuneration,
            paye_douze_mois: true,
            statut: StatutEngagement::EnCours,
            arrete_le: None,
            motif_fin: String::new(),
            precedent_id: None,
            note: String::new(),
        }
    }

    fn cdi(n: u8, debut: &str, salaire: i64) -> Engagement {
        engagement(
            n,
            NatureEngagement::DureeIndeterminee,
            debut,
            None,
            Remuneration::Mensuelle {
                montant: f(salaire),
            },
        )
    }

    fn donnees<'a>(
        m: &str,
        engagements: &'a [Engagement],
        elements: &'a [ElementPaie],
        versions: &'a [VersionElement],
    ) -> DonneesBulletin<'a> {
        DonneesBulletin {
            mois: mois(m),
            prorata: Prorata::Trentieme,
            mois_vacances: &[7, 8, 9],
            samedi_compte: false,
            fonctionnaire: false,
            situation_famille: None,
            enfants_a_charge: None,
            engagements,
            heures: &[],
            elements,
            versions,
            attributions: &[],
            saisies: &[],
            absences: &[],
            avances: &[],
            cumuls_anterieurs: Cumuls::default(),
        }
    }

    fn ligne<'a>(r: &'a ResultatBulletin, libelle: &str) -> &'a LigneCalculee {
        r.lignes
            .iter()
            .find(|l| l.libelle == libelle)
            .unwrap_or_else(|| unreachable!("{libelle} absent"))
    }

    fn calcul(d: &DonneesBulletin<'_>) -> ResultatBulletin {
        calculer(d).unwrap_or_else(|e| unreachable!("{e}"))
    }

    #[test]
    fn la_secretaire_en_octobre_cas_de_reference() {
        // Salaire de base 90 000 F, prime de transport 10 000 F : brut 100 000 F. Mariée, deux
        // enfants. Avance de 30 000 F en trois mois.
        let (elements, versions) = ecole(&[
            "INPS_RET",
            "AMO",
            "ITS",
            "INPS_RET_PAT",
            "INPS_PF",
            "TRANSPORT",
        ]);
        let engagements = [cdi(3, "2025-10-01", 90_000)];
        let avances = [AvanceARembourser {
            avance_id: None,
            libelle: "Avance du 1er octobre".to_owned(),
            mensualite: f(10_000),
            reste: f(30_000),
        }];
        let mut d = donnees("2026-10", &engagements, &elements, &versions);
        d.situation_famille = Some(SituationFamille::Marie);
        d.enfants_a_charge = Some(2);
        d.avances = &avances;
        d.cumuls_anterieurs = Cumuls {
            brut: f(900_000),
            retenues: f(97_353),
            net: f(802_647),
        };
        let r = calcul(&d);
        assert_eq!(ligne(&r, "Salaire de base").montant, f(90_000));
        assert_eq!(ligne(&r, "Salaire de base").jours, Some((30, 30)));
        assert_eq!(ligne(&r, "Prime de transport").montant, f(10_000));
        assert_eq!(r.brut, f(100_000));
        // Retraite 3,6 % et AMO 3,06 % du brut.
        assert_eq!(ligne(&r, "Retraite INPS").montant, f(3_600));
        assert_eq!(ligne(&r, "AMO").montant, f(3_060));
        // ITS : assiette 100 000 − 3 600 = 96 400 F, soit 1 156 800 F l'an ; barème :
        // 248 400 × 5 % + 578 400 × 12 % = 12 420 + 69 408 = 81 828 F ; moins 2 % de l'assiette
        // annuelle (23 136 F) : 58 692 F ; moins 15 % pour la famille (8 803 F) : 49 889 F ; par
        // mois : 4 157 F.
        let its = ligne(&r, "Impôt sur les traitements et salaires");
        assert_eq!(its.base, Some(f(96_400)));
        assert_eq!(its.montant, f(4_157));
        assert_eq!(r.retenues, f(3_600 + 3_060 + 4_157));
        // Avance : 10 000 F retenus, 20 000 F restent.
        let avance = ligne(&r, "Avance du 1er octobre");
        assert_eq!(avance.montant, f(10_000));
        assert_eq!(avance.reste, Some(f(20_000)));
        assert_eq!(r.net, f(100_000 - 10_817 - 10_000));
        // Cotisations patronales : 5,4 % et 8 % du brut, hors du net.
        assert_eq!(
            ligne(&r, "Retraite INPS (part patronale)").montant,
            f(5_400)
        );
        assert_eq!(r.patronales, f(13_400));
        assert_eq!(
            r.cumuls,
            Cumuls {
                brut: f(1_000_000),
                retenues: f(97_353 + 10_817),
                net: f(802_647 + 79_183),
            }
        );
        assert!(r.avertissements.is_empty());
    }

    #[test]
    fn un_vacataire_fonctionnaire_paye_ses_heures_et_l_impot_seul() {
        // 22 h 30 à 2 500 F : 56 250 F. Fonctionnaire : ni retraite ni AMO ; l'ITS : 675 000 F
        // l'an, 12 420 + 96 600 × 12 % = 24 012 F, moins 13 500 F : 10 512 F, soit 876 F par mois.
        let (elements, versions) = ecole(&["INPS_RET", "AMO", "ITS"]);
        let vacation = engagement(
            4,
            NatureEngagement::Vacation,
            "2026-10-01",
            Some("2027-06-30"),
            Remuneration::Horaire {
                taux: f(2_500),
                minutes_par_semaine: 360,
            },
        );
        let engagements = [vacation];
        let heures = [
            HeuresAPayer {
                heures_id: id(5),
                engagement_id: id(4),
                mois: mois("2026-10"),
                minutes: 1_350,
                taux: f(2_500),
            },
            HeuresAPayer {
                heures_id: id(6),
                engagement_id: id(4),
                mois: mois("2026-09"),
                minutes: 0,
                taux: f(2_500),
            },
        ];
        let mut d = donnees("2026-10", &engagements, &elements, &versions);
        d.fonctionnaire = true;
        d.heures = &heures[..1];
        let r = calcul(&d);
        assert_eq!(r.brut, f(56_250));
        assert!(r.lignes.iter().all(|l| l.libelle != "Retraite INPS"));
        assert_eq!(
            ligne(&r, "Impôt sur les traitements et salaires").montant,
            f(876)
        );
        assert_eq!(r.net, f(55_374));
        // Un rappel garde son mois.
        d.heures = &heures;
        let r = calcul(&d);
        assert_eq!(r.lignes[1].mois_rappel, Some(mois("2026-09")));
    }

    #[test]
    fn un_mois_incomplet_suit_le_prorata_de_l_ecole() {
        let (elements, versions) = ecole(&[]);
        // Arrivée le vendredi 16 octobre 2026 : 15 trentièmes, 16 jours sur 31, 11 jours ouvrables
        // sur 22.
        let engagements = [cdi(3, "2026-10-16", 90_000)];
        let mut d = donnees("2026-10", &engagements, &elements, &versions);
        assert_eq!(calcul(&d).brut, f(45_000));
        d.prorata = Prorata::Calendrier;
        assert_eq!(calcul(&d).brut, f(46_451));
        d.prorata = Prorata::Ouvrables;
        let r = calcul(&d);
        assert_eq!(ligne(&r, "Salaire de base").jours, Some((11, 22)));
        assert_eq!(r.brut, f(45_000));
        // Un nouveau salaire au 16 : 15 trentièmes de chacun, le mois entier payé.
        let mut ancien = cdi(3, "2025-10-01", 90_000);
        ancien.statut = StatutEngagement::Termine;
        ancien.arrete_le = Some(jour("2026-10-15"));
        let engagements = [ancien, cdi(4, "2026-10-16", 100_000)];
        let d = donnees("2026-10", &engagements, &elements, &versions);
        assert_eq!(calcul(&d).brut, f(45_000 + 50_000));
        // En février, l'arrivée le 15 : 13 jours et le dernier qui en vaut trois, 16 trentièmes.
        let engagements = [cdi(3, "2027-02-15", 90_000)];
        let d = donnees("2027-02", &engagements, &elements, &versions);
        assert_eq!(calcul(&d).brut, f(48_000));
    }

    #[test]
    fn les_vacances_ne_paient_que_les_engagements_de_douze_mois() {
        let (elements, versions) = ecole(&[]);
        let mut annee = cdi(3, "2026-10-01", 90_000);
        annee.paye_douze_mois = false;
        let engagements = [annee.clone()];
        let r = calcul(&donnees("2027-08", &engagements, &elements, &versions));
        assert!(r.lignes.is_empty());
        assert_eq!(r.net, Montant::ZERO);
        annee.paye_douze_mois = true;
        let engagements = [annee];
        assert_eq!(
            calcul(&donnees("2027-08", &engagements, &elements, &versions)).net,
            f(90_000)
        );
    }

    #[test]
    fn une_absence_confirmee_se_retient_au_trentieme_ou_au_jour_ouvrable() {
        let (elements, versions) = ecole(&[]);
        let engagements = [cdi(3, "2025-10-01", 90_000)];
        let absences = [
            AbsenceRetenue {
                date: jour("2026-10-13"),
                demi_journee: false,
            },
            AbsenceRetenue {
                date: jour("2026-10-14"),
                demi_journee: true,
            },
        ];
        let mut d = donnees("2026-10", &engagements, &elements, &versions);
        d.absences = &absences;
        let r = calcul(&d);
        assert_eq!(r.retenues, f(3_000 + 1_500));
        assert_eq!(r.net, f(85_500));
        d.prorata = Prorata::Ouvrables;
        assert_eq!(calcul(&d).retenues, f(4_090 + 2_045));
    }

    #[test]
    fn une_avance_ne_rend_jamais_le_net_negatif() {
        let (elements, versions) = ecole(&[]);
        let stage = engagement(
            3,
            NatureEngagement::Stage,
            "2026-10-01",
            Some("2027-06-30"),
            Remuneration::Mensuelle { montant: f(5_000) },
        );
        let engagements = [stage];
        let avances = [AvanceARembourser {
            avance_id: None,
            libelle: "Avance".to_owned(),
            mensualite: f(10_000),
            reste: f(30_000),
        }];
        let mut d = donnees("2026-10", &engagements, &elements, &versions);
        d.avances = &avances;
        let r = calcul(&d);
        assert_eq!(ligne(&r, "Indemnité de stage").montant, f(5_000));
        assert_eq!(ligne(&r, "Avance").montant, f(5_000));
        assert_eq!(ligne(&r, "Avance").reste, Some(f(25_000)));
        assert_eq!(r.net, Montant::ZERO);
        assert_eq!(r.avertissements, ["paie.avance_reportee"]);
    }

    #[test]
    fn plafond_saisies_et_net_negatif() {
        let (mut elements, mut versions) = ecole(&[]);
        elements.push(ElementPaie {
            id: id(91),
            etablissement_id: id(1),
            code: None,
            libelle: "Mutuelle".to_owned(),
            nature: NatureElement::Retenue,
            natures: NATURES_PAYEES.to_vec(),
            fonctionnaires: true,
            ordre: 60,
            actif: true,
        });
        versions.push(VersionElement {
            id: id(191),
            etablissement_id: id(1),
            element_id: id(91),
            effet: mois("2026-01"),
            calcul: Calcul::Taux {
                taux: 1_000,
                plafond: Some(f(50_000)),
            },
            assiette_deduit: vec![],
        });
        let engagements = [cdi(3, "2025-10-01", 100_000)];
        let regul = elements
            .iter()
            .find(|e| e.code.as_deref() == Some("REGUL"))
            .map(|e| e.id)
            .unwrap_or_else(|| unreachable!());
        let retenue = elements
            .iter()
            .find(|e| e.code.as_deref() == Some("RETENUE"))
            .map(|e| e.id)
            .unwrap_or_else(|| unreachable!());
        let saisies = [SaisiePaie {
            element_id: regul,
            montant: f(-5_000),
            libelle: "Trop-perçu de septembre".to_owned(),
        }];
        let mut d = donnees("2026-10", &engagements, &elements, &versions);
        d.saisies = &saisies;
        let r = calcul(&d);
        // 10 % d'une assiette plafonnée à 50 000 F ; la régularisation retire 5 000 F du brut.
        assert_eq!(r.brut, f(95_000));
        let mutuelle = ligne(&r, "Mutuelle");
        assert_eq!(
            (mutuelle.base, mutuelle.montant),
            (Some(f(50_000)), f(5_000))
        );
        assert_eq!(r.net, f(90_000));
        // Une retenue saisie plus forte que le brut : le net négatif est signalé.
        let saisies = [SaisiePaie {
            element_id: retenue,
            montant: f(120_000),
            libelle: "Retenue".to_owned(),
        }];
        d.saisies = &saisies;
        let r = calcul(&d);
        assert_eq!(r.avertissements, ["paie.net_negatif"]);
        assert!(r.net.est_negatif());
    }

    #[test]
    fn le_trentieme_fait_toujours_trente() {
        for m in ["2026-10", "2026-11", "2027-02", "2028-02"] {
            let mois = mois(m);
            let jours = jours_du_mois(mois).unwrap_or_default();
            for coupure in &jours {
                let avant = fraction_payee(Prorata::Trentieme, mois, |j| j < *coupure, false);
                let apres = fraction_payee(Prorata::Trentieme, mois, |j| j >= *coupure, false);
                let total = avant.map_or(0, |a| a.0) + apres.map_or(0, |a| a.0);
                assert_eq!(total, 30, "{m} coupé le {coupure}");
            }
        }
    }
}
