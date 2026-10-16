-- Fournisseurs et leurs factures (SPEC_PHASE_8 §4) : une facture se paie en une ou plusieurs
-- dépenses, qui la portent ; le reste dû se lit. Rien ne se supprime (R-14).

CREATE TABLE fournisseur (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    nom               TEXT NOT NULL CHECK (length(trim(nom)) BETWEEN 1 AND 120),
    telephone         TEXT NOT NULL DEFAULT '',
    fourniture        TEXT NOT NULL DEFAULT '',
    actif             INTEGER NOT NULL DEFAULT 1 CHECK (actif IN (0, 1)),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);

CREATE TABLE facture_fournisseur (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    fournisseur_id    TEXT NOT NULL REFERENCES fournisseur(id),
    numero            TEXT NOT NULL DEFAULT '',
    date              TEXT NOT NULL CHECK (length(date) = 10),
    echeance          TEXT CHECK (echeance IS NULL OR length(echeance) = 10),
    libelle           TEXT NOT NULL CHECK (length(trim(libelle)) BETWEEN 1 AND 120),
    categorie_id      TEXT NOT NULL REFERENCES categorie_tresorerie(id),
    montant           INTEGER NOT NULL CHECK (montant > 0),
    annulee           INTEGER NOT NULL DEFAULT 0 CHECK (annulee IN (0, 1)),
    motif_annulation  TEXT NOT NULL DEFAULT '',
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    CHECK (annulee = 0 OR length(trim(motif_annulation)) > 0)
);
CREATE INDEX facture_fournisseur_fournisseur ON facture_fournisseur (fournisseur_id);

-- Une dépense qui paie une facture la porte. Sans clé étrangère, que SQLite refuse de retirer
-- d'une colonne : le service ne pose qu'une facture de l'école.
ALTER TABLE depense ADD COLUMN facture_id TEXT;
CREATE INDEX depense_facture ON depense (facture_id) WHERE facture_id IS NOT NULL;

CREATE TRIGGER fournisseur_sans_suppression BEFORE DELETE ON fournisseur
BEGIN SELECT RAISE(ABORT, 'un fournisseur ne se supprime pas'); END;
CREATE TRIGGER facture_fournisseur_sans_suppression BEFORE DELETE ON facture_fournisseur
BEGIN SELECT RAISE(ABORT, 'une facture de fournisseur ne se supprime pas'); END;
