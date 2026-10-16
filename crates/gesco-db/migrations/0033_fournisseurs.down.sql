DROP TRIGGER facture_fournisseur_sans_suppression;
DROP TRIGGER fournisseur_sans_suppression;
DROP INDEX depense_facture;
ALTER TABLE depense DROP COLUMN facture_id;
DROP TABLE facture_fournisseur;
DROP TABLE fournisseur;
