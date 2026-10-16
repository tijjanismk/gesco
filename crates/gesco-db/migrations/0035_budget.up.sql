-- Budget de l'année scolaire (SPEC_PHASE_8 §7) : un montant prévu par catégorie de recette et de
-- dépense ; le réalisé se calcule, un dépassement avertit sans jamais bloquer. Rien ne se supprime
-- (R-14) : une ligne se remet à zéro.

CREATE TABLE budget_tresorerie (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    annee_id          TEXT NOT NULL REFERENCES annee_scolaire(id),
    categorie_id      TEXT NOT NULL REFERENCES categorie_tresorerie(id),
    montant           INTEGER NOT NULL CHECK (montant >= 0),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE UNIQUE INDEX budget_tresorerie_ligne ON budget_tresorerie (annee_id, categorie_id);

CREATE TRIGGER budget_tresorerie_sans_suppression BEFORE DELETE ON budget_tresorerie
BEGIN SELECT RAISE(ABORT, 'une ligne de budget ne se supprime pas'); END;
