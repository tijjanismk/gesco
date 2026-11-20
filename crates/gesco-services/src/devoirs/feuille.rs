//! Feuille du devoir (SPEC_PHASE_12_LOT_5 §3) : un PDF A4 pour les élèves sans appareil, avec
//! l'en-tête de l'école, la classe, la matière, le titre, la date limite, le barème et les
//! consignes. Il passe par le modèle des leçons : le texte y est échappé, une formule que la
//! conversion ne prend pas s'imprime telle qu'elle est écrite, et l'impression ne se refuse pas.

use base64::Engine as _;
use gesco_core::bibliotheque::analyser;
use gesco_core::commun::Horodatage;

use super::{EtatDevoir, FormeRendu, ServiceDevoirs};
use crate::bibliotheque::Impression;
use crate::erreur::Resultat;
use crate::session::Session;

/// Date et heure limites en toutes lettres : « 12 mars 2026 à 18 h », « 1er avril 2026 à
/// 10 h 30 ».
pub fn date_limite_longue(millis: i64) -> String {
    let instant = Horodatage::depuis_millis_epoque(millis);
    let civil = instant.civil_utc();
    let heure = if civil.minute == 0 {
        format!("{} h", civil.heure)
    } else {
        format!("{} h {:02}", civil.heure, civil.minute)
    };
    format!("{} à {heure}", crate::documents::date_longue(instant))
}

/// Barème lisible : « sur 20 », « sur 12,5 ».
pub(crate) fn sur(bareme: i64) -> String {
    let entier = bareme / 100;
    let reste = bareme % 100;
    if reste == 0 {
        format!("sur {entier}")
    } else if reste % 10 == 0 {
        format!("sur {entier},{}", reste / 10)
    } else {
        format!("sur {entier},{reste:02}")
    }
}

impl ServiceDevoirs<'_> {
    /// Ce qu'on imprime d'un devoir de sa portée. Un brouillon s'imprime aussi (le maître le
    /// relit avant de le donner).
    pub(crate) async fn feuille(&self, session: &Session, id: &str) -> Resultat<Impression> {
        let (devoir, _) = self.devoir_permis(session, id).await?;
        let affiche = self.lire(session, id).await?;
        let mut sous_titre = format!(
            "À rendre le {} · {}",
            date_limite_longue(devoir.date_limite),
            sur(devoir.bareme)
        );
        if FormeRendu::depuis_nom(&devoir.forme_rendu) == FormeRendu::Papier {
            sous_titre.push_str(" · Sur papier");
        }
        if EtatDevoir::depuis_nom(&devoir.etat) == EtatDevoir::Brouillon {
            sous_titre.push_str(" · Brouillon");
        }
        let lecon = analyser(&devoir.consignes);
        let images = affiche
            .images
            .into_iter()
            .filter_map(|i| {
                let octets = base64::engine::general_purpose::STANDARD
                    .decode(&i.donnees)
                    .ok()?;
                Some((i.cle, i.type_mime, octets))
            })
            .collect();
        Ok(Impression::feuille_de_devoir(
            &devoir.titre,
            &sous_titre,
            (&affiche.classe, &affiche.matiere),
            &lecon,
            images,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_date_limite_et_le_bareme_se_disent_en_mots() {
        // 12 mars 2026, 18 h 00 UTC (heure du Mali).
        let millis = 1_773_338_400_000;
        assert_eq!(date_limite_longue(millis), "12 mars 2026 à 18 h");
        assert_eq!(
            date_limite_longue(millis + 30 * 60_000),
            "12 mars 2026 à 18 h 30"
        );
        assert_eq!(sur(2_000), "sur 20");
        assert_eq!(sur(1_250), "sur 12,5");
        assert_eq!(sur(1_025), "sur 10,25");
    }
}
