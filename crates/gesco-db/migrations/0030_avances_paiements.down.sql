DROP TRIGGER paiement_personnel_sans_suppression;
DROP TRIGGER remboursement_avance_sans_suppression;
DROP TRIGGER avance_salaire_sans_suppression;
DROP TRIGGER mouvement_caisse_sans_suppression;
CREATE TABLE mouvement_caisse_nouveau AS SELECT * FROM mouvement_caisse
    WHERE nature = 'sortie_regularisation';
DROP TABLE mouvement_caisse;
CREATE TABLE mouvement_caisse (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    session_id        TEXT NOT NULL REFERENCES session_caisse(id),
    nature            TEXT NOT NULL CHECK (nature IN ('sortie_regularisation')),
    montant           INTEGER NOT NULL CHECK (montant > 0),
    mode              TEXT NOT NULL CHECK (mode IN ('especes', 'orange_money', 'wave', 'moov', 'cheque', 'virement')),
    motif             TEXT NOT NULL CHECK (length(trim(motif)) > 0),
    versement_id      TEXT REFERENCES versement(id),
    le                INTEGER NOT NULL,
    par               TEXT NOT NULL REFERENCES utilisateur(id),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    identifiant_poste TEXT NOT NULL
);
INSERT INTO mouvement_caisse SELECT id, etablissement_id, session_id, nature, montant, mode, motif,
    versement_id, le, par, cree_le, cree_par, identifiant_poste FROM mouvement_caisse_nouveau;
DROP TABLE mouvement_caisse_nouveau;
CREATE INDEX mouvement_caisse_session ON mouvement_caisse (session_id);
CREATE INDEX mouvement_caisse_date ON mouvement_caisse (etablissement_id, le);
CREATE TRIGGER mouvement_caisse_sans_suppression BEFORE DELETE ON mouvement_caisse
BEGIN SELECT RAISE(ABORT, 'un mouvement de caisse ne se supprime pas'); END;
DROP TABLE paiement_personnel;
DROP TABLE remboursement_avance;
DROP TABLE avance_salaire;
