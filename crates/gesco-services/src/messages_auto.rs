//! Messages qui partent d'eux-mêmes (SPEC_PHASE_5 §5, T-75) : bulletin à retirer, relance
//! d'impayé, absence du jour ; et l'avis de sanction, préparé si l'école le veut, qui attend
//! toujours la décision d'une personne. Chaque message naît dans la transaction du fait qui le
//! cause (publication, relevé, sanction), ou au passage quotidien de la file pour les relances.
//! Rien ne se met en file tant que l'école n'a pas choisi de canal.

use std::collections::HashSet;

use gesco_core::commun::{DateCivile, Horodatage, Id};
use gesco_core::core::StatutAnnee;
use gesco_core::finance::recouvrement::impaye;
use gesco_core::notification::{
    EtatMessage, Message, NatureMessage, TypeMessage, modele, numero_destinataire, recoit,
};
use gesco_core::pedagogie::assiduite::{DemiJournee, NatureAbsence};
use gesco_core::pedagogie::bulletin::Bulletin;
use gesco_core::scolarite::discipline::Sanction;
use gesco_core::scolarite::{Eleve, Inscription, Qualites, Responsable, StatutInscription};
use gesco_db::depots::{
    absences, academique, annee_scolaire, factures, file_sms, inscriptions, messages, parametre,
    pedagogie, personnes,
};
use gesco_db::{Connexion, Contexte};

use crate::erreur::Resultat;
use crate::file_sms::{MessageAFaire, canal_en_vigueur, mettre_en_file, modele_de, signature};

const JOUR_MS: i64 = 86_400_000;

