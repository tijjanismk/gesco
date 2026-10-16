-- Engagements du personnel (SPEC_PHASE_6 §3) : ce qui lie chaque membre à l'école, écrit ou non.
-- Un changement de salaire ou de taux est un nouvel engagement qui succède au précédent à une
-- date (Q-65) ; un engagement ne se supprime jamais, il se termine ou se rompt.

CREATE TABLE engagement (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    membre_id         TEXT NOT NULL REFERENCES membre_personnel(id),
    nature            TEXT NOT NULL CHECK (nature IN ('duree_indeterminee', 'duree_determinee', 'vacation', 'stage', 'benevole')),
    debut             TEXT NOT NULL CHECK (length(debut) = 10),
    fin               TEXT CHECK (fin IS NULL OR length(fin) = 10),
    fonction          TEXT NOT NULL CHECK (length(trim(fonction)) > 0),
    -- Rémunération, en JSON : {"type":"mensuelle","montant":…}, {"type":"horaire",…}.
    remuneration      TEXT NOT NULL,
    statut            TEXT NOT NULL CHECK (statut IN ('en_cours', 'termine', 'rompu')),
    arrete_le         TEXT CHECK (arrete_le IS NULL OR length(arrete_le) = 10),
    motif_fin         TEXT NOT NULL DEFAULT '',
    precedent_id      TEXT REFERENCES engagement(id),
    note              TEXT NOT NULL DEFAULT '',
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    -- Une durée déterminée et un stage ont leur fin.
    CHECK (nature NOT IN ('duree_determinee', 'stage') OR fin IS NOT NULL),
    -- Un engagement clos a son dernier jour.
    CHECK ((statut = 'en_cours') = (arrete_le IS NULL))
);
CREATE INDEX engagement_membre ON engagement (membre_id, debut);

CREATE TRIGGER engagement_sans_suppression BEFORE DELETE ON engagement
BEGIN SELECT RAISE(ABORT, 'un engagement ne se supprime pas'); END;
