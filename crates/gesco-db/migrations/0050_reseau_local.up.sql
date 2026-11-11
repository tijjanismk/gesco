-- Réseau local de l'école (SPEC_PHASE_11_A §4, ADR du 10 novembre 2026).
--
-- Le poste principal garde, dans la base chiffrée, l'état du réseau (actif ou non, port, son
-- certificat et sa clé) et la liste des postes reliés. D'un poste relié il ne garde que
-- l'empreinte du secret : le secret lui-même ne vit que dans le trousseau de ce poste.
-- Rien ne se supprime (R-14) : un poste se révoque, et son nom redevient libre.

CREATE TABLE reseau_local (
    etablissement_id  TEXT PRIMARY KEY REFERENCES etablissement(id),
    actif             INTEGER NOT NULL DEFAULT 0 CHECK (actif IN (0, 1)),
    port              INTEGER NOT NULL DEFAULT 47800 CHECK (port BETWEEN 1024 AND 65535),
    certificat        BLOB,
    cle               BLOB,
    fabrique_le       INTEGER,
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    -- Le certificat et sa clé vont ensemble, avec leur date de fabrication.
    CHECK ((certificat IS NULL) = (cle IS NULL) AND (certificat IS NULL) = (fabrique_le IS NULL))
);

CREATE TABLE poste_relie (
    id                 TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id   TEXT NOT NULL REFERENCES etablissement(id),
    nom                TEXT NOT NULL CHECK (length(trim(nom)) BETWEEN 1 AND 40),
    empreinte_secret   TEXT NOT NULL CHECK (length(empreinte_secret) = 64),
    relie_le           INTEGER NOT NULL,
    relie_par          TEXT REFERENCES utilisateur(id),
    derniere_connexion INTEGER,
    revoque_le         INTEGER,
    revoque_par        TEXT REFERENCES utilisateur(id),
    cree_le            INTEGER NOT NULL,
    cree_par           TEXT,
    mis_a_jour_le      INTEGER NOT NULL,
    mis_a_jour_par     TEXT,
    identifiant_poste  TEXT NOT NULL
);
-- Un nom ne désigne qu'un poste en service : c'est lui qui signe les écritures au journal.
CREATE UNIQUE INDEX poste_relie_nom_en_service ON poste_relie (etablissement_id, nom)
    WHERE revoque_le IS NULL;
CREATE INDEX poste_relie_etablissement ON poste_relie (etablissement_id, relie_le);

CREATE TRIGGER poste_relie_sans_suppression BEFORE DELETE ON poste_relie
BEGIN SELECT RAISE(ABORT, 'un poste relié ne se supprime pas, il se révoque'); END;
