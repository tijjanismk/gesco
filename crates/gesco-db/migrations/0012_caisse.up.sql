-- Sessions de caisse (SPEC_PHASE_3 §6). Une seule session ouverte par caissier ; une session
-- clôturée ne se rouvre pas ; jamais de DELETE (R-14).

CREATE TABLE session_caisse (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    caissier_id       TEXT NOT NULL REFERENCES utilisateur(id),
    ouverte_le        INTEGER NOT NULL,
    fonds_initial     INTEGER NOT NULL CHECK (fonds_initial >= 0),
    statut            TEXT NOT NULL CHECK (statut IN ('ouverte', 'cloturee', 'rapprochee')),
    cloturee_le       INTEGER,
    total_theorique   INTEGER,
    total_compte      INTEGER CHECK (total_compte IS NULL OR total_compte >= 0),
    ecart             INTEGER,
    motif_ecart       TEXT NOT NULL DEFAULT '',
    rapprochee_par    TEXT REFERENCES utilisateur(id),
    rapprochee_le     INTEGER,
    commentaire_visa  TEXT NOT NULL DEFAULT '',
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE UNIQUE INDEX session_caisse_ouverte_unique
    ON session_caisse (etablissement_id, caissier_id) WHERE statut = 'ouverte';
CREATE INDEX session_caisse_date ON session_caisse (etablissement_id, ouverte_le);

CREATE TRIGGER session_caisse_sans_suppression BEFORE DELETE ON session_caisse
BEGIN SELECT RAISE(ABORT, 'une session de caisse ne se supprime pas'); END;
