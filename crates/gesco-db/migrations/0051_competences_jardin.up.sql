-- Évaluation par compétences au jardin d'enfants (SPEC_PHASE_11_B §3) : domaines communs aux
-- trois sections, compétences de chaque section, une observation par enfant, compétence et
-- période. La liste n'a pas d'année scolaire : elle vaut d'une année à l'autre. Rien ne se
-- supprime (R-14) : un domaine ou une compétence se retire et se remet ; une observation se vide.
-- La liste livrée est posée par l'application, pour les seules sections que l'école a ouvertes ;
-- un domaine livré garde son code, qui le retrouve même renommé.

CREATE TABLE domaine_competence (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    code              TEXT,
    libelle           TEXT NOT NULL CHECK (length(trim(libelle)) BETWEEN 1 AND 60),
    ordre             INTEGER NOT NULL,
    livre             INTEGER NOT NULL DEFAULT 0 CHECK (livre IN (0, 1)),
    retire_le         INTEGER,
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE INDEX domaine_competence_etablissement ON domaine_competence (etablissement_id, ordre);
CREATE UNIQUE INDEX domaine_competence_code ON domaine_competence (etablissement_id, code)
    WHERE code IS NOT NULL;

CREATE TRIGGER domaine_competence_sans_suppression BEFORE DELETE ON domaine_competence
BEGIN SELECT RAISE(ABORT, 'un domaine ne se supprime pas'); END;

CREATE TABLE competence (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    domaine_id        TEXT NOT NULL REFERENCES domaine_competence(id),
    niveau_id         TEXT NOT NULL REFERENCES niveau(id),
    libelle           TEXT NOT NULL CHECK (length(trim(libelle)) BETWEEN 1 AND 120),
    ordre             INTEGER NOT NULL,
    livree            INTEGER NOT NULL DEFAULT 0 CHECK (livree IN (0, 1)),
    retiree_le        INTEGER,
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE INDEX competence_section ON competence (etablissement_id, niveau_id, domaine_id, ordre);

CREATE TRIGGER competence_sans_suppression BEFORE DELETE ON competence
BEGIN SELECT RAISE(ABORT, 'une compétence ne se supprime pas'); END;

CREATE TABLE observation (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    inscription_id    TEXT NOT NULL REFERENCES inscription(id),
    competence_id     TEXT NOT NULL REFERENCES competence(id),
    periode_id        TEXT NOT NULL REFERENCES periode_academique(id),
    niveau            TEXT CHECK (niveau IN ('acquis', 'en_cours', 'non_acquis')),
    saisie_le         INTEGER NOT NULL,
    saisie_par        TEXT REFERENCES utilisateur(id),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE UNIQUE INDEX observation_unique ON observation (inscription_id, competence_id, periode_id);
CREATE INDEX observation_periode ON observation (periode_id, competence_id);

CREATE TRIGGER observation_sans_suppression BEFORE DELETE ON observation
BEGIN SELECT RAISE(ABORT, 'une observation ne se supprime pas'); END;
