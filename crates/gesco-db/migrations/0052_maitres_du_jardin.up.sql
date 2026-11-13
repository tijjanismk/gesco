-- Maîtres des classes du jardin (SPEC_PHASE_11_B §7) : le jardin n'a pas de matière, l'affectation
-- d'un enseignant à une classe du jardin se fait donc sans matière. Elle décide, quand l'école
-- ouvre la saisie aux maîtres, quels enfants il observe. Comme une affectation, elle se retire.

CREATE TABLE maitre_jardin (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    annee_id          TEXT NOT NULL REFERENCES annee_scolaire(id),
    classe_id         TEXT NOT NULL REFERENCES classe(id),
    enseignant_id     TEXT NOT NULL REFERENCES enseignant(id),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE UNIQUE INDEX maitre_jardin_unique ON maitre_jardin (classe_id, enseignant_id);
CREATE INDEX maitre_jardin_enseignant ON maitre_jardin (enseignant_id, annee_id);
CREATE INDEX maitre_jardin_annee ON maitre_jardin (annee_id);
