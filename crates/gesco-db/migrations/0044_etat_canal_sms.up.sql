-- État du canal des SMS (SPEC_PHASE_5 §4, C-5) : le dernier crédit lu, l'arrêt de la file et sa
-- raison, les échecs de crédit d'affilée, la clé qui ne répond pas. Une ligne par établissement.

CREATE TABLE etat_canal_sms (
    etablissement_id  TEXT PRIMARY KEY REFERENCES etablissement(id),
    credit            INTEGER,
    credit_texte      TEXT NOT NULL DEFAULT '',
    credit_lu_le      INTEGER,
    arretee           INTEGER NOT NULL DEFAULT 0 CHECK (arretee IN (0, 1)),
    raison            TEXT CHECK (raison IN ('credit_bas', 'echecs_credit')),
    echecs_credit     INTEGER NOT NULL DEFAULT 0 CHECK (echecs_credit BETWEEN 0 AND 255),
    cle_absente       INTEGER NOT NULL DEFAULT 0 CHECK (cle_absente IN (0, 1)),
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    CHECK (arretee = 1 OR raison IS NULL)
);

CREATE TRIGGER etat_canal_sms_sans_suppression BEFORE DELETE ON etat_canal_sms
BEGIN SELECT RAISE(ABORT, 'l''état du canal des SMS ne se supprime pas'); END;
