DROP TRIGGER difficulte_eleve_sans_suppression;
DROP TABLE difficulte_eleve;
ALTER TABLE membre_personnel DROP COLUMN diplome_eleve;
ALTER TABLE membre_personnel DROP COLUMN specialite;
ALTER TABLE membre_personnel DROP COLUMN statut_rapport;
ALTER TABLE classe DROP COLUMN pedagogie;
ALTER TABLE classe DROP COLUMN fonctionnement;
ALTER TABLE inscription DROP COLUMN prise_en_charge_etat;
ALTER TABLE eleve DROP COLUMN annee_recrutement;
