-- Factures et échéances (SPEC_PHASE_3 §4). Jamais de DELETE (R-14) : une facture s'annule, une
-- échéance s'annule par contre-passation. Une seule facture non annulée par inscription.

CREATE TABLE facture (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    inscription_id    TEXT NOT NULL REFERENCES inscription(id),
    annee_id          TEXT NOT NULL REFERENCES annee_scolaire(id),
    grille_id         TEXT NOT NULL REFERENCES grille_tarifaire(id),
    plan_paiement_id  TEXT NOT NULL REFERENCES plan_paiement(id),
    numero            TEXT NOT NULL,
    date_emission     TEXT NOT NULL CHECK (length(date_emission) = 10),
    statut            TEXT NOT NULL CHECK (statut IN ('emise', 'partiellement_reglee', 'soldee', 'annulee')),
    total             INTEGER NOT NULL CHECK (total >= 0),
    motif_annulation  TEXT NOT NULL DEFAULT '',
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE UNIQUE INDEX facture_numero_unique ON facture (etablissement_id, numero);
CREATE UNIQUE INDEX facture_vivante_unique ON facture (inscription_id) WHERE statut <> 'annulee';
CREATE INDEX facture_annee ON facture (etablissement_id, annee_id, statut);

CREATE TABLE ligne_facture (
    id                  TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id    TEXT NOT NULL REFERENCES etablissement(id),
    facture_id          TEXT NOT NULL REFERENCES facture(id),
    ordre               INTEGER NOT NULL,
    ligne_tarifaire_id  TEXT NOT NULL REFERENCES ligne_tarifaire(id),
    type_frais          TEXT NOT NULL CHECK (type_frais IN ('inscription', 'scolarite', 'cantine', 'transport', 'tenue', 'examen', 'autre')),
    libelle             TEXT NOT NULL,
    montant             INTEGER NOT NULL CHECK (montant >= 0),
    cree_le             INTEGER NOT NULL,
    cree_par            TEXT,
    mis_a_jour_le       INTEGER NOT NULL,
    mis_a_jour_par      TEXT,
    identifiant_poste   TEXT NOT NULL
);
CREATE INDEX ligne_facture_facture ON ligne_facture (facture_id);

CREATE TABLE echeance (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    facture_id        TEXT NOT NULL REFERENCES facture(id),
    ligne_facture_id  TEXT NOT NULL REFERENCES ligne_facture(id),
    rang              INTEGER NOT NULL,
    libelle           TEXT NOT NULL,
    date_echeance     TEXT NOT NULL CHECK (length(date_echeance) = 10),
    montant           INTEGER NOT NULL CHECK (montant >= 0),
    statut            TEXT NOT NULL CHECK (statut IN ('due', 'annulee')),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE INDEX echeance_facture ON echeance (facture_id);
-- Blueprint §8.9 : l'état des impayés parcourt les échéances par statut et par date.
CREATE INDEX echeance_statut_date ON echeance (etablissement_id, statut, date_echeance);

-- R-14 : la suppression physique est refusée mécaniquement ; aucune logique ici (R-15).
CREATE TRIGGER facture_sans_suppression BEFORE DELETE ON facture
BEGIN SELECT RAISE(ABORT, 'une facture ne se supprime pas'); END;
CREATE TRIGGER ligne_facture_sans_suppression BEFORE DELETE ON ligne_facture
BEGIN SELECT RAISE(ABORT, 'une ligne de facture ne se supprime pas'); END;
CREATE TRIGGER echeance_sans_suppression BEFORE DELETE ON echeance
BEGIN SELECT RAISE(ABORT, 'une échéance ne se supprime pas'); END;
