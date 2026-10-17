DROP TRIGGER recette_tresorerie_sans_suppression;
CREATE TABLE pointage_tresorerie_nouveau AS SELECT * FROM pointage_tresorerie
    WHERE origine <> 'recette';
DROP TABLE pointage_tresorerie;
CREATE TABLE pointage_tresorerie (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    compte_id         TEXT NOT NULL REFERENCES compte_tresorerie(id),
    origine           TEXT NOT NULL CHECK (origine IN ('versement', 'mouvement', 'depense', 'transfert')),
    origine_id        TEXT NOT NULL CHECK (length(origine_id) = 26),
    pointee           INTEGER NOT NULL CHECK (pointee IN (0, 1)),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
INSERT INTO pointage_tresorerie SELECT * FROM pointage_tresorerie_nouveau;
DROP TABLE pointage_tresorerie_nouveau;
CREATE UNIQUE INDEX pointage_tresorerie_ligne
    ON pointage_tresorerie (compte_id, origine, origine_id);
CREATE TRIGGER pointage_tresorerie_sans_suppression BEFORE DELETE ON pointage_tresorerie
BEGIN SELECT RAISE(ABORT, 'une coche du rapprochement ne se supprime pas'); END;
CREATE TABLE mouvement_caisse_nouveau AS SELECT * FROM mouvement_caisse
    WHERE nature <> 'entree_recette';
DROP TABLE mouvement_caisse;
CREATE TABLE mouvement_caisse (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    session_id        TEXT NOT NULL REFERENCES session_caisse(id),
    nature            TEXT NOT NULL CHECK (nature IN ('sortie_regularisation', 'sortie_salaire', 'sortie_avance', 'sortie_depense', 'sortie_transfert', 'entree_regularisation', 'entree_transfert')),
    montant           INTEGER NOT NULL CHECK (montant > 0),
    mode              TEXT NOT NULL CHECK (mode IN ('especes', 'orange_money', 'wave', 'moov', 'cheque', 'virement')),
    motif             TEXT NOT NULL CHECK (length(trim(motif)) > 0),
    versement_id      TEXT REFERENCES versement(id),
    le                INTEGER NOT NULL,
    par               TEXT NOT NULL REFERENCES utilisateur(id),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    identifiant_poste TEXT NOT NULL,
    paiement_id       TEXT REFERENCES paiement_personnel(id),
    depense_id        TEXT REFERENCES depense(id),
    transfert_id      TEXT REFERENCES transfert_tresorerie(id)
);
INSERT INTO mouvement_caisse SELECT id, etablissement_id, session_id, nature, montant, mode, motif,
    versement_id, le, par, cree_le, cree_par, identifiant_poste, paiement_id, depense_id,
    transfert_id FROM mouvement_caisse_nouveau;
DROP TABLE mouvement_caisse_nouveau;
CREATE INDEX mouvement_caisse_session ON mouvement_caisse (session_id);
CREATE INDEX mouvement_caisse_date ON mouvement_caisse (etablissement_id, le);
CREATE TRIGGER mouvement_caisse_sans_suppression BEFORE DELETE ON mouvement_caisse
BEGIN SELECT RAISE(ABORT, 'un mouvement de caisse ne se supprime pas'); END;
DROP TABLE recette_tresorerie;
