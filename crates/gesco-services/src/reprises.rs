//! Reprise avant GESCO (SPEC_REPRISE_AVANT_GESCO §3 à §6) : ce que les parents ont payé avant
//! l'installation réduit le reste à payer dès le premier jour, sans versement, sans reçu, sans
//! caisse. La reprise règle des échéances comme un avoir ; ses parts se recalculent dans
//! `facturation::recalculer_dans`, seul point d'entrée de la réimputation. Une modification
//! annule l'ancienne reprise et en crée une nouvelle, dans une transaction (R-14).

use std::collections::HashMap;

use gesco_core::commun::{DateCivile, Id, Montant};
use gesco_core::core::{AnneeScolaire, Operation, StatutAnnee, TypeEvenement};
use gesco_core::finance::reprise::{
    ChiffresReprise, OrigineReprise, RepriseAvantGesco, StatutReprise, date_d_arret, minimum,
    verifier_note,
};
use gesco_core::finance::{Facture, StatutFacture};
use gesco_core::scolarite::{Classe, Eleve, Inscription, StatutInscription};
use gesco_db::depots::fraicheur::Ligne;
use gesco_db::depots::{academique, ajustements, annee_scolaire, factures, inscriptions};
use gesco_db::depots::{personnes, reprises};
use gesco_db::unite::Durabilite;
use gesco_db::{Contexte, UniteDeTravail, nouvel_id};
use serde::{Deserialize, Serialize};

use crate::effets::Effet;
use crate::erreur::{ErreurService, Resultat};
use crate::facturation::{chiffres_reprise, etat_echeances, recalculer_dans};
use crate::inscriptions::jour;
use crate::noyau::Noyau;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

fn calcul(r: Result<Montant, gesco_core::commun::ErreurMontant>) -> Resultat<Montant> {
    r.map_err(|_| regle("facture.calcul_impossible"))
}

/// Motif d'annulation d'une reprise remplacée par un autre montant.
pub const MOTIF_REMPLACEE: &str = "remplacée";
/// Motif proposé quand la case est vidée.
pub const MOTIF_SAISIE_ERRONEE: &str = "Saisie erronée";

/// Sélection de l'écran « Payé avant GESCO ».
#[derive(Debug, Clone, Default, Deserialize, specta::Type)]
pub struct DemandeReprises {
    /// Année ; absente, l'année courante.
    #[serde(default)]
    pub annee_id: Option<Id<AnneeScolaire>>,
    /// Classe ; absente, toutes.
    #[serde(default)]
    pub classe_id: Option<Id<Classe>>,
}

/// Ligne de l'écran : une inscription validée.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct LigneReprise {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Élève.
    pub eleve_id: Id<Eleve>,
    /// Classe.
    pub classe: String,
    /// Nom et prénoms.
    pub eleve: String,
    /// Matricule interne.
    pub matricule: String,
    /// Dû de l'année, net des remises et des baisses.
    pub du: Montant,
    /// Payé avant GESCO : montant de la reprise active (zéro sans reprise).
    pub reprise: Montant,
    /// Note de la reprise active.
    pub note: String,
    /// Payé dans GESCO : versements et avoirs.
    pub paye_gesco: Montant,
    /// Reste à payer.
    pub reste: Montant,
    /// À rembourser au parent.
    pub a_rembourser: Montant,
    /// Déjà remboursé au parent.
    pub rembourse: Montant,
    /// Plus petit montant que la reprise peut prendre (déjà remboursé, avoirs à couvrir).
    pub minimum: Montant,
    /// Instant de lecture, renvoyé à l'enregistrement (modification simultanée).
    pub lue_le: i64,
}

/// Contenu de l'écran « Payé avant GESCO ».
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ListeReprises {
    /// Année montrée.
    pub annee_id: Id<AnneeScolaire>,
    /// Libellé de l'année.
    pub annee: String,
    /// Année close ou archivée : la liste se lit seulement.
    pub lecture_seule: bool,
    /// Arrêté avant GESCO : la veille de la première caisse de l'année.
    pub date_arret: DateCivile,
    /// Vrai tant qu'aucune caisse n'a été ouverte dans l'année (« à ce jour »).
    pub a_ce_jour: bool,
    /// Lignes, par classe puis par nom.
    pub lignes: Vec<LigneReprise>,
    /// Total payé avant GESCO de la sélection.
    pub total: Montant,
    /// Nombre d'élèves qui ont une reprise.
    pub nombre: u32,
    /// Total à rembourser.
    pub total_a_rembourser: Montant,
    /// Nombre de parents à rembourser.
    pub nombre_a_rembourser: u32,
    /// Inscriptions encore en brouillon dans la sélection.
    pub brouillons: u32,
}

