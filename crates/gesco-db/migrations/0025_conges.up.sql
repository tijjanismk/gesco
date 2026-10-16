-- Absences, retards et congés du personnel (SPEC_PHASE_6 §5 et §6, T-43, T-44) : les absences et
-- les retards se notent sans pointage ; un congé se demande, la direction l'accorde ou le refuse.
-- Rien ne se supprime : une absence notée par erreur s'annule, un congé accordé s'annule avant son
-- début, un type de congé se retire (R-14).

CREATE TABLE absence_personnel (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    membre_id         TEXT NOT NULL REFERENCES membre_personnel(id),
    date              TEXT NOT NULL CHECK (length(date) = 10),
    nature            TEXT NOT NULL CHECK (nature IN ('journee', 'demi_journee', 'retard')),
    minutes           INTEGER CHECK (minutes IS NULL OR minutes BETWEEN 1 AND 240),
    motif             TEXT NOT NULL DEFAULT '',
    justifiee         INTEGER NOT NULL DEFAULT 0 CHECK (justifiee IN (0, 1)),
    annulee           INTEGER NOT NULL DEFAULT 0 CHECK (annulee IN (0, 1)),
    motif_annulation  TEXT NOT NULL DEFAULT '',
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    -- Seul un retard a une durée ; une annulation a son motif.
    CHECK (nature = 'retard' OR minutes IS NULL),
    CHECK (annulee = 0 OR length(trim(motif_annulation)) > 0)
);
CREATE INDEX absence_personnel_date ON absence_personnel (etablissement_id, date);
CREATE INDEX absence_personnel_membre ON absence_personnel (membre_id, date);

CREATE TABLE type_conge (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    code              TEXT,
    libelle           TEXT NOT NULL CHECK (length(trim(libelle)) > 0),
    ordre             INTEGER NOT NULL,
    actif             INTEGER NOT NULL DEFAULT 1 CHECK (actif IN (0, 1)),
    deduit_du_solde   INTEGER NOT NULL DEFAULT 0 CHECK (deduit_du_solde IN (0, 1)),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE UNIQUE INDEX type_conge_code ON type_conge (etablissement_id, code) WHERE code IS NOT NULL;

CREATE TABLE conge (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    membre_id         TEXT NOT NULL REFERENCES membre_personnel(id),
    type_id           TEXT NOT NULL REFERENCES type_conge(id),
    debut             TEXT NOT NULL CHECK (length(debut) = 10),
    fin               TEXT NOT NULL CHECK (length(fin) = 10),
    jours             INTEGER NOT NULL CHECK (jours > 0),
    motif             TEXT NOT NULL DEFAULT '',
    statut            TEXT NOT NULL CHECK (statut IN ('demande', 'accorde', 'refuse', 'annule')),
    decide_le         INTEGER,
    decide_par        TEXT,
    motif_refus       TEXT NOT NULL DEFAULT '',
    motif_annulation  TEXT NOT NULL DEFAULT '',
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    CHECK (fin >= debut),
    -- Un refus et une annulation ont leur motif.
    CHECK (statut <> 'refuse' OR length(trim(motif_refus)) > 0),
    CHECK (statut <> 'annule' OR length(trim(motif_annulation)) > 0)
);
CREATE INDEX conge_membre ON conge (membre_id, debut);
CREATE INDEX conge_periode ON conge (etablissement_id, debut, fin);

CREATE TRIGGER absence_personnel_sans_suppression BEFORE DELETE ON absence_personnel
BEGIN SELECT RAISE(ABORT, 'une absence du personnel ne se supprime pas'); END;
CREATE TRIGGER type_conge_sans_suppression BEFORE DELETE ON type_conge
BEGIN SELECT RAISE(ABORT, 'un type de congé ne se supprime pas'); END;
CREATE TRIGGER conge_sans_suppression BEFORE DELETE ON conge
BEGIN SELECT RAISE(ABORT, 'un congé ne se supprime pas'); END;
