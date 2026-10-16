-- Évaluations et notes (SPEC_PHASE_4 §4, §8.4 du blueprint). La note est stockée telle
-- qu'écrite, avec le barème de son évaluation (RC-2) ; un absent n'a pas de valeur (RC-3). Une
-- note ne se supprime jamais : vidée, elle reste tracée par l'audit et sa ligne disparaît de la
-- grille par l'indicateur `retiree`.

CREATE TABLE evaluation (
    id                    TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id      TEXT NOT NULL REFERENCES etablissement(id),
    annee_id              TEXT NOT NULL REFERENCES annee_scolaire(id),
    periode_id            TEXT NOT NULL REFERENCES periode_academique(id),
    classe_id             TEXT NOT NULL REFERENCES classe(id),
    matiere_id            TEXT NOT NULL REFERENCES matiere(id),
    type_evaluation       TEXT NOT NULL CHECK (type_evaluation IN ('devoir', 'composition')),
    libelle               TEXT NOT NULL,
    date                  TEXT NOT NULL CHECK (length(date) = 10),
    bareme                INTEGER NOT NULL CHECK (bareme BETWEEN 100 AND 10000),
    ponderation           INTEGER NOT NULL CHECK (ponderation BETWEEN 1 AND 10),
    enseignant_id         TEXT REFERENCES enseignant(id),
    statut                TEXT NOT NULL CHECK (statut IN ('en_saisie', 'validee')),
    motif_deverrouillage  TEXT NOT NULL DEFAULT '',
    cree_le               INTEGER NOT NULL,
    cree_par              TEXT,
    mis_a_jour_le         INTEGER NOT NULL,
    mis_a_jour_par        TEXT,
    identifiant_poste     TEXT NOT NULL
);
CREATE INDEX evaluation_classe ON evaluation (classe_id, periode_id, matiere_id);

CREATE TABLE note (
    evaluation_id     TEXT NOT NULL REFERENCES evaluation(id),
    inscription_id    TEXT NOT NULL REFERENCES inscription(id),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    valeur            INTEGER CHECK (valeur IS NULL OR valeur >= 0),
    absent            INTEGER NOT NULL CHECK (absent IN (0, 1)),
    justifie          INTEGER NOT NULL CHECK (justifie IN (0, 1)),
    dispense          INTEGER NOT NULL CHECK (dispense IN (0, 1)),
    retiree           INTEGER NOT NULL DEFAULT 0 CHECK (retiree IN (0, 1)),
    saisie_le         INTEGER NOT NULL,
    saisie_par        TEXT,
    identifiant_poste TEXT NOT NULL,
    PRIMARY KEY (evaluation_id, inscription_id),
    -- Une valeur ou une absence ou une dispense, jamais deux à la fois.
    CHECK (NOT (valeur IS NOT NULL AND (absent = 1 OR dispense = 1))),
    CHECK (NOT (absent = 1 AND dispense = 1))
);
CREATE INDEX note_inscription ON note (inscription_id);

CREATE TRIGGER note_sans_suppression BEFORE DELETE ON note
BEGIN SELECT RAISE(ABORT, 'une note ne se supprime pas'); END;
