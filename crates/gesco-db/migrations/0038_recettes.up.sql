-- Autres recettes (SPEC_VERSION_1_1 §1, étape 1) : un don, une location de salle, hors des
-- versements des parents. En espèces, une entrée dans la session de caisse ouverte. Rien ne se
-- supprime (R-14) : une recette saisie par erreur s'annule avec un motif.

CREATE TABLE recette_tresorerie (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    date              TEXT NOT NULL CHECK (length(date) = 10),
    categorie_id      TEXT NOT NULL REFERENCES categorie_tresorerie(id),
    libelle           TEXT NOT NULL CHECK (length(trim(libelle)) BETWEEN 1 AND 120),
    payeur            TEXT NOT NULL CHECK (length(trim(payeur)) BETWEEN 1 AND 120),
    montant           INTEGER NOT NULL CHECK (montant > 0),
    compte_id         TEXT NOT NULL REFERENCES compte_tresorerie(id),
    mode              TEXT NOT NULL CHECK (mode IN ('especes', 'orange_money', 'wave', 'moov', 'cheque', 'virement')),
    reference         TEXT NOT NULL DEFAULT '',
    session_id        TEXT REFERENCES session_caisse(id),
    saisie_par        TEXT NOT NULL REFERENCES utilisateur(id),
    annulee           INTEGER NOT NULL DEFAULT 0 CHECK (annulee IN (0, 1)),
    motif_annulation  TEXT NOT NULL DEFAULT '',
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    CHECK (annulee = 0 OR length(trim(motif_annulation)) > 0)
);
CREATE INDEX recette_tresorerie_date ON recette_tresorerie (etablissement_id, date);

-- Les mouvements de caisse reçoivent les recettes en espèces ; les coches du rapprochement, les
-- recettes d'une banque ou d'un compte mobile money. SQLite ne modifie pas une contrainte CHECK :
-- les deux tables sont reconstruites.
CREATE TABLE mouvement_caisse_ancien AS SELECT * FROM mouvement_caisse;
DROP TABLE mouvement_caisse;
CREATE TABLE mouvement_caisse (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    session_id        TEXT NOT NULL REFERENCES session_caisse(id),
    nature            TEXT NOT NULL CHECK (nature IN ('sortie_regularisation', 'sortie_salaire', 'sortie_avance', 'sortie_depense', 'sortie_transfert', 'entree_regularisation', 'entree_transfert', 'entree_recette')),
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
    transfert_id      TEXT REFERENCES transfert_tresorerie(id),
    recette_id        TEXT REFERENCES recette_tresorerie(id)
);
INSERT INTO mouvement_caisse (id, etablissement_id, session_id, nature, montant, mode, motif,
    versement_id, le, par, cree_le, cree_par, identifiant_poste, paiement_id, depense_id,
    transfert_id)
SELECT id, etablissement_id, session_id, nature, montant, mode, motif, versement_id, le, par,
    cree_le, cree_par, identifiant_poste, paiement_id, depense_id, transfert_id
    FROM mouvement_caisse_ancien;
DROP TABLE mouvement_caisse_ancien;
CREATE INDEX mouvement_caisse_session ON mouvement_caisse (session_id);
CREATE INDEX mouvement_caisse_date ON mouvement_caisse (etablissement_id, le);
CREATE TRIGGER mouvement_caisse_sans_suppression BEFORE DELETE ON mouvement_caisse
BEGIN SELECT RAISE(ABORT, 'un mouvement de caisse ne se supprime pas'); END;

CREATE TABLE pointage_tresorerie_ancien AS SELECT * FROM pointage_tresorerie;
DROP TABLE pointage_tresorerie;
CREATE TABLE pointage_tresorerie (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    compte_id         TEXT NOT NULL REFERENCES compte_tresorerie(id),
    origine           TEXT NOT NULL CHECK (origine IN ('versement', 'mouvement', 'depense', 'transfert', 'recette')),
    origine_id        TEXT NOT NULL CHECK (length(origine_id) = 26),
    pointee           INTEGER NOT NULL CHECK (pointee IN (0, 1)),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
INSERT INTO pointage_tresorerie SELECT * FROM pointage_tresorerie_ancien;
DROP TABLE pointage_tresorerie_ancien;
CREATE UNIQUE INDEX pointage_tresorerie_ligne
    ON pointage_tresorerie (compte_id, origine, origine_id);
CREATE TRIGGER pointage_tresorerie_sans_suppression BEFORE DELETE ON pointage_tresorerie
BEGIN SELECT RAISE(ABORT, 'une coche du rapprochement ne se supprime pas'); END;

CREATE TRIGGER recette_tresorerie_sans_suppression BEFORE DELETE ON recette_tresorerie
BEGIN SELECT RAISE(ABORT, 'une recette ne se supprime pas'); END;
