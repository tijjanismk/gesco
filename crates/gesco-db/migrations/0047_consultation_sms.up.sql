-- Code d'accès et consultation par SMS entrant (SPEC_PHASE_5 §9, blueprint §25.4 et §25.5).
--
-- Le code d'accès est une identité du responsable : la base n'en garde que l'empreinte ; un seul
-- code actif par responsable, l'ancien révoqué quand un nouveau est remis. Chaque SMS reçu reste
-- au journal avec son issue et la réponse partie, le code masqué. Rien ne se supprime (R-14).
--
-- La file des SMS accepte une nature de plus, la réponse à une consultation. SQLite ne modifie pas
-- une contrainte CHECK : la table est reconstruite à l'identique, contrôle élargi. Elle n'est la
-- cible d'aucune clé étrangère avant cette migration.

DROP TRIGGER message_sms_sans_suppression;
CREATE TABLE message_sms_ancien AS SELECT * FROM message_sms;
DROP TABLE message_sms;
CREATE TABLE message_sms (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    nature            TEXT NOT NULL CHECK (nature IN ('bulletin', 'impaye', 'absence', 'convocation', 'sanction', 'essai', 'consultation')),
    cle_fait          TEXT NOT NULL CHECK (length(cle_fait) BETWEEN 1 AND 200),
    responsable_id    TEXT REFERENCES responsable(id),
    eleve_id          TEXT REFERENCES eleve(id),
    numero            TEXT NOT NULL CHECK (numero GLOB '+[0-9]*' AND length(numero) BETWEEN 9 AND 16),
    texte             TEXT NOT NULL CHECK (length(texte) BETWEEN 1 AND 480),
    sms               INTEGER NOT NULL CHECK (sms BETWEEN 1 AND 3),
    etat              TEXT NOT NULL CHECK (etat IN ('a_decider', 'en_attente', 'envoye', 'remis', 'echoue', 'annule', 'expire')),
    essais            INTEGER NOT NULL DEFAULT 0 CHECK (essais BETWEEN 0 AND 255),
    pas_avant         INTEGER NOT NULL,
    expire_le         INTEGER,
    reference         TEXT,
    envoye_le         INTEGER,
    remis_le          INTEGER,
    erreur            TEXT CHECK (erreur IN ('reseau', 'credit', 'numero_refuse', 'numero_injoignable', 'non_remis')),
    motif_annulation  TEXT NOT NULL DEFAULT '',
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    CHECK (etat <> 'annule' OR length(trim(motif_annulation)) > 0)
);
INSERT INTO message_sms SELECT * FROM message_sms_ancien;
DROP TABLE message_sms_ancien;
CREATE UNIQUE INDEX message_sms_fait ON message_sms (etablissement_id, cle_fait, numero);
CREATE INDEX message_sms_file ON message_sms (etablissement_id, etat, pas_avant);
CREATE INDEX message_sms_reference ON message_sms (reference) WHERE reference IS NOT NULL;
CREATE TRIGGER message_sms_sans_suppression BEFORE DELETE ON message_sms
BEGIN SELECT RAISE(ABORT, 'un message ne se supprime pas'); END;

CREATE TABLE code_acces (
    id                   TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id     TEXT NOT NULL REFERENCES etablissement(id),
    responsable_id       TEXT NOT NULL REFERENCES responsable(id),
    empreinte            TEXT NOT NULL CHECK (length(empreinte) > 0),
    etat                 TEXT NOT NULL CHECK (etat IN ('actif', 'revoque')),
    cree_le              INTEGER NOT NULL,
    cree_par             TEXT,
    revoque_le           INTEGER,
    derniere_utilisation INTEGER,
    mis_a_jour_le        INTEGER NOT NULL,
    mis_a_jour_par       TEXT,
    identifiant_poste    TEXT NOT NULL,
    CHECK (etat = 'actif' OR revoque_le IS NOT NULL)
);
-- Un seul code actif par responsable.
CREATE UNIQUE INDEX code_acces_actif ON code_acces (responsable_id) WHERE etat = 'actif';
CREATE INDEX code_acces_responsable ON code_acces (etablissement_id, responsable_id);

CREATE TRIGGER code_acces_sans_suppression BEFORE DELETE ON code_acces
BEGIN SELECT RAISE(ABORT, 'un code d''accès ne se supprime pas, il se révoque'); END;

CREATE TABLE sms_recu (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    numero            TEXT NOT NULL CHECK (length(numero) BETWEEN 1 AND 24),
    texte             TEXT NOT NULL CHECK (length(texte) <= 1000),
    mot               TEXT CHECK (mot IN ('solde', 'abs', 'note', 'aide')),
    issue             TEXT NOT NULL CHECK (issue IN ('servie', 'aide', 'code_incorrect', 'code_incorrect_muet', 'bloquee', 'quota_depasse', 'numero_inconnu', 'inactive')),
    responsable_id    TEXT REFERENCES responsable(id),
    reponse_id        TEXT REFERENCES message_sms(id),
    recu_le           INTEGER NOT NULL,
    identifiant_poste TEXT NOT NULL
);
CREATE INDEX sms_recu_numero ON sms_recu (etablissement_id, numero, recu_le);
CREATE INDEX sms_recu_date ON sms_recu (etablissement_id, recu_le);

CREATE TRIGGER sms_recu_sans_suppression BEFORE DELETE ON sms_recu
BEGIN SELECT RAISE(ABORT, 'un SMS reçu ne se supprime pas'); END;
