-- Départs et exclusions (SPEC_PHASE_4 §9, réponse du 13 octobre 2026). Une exclusion ne se
-- supprime jamais : levée ou révoquée, elle reste dans l'historique. Le certificat de transfert
-- garde son numéro (CTR-2026-000001), réimprimable à l'identique.

CREATE TABLE exclusion (
    id                 TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id   TEXT NOT NULL REFERENCES etablissement(id),
    inscription_id     TEXT NOT NULL REFERENCES inscription(id),
    nature             TEXT NOT NULL CHECK (nature IN ('temporaire', 'definitive')),
    du                 TEXT NOT NULL CHECK (length(du) = 10),
    au                 TEXT CHECK (au IS NULL OR length(au) = 10),
    motif              TEXT NOT NULL,
    decision           TEXT NOT NULL DEFAULT '',
    statut             TEXT NOT NULL CHECK (statut IN ('en_cours', 'levee', 'revoquee')),
    leve_le            TEXT CHECK (leve_le IS NULL OR length(leve_le) = 10),
    motif_levee        TEXT NOT NULL DEFAULT '',
    echeances_annulees TEXT NOT NULL DEFAULT '[]',
    cree_le            INTEGER NOT NULL,
    cree_par           TEXT,
    mis_a_jour_le      INTEGER NOT NULL,
    mis_a_jour_par     TEXT,
    identifiant_poste  TEXT NOT NULL,
    -- Une exclusion temporaire a sa date de retour, une définitive n'en a pas.
    CHECK ((nature = 'temporaire') = (au IS NOT NULL))
);
CREATE INDEX exclusion_inscription ON exclusion (inscription_id);

CREATE TRIGGER exclusion_sans_suppression BEFORE DELETE ON exclusion
BEGIN SELECT RAISE(ABORT, 'une exclusion ne se supprime pas'); END;

CREATE TABLE certificat_transfert (
    inscription_id    TEXT PRIMARY KEY REFERENCES inscription(id),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    numero            TEXT NOT NULL UNIQUE,
    delivre_le        INTEGER NOT NULL,
    delivre_par       TEXT,
    identifiant_poste TEXT NOT NULL
);
