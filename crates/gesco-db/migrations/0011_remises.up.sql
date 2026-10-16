-- Remises (SPEC_PHASE_3 §5) et leur part sur chaque échéance. Jamais de DELETE (R-14) : une
-- remise s'annule, sa part sur une échéance se contre-passe.

CREATE TABLE reduction (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    inscription_id    TEXT NOT NULL REFERENCES inscription(id),
    type_remise       TEXT NOT NULL CHECK (type_remise IN ('bourse', 'demi_bourse', 'fratrie', 'enfant_personnel', 'arrangement')),
    nature_valeur     TEXT NOT NULL CHECK (nature_valeur IN ('taux', 'montant')),
    valeur            INTEGER NOT NULL CHECK (valeur > 0),
    type_frais        TEXT NOT NULL CHECK (type_frais IN ('inscription', 'scolarite', 'cantine', 'transport', 'tenue', 'examen', 'autre')),
    motif             TEXT NOT NULL CHECK (length(trim(motif)) > 0),
    statut            TEXT NOT NULL CHECK (statut IN ('proposee', 'accordee', 'refusee', 'annulee')),
    proposee_par      TEXT NOT NULL,
    proposee_le       TEXT NOT NULL CHECK (length(proposee_le) = 10),
    decidee_par       TEXT,
    motif_decision    TEXT NOT NULL DEFAULT '',
    montant_applique  INTEGER NOT NULL DEFAULT 0 CHECK (montant_applique >= 0),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE INDEX reduction_inscription ON reduction (inscription_id);
CREATE INDEX reduction_statut ON reduction (etablissement_id, statut);

CREATE TABLE reduction_echeance (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    reduction_id      TEXT NOT NULL REFERENCES reduction(id),
    echeance_id       TEXT NOT NULL REFERENCES echeance(id),
    montant           INTEGER NOT NULL CHECK (montant > 0),
    statut            TEXT NOT NULL CHECK (statut IN ('active', 'annulee')),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE INDEX reduction_echeance_echeance ON reduction_echeance (echeance_id) WHERE statut = 'active';
CREATE INDEX reduction_echeance_reduction ON reduction_echeance (reduction_id);

CREATE TRIGGER reduction_sans_suppression BEFORE DELETE ON reduction
BEGIN SELECT RAISE(ABORT, 'une remise ne se supprime pas'); END;
CREATE TRIGGER reduction_echeance_sans_suppression BEFORE DELETE ON reduction_echeance
BEGIN SELECT RAISE(ABORT, 'la part d''une remise ne se supprime pas'); END;
