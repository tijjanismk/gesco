-- Descente de la 0065 : la reprise avant GESCO et le lien d'une dépense à une inscription.
DROP INDEX IF EXISTS depense_inscription;
ALTER TABLE depense DROP COLUMN inscription_id;
DROP TRIGGER IF EXISTS reprise_imputation_sans_suppression;
DROP TABLE IF EXISTS reprise_imputation;
DROP TRIGGER IF EXISTS reprise_avant_gesco_sans_suppression;
DROP TABLE IF EXISTS reprise_avant_gesco;