/// Enregistrer, remplacer ou (montant zéro) annuler la reprise d'une inscription.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeReprise {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Montant payé avant GESCO ; zéro annule la reprise active.
    pub montant: Montant,
    /// Note libre (120 caractères au plus).
    #[serde(default)]
    pub note: String,
    /// Motif, pour un montant zéro qui annule ; vide, « Saisie erronée ».
    #[serde(default)]
    pub motif: String,
    /// Instant de lecture de la ligne.
    #[serde(default)]
    pub lue_le: Option<i64>,
}

/// Annuler la reprise d'une inscription.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeAnnulationReprise {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Motif, obligatoire.
    pub motif: String,
    /// Instant de lecture.
    #[serde(default)]
    pub lue_le: Option<i64>,
}

/// Inscription visée.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeInscriptionReprise {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
}

/// Remboursement fait (ou en attente) au parent, pour la situation de l'élève.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct Remboursement {
    /// Jour de la dépense.
    pub date: DateCivile,
    /// Montant.
    pub montant: Montant,
    /// Statut de la dépense (`a_approuver`, `approuvee`, `payee`).
    pub statut: String,
    /// Mode de paiement, une fois payée.
    pub mode: Option<String>,
}

/// Payé avant GESCO d'une inscription, pour la situation et le relevé.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct RepriseDeLInscription {
    /// Reprise active, s'il y en a une.
    pub active: Option<RepriseAvantGesco>,
    /// Chiffres : montant, remboursé, imputé, à rembourser.
    pub chiffres: ChiffresReprise,
    /// Remboursements engagés, le plus ancien d'abord.
    pub remboursements: Vec<Remboursement>,
    /// Date d'arrêt avant GESCO de l'année de l'inscription.
    pub date_arret: DateCivile,
    /// Vrai tant qu'aucune caisse n'a été ouverte dans l'année.
    pub a_ce_jour: bool,
}

/// Lignes de fraîcheur d'une inscription : ses reprises et sa facture.
fn lignes_fraicheur(inscription_id: Id<Inscription>, facture: &Facture) -> Vec<Ligne> {
    vec![
        Ligne {
            table: "reprise_avant_gesco",
            cles: vec![("inscription_id", inscription_id.vers_chaine())],
        },
        Ligne::par_id("facture", facture.id.vers_chaine()),
    ]
}

/// Année ouverte aux reprises : en préparation ou active.
fn annee_modifiable(annee: &AnneeScolaire) -> bool {
    matches!(annee.statut, StatutAnnee::Preparation | StatutAnnee::Active)
}

/// Date d'arrêt avant GESCO d'une année (§3.2), calculée.
pub(crate) async fn arret_de_l_annee(
    cnx: &mut gesco_db::Connexion,
    annee: &AnneeScolaire,
    contexte: &Contexte,
) -> Resultat<(DateCivile, bool)> {
    let premiere = reprises::premiere_caisse(cnx, annee.debut, annee.fin, contexte).await?;
    let premiere = premiere.map(jour).transpose()?;
    Ok(date_d_arret(premiere, jour(contexte.maintenant)?))
}

/// Facture vivante et année d'une inscription validée, ou le refus de la reprise.
async fn cible(
    cnx: &mut gesco_db::Connexion,
    inscription_id: Id<Inscription>,
    contexte: &Contexte,
) -> Resultat<(Inscription, Facture, AnneeScolaire)> {
    let inscription = inscriptions::lire(cnx, inscription_id, contexte).await?;
    if inscription.statut == StatutInscription::Brouillon {
        return Err(regle("reprise.sans_facture"));
    }
    let facture = factures::facture_vivante(cnx, inscription_id, contexte)
        .await?
        .filter(|f| f.statut != StatutFacture::Annulee)
        .ok_or(regle("reprise.sans_facture"))?;
    let annee = annee_scolaire::lire(cnx, inscription.annee_id, contexte).await?;
    Ok((inscription, facture, annee))
}

