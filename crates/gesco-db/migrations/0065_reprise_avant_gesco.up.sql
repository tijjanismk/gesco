-- Reprise avant GESCO (SPEC_REPRISE_AVANT_GESCO §10, ADR 2026-11-17-reprise-avant-gesco).
--
-- Ce que les parents ont payé avant l'installation : un montant par inscription, qui règle des
-- échéances comme un avoir, sans versement, sans reçu, sans caisse. Rien ne se supprime ni ne se
-- réécrit (R-14) : une modification annule l'ancienne reprise et en crée une nouvelle.
CREATE TABLE reprise_avant_gesco (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    inscription_id    TEXT NOT NULL REFERENCES inscription(id),
    montant           INTEGER NOT NULL CHECK (montant > 0),
    note              TEXT NOT NULL DEFAULT '' CHECK (length(note) <= 120),
    origine           TEXT NOT NULL CHECK (origine IN ('ecran', 'import')),
    statut            TEXT NOT NULL CHECK (statut IN ('active', 'annulee')),
    motif_annulation  TEXT NOT NULL DEFAULT '',
    remplacee_par     TEXT REFERENCES reprise_avant_gesco(id),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    CHECK (statut <> 'annulee' OR length(trim(motif_annulation)) > 0)
);
-- Une seule reprise active par inscription, tenue par la base.
CREATE UNIQUE INDEX reprise_avant_gesco_active ON reprise_avant_gesco (inscription_id)
    WHERE statut = 'active';
CREATE INDEX reprise_avant_gesco_inscription ON reprise_avant_gesco (inscription_id);

-- Part d'une reprise portée par une échéance : un règlement sans encaissement.
CREATE TABLE reprise_imputation (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    reprise_id        TEXT NOT NULL REFERENCES reprise_avant_gesco(id),
    echeance_id       TEXT NOT NULL REFERENCES echeance(id),
    montant           INTEGER NOT NULL CHECK (montant > 0),
    statut            TEXT NOT NULL CHECK (statut IN ('active', 'annulee')),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE INDEX reprise_imputation_echeance ON reprise_imputation (echeance_id) WHERE statut = 'active';
CREATE INDEX reprise_imputation_reprise ON reprise_imputation (reprise_id);

CREATE TRIGGER reprise_avant_gesco_sans_suppression BEFORE DELETE ON reprise_avant_gesco
BEGIN SELECT RAISE(ABORT, 'une reprise avant GESCO ne se supprime pas'); END;
CREATE TRIGGER reprise_imputation_sans_suppression BEFORE DELETE ON reprise_imputation
BEGIN SELECT RAISE(ABORT, 'une part de reprise ne se supprime pas'); END;

-- Le remboursement d'un excédent au parent est une dépense liée à l'inscription.
ALTER TABLE depense ADD COLUMN inscription_id TEXT REFERENCES inscription(id);
CREATE INDEX depense_inscription ON depense (inscription_id) WHERE inscription_id IS NOT NULL;
