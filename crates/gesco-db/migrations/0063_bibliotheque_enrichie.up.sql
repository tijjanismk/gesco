-- Bibliothèque de cours, phase 12, lot 2 : sons, ajout de l'école réservé à certaines classes,
-- leçons reçues d'une autre école. Rien de ce qui existe ne se perd : `fichier_bibliotheque` est
-- reconstruite pour accepter la nature `son` (SQLite ne modifie pas une contrainte), avec toutes
-- ses lignes, son index unique et son déclencheur. Aucune table n'a d'année scolaire, sauf par la
-- classe choisie : une classe appartient à une année.

-- Reconstruction de `fichier_bibliotheque`. Les contraintes de clé étrangère sont différées
-- jusqu'à la fin de la migration : retirer l'ancienne table compte les lignes de
-- `fichier_contenu` qui la désignent, les remettre dans la nouvelle les décompte. La copie
-- passe par une table d'attente, sans lien, pour que les lignes reviennent sous le même nom.
PRAGMA defer_foreign_keys = ON;

CREATE TABLE fichier_bibliotheque_attente AS SELECT * FROM fichier_bibliotheque;

DROP TRIGGER fichier_bibliotheque_sans_suppression;
DROP INDEX fichier_bibliotheque_empreinte;
DROP TABLE fichier_bibliotheque;

-- Image, document ou son joint par l'école, dans la base chiffrée, gardé une fois par empreinte.
CREATE TABLE fichier_bibliotheque (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    empreinte_sha256  TEXT NOT NULL CHECK (length(empreinte_sha256) = 64),
    nature            TEXT NOT NULL CHECK (nature IN ('image', 'document', 'son')),
    type_mime         TEXT NOT NULL CHECK (type_mime IN ('image/svg+xml', 'image/png',
                          'image/jpeg', 'application/pdf', 'audio/mpeg', 'audio/mp4',
                          'audio/ogg', 'audio/wav')),
    nom_fichier       TEXT NOT NULL,
    taille            INTEGER NOT NULL CHECK (taille > 0),
    contenu           BLOB NOT NULL,
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);

INSERT INTO fichier_bibliotheque (id, etablissement_id, empreinte_sha256, nature, type_mime,
    nom_fichier, taille, contenu, cree_le, cree_par, mis_a_jour_le, mis_a_jour_par,
    identifiant_poste)
SELECT id, etablissement_id, empreinte_sha256, nature, type_mime, nom_fichier, taille, contenu,
    cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste
FROM fichier_bibliotheque_attente;

DROP TABLE fichier_bibliotheque_attente;

CREATE UNIQUE INDEX fichier_bibliotheque_empreinte
    ON fichier_bibliotheque (etablissement_id, empreinte_sha256);

CREATE TRIGGER fichier_bibliotheque_sans_suppression BEFORE DELETE ON fichier_bibliotheque
BEGIN SELECT RAISE(ABORT, 'un fichier de la bibliothèque ne se supprime pas'); END;

-- Un ajout de l'école (chapitre ou contenu) réservé à certaines classes. Aucune ligne active
-- pour l'année d'une classe : tout le niveau. Décocher une classe remplit `retiree_le` ; la
-- recocher crée une ligne nouvelle. Une classe d'une année passée ne compte plus pour l'année
-- suivante : le choix ancien reste dans l'historique.
CREATE TABLE contenu_classe (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    contenu_id        TEXT REFERENCES contenu(id),
    chapitre_id       TEXT REFERENCES chapitre_programme(id),
    classe_id         TEXT NOT NULL REFERENCES classe(id),
    retiree_le        INTEGER,
    retiree_par       TEXT REFERENCES utilisateur(id),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    CHECK ((contenu_id IS NULL) <> (chapitre_id IS NULL))
);
CREATE UNIQUE INDEX contenu_classe_active_contenu ON contenu_classe (contenu_id, classe_id)
    WHERE retiree_le IS NULL AND contenu_id IS NOT NULL;
CREATE UNIQUE INDEX contenu_classe_active_chapitre ON contenu_classe (chapitre_id, classe_id)
    WHERE retiree_le IS NULL AND chapitre_id IS NOT NULL;
CREATE INDEX contenu_classe_classe ON contenu_classe (classe_id) WHERE retiree_le IS NULL;

CREATE TRIGGER contenu_classe_sans_suppression BEFORE DELETE ON contenu_classe
BEGIN SELECT RAISE(ABORT, 'un choix de classes ne se supprime pas'); END;

-- Un fichier de leçons reçu d'une autre école (`.gles`), par son empreinte.
CREATE TABLE import_lecons (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    empreinte_sha256  TEXT NOT NULL CHECK (length(empreinte_sha256) = 64),
    nom_fichier       TEXT NOT NULL,
    importe_le        INTEGER NOT NULL,
    importe_par       TEXT NOT NULL REFERENCES utilisateur(id),
    nombre_lecons     INTEGER NOT NULL CHECK (nombre_lecons >= 0),
    nombre_ignorees   INTEGER NOT NULL CHECK (nombre_ignorees >= 0),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE INDEX import_lecons_quand ON import_lecons (etablissement_id, importe_le);

CREATE TRIGGER import_lecons_sans_suppression BEFORE DELETE ON import_lecons
BEGIN SELECT RAISE(ABORT, 'un import de leçons ne se supprime pas'); END;

-- Version d'une leçon reçue alors que l'école en avait déjà un brouillon : elle ne remplace
-- rien, elle se compare et l'on peut en repartir.
CREATE TABLE version_recue (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    contenu_id        TEXT NOT NULL REFERENCES contenu(id),
    texte             TEXT NOT NULL,
    empreinte_sha256  TEXT NOT NULL CHECK (length(empreinte_sha256) = 64),
    import_id         TEXT NOT NULL REFERENCES import_lecons(id),
    recue_le          INTEGER NOT NULL,
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE INDEX version_recue_contenu ON version_recue (contenu_id, empreinte_sha256);

CREATE TRIGGER version_recue_sans_suppression BEFORE DELETE ON version_recue
BEGIN SELECT RAISE(ABORT, 'une version reçue ne se supprime pas'); END;

-- Un brouillon né d'un import garde son import : « Publier les leçons reçues de cet import ».
-- Sans clé étrangère déclarée : SQLite ne retirerait plus la colonne à la descente.
ALTER TABLE version_contenu ADD COLUMN import_id TEXT;
CREATE INDEX version_contenu_import ON version_contenu (import_id) WHERE import_id IS NOT NULL;
