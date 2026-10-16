//! Paie (SPEC_PHASE_7) : éléments de paie de l'école et versions datées de leur calcul,
//! attributions à un membre, réglages des mois payés et du prorata. Les retenues du Mali sont
//! livrées inactives, aux taux proposés, à vérifier par l'école (Q-79).

use gesco_core::commun::{Id, Montant};
use gesco_core::core::parametre::{Parametre, ValeurParametre, hors_bornes};
use gesco_core::core::{Etablissement, Operation, TypeEvenement};
use gesco_core::paie::element::{
    ELEMENTS_LIVRES, NATURES_PAYEES, verifier_attribution, verifier_calcul, verifier_element,
    version_en_vigueur,
};
use gesco_core::paie::reglages::{
    MOIS_VACANCES_DEFAUT, PARAMETRE_DOUZE_MOIS, PARAMETRE_MOIS_VACANCES, PARAMETRE_PRORATA,
    ecrire_mois_vacances, lire_mois_vacances,
};
use gesco_core::paie::{Attribution, Calcul, ElementPaie, NatureElement, Prorata, VersionElement};
use gesco_core::rh::{MembrePersonnel, Mois, NatureEngagement};
use gesco_db::depots::{paie, parametre, personnel, personnes};
use gesco_db::unite::Durabilite;
use gesco_db::{UniteDeTravail, nouvel_id};
use serde::{Deserialize, Serialize};

use crate::effets::Effet;
use crate::erreur::{ErreurService, Resultat};
use crate::noyau::Noyau;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Pose les éléments livrés qui manquent à l'établissement, reconnus par leur code, chacun avec
/// la version proposée de son calcul. Rend le nombre d'éléments ajoutés.
pub(crate) async fn poser_elements(
    unite: &mut UniteDeTravail<Effet>,
    etablissement_id: Id<Etablissement>,
    origine: &str,
) -> Resultat<usize> {
    let contexte = unite.contexte().clone();
    let mut existants = paie::elements(unite.connexion(), &contexte).await?;
    let mut ajoutes = 0;
    for (ordre, livre) in (1_i64..).zip(ELEMENTS_LIVRES) {
        if existants
            .iter()
            .any(|e| e.code.as_deref() == Some(livre.code))
        {
            continue;
        }
        let element = ElementPaie {
            id: nouvel_id(),
            etablissement_id,
            code: Some(livre.code.to_owned()),
            libelle: livre.libelle.to_owned(),
            nature: livre.nature,
            natures: NATURES_PAYEES.to_vec(),
            fonctionnaires: livre.fonctionnaires,
            ordre,
            actif: livre.actif,
        };
        let effet = Mois::analyser(livre.effet).ok_or(regle("element.effet_invalide"))?;
        let version = VersionElement {
            id: nouvel_id(),
            etablissement_id,
            element_id: element.id,
            effet,
            calcul: livre.calcul.calcul(),
            assiette_deduit: livre
                .assiette_deduit
                .iter()
                .filter_map(|code| {
                    existants
                        .iter()
                        .find(|e| e.code.as_deref() == Some(*code))
                        .map(|e| e.id)
                })
                .collect(),
        };
        paie::inserer_element(unite.connexion(), &element, &contexte).await?;
        paie::inserer_version(unite.connexion(), &version, &contexte).await?;
        unite.auditer(
            "element_paie",
            &element.id.vers_chaine(),
            Operation::Creation,
            None,
            Some(serde_json::json!({ "element": element, "version": version })),
            origine,
        );
        existants.push(element);
        ajoutes += 1;
    }
    Ok(ajoutes)
}

/// Réglages de la paie de l'école.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ReglagesPaie {
    /// Un nouvel engagement mensuel est payé douze mois (sinon, hors des mois de vacances).
    pub douze_mois: bool,
    /// Mois de vacances, de 1 à 12, triés.
    pub mois_vacances: Vec<u8>,
    /// Règle du prorata d'un mois incomplet.
    pub prorata: Prorata,
}

