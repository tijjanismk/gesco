-- Bibliothèque de cours (phase 12, lot 1) : paquets installés, programme (chapitres livrés et de
-- l'école), contenus, versions écrites par l'école, relectures, retraits, fichiers joints,
-- vidéos, droit de publier donné à un maître, correspondance des disciplines, index de
-- recherche. Aucune table n'a d'année scolaire : un contenu dure d'une année à l'autre.
--
-- Le livré n'est jamais modifié par l'école : les lignes d'origine `livre` sont réécrites par
-- l'installation d'un paquet, par leur clé ; les lignes d'origine `ecole`, les versions, les
-- relectures, les retraits, la correspondance et les droits ne le sont jamais. Rien ne se
-- supprime (R-14) : un retrait se remet, un brouillon s'abandonne, un contenu qui quitte le
-- paquet est marqué « plus livré ». Seul l'index de recherche, refait d'après le reste, perd
-- et reprend ses lignes.

CREATE TABLE bibliotheque_paquet (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    code              TEXT NOT NULL CHECK (length(trim(code)) BETWEEN 1 AND 60),
    version           TEXT NOT NULL CHECK (length(trim(version)) BETWEEN 1 AND 30),
    date_paquet       TEXT NOT NULL DEFAULT '',
    gesco_minimum     TEXT NOT NULL DEFAULT '',
    nom_fichier       TEXT NOT NULL,
    taille            INTEGER NOT NULL CHECK (taille > 0),
    empreinte_sha256  TEXT NOT NULL CHECK (length(empreinte_sha256) = 64),
    -- Libellés de référence des disciplines du paquet, en JSON (`[{"code","libelle"}]`).
    disciplines       TEXT NOT NULL DEFAULT '[]',
    actif             INTEGER NOT NULL DEFAULT 1 CHECK (actif IN (0, 1)),
    installe_le       INTEGER NOT NULL,
    installe_par      TEXT REFERENCES utilisateur(id),
    remplace_le       INTEGER,
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE UNIQUE INDEX bibliotheque_paquet_actif ON bibliotheque_paquet (etablissement_id, code)
    WHERE actif = 1;

CREATE TRIGGER bibliotheque_paquet_sans_suppression BEFORE DELETE ON bibliotheque_paquet
BEGIN SELECT RAISE(ABORT, 'un paquet de la bibliothèque ne se supprime pas'); END;

-- La colonne vertébrale : un chapitre du programme, livré (repéré par les codes nationaux du
-- niveau et de la discipline) ou ajouté par l'école (repéré par son niveau et sa matière).
CREATE TABLE chapitre_programme (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    cle               TEXT NOT NULL CHECK (length(trim(cle)) BETWEEN 1 AND 300),
    origine           TEXT NOT NULL CHECK (origine IN ('livre', 'ecole')),
    paquet_code       TEXT,
    niveau_code       TEXT NOT NULL,
    discipline_code   TEXT,
    niveau_id         TEXT REFERENCES niveau(id),
    matiere_id        TEXT REFERENCES matiere(id),
    partie            TEXT NOT NULL DEFAULT '',
    titre             TEXT NOT NULL CHECK (length(trim(titre)) BETWEEN 1 AND 200),
    ordre             INTEGER NOT NULL,
    -- Chapitre de l'école : la clé du chapitre après lequel il se range (aucune : en tête).
    apres_cle         TEXT,
    a_confirmer       INTEGER NOT NULL DEFAULT 0 CHECK (a_confirmer IN (0, 1)),
    plus_livre_le     INTEGER,
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    CHECK (origine = 'ecole' OR (paquet_code IS NOT NULL AND discipline_code IS NOT NULL)),
    CHECK (origine = 'livre' OR (niveau_id IS NOT NULL AND matiere_id IS NOT NULL))
);
CREATE UNIQUE INDEX chapitre_programme_cle ON chapitre_programme (etablissement_id, cle);
CREATE INDEX chapitre_programme_niveau ON chapitre_programme (etablissement_id, niveau_code, ordre);

CREATE TRIGGER chapitre_programme_sans_suppression BEFORE DELETE ON chapitre_programme
BEGIN SELECT RAISE(ABORT, 'un chapitre de la bibliothèque ne se supprime pas'); END;

-- Séries d'un chapitre du lycée ; aucune série active : toutes les séries du niveau.
CREATE TABLE chapitre_serie (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    chapitre_id       TEXT NOT NULL REFERENCES chapitre_programme(id),
    serie_code        TEXT NOT NULL CHECK (length(trim(serie_code)) BETWEEN 1 AND 20),
    retiree_le        INTEGER,
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE UNIQUE INDEX chapitre_serie_unique ON chapitre_serie (chapitre_id, serie_code);

CREATE TRIGGER chapitre_serie_sans_suppression BEFORE DELETE ON chapitre_serie
BEGIN SELECT RAISE(ABORT, 'une série de chapitre ne se supprime pas'); END;

-- Un contenu : leçon, sujet, corrigé ou document. Le texte livré reste dans le paquet ; la base
-- garde son nom de fichier et son empreinte. Le texte de l'école est dans `version_contenu`.
CREATE TABLE contenu (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    cle               TEXT NOT NULL CHECK (length(trim(cle)) BETWEEN 1 AND 300),
    origine           TEXT NOT NULL CHECK (origine IN ('livre', 'ecole')),
    nature            TEXT NOT NULL CHECK (nature IN ('lecon', 'sujet', 'corrige', 'document')),
    chapitre_id       TEXT NOT NULL REFERENCES chapitre_programme(id),
    titre             TEXT NOT NULL CHECK (length(trim(titre)) BETWEEN 1 AND 200),
    -- Phrase de présentation d'un document de l'école.
    presentation      TEXT NOT NULL DEFAULT '',
    ordre             INTEGER NOT NULL,
    apres_cle         TEXT,
    langue            TEXT NOT NULL DEFAULT 'fr' CHECK (length(langue) BETWEEN 2 AND 8),
    -- Le corrigé d'un sujet, et l'inverse, par sa clé.
    lie_cle           TEXT,
    examen            TEXT CHECK (examen IS NULL OR examen IN ('def', 'bac')),
    serie_code        TEXT,
    session           INTEGER CHECK (session IS NULL OR session BETWEEN 1950 AND 2200),
    paquet_code       TEXT,
    fichier           TEXT,
    empreinte_sha256  TEXT CHECK (empreinte_sha256 IS NULL OR length(empreinte_sha256) = 64),
    -- Liste de faits à cocher jointe à une leçon livrée : nom du fichier dans le paquet.
    faits             TEXT,
    plus_livre_le     INTEGER,
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    CHECK (origine = 'ecole' OR (paquet_code IS NOT NULL AND fichier IS NOT NULL
        AND empreinte_sha256 IS NOT NULL))
);
CREATE UNIQUE INDEX contenu_cle ON contenu (etablissement_id, cle);
CREATE INDEX contenu_chapitre ON contenu (chapitre_id, ordre);

CREATE TRIGGER contenu_sans_suppression BEFORE DELETE ON contenu
BEGIN SELECT RAISE(ABORT, 'un contenu de la bibliothèque ne se supprime pas'); END;

-- Une version écrite par l'école. Un seul brouillon et une seule version publiée à la fois par
-- contenu ; le brouillon se réécrit sur place, avec un compteur de révision ; une version qui
-- n'est plus brouillon ne change plus de texte.
CREATE TABLE version_contenu (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    contenu_id        TEXT NOT NULL REFERENCES contenu(id),
    numero            INTEGER NOT NULL CHECK (numero >= 1),
    texte             TEXT NOT NULL,
    empreinte_sha256  TEXT NOT NULL CHECK (length(empreinte_sha256) = 64),
    etat              TEXT NOT NULL CHECK (etat IN ('brouillon', 'soumise', 'renvoyee',
                          'publiee', 'remplacee', 'abandonnee')),
    -- « Revenir à la version livrée » : une version sans texte qui suit le même circuit.
    retour_livre      INTEGER NOT NULL DEFAULT 0 CHECK (retour_livre IN (0, 1)),
    depart_version_id TEXT REFERENCES version_contenu(id),
    -- Empreinte du texte livré dont la version est partie, s'il y en a un.
    depart_livre_sha256 TEXT CHECK (depart_livre_sha256 IS NULL
                          OR length(depart_livre_sha256) = 64),
    revision          INTEGER NOT NULL DEFAULT 1 CHECK (revision >= 1),
    auteur_id         TEXT NOT NULL REFERENCES utilisateur(id),
    soumise_le        INTEGER,
    decidee_le        INTEGER,
    decidee_par       TEXT REFERENCES utilisateur(id),
    commentaire       TEXT NOT NULL DEFAULT '',
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    CHECK (retour_livre = 0 OR texte = '')
);
CREATE UNIQUE INDEX version_contenu_numero ON version_contenu (contenu_id, numero);
CREATE UNIQUE INDEX version_contenu_brouillon ON version_contenu (contenu_id)
    WHERE etat = 'brouillon';
CREATE UNIQUE INDEX version_contenu_publiee ON version_contenu (contenu_id)
    WHERE etat = 'publiee';
CREATE INDEX version_contenu_etat ON version_contenu (etablissement_id, etat);

CREATE TRIGGER version_contenu_sans_suppression BEFORE DELETE ON version_contenu
BEGIN SELECT RAISE(ABORT, 'une version d''un contenu ne se supprime pas'); END;

CREATE TRIGGER version_contenu_texte_fige BEFORE UPDATE OF texte ON version_contenu
WHEN OLD.etat <> 'brouillon' AND NEW.texte IS NOT OLD.texte
BEGIN SELECT RAISE(ABORT, 'une version soumise ne change plus'); END;

-- « Relue » : le texte relu, par son empreinte.
CREATE TABLE relecture_contenu (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    contenu_id        TEXT NOT NULL REFERENCES contenu(id),
    empreinte_sha256  TEXT NOT NULL CHECK (length(empreinte_sha256) = 64),
    relue_le          INTEGER NOT NULL,
    relue_par         TEXT NOT NULL REFERENCES utilisateur(id),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE INDEX relecture_contenu_contenu ON relecture_contenu (contenu_id, empreinte_sha256);

CREATE TRIGGER relecture_contenu_sans_suppression BEFORE DELETE ON relecture_contenu
BEGIN SELECT RAISE(ABORT, 'une relecture ne se supprime pas'); END;

-- Un retrait et sa remise. La cible est une clé stable : clé du contenu ou du chapitre, code de
-- la discipline (ou identifiant de la matière de l'école) avec son niveau et sa série, code du
-- niveau, code du paquet. Retirer de nouveau crée une ligne nouvelle.
CREATE TABLE retrait_bibliotheque (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    grain             TEXT NOT NULL CHECK (grain IN ('contenu', 'chapitre', 'matiere', 'niveau',
                          'paquet')),
    cible             TEXT NOT NULL CHECK (length(trim(cible)) BETWEEN 1 AND 300),
    niveau_code       TEXT,
    serie_code        TEXT,
    retire_le         INTEGER NOT NULL,
    retire_par        TEXT NOT NULL REFERENCES utilisateur(id),
    remis_le          INTEGER,
    remis_par         TEXT REFERENCES utilisateur(id),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    CHECK (grain <> 'matiere' OR niveau_code IS NOT NULL)
);
CREATE INDEX retrait_bibliotheque_actif ON retrait_bibliotheque (etablissement_id, grain, cible)
    WHERE remis_le IS NULL;

CREATE TRIGGER retrait_bibliotheque_sans_suppression BEFORE DELETE ON retrait_bibliotheque
BEGIN SELECT RAISE(ABORT, 'un retrait ne se supprime pas'); END;

-- Image ou document joint par l'école, dans la base chiffrée, gardé une fois par empreinte.
CREATE TABLE fichier_bibliotheque (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    empreinte_sha256  TEXT NOT NULL CHECK (length(empreinte_sha256) = 64),
    nature            TEXT NOT NULL CHECK (nature IN ('image', 'document')),
    type_mime         TEXT NOT NULL CHECK (type_mime IN ('image/svg+xml', 'image/png',
                          'image/jpeg', 'application/pdf')),
    nom_fichier       TEXT NOT NULL,
    taille            INTEGER NOT NULL CHECK (taille > 0),
    contenu           BLOB NOT NULL,
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE UNIQUE INDEX fichier_bibliotheque_empreinte
    ON fichier_bibliotheque (etablissement_id, empreinte_sha256);

CREATE TRIGGER fichier_bibliotheque_sans_suppression BEFORE DELETE ON fichier_bibliotheque
BEGIN SELECT RAISE(ABORT, 'un fichier de la bibliothèque ne se supprime pas'); END;

-- Fichiers d'un document de l'école, dans l'ordre ; un fichier retiré du document reste en base.
CREATE TABLE fichier_contenu (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    contenu_id        TEXT NOT NULL REFERENCES contenu(id),
    fichier_id        TEXT NOT NULL REFERENCES fichier_bibliotheque(id),
    ordre             INTEGER NOT NULL,
    retire_le         INTEGER,
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE INDEX fichier_contenu_contenu ON fichier_contenu (contenu_id, ordre);

CREATE TRIGGER fichier_contenu_sans_suppression BEFORE DELETE ON fichier_contenu
BEGIN SELECT RAISE(ABORT, 'un fichier d''un document ne se supprime pas'); END;

-- Vidéo de l'école : le fichier est dans `videos\` du poste principal, nommé par son empreinte,
-- hors de la base et de la sauvegarde.
CREATE TABLE video_bibliotheque (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    empreinte_sha256  TEXT NOT NULL CHECK (length(empreinte_sha256) = 64),
    nom_fichier       TEXT NOT NULL,
    taille            INTEGER NOT NULL CHECK (taille > 0),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE UNIQUE INDEX video_bibliotheque_empreinte
    ON video_bibliotheque (etablissement_id, empreinte_sha256);

CREATE TRIGGER video_bibliotheque_sans_suppression BEFORE DELETE ON video_bibliotheque
BEGIN SELECT RAISE(ABORT, 'une vidéo de la bibliothèque ne se supprime pas'); END;

-- Le droit de publier donné nommément à un maître, puis retiré.
CREATE TABLE droit_publication (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    utilisateur_id    TEXT NOT NULL REFERENCES utilisateur(id),
    donne_le          INTEGER NOT NULL,
    donne_par         TEXT NOT NULL REFERENCES utilisateur(id),
    retire_le         INTEGER,
    retire_par        TEXT REFERENCES utilisateur(id),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE UNIQUE INDEX droit_publication_actif ON droit_publication (etablissement_id, utilisateur_id)
    WHERE retire_le IS NULL;

CREATE TRIGGER droit_publication_sans_suppression BEFORE DELETE ON droit_publication
BEGIN SELECT RAISE(ABORT, 'un droit de publier ne se supprime pas'); END;

-- Sous quelle matière de l'école se range une discipline de référence, dans un niveau. Posée
-- par code exact à l'installation du paquet, changée par la direction ; une mise à jour du
-- paquet ne la réécrit jamais. Matière vide : les contenus restent sous le libellé de référence.
CREATE TABLE correspondance_discipline (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    discipline_code   TEXT NOT NULL CHECK (length(trim(discipline_code)) BETWEEN 1 AND 20),
    niveau_code       TEXT NOT NULL,
    matiere_id        TEXT REFERENCES matiere(id),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE UNIQUE INDEX correspondance_discipline_unique
    ON correspondance_discipline (etablissement_id, discipline_code, niveau_code);

CREATE TRIGGER correspondance_discipline_sans_suppression BEFORE DELETE ON correspondance_discipline
BEGIN SELECT RAISE(ABORT, 'une correspondance de discipline ne se supprime pas'); END;

-- Index plein texte du titre et du texte de la version visible de chaque contenu. Il se refait
-- d'après le paquet et les versions : ses lignes s'effacent et se réécrivent.
CREATE VIRTUAL TABLE recherche_bibliotheque USING fts5(
    contenu_id UNINDEXED,
    titre,
    texte,
    tokenize = 'unicode61 remove_diacritics 2'
);
