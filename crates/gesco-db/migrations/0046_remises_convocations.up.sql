-- Remise des documents et convocations (SPEC_PHASE_5 §6 et §7, blueprint §8.5bis) : un bulletin
-- publié n'est pas un bulletin remis ; une convocation sait par quel canal elle est partie, si
-- elle a été reçue, et la suite qu'elle a eue. Rien ne se supprime (R-14).

CREATE TABLE remise_document (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    bulletin_id       TEXT NOT NULL REFERENCES bulletin(id),
    inscription_id    TEXT NOT NULL REFERENCES inscription(id),
    periode_id        TEXT NOT NULL REFERENCES periode_academique(id),
    statut            TEXT NOT NULL CHECK (statut IN ('a_remettre', 'remis', 'signe', 'non_retire')),
    remis_a           TEXT CHECK (remis_a IN ('responsable', 'eleve', 'personne_autorisee', 'tiers')),
    responsable_id    TEXT REFERENCES responsable(id),
    nom_retrait       TEXT NOT NULL DEFAULT '' CHECK (length(nom_retrait) <= 120),
    signataire        TEXT NOT NULL DEFAULT '' CHECK (length(signataire) <= 120),
    remis_le          INTEGER,
    signe_le          INTEGER,
    remis_par         TEXT REFERENCES utilisateur(id),
    publie_le         INTEGER NOT NULL,
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    UNIQUE (inscription_id, periode_id),
    CHECK (statut IN ('a_remettre', 'non_retire') OR remis_a IS NOT NULL)
);
CREATE INDEX remise_document_statut ON remise_document (etablissement_id, statut);

CREATE TABLE convocation (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    inscription_id    TEXT NOT NULL REFERENCES inscription(id),
    responsable_id    TEXT REFERENCES responsable(id),
    sanction_id       TEXT REFERENCES sanction(id),
    motif             TEXT NOT NULL CHECK (length(trim(motif)) BETWEEN 1 AND 200),
    date              TEXT NOT NULL CHECK (length(date) = 10),
    heure             TEXT NOT NULL CHECK (length(heure) = 5),
    canal             TEXT NOT NULL CHECK (canal IN ('eleve', 'main_propre', 'sms', 'whatsapp')),
    accuse            TEXT NOT NULL CHECK (accuse IN ('confirme', 'transmis_non_confirme', 'sans_reponse')),
    suite             TEXT NOT NULL CHECK (suite IN ('en_attente', 'honoree', 'non_honoree', 'reportee')),
    emise_le          INTEGER NOT NULL,
    emise_par         TEXT NOT NULL REFERENCES utilisateur(id),
    annulee           INTEGER NOT NULL DEFAULT 0 CHECK (annulee IN (0, 1)),
    motif_annulation  TEXT NOT NULL DEFAULT '',
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    CHECK (annulee = 0 OR length(trim(motif_annulation)) > 0)
);
CREATE INDEX convocation_inscription ON convocation (inscription_id, date);

CREATE TRIGGER remise_document_sans_suppression BEFORE DELETE ON remise_document
BEGIN SELECT RAISE(ABORT, 'une remise de document ne se supprime pas'); END;

CREATE TRIGGER convocation_sans_suppression BEFORE DELETE ON convocation
BEGIN SELECT RAISE(ABORT, 'une convocation ne se supprime pas'); END;
