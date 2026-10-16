-- Cantine activable (SPEC_PHASE_8 §9, T-47) : les repas pointés chaque jour, quand l'école les
-- compte. Les abonnés se lisent dans les factures ; les vivres sont des dépenses. Rien ne se
-- supprime (R-14) : un repas pointé par erreur se décoche.

CREATE TABLE repas_cantine (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    inscription_id    TEXT NOT NULL REFERENCES inscription(id),
    date              TEXT NOT NULL CHECK (length(date) = 10),
    pris              INTEGER NOT NULL CHECK (pris IN (0, 1)),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE UNIQUE INDEX repas_cantine_jour ON repas_cantine (inscription_id, date);
CREATE INDEX repas_cantine_date ON repas_cantine (etablissement_id, date);

CREATE TRIGGER repas_cantine_sans_suppression BEFORE DELETE ON repas_cantine
BEGIN SELECT RAISE(ABORT, 'un repas pointé ne se supprime pas'); END;
