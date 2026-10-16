-- Journée de l'école et emploi du temps (SPEC_PHASE_8 §8, T-38, T-64) : la journée de chaque
-- cycle, réglée par l'école ; les séances placées à la main. Rien ne se supprime (R-14) : une
-- séance se retire.

CREATE TABLE journee_cycle (
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    cycle             TEXT NOT NULL CHECK (cycle IN ('jardin', 'premier_cycle', 'second_cycle', 'lycee', 'technique')),
    contenu           TEXT NOT NULL CHECK (json_valid(contenu)),
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    PRIMARY KEY (etablissement_id, cycle)
);

CREATE TABLE seance (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    annee_id          TEXT NOT NULL REFERENCES annee_scolaire(id),
    classe_id         TEXT NOT NULL REFERENCES classe(id),
    jour              INTEGER NOT NULL CHECK (jour BETWEEN 1 AND 6),
    debut             INTEGER NOT NULL CHECK (debut >= 0),
    fin               INTEGER NOT NULL CHECK (fin <= 1440),
    matiere_id        TEXT NOT NULL REFERENCES matiere(id),
    enseignant_id     TEXT REFERENCES enseignant(id),
    salle             TEXT NOT NULL DEFAULT '',
    active            INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    CHECK (debut < fin)
);
CREATE INDEX seance_annee ON seance (annee_id, classe_id);

CREATE TRIGGER journee_cycle_sans_suppression BEFORE DELETE ON journee_cycle
BEGIN SELECT RAISE(ABORT, 'la journée d''un cycle ne se supprime pas'); END;
CREATE TRIGGER seance_sans_suppression BEFORE DELETE ON seance
BEGIN SELECT RAISE(ABORT, 'une séance ne se supprime pas'); END;
