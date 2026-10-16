-- Retour aux contrôles de la phase 4 : ne réussit que sans niveau ni règle du cycle technique.

PRAGMA defer_foreign_keys = ON;

CREATE TABLE niveau_ancien AS SELECT * FROM niveau;
DROP TABLE niveau;
CREATE TABLE niveau (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    code              TEXT NOT NULL,
    libelle           TEXT NOT NULL,
    cycle             TEXT NOT NULL CHECK (cycle IN ('jardin', 'premier_cycle', 'second_cycle', 'lycee')),
    ordre             INTEGER NOT NULL,
    mode_evaluation   TEXT NOT NULL CHECK (mode_evaluation IN ('notes', 'competences')),
    diplome_terminal  TEXT NOT NULL CHECK (diplome_terminal IN ('aucun', 'def', 'bac')),
    actif             INTEGER NOT NULL DEFAULT 1 CHECK (actif IN (0, 1)),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    supprime_le       INTEGER,
    supprime_par      TEXT
);
INSERT INTO niveau SELECT * FROM niveau_ancien;
DROP TABLE niveau_ancien;
CREATE UNIQUE INDEX niveau_code_unique ON niveau (etablissement_id, code);

CREATE TABLE regle_evaluation_ancienne AS SELECT * FROM regle_evaluation;
DROP TABLE regle_evaluation;
CREATE TABLE regle_evaluation (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    annee_id          TEXT NOT NULL REFERENCES annee_scolaire(id),
    cycle             TEXT NOT NULL CHECK (cycle IN ('premier_cycle', 'second_cycle', 'lycee')),
    version           INTEGER NOT NULL CHECK (version >= 1),
    contenu           TEXT NOT NULL,
    le                INTEGER NOT NULL,
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    identifiant_poste TEXT NOT NULL
);
INSERT INTO regle_evaluation SELECT * FROM regle_evaluation_ancienne;
DROP TABLE regle_evaluation_ancienne;
CREATE UNIQUE INDEX regle_evaluation_version_unique ON regle_evaluation (annee_id, cycle, version);
CREATE TRIGGER regle_evaluation_immuable BEFORE UPDATE ON regle_evaluation
BEGIN SELECT RAISE(ABORT, 'une version de règle ne se modifie pas'); END;
CREATE TRIGGER regle_evaluation_sans_suppression BEFORE DELETE ON regle_evaluation
BEGIN SELECT RAISE(ABORT, 'une version de règle ne se supprime pas'); END;
