-- Versements et imputations (SPEC_PHASE_3 §7). Un reçu par versement, numéroté sans trou
-- (R-46) ; une clé d'idempotence par encaissement (R-35) ; jamais de DELETE (R-14).

CREATE TABLE versement (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    inscription_id    TEXT NOT NULL REFERENCES inscription(id),
    facture_id        TEXT NOT NULL REFERENCES facture(id),
    session_id        TEXT NOT NULL REFERENCES session_caisse(id),
    numero_recu       TEXT NOT NULL,
    encaisse_le       INTEGER NOT NULL,
    montant           INTEGER NOT NULL CHECK (montant > 0),
    mode              TEXT NOT NULL CHECK (mode IN ('especes', 'orange_money', 'wave', 'moov', 'cheque', 'virement')),
    reference         TEXT NOT NULL DEFAULT '',
    payeur_id         TEXT REFERENCES responsable(id),
    payeur_nom        TEXT NOT NULL,
    encaisse_par      TEXT NOT NULL REFERENCES utilisateur(id),
    cle_idempotence   TEXT NOT NULL,
    statut            TEXT NOT NULL CHECK (statut IN ('valide', 'annule')),
    motif_annulation  TEXT NOT NULL DEFAULT '',
    annule_par        TEXT REFERENCES utilisateur(id),
    annule_le         INTEGER,
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE UNIQUE INDEX versement_recu_unique ON versement (etablissement_id, numero_recu);
CREATE UNIQUE INDEX versement_idempotence_unique ON versement (etablissement_id, cle_idempotence);
-- Blueprint §8.9 : la situation financière parcourt les versements d'une inscription par date.
CREATE INDEX versement_inscription ON versement (inscription_id, encaisse_le);
CREATE INDEX versement_session ON versement (session_id);

CREATE TABLE imputation_versement (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    versement_id      TEXT NOT NULL REFERENCES versement(id),
    echeance_id       TEXT NOT NULL REFERENCES echeance(id),
    montant           INTEGER NOT NULL CHECK (montant > 0),
    statut            TEXT NOT NULL CHECK (statut IN ('active', 'annulee')),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE INDEX imputation_echeance ON imputation_versement (echeance_id) WHERE statut = 'active';
CREATE INDEX imputation_versement_versement ON imputation_versement (versement_id);

CREATE TRIGGER versement_sans_suppression BEFORE DELETE ON versement
BEGIN SELECT RAISE(ABORT, 'un versement ne se supprime pas'); END;
CREATE TRIGGER imputation_sans_suppression BEFORE DELETE ON imputation_versement
BEGIN SELECT RAISE(ABORT, 'une imputation ne se supprime pas'); END;