/// Lit les réglages de la paie, avec leurs valeurs livrées si l'école ne les a pas.
pub(crate) async fn reglages(
    cnx: &mut gesco_db::Connexion,
    contexte: &gesco_db::Contexte,
) -> Resultat<ReglagesPaie> {
    Ok(ReglagesPaie {
        douze_mois: parametre::booleen_param(cnx, PARAMETRE_DOUZE_MOIS, true, contexte).await?,
        mois_vacances: lire_mois_vacances(
            &parametre::texte(cnx, PARAMETRE_MOIS_VACANCES, MOIS_VACANCES_DEFAUT, contexte).await?,
        )
        .unwrap_or_else(|| vec![7, 8, 9]),
        prorata: Prorata::depuis_nom(
            &parametre::texte(cnx, PARAMETRE_PRORATA, Prorata::Trentieme.nom(), contexte).await?,
        )
        .unwrap_or(Prorata::Trentieme),
    })
}

/// Élément affiché : ses versions et celle en vigueur ce mois.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ElementAffiche {
    /// Élément.
    pub element: ElementPaie,
    /// Versions, de la plus ancienne à la plus récente.
    pub versions: Vec<VersionElement>,
    /// Version en vigueur ce mois.
    pub en_vigueur: Option<VersionElement>,
}

/// Attribution affichée.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct AttributionAffichee {
    /// Attribution.
    pub attribution: Attribution,
    /// Prénoms et nom du membre.
    pub membre: String,
    /// Matricule.
    pub matricule: String,
    /// Libellé de l'élément.
    pub element: String,
}

/// Éléments de paie, attributions et réglages de l'école.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ElementsPaie {
    /// Mois en cours.
    pub mois: Mois,
    /// Éléments, dans l'ordre du bulletin.
    pub elements: Vec<ElementAffiche>,
    /// Attributions, par membre.
    pub attributions: Vec<AttributionAffichee>,
    /// Réglages.
    pub reglages: ReglagesPaie,
}

/// Élément saisi ; sans `id`, une création avec son premier calcul, en vigueur ce mois.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeElement {
    /// Élément modifié ; absent pour une création.
    pub id: Option<Id<ElementPaie>>,
    /// Libellé.
    pub libelle: String,
    /// Nature, fixée à la création.
    pub nature: NatureElement,
    /// Natures d'engagement concernées.
    pub natures: Vec<NatureEngagement>,
    /// S'applique aussi aux fonctionnaires.
    pub fonctionnaires: bool,
    /// Actif.
    pub actif: bool,
    /// Calcul d'un élément créé.
    pub calcul: Option<Calcul>,
}

/// Version du calcul d'un élément, à partir d'un mois.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeVersion {
    /// Élément.
    pub element_id: Id<ElementPaie>,
    /// Premier mois d'application.
    pub effet: Mois,
    /// Calcul.
    pub calcul: Calcul,
    /// Retenues retranchées de l'assiette.
    pub assiette_deduit: Vec<Id<ElementPaie>>,
}

/// Attribution saisie ; sans `id`, une création.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeAttribution {
    /// Attribution modifiée ; absente pour une création.
    pub id: Option<Id<Attribution>>,
    /// Membre.
    pub membre_id: Id<MembrePersonnel>,
    /// Élément.
    pub element_id: Id<ElementPaie>,
    /// Montant mensuel.
    pub montant: Montant,
    /// Premier mois.
    pub debut: Mois,
    /// Dernier mois.
    pub fin: Option<Mois>,
}

