-- Mouvements de caisse (SPEC_PHASE_3 §8) : la sortie de régularisation d'un versement annulé
-- après la clôture de sa session, passée dans la session ouverte de qui annule. Jamais de
-- DELETE (R-14).

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
CREATE INDEX mouvement_caisse_session ON mouvement_caisse (session_id);
CREATE INDEX mouvement_caisse_date ON mouvement_caisse (etablissement_id, le);

CREATE TRIGGER mouvement_caisse_sans_suppression BEFORE DELETE ON mouvement_caisse
BEGIN SELECT RAISE(ABORT, 'un mouvement de caisse ne se supprime pas'); END;
