-- Relances d'impayés (SPEC_PHASE_5 §5) : le jour où la file les a faites, une fois par jour.

ALTER TABLE etat_canal_sms ADD COLUMN relances_le TEXT CHECK (relances_le IS NULL OR length(relances_le) = 10);
