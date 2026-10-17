-- Discipline (SPEC_VERSION_1_1 §1, étape 3, T-70) : le dossier de discipline d'un élève. Rien ne
-- se supprime (R-14) : une sanction saisie par erreur s'annule avec un motif.

CREATE TABLE sanction (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    inscription_id    TEXT NOT NULL REFERENCES inscription(id),
    nature            TEXT NOT NULL CHECK (nature IN ('avertissement', 'blame', 'retenue', 'convocation_parents')),
    date              TEXT NOT NULL CHECK (length(date) = 10),
    motif             TEXT NOT NULL CHECK (length(trim(motif)) BETWEEN 1 AND 200),
    decidee_par       TEXT NOT NULL CHECK (length(trim(decidee_par)) BETWEEN 1 AND 200),
    periode_id        TEXT REFERENCES periode_academique(id),
    saisie_par        TEXT NOT NULL REFERENCES utilisateur(id),
    annulee           INTEGER NOT NULL DEFAULT 0 CHECK (annulee IN (0, 1)),
    motif_annulation  TEXT NOT NULL DEFAULT '',
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    CHECK (periode_id IS NULL OR nature IN ('avertissement', 'blame')),
    CHECK (annulee = 0 OR length(trim(motif_annulation)) > 0)
);
CREATE INDEX sanction_inscription ON sanction (inscription_id, date);

CREATE TRIGGER sanction_sans_suppression BEFORE DELETE ON sanction
BEGIN SELECT RAISE(ABORT, 'une sanction ne se supprime pas'); END;
