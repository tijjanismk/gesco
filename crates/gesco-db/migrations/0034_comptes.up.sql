-- Transferts entre comptes, rapprochement, compte de chaque mode, dépenses nées des salaires et
-- des avances (SPEC_PHASE_8 §5, §6). Rien ne se supprime (R-14) : un transfert erroné se corrige
-- par le transfert inverse ; une coche se retire en passant à zéro.

CREATE TABLE transfert_tresorerie (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    date              TEXT NOT NULL CHECK (length(date) = 10),
    source_id         TEXT NOT NULL REFERENCES compte_tresorerie(id),
    destination_id    TEXT NOT NULL REFERENCES compte_tresorerie(id),
    montant           INTEGER NOT NULL CHECK (montant > 0),
    reference         TEXT NOT NULL CHECK (length(trim(reference)) BETWEEN 1 AND 120),
    motif             TEXT NOT NULL CHECK (length(trim(motif)) BETWEEN 1 AND 120),
    session_id        TEXT REFERENCES session_caisse(id),
    fait_par          TEXT NOT NULL REFERENCES utilisateur(id),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    identifiant_poste TEXT NOT NULL,
    CHECK (source_id <> destination_id)
);
CREATE INDEX transfert_tresorerie_date ON transfert_tresorerie (etablissement_id, date);

-- Une ligne du livre d'un compte, cochée contre le relevé.
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
CREATE UNIQUE INDEX pointage_tresorerie_ligne
    ON pointage_tresorerie (compte_id, origine, origine_id);

-- Le compte où arrive l'argent d'un mode, quand l'école en a plusieurs de la même nature.
CREATE TABLE compte_du_mode (
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    mode              TEXT NOT NULL CHECK (mode IN ('orange_money', 'wave', 'moov', 'cheque', 'virement')),
    compte_id         TEXT NOT NULL REFERENCES compte_tresorerie(id),
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    PRIMARY KEY (etablissement_id, mode)
);

-- Chaque salaire et chaque avance payés deviennent une dépense « Salaires », une seule fois.
ALTER TABLE depense ADD COLUMN paiement_id TEXT;
CREATE UNIQUE INDEX depense_paiement ON depense (paiement_id) WHERE paiement_id IS NOT NULL;

-- Les mouvements de caisse reçoivent les transferts : la nature s'élargit, le transfert porté
-- s'ajoute. SQLite ne modifie pas une contrainte CHECK : la table est reconstruite.
CREATE TABLE mouvement_caisse_ancien AS SELECT * FROM mouvement_caisse;
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
INSERT INTO mouvement_caisse (id, etablissement_id, session_id, nature, montant, mode, motif,
    versement_id, le, par, cree_le, cree_par, identifiant_poste, paiement_id, depense_id)
SELECT id, etablissement_id, session_id, nature, montant, mode, motif, versement_id, le, par,
    cree_le, cree_par, identifiant_poste, paiement_id, depense_id FROM mouvement_caisse_ancien;
DROP TABLE mouvement_caisse_ancien;
CREATE INDEX mouvement_caisse_session ON mouvement_caisse (session_id);
CREATE INDEX mouvement_caisse_date ON mouvement_caisse (etablissement_id, le);

CREATE TRIGGER mouvement_caisse_sans_suppression BEFORE DELETE ON mouvement_caisse
BEGIN SELECT RAISE(ABORT, 'un mouvement de caisse ne se supprime pas'); END;
CREATE TRIGGER transfert_tresorerie_sans_suppression BEFORE DELETE ON transfert_tresorerie
BEGIN SELECT RAISE(ABORT, 'un transfert ne se supprime pas'); END;
CREATE TRIGGER pointage_tresorerie_sans_suppression BEFORE DELETE ON pointage_tresorerie
BEGIN SELECT RAISE(ABORT, 'une coche du rapprochement ne se supprime pas'); END;
CREATE TRIGGER compte_du_mode_sans_suppression BEFORE DELETE ON compte_du_mode
BEGIN SELECT RAISE(ABORT, 'le compte d''un mode ne se supprime pas'); END;
