-- Dépenses (SPEC_PHASE_8 §3) : chaque sortie d'argent de l'école, sa catégorie, son compte, son
-- justificatif ; au-dessus du seuil, l'accord de la direction. Les mouvements de caisse reçoivent
-- les dépenses payées en espèces. Rien ne se supprime (R-14).

CREATE TABLE depense (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    date              TEXT NOT NULL CHECK (length(date) = 10),
    categorie_id      TEXT NOT NULL REFERENCES categorie_tresorerie(id),
    libelle           TEXT NOT NULL CHECK (length(trim(libelle)) BETWEEN 1 AND 120),
    beneficiaire      TEXT NOT NULL CHECK (length(trim(beneficiaire)) BETWEEN 1 AND 120),
    montant           INTEGER NOT NULL CHECK (montant > 0),
    statut            TEXT NOT NULL CHECK (statut IN ('a_approuver', 'approuvee', 'refusee', 'payee', 'annulee')),
    saisie_par        TEXT NOT NULL REFERENCES utilisateur(id),
    decidee_par       TEXT REFERENCES utilisateur(id),
    compte_id         TEXT REFERENCES compte_tresorerie(id),
    mode              TEXT CHECK (mode IS NULL OR mode IN ('especes', 'orange_money', 'wave', 'moov', 'cheque', 'virement')),
    reference         TEXT NOT NULL DEFAULT '',
    session_id        TEXT REFERENCES session_caisse(id),
    payee_le          INTEGER,
    motif             TEXT NOT NULL DEFAULT '',
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    CHECK (statut <> 'payee' OR (compte_id IS NOT NULL AND mode IS NOT NULL AND payee_le IS NOT NULL)),
    CHECK (statut NOT IN ('refusee', 'annulee') OR length(trim(motif)) > 0)
);
CREATE INDEX depense_date ON depense (etablissement_id, date);

CREATE TABLE piece_depense (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    depense_id        TEXT NOT NULL REFERENCES depense(id),
    nom_fichier       TEXT NOT NULL,
    contenu           BLOB NOT NULL,
    taille            INTEGER NOT NULL CHECK (taille > 0),
    empreinte_sha256  TEXT NOT NULL CHECK (length(empreinte_sha256) = 64),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE INDEX piece_depense_depense ON piece_depense (depense_id);

-- Les mouvements de caisse reçoivent les dépenses en espèces : la nature s'élargit, la dépense
-- portée s'ajoute. SQLite ne modifie pas une contrainte CHECK : la table est reconstruite.
CREATE TABLE mouvement_caisse_ancien AS SELECT * FROM mouvement_caisse;
DROP TABLE mouvement_caisse;
CREATE TABLE mouvement_caisse (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    session_id        TEXT NOT NULL REFERENCES session_caisse(id),
    nature            TEXT NOT NULL CHECK (nature IN ('sortie_regularisation', 'sortie_salaire', 'sortie_avance', 'sortie_depense', 'entree_regularisation')),
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
    depense_id        TEXT REFERENCES depense(id)
);
INSERT INTO mouvement_caisse (id, etablissement_id, session_id, nature, montant, mode, motif,
    versement_id, le, par, cree_le, cree_par, identifiant_poste, paiement_id)
SELECT id, etablissement_id, session_id, nature, montant, mode, motif, versement_id, le, par,
    cree_le, cree_par, identifiant_poste, paiement_id FROM mouvement_caisse_ancien;
DROP TABLE mouvement_caisse_ancien;
CREATE INDEX mouvement_caisse_session ON mouvement_caisse (session_id);
CREATE INDEX mouvement_caisse_date ON mouvement_caisse (etablissement_id, le);

CREATE TRIGGER mouvement_caisse_sans_suppression BEFORE DELETE ON mouvement_caisse
BEGIN SELECT RAISE(ABORT, 'un mouvement de caisse ne se supprime pas'); END;
CREATE TRIGGER depense_sans_suppression BEFORE DELETE ON depense
BEGIN SELECT RAISE(ABORT, 'une dépense ne se supprime pas'); END;
CREATE TRIGGER piece_depense_sans_suppression BEFORE DELETE ON piece_depense
BEGIN SELECT RAISE(ABORT, 'un justificatif de dépense ne se supprime pas'); END;
