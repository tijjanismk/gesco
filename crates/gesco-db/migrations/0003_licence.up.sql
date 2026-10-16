-- Licence installée sur le poste (spécification §13, blueprint §26.5). Le fichier signé est gardé
-- tel quel et revérifié à chaque lecture : la base ne fait jamais foi de son authenticité. Une
-- licence remplacée est supprimée logiquement, ce qui garde l'historique des installations.
CREATE TABLE licence (
    id                      TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id        TEXT NOT NULL REFERENCES etablissement(id),
    licence_id              TEXT NOT NULL,
    contenu                 TEXT NOT NULL,
    -- Premier constat d'une empreinte différente de celle de la licence : départ de la grâce.
    ecart_empreinte_depuis  INTEGER,
    cree_le                 INTEGER NOT NULL,
    cree_par                TEXT,
    mis_a_jour_le           INTEGER NOT NULL,
    mis_a_jour_par          TEXT,
    identifiant_poste       TEXT NOT NULL,
    supprime_le             INTEGER,
    supprime_par            TEXT
);

CREATE UNIQUE INDEX licence_courante ON licence (etablissement_id) WHERE supprime_le IS NULL;
