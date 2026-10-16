-- Enseignants et affectations (SPEC_PHASE_4 §3) : une personne, un statut, un compte facultatif ;
-- l'affectation d'un enseignant à une matière d'une classe pour une année décide où il peut
-- saisir des notes (§11.3 du blueprint). Plusieurs enseignants par matière d'une classe (T-22).

CREATE TABLE enseignant (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    personne_id       TEXT NOT NULL REFERENCES personne(id),
    statut            TEXT NOT NULL CHECK (statut IN ('titulaire', 'vacataire')),
    utilisateur_id    TEXT REFERENCES utilisateur(id),
    actif             INTEGER NOT NULL DEFAULT 1 CHECK (actif IN (0, 1)),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
-- Un compte ne représente qu'un enseignant.
CREATE UNIQUE INDEX enseignant_compte_unique ON enseignant (utilisateur_id) WHERE utilisateur_id IS NOT NULL;

CREATE TABLE affectation_enseignement (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    annee_id          TEXT NOT NULL REFERENCES annee_scolaire(id),
    classe_id         TEXT NOT NULL REFERENCES classe(id),
    matiere_id        TEXT NOT NULL REFERENCES matiere(id),
    enseignant_id     TEXT NOT NULL REFERENCES enseignant(id),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE UNIQUE INDEX affectation_unique ON affectation_enseignement (classe_id, matiere_id, enseignant_id);
CREATE INDEX affectation_enseignant ON affectation_enseignement (enseignant_id, annee_id);
CREATE INDEX affectation_annee ON affectation_enseignement (annee_id);
