-- Descente de la 0063 : les sons ne reviennent pas dans l'ancienne table, qui ne les accepte pas.
DROP INDEX IF EXISTS version_contenu_import;
ALTER TABLE version_contenu DROP COLUMN import_id;
DROP TRIGGER IF EXISTS version_recue_sans_suppression;
DROP TABLE IF EXISTS version_recue;
DROP TRIGGER IF EXISTS import_lecons_sans_suppression;
DROP TABLE IF EXISTS import_lecons;
DROP TRIGGER IF EXISTS contenu_classe_sans_suppression;
DROP TABLE IF EXISTS contenu_classe;

PRAGMA defer_foreign_keys = ON;
CREATE TABLE fichier_bibliotheque_attente AS
    SELECT * FROM fichier_bibliotheque WHERE nature <> 'son';
DROP TRIGGER fichier_bibliotheque_sans_suppression;
DROP INDEX fichier_bibliotheque_empreinte;
DROP TABLE fichier_bibliotheque;
CREATE TABLE fichier_bibliotheque (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    empreinte_sha256  TEXT NOT NULL CHECK (length(empreinte_sha256) = 64),
    nature            TEXT NOT NULL CHECK (nature IN ('image', 'document')),
    type_mime         TEXT NOT NULL CHECK (type_mime IN ('image/svg+xml', 'image/png',
                          'image/jpeg', 'application/pdf')),
    nom_fichier       TEXT NOT NULL,
    taille            INTEGER NOT NULL CHECK (taille > 0),
    contenu           BLOB NOT NULL,
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
INSERT INTO fichier_bibliotheque SELECT * FROM fichier_bibliotheque_attente;
DROP TABLE fichier_bibliotheque_attente;
CREATE UNIQUE INDEX fichier_bibliotheque_empreinte
    ON fichier_bibliotheque (etablissement_id, empreinte_sha256);
CREATE TRIGGER fichier_bibliotheque_sans_suppression BEFORE DELETE ON fichier_bibliotheque
BEGIN SELECT RAISE(ABORT, 'un fichier de la bibliothèque ne se supprime pas'); END;
