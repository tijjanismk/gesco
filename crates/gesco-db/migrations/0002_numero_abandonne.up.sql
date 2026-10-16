-- Numéros attribués puis abandonnés (R-46) : un numéro de reçu ou de facture tiré de la séquence
-- et non utilisé laisse une trace annulée, jamais un trou. Le document « annulé » correspondant
-- est produit par le moteur documentaire à partir de cette ligne.
CREATE TABLE numero_abandonne (
    numero TEXT NOT NULL,
    etablissement_id TEXT NOT NULL REFERENCES etablissement (id),
    type_document TEXT NOT NULL,
    exercice TEXT NOT NULL,
    motif TEXT NOT NULL,
    abandonne_le INTEGER NOT NULL,
    abandonne_par TEXT,
    identifiant_poste TEXT NOT NULL,
    PRIMARY KEY (etablissement_id, type_document, exercice, numero)
);
