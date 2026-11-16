DROP INDEX IF EXISTS sauvegarde_par_sorte;
ALTER TABLE sauvegarde DROP COLUMN etat_verifie_le;
ALTER TABLE sauvegarde DROP COLUMN etat_envoi;
ALTER TABLE sauvegarde DROP COLUMN sorte;
