//! Tests d'intégration de `gesco-services`, réunis en un seul exécutable : un fichier par module,
//! liés à un seul exemplaire des dépendances au lieu d'un exécutable de 140 Mo par fichier.
//! Un seul fichier : `cargo test -p gesco-services --test services <module>`.

mod commun;

mod absences;
mod annuelle;
mod bulletins;
mod caisse;
mod charge;
mod conges;
mod consultation;
mod convocations;
mod departs;
mod depenses;
mod droits_personnel;
mod effets_documents;
mod emploi_du_temps;
mod engagements;
mod enseignants;
mod evaluations;
mod export;
mod facturation;
mod file_sms;
mod fin_annee;
mod fournisseurs;
mod frais;
mod frontiere;
mod guichet;
mod heures;
mod import_eleves;
mod import_programme;
mod import_referentiel;
mod journal_technique;
mod livres;
mod matrice_permissions;
mod messages;
mod messages_auto;
mod mise_a_niveau;
mod modem;
mod paie;
mod paie_mois;
mod pedagogie;
mod personnel;
mod recettes;
mod scolarite;
mod socle_services;
mod technique;
mod tresorerie;
mod types_typescript;
