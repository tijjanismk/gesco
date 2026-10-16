-- Migration initiale : tables du domaine CORE (SPEC_PHASE_0_5 §4).
-- Colonnes obligatoires (§4.1) : id, etablissement_id, cree_le, cree_par, mis_a_jour_le,
-- mis_a_jour_par, identifiant_poste, supprime_le, supprime_par. Les identifiants sont des ULID en
-- texte (26 caractères), les instants des millisecondes UTC, les montants des entiers de francs.

CREATE TABLE organisation (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    nom               TEXT NOT NULL,
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    supprime_le       INTEGER,
    supprime_par      TEXT
);

CREATE TABLE etablissement (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    organisation_id   TEXT NOT NULL REFERENCES organisation(id),
    nom               TEXT NOT NULL,
    statut            TEXT NOT NULL CHECK (statut IN ('public', 'prive', 'communautaire', 'medersa')),
    code_academie     TEXT NOT NULL DEFAULT '',
    code_cap          TEXT NOT NULL DEFAULT '',
    adresse           TEXT NOT NULL DEFAULT '',
    telephones        TEXT NOT NULL DEFAULT '[]',
    mentions_legales  TEXT NOT NULL DEFAULT '',
    multiple_arrondi  INTEGER NOT NULL DEFAULT 5 CHECK (multiple_arrondi > 0),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    supprime_le       INTEGER,
    supprime_par      TEXT
);

CREATE TABLE campus (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    nom               TEXT NOT NULL,
    adresse           TEXT NOT NULL DEFAULT '',
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    supprime_le       INTEGER,
    supprime_par      TEXT
);

CREATE TABLE annee_scolaire (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    libelle           TEXT NOT NULL,
    debut             INTEGER NOT NULL,
    fin               INTEGER NOT NULL CHECK (fin > debut),
    statut            TEXT NOT NULL CHECK (statut IN ('preparation', 'active', 'cloturee', 'archivee')),
    est_courante      INTEGER NOT NULL DEFAULT 0 CHECK (est_courante IN (0, 1)),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    supprime_le       INTEGER,
    supprime_par      TEXT
);
-- Une seule année courante par établissement, garantie en base (§4.2).
CREATE UNIQUE INDEX annee_scolaire_courante_unique
    ON annee_scolaire (etablissement_id) WHERE est_courante = 1 AND supprime_le IS NULL;
CREATE INDEX annee_scolaire_etablissement ON annee_scolaire (etablissement_id, est_courante);

CREATE TABLE periode_academique (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    annee_id          TEXT NOT NULL REFERENCES annee_scolaire(id),
    niveau_id         TEXT,  -- renseigné à partir de la phase 2 (référentiel académique)
    rang              INTEGER NOT NULL CHECK (rang >= 1),
    type              TEXT NOT NULL CHECK (type IN ('mensuelle', 'trimestre', 'semestre')),
    debut             INTEGER NOT NULL,
    fin               INTEGER NOT NULL CHECK (fin > debut),
    ponderation       INTEGER NOT NULL DEFAULT 1 CHECK (ponderation >= 1),
    verrouillee       INTEGER NOT NULL DEFAULT 0 CHECK (verrouillee IN (0, 1)),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    supprime_le       INTEGER,
    supprime_par      TEXT
);

CREATE TABLE utilisateur (
    id                        TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id          TEXT NOT NULL REFERENCES etablissement(id),
    personne_id               TEXT,  -- rattachement à la phase 2
    identifiant               TEXT NOT NULL,
    nom_affiche               TEXT NOT NULL,
    actif                     INTEGER NOT NULL DEFAULT 1 CHECK (actif IN (0, 1)),
    doit_changer_mot_de_passe INTEGER NOT NULL DEFAULT 1 CHECK (doit_changer_mot_de_passe IN (0, 1)),
    derniere_connexion        INTEGER,
    echecs_consecutifs        INTEGER NOT NULL DEFAULT 0,
    verrouille_jusqua         INTEGER,
    cree_le                   INTEGER NOT NULL,
    cree_par                  TEXT,
    mis_a_jour_le             INTEGER NOT NULL,
    mis_a_jour_par            TEXT,
    identifiant_poste         TEXT NOT NULL,
    supprime_le               INTEGER,
    supprime_par              TEXT
);
CREATE UNIQUE INDEX utilisateur_identifiant_unique ON utilisateur (etablissement_id, identifiant);

-- Les secrets sont isolés : aucune lecture d'utilisateur ne les rapporte par accident (R-25).
CREATE TABLE utilisateur_secret (
    utilisateur_id        TEXT PRIMARY KEY REFERENCES utilisateur(id),
    empreinte_mot_de_passe TEXT NOT NULL,
    empreinte_pin         TEXT,
    mis_a_jour_le         INTEGER NOT NULL
);

CREATE TABLE role (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    code              TEXT NOT NULL,
    libelle           TEXT NOT NULL,
    systeme           INTEGER NOT NULL DEFAULT 0 CHECK (systeme IN (0, 1)),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    supprime_le       INTEGER,
    supprime_par      TEXT
);
CREATE UNIQUE INDEX role_code_unique ON role (etablissement_id, code);

-- Référentiel partagé : copie du catalogue du noyau, synchronisée à l'ouverture (dérogation R-12).
CREATE TABLE permission (
    code     TEXT PRIMARY KEY,
    domaine  TEXT NOT NULL
);

