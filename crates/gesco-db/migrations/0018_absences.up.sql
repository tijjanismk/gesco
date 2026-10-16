-- Absences et retards (SPEC_PHASE_4 §6, T-37) : une ligne par élève et par demi-journée, absence
-- ou retard. Une absence ne se supprime jamais : effacée du relevé, elle reste tracée par l'audit
-- et sa ligne est marquée `retiree`. Le relevé d'une classe un jour donné est noté, pour qu'une
-- journée sans absent se distingue d'une journée non relevée.

CREATE TABLE absence (
    inscription_id    TEXT NOT NULL REFERENCES inscription(id),
    date              TEXT NOT NULL CHECK (length(date) = 10),
    demi_journee      TEXT NOT NULL CHECK (demi_journee IN ('matin', 'apres_midi')),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    classe_id         TEXT NOT NULL REFERENCES classe(id),
    nature            TEXT NOT NULL CHECK (nature IN ('absence', 'retard')),
    minutes           INTEGER CHECK (minutes IS NULL OR minutes BETWEEN 1 AND 240),
    justifiee         INTEGER NOT NULL CHECK (justifiee IN (0, 1)),
    motif             TEXT NOT NULL DEFAULT '',
    justificatif      TEXT NOT NULL DEFAULT '',
    retiree           INTEGER NOT NULL DEFAULT 0 CHECK (retiree IN (0, 1)),
    saisie_le         INTEGER NOT NULL,
    saisie_par        TEXT,
    justifiee_le      INTEGER,
    justifiee_par     TEXT,
    identifiant_poste TEXT NOT NULL,
    PRIMARY KEY (inscription_id, date, demi_journee),
    -- Seul un retard a une durée.
    CHECK (nature = 'retard' OR minutes IS NULL)
);
CREATE INDEX absence_classe ON absence (classe_id, date);

CREATE TRIGGER absence_sans_suppression BEFORE DELETE ON absence
BEGIN SELECT RAISE(ABORT, 'une absence ne se supprime pas'); END;

CREATE TABLE releve_absences (
    classe_id         TEXT NOT NULL REFERENCES classe(id),
    date              TEXT NOT NULL CHECK (length(date) = 10),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    fait_le           INTEGER NOT NULL,
    fait_par          TEXT,
    identifiant_poste TEXT NOT NULL,
    PRIMARY KEY (classe_id, date)
);
