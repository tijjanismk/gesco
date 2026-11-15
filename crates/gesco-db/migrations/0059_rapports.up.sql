-- Rapports de l'école (SPEC_PHASE_9 §8.6) : un rapport de chaque type par année et par
-- périmètre. Ses textes sont rédigés par la direction ; ses tableaux se calculent à sa date
-- d'arrêt. À l'arrêt, le document imprimé et le contenu figé (en-tête, textes, tableaux) sont
-- gardés avec lui, pour qu'une réimpression donne le même papier. Rouvrir passe à la version
-- suivante ; l'ancien document reste dans `document`. Rien ne se supprime (R-14). Le type
-- « fermeture » et le périmètre « secondaire » sont prévus pour les étapes suivantes.

CREATE TABLE rapport_ecole (
    id                 TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id   TEXT NOT NULL REFERENCES etablissement(id),
    annee_id           TEXT NOT NULL REFERENCES annee_scolaire(id),
    type_rapport       TEXT NOT NULL CHECK (type_rapport IN ('succinct_rentree', 'rentree', 'fermeture')),
    perimetre          TEXT NOT NULL DEFAULT 'fondamental' CHECK (perimetre IN ('fondamental', 'secondaire')),
    etat               TEXT NOT NULL DEFAULT 'preparation' CHECK (etat IN ('preparation', 'arrete')),
    version            INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
    date_arret         TEXT NOT NULL,
    lieu               TEXT NOT NULL DEFAULT '',
    date_signature     TEXT,
    qualite_signataire TEXT NOT NULL DEFAULT '',
    document_id        TEXT REFERENCES document(id),
    contenu_fige       TEXT,
    arrete_le          INTEGER,
    arrete_par         TEXT,
    cree_le            INTEGER NOT NULL,
    cree_par           TEXT,
    mis_a_jour_le      INTEGER NOT NULL,
    mis_a_jour_par     TEXT,
    identifiant_poste  TEXT NOT NULL
);
CREATE UNIQUE INDEX rapport_ecole_unique ON rapport_ecole (annee_id, type_rapport, perimetre);
CREATE INDEX rapport_ecole_etablissement ON rapport_ecole (etablissement_id);

CREATE TRIGGER rapport_ecole_sans_suppression BEFORE DELETE ON rapport_ecole
BEGIN SELECT RAISE(ABORT, 'un rapport ne se supprime pas'); END;

CREATE TABLE texte_rapport (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    rapport_id        TEXT NOT NULL REFERENCES rapport_ecole(id),
    section           TEXT NOT NULL,
    texte             TEXT NOT NULL DEFAULT '',
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE UNIQUE INDEX texte_rapport_unique ON texte_rapport (rapport_id, section);

CREATE TRIGGER texte_rapport_sans_suppression BEFORE DELETE ON texte_rapport
BEGIN SELECT RAISE(ABORT, 'un texte de rapport ne se supprime pas'); END;
