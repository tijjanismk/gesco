-- Version de la composition par défaut déjà appliquée à chaque rôle livré (SPEC_PHASE_1 §5.6).
-- Les rôles existants partent de 0 : à l'ouverture, le noyau leur ajoute les permissions de leur
-- composition qui leur manquent, sans jamais en retirer. Sans objet pour un rôle personnalisé.
ALTER TABLE role ADD COLUMN composition_livree INTEGER NOT NULL DEFAULT 0;
