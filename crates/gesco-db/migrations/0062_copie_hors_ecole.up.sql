-- Copie hors de l'école : chaque ligne de l'historique dit la sorte de sa destination (dossier,
-- support amovible, dossier synchronisé) ; une copie déposée dans un dossier synchronisé porte en
-- plus son état d'envoi et l'instant de sa dernière vérification. Les lignes existantes sont des
-- dossiers. Les noms ne parlent pas de dossier : un serveur de l'éditeur reprendra les mêmes états.
ALTER TABLE sauvegarde ADD COLUMN sorte TEXT NOT NULL DEFAULT 'dossier'
    CHECK (sorte IN ('dossier', 'amovible', 'synchronise'));
ALTER TABLE sauvegarde ADD COLUMN etat_envoi TEXT
    CHECK (etat_envoi IS NULL OR etat_envoi IN ('deposee', 'en_attente', 'envoyee', 'echec_envoi', 'non_deposee', 'absente'));
ALTER TABLE sauvegarde ADD COLUMN etat_verifie_le INTEGER;
CREATE INDEX sauvegarde_par_sorte ON sauvegarde (etablissement_id, sorte, horodatage);