/// Vrai si l'école a activé les messages aux familles et choisi un canal d'envoi.
async fn canal_actif(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<bool> {
    Ok(canal_en_vigueur(cnx, contexte).await? != "aucun")
}

/// Vrai si un message automatique est actif : un canal est choisi et son réglage est coché.
async fn actif(
    cnx: &mut Connexion,
    cle: &str,
    defaut: bool,
    contexte: &Contexte,
) -> Resultat<bool> {
    Ok(canal_actif(cnx, contexte).await?
        && parametre::booleen_param(cnx, cle, defaut, contexte).await?)
}

/// Date écrite comme dans un SMS : `14/10/2026`.
fn date_courte(d: DateCivile) -> String {
    format!("{:02}/{:02}/{}", d.jour(), d.mois(), d.annee())
}

/// Minuit au début d'un jour.
fn debut_du_jour(d: DateCivile) -> Horodatage {
    Horodatage::depuis_millis_epoque(d.jours_depuis_epoque() * JOUR_MS)
}

/// Responsables d'un élève qui reçoivent ce type de message, avec le numéro où l'envoyer : la
/// case « reçoit les SMS » de leur lien, le filtre propre au message (payeur, responsable
/// légal), leur consentement, puis le premier de leurs numéros qui peut recevoir.
async fn destinataires(
    cnx: &mut Connexion,
    eleve_id: Id<Eleve>,
    type_message: TypeMessage,
    garder: impl Fn(&Qualites) -> bool,
    contexte: &Contexte,
) -> Resultat<Vec<(Id<Responsable>, String)>> {
    destinataires_parmi(cnx, eleve_id, type_message, |_, q| garder(q), contexte).await
}

/// Comme `destinataires`, le filtre voyant aussi le responsable.
async fn destinataires_parmi(
    cnx: &mut Connexion,
    eleve_id: Id<Eleve>,
    type_message: TypeMessage,
    garder: impl Fn(Id<Responsable>, &Qualites) -> bool,
    contexte: &Contexte,
) -> Resultat<Vec<(Id<Responsable>, String)>> {
    let mut trouves: Vec<(Id<Responsable>, String)> = Vec::new();
    for (_, responsable_id, qualites) in
        personnes::liens_de_l_eleve(cnx, eleve_id, contexte).await?
    {
        if !garder(responsable_id, &qualites) {
            continue;
        }
        let consentements = messages::consentements(cnx, responsable_id, contexte).await?;
        if !recoit(
            type_message,
            qualites.recoit_notifications,
            responsable_id,
            &consentements,
        ) {
            continue;
        }
        let (_, personne) = personnes::responsable(cnx, responsable_id, contexte).await?;
        let joignabilites = messages::joignabilites(cnx, &personne.telephones, contexte).await?;
        if let Some(numero) = numero_destinataire(&personne.telephones, &joignabilites)
            && !trouves.iter().any(|(_, n)| n == numero)
        {
            trouves.push((responsable_id, numero.clone()));
        }
    }
    Ok(trouves)
}

/// Élève et classe d'une inscription : prénoms et nom, libellé de la classe.
async fn eleve_et_classe(
    cnx: &mut Connexion,
    inscription: &Inscription,
    contexte: &Contexte,
) -> Resultat<(String, String)> {
    let (_, personne) = personnes::eleve(cnx, inscription.eleve_id, contexte).await?;
    let classe = academique::classe(cnx, inscription.classe_id, contexte).await?;
    Ok((personne.nom_complet(), classe.libelle))
}

/// Met en file un même texte pour chaque destinataire ; rend le nombre de messages nouveaux.
#[allow(clippy::too_many_arguments)]
async fn pour_chacun(
    cnx: &mut Connexion,
    nature: NatureMessage,
    cle_fait: &str,
    eleve_id: Id<Eleve>,
    destinataires: Vec<(Id<Responsable>, String)>,
    texte: &str,
    attente_ms: i64,
    expire_le: Option<Horodatage>,
    contexte: &Contexte,
) -> Resultat<u32> {
    let mut nouveaux = 0;
    for (responsable_id, numero) in destinataires {
        let mis = mettre_en_file(
            cnx,
            MessageAFaire {
                nature,
                cle_fait: cle_fait.to_owned(),
                responsable_id: Some(responsable_id),
                eleve_id: Some(eleve_id),
                numero,
                texte: texte.to_owned(),
                attente_ms,
                expire_le,
            },
            contexte,
        )
        .await?;
        if mis.is_some() {
            nouveaux += 1;
        }
    }
    Ok(nouveaux)
}

/// Avis « bulletin à retirer » à la publication d'un bulletin, une fois par élève et par période :
/// un bulletin corrigé puis republié ne renvoie pas d'avis.
pub(crate) async fn avis_bulletin(
    cnx: &mut Connexion,
    bulletin: &Bulletin,
    contexte: &Contexte,
) -> Resultat<u32> {
    if !actif(cnx, "sms.auto_bulletin", true, contexte).await? {
        return Ok(0);
    }
    let inscription = inscriptions::lire(cnx, bulletin.inscription_id, contexte).await?;
    let (eleve, classe) = eleve_et_classe(cnx, &inscription, contexte).await?;
    let periode = pedagogie::periodes(cnx, inscription.annee_id, contexte)
        .await?
        .into_iter()
        .find(|p| p.id == bulletin.periode_id)
        .map(|p| p.libelle())
        .unwrap_or_default();
    let ecole = signature(cnx, contexte).await?;
    let texte = modele::formuler(
        &modele_de(cnx, NatureMessage::Bulletin, contexte).await?,
        &[
            ("ecole", &ecole),
            ("eleve", &eleve),
            ("classe", &classe),
            ("periode", &periode),
        ],
    );
    let pour = destinataires(
        cnx,
        inscription.eleve_id,
        TypeMessage::Bulletin,
        |_| true,
        contexte,
    )
    .await?;
    pour_chacun(
        cnx,
        NatureMessage::Bulletin,
        &format!("bulletin:{}:{}", inscription.id, bulletin.periode_id),
        inscription.eleve_id,
        pour,
        &texte,
        0,
        None,
        contexte,
    )
    .await
}

/// Avis d'absence d'un élève pour un jour, recalculé après chaque relevé ou justification : un
/// avis par jour, même pour deux demi-journées ; il attend le délai de grâce avant de partir ;
/// une absence corrigée ou justifiée pendant ce délai l'annule, une seconde demi-journée
/// relevée avant son départ complète son texte. Il expire à la fin du jour.
pub(crate) async fn actualiser_avis_absence(
    cnx: &mut Connexion,
    inscription_id: Id<Inscription>,
    date: DateCivile,
    contexte: &Contexte,
) -> Resultat<()> {
    let cle_fait = format!("absence:{inscription_id}:{date}");
    let mut absent = Vec::new();
    for demi_journee in DemiJournee::TOUTES {
        if absences::lire(cnx, inscription_id, date, demi_journee, contexte)
            .await?
            .is_some_and(|a| a.nature == NatureAbsence::Absence && !a.justifiee)
        {
            absent.push(demi_journee);
        }
    }
    let en_attente: Vec<Message> = file_sms::par_fait(cnx, &cle_fait, contexte)
        .await?
        .into_iter()
        .filter(|m| m.etat == EtatMessage::EnAttente)
        .collect();
    if absent.is_empty() {
        for mut m in en_attente {
            m.etat = EtatMessage::Annule;
            m.motif_annulation = "Absence corrigée ou justifiée avant l'envoi".to_owned();
            file_sms::mettre_a_jour(cnx, &m, contexte).await?;
        }
        return Ok(());
    }
    if !actif(cnx, "sms.auto_absence", true, contexte).await? {
        return Ok(());
    }
    let inscription = inscriptions::lire(cnx, inscription_id, contexte).await?;
    let (eleve, classe) = eleve_et_classe(cnx, &inscription, contexte).await?;
    let demi_journees = match absent.as_slice() {
        [DemiJournee::Matin] => "matinee",
        [DemiJournee::ApresMidi] => "apres-midi",
        _ => "matinee et apres-midi",
    };
    let ecole = signature(cnx, contexte).await?;
    let texte = modele::formuler(
        &modele_de(cnx, NatureMessage::Absence, contexte).await?,
        &[
            ("ecole", &ecole),
            ("eleve", &eleve),
            ("classe", &classe),
            ("date", &date_courte(date)),
            ("demi_journee", demi_journees),
        ],
    );
    let deja = file_sms::par_fait(cnx, &cle_fait, contexte).await?;
    if !deja.is_empty() {
        // Un avis existe déjà pour ce jour : on complète son texte s'il n'est pas parti.
        for mut m in en_attente {
            if m.texte != texte
                && let Ok(n) = u8::try_from(gesco_core::notification::gsm7::nombre_sms(&texte))
                && (1..=3).contains(&n)
            {
                m.texte = texte.clone();
                m.sms = n;
                file_sms::mettre_a_jour(cnx, &m, contexte).await?;
            }
        }
        return Ok(());
    }
    let grace = parametre::entier(cnx, "sms.absence_delai_grace_min", 30, contexte).await?;
    let fin_du_jour = date.decaler(1).map(debut_du_jour);
    let pour = destinataires(
        cnx,
        inscription.eleve_id,
        TypeMessage::Absence,
        |_| true,
        contexte,
    )
    .await?;
    pour_chacun(
        cnx,
        NatureMessage::Absence,
        &cle_fait,
        inscription.eleve_id,
        pour,
        &texte,
        grace.max(0) * 60_000,
        fin_du_jour,
        contexte,
    )
    .await?;
    Ok(())
}

/// Avis de sanction, préparé à la saisie si l'école le veut : il naît « à décider » et ne part
/// qu'après la relecture d'une personne. Il va aux responsables légaux.
pub(crate) async fn preparer_avis_sanction(
    cnx: &mut Connexion,
    sanction: &Sanction,
    contexte: &Contexte,
) -> Resultat<u32> {
    if !actif(cnx, "sms.sanction_preparer", false, contexte).await? {
        return Ok(0);
    }
    let inscription = inscriptions::lire(cnx, sanction.inscription_id, contexte).await?;
    let (eleve, classe) = eleve_et_classe(cnx, &inscription, contexte).await?;
    let ecole = signature(cnx, contexte).await?;
    let texte = modele::formuler(
        &modele_de(cnx, NatureMessage::Sanction, contexte).await?,
        &[
            ("ecole", &ecole),
            ("eleve", &eleve),
            ("classe", &classe),
            ("sanction", sanction.nature.libelle()),
            ("date", &date_courte(sanction.date)),
        ],
    );
    let pour = destinataires(
        cnx,
        inscription.eleve_id,
        TypeMessage::Convocation,
        |q| q.est_responsable_legal,
        contexte,
    )
    .await?;
    pour_chacun(
        cnx,
        NatureMessage::Sanction,
        &format!("sanction:{}", sanction.id),
        inscription.eleve_id,
        pour,
        &texte,
        0,
        None,
        contexte,
    )
    .await
}

/// Clé de fait d'une relance : la facture, la plus ancienne échéance non soldée, le rang.
fn prefixe_relance(facture: &str, plus_ancienne: DateCivile) -> String {
    format!("impaye:{facture}:{plus_ancienne}:")
}

/// Relances d'impayés du jour, au premier passage de la file : une échéance non soldée depuis le
/// délai réglé relance le payeur pour tout ce que l'élève doit d'échu ; une nouvelle relance
/// après l'intervalle, jamais plus que le nombre réglé pour une même échéance ; rien sous le
/// montant minimal, rien pour un élève qui n'est plus inscrit. Rend le nombre de messages.
pub(crate) async fn relances_du_jour(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<u32> {
    if !actif(cnx, "sms.auto_impaye", true, contexte).await? {
        return Ok(0);
    }
    let Some(aujourd_hui) = DateCivile::du_jour(contexte.maintenant) else {
        return Ok(0);
    };
    let Some(courante) = annee_scolaire::courante(cnx, contexte).await? else {
        return Ok(0);
    };
    let delai = parametre::entier(cnx, "sms.impaye_delai_jours", 7, contexte).await?;
    let intervalle = parametre::entier(cnx, "sms.impaye_intervalle_jours", 15, contexte).await?;
    let nombre_max = parametre::entier(cnx, "sms.impaye_nombre_max", 3, contexte).await?;
    let montant_min = parametre::entier(cnx, "sms.impaye_montant_min", 0, contexte).await?;
    let inscrits: HashSet<Id<Eleve>> = inscriptions::de_l_annee(cnx, courante.id, contexte)
        .await?
        .into_iter()
        .filter(|i| i.statut == StatutInscription::Validee)
        .map(|i| i.eleve_id)
        .collect();
    let mut annees = vec![courante.id];
    annees.extend(
        annee_scolaire::lister(cnx, contexte)
            .await?
            .into_iter()
            .filter(|a| {
                a.id != courante.id
                    && matches!(a.statut, StatutAnnee::Cloturee | StatutAnnee::Archivee)
            })
            .map(|a| a.id),
    );
    let ecole = signature(cnx, contexte).await?;
    let modele_impaye = modele_de(cnx, NatureMessage::Impaye, contexte).await?;
    let mut nouveaux = 0;
    for annee_id in annees {
        for facture in factures::factures_ouvertes(cnx, annee_id, contexte).await? {
            let echeances: Vec<_> = crate::facturation::etat_echeances(cnx, facture.id, contexte)
                .await?
                .iter()
                .map(|e| (e.echeance.date, e.reste))
                .collect();
            let Ok(Some(du)) = impaye(&echeances, aujourd_hui) else {
                continue;
            };
            if du.jours < delai || du.echu.francs() < montant_min {
                continue;
            }
            let inscription = inscriptions::lire(cnx, facture.inscription_id, contexte).await?;
            if !inscrits.contains(&inscription.eleve_id) {
                continue;
            }
            let prefixe = prefixe_relance(&facture.id.vers_chaine(), du.plus_ancienne);
            let faites: Vec<Message> = file_sms::par_prefixe(cnx, &prefixe, contexte)
                .await?
                .into_iter()
                .filter(|m| m.etat != EtatMessage::Annule)
                .collect();
            let rangs: HashSet<&str> = faites.iter().map(|m| m.cle_fait.as_str()).collect();
            let rang = i64::try_from(rangs.len()).unwrap_or(i64::MAX);
            if rang >= nombre_max {
                continue;
            }
            if faites.iter().any(|m| {
                contexte.maintenant.millis_epoque() - m.cree_le.millis_epoque()
                    < intervalle * JOUR_MS
            }) {
                continue;
            }
            let (eleve, classe) = eleve_et_classe(cnx, &inscription, contexte).await?;
            let texte = modele::formuler(
                &modele_impaye,
                &[
                    ("ecole", &ecole),
                    ("eleve", &eleve),
                    ("classe", &classe),
                    ("montant", &du.echu.formater()),
                    ("date", &date_courte(du.plus_ancienne)),
                ],
            );
            let pour = destinataires(
                cnx,
                inscription.eleve_id,
                TypeMessage::Impaye,
                |q| q.est_payeur,
                contexte,
            )
            .await?;
            nouveaux += pour_chacun(
                cnx,
                NatureMessage::Impaye,
                &format!("{prefixe}{}", rang + 1),
                inscription.eleve_id,
                pour,
                &texte,
                0,
                None,
                contexte,
            )
            .await?;
        }
    }
    Ok(nouveaux)
}

/// Vrai si une relance mise en file est encore due : sa facture garde la même plus ancienne
/// échéance non soldée. Un versement qui a soldé l'échéance entre-temps rend la relance caduque.
pub(crate) async fn relance_toujours_due(
    cnx: &mut Connexion,
    message: &Message,
    contexte: &Contexte,
) -> Resultat<bool> {
    let mut morceaux = message.cle_fait.split(':');
    let (Some("impaye"), Some(facture), Some(date)) =
        (morceaux.next(), morceaux.next(), morceaux.next())
    else {
        return Ok(true);
    };
    let (Ok(facture_id), Some(date)) = (Id::analyser(facture), DateCivile::analyser(date)) else {
        return Ok(true);
    };
    let Some(aujourd_hui) = DateCivile::du_jour(contexte.maintenant) else {
        return Ok(true);
    };
    let echeances: Vec<_> = crate::facturation::etat_echeances(cnx, facture_id, contexte)
        .await?
        .iter()
        .map(|e| (e.echeance.date, e.reste))
        .collect();
    Ok(matches!(impaye(&echeances, aujourd_hui), Ok(Some(du)) if du.plus_ancienne == date))
}

/// SMS d'une convocation, préparé quand l'école choisit ce canal : il naît « à décider ». Il va
/// au responsable convoqué ; à défaut, aux responsables légaux.
pub(crate) async fn preparer_convocation(
    cnx: &mut Connexion,
    convocation: &gesco_core::scolarite::convocation::Convocation,
    contexte: &Contexte,
) -> Resultat<u32> {
    let inscription = inscriptions::lire(cnx, convocation.inscription_id, contexte).await?;
    let (eleve, _) = eleve_et_classe(cnx, &inscription, contexte).await?;
    let ecole = signature(cnx, contexte).await?;
    let texte = modele::formuler(
        &modele_de(cnx, NatureMessage::Convocation, contexte).await?,
        &[
            ("ecole", &ecole),
            ("eleve", &eleve),
            ("date", &date_courte(convocation.date)),
            (
                "heure",
                &gesco_core::scolarite::convocation::heure_lisible(&convocation.heure),
            ),
            ("motif", &convocation.motif),
        ],
    );
    let convoque = convocation.responsable_id;
    let pour = destinataires_parmi(
        cnx,
        inscription.eleve_id,
        TypeMessage::Convocation,
        |id, q| convoque.map_or(q.est_responsable_legal, |c| c == id),
        contexte,
    )
    .await?;
    pour_chacun(
        cnx,
        NatureMessage::Convocation,
        &format!("convocation:{}", convocation.id),
        inscription.eleve_id,
        pour,
        &texte,
        0,
        None,
        contexte,
    )
    .await
}

/// Rappel « bulletin à retirer » envoyé à la main depuis la liste des bulletins non retirés : un
/// par jour au plus.
pub(crate) async fn rappel_bulletin(
    cnx: &mut Connexion,
    remise: &gesco_core::scolarite::remise_document::RemiseDocument,
    contexte: &Contexte,
) -> Resultat<u32> {
    if !canal_actif(cnx, contexte).await? {
        return Err(crate::erreur::ErreurService::Regle {
            cle: "sms.canal_aucun",
        });
    }
    let Some(aujourd_hui) = DateCivile::du_jour(contexte.maintenant) else {
        return Ok(0);
    };
    let inscription = inscriptions::lire(cnx, remise.inscription_id, contexte).await?;
    let (eleve, classe) = eleve_et_classe(cnx, &inscription, contexte).await?;
    let periode = pedagogie::periodes(cnx, inscription.annee_id, contexte)
        .await?
        .into_iter()
        .find(|p| p.id == remise.periode_id)
        .map(|p| p.libelle())
        .unwrap_or_default();
    let ecole = signature(cnx, contexte).await?;
    let texte = modele::formuler(
        &modele_de(cnx, NatureMessage::Bulletin, contexte).await?,
        &[
            ("ecole", &ecole),
            ("eleve", &eleve),
            ("classe", &classe),
            ("periode", &periode),
        ],
    );
    let pour = destinataires(
        cnx,
        inscription.eleve_id,
        TypeMessage::Bulletin,
        |_| true,
        contexte,
    )
    .await?;
    pour_chacun(
        cnx,
        NatureMessage::Bulletin,
        &format!(
            "bulletin:{}:{}:rappel:{aujourd_hui}",
            inscription.id, remise.periode_id
        ),
        inscription.eleve_id,
        pour,
        &texte,
        0,
        None,
        contexte,
    )
    .await
}
