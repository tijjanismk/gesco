-- Note de conduite (porteur du projet, 24 octobre 2026) : la note saisie d'un élève pour une
-- période, ou la correction d'une note calculée. Rien ne se supprime (R-14) : une note saisie par
-- erreur, ou remplacée, est retirée avec un motif ; une seule note en vigueur par élève et par
-- période.

CREATE TABLE note_conduite (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    inscription_id    TEXT NOT NULL REFERENCES inscription(id),
    periode_id        TEXT NOT NULL REFERENCES periode_academique(id),
    note              INTEGER NOT NULL CHECK (note BETWEEN 0 AND 2000),
    motif             TEXT NOT NULL DEFAULT '' CHECK (length(motif) <= 200),
    saisie_par        TEXT NOT NULL REFERENCES utilisateur(id),
    annulee           INTEGER NOT NULL DEFAULT 0 CHECK (annulee IN (0, 1)),
    motif_annulation  TEXT NOT NULL DEFAULT '',
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    CHECK (annulee = 0 OR length(trim(motif_annulation)) > 0)
);
CREATE UNIQUE INDEX note_conduite_en_vigueur ON note_conduite (inscription_id, periode_id)
    WHERE annulee = 0;
CREATE INDEX note_conduite_periode ON note_conduite (periode_id);

CREATE TRIGGER note_conduite_sans_suppression BEFORE DELETE ON note_conduite
BEGIN SELECT RAISE(ABORT, 'une note de conduite ne se supprime pas'); END;
