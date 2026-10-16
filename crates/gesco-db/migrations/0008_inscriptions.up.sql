-- Inscriptions (SPEC_PHASE_2 §5, blueprint §8.3, §24.7) : la table pivot. Jamais de DELETE
-- (R-14) : un brouillon s'abandonne, une inscription validée s'abandonne ou se transfère.

CREATE TABLE inscription (
    id                         TEXT PRIMARY KEY CHECK (length(id) = 26),
    etablissement_id           TEXT NOT NULL REFERENCES etablissement(id),
    annee_id                   TEXT NOT NULL REFERENCES annee_scolaire(id),
    eleve_id                   TEXT NOT NULL REFERENCES eleve(id),
    classe_id                  TEXT NOT NULL REFERENCES classe(id),
    date_inscription           TEXT NOT NULL CHECK (length(date_inscription) = 10),
    type                       TEXT NOT NULL CHECK (type IN ('nouvelle', 'reinscription', 'transfert_entrant')),
    statut                     TEXT NOT NULL CHECK (statut IN ('brouillon', 'validee', 'transferee', 'abandonnee', 'radiee', 'achevee')),
    redoublant                 INTEGER NOT NULL DEFAULT 0 CHECK (redoublant IN (0, 1)),
    decision_fin_annee         TEXT NOT NULL DEFAULT 'non_statue'
        CHECK (decision_fin_annee IN ('non_statue', 'admis', 'redouble', 'exclu', 'rattrapage')),
    classe_precedente_id       TEXT REFERENCES classe(id),
    etablissement_origine      TEXT NOT NULL DEFAULT '',
    motif_abandon              TEXT NOT NULL DEFAULT '',
    date_depart                TEXT CHECK (date_depart IS NULL OR length(date_depart) = 10),
    etablissement_destination  TEXT NOT NULL DEFAULT '',
    motif_depart               TEXT NOT NULL DEFAULT '',
    cree_le                    INTEGER NOT NULL,
    cree_par                   TEXT,
    mis_a_jour_le              INTEGER NOT NULL,
    mis_a_jour_par             TEXT,
    identifiant_poste          TEXT NOT NULL
);
-- Un élève n'a qu'une inscription vivante (brouillon ou validée) par année (§0.4, assertion 8).
CREATE UNIQUE INDEX inscription_vivante_unique
    ON inscription (etablissement_id, annee_id, eleve_id) WHERE statut IN ('brouillon', 'validee');
CREATE INDEX inscription_classe ON inscription (classe_id, statut);
CREATE INDEX inscription_eleve ON inscription (eleve_id);
-- R-14 : la suppression physique est refusée mécaniquement ; aucune logique ici (R-15).
CREATE TRIGGER inscription_sans_suppression BEFORE DELETE ON inscription
BEGIN SELECT RAISE(ABORT, 'une inscription ne se supprime pas'); END;
