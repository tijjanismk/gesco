-- Référentiel académique de la phase 2 (SPEC_PHASE_2 §2, §3) : niveaux et séries, copies du
-- référentiel national que l'école renomme ou désactive, et classes de chaque année (R-16).

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
CREATE UNIQUE INDEX niveau_code_unique ON niveau (etablissement_id, code);

CREATE TABLE serie (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    niveau_id         TEXT NOT NULL REFERENCES niveau(id),
    code              TEXT NOT NULL,
    libelle           TEXT NOT NULL,
    actif             INTEGER NOT NULL DEFAULT 1 CHECK (actif IN (0, 1)),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    supprime_le       INTEGER,
    supprime_par      TEXT
);
CREATE UNIQUE INDEX serie_code_unique ON serie (etablissement_id, niveau_id, code);

CREATE TABLE classe (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    annee_id          TEXT NOT NULL REFERENCES annee_scolaire(id),
    niveau_id         TEXT NOT NULL REFERENCES niveau(id),
    serie_id          TEXT REFERENCES serie(id),
    libelle           TEXT NOT NULL,
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    supprime_le       INTEGER,
    supprime_par      TEXT
);
-- Libellé unique dans l'année, parmi les classes non supprimées.
CREATE UNIQUE INDEX classe_libelle_unique
    ON classe (etablissement_id, annee_id, libelle) WHERE supprime_le IS NULL;
CREATE INDEX classe_annee ON classe (etablissement_id, annee_id);