CREATE TABLE role_permission (
    role_id          TEXT NOT NULL REFERENCES role(id),
    permission_code  TEXT NOT NULL REFERENCES permission(code),
    PRIMARY KEY (role_id, permission_code)
);

CREATE TABLE utilisateur_role (
    utilisateur_id  TEXT NOT NULL REFERENCES utilisateur(id),
    role_id         TEXT NOT NULL REFERENCES role(id),
    portee          TEXT,  -- portée ressource sérialisée, renseignée à partir de la phase 2
    PRIMARY KEY (utilisateur_id, role_id)
);

CREATE TABLE parametre (
    etablissement_id            TEXT NOT NULL REFERENCES etablissement(id),
    cle                         TEXT NOT NULL,
    type                        TEXT NOT NULL CHECK (type IN ('texte', 'entier', 'booleen')),
    valeur                      TEXT NOT NULL,
    modifiable_par_utilisateur  INTEGER NOT NULL DEFAULT 1 CHECK (modifiable_par_utilisateur IN (0, 1)),
    mis_a_jour_le               INTEGER NOT NULL,
    mis_a_jour_par              TEXT,
    PRIMARY KEY (etablissement_id, cle)
);

CREATE TABLE sequence_numerotation (
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    type_document     TEXT NOT NULL,
    exercice          TEXT NOT NULL,
    compteur          INTEGER NOT NULL DEFAULT 0,
    format            TEXT NOT NULL,
    PRIMARY KEY (etablissement_id, type_document, exercice)
);

CREATE TABLE document (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    type              TEXT NOT NULL,
    entite            TEXT NOT NULL,
    entite_id         TEXT NOT NULL,
    chemin            TEXT NOT NULL,
    empreinte_sha256  TEXT NOT NULL CHECK (length(empreinte_sha256) = 64),
    taille            INTEGER NOT NULL CHECK (taille >= 0),
    modele_id         TEXT NOT NULL,
    modele_version    INTEGER NOT NULL,
    produit_le        INTEGER NOT NULL,
    produit_par       TEXT,
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    supprime_le       INTEGER,
    supprime_par      TEXT
);
CREATE INDEX document_entite ON document (entite, entite_id);

CREATE TABLE piece_jointe (
    empreinte_sha256  TEXT PRIMARY KEY CHECK (length(empreinte_sha256) = 64),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    type_mime         TEXT NOT NULL,
    taille            INTEGER NOT NULL CHECK (taille >= 0),
    chemin            TEXT NOT NULL,
    references_       INTEGER NOT NULL DEFAULT 0,
    cree_le           INTEGER NOT NULL,
    identifiant_poste TEXT NOT NULL
);

-- Journal d'audit : ajout seul, chaîné (R-27). Aucune colonne de suppression.
CREATE TABLE journal_audit (
    id                    TEXT PRIMARY KEY CHECK (length(id) = 26),
    horodatage            INTEGER NOT NULL,
    utilisateur_id        TEXT,
    role_actif            TEXT,
    etablissement_id      TEXT NOT NULL REFERENCES etablissement(id),
    entite                TEXT NOT NULL,
    entite_id             TEXT NOT NULL,
    operation             TEXT NOT NULL,
    avant                 TEXT,
    apres                 TEXT,
    contexte              TEXT NOT NULL,
    poste                 TEXT NOT NULL,
    empreinte_machine     TEXT NOT NULL,
    correlation_id        TEXT NOT NULL,
    empreinte_precedente  TEXT NOT NULL CHECK (length(empreinte_precedente) = 64),
    empreinte             TEXT NOT NULL CHECK (length(empreinte) = 64)
);
CREATE INDEX journal_audit_entite ON journal_audit (entite, entite_id, horodatage);
CREATE INDEX journal_audit_horodatage ON journal_audit (etablissement_id, horodatage);
-- Interdiction mécanique de la mise à jour et de la suppression : la logique est en Rust (R-15),
-- ces déclencheurs ne font que refuser, ils ne calculent rien.
CREATE TRIGGER journal_audit_ajout_seul_update BEFORE UPDATE ON journal_audit
BEGIN SELECT RAISE(ABORT, 'journal_audit est en ajout seul'); END;
CREATE TRIGGER journal_audit_ajout_seul_delete BEFORE DELETE ON journal_audit
BEGIN SELECT RAISE(ABORT, 'journal_audit est en ajout seul'); END;

CREATE TABLE outbox (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    type_evenement    TEXT NOT NULL,
    version           INTEGER NOT NULL CHECK (version >= 1),
    entite            TEXT NOT NULL,
    entite_id         TEXT NOT NULL,
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    acteur_id         TEXT,
    horodatage        INTEGER NOT NULL,
    identifiant_poste TEXT NOT NULL,
    charge_utile      TEXT NOT NULL,
    correlation_id    TEXT NOT NULL,
    consomme_le       INTEGER
);
CREATE INDEX outbox_a_consommer ON outbox (consomme_le);

CREATE TABLE sauvegarde (
    id                       TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id         TEXT NOT NULL REFERENCES etablissement(id),
    horodatage               INTEGER NOT NULL,
    destination              TEXT NOT NULL,
    taille                   INTEGER NOT NULL CHECK (taille >= 0),
    empreinte_sha256         TEXT NOT NULL CHECK (length(empreinte_sha256) = 64),
    version_schema           INTEGER NOT NULL,
    version_applicative      TEXT NOT NULL,
    resultat                 TEXT NOT NULL CHECK (resultat IN ('reussie', 'echouee')),
    dernier_test_restauration INTEGER,
    identifiant_poste        TEXT NOT NULL
);
