-- Descente de la 0064 : les tables du portail des élèves.
DROP TRIGGER IF EXISTS avancement_lecon_sans_suppression;
DROP TABLE IF EXISTS avancement_lecon;
DROP TRIGGER IF EXISTS essai_code_eleve_sans_suppression;
DROP TABLE IF EXISTS essai_code_eleve;
DROP TRIGGER IF EXISTS code_eleve_sans_suppression;
DROP TABLE IF EXISTS code_eleve;
