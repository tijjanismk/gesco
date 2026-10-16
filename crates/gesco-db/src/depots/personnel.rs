//! Registre du personnel (SPEC_PHASE_6 §2) : catégories, fonctions, membres, pièces du dossier.

use gesco_core::commun::{DateCivile, Id};
use gesco_core::core::Utilisateur;
use gesco_core::rh::{
    CategoriePersonnel, Engagement, FonctionPersonnel, HeuresVacation, MembrePersonnel, Mois,
    NatureEngagement, NaturePiece, PieceDossier, SituationFamille, StatutEngagement, StatutHeures,
};
use gesco_core::scolarite::Personne;
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::depots::booleen;
use crate::erreur::{ErreurBase, Resultat};
use crate::id::lire_id;

fn cloisonne<T>(entite: &'static str, id: Id<T>, etablissement: bool) -> Resultat<()> {
    if etablissement {
        Ok(())
    } else {
        Err(ErreurBase::Cloisonnement {
            entite,
            id: id.vers_chaine(),
        })
    }
}

#[derive(FromRow)]
struct LigneCategorie {
    id: String,
    etablissement_id: String,
    code: Option<String>,
    libelle: String,
    ordre: i64,
    active: i64,
}

impl LigneCategorie {
    fn vers_entite(self) -> Resultat<CategoriePersonnel> {
        let t = "categorie_personnel";
        Ok(CategoriePersonnel {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            code: self.code,
            libelle: self.libelle,
            ordre: self.ordre,
            active: booleen(self.active),
        })
    }
}

