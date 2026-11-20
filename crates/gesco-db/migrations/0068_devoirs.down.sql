-- Descente de la 0068 : les devoirs, leurs rendus et leurs photos disparaissent ; les évaluations
-- reportées restent, sans leur lien au devoir.
DROP INDEX IF EXISTS evaluation_devoir;
ALTER TABLE evaluation DROP COLUMN devoir_id;
ALTER TABLE evaluation DROP COLUMN origine;
DROP TRIGGER IF EXISTS photo_rendu_sans_suppression;
DROP TABLE IF EXISTS photo_rendu;
DROP TRIGGER IF EXISTS fichier_rendu_sans_suppression;
DROP TABLE IF EXISTS fichier_rendu;
DROP TRIGGER IF EXISTS rendu_devoir_sans_suppression;
DROP TABLE IF EXISTS rendu_devoir;
DROP TRIGGER IF EXISTS devoir_sans_suppression;
DROP TABLE IF EXISTS devoir;
