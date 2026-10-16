-- Registre du personnel (SPEC_PHASE_6 §2) : catégories et fonctions propres à l'école, sans liste
-- fermée (T-39) ; un membre par personne, l'enseignant de la phase 4 partageant sa personne étant le
-- même membre (Q-63). Rien ne se supprime : une catégorie ou une fonction se retire, un membre
-- part, une pièce se retire du dossier en restant dans la base (R-14).

CREATE TABLE categorie_personnel (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    code              TEXT,
    libelle           TEXT NOT NULL CHECK (length(trim(libelle)) > 0),
    ordre             INTEGER NOT NULL,
    active            INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE UNIQUE INDEX categorie_personnel_code ON categorie_personnel (etablissement_id, code) WHERE code IS NOT NULL;

CREATE TABLE fonction_personnel (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    categorie_id      TEXT NOT NULL REFERENCES categorie_personnel(id),
    code              TEXT,
    libelle           TEXT NOT NULL CHECK (length(trim(libelle)) > 0),
    ordre             INTEGER NOT NULL,
    active            INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE UNIQUE INDEX fonction_personnel_code ON fonction_personnel (etablissement_id, code) WHERE code IS NOT NULL;

CREATE TABLE membre_personnel (
    id                   TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id     TEXT NOT NULL REFERENCES etablissement(id),
    personne_id          TEXT NOT NULL UNIQUE REFERENCES personne(id),
    matricule            TEXT NOT NULL CHECK (length(trim(matricule)) BETWEEN 1 AND 20),
    categorie_id         TEXT NOT NULL REFERENCES categorie_personnel(id),
    fonction             TEXT NOT NULL CHECK (length(trim(fonction)) > 0),
    arrive_le            TEXT CHECK (arrive_le IS NULL OR length(arrive_le) = 10),
    parti_le             TEXT CHECK (parti_le IS NULL OR length(parti_le) = 10),
    motif_depart         TEXT NOT NULL DEFAULT '',
    fonctionnaire        INTEGER NOT NULL DEFAULT 0 CHECK (fonctionnaire IN (0, 1)),
    numero_fonctionnaire TEXT NOT NULL DEFAULT '',
    numero_inps          TEXT NOT NULL DEFAULT '',
    prevenir_nom         TEXT NOT NULL DEFAULT '',
    prevenir_telephone   TEXT NOT NULL DEFAULT '',
    -- Moyen de paiement, en JSON : {"mode":"especes"}, {"mode":"mobile_money","numero":…}.
    paiement             TEXT NOT NULL DEFAULT '{"mode":"especes"}',
    -- Diplômes et qualifications, en JSON : [{"intitule":…,"delivre_par":…,"annee":…}].
    diplomes             TEXT NOT NULL DEFAULT '[]',
    utilisateur_id       TEXT REFERENCES utilisateur(id),
    cree_le              INTEGER NOT NULL,
    cree_par             TEXT,
    mis_a_jour_le        INTEGER NOT NULL,
    mis_a_jour_par       TEXT,
    identifiant_poste    TEXT NOT NULL
);
CREATE UNIQUE INDEX membre_personnel_matricule ON membre_personnel (etablissement_id, matricule);
-- Un compte ne représente qu'un membre.
CREATE UNIQUE INDEX membre_personnel_compte ON membre_personnel (utilisateur_id) WHERE utilisateur_id IS NOT NULL;

CREATE TABLE piece_dossier (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    membre_id         TEXT NOT NULL REFERENCES membre_personnel(id),
    intitule          TEXT NOT NULL CHECK (length(trim(intitule)) > 0),
    nature            TEXT NOT NULL CHECK (nature IN ('identite', 'diplome', 'engagement', 'autre')),
    nom_fichier       TEXT NOT NULL,
    contenu           BLOB NOT NULL,
    taille            INTEGER NOT NULL CHECK (taille > 0),
    empreinte_sha256  TEXT NOT NULL CHECK (length(empreinte_sha256) = 64),
    active            INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE INDEX piece_dossier_membre ON piece_dossier (membre_id);

CREATE TRIGGER categorie_personnel_sans_suppression BEFORE DELETE ON categorie_personnel
BEGIN SELECT RAISE(ABORT, 'une catégorie de personnel ne se supprime pas'); END;
CREATE TRIGGER fonction_personnel_sans_suppression BEFORE DELETE ON fonction_personnel
BEGIN SELECT RAISE(ABORT, 'une fonction de personnel ne se supprime pas'); END;
CREATE TRIGGER membre_personnel_sans_suppression BEFORE DELETE ON membre_personnel
BEGIN SELECT RAISE(ABORT, 'un membre du personnel ne se supprime pas'); END;
CREATE TRIGGER piece_dossier_sans_suppression BEFORE DELETE ON piece_dossier
BEGIN SELECT RAISE(ABORT, 'une pièce du dossier ne se supprime pas'); END;
