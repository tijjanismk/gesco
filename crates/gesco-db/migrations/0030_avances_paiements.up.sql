-- Avances sur salaire et paiements du personnel (SPEC_PHASE_7 §4 et §7) : une avance se
-- rembourse sur les paies validées ; un salaire ou une avance se paie en une ou plusieurs fois,
-- en espèces de la caisse ou hors caisse. Rien ne se supprime (R-14).

CREATE TABLE avance_salaire (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    membre_id         TEXT NOT NULL REFERENCES membre_personnel(id),
    montant           INTEGER NOT NULL CHECK (montant > 0),
    mensualites       INTEGER NOT NULL CHECK (mensualites BETWEEN 1 AND 24),
    premier_mois      TEXT NOT NULL CHECK (length(premier_mois) = 7),
    accordee_le       TEXT NOT NULL CHECK (length(accordee_le) = 10),
    motif             TEXT NOT NULL CHECK (length(trim(motif)) BETWEEN 1 AND 120),
    statut            TEXT NOT NULL CHECK (statut IN ('en_cours', 'soldee', 'annulee')),
    motif_annulation  TEXT NOT NULL DEFAULT '',
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    CHECK (statut <> 'annulee' OR length(trim(motif_annulation)) > 0)
);
CREATE INDEX avance_salaire_membre ON avance_salaire (membre_id);

-- Ce que chaque paie validée a retenu sur une avance.
CREATE TABLE remboursement_avance (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    avance_id         TEXT NOT NULL REFERENCES avance_salaire(id),
    bulletin_id       TEXT NOT NULL REFERENCES bulletin_paie(id),
    montant           INTEGER NOT NULL CHECK (montant > 0),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE UNIQUE INDEX remboursement_avance_unique ON remboursement_avance (avance_id, bulletin_id);

CREATE TABLE paiement_personnel (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    membre_id         TEXT NOT NULL REFERENCES membre_personnel(id),
    objet             TEXT NOT NULL CHECK (objet IN ('salaire', 'avance')),
    bulletin_id       TEXT REFERENCES bulletin_paie(id),
    avance_id         TEXT REFERENCES avance_salaire(id),
    montant           INTEGER NOT NULL CHECK (montant > 0),
    le                INTEGER NOT NULL,
    mode              TEXT NOT NULL CHECK (mode IN ('especes', 'orange_money', 'wave', 'moov', 'cheque', 'virement')),
    reference         TEXT NOT NULL DEFAULT '',
    source            TEXT NOT NULL CHECK (source IN ('caisse', 'hors_caisse')),
    session_id        TEXT REFERENCES session_caisse(id),
    par               TEXT NOT NULL REFERENCES utilisateur(id),
    annule            INTEGER NOT NULL DEFAULT 0 CHECK (annule IN (0, 1)),
    motif_annulation  TEXT NOT NULL DEFAULT '',
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    CHECK ((objet = 'salaire' AND bulletin_id IS NOT NULL AND avance_id IS NULL)
        OR (objet = 'avance' AND avance_id IS NOT NULL AND bulletin_id IS NULL)),
    CHECK (source = 'hors_caisse' OR (session_id IS NOT NULL AND mode = 'especes')),
    CHECK (mode = 'especes' OR length(trim(reference)) > 0),
    CHECK (annule = 0 OR length(trim(motif_annulation)) > 0)
);
CREATE INDEX paiement_personnel_membre ON paiement_personnel (membre_id);
CREATE INDEX paiement_personnel_bulletin ON paiement_personnel (bulletin_id);

-- Les mouvements de caisse reçoivent les salaires, les avances et les entrées qui remettent un
-- paiement annulé. SQLite ne modifie pas une contrainte CHECK : la table est reconstruite à
-- l'identique, la nature élargie, le paiement porté ajouté.
CREATE TABLE mouvement_caisse_ancien AS SELECT * FROM mouvement_caisse;
DROP TABLE mouvement_caisse;
CREATE TABLE mouvement_caisse (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    session_id        TEXT NOT NULL REFERENCES session_caisse(id),
    nature            TEXT NOT NULL CHECK (nature IN ('sortie_regularisation', 'sortie_salaire', 'sortie_avance', 'entree_regularisation')),
    montant           INTEGER NOT NULL CHECK (montant > 0),
    mode              TEXT NOT NULL CHECK (mode IN ('especes', 'orange_money', 'wave', 'moov', 'cheque', 'virement')),
    motif             TEXT NOT NULL CHECK (length(trim(motif)) > 0),
    versement_id      TEXT REFERENCES versement(id),
    le                INTEGER NOT NULL,
    par               TEXT NOT NULL REFERENCES utilisateur(id),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    identifiant_poste TEXT NOT NULL,
    paiement_id       TEXT REFERENCES paiement_personnel(id)
);
INSERT INTO mouvement_caisse (id, etablissement_id, session_id, nature, montant, mode, motif,
    versement_id, le, par, cree_le, cree_par, identifiant_poste)
SELECT id, etablissement_id, session_id, nature, montant, mode, motif, versement_id, le, par,
    cree_le, cree_par, identifiant_poste FROM mouvement_caisse_ancien;
DROP TABLE mouvement_caisse_ancien;
CREATE INDEX mouvement_caisse_session ON mouvement_caisse (session_id);
CREATE INDEX mouvement_caisse_date ON mouvement_caisse (etablissement_id, le);

CREATE TRIGGER mouvement_caisse_sans_suppression BEFORE DELETE ON mouvement_caisse
BEGIN SELECT RAISE(ABORT, 'un mouvement de caisse ne se supprime pas'); END;
CREATE TRIGGER avance_salaire_sans_suppression BEFORE DELETE ON avance_salaire
BEGIN SELECT RAISE(ABORT, 'une avance ne se supprime pas'); END;
CREATE TRIGGER remboursement_avance_sans_suppression BEFORE DELETE ON remboursement_avance
BEGIN SELECT RAISE(ABORT, 'un remboursement d''avance ne se supprime pas'); END;
CREATE TRIGGER paiement_personnel_sans_suppression BEFORE DELETE ON paiement_personnel
BEGIN SELECT RAISE(ABORT, 'un paiement du personnel ne se supprime pas'); END;
