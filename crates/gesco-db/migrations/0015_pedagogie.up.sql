-- Référentiel pédagogique (SPEC_PHASE_4 §2) : matières de l'établissement, programme de chaque
-- niveau et série par année, règles d'évaluation versionnées (RC-4). Les périodes existent depuis
-- la phase 0.5 (`periode_academique`) ; elles reçoivent ici leur unicité par niveau.

CREATE TABLE matiere (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    code              TEXT NOT NULL,
    libelle           TEXT NOT NULL,
    libelle_court     TEXT NOT NULL,
    groupe            TEXT NOT NULL CHECK (groupe IN ('litteraire', 'scientifique', 'autre')),
    ordre             INTEGER NOT NULL,
    active            INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE UNIQUE INDEX matiere_code_unique ON matiere (etablissement_id, code);

CREATE TABLE programme_matiere (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    annee_id          TEXT NOT NULL REFERENCES annee_scolaire(id),
    niveau_id         TEXT NOT NULL REFERENCES niveau(id),
    serie_id          TEXT REFERENCES serie(id),
    matiere_id        TEXT NOT NULL REFERENCES matiere(id),
    coefficient       INTEGER CHECK (coefficient IS NULL OR coefficient BETWEEN 1 AND 20),
    minutes_hebdo     INTEGER CHECK (minutes_hebdo IS NULL OR minutes_hebdo BETWEEN 0 AND 3000),
    ordre             INTEGER NOT NULL,
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE UNIQUE INDEX programme_matiere_unique
    ON programme_matiere (annee_id, niveau_id, coalesce(serie_id, ''), matiere_id);

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
CREATE UNIQUE INDEX regle_evaluation_version_unique ON regle_evaluation (annee_id, cycle, version);
-- Une version de règle a produit des bulletins : elle ne change ni ne disparaît (RC-4).
CREATE TRIGGER regle_evaluation_immuable BEFORE UPDATE ON regle_evaluation
BEGIN SELECT RAISE(ABORT, 'une version de règle ne se modifie pas'); END;
CREATE TRIGGER regle_evaluation_sans_suppression BEFORE DELETE ON regle_evaluation
BEGIN SELECT RAISE(ABORT, 'une version de règle ne se supprime pas'); END;

CREATE UNIQUE INDEX periode_rang_unique ON periode_academique (annee_id, niveau_id, rang)
    WHERE supprime_le IS NULL;
