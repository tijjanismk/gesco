-- Appréciation posée d'office d'après la moyenne (revue du 10 novembre 2026) : marquée, elle se
-- refait à chaque calcul des bulletins et suit donc une note corrigée ; celle qu'un enseignant a
-- écrite ne change jamais. Les appréciations déjà en base sont tenues pour écrites à la main.

ALTER TABLE appreciation ADD COLUMN automatique INTEGER NOT NULL DEFAULT 0 CHECK (automatique IN (0, 1));