/// Catégories de l'établissement, dans l'ordre.
pub async fn categories(
    cnx: &mut Connexion,
    contexte: &Contexte,
) -> Resultat<Vec<CategoriePersonnel>> {
    sqlx::query_as::<_, LigneCategorie>(
        "SELECT id, etablissement_id, code, libelle, ordre, active FROM categorie_personnel
         WHERE etablissement_id = ?1 ORDER BY ordre, libelle",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneCategorie::vers_entite)
    .collect()
}

/// Enregistre une catégorie.
pub async fn inserer_categorie(
    cnx: &mut Connexion,
    c: &CategoriePersonnel,
    contexte: &Contexte,
) -> Resultat<()> {
    cloisonne(
        "categorie_personnel",
        c.id,
        c.etablissement_id == contexte.etablissement_id,
    )?;
    sqlx::query(
        "INSERT INTO categorie_personnel (id, etablissement_id, code, libelle, ordre, active,
            cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?7, ?8, ?9)",
    )
    .bind(c.id.vers_chaine())
    .bind(c.etablissement_id.vers_chaine())
    .bind(c.code.as_deref())
    .bind(&c.libelle)
    .bind(c.ordre)
    .bind(i64::from(c.active))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Récrit le libellé, l'ordre et l'état d'une catégorie.
pub async fn ecrire_categorie(
    cnx: &mut Connexion,
    c: &CategoriePersonnel,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE categorie_personnel SET libelle = ?1, ordre = ?2, active = ?3,
            mis_a_jour_le = ?4, mis_a_jour_par = ?5, identifiant_poste = ?6
         WHERE id = ?7 AND etablissement_id = ?8",
    )
    .bind(&c.libelle)
    .bind(c.ordre)
    .bind(i64::from(c.active))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(c.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

#[derive(FromRow)]
struct LigneFonction {
    id: String,
    etablissement_id: String,
    categorie_id: String,
    code: Option<String>,
    libelle: String,
    ordre: i64,
    active: i64,
}

impl LigneFonction {
    fn vers_entite(self) -> Resultat<FonctionPersonnel> {
        let t = "fonction_personnel";
        Ok(FonctionPersonnel {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            categorie_id: lire_id(t, "categorie_id", &self.categorie_id)?,
            code: self.code,
            libelle: self.libelle,
            ordre: self.ordre,
            active: booleen(self.active),
        })
    }
}

/// Fonctions de l'établissement, dans l'ordre.
pub async fn fonctions(
    cnx: &mut Connexion,
    contexte: &Contexte,
) -> Resultat<Vec<FonctionPersonnel>> {
    sqlx::query_as::<_, LigneFonction>(
        "SELECT id, etablissement_id, categorie_id, code, libelle, ordre, active
         FROM fonction_personnel WHERE etablissement_id = ?1 ORDER BY ordre, libelle",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneFonction::vers_entite)
    .collect()
}

/// Enregistre une fonction.
pub async fn inserer_fonction(
    cnx: &mut Connexion,
    f: &FonctionPersonnel,
    contexte: &Contexte,
) -> Resultat<()> {
    cloisonne(
        "fonction_personnel",
        f.id,
        f.etablissement_id == contexte.etablissement_id,
    )?;
    sqlx::query(
        "INSERT INTO fonction_personnel (id, etablissement_id, categorie_id, code, libelle, ordre,
            active, cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?8, ?9, ?10)",
    )
    .bind(f.id.vers_chaine())
    .bind(f.etablissement_id.vers_chaine())
    .bind(f.categorie_id.vers_chaine())
    .bind(f.code.as_deref())
    .bind(&f.libelle)
    .bind(f.ordre)
    .bind(i64::from(f.active))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Récrit la catégorie, le libellé, l'ordre et l'état d'une fonction.
pub async fn ecrire_fonction(
    cnx: &mut Connexion,
    f: &FonctionPersonnel,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE fonction_personnel SET categorie_id = ?1, libelle = ?2, ordre = ?3, active = ?4,
            mis_a_jour_le = ?5, mis_a_jour_par = ?6, identifiant_poste = ?7
         WHERE id = ?8 AND etablissement_id = ?9",
    )
    .bind(f.categorie_id.vers_chaine())
    .bind(&f.libelle)
    .bind(f.ordre)
    .bind(i64::from(f.active))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(f.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

#[derive(FromRow)]
struct LigneMembre {
    id: String,
    etablissement_id: String,
    personne_id: String,
    matricule: String,
    categorie_id: String,
    fonction: String,
    arrive_le: Option<String>,
    parti_le: Option<String>,
    motif_depart: String,
    fonctionnaire: i64,
    numero_fonctionnaire: String,
    numero_inps: String,
    prevenir_nom: String,
    prevenir_telephone: String,
    paiement: String,
    diplomes: String,
    utilisateur_id: Option<String>,
    situation_famille: Option<String>,
    enfants_a_charge: Option<i64>,
}

impl LigneMembre {
    fn vers_entite(self) -> Resultat<MembrePersonnel> {
        let t = "membre_personnel";
        let date = |colonne: &'static str, texte: &str| {
            DateCivile::analyser(texte).ok_or_else(|| ErreurBase::illisible(t, colonne, texte))
        };
        Ok(MembrePersonnel {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            personne_id: lire_id(t, "personne_id", &self.personne_id)?,
            matricule: self.matricule,
            categorie_id: lire_id(t, "categorie_id", &self.categorie_id)?,
            fonction: self.fonction,
            arrive_le: self
                .arrive_le
                .as_deref()
                .map(|d| date("arrive_le", d))
                .transpose()?,
            parti_le: self
                .parti_le
                .as_deref()
                .map(|d| date("parti_le", d))
                .transpose()?,
            motif_depart: self.motif_depart,
            fonctionnaire: booleen(self.fonctionnaire),
            numero_fonctionnaire: self.numero_fonctionnaire,
            numero_inps: self.numero_inps,
            prevenir_nom: self.prevenir_nom,
            prevenir_telephone: self.prevenir_telephone,
            paiement: serde_json::from_str(&self.paiement)?,
            diplomes: serde_json::from_str(&self.diplomes)?,
            utilisateur_id: self
                .utilisateur_id
                .as_deref()
                .map(|u| lire_id::<Utilisateur>(t, "utilisateur_id", u))
                .transpose()?,
            situation_famille: self
                .situation_famille
                .as_deref()
                .map(|s| {
                    SituationFamille::depuis_nom(s)
                        .ok_or_else(|| ErreurBase::illisible(t, "situation_famille", s))
                })
                .transpose()?,
            enfants_a_charge: self.enfants_a_charge,
        })
    }
}

macro_rules! selectionner_membre {
    ($suite:literal) => {
        concat!(
            "SELECT id, etablissement_id, personne_id, matricule, categorie_id, fonction, arrive_le,
                parti_le, motif_depart, fonctionnaire, numero_fonctionnaire, numero_inps,
                prevenir_nom, prevenir_telephone, paiement, diplomes, utilisateur_id,
                situation_famille, enfants_a_charge
             FROM membre_personnel ",
            $suite
        )
    };
}

/// Membres de l'établissement, dans l'ordre d'enregistrement.
pub async fn membres(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Vec<MembrePersonnel>> {
    sqlx::query_as::<_, LigneMembre>(selectionner_membre!(
        "WHERE etablissement_id = ?1 ORDER BY cree_le, id"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneMembre::vers_entite)
    .collect()
}

/// Un membre.
pub async fn membre(
    cnx: &mut Connexion,
    id: Id<MembrePersonnel>,
    contexte: &Contexte,
) -> Resultat<MembrePersonnel> {
    sqlx::query_as::<_, LigneMembre>(selectionner_membre!(
        "WHERE id = ?1 AND etablissement_id = ?2"
    ))
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .ok_or_else(|| ErreurBase::Introuvable {
        entite: "membre_personnel",
        id: id.vers_chaine(),
    })?
    .vers_entite()
}

/// Le membre qu'est une personne, s'il y en a un.
pub async fn membre_de_personne(
    cnx: &mut Connexion,
    personne_id: Id<Personne>,
    contexte: &Contexte,
) -> Resultat<Option<MembrePersonnel>> {
    sqlx::query_as::<_, LigneMembre>(selectionner_membre!(
        "WHERE personne_id = ?1 AND etablissement_id = ?2"
    ))
    .bind(personne_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .map(LigneMembre::vers_entite)
    .transpose()
}

/// Le membre lié à un compte, s'il y en a un.
pub async fn membre_du_compte(
    cnx: &mut Connexion,
    utilisateur_id: Id<Utilisateur>,
    contexte: &Contexte,
) -> Resultat<Option<MembrePersonnel>> {
    sqlx::query_as::<_, LigneMembre>(selectionner_membre!(
        "WHERE utilisateur_id = ?1 AND etablissement_id = ?2"
    ))
    .bind(utilisateur_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .map(LigneMembre::vers_entite)
    .transpose()
}

/// Enregistre un membre.
pub async fn inserer_membre(
    cnx: &mut Connexion,
    m: &MembrePersonnel,
    contexte: &Contexte,
) -> Resultat<()> {
    cloisonne(
        "membre_personnel",
        m.id,
        m.etablissement_id == contexte.etablissement_id,
    )?;
    sqlx::query(
        "INSERT INTO membre_personnel (id, etablissement_id, personne_id, matricule, categorie_id,
            fonction, arrive_le, parti_le, motif_depart, fonctionnaire, numero_fonctionnaire,
            numero_inps, prevenir_nom, prevenir_telephone, paiement, diplomes, utilisateur_id,
            situation_famille, enfants_a_charge,
            cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17,
            ?21, ?22, ?18, ?19, ?18, ?19, ?20)",
    )
    .bind(m.id.vers_chaine())
    .bind(m.etablissement_id.vers_chaine())
    .bind(m.personne_id.vers_chaine())
    .bind(&m.matricule)
    .bind(m.categorie_id.vers_chaine())
    .bind(&m.fonction)
    .bind(m.arrive_le.map(|d| d.to_string()))
    .bind(m.parti_le.map(|d| d.to_string()))
    .bind(&m.motif_depart)
    .bind(i64::from(m.fonctionnaire))
    .bind(&m.numero_fonctionnaire)
    .bind(&m.numero_inps)
    .bind(&m.prevenir_nom)
    .bind(&m.prevenir_telephone)
    .bind(serde_json::to_string(&m.paiement)?)
    .bind(serde_json::to_string(&m.diplomes)?)
    .bind(m.utilisateur_id.map(|u| u.vers_chaine()))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(m.situation_famille.map(|s| s.nom()))
    .bind(m.enfants_a_charge)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Récrit un membre, sauf sa personne.
pub async fn ecrire_membre(
    cnx: &mut Connexion,
    m: &MembrePersonnel,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE membre_personnel SET matricule = ?1, categorie_id = ?2, fonction = ?3,
            arrive_le = ?4, parti_le = ?5, motif_depart = ?6, fonctionnaire = ?7,
            numero_fonctionnaire = ?8, numero_inps = ?9, prevenir_nom = ?10,
            prevenir_telephone = ?11, paiement = ?12, diplomes = ?13, utilisateur_id = ?14,
            mis_a_jour_le = ?15, mis_a_jour_par = ?16, identifiant_poste = ?17,
            situation_famille = ?20, enfants_a_charge = ?21
         WHERE id = ?18 AND etablissement_id = ?19",
    )
    .bind(&m.matricule)
    .bind(m.categorie_id.vers_chaine())
    .bind(&m.fonction)
    .bind(m.arrive_le.map(|d| d.to_string()))
    .bind(m.parti_le.map(|d| d.to_string()))
    .bind(&m.motif_depart)
    .bind(i64::from(m.fonctionnaire))
    .bind(&m.numero_fonctionnaire)
    .bind(&m.numero_inps)
    .bind(&m.prevenir_nom)
    .bind(&m.prevenir_telephone)
    .bind(serde_json::to_string(&m.paiement)?)
    .bind(serde_json::to_string(&m.diplomes)?)
    .bind(m.utilisateur_id.map(|u| u.vers_chaine()))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(m.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(m.situation_famille.map(|s| s.nom()))
    .bind(m.enfants_a_charge)
    .execute(cnx)
    .await?;
    Ok(())
}

#[derive(FromRow)]
struct LignePiece {
    id: String,
    membre_id: String,
    intitule: String,
    nature: String,
    nom_fichier: String,
    taille: i64,
    active: i64,
}

impl LignePiece {
    fn vers_entite(self) -> Resultat<PieceDossier> {
        let t = "piece_dossier";
        Ok(PieceDossier {
            id: lire_id(t, "id", &self.id)?,
            membre_id: lire_id(t, "membre_id", &self.membre_id)?,
            intitule: self.intitule,
            nature: NaturePiece::depuis_nom(&self.nature)
                .ok_or_else(|| ErreurBase::illisible(t, "nature", &self.nature))?,
            nom_fichier: self.nom_fichier,
            taille: self.taille,
            active: booleen(self.active),
        })
    }
}

/// Pièces du dossier d'un membre, retirées comprises, sans leur contenu.
pub async fn pieces(
    cnx: &mut Connexion,
    membre_id: Id<MembrePersonnel>,
    contexte: &Contexte,
) -> Resultat<Vec<PieceDossier>> {
    sqlx::query_as::<_, LignePiece>(
        "SELECT id, membre_id, intitule, nature, nom_fichier, taille, active FROM piece_dossier
         WHERE membre_id = ?1 AND etablissement_id = ?2 ORDER BY cree_le, id",
    )
    .bind(membre_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LignePiece::vers_entite)
    .collect()
}

/// Une pièce et son contenu.
pub async fn piece(
    cnx: &mut Connexion,
    id: Id<PieceDossier>,
    contexte: &Contexte,
) -> Resultat<(PieceDossier, Vec<u8>)> {
    let introuvable = || ErreurBase::Introuvable {
        entite: "piece_dossier",
        id: id.vers_chaine(),
    };
    let piece = sqlx::query_as::<_, LignePiece>(
        "SELECT id, membre_id, intitule, nature, nom_fichier, taille, active FROM piece_dossier
         WHERE id = ?1 AND etablissement_id = ?2",
    )
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(&mut *cnx)
    .await?
    .ok_or_else(introuvable)?
    .vers_entite()?;
    let contenu: Vec<u8> = sqlx::query_scalar(
        "SELECT contenu FROM piece_dossier WHERE id = ?1 AND etablissement_id = ?2",
    )
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .ok_or_else(introuvable)?;
    Ok((piece, contenu))
}

/// Ajoute une pièce au dossier.
pub async fn inserer_piece(
    cnx: &mut Connexion,
    p: &PieceDossier,
    contenu: &[u8],
    empreinte: &str,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "INSERT INTO piece_dossier (id, etablissement_id, membre_id, intitule, nature, nom_fichier,
            contenu, taille, empreinte_sha256, active, cree_le, cree_par, mis_a_jour_le,
            mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?11, ?12, ?13)",
    )
    .bind(p.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(p.membre_id.vers_chaine())
    .bind(&p.intitule)
    .bind(p.nature.nom())
    .bind(&p.nom_fichier)
    .bind(contenu)
    .bind(p.taille)
    .bind(empreinte)
    .bind(i64::from(p.active))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Retire une pièce du dossier, ou l'y remet.
pub async fn ecrire_piece_active(
    cnx: &mut Connexion,
    id: Id<PieceDossier>,
    active: bool,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE piece_dossier SET active = ?1, mis_a_jour_le = ?2, mis_a_jour_par = ?3,
            identifiant_poste = ?4
         WHERE id = ?5 AND etablissement_id = ?6",
    )
    .bind(i64::from(active))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

#[derive(FromRow)]
struct LigneEngagement {
    id: String,
    etablissement_id: String,
    membre_id: String,
    nature: String,
    debut: String,
    fin: Option<String>,
    fonction: String,
    remuneration: String,
    paye_douze_mois: i64,
    statut: String,
    arrete_le: Option<String>,
    motif_fin: String,
    precedent_id: Option<String>,
    note: String,
}

impl LigneEngagement {
    fn vers_entite(self) -> Resultat<Engagement> {
        let t = "engagement";
        let date = |colonne: &'static str, texte: &str| {
            DateCivile::analyser(texte).ok_or_else(|| ErreurBase::illisible(t, colonne, texte))
        };
        Ok(Engagement {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            membre_id: lire_id(t, "membre_id", &self.membre_id)?,
            nature: NatureEngagement::depuis_nom(&self.nature)
                .ok_or_else(|| ErreurBase::illisible(t, "nature", &self.nature))?,
            debut: date("debut", &self.debut)?,
            fin: self.fin.as_deref().map(|d| date("fin", d)).transpose()?,
            fonction: self.fonction,
            remuneration: serde_json::from_str(&self.remuneration)?,
            paye_douze_mois: booleen(self.paye_douze_mois),
            statut: StatutEngagement::depuis_nom(&self.statut)
                .ok_or_else(|| ErreurBase::illisible(t, "statut", &self.statut))?,
            arrete_le: self
                .arrete_le
                .as_deref()
                .map(|d| date("arrete_le", d))
                .transpose()?,
            motif_fin: self.motif_fin,
            precedent_id: self
                .precedent_id
                .as_deref()
                .map(|p| lire_id::<Engagement>(t, "precedent_id", p))
                .transpose()?,
            note: self.note,
        })
    }
}

macro_rules! selectionner_engagement {
    ($suite:literal) => {
        concat!(
            "SELECT id, etablissement_id, membre_id, nature, debut, fin, fonction, remuneration,
                paye_douze_mois, statut, arrete_le, motif_fin, precedent_id, note
             FROM engagement ",
            $suite
        )
    };
}

/// Engagements de l'établissement, par membre puis par début.
pub async fn engagements(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Vec<Engagement>> {
    sqlx::query_as::<_, LigneEngagement>(selectionner_engagement!(
        "WHERE etablissement_id = ?1 ORDER BY membre_id, debut, cree_le"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneEngagement::vers_entite)
    .collect()
}

/// Engagements d'un membre, du plus ancien au plus récent.
pub async fn engagements_du_membre(
    cnx: &mut Connexion,
    membre_id: Id<MembrePersonnel>,
    contexte: &Contexte,
) -> Resultat<Vec<Engagement>> {
    sqlx::query_as::<_, LigneEngagement>(selectionner_engagement!(
        "WHERE membre_id = ?1 AND etablissement_id = ?2 ORDER BY debut, cree_le"
    ))
    .bind(membre_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneEngagement::vers_entite)
    .collect()
}

/// Un engagement.
pub async fn engagement(
    cnx: &mut Connexion,
    id: Id<Engagement>,
    contexte: &Contexte,
) -> Resultat<Engagement> {
    sqlx::query_as::<_, LigneEngagement>(selectionner_engagement!(
        "WHERE id = ?1 AND etablissement_id = ?2"
    ))
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .ok_or_else(|| ErreurBase::Introuvable {
        entite: "engagement",
        id: id.vers_chaine(),
    })?
    .vers_entite()
}

/// Enregistre un engagement.
pub async fn inserer_engagement(
    cnx: &mut Connexion,
    e: &Engagement,
    contexte: &Contexte,
) -> Resultat<()> {
    cloisonne(
        "engagement",
        e.id,
        e.etablissement_id == contexte.etablissement_id,
    )?;
    sqlx::query(
        "INSERT INTO engagement (id, etablissement_id, membre_id, nature, debut, fin, fonction,
            remuneration, statut, arrete_le, motif_fin, precedent_id, note, cree_le, cree_par,
            mis_a_jour_le, mis_a_jour_par, identifiant_poste, paye_douze_mois)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?14, ?15, ?16,
            ?17)",
    )
    .bind(e.id.vers_chaine())
    .bind(e.etablissement_id.vers_chaine())
    .bind(e.membre_id.vers_chaine())
    .bind(e.nature.nom())
    .bind(e.debut.to_string())
    .bind(e.fin.map(|d| d.to_string()))
    .bind(&e.fonction)
    .bind(serde_json::to_string(&e.remuneration)?)
    .bind(gesco_core::core::transition::MachineAEtats::nom(&e.statut))
    .bind(e.arrete_le.map(|d| d.to_string()))
    .bind(&e.motif_fin)
    .bind(e.precedent_id.map(|p| p.vers_chaine()))
    .bind(&e.note)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(i64::from(e.paye_douze_mois))
    .execute(cnx)
    .await?;
    Ok(())
}

/// Récrit les mois payés d'un engagement mensuel (SPEC_PHASE_7 §2) : douze, ou hors vacances.
pub async fn ecrire_mois_payes(
    cnx: &mut Connexion,
    e: &Engagement,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE engagement SET paye_douze_mois = ?1,
            mis_a_jour_le = ?2, mis_a_jour_par = ?3, identifiant_poste = ?4
         WHERE id = ?5 AND etablissement_id = ?6",
    )
    .bind(i64::from(e.paye_douze_mois))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(e.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Récrit l'état, le dernier jour et le motif de fin d'un engagement : seule sa clôture change.
pub async fn clore_engagement(
    cnx: &mut Connexion,
    e: &Engagement,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE engagement SET statut = ?1, arrete_le = ?2, motif_fin = ?3,
            mis_a_jour_le = ?4, mis_a_jour_par = ?5, identifiant_poste = ?6
         WHERE id = ?7 AND etablissement_id = ?8",
    )
    .bind(gesco_core::core::transition::MachineAEtats::nom(&e.statut))
    .bind(e.arrete_le.map(|d| d.to_string()))
    .bind(&e.motif_fin)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(e.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

#[derive(FromRow)]
struct LigneHeures {
    id: String,
    etablissement_id: String,
    engagement_id: String,
    membre_id: String,
    mois: String,
    minutes_prevues: i64,
    minutes_faites: Option<i64>,
    observation: String,
    statut: String,
    vise_le: Option<i64>,
    vise_par: Option<String>,
    motif_deverrouillage: String,
}

impl LigneHeures {
    fn vers_entite(self) -> Resultat<HeuresVacation> {
        let t = "heures_vacation";
        Ok(HeuresVacation {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            engagement_id: lire_id(t, "engagement_id", &self.engagement_id)?,
            membre_id: lire_id(t, "membre_id", &self.membre_id)?,
            mois: Mois::analyser(&self.mois)
                .ok_or_else(|| ErreurBase::illisible(t, "mois", &self.mois))?,
            minutes_prevues: self.minutes_prevues,
            minutes_faites: self.minutes_faites,
            observation: self.observation,
            statut: StatutHeures::depuis_nom(&self.statut)
                .ok_or_else(|| ErreurBase::illisible(t, "statut", &self.statut))?,
            vise_le: crate::depots::horodatage_optionnel(self.vise_le),
            vise_par: self
                .vise_par
                .as_deref()
                .map(|u| lire_id::<Utilisateur>(t, "vise_par", u))
                .transpose()?,
            motif_deverrouillage: self.motif_deverrouillage,
        })
    }
}

macro_rules! selectionner_heures {
    ($suite:literal) => {
        concat!(
            "SELECT id, etablissement_id, engagement_id, membre_id, mois, minutes_prevues,
                minutes_faites, observation, statut, vise_le, vise_par, motif_deverrouillage
             FROM heures_vacation ",
            $suite
        )
    };
}

/// Heures de vacation d'un mois.
pub async fn heures_du_mois(
    cnx: &mut Connexion,
    mois: Mois,
    contexte: &Contexte,
) -> Resultat<Vec<HeuresVacation>> {
    sqlx::query_as::<_, LigneHeures>(selectionner_heures!(
        "WHERE etablissement_id = ?1 AND mois = ?2 ORDER BY cree_le, id"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(mois.to_string())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneHeures::vers_entite)
    .collect()
}

/// Heures visées et pas encore payées, jusqu'au mois donné compris : celles du mois et les
/// rappels des mois précédents.
pub async fn heures_a_payer(
    cnx: &mut Connexion,
    jusqua: Mois,
    contexte: &Contexte,
) -> Resultat<Vec<HeuresVacation>> {
    sqlx::query_as::<_, LigneHeures>(selectionner_heures!(
        "WHERE etablissement_id = ?1 AND statut = 'visee' AND bulletin_paie_id IS NULL AND mois <= ?2
         ORDER BY mois, cree_le, id"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(jusqua.to_string())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneHeures::vers_entite)
    .collect()
}

/// Bulletin de paie qui a payé une ligne d'heures, s'il y en a un.
pub async fn bulletin_des_heures(
    cnx: &mut Connexion,
    id: Id<HeuresVacation>,
    contexte: &Contexte,
) -> Resultat<Option<String>> {
    Ok(sqlx::query_scalar(
        "SELECT bulletin_paie_id FROM heures_vacation WHERE id = ?1 AND etablissement_id = ?2",
    )
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .flatten())
}

/// Marque une ligne d'heures payée par un bulletin.
pub async fn marquer_heures_payees(
    cnx: &mut Connexion,
    id: Id<HeuresVacation>,
    bulletin_id: &str,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE heures_vacation SET bulletin_paie_id = ?1, mis_a_jour_le = ?2, mis_a_jour_par = ?3,
            identifiant_poste = ?4
         WHERE id = ?5 AND etablissement_id = ?6 AND bulletin_paie_id IS NULL",
    )
    .bind(bulletin_id)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Une ligne d'heures.
pub async fn heures(
    cnx: &mut Connexion,
    id: Id<HeuresVacation>,
    contexte: &Contexte,
) -> Resultat<HeuresVacation> {
    sqlx::query_as::<_, LigneHeures>(selectionner_heures!(
        "WHERE id = ?1 AND etablissement_id = ?2"
    ))
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .ok_or_else(|| ErreurBase::Introuvable {
        entite: "heures_vacation",
        id: id.vers_chaine(),
    })?
    .vers_entite()
}

/// Enregistre une ligne d'heures.
pub async fn inserer_heures(
    cnx: &mut Connexion,
    h: &HeuresVacation,
    contexte: &Contexte,
) -> Resultat<()> {
    cloisonne(
        "heures_vacation",
        h.id,
        h.etablissement_id == contexte.etablissement_id,
    )?;
    sqlx::query(
        "INSERT INTO heures_vacation (id, etablissement_id, engagement_id, membre_id, mois,
            minutes_prevues, minutes_faites, observation, statut, vise_le, vise_par,
            motif_deverrouillage, cree_le, cree_par, mis_a_jour_le, mis_a_jour_par,
            identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?13, ?14, ?15)",
    )
    .bind(h.id.vers_chaine())
    .bind(h.etablissement_id.vers_chaine())
    .bind(h.engagement_id.vers_chaine())
    .bind(h.membre_id.vers_chaine())
    .bind(h.mois.to_string())
    .bind(h.minutes_prevues)
    .bind(h.minutes_faites)
    .bind(&h.observation)
    .bind(gesco_core::core::transition::MachineAEtats::nom(&h.statut))
    .bind(h.vise_le.map(|v| v.millis_epoque()))
    .bind(h.vise_par.map(|u| u.vers_chaine()))
    .bind(&h.motif_deverrouillage)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Récrit les heures, l'observation, l'état et le visa d'une ligne.
pub async fn ecrire_heures(
    cnx: &mut Connexion,
    h: &HeuresVacation,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE heures_vacation SET minutes_prevues = ?1, minutes_faites = ?2, observation = ?3,
            statut = ?4, vise_le = ?5, vise_par = ?6, motif_deverrouillage = ?7,
            mis_a_jour_le = ?8, mis_a_jour_par = ?9, identifiant_poste = ?10
         WHERE id = ?11 AND etablissement_id = ?12",
    )
    .bind(h.minutes_prevues)
    .bind(h.minutes_faites)
    .bind(&h.observation)
    .bind(gesco_core::core::transition::MachineAEtats::nom(&h.statut))
    .bind(h.vise_le.map(|v| v.millis_epoque()))
    .bind(h.vise_par.map(|u| u.vers_chaine()))
    .bind(&h.motif_deverrouillage)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(h.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}