/// Plus petit montant permis pour la reprise d'une facture (§3.3), et la règle qui le fixe.
async fn minimum_de_la_facture(
    cnx: &mut gesco_db::Connexion,
    facture: &Facture,
    contexte: &Contexte,
) -> Resultat<(Montant, Option<&'static str>)> {
    let etats = etat_echeances(cnx, facture.id, contexte).await?;
    let somme = |f: fn(&crate::facturation::EcheanceAffichee) -> Montant| {
        calcul(etats.iter().try_fold(Montant::ZERO, |s, e| s.ajouter(f(e))))
    };
    let du_net = somme(|e| e.net)?;
    let paye_hors_reprise = calcul(somme(|e| e.paye)?.soustraire(somme(|e| e.reprise)?))?;
    let avoirs_nes = calcul(
        ajustements::de_la_facture(cnx, facture.id, contexte)
            .await?
            .iter()
            .filter(|a| !a.remplace)
            .try_fold(Montant::ZERO, |s, a| s.ajouter(a.avoir)),
    )?;
    let rembourse = reprises::rembourse(cnx, facture.inscription_id, contexte).await?;
    minimum(rembourse, du_net, avoirs_nes, paye_hors_reprise)
        .map_err(|_| regle("facture.calcul_impossible"))
}

/// Enregistre, remplace ou annule (montant zéro) la reprise d'une inscription dans une unité
/// ouverte : reprise, imputations, facture recalculée, audit et événement. Rend vrai si quelque
/// chose a changé. L'import et l'écran passent par ici.
pub(crate) async fn enregistrer_dans(
    unite: &mut UniteDeTravail<Effet>,
    contexte: &Contexte,
    inscription_id: Id<Inscription>,
    montant: Montant,
    note: &str,
    motif_zero: &str,
    origine: OrigineReprise,
) -> Resultat<bool> {
    if montant.est_negatif() {
        return Err(regle("reprise.montant_invalide"));
    }
    let note = verifier_note(note).map_err(regle)?;
    let (_, facture, annee) = cible(unite.connexion(), inscription_id, contexte).await?;
    if !annee_modifiable(&annee) {
        return Err(regle("reprise.annee_close"));
    }
    let active = reprises::active(unite.connexion(), inscription_id, contexte).await?;
    if montant.est_nul() {
        let Some(active) = active else {
            return Ok(false);
        };
        let motif = if motif_zero.trim().is_empty() {
            MOTIF_SAISIE_ERRONEE
        } else {
            motif_zero.trim()
        };
        annuler_dans(unite, contexte, &active, &facture, motif).await?;
        return Ok(true);
    }
    if let Some(a) = &active
        && a.montant == montant
        && a.note == note
    {
        return Ok(false);
    }
    let (plancher, regle_plancher) =
        minimum_de_la_facture(unite.connexion(), &facture, contexte).await?;
    if montant < plancher
        && let Some(cle) = regle_plancher
    {
        return Err(regle(cle));
    }
    let nouvelle = RepriseAvantGesco {
        id: nouvel_id(),
        etablissement_id: contexte.etablissement_id,
        inscription_id,
        montant,
        note,
        origine,
        statut: StatutReprise::Active,
        motif_annulation: String::new(),
        remplacee_par: None,
        cree_le: contexte.maintenant,
        cree_par: contexte.utilisateur_id,
        mis_a_jour_le: contexte.maintenant,
        mis_a_jour_par: contexte.utilisateur_id,
    };
    if let Some(ancienne) = &active {
        reprises::annuler(
            unite.connexion(),
            ancienne.id,
            MOTIF_REMPLACEE,
            Some(nouvelle.id),
            contexte,
        )
        .await?;
    }
    reprises::inserer(unite.connexion(), &nouvelle, contexte).await?;
    // Les parts de la nouvelle reprise naissent ici, comme pour tout geste qui change le payé.
    let apres = recalculer_dans(unite, contexte, &facture).await?;
    let chiffres = chiffres_reprise(unite.connexion(), &apres, contexte).await?;
    let id = nouvelle.id.vers_chaine();
    let (operation, evenement) = if active.is_some() {
        (
            Operation::Modification,
            TypeEvenement::RepriseAvantGescoModifiee,
        )
    } else {
        (
            Operation::Creation,
            TypeEvenement::RepriseAvantGescoEnregistree,
        )
    };
    unite.auditer(
        "reprise_avant_gesco",
        &id,
        operation,
        active.as_ref().map(serde_json::to_value).transpose()?,
        Some(serde_json::to_value(&nouvelle)?),
        "reprises.enregistrer",
    );
    unite.emettre(
        evenement,
        "reprise_avant_gesco",
        &id,
        serde_json::json!({
            "inscription_id": inscription_id,
            "montant": montant,
            "ancien_montant": active.as_ref().map(|a| a.montant),
            "a_rembourser": chiffres.a_rembourser,
            "origine": origine.nom(),
        }),
    );
    Ok(true)
}

