-- Bibliothèque de cours, phase 12, lot 3 : les quiz. `contenu` est reconstruite pour accepter la
-- nature `quiz` (un quiz de chapitre écrit par l'école ; SQLite ne modifie pas une contrainte),
-- avec toutes ses lignes, ses index et son déclencheur. L'index plein texte
-- `recherche_bibliotheque` n'a pas de déclencheur sur `contenu` : il est tenu par le code et ne
-- bouge pas. Les tentatives des élèves vivent par année scolaire et ne se suppriment pas.

-- Reconstruction de `contenu`. Les contraintes de clé étrangère sont différées jusqu'à la fin de
-- la migration : retirer l'ancienne table compte les lignes qui la désignent (versions, fichiers,
-- relectures, avancement…), les remettre dans la nouvelle les décompte.
PRAGMA defer_foreign_keys = ON;

CREATE TABLE contenu_attente AS SELECT * FROM contenu;

DROP TRIGGER contenu_sans_suppression;
DROP INDEX contenu_cle;
DROP INDEX contenu_chapitre;
DROP TABLE contenu;

CREATE TABLE contenu (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    cle               TEXT NOT NULL CHECK (length(trim(cle)) BETWEEN 1 AND 300),
    origine           TEXT NOT NULL CHECK (origine IN ('livre', 'ecole')),
    nature            TEXT NOT NULL CHECK (nature IN ('lecon', 'sujet', 'corrige', 'document', 'quiz')),
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

INSERT INTO contenu (id, etablissement_id, cle, origine, nature, chapitre_id, titre, presentation,
    ordre, apres_cle, langue, lie_cle, examen, serie_code, session, paquet_code, fichier,
    empreinte_sha256, faits, plus_livre_le, cree_le, cree_par, mis_a_jour_le, mis_a_jour_par,
    identifiant_poste)
SELECT id, etablissement_id, cle, origine, nature, chapitre_id, titre, presentation,
    ordre, apres_cle, langue, lie_cle, examen, serie_code, session, paquet_code, fichier,
    empreinte_sha256, faits, plus_livre_le, cree_le, cree_par, mis_a_jour_le, mis_a_jour_par,
    identifiant_poste
FROM contenu_attente;

DROP TABLE contenu_attente;

CREATE UNIQUE INDEX contenu_cle ON contenu (etablissement_id, cle);
CREATE INDEX contenu_chapitre ON contenu (chapitre_id, ordre);

CREATE TRIGGER contenu_sans_suppression BEFORE DELETE ON contenu
BEGIN SELECT RAISE(ABORT, 'un contenu de la bibliothèque ne se supprime pas'); END;

-- Un quiz passé jusqu'au bout sur le portail par un élève entré avec son code, suivi allumé.
-- `contenu_id` : le quiz d'une leçon ou d'un quiz de chapitre ; `chapitre_id` : une révision du
-- chapitre (questions tirées de plusieurs leçons, chacune avec son contenu dans `reponse_quiz`).
-- `empreinte_version` : la version passée (pour une révision, celle de la première question).
CREATE TABLE tentative_quiz (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    eleve_id          TEXT NOT NULL REFERENCES eleve(id),
    annee_id          TEXT NOT NULL REFERENCES annee_scolaire(id),
    contenu_id        TEXT REFERENCES contenu(id),
    chapitre_id       TEXT REFERENCES chapitre_programme(id),
    empreinte_version TEXT NOT NULL CHECK (length(empreinte_version) > 0),
    origine           TEXT NOT NULL DEFAULT 'portail' CHECK (origine IN ('portail')),
    commencee_le      INTEGER NOT NULL,
    finie_le          INTEGER NOT NULL,
    score             INTEGER NOT NULL CHECK (score >= 0),
    sur               INTEGER NOT NULL CHECK (sur > 0 AND score <= sur),
    CHECK ((contenu_id IS NULL) <> (chapitre_id IS NULL))
);
CREATE INDEX tentative_quiz_contenu ON tentative_quiz (etablissement_id, annee_id, contenu_id);
CREATE INDEX tentative_quiz_eleve ON tentative_quiz (eleve_id, annee_id, finie_le);
CREATE TRIGGER tentative_quiz_sans_suppression BEFORE DELETE ON tentative_quiz
BEGIN SELECT RAISE(ABORT, 'une tentative de quiz ne se supprime pas'); END;

-- Chaque réponse d'une tentative, avec la version de la question telle qu'elle a été posée.
CREATE TABLE reponse_quiz (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    tentative_id      TEXT NOT NULL REFERENCES tentative_quiz(id),
    contenu_id        TEXT NOT NULL REFERENCES contenu(id),
    empreinte_version TEXT NOT NULL CHECK (length(empreinte_version) > 0),
    numero            INTEGER NOT NULL CHECK (numero >= 0),
    reponse           TEXT NOT NULL DEFAULT '',
    juste             INTEGER NOT NULL CHECK (juste IN (0, 1))
);
CREATE INDEX reponse_quiz_tentative ON reponse_quiz (tentative_id);
CREATE INDEX reponse_quiz_question ON reponse_quiz (contenu_id, empreinte_version, numero);
CREATE TRIGGER reponse_quiz_sans_suppression BEFORE DELETE ON reponse_quiz
BEGIN SELECT RAISE(ABORT, 'une réponse de quiz ne se supprime pas'); END;
