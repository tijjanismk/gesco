-- Moyennes reprises du relevé officiel d'une autre école (SPEC_PHASE_4 §5, Q-28) : la moyenne
-- générale d'une période qu'un élève arrivé en cours d'année a suivie ailleurs. Jamais les notes
-- de détail. Une moyenne reprise ne se supprime pas : effacée, elle est marquée `retiree`.

CREATE TABLE moyenne_reprise (
    inscription_id        TEXT NOT NULL REFERENCES inscription(id),
    periode_id            TEXT NOT NULL REFERENCES periode_academique(id),
    etablissement_id      TEXT NOT NULL REFERENCES etablissement(id),
    moyenne               INTEGER NOT NULL CHECK (moyenne BETWEEN 0 AND 10000),
    etablissement_origine TEXT NOT NULL,
    piece                 TEXT NOT NULL DEFAULT '',
    retiree               INTEGER NOT NULL DEFAULT 0 CHECK (retiree IN (0, 1)),
    saisie_le             INTEGER NOT NULL,
    saisie_par            TEXT,
    identifiant_poste     TEXT NOT NULL,
    PRIMARY KEY (inscription_id, periode_id)
);

CREATE TRIGGER moyenne_reprise_sans_suppression BEFORE DELETE ON moyenne_reprise
BEGIN SELECT RAISE(ABORT, 'une moyenne reprise ne se supprime pas'); END;
