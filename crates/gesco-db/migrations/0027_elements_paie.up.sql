-- Éléments de paie de l'école (SPEC_PHASE_7 §3) : gains, retenues salariales et cotisations
-- patronales, chacun avec des versions datées de son calcul ; attributions à un membre. Les mois
-- payés d'un engagement mensuel (Q-73) et la situation de famille du membre (réduction d'impôt).

CREATE TABLE element_paie (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    -- Code stable d'un élément livré ; absent pour un élément créé par l'école.
    code              TEXT,
    libelle           TEXT NOT NULL CHECK (length(trim(libelle)) BETWEEN 1 AND 60),
    nature            TEXT NOT NULL CHECK (nature IN ('gain', 'retenue', 'cotisation_patronale')),
    -- Natures d'engagement concernées, en JSON : ["duree_indeterminee", "vacation"].
    natures           TEXT NOT NULL,
    fonctionnaires    INTEGER NOT NULL CHECK (fonctionnaires IN (0, 1)),
    ordre             INTEGER NOT NULL,
    actif             INTEGER NOT NULL CHECK (actif IN (0, 1)),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE UNIQUE INDEX element_paie_code ON element_paie (etablissement_id, code) WHERE code IS NOT NULL;

CREATE TABLE version_element (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    element_id        TEXT NOT NULL REFERENCES element_paie(id),
    -- Premier mois d'application, AAAA-MM.
    effet             TEXT NOT NULL CHECK (length(effet) = 7),
    -- Calcul, en JSON : {"mode":"taux","taux":360,"plafond":null}.
    calcul            TEXT NOT NULL,
    -- Retenues retranchées de l'assiette, en JSON : liste d'identifiants d'éléments.
    assiette_deduit   TEXT NOT NULL DEFAULT '[]',
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE UNIQUE INDEX version_element_effet ON version_element (element_id, effet);

CREATE TABLE attribution_element (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    membre_id         TEXT NOT NULL REFERENCES membre_personnel(id),
    element_id        TEXT NOT NULL REFERENCES element_paie(id),
    montant           INTEGER NOT NULL CHECK (montant > 0),
    debut             TEXT NOT NULL CHECK (length(debut) = 7),
    fin               TEXT CHECK (fin IS NULL OR (length(fin) = 7 AND fin >= debut)),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE INDEX attribution_element_membre ON attribution_element (etablissement_id, membre_id);

-- Rien ne se supprime (R-14) : un élément se retire, une attribution se termine.
CREATE TRIGGER element_paie_sans_suppression BEFORE DELETE ON element_paie
BEGIN SELECT RAISE(ABORT, 'un élément de paie ne se supprime pas'); END;
CREATE TRIGGER version_element_sans_suppression BEFORE DELETE ON version_element
BEGIN SELECT RAISE(ABORT, 'une version d''élément de paie ne se supprime pas'); END;
CREATE TRIGGER attribution_element_sans_suppression BEFORE DELETE ON attribution_element
BEGIN SELECT RAISE(ABORT, 'une attribution ne se supprime pas'); END;

-- Mois payés d'un engagement mensuel : douze, ou hors des mois de vacances de l'école.
ALTER TABLE engagement ADD COLUMN paye_douze_mois INTEGER NOT NULL DEFAULT 1
    CHECK (paye_douze_mois IN (0, 1));

-- Situation de famille, pour la réduction d'impôt pour charges de famille.
ALTER TABLE membre_personnel ADD COLUMN situation_famille TEXT
    CHECK (situation_famille IS NULL OR situation_famille IN ('celibataire', 'marie', 'veuf', 'divorce'));
ALTER TABLE membre_personnel ADD COLUMN enfants_a_charge INTEGER
    CHECK (enfants_a_charge IS NULL OR enfants_a_charge BETWEEN 0 AND 30);
