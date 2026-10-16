-- Personnes, élèves, responsables et leurs liens (SPEC_PHASE_2 §4). Aucun index unique sur nom,
-- prénoms et date de naissance (blueprint §3.5). La photo vit dans la base, comme le logo : elle
-- voyage avec les sauvegardes ; les listes ne la lisent jamais.

CREATE TABLE personne (
    id                      TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id        TEXT NOT NULL REFERENCES etablissement(id),
    nom                     TEXT NOT NULL,
    prenoms                 TEXT NOT NULL,
    sexe                    TEXT CHECK (sexe IN ('masculin', 'feminin')),
    date_naissance          TEXT CHECK (date_naissance IS NULL OR length(date_naissance) = 10),
    date_naissance_estimee  INTEGER NOT NULL DEFAULT 0 CHECK (date_naissance_estimee IN (0, 1)),
    lieu_naissance          TEXT NOT NULL DEFAULT '',
    nationalite             TEXT NOT NULL DEFAULT '',
    adresse                 TEXT NOT NULL DEFAULT '',
    telephones              TEXT NOT NULL DEFAULT '[]',
    photo                   BLOB,
    photo_format            TEXT CHECK (photo_format IN ('png', 'jpeg')),
    cree_le                 INTEGER NOT NULL,
    cree_par                TEXT,
    mis_a_jour_le           INTEGER NOT NULL,
    mis_a_jour_par          TEXT,
    identifiant_poste       TEXT NOT NULL,
    supprime_le             INTEGER,
    supprime_par            TEXT
);
CREATE INDEX personne_etablissement ON personne (etablissement_id);

CREATE TABLE eleve (
    id                  TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id    TEXT NOT NULL REFERENCES etablissement(id),
    personne_id         TEXT NOT NULL REFERENCES personne(id),
    matricule_interne   TEXT NOT NULL,
    matricule_national  TEXT,
    acte_nature         TEXT CHECK (acte_nature IN ('acte', 'jugement_suppletif')),
    acte_numero         TEXT NOT NULL DEFAULT '',
    acte_date           TEXT CHECK (acte_date IS NULL OR length(acte_date) = 10),
    ecole_origine       TEXT NOT NULL DEFAULT '',
    fusionne_dans       TEXT REFERENCES eleve(id),
    cree_le             INTEGER NOT NULL,
    cree_par            TEXT,
    mis_a_jour_le       INTEGER NOT NULL,
    mis_a_jour_par      TEXT,
    identifiant_poste   TEXT NOT NULL,
    supprime_le         INTEGER,
    supprime_par        TEXT
);
-- Le matricule interne ne change jamais, même après une fusion (SPEC_PHASE_2 §4.2, §8).
CREATE UNIQUE INDEX eleve_matricule_interne_unique ON eleve (etablissement_id, matricule_interne);
-- Unicité du matricule national parmi les seules valeurs renseignées (§24.4).
CREATE UNIQUE INDEX eleve_matricule_national_unique
    ON eleve (etablissement_id, matricule_national)
    WHERE matricule_national IS NOT NULL AND fusionne_dans IS NULL AND supprime_le IS NULL;

CREATE TABLE responsable (
    id                 TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id   TEXT NOT NULL REFERENCES etablissement(id),
    personne_id        TEXT NOT NULL REFERENCES personne(id),
    profession         TEXT NOT NULL DEFAULT '',
    employeur          TEXT NOT NULL DEFAULT '',
    cree_le            INTEGER NOT NULL,
    cree_par           TEXT,
    mis_a_jour_le      INTEGER NOT NULL,
    mis_a_jour_par     TEXT,
    identifiant_poste  TEXT NOT NULL,
    supprime_le        INTEGER,
    supprime_par       TEXT
);

-- Qualités indépendantes et propres à chaque lien (blueprint §3.6, T-03).
CREATE TABLE eleve_responsable (
    etablissement_id       TEXT NOT NULL REFERENCES etablissement(id),
    eleve_id               TEXT NOT NULL REFERENCES eleve(id),
    responsable_id         TEXT NOT NULL REFERENCES responsable(id),
    lien                   TEXT NOT NULL CHECK (lien IN ('pere', 'mere', 'tuteur', 'autre')),
    est_responsable_legal  INTEGER NOT NULL CHECK (est_responsable_legal IN (0, 1)),
    est_payeur             INTEGER NOT NULL CHECK (est_payeur IN (0, 1)),
    recoit_notifications   INTEGER NOT NULL CHECK (recoit_notifications IN (0, 1)),
    vient_chercher         INTEGER NOT NULL CHECK (vient_chercher IN (0, 1)),
    ordre_priorite         INTEGER NOT NULL CHECK (ordre_priorite >= 1),
    cree_le                INTEGER NOT NULL,
    cree_par               TEXT,
    mis_a_jour_le          INTEGER NOT NULL,
    mis_a_jour_par         TEXT,
    identifiant_poste      TEXT NOT NULL,
    supprime_le            INTEGER,
    supprime_par           TEXT
);
CREATE UNIQUE INDEX eleve_responsable_unique
    ON eleve_responsable (eleve_id, responsable_id) WHERE supprime_le IS NULL;
CREATE INDEX eleve_responsable_responsable ON eleve_responsable (responsable_id);
