ALTER TABLE membre_personnel DROP COLUMN enfants_a_charge;
ALTER TABLE membre_personnel DROP COLUMN situation_famille;
ALTER TABLE engagement DROP COLUMN paye_douze_mois;
DROP TRIGGER attribution_element_sans_suppression;
DROP TRIGGER version_element_sans_suppression;
DROP TRIGGER element_paie_sans_suppression;
DROP TABLE attribution_element;
DROP TABLE version_element;
DROP TABLE element_paie;
