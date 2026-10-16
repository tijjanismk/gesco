-- Appréciations et bulletins (SPEC_PHASE_4 §7). Le bulletin porte son contenu figé au calcul, en
-- JSON, et en colonnes ce que la suite interroge (moyenne et rang, pour la moyenne annuelle). Un
-- bulletin ne se supprime jamais ; publié, son contenu ne change plus (R-14) : une correction crée
-- une nouvelle version.

CREATE TABLE appreciation (
    inscription_id    TEXT NOT NULL REFERENCES inscription(id),
    periode_id        TEXT NOT NULL REFERENCES periode_academique(id),
    -- Identifiant de la matière, ou « generale » pour l'appréciation de la direction.
    matiere_cle       TEXT NOT NULL,
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    texte             TEXT NOT NULL CHECK (length(texte) <= 300),
    saisie_le         INTEGER NOT NULL,
    saisie_par        TEXT,
    identifiant_poste TEXT NOT NULL,
    PRIMARY KEY (inscription_id, periode_id, matiere_cle)
);

CREATE TABLE bulletin (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    inscription_id    TEXT NOT NULL REFERENCES inscription(id),
    periode_id        TEXT NOT NULL REFERENCES periode_academique(id),
    classe_id         TEXT NOT NULL REFERENCES classe(id),
    version           INTEGER NOT NULL CHECK (version >= 1),
    statut            TEXT NOT NULL CHECK (statut IN ('calcule', 'valide', 'publie', 'remplace')),
    contenu           TEXT NOT NULL,
    moyenne           INTEGER,
    rang              INTEGER,
    ex_aequo          INTEGER NOT NULL DEFAULT 0 CHECK (ex_aequo IN (0, 1)),
    regle_version     INTEGER NOT NULL,
    motif_correction  TEXT NOT NULL DEFAULT '',
    calcule_le        INTEGER NOT NULL,
    calcule_par       TEXT,
    valide_le         INTEGER,
    valide_par        TEXT,
    publie_le         INTEGER,
    publie_par        TEXT,
    mis_a_jour_le     INTEGER,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    UNIQUE (inscription_id, periode_id, version)
);
CREATE INDEX bulletin_classe ON bulletin (classe_id, periode_id);

CREATE TRIGGER bulletin_sans_suppression BEFORE DELETE ON bulletin
BEGIN SELECT RAISE(ABORT, 'un bulletin ne se supprime pas'); END;

CREATE TRIGGER bulletin_publie_immuable BEFORE UPDATE OF contenu, moyenne, rang, ex_aequo ON bulletin
WHEN OLD.statut IN ('publie', 'remplace')
BEGIN SELECT RAISE(ABORT, 'un bulletin publié ne change plus'); END;