/// Annule une reprise active et ses parts, recalcule la facture, audite et émet.
async fn annuler_dans(
    unite: &mut UniteDeTravail<Effet>,
    contexte: &Contexte,
    active: &RepriseAvantGesco,
    facture: &Facture,
    motif: &str,
) -> Resultat<()> {
    let (plancher, regle_plancher) =
        minimum_de_la_facture(unite.connexion(), facture, contexte).await?;
    if !plancher.est_nul()
        && let Some(cle) = regle_plancher
    {
        return Err(regle(cle));
    }
    reprises::annuler(unite.connexion(), active.id, motif, None, contexte).await?;
    recalculer_dans(unite, contexte, facture).await?;
    let id = active.id.vers_chaine();
    unite.auditer(
        "reprise_avant_gesco",
        &id,
        Operation::Modification,
        Some(serde_json::to_value(active)?),
        Some(serde_json::json!({ "statut": "annulee", "motif_annulation": motif })),
        "reprises.annuler",
    );
    unite.emettre(
        TypeEvenement::RepriseAvantGescoAnnulee,
        "reprise_avant_gesco",
        &id,
        serde_json::json!({
            "inscription_id": active.inscription_id,
            "montant": active.montant,
            "motif": motif,
        }),
    );
    Ok(())
}

