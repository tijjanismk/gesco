-- Absences par séance (SPEC_VERSION_1_1 §1, étape 6, T-71) : quand l'école le règle, l'absence
-- se relève à chaque séance de l'emploi du temps ; la demi-journée s'en déduit. Rien ne se
-- supprime (R-14) : une case décochée passe à zéro.

CREATE TABLE absence_seance (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    inscription_id    TEXT NOT NULL REFERENCES inscription(id),
    date              TEXT NOT NULL CHECK (length(date) = 10),
    seance_id         TEXT NOT NULL REFERENCES seance(id),
    absent            INTEGER NOT NULL CHECK (absent IN (0, 1)),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE UNIQUE INDEX absence_seance_case ON absence_seance (inscription_id, date, seance_id);
CREATE INDEX absence_seance_jour ON absence_seance (etablissement_id, date);

CREATE TRIGGER absence_seance_sans_suppression BEFORE DELETE ON absence_seance
BEGIN SELECT RAISE(ABORT, 'une absence de séance ne se supprime pas'); END;