/// Service de la paie.
pub struct ServicePaie<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServicePaie<'n> {
    /// Service sur le noyau ouvert.
    pub const fn nouveau(noyau: &'n Noyau) -> Self {
        Self { noyau }
    }

    async fn connexion(&self) -> Resultat<gesco_db::ConnexionEmpruntee> {
        Ok(self
            .noyau
            .base()
            .pool()
            .acquire()
            .await
            .map_err(gesco_db::ErreurBase::from)?)
    }

    fn mois_courant(&self) -> Resultat<Mois> {
        Ok(Mois::de(crate::inscriptions::jour(
            self.noyau.horloge().maintenant(),
        )?))
    }

    /// Éléments, attributions et réglages.
    pub async fn lire(&self, session: &Session) -> Resultat<ElementsPaie> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mois = self.mois_courant()?;
        let mut cnx = self.connexion().await?;
        let elements = paie::elements(&mut cnx, &contexte).await?;
        let versions = paie::versions(&mut cnx, &contexte).await?;
        let membres = personnel::membres(&mut cnx, &contexte).await?;
        let mut attributions = Vec::new();
        for a in paie::attributions(&mut cnx, &contexte).await? {
            let Some(m) = membres.iter().find(|m| m.id == a.membre_id) else {
                continue;
            };
            let p = personnes::personne(&mut cnx, m.personne_id, &contexte).await?;
            attributions.push(AttributionAffichee {
                membre: format!("{} {}", p.prenoms, p.nom),
                matricule: m.matricule.clone(),
                element: elements
                    .iter()
                    .find(|e| e.id == a.element_id)
                    .map(|e| e.libelle.clone())
                    .unwrap_or_default(),
                attribution: a,
            });
        }
        attributions.sort_by(|a, b| {
            a.membre
                .cmp(&b.membre)
                .then(a.attribution.debut.cmp(&b.attribution.debut))
        });
        let reglages = reglages(&mut cnx, &contexte).await?;
        Ok(ElementsPaie {
            mois,
            elements: elements
                .into_iter()
                .map(|e| {
                    let siennes: Vec<VersionElement> = versions
                        .iter()
                        .filter(|v| v.element_id == e.id)
                        .cloned()
                        .collect();
                    ElementAffiche {
                        en_vigueur: version_en_vigueur(&siennes, mois).cloned(),
                        versions: siennes,
                        element: e,
                    }
                })
                .collect(),
            attributions,
            reglages,
        })
    }

    /// Crée un élément avec son premier calcul, ou change son libellé, ses natures concernées et
    /// son état ; sa nature ne change pas.
    pub async fn enregistrer_element(
        &self,
        session: &Session,
        demande: &DemandeElement,
    ) -> Resultat<ElementsPaie> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mois = self.mois_courant()?;
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let existants = paie::elements(unite.connexion(), &contexte).await?;
        let libelle = demande.libelle.trim().to_owned();
        if existants
            .iter()
            .any(|e| e.libelle.to_lowercase() == libelle.to_lowercase() && Some(e.id) != demande.id)
        {
            return Err(regle("element.libelle_pris"));
        }
        let mut natures: Vec<NatureEngagement> = Vec::new();
        for n in &demande.natures {
            if !natures.contains(n) {
                natures.push(*n);
            }
        }
        match demande.id {
            Some(id) => {
                let avant = existants
                    .iter()
                    .find(|e| e.id == id)
                    .cloned()
                    .ok_or(regle("element.inconnu"))?;
                let apres = ElementPaie {
                    libelle,
                    natures,
                    fonctionnaires: demande.fonctionnaires,
                    actif: demande.actif,
                    ..avant.clone()
                };
                verifier_element(&apres).map_err(regle)?;
                paie::ecrire_element(unite.connexion(), &apres, &contexte).await?;
                tracer(
                    &mut unite,
                    &apres.id,
                    Operation::Modification,
                    Some(serde_json::to_value(&avant)?),
                    serde_json::to_value(&apres)?,
                    "paie.element",
                );
            }
            None => {
                let calcul = demande
                    .calcul
                    .clone()
                    .ok_or(regle("element.calcul_manquant"))?;
                let element = ElementPaie {
                    id: nouvel_id(),
                    etablissement_id: session.etablissement_id,
                    code: None,
                    libelle,
                    nature: demande.nature,
                    natures,
                    fonctionnaires: demande.fonctionnaires,
                    ordre: existants.iter().map(|e| e.ordre).max().unwrap_or(0) + 1,
                    actif: demande.actif,
                };
                verifier_element(&element).map_err(regle)?;
                verifier_calcul(element.nature, &calcul).map_err(regle)?;
                let version = VersionElement {
                    id: nouvel_id(),
                    etablissement_id: session.etablissement_id,
                    element_id: element.id,
                    effet: mois,
                    calcul,
                    assiette_deduit: Vec::new(),
                };
                paie::inserer_element(unite.connexion(), &element, &contexte).await?;
                paie::inserer_version(unite.connexion(), &version, &contexte).await?;
                tracer(
                    &mut unite,
                    &element.id,
                    Operation::Creation,
                    None,
                    serde_json::json!({ "element": element, "version": version }),
                    "paie.element",
                );
            }
        }
        self.noyau.terminer(unite).await?;
        self.lire(session).await
    }

    /// Donne à un élément un calcul à partir d'un mois : une nouvelle version, ou la version de
    /// ce mois corrigée.
    pub async fn enregistrer_version(
        &self,
        session: &Session,
        demande: &DemandeVersion,
    ) -> Resultat<ElementsPaie> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let elements = paie::elements(unite.connexion(), &contexte).await?;
        let element = elements
            .iter()
            .find(|e| e.id == demande.element_id)
            .ok_or(regle("element.inconnu"))?;
        verifier_calcul(element.nature, &demande.calcul).map_err(regle)?;
        // L'assiette ne retranche que des retenues salariales, jamais l'élément lui-même ; seuls
        // un taux et un barème ont une assiette.
        let assiette_deduit = match demande.calcul {
            Calcul::Taux { .. } | Calcul::Bareme { .. } => {
                let mut deduit: Vec<Id<ElementPaie>> = Vec::new();
                for id in &demande.assiette_deduit {
                    if !deduit.contains(id) {
                        deduit.push(*id);
                    }
                }
                for id in &deduit {
                    let ok = *id != element.id
                        && elements
                            .iter()
                            .any(|e| e.id == *id && e.nature == NatureElement::Retenue);
                    if !ok {
                        return Err(regle("element.assiette_invalide"));
                    }
                }
                deduit
            }
            Calcul::Fixe { .. } | Calcul::Saisi => Vec::new(),
        };
        let versions = paie::versions(unite.connexion(), &contexte).await?;
        let avant = versions
            .iter()
            .find(|v| v.element_id == element.id && v.effet == demande.effet)
            .cloned();
        let apres = VersionElement {
            id: avant.as_ref().map_or_else(nouvel_id, |v| v.id),
            etablissement_id: session.etablissement_id,
            element_id: element.id,
            effet: demande.effet,
            calcul: demande.calcul.clone(),
            assiette_deduit,
        };
        match &avant {
            Some(_) => paie::ecrire_version(unite.connexion(), &apres, &contexte).await?,
            None => paie::inserer_version(unite.connexion(), &apres, &contexte).await?,
        }
        tracer(
            &mut unite,
            &element.id,
            Operation::Modification,
            avant.as_ref().map(serde_json::to_value).transpose()?,
            serde_json::to_value(&apres)?,
            "paie.version",
        );
        self.noyau.terminer(unite).await?;
        self.lire(session).await
    }

    /// Attribue un gain ou une retenue à un membre, ou change le montant et la période d'une
    /// attribution ; elle se termine en lui donnant un dernier mois.
    pub async fn enregistrer_attribution(
        &self,
        session: &Session,
        demande: &DemandeAttribution,
    ) -> Resultat<ElementsPaie> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        personnel::membre(unite.connexion(), demande.membre_id, &contexte).await?;
        let element = paie::elements(unite.connexion(), &contexte)
            .await?
            .into_iter()
            .find(|e| e.id == demande.element_id)
            .ok_or(regle("element.inconnu"))?;
        let existantes = paie::attributions(unite.connexion(), &contexte).await?;
        let avant = match demande.id {
            Some(id) => Some(
                existantes
                    .iter()
                    .find(|a| a.id == id)
                    .cloned()
                    .ok_or(regle("attribution.inconnue"))?,
            ),
            None => None,
        };
        let apres = Attribution {
            id: avant.as_ref().map_or_else(nouvel_id, |a| a.id),
            etablissement_id: session.etablissement_id,
            membre_id: demande.membre_id,
            element_id: element.id,
            montant: demande.montant,
            debut: demande.debut,
            fin: demande.fin,
        };
        verifier_attribution(&element, &apres).map_err(regle)?;
        match &avant {
            Some(_) => paie::ecrire_attribution(unite.connexion(), &apres, &contexte).await?,
            None => paie::inserer_attribution(unite.connexion(), &apres, &contexte).await?,
        }
        let id = apres.id.vers_chaine();
        unite.auditer(
            "attribution_element",
            &id,
            if avant.is_some() {
                Operation::Modification
            } else {
                Operation::Creation
            },
            avant.as_ref().map(serde_json::to_value).transpose()?,
            Some(serde_json::to_value(&apres)?),
            "paie.attribution",
        );
        unite.emettre(
            TypeEvenement::ElementPaieModifie,
            "attribution_element",
            &id,
            serde_json::json!({ "membre": apres.membre_id, "element": apres.element_id }),
        );
        self.noyau.terminer(unite).await?;
        self.lire(session).await
    }

    /// Règle les mois payés et le prorata de l'école.
    pub async fn regler(
        &self,
        session: &Session,
        demande: &ReglagesPaie,
    ) -> Resultat<ElementsPaie> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        let valeurs = [
            (
                PARAMETRE_DOUZE_MOIS,
                ValeurParametre::Booleen(demande.douze_mois),
            ),
            (
                PARAMETRE_MOIS_VACANCES,
                ValeurParametre::Texte(ecrire_mois_vacances(&demande.mois_vacances)),
            ),
            (
                PARAMETRE_PRORATA,
                ValeurParametre::Texte(demande.prorata.nom().to_owned()),
            ),
        ];
        for (cle, valeur) in valeurs {
            if hors_bornes(cle, &valeur) {
                return Err(regle("paie.reglage_invalide"));
            }
            let avant = parametre::lire(unite.connexion(), cle, &contexte)
                .await?
                .ok_or(regle("parametre.inconnu"))?;
            if avant.valeur == valeur {
                continue;
            }
            let apres = Parametre {
                valeur,
                ..avant.clone()
            };
            parametre::definir(unite.connexion(), &apres, &contexte).await?;
            unite.auditer(
                "parametre",
                cle,
                Operation::Modification,
                Some(serde_json::to_value(&avant)?),
                Some(serde_json::to_value(&apres)?),
                "paie.reglages",
            );
        }
        self.noyau.terminer(unite).await?;
        self.lire(session).await
    }
}

/// Trace une écriture sur un élément ou sa version : audit et événement.
fn tracer(
    unite: &mut UniteDeTravail<Effet>,
    id: &Id<ElementPaie>,
    operation: Operation,
    avant: Option<serde_json::Value>,
    apres: serde_json::Value,
    origine: &str,
) {
    let id = id.vers_chaine();
    unite.auditer("element_paie", &id, operation, avant, Some(apres), origine);
    unite.emettre(
        TypeEvenement::ElementPaieModifie,
        "element_paie",
        &id,
        serde_json::json!({ "origine": origine }),
    );
}