/// Service de la reprise avant GESCO.
pub struct ServiceReprises<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceReprises<'n> {
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

    /// Lignes de l'écran « Payé avant GESCO » : inscriptions validées de l'année et de la classe,
    /// avec dû, payé avant et dans GESCO, reste et excédent ; date d'arrêt et totaux.
    pub async fn lister(
        &self,
        session: &Session,
        demande: &DemandeReprises,
    ) -> Resultat<ListeReprises> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let annee = match demande.annee_id {
            Some(id) => annee_scolaire::lire(&mut cnx, id, &contexte).await?,
            None => annee_scolaire::courante(&mut cnx, &contexte)
                .await?
                .ok_or(regle("eleve.annee_absente"))?,
        };
        let (date_arret, d_une_caisse) = arret_de_l_annee(&mut cnx, &annee, &contexte).await?;
        let classes: HashMap<Id<Classe>, String> =
            academique::classes(&mut cnx, annee.id, &contexte)
                .await?
                .into_iter()
                .map(|c| (c.id, c.libelle))
                .collect();
        let eleves: HashMap<Id<Eleve>, (String, String)> = personnes::eleves(&mut cnx, &contexte)
            .await?
            .into_iter()
            .map(|(e, p)| (e.id, (p.nom_complet(), e.matricule_interne)))
            .collect();
        let selection: Vec<Inscription> = inscriptions::de_l_annee(&mut cnx, annee.id, &contexte)
            .await?
            .into_iter()
            .filter(|i| demande.classe_id.is_none_or(|c| c == i.classe_id))
            .collect();
        let brouillons = selection
            .iter()
            .filter(|i| i.statut == StatutInscription::Brouillon)
            .count();
        let mut lignes = Vec::new();
        for inscription in selection
            .into_iter()
            .filter(|i| i.statut != StatutInscription::Brouillon)
        {
            let Some(facture) =
                factures::facture_vivante(&mut cnx, inscription.id, &contexte).await?
            else {
                continue;
            };
            lignes.push(
                self.ligne(
                    &mut cnx,
                    &contexte,
                    &inscription,
                    &facture,
                    &classes,
                    &eleves,
                )
                .await?,
            );
        }
        lignes.sort_by(|a, b| (&a.classe, &a.eleve).cmp(&(&b.classe, &b.eleve)));
        let somme = |f: fn(&LigneReprise) -> Montant| {
            calcul(
                lignes
                    .iter()
                    .try_fold(Montant::ZERO, |s, l| s.ajouter(f(l))),
            )
        };
        let compte = |f: fn(&LigneReprise) -> bool| {
            u32::try_from(lignes.iter().filter(|l| f(l)).count()).unwrap_or(u32::MAX)
        };
        Ok(ListeReprises {
            annee_id: annee.id,
            annee: annee.libelle.clone(),
            lecture_seule: !annee_modifiable(&annee),
            date_arret,
            a_ce_jour: !d_une_caisse,
            total: somme(|l| l.reprise)?,
            nombre: compte(|l| !l.reprise.est_nul()),
            total_a_rembourser: somme(|l| l.a_rembourser)?,
            nombre_a_rembourser: compte(|l| !l.a_rembourser.est_nul()),
            brouillons: u32::try_from(brouillons).unwrap_or(u32::MAX),
            lignes,
        })
    }

    async fn ligne(
        &self,
        cnx: &mut gesco_db::Connexion,
        contexte: &Contexte,
        inscription: &Inscription,
        facture: &Facture,
        classes: &HashMap<Id<Classe>, String>,
        eleves: &HashMap<Id<Eleve>, (String, String)>,
    ) -> Resultat<LigneReprise> {
        let etats = etat_echeances(cnx, facture.id, contexte).await?;
        let somme = |f: fn(&crate::facturation::EcheanceAffichee) -> Montant| {
            calcul(etats.iter().try_fold(Montant::ZERO, |s, e| s.ajouter(f(e))))
        };
        let chiffres = chiffres_reprise(cnx, facture, contexte).await?;
        let active = reprises::active(cnx, inscription.id, contexte).await?;
        let (plancher, _) = minimum_de_la_facture(cnx, facture, contexte).await?;
        let lue_le =
            gesco_db::depots::fraicheur::derniere(cnx, &lignes_fraicheur(inscription.id, facture))
                .await?
                .map_or(0, |d| d.le);
        let (eleve, matricule) = eleves
            .get(&inscription.eleve_id)
            .cloned()
            .unwrap_or_default();
        Ok(LigneReprise {
            inscription_id: inscription.id,
            eleve_id: inscription.eleve_id,
            classe: classes
                .get(&inscription.classe_id)
                .cloned()
                .unwrap_or_default(),
            eleve,
            matricule,
            du: somme(|e| e.net)?,
            reprise: chiffres.montant,
            note: active.map(|a| a.note).unwrap_or_default(),
            paye_gesco: calcul(somme(|e| e.paye)?.soustraire(somme(|e| e.reprise)?))?,
            reste: somme(|e| e.reste)?,
            a_rembourser: chiffres.a_rembourser,
            rembourse: chiffres.rembourse,
            minimum: plancher,
            lue_le,
        })
    }

    /// Ligne d'une inscription, telle que l'écran la montre après un geste.
    async fn ligne_de(
        &self,
        session: &Session,
        inscription_id: Id<Inscription>,
    ) -> Resultat<LigneReprise> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let (inscription, facture, _) = cible(&mut cnx, inscription_id, &contexte).await?;
        let classe = academique::classe(&mut cnx, inscription.classe_id, &contexte).await?;
        let classes = HashMap::from([(classe.id, classe.libelle)]);
        let eleves: HashMap<Id<Eleve>, (String, String)> = personnes::eleves(&mut cnx, &contexte)
            .await?
            .into_iter()
            .filter(|(e, _)| e.id == inscription.eleve_id)
            .map(|(e, p)| (e.id, (p.nom_complet(), e.matricule_interne)))
            .collect();
        self.ligne(
            &mut cnx,
            &contexte,
            &inscription,
            &facture,
            &classes,
            &eleves,
        )
        .await
    }

    /// Enregistre ou remplace la reprise d'une inscription ; un montant zéro l'annule (§3.4).
    /// Rend la ligne à jour.
    pub async fn enregistrer(
        &self,
        session: &Session,
        demande: &DemandeReprise,
    ) -> Resultat<LigneReprise> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        if let Some(lue_le) = demande.lue_le {
            let (_, facture, _) =
                cible(unite.connexion(), demande.inscription_id, &contexte).await?;
            crate::fraicheur::verifier(
                unite.connexion(),
                &lignes_fraicheur(demande.inscription_id, &facture),
                lue_le,
            )
            .await?;
        }
        enregistrer_dans(
            &mut unite,
            &contexte,
            demande.inscription_id,
            demande.montant,
            &demande.note,
            &demande.motif,
            OrigineReprise::Ecran,
        )
        .await?;
        self.noyau.terminer(unite).await?;
        self.ligne_de(session, demande.inscription_id).await
    }

    /// Annule la reprise active d'une inscription, avec un motif (§3.4).
    pub async fn annuler(
        &self,
        session: &Session,
        demande: &DemandeAnnulationReprise,
    ) -> Resultat<LigneReprise> {
        if demande.motif.trim().is_empty() {
            return Err(regle("reprise.motif_obligatoire"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let (_, facture, annee) =
            cible(unite.connexion(), demande.inscription_id, &contexte).await?;
        if !annee_modifiable(&annee) {
            return Err(regle("reprise.annee_close"));
        }
        if let Some(lue_le) = demande.lue_le {
            crate::fraicheur::verifier(
                unite.connexion(),
                &lignes_fraicheur(demande.inscription_id, &facture),
                lue_le,
            )
            .await?;
        }
        let active = reprises::active(unite.connexion(), demande.inscription_id, &contexte)
            .await?
            .ok_or(regle("reprise.absente"))?;
        annuler_dans(
            &mut unite,
            &contexte,
            &active,
            &facture,
            demande.motif.trim(),
        )
        .await?;
        self.noyau.terminer(unite).await?;
        self.ligne_de(session, demande.inscription_id).await
    }

    /// Reprises d'une inscription, actives et annulées, la plus récente d'abord.
    pub async fn historique(
        &self,
        session: &Session,
        inscription_id: Id<Inscription>,
    ) -> Resultat<Vec<RepriseAvantGesco>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        Ok(reprises::historique(&mut cnx, inscription_id, &contexte).await?)
    }

    /// Payé avant GESCO d'une inscription : reprise active, chiffres, remboursements, date
    /// d'arrêt. `None` sans facture vivante.
    pub async fn de_l_inscription(
        &self,
        session: &Session,
        inscription_id: Id<Inscription>,
    ) -> Resultat<Option<RepriseDeLInscription>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        de_l_inscription_dans(&mut cnx, inscription_id, &contexte).await
    }
}

