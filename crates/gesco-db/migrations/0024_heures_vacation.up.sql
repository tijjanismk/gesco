-- Heures des vacataires (SPEC_PHASE_6 §4, T-41) : une ligne par engagement de vacation et par
-- mois, heures prévues et faites, visée par la direction ; la paie (phase 7) paie les heures
-- visées. Une ligne visée ne change plus que par un déverrouillage motivé ; rien ne se supprime.

CREATE TABLE heures_vacation (
    id                   TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id     TEXT NOT NULL REFERENCES etablissement(id),
    engagement_id        TEXT NOT NULL REFERENCES engagement(id),
    membre_id            TEXT NOT NULL REFERENCES membre_personnel(id),
    mois                 TEXT NOT NULL CHECK (length(mois) = 7),
    minutes_prevues      INTEGER NOT NULL CHECK (minutes_prevues >= 0),
    minutes_faites       INTEGER CHECK (minutes_faites IS NULL OR minutes_faites >= 0),
    observation          TEXT NOT NULL DEFAULT '',
    statut               TEXT NOT NULL CHECK (statut IN ('en_saisie', 'visee')),
    vise_le              INTEGER,
    vise_par             TEXT,
    motif_deverrouillage TEXT NOT NULL DEFAULT '',
    cree_le              INTEGER NOT NULL,
    cree_par             TEXT,
    mis_a_jour_le        INTEGER NOT NULL,
    mis_a_jour_par       TEXT,
    identifiant_poste    TEXT NOT NULL,
    -- Une ligne visée a ses heures faites.
    CHECK (statut = 'en_saisie' OR minutes_faites IS NOT NULL)
);
CREATE UNIQUE INDEX heures_vacation_unique ON heures_vacation (engagement_id, mois);
CREATE INDEX heures_vacation_mois ON heures_vacation (etablissement_id, mois);

CREATE TRIGGER heures_vacation_sans_suppression BEFORE DELETE ON heures_vacation
BEGIN SELECT RAISE(ABORT, 'des heures de vacation ne se suppriment pas'); END;
