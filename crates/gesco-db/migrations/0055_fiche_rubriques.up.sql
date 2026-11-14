-- Fiche de l'établissement pour les rapports (SPEC_PHASE_9 §8.1) : un catalogue de rubriques,
-- livrées par l'application ou ajoutées par l'école, et les réponses. Une rubrique annuelle a une
-- réponse par année ; une rubrique permanente, une seule (année vide). Une rubrique sans réponse
-- n'a pas de ligne. Rien ne se supprime (R-14) : une rubrique se désactive, une réponse se vide.
-- Les choix d'une rubrique et les réponses à plusieurs choix sont séparés par un saut de ligne.

CREATE TABLE rubrique_fiche (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    code              TEXT,
    theme             TEXT NOT NULL CHECK (theme IN ('localisation', 'identite', 'actes',
                          'eau_hygiene', 'energie', 'equipements', 'vie_ecole', 'accessibilite',
                          'autres')),
    libelle           TEXT NOT NULL CHECK (length(trim(libelle)) BETWEEN 1 AND 120),
    aide              TEXT NOT NULL DEFAULT '',
    type_reponse      TEXT NOT NULL CHECK (type_reponse IN ('oui_non', 'nombre', 'texte', 'date',
                          'choix', 'plusieurs_choix')),
    choix             TEXT NOT NULL DEFAULT '',
    annuelle          INTEGER NOT NULL CHECK (annuelle IN (0, 1)),
    livree            INTEGER NOT NULL DEFAULT 0 CHECK (livree IN (0, 1)),
    ordre             INTEGER NOT NULL,
    retiree_le        INTEGER,
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE INDEX rubrique_fiche_etablissement ON rubrique_fiche (etablissement_id, theme, ordre);
CREATE UNIQUE INDEX rubrique_fiche_code ON rubrique_fiche (etablissement_id, code)
    WHERE code IS NOT NULL;

CREATE TRIGGER rubrique_fiche_sans_suppression BEFORE DELETE ON rubrique_fiche
BEGIN SELECT RAISE(ABORT, 'une rubrique ne se supprime pas'); END;

CREATE TABLE valeur_fiche (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    rubrique_id       TEXT NOT NULL REFERENCES rubrique_fiche(id),
    annee_id          TEXT REFERENCES annee_scolaire(id),
    valeur            TEXT,
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE UNIQUE INDEX valeur_fiche_unique ON valeur_fiche (rubrique_id, ifnull(annee_id, ''));
CREATE INDEX valeur_fiche_etablissement ON valeur_fiche (etablissement_id, annee_id);

CREATE TRIGGER valeur_fiche_sans_suppression BEFORE DELETE ON valeur_fiche
BEGIN SELECT RAISE(ABORT, 'une réponse de la fiche ne se supprime pas'); END;
