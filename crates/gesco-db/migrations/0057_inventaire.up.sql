-- Inventaire de l'école (SPEC_PHASE_9 §8.4) : un catalogue d'articles, livrés ou ajoutés par
-- l'école, et un relevé par année, par article et, au choix, par cycle (vide pour toute l'école).
-- Toutes les quantités sont facultatives : vide n'est pas zéro. Rien ne se supprime (R-14) : un
-- article se retire ; une ligne de relevé se vide.

CREATE TABLE article_inventaire (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    code              TEXT,
    famille           TEXT NOT NULL CHECK (famille IN ('local', 'mobilier', 'materiel', 'manuel',
                          'adapte')),
    designation       TEXT NOT NULL CHECK (length(trim(designation)) BETWEEN 1 AND 120),
    matiere           TEXT NOT NULL DEFAULT '',
    niveau_id         TEXT REFERENCES niveau(id),
    livre             INTEGER NOT NULL DEFAULT 0 CHECK (livre IN (0, 1)),
    ordre             INTEGER NOT NULL,
    retire_le         INTEGER,
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE INDEX article_inventaire_etablissement ON article_inventaire (etablissement_id, famille, ordre);
CREATE UNIQUE INDEX article_inventaire_code ON article_inventaire (etablissement_id, code)
    WHERE code IS NOT NULL;

CREATE TRIGGER article_inventaire_sans_suppression BEFORE DELETE ON article_inventaire
BEGIN SELECT RAISE(ABORT, 'un article de l''inventaire ne se supprime pas'); END;

CREATE TABLE releve_inventaire (
    id                  TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id    TEXT NOT NULL REFERENCES etablissement(id),
    annee_id            TEXT NOT NULL REFERENCES annee_scolaire(id),
    article_id          TEXT NOT NULL REFERENCES article_inventaire(id),
    cycle               TEXT CHECK (cycle IS NULL OR cycle IN ('jardin', 'premier_cycle',
                            'second_cycle', 'lycee', 'technique')),
    existant            INTEGER CHECK (existant IS NULL OR existant >= 0),
    bon_etat            INTEGER CHECK (bon_etat IS NULL OR bon_etat >= 0),
    besoins             INTEGER CHECK (besoins IS NULL OR besoins >= 0),
    dur                 INTEGER CHECK (dur IS NULL OR dur >= 0),
    semi_dur            INTEGER CHECK (semi_dur IS NULL OR semi_dur >= 0),
    banco               INTEGER CHECK (banco IS NULL OR banco >= 0),
    autre_construction  INTEGER CHECK (autre_construction IS NULL OR autre_construction >= 0),
    neufs               INTEGER CHECK (neufs IS NULL OR neufs >= 0),
    en_location         INTEGER CHECK (en_location IS NULL OR en_location >= 0),
    murs_bons           INTEGER CHECK (murs_bons IS NULL OR murs_bons >= 0),
    murs_mauvais        INTEGER CHECK (murs_mauvais IS NULL OR murs_mauvais >= 0),
    toits_bons          INTEGER CHECK (toits_bons IS NULL OR toits_bons >= 0),
    toits_mauvais       INTEGER CHECK (toits_mauvais IS NULL OR toits_mauvais >= 0),
    cabines_filles      INTEGER CHECK (cabines_filles IS NULL OR cabines_filles >= 0),
    cabines_garcons     INTEGER CHECK (cabines_garcons IS NULL OR cabines_garcons >= 0),
    cabines_enseignants INTEGER CHECK (cabines_enseignants IS NULL OR cabines_enseignants >= 0),
    observation         TEXT NOT NULL DEFAULT '',
    cree_le             INTEGER NOT NULL,
    cree_par            TEXT,
    mis_a_jour_le       INTEGER NOT NULL,
    mis_a_jour_par      TEXT,
    identifiant_poste   TEXT NOT NULL
);
CREATE UNIQUE INDEX releve_inventaire_unique
    ON releve_inventaire (annee_id, article_id, ifnull(cycle, ''));
CREATE INDEX releve_inventaire_etablissement ON releve_inventaire (etablissement_id, annee_id);

CREATE TRIGGER releve_inventaire_sans_suppression BEFORE DELETE ON releve_inventaire
BEGIN SELECT RAISE(ABORT, 'une ligne de l''inventaire ne se supprime pas'); END;
