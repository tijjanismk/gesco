-- Import des relevés (SPEC_VERSION_1_1 §1, étape 7, T-72) : les colonnes du relevé de chaque
-- compte, dites une fois et gardées. Rien ne se supprime (R-14).

CREATE TABLE format_releve (
    compte_id         TEXT PRIMARY KEY REFERENCES compte_tresorerie(id),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    contenu           TEXT NOT NULL CHECK (json_valid(contenu)),
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);

CREATE TRIGGER format_releve_sans_suppression BEFORE DELETE ON format_releve
BEGIN SELECT RAISE(ABORT, 'le format du relevé d''un compte ne se supprime pas'); END;
