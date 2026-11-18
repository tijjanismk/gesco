-- Prise en charge par l'État (SPEC_PRISE_EN_CHARGE_ETAT §3.1, §3.2, §7.1 ; ADR
-- 2026-11-18-prise-en-charge-par-l-etat).
--
-- La scolarité d'un élève orienté par l'État est due par l'État ; le partage de chaque échéance
-- entre l'État et le parent se calcule à la lecture (R-20) et ne s'écrit nulle part. Ces tables ne
-- portent que les versions de la prise en charge, le montant attendu par niveau, les versements de
-- l'État et leur répartition. Rien ne se supprime (R-14).

-- Une ligne par version de la prise en charge d'une inscription ; une seule courante.
CREATE TABLE prise_en_charge_etat (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    inscription_id    TEXT NOT NULL REFERENCES inscription(id),
    statut            TEXT NOT NULL CHECK (statut IN ('en_cours', 'terminee', 'remplacee')),
    montant_attendu   INTEGER NOT NULL CHECK (montant_attendu >= 0),
    abandonne         INTEGER NOT NULL DEFAULT 0
                      CHECK (abandonne >= 0 AND abandonne <= montant_attendu),
    reference         TEXT NOT NULL DEFAULT '' CHECK (length(reference) <= 60),
    motif             TEXT NOT NULL DEFAULT '',
    remplace_id       TEXT REFERENCES prise_en_charge_etat(id),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    -- Motif obligatoire pour une version terminée et pour un abandon.
    CHECK (statut <> 'terminee' OR length(trim(motif)) > 0),
    CHECK (abandonne = 0 OR length(trim(motif)) > 0)
);
-- Une seule version courante (en cours ou terminée) par inscription, tenue par la base.
CREATE UNIQUE INDEX prise_en_charge_etat_courante ON prise_en_charge_etat (inscription_id)
    WHERE statut <> 'remplacee';
CREATE INDEX prise_en_charge_etat_inscription ON prise_en_charge_etat (inscription_id);

-- Montant attendu de l'État par année, niveau et série ; vide, les frais de l'école.
CREATE TABLE montant_etat (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    annee_id          TEXT NOT NULL REFERENCES annee_scolaire(id),
    niveau_id         TEXT NOT NULL REFERENCES niveau(id),
    serie_id          TEXT REFERENCES serie(id),
    montant           INTEGER CHECK (montant IS NULL OR montant >= 0),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE UNIQUE INDEX montant_etat_ligne
    ON montant_etat (etablissement_id, annee_id, niveau_id, COALESCE(serie_id, ''));

-- Un versement de l'État : l'argent porté par une recette de trésorerie, ou reçu avant GESCO
-- (déjà dans le solde d'ouverture du compte, sans recette).
CREATE TABLE versement_etat (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    annee_id          TEXT NOT NULL REFERENCES annee_scolaire(id),
    date              TEXT NOT NULL CHECK (length(date) = 10),
    montant           INTEGER NOT NULL CHECK (montant > 0),
    reference         TEXT NOT NULL DEFAULT '' CHECK (length(reference) <= 120),
    libelle           TEXT NOT NULL CHECK (length(trim(libelle)) BETWEEN 1 AND 120),
    payeur            TEXT NOT NULL DEFAULT 'État' CHECK (length(trim(payeur)) BETWEEN 1 AND 120),
    recette_id        TEXT REFERENCES recette_tresorerie(id),
    avant_gesco       INTEGER NOT NULL DEFAULT 0 CHECK (avant_gesco IN (0, 1)),
    statut            TEXT NOT NULL CHECK (statut IN ('valide', 'annule')),
    motif_annulation  TEXT NOT NULL DEFAULT '',
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    -- Reçu avant GESCO exactement quand il n'y a pas de recette.
    CHECK ((avant_gesco = 1) = (recette_id IS NULL)),
    CHECK (statut <> 'annule' OR length(trim(motif_annulation)) > 0)
);
CREATE INDEX versement_etat_annee ON versement_etat (etablissement_id, annee_id);
CREATE UNIQUE INDEX versement_etat_recette ON versement_etat (recette_id)
    WHERE recette_id IS NOT NULL;

-- Ce qu'un versement de l'État attribue à une inscription.
CREATE TABLE versement_etat_part (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    versement_etat_id TEXT NOT NULL REFERENCES versement_etat(id),
    inscription_id    TEXT NOT NULL REFERENCES inscription(id),
    montant           INTEGER NOT NULL CHECK (montant > 0),
    origine           TEXT NOT NULL CHECK (origine IN ('ecran', 'import')),
    statut            TEXT NOT NULL CHECK (statut IN ('active', 'annulee')),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE INDEX versement_etat_part_versement ON versement_etat_part (versement_etat_id);
CREATE INDEX versement_etat_part_inscription ON versement_etat_part (inscription_id)
    WHERE statut = 'active';

CREATE TRIGGER prise_en_charge_etat_sans_suppression BEFORE DELETE ON prise_en_charge_etat
BEGIN SELECT RAISE(ABORT, 'une prise en charge par l''État ne se supprime pas'); END;
CREATE TRIGGER montant_etat_sans_suppression BEFORE DELETE ON montant_etat
BEGIN SELECT RAISE(ABORT, 'un montant attendu de l''État ne se supprime pas'); END;
CREATE TRIGGER versement_etat_sans_suppression BEFORE DELETE ON versement_etat
BEGIN SELECT RAISE(ABORT, 'un versement de l''État ne se supprime pas'); END;
CREATE TRIGGER versement_etat_part_sans_suppression BEFORE DELETE ON versement_etat_part
BEGIN SELECT RAISE(ABORT, 'une part d''un versement de l''État ne se supprime pas'); END;
