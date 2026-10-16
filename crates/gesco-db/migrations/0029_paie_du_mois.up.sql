-- Paie du mois (SPEC_PHASE_7 §5) : une paie par mois, calculée puis validée ; un bulletin par
-- membre payé ; les montants saisis par la direction ; ses décisions sur les absences non
-- justifiées ; les heures de vacation payées par un bulletin. Rien ne se supprime (R-14).

CREATE TABLE paie_mois (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    mois              TEXT NOT NULL CHECK (length(mois) = 7),
    statut            TEXT NOT NULL CHECK (statut IN ('calculee', 'validee')),
    calculee_le       INTEGER NOT NULL,
    calculee_par      TEXT,
    validee_le        INTEGER,
    validee_par       TEXT,
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    CHECK (statut = 'calculee' OR validee_le IS NOT NULL)
);
CREATE UNIQUE INDEX paie_mois_unique ON paie_mois (etablissement_id, mois);

CREATE TABLE bulletin_paie (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    paie_id           TEXT NOT NULL REFERENCES paie_mois(id),
    membre_id         TEXT NOT NULL REFERENCES membre_personnel(id),
    numero            TEXT,
    -- Lignes et totaux calculés, en JSON.
    contenu           TEXT NOT NULL,
    brut              INTEGER NOT NULL,
    net               INTEGER NOT NULL,
    exclu             INTEGER NOT NULL DEFAULT 0 CHECK (exclu IN (0, 1)),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE UNIQUE INDEX bulletin_paie_unique ON bulletin_paie (paie_id, membre_id);
CREATE UNIQUE INDEX bulletin_paie_numero ON bulletin_paie (etablissement_id, numero)
    WHERE numero IS NOT NULL;
CREATE INDEX bulletin_paie_membre ON bulletin_paie (membre_id);

CREATE TABLE saisie_paie (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    paie_id           TEXT NOT NULL REFERENCES paie_mois(id),
    membre_id         TEXT NOT NULL REFERENCES membre_personnel(id),
    element_id        TEXT NOT NULL REFERENCES element_paie(id),
    montant           INTEGER NOT NULL CHECK (montant <> 0),
    libelle           TEXT NOT NULL CHECK (length(trim(libelle)) BETWEEN 1 AND 80),
    retiree           INTEGER NOT NULL DEFAULT 0 CHECK (retiree IN (0, 1)),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE INDEX saisie_paie_paie ON saisie_paie (paie_id);

CREATE TABLE absence_confirmee (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    paie_id           TEXT NOT NULL REFERENCES paie_mois(id),
    absence_id        TEXT NOT NULL REFERENCES absence_personnel(id),
    retenue           INTEGER NOT NULL CHECK (retenue IN (0, 1)),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE UNIQUE INDEX absence_confirmee_unique ON absence_confirmee (paie_id, absence_id);

-- Une ligne d'heures payée l'est une seule fois : le bulletin qui l'a payée.
ALTER TABLE heures_vacation ADD COLUMN bulletin_paie_id TEXT REFERENCES bulletin_paie(id);

CREATE TRIGGER paie_mois_sans_suppression BEFORE DELETE ON paie_mois
BEGIN SELECT RAISE(ABORT, 'une paie ne se supprime pas'); END;
CREATE TRIGGER bulletin_paie_sans_suppression BEFORE DELETE ON bulletin_paie
BEGIN SELECT RAISE(ABORT, 'un bulletin de paie ne se supprime pas'); END;
CREATE TRIGGER saisie_paie_sans_suppression BEFORE DELETE ON saisie_paie
BEGIN SELECT RAISE(ABORT, 'un montant saisi dans la paie ne se supprime pas'); END;
CREATE TRIGGER absence_confirmee_sans_suppression BEFORE DELETE ON absence_confirmee
BEGIN SELECT RAISE(ABORT, 'une décision sur une absence ne se supprime pas'); END;
-- Une paie validée ne change plus (§10.6) : ni ses bulletins, ni ce qui y fut saisi.
CREATE TRIGGER bulletin_paie_fige BEFORE UPDATE OF contenu, brut, net, exclu ON bulletin_paie
WHEN (SELECT statut FROM paie_mois WHERE id = OLD.paie_id) = 'validee'
BEGIN SELECT RAISE(ABORT, 'le bulletin d''une paie validée ne change plus'); END;
