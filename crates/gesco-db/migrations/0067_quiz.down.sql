-- Descente de la 0067 : les tentatives disparaissent ; les quiz de chapitre ne reviennent pas dans
-- l'ancienne table, qui ne les accepte pas.
DROP TRIGGER IF EXISTS reponse_quiz_sans_suppression;
DROP TABLE IF EXISTS reponse_quiz;
DROP TRIGGER IF EXISTS tentative_quiz_sans_suppression;
DROP TABLE IF EXISTS tentative_quiz;

PRAGMA defer_foreign_keys = ON;
CREATE TABLE contenu_attente AS SELECT * FROM contenu WHERE nature <> 'quiz';
DROP TRIGGER contenu_sans_suppression;
DROP INDEX contenu_cle;
DROP INDEX contenu_chapitre;
DROP TABLE contenu;

CREATE TABLE contenu (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    cle               TEXT NOT NULL CHECK (length(trim(cle)) BETWEEN 1 AND 300),
    origine           TEXT NOT NULL CHECK (origine IN ('livre', 'ecole')),
    nature            TEXT NOT NULL CHECK (nature IN ('lecon', 'sujet', 'corrige', 'document')),
    chapitre_id       TEXT NOT NULL REFERENCES chapitre_programme(id),
    titre             TEXT NOT NULL CHECK (length(trim(titre)) BETWEEN 1 AND 200),
    -- Phrase de présentation d'un document de l'école.
    presentation      TEXT NOT NULL DEFAULT '',
    ordre             INTEGER NOT NULL,
    apres_cle         TEXT,
    langue            TEXT NOT NULL DEFAULT 'fr' CHECK (length(langue) BETWEEN 2 AND 8),
    -- Le corrigé d'un sujet, et l'inverse, par sa clé.
    lie_cle           TEXT,
    examen            TEXT CHECK (examen IS NULL OR examen IN ('def', 'bac')),
    serie_code        TEXT,
    session           INTEGER CHECK (session IS NULL OR session BETWEEN 1950 AND 2200),
    paquet_code       TEXT,
    fichier           TEXT,
    empreinte_sha256  TEXT CHECK (empreinte_sha256 IS NULL OR length(empreinte_sha256) = 64),
    -- Liste de faits à cocher jointe à une leçon livrée : nom du fichier dans le paquet.
    faits             TEXT,
    plus_livre_le     INTEGER,
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    CHECK (origine = 'ecole' OR (paquet_code IS NOT NULL AND fichier IS NOT NULL
        AND empreinte_sha256 IS NOT NULL))
);

INSERT INTO contenu SELECT * FROM contenu_attente;
DROP TABLE contenu_attente;

CREATE UNIQUE INDEX contenu_cle ON contenu (etablissement_id, cle);
CREATE INDEX contenu_chapitre ON contenu (chapitre_id, ordre);

CREATE TRIGGER contenu_sans_suppression BEFORE DELETE ON contenu
BEGIN SELECT RAISE(ABORT, 'un contenu de la bibliothèque ne se supprime pas'); END;
