-- Numéros et consentement (SPEC_PHASE_5 §2, blueprint §25.6) : la joignabilité de chaque numéro,
-- alimentée par les accusés de remise, et le refus d'un type de message par un responsable. Une
-- ligne absente vaut « non vérifié » et « accepté ». Rien ne se supprime (R-14).

CREATE TABLE joignabilite_numero (
    etablissement_id   TEXT NOT NULL REFERENCES etablissement(id),
    numero             TEXT NOT NULL CHECK (numero GLOB '+[0-9]*' AND length(numero) BETWEEN 9 AND 16),
    etat               TEXT NOT NULL CHECK (etat IN ('non_verifie', 'joignable', 'injoignable', 'invalide')),
    echecs_consecutifs INTEGER NOT NULL DEFAULT 0 CHECK (echecs_consecutifs BETWEEN 0 AND 255),
    mis_a_jour_le      INTEGER NOT NULL,
    mis_a_jour_par     TEXT,
    identifiant_poste  TEXT NOT NULL,
    PRIMARY KEY (etablissement_id, numero)
);

CREATE TABLE consentement_message (
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    responsable_id    TEXT NOT NULL REFERENCES responsable(id),
    type_message      TEXT NOT NULL CHECK (type_message IN ('bulletin', 'impaye', 'absence', 'convocation')),
    accepte           INTEGER NOT NULL CHECK (accepte IN (0, 1)),
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL,
    PRIMARY KEY (responsable_id, type_message)
);

CREATE TRIGGER joignabilite_numero_sans_suppression BEFORE DELETE ON joignabilite_numero
BEGIN SELECT RAISE(ABORT, 'la joignabilité d''un numéro ne se supprime pas'); END;

CREATE TRIGGER consentement_message_sans_suppression BEFORE DELETE ON consentement_message
BEGIN SELECT RAISE(ABORT, 'un consentement ne se supprime pas'); END;
