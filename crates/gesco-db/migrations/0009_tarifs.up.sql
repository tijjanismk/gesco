-- Grilles de frais et plans de paiement (SPEC_PHASE_3 §3, blueprint §8.8). Une grille par
-- version et par année ; une seule en vigueur. Le contenu d'un brouillon se remplace par
-- suppression logique ; une grille en vigueur ou remplacée ne change plus.

CREATE TABLE grille_tarifaire (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    annee_id          TEXT NOT NULL REFERENCES annee_scolaire(id),
    libelle           TEXT NOT NULL,
    version           INTEGER NOT NULL CHECK (version >= 1),
    etat              TEXT NOT NULL CHECK (etat IN ('brouillon', 'en_vigueur', 'remplacee')),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE UNIQUE INDEX grille_version_unique ON grille_tarifaire (etablissement_id, annee_id, version);
CREATE UNIQUE INDEX grille_en_vigueur_unique
    ON grille_tarifaire (etablissement_id, annee_id) WHERE etat = 'en_vigueur';

CREATE TABLE ligne_tarifaire (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    grille_id         TEXT NOT NULL REFERENCES grille_tarifaire(id),
    ordre             INTEGER NOT NULL,
    niveau_id         TEXT NOT NULL REFERENCES niveau(id),
    serie_id          TEXT REFERENCES serie(id),
    type_frais        TEXT NOT NULL CHECK (type_frais IN ('inscription', 'scolarite', 'cantine', 'transport', 'tenue', 'examen', 'autre')),
    libelle           TEXT NOT NULL,
    montant           INTEGER NOT NULL CHECK (montant >= 0),
    obligatoire       INTEGER NOT NULL CHECK (obligatoire IN (0, 1)),
    exigibilite       TEXT NOT NULL CHECK (exigibilite IN ('une_fois', 'selon_plan')),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    identifiant_poste TEXT NOT NULL,
    supprime_le       INTEGER,
    supprime_par      TEXT
);
CREATE INDEX ligne_tarifaire_grille ON ligne_tarifaire (grille_id) WHERE supprime_le IS NULL;

CREATE TABLE plan_paiement (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    grille_id         TEXT NOT NULL REFERENCES grille_tarifaire(id),
    ordre             INTEGER NOT NULL,
    libelle           TEXT NOT NULL,
    par_defaut        INTEGER NOT NULL CHECK (par_defaut IN (0, 1)),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    identifiant_poste TEXT NOT NULL,
    supprime_le       INTEGER,
    supprime_par      TEXT
);
CREATE INDEX plan_paiement_grille ON plan_paiement (grille_id) WHERE supprime_le IS NULL;

CREATE TABLE tranche (
    plan_id           TEXT NOT NULL REFERENCES plan_paiement(id),
    rang              INTEGER NOT NULL CHECK (rang >= 1),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    libelle           TEXT NOT NULL,
    echeance          TEXT NOT NULL CHECK (length(echeance) = 10),
    part              INTEGER NOT NULL CHECK (part BETWEEN 1 AND 100),
    PRIMARY KEY (plan_id, rang)
);
