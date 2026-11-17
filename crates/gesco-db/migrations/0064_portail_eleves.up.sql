-- Portail des élèves sur le Wi-Fi de l'école, phase 12, lot 4 (ADR « identité de l'élève » et
-- « tables des lots 2 à 5 ») : le code de l'élève (son empreinte seulement), les essais faux
-- comptés par matricule et par jour, et l'avancement d'un élève dans les leçons. Aucune table ne
-- copie un nom : l'élève est désigné par son identifiant, sa classe se lit par son inscription.
-- Rien ne se supprime.

CREATE TABLE code_eleve (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    eleve_id          TEXT NOT NULL REFERENCES eleve(id),
    empreinte         TEXT NOT NULL CHECK (length(empreinte) > 0),
    actif             INTEGER NOT NULL DEFAULT 1 CHECK (actif IN (0, 1)),
    remis_le          INTEGER NOT NULL,
    remis_par         TEXT,
    revoque_le        INTEGER,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    CHECK (actif = 1 OR revoque_le IS NOT NULL)
);
-- Un seul code actif par élève.
CREATE UNIQUE INDEX code_eleve_actif ON code_eleve (eleve_id) WHERE actif = 1;
CREATE INDEX code_eleve_eleve ON code_eleve (etablissement_id, eleve_id);
CREATE TRIGGER code_eleve_sans_suppression BEFORE DELETE ON code_eleve
BEGIN SELECT RAISE(ABORT, 'un code d''élève ne se supprime pas, il se remplace'); END;

-- Codes faux d'un matricule dans une journée (jour au format AAAA-MM-JJ, heure du Mali).
CREATE TABLE essai_code_eleve (
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    matricule         TEXT NOT NULL CHECK (length(matricule) BETWEEN 1 AND 64),
    jour              TEXT NOT NULL CHECK (length(jour) = 10),
    essais_faux       INTEGER NOT NULL DEFAULT 0 CHECK (essais_faux >= 0),
    bloque_le         INTEGER,
    mis_a_jour_le     INTEGER NOT NULL,
    PRIMARY KEY (etablissement_id, matricule, jour)
);
CREATE TRIGGER essai_code_eleve_sans_suppression BEFORE DELETE ON essai_code_eleve
BEGIN SELECT RAISE(ABORT, 'un essai compté ne se supprime pas'); END;

-- Avancement d'un élève dans une leçon, par année : ouverte (première et dernière fois), finie.
CREATE TABLE avancement_lecon (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    eleve_id          TEXT NOT NULL REFERENCES eleve(id),
    annee_id          TEXT NOT NULL REFERENCES annee_scolaire(id),
    contenu_id        TEXT NOT NULL REFERENCES contenu(id),
    empreinte_version TEXT NOT NULL CHECK (length(empreinte_version) > 0),
    ouvert_le         INTEGER NOT NULL,
    dernier_le        INTEGER NOT NULL,
    fini_le           INTEGER,
    mis_a_jour_le     INTEGER NOT NULL
);
CREATE UNIQUE INDEX avancement_lecon_unique ON avancement_lecon (eleve_id, annee_id, contenu_id);
CREATE INDEX avancement_lecon_contenu ON avancement_lecon (etablissement_id, annee_id, contenu_id);
CREATE TRIGGER avancement_lecon_sans_suppression BEFORE DELETE ON avancement_lecon
BEGIN SELECT RAISE(ABORT, 'un avancement ne se supprime pas'); END;
