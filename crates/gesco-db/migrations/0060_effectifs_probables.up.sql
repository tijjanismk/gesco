-- Effectifs probables à la rentrée (SPEC_PHASE_9 §5.2, règle 9) : les seuls chiffres qui se
-- saisissent dans un rapport. GESCO propose les passants du niveau d'en dessous plus les
-- redoublants du niveau ; la direction corrige, surtout la 1re année. Une ligne par rapport et
-- par niveau ; une case vide reprend le nombre proposé. Rien ne se supprime (R-14).

CREATE TABLE effectif_probable (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    rapport_id        TEXT NOT NULL REFERENCES rapport_ecole(id),
    niveau_id         TEXT NOT NULL REFERENCES niveau(id),
    garcons           INTEGER CHECK (garcons IS NULL OR garcons BETWEEN 0 AND 100000),
    filles            INTEGER CHECK (filles IS NULL OR filles BETWEEN 0 AND 100000),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE UNIQUE INDEX effectif_probable_unique ON effectif_probable (rapport_id, niveau_id);

CREATE TRIGGER effectif_probable_sans_suppression BEFORE DELETE ON effectif_probable
BEGIN SELECT RAISE(ABORT, 'un effectif probable ne se supprime pas'); END;