/// Payé avant GESCO d'une inscription, lu sur une connexion ouverte.
pub(crate) async fn de_l_inscription_dans(
    cnx: &mut gesco_db::Connexion,
    inscription_id: Id<Inscription>,
    contexte: &Contexte,
) -> Resultat<Option<RepriseDeLInscription>> {
    let inscription = inscriptions::lire(cnx, inscription_id, contexte).await?;
    let Some(facture) = factures::facture_vivante(cnx, inscription_id, contexte).await? else {
        return Ok(None);
    };
    let annee = annee_scolaire::lire(cnx, inscription.annee_id, contexte).await?;
    let (date_arret, d_une_caisse) = arret_de_l_annee(cnx, &annee, contexte).await?;
    let chiffres = chiffres_reprise(cnx, &facture, contexte).await?;
    let active = reprises::active(cnx, inscription_id, contexte).await?;
    let remboursements = reprises::remboursements(cnx, inscription_id, contexte)
        .await?
        .into_iter()
        .map(|(date, montant, statut, mode)| Remboursement {
            date,
            montant,
            statut,
            mode,
        })
        .collect();
    Ok(Some(RepriseDeLInscription {
        active,
        chiffres,
        remboursements,
        date_arret,
        a_ce_jour: !d_une_caisse,
    }))
}
