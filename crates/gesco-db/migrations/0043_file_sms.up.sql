-- File des SMS (SPEC_PHASE_5 §3, blueprint §25.3) : chaque message, son fait, son destinataire,
-- son texte en GSM-7 et ses états. Persistante : elle survit à la fermeture du poste. Un message
-- parti reste au journal ; rien ne se supprime (R-14).

CREATE TABLE message_sms (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    nature            TEXT NOT NULL CHECK (nature IN ('bulletin', 'impaye', 'absence', 'convocation', 'sanction', 'essai')),
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
-- Un même fait ne met en file qu'un message par destinataire.
CREATE UNIQUE INDEX message_sms_fait ON message_sms (etablissement_id, cle_fait, numero);
CREATE INDEX message_sms_file ON message_sms (etablissement_id, etat, pas_avant);
CREATE INDEX message_sms_reference ON message_sms (reference) WHERE reference IS NOT NULL;

CREATE TRIGGER message_sms_sans_suppression BEFORE DELETE ON message_sms
BEGIN SELECT RAISE(ABORT, 'un message ne se supprime pas'); END;
