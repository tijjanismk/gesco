-- Devoirs à rendre, phase 12, lot 5 (ADR « tables des lots 2 à 5 », « copies photographiées »
-- et « report d'une note de devoir au bulletin ») : le devoir que le maître donne à une classe,
-- les rendus des élèves (par le portail ou sur papier), les photos des copies rangées une fois
-- par empreinte, et le lien d'une évaluation à son devoir. Aucune table ne copie un nom : l'élève
-- est désigné par son identifiant. Rien ne se supprime ; seul le contenu d'une photo d'une année
-- close peut être vidé (`libere_le`).

CREATE TABLE devoir (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    annee_id          TEXT NOT NULL REFERENCES annee_scolaire(id),
    classe_id         TEXT NOT NULL REFERENCES classe(id),
    matiere_id        TEXT NOT NULL REFERENCES matiere(id),
    auteur_id         TEXT NOT NULL REFERENCES utilisateur(id),
    titre             TEXT NOT NULL CHECK (length(trim(titre)) BETWEEN 1 AND 200),
    -- Consignes au format des leçons (texte, formules, images).
    consignes         TEXT NOT NULL DEFAULT '',
    -- Leçon liée, facultative.
    contenu_id        TEXT REFERENCES contenu(id),
    -- Date et heure limites (millisecondes).
    date_limite       INTEGER NOT NULL,
    -- Barème en centièmes, comme `evaluation.bareme` : de 10 à 100.
    bareme            INTEGER NOT NULL DEFAULT 2000 CHECK (bareme BETWEEN 1000 AND 10000),
    forme_rendu       TEXT NOT NULL DEFAULT 'texte_ou_photos'
                          CHECK (forme_rendu IN ('texte', 'photos', 'texte_ou_photos', 'papier')),
    etat              TEXT NOT NULL DEFAULT 'brouillon'
                          CHECK (etat IN ('brouillon', 'donne', 'clos')),
    donne_le          INTEGER,
    clos_le           INTEGER,
    -- « Rendre les copies corrigées » : notes et commentaires visibles des élèves.
    copies_rendues_le INTEGER,
    -- Dernier report au bulletin (pour signaler une note changée depuis).
    reporte_le        INTEGER,
    -- Devoir copié d'un autre (« Donner aussi à une autre classe »).
    copie_de          TEXT REFERENCES devoir(id),
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    CHECK (etat = 'brouillon' OR donne_le IS NOT NULL),
    CHECK (etat <> 'clos' OR clos_le IS NOT NULL)
);
CREATE INDEX devoir_classe ON devoir (etablissement_id, annee_id, classe_id, matiere_id);
CREATE TRIGGER devoir_sans_suppression BEFORE DELETE ON devoir
BEGIN SELECT RAISE(ABORT, 'un devoir ne se supprime pas'); END;

-- Chaque rendu d'un élève ; le dernier (plus grand numéro) compte, les précédents restent.
CREATE TABLE rendu_devoir (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    devoir_id         TEXT NOT NULL REFERENCES devoir(id),
    eleve_id          TEXT NOT NULL REFERENCES eleve(id),
    numero            INTEGER NOT NULL CHECK (numero >= 1),
    origine           TEXT NOT NULL CHECK (origine IN ('portail', 'papier')),
    texte             TEXT NOT NULL DEFAULT '',
    rendu_le          INTEGER NOT NULL,
    en_retard         INTEGER NOT NULL DEFAULT 0 CHECK (en_retard IN (0, 1)),
    -- Note en centièmes, sur le barème du devoir.
    note              INTEGER CHECK (note IS NULL OR note >= 0),
    commentaire       TEXT NOT NULL DEFAULT '',
    note_le           INTEGER,
    note_par          TEXT REFERENCES utilisateur(id),
    identifiant_poste TEXT NOT NULL
);
CREATE UNIQUE INDEX rendu_devoir_numero ON rendu_devoir (devoir_id, eleve_id, numero);
CREATE INDEX rendu_devoir_eleve ON rendu_devoir (eleve_id);
CREATE TRIGGER rendu_devoir_sans_suppression BEFORE DELETE ON rendu_devoir
BEGIN SELECT RAISE(ABORT, 'un rendu de devoir ne se supprime pas'); END;

-- Photos des copies, une fois par empreinte, sans métadonnée ; le contenu se vide seulement
-- pour libérer la place d'une année close.
CREATE TABLE fichier_rendu (
    empreinte         TEXT PRIMARY KEY CHECK (length(empreinte) = 64),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    contenu           BLOB,
    taille            INTEGER NOT NULL CHECK (taille > 0),
    sorte             TEXT NOT NULL CHECK (sorte IN ('jpeg', 'png')),
    cree_le           INTEGER NOT NULL,
    libere_le         INTEGER,
    CHECK ((contenu IS NULL) = (libere_le IS NOT NULL))
);
CREATE TRIGGER fichier_rendu_sans_suppression BEFORE DELETE ON fichier_rendu
BEGIN SELECT RAISE(ABORT, 'une photo de copie ne se supprime pas'); END;

CREATE TABLE photo_rendu (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    rendu_id          TEXT NOT NULL REFERENCES rendu_devoir(id),
    ordre             INTEGER NOT NULL CHECK (ordre BETWEEN 1 AND 5),
    empreinte         TEXT NOT NULL REFERENCES fichier_rendu(empreinte)
);
CREATE UNIQUE INDEX photo_rendu_ordre ON photo_rendu (rendu_id, ordre);
CREATE INDEX photo_rendu_empreinte ON photo_rendu (empreinte);
CREATE TRIGGER photo_rendu_sans_suppression BEFORE DELETE ON photo_rendu
BEGIN SELECT RAISE(ABORT, 'une photo de copie ne se supprime pas'); END;

-- Une évaluation née du report d'un devoir ; les autres restent « saisie ».
ALTER TABLE evaluation ADD COLUMN origine TEXT NOT NULL DEFAULT 'saisie'
    CHECK (origine IN ('saisie', 'devoir_en_ligne'));
ALTER TABLE evaluation ADD COLUMN devoir_id TEXT REFERENCES devoir(id);
CREATE UNIQUE INDEX evaluation_devoir ON evaluation (devoir_id) WHERE devoir_id IS NOT NULL;
