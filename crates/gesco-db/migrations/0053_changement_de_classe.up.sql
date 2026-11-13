-- Changement de classe en cours d'année (décisions du porteur du 2026-11-13).
--
-- Notes : l'élève compte dans sa nouvelle classe à partir du jour du changement ; les notes prises
-- avant restent dans son historique, le bulletin de la nouvelle classe ne compte que les siennes.
ALTER TABLE inscription ADD COLUMN classe_depuis TEXT
    CHECK (classe_depuis IS NULL OR length(classe_depuis) = 10);

-- Facture : la direction garde les frais actuels ou applique ceux de la nouvelle classe. Rien ne se
-- réécrit : l'écart s'enregistre en ajustement daté, une hausse en ligne de facture en plus, une
-- baisse en parts portées par les échéances à venir, le trop-perçu en avoir de l'élève.
CREATE TABLE ajustement_facture (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    facture_id        TEXT NOT NULL REFERENCES facture(id),
    date_ajustement   TEXT NOT NULL CHECK (length(date_ajustement) = 10),
    hausse            INTEGER NOT NULL CHECK (hausse >= 0),
    baisse            INTEGER NOT NULL CHECK (baisse >= 0),
    avoir             INTEGER NOT NULL CHECK (avoir >= 0 AND avoir <= baisse),
    classe_avant_id   TEXT NOT NULL REFERENCES classe(id),
    classe_apres_id   TEXT NOT NULL REFERENCES classe(id),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    identifiant_poste TEXT NOT NULL,
    CHECK (hausse > 0 OR baisse > 0)
);
CREATE INDEX ajustement_facture_facture ON ajustement_facture (facture_id);

-- Ligne de facture née d'une hausse.
ALTER TABLE ligne_facture ADD COLUMN ajustement_id TEXT REFERENCES ajustement_facture(id);

-- Baisse portée par une échéance due, comme une part de remise.
CREATE TABLE ajustement_echeance (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    ajustement_id     TEXT NOT NULL REFERENCES ajustement_facture(id),
    echeance_id       TEXT NOT NULL REFERENCES echeance(id),
    montant           INTEGER NOT NULL CHECK (montant > 0),
    statut            TEXT NOT NULL CHECK (statut IN ('active', 'annulee')),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE INDEX ajustement_echeance_echeance ON ajustement_echeance (echeance_id) WHERE statut = 'active';

-- Trop-perçu d'une baisse : un avoir au nom de l'élève, jamais effacé. L'élève se retrouve par la
-- facture de l'ajustement : une fusion de fiches emporte ses avoirs avec ses inscriptions.
CREATE TABLE avoir (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    ajustement_id     TEXT NOT NULL REFERENCES ajustement_facture(id),
    -- Échéance payée qui l'a fait naître.
    echeance_id       TEXT NOT NULL REFERENCES echeance(id),
    montant           INTEGER NOT NULL CHECK (montant > 0),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE INDEX avoir_ajustement ON avoir (ajustement_id);

-- Usage d'un avoir sur une échéance de l'élève : un règlement sans encaissement.
CREATE TABLE avoir_imputation (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    avoir_id          TEXT NOT NULL REFERENCES avoir(id),
    echeance_id       TEXT NOT NULL REFERENCES echeance(id),
    montant           INTEGER NOT NULL CHECK (montant > 0),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE INDEX avoir_imputation_avoir ON avoir_imputation (avoir_id);
CREATE INDEX avoir_imputation_echeance ON avoir_imputation (echeance_id);

-- R-14 : rien de tout cela ne se supprime.
CREATE TRIGGER ajustement_facture_sans_suppression BEFORE DELETE ON ajustement_facture
BEGIN SELECT RAISE(ABORT, 'un ajustement de facture ne se supprime pas'); END;
CREATE TRIGGER ajustement_echeance_sans_suppression BEFORE DELETE ON ajustement_echeance
BEGIN SELECT RAISE(ABORT, 'une part d''ajustement ne se supprime pas'); END;
CREATE TRIGGER avoir_sans_suppression BEFORE DELETE ON avoir
BEGIN SELECT RAISE(ABORT, 'un avoir ne se supprime pas'); END;
CREATE TRIGGER avoir_imputation_sans_suppression BEFORE DELETE ON avoir_imputation
BEGIN SELECT RAISE(ABORT, 'un usage d''avoir ne se supprime pas'); END;
