-- Résultats aux examens (SPEC_PHASE_9 §8.5) : élève par élève, ou en totaux par sexe pour l'école
-- qui préfère. Tout est facultatif : une case vide n'est ni « non » ni zéro. Quand un examen a des
-- résultats par élève dans l'année, ce détail l'emporte sur les totaux. Rien ne se supprime
-- (R-14) : une ligne se vide.

CREATE TABLE resultat_examen (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    inscription_id    TEXT NOT NULL UNIQUE REFERENCES inscription(id),
    examen            TEXT NOT NULL CHECK (examen IN ('entree_7e', 'def', 'bac', 'cap', 'bt')),
    inscrit           INTEGER CHECK (inscrit IS NULL OR inscrit IN (0, 1)),
    present           INTEGER CHECK (present IS NULL OR present IN (0, 1)),
    admis             INTEGER CHECK (admis IS NULL OR admis IN (0, 1)),
    mention           TEXT NOT NULL DEFAULT '',
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE INDEX resultat_examen_etablissement ON resultat_examen (etablissement_id);

CREATE TRIGGER resultat_examen_sans_suppression BEFORE DELETE ON resultat_examen
BEGIN SELECT RAISE(ABORT, 'un résultat d''examen ne se supprime pas'); END;

CREATE TABLE total_examen (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    annee_id          TEXT NOT NULL REFERENCES annee_scolaire(id),
    examen            TEXT NOT NULL CHECK (examen IN ('entree_7e', 'def', 'bac', 'cap', 'bt')),
    sexe              TEXT NOT NULL CHECK (sexe IN ('masculin', 'feminin')),
    inscrits          INTEGER CHECK (inscrits IS NULL OR inscrits >= 0),
    presentes         INTEGER CHECK (presentes IS NULL OR presentes >= 0),
    admis             INTEGER CHECK (admis IS NULL OR admis >= 0),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE UNIQUE INDEX total_examen_unique ON total_examen (annee_id, examen, sexe);

CREATE TRIGGER total_examen_sans_suppression BEFORE DELETE ON total_examen
BEGIN SELECT RAISE(ABORT, 'un total d''examen ne se supprime pas'); END;
