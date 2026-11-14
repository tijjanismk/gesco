-- Compléments pour les rapports (SPEC_PHASE_9 §8.2, §8.3). Tout est facultatif : une colonne vide
-- est une case vide dans les rapports, jamais un refus. Le statut du personnel vide se déduit de
-- la fiche ; le fonctionnement et la pédagogie vides d'une classe prennent la valeur de la fiche
-- de l'établissement. Les difficultés d'un élève vivent à part, lues avec un droit propre ; rien
-- ne se supprime : une difficulté retirée perd son degré.

ALTER TABLE eleve ADD COLUMN annee_recrutement INTEGER
    CHECK (annee_recrutement IS NULL OR annee_recrutement BETWEEN 1950 AND 2200);

ALTER TABLE inscription ADD COLUMN prise_en_charge_etat INTEGER NOT NULL DEFAULT 0
    CHECK (prise_en_charge_etat IN (0, 1));

ALTER TABLE classe ADD COLUMN fonctionnement TEXT
    CHECK (fonctionnement IS NULL OR fonctionnement IN ('simple', 'double_division',
        'double_vacation'));
ALTER TABLE classe ADD COLUMN pedagogie TEXT
    CHECK (pedagogie IS NULL OR pedagogie IN ('classique', 'pedagogie_convergente', 'curriculum'));

ALTER TABLE membre_personnel ADD COLUMN statut_rapport TEXT
    CHECK (statut_rapport IS NULL OR statut_rapport IN ('fonctionnaire_etat',
        'fonctionnaire_collectivites', 'contractuel_etat', 'contractuel_collectivites',
        'contractuel_ecole', 'vacataire', 'eleve_maitre', 'autre'));
ALTER TABLE membre_personnel ADD COLUMN specialite TEXT
    CHECK (specialite IS NULL OR specialite IN ('lhg', 'mpc', 'snpc', 'ldm', 'eps', 'autre'));
ALTER TABLE membre_personnel ADD COLUMN diplome_eleve TEXT
    CHECK (diplome_eleve IS NULL OR diplome_eleve IN ('doctorat', 'master', 'dea', 'ensup',
        'ingenieur', 'maitrise', 'licence', 'deug_bts', 'bt', 'baccalaureat', 'cap', 'ifm',
        'autre'));

CREATE TABLE difficulte_eleve (
    id                TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id  TEXT NOT NULL REFERENCES etablissement(id),
    eleve_id          TEXT NOT NULL REFERENCES eleve(id),
    type              TEXT NOT NULL CHECK (type IN ('vue', 'ouie', 'motricite_globale',
                          'motricite_fine', 'intellect', 'communication', 'comportement',
                          'multiple', 'autre')),
    degre             TEXT CHECK (degre IS NULL OR degre IN ('quelques', 'beaucoup', 'impossible')),
    note              TEXT NOT NULL DEFAULT '',
    cree_le           INTEGER NOT NULL,
    cree_par          TEXT,
    mis_a_jour_le     INTEGER NOT NULL,
    mis_a_jour_par    TEXT,
    identifiant_poste TEXT NOT NULL
);
CREATE UNIQUE INDEX difficulte_eleve_unique ON difficulte_eleve (eleve_id, type);

CREATE TRIGGER difficulte_eleve_sans_suppression BEFORE DELETE ON difficulte_eleve
BEGIN SELECT RAISE(ABORT, 'une difficulté ne se supprime pas'); END;
