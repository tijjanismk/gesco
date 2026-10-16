-- Trésorerie (SPEC_PHASE_8 §2 et §5) : catégories de recettes et de dépenses de l'école, comptes
-- de trésorerie (la caisse, les comptes en banque, les comptes mobile money). Rien ne se supprime :
-- une catégorie se retire, un compte se ferme (R-14).

CREATE TABLE categorie_tresorerie (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    code              TEXT,
    sens              TEXT NOT NULL CHECK (sens IN ('recette', 'depense')),
    libelle           TEXT NOT NULL CHECK (length(trim(libelle)) BETWEEN 1 AND 60),
    ordre             INTEGER NOT NULL,
    active            INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE UNIQUE INDEX categorie_tresorerie_code ON categorie_tresorerie (etablissement_id, code)
    WHERE code IS NOT NULL;

CREATE TABLE compte_tresorerie (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    nature            TEXT NOT NULL CHECK (nature IN ('caisse', 'banque', 'mobile_money')),
    libelle           TEXT NOT NULL CHECK (length(trim(libelle)) BETWEEN 1 AND 60),
    organisme         TEXT NOT NULL DEFAULT '',
    numero            TEXT NOT NULL DEFAULT '',
    solde_ouverture   INTEGER NOT NULL CHECK (solde_ouverture >= 0),
    ouvert_le         TEXT NOT NULL CHECK (length(ouvert_le) = 10),
    actif             INTEGER NOT NULL DEFAULT 1 CHECK (actif IN (0, 1)),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    CHECK (nature = 'caisse' OR length(trim(organisme)) > 0)
);
-- Une seule caisse par école.
CREATE UNIQUE INDEX compte_tresorerie_caisse ON compte_tresorerie (etablissement_id)
    WHERE nature = 'caisse';

CREATE TRIGGER categorie_tresorerie_sans_suppression BEFORE DELETE ON categorie_tresorerie
BEGIN SELECT RAISE(ABORT, 'une catégorie de trésorerie ne se supprime pas'); END;
CREATE TRIGGER compte_tresorerie_sans_suppression BEFORE DELETE ON compte_tresorerie
BEGIN SELECT RAISE(ABORT, 'un compte de trésorerie ne se supprime pas'); END;
