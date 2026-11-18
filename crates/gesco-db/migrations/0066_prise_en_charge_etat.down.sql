-- Descente de la 0066 : les tables de la prise en charge par l'État.
DROP TRIGGER IF EXISTS versement_etat_part_sans_suppression;
DROP TRIGGER IF EXISTS versement_etat_sans_suppression;
DROP TRIGGER IF EXISTS montant_etat_sans_suppression;
DROP TRIGGER IF EXISTS prise_en_charge_etat_sans_suppression;
DROP TABLE IF EXISTS versement_etat_part;
DROP TABLE IF EXISTS versement_etat;
DROP TABLE IF EXISTS montant_etat;
DROP TABLE IF EXISTS prise_en_charge_etat;
