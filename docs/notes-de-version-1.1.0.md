# GESCO 1.1.0 — notes de version

Version du 17 octobre 2026. Elle complète les modules de la 1.0 avec ce qui y manquait encore.
Les SMS aux familles, les rapports et statistiques et l'e-learning viendront dans la version 2.

## Ce qui est nouveau

**Autres recettes.** Les dons, les locations de salle, les subventions et les autres entrées
d'argent qui ne viennent pas des familles. On les saisit dans « Autres recettes » (groupe
Finance). Elles entrent dans le compte choisi : en espèces dans la caisse ouverte, ou par
virement ou mobile money. On les retrouve dans le livre du compte, le journal du mois et le
budget. Une recette s'annule avec un motif.

**Photo à la webcam.** La photo d'un élève se prend avec la caméra du poste, depuis sa fiche. La
première fois, Windows demande l'autorisation d'utiliser la caméra.

**Discipline.** L'écran « Discipline » (groupe Scolarité) tient le registre de l'année et le
dossier de chaque élève : avertissement, blâme, retenue, convocation des parents. Une sanction
s'annule avec un motif. L'avertissement et le blâme du conseil de classe sont imprimés sur le
bulletin de la période.

**Heures des vacataires d'après l'emploi du temps.** Quand un vacataire a des séances dans
l'emploi du temps, l'état des heures du mois propose ses heures d'après ces séances. Les jours
de canicule sont pris en compte. Les heures restent à valider comme avant.

**Archive signée du journal d'audit.** À la clôture de l'année, le journal de l'année est écrit
dans un fichier signé (`journal-2026-2027.gja`). Ce fichier est rangé avec les sauvegardes et
copié sur leurs destinations. L'écran « Journal d'audit » vérifie un fichier d'archive : si un seul octet
a changé, il le signale.

**Absences par séance (au choix de l'école).** Pour l'activer, cochez dans les paramètres le
réglage « Absences : relever à chaque séance de l'emploi du temps ». Le relevé d'une classe montre alors les séances du jour d'après
l'emploi du temps. Un élève absent à une séance du matin compte la demi-journée, comme avant :
les totaux, les bulletins et les avertissements ne changent pas.

**Import des relevés.** Dans « Comptes et soldes », le relevé d'Orange Money, de Wave, de Moov ou
de la banque s'importe, en Excel ou en CSV. Indiquez ses colonnes une fois pour chaque compte ;
GESCO les retient. GESCO coche les lignes du livre qu'il retrouve, par la référence ou par le
montant à trois jours près. Il montre aussi les lignes du relevé que le livre n'a pas, par
exemple les frais bancaires.

**Import du personnel.** Dans le registre du personnel, le bouton « Importer une liste… »
fournit d'abord le modèle Excel à remplir. GESCO vérifie ensuite chaque ligne avant de créer les
fiches. Il laisse de côté les lignes en erreur et les personnes déjà au registre. Une sauvegarde
est faite avant l'import.

## Mettre à jour une école en 1.0.0

Installez la 1.1.0 par-dessus la version en place : les données sont gardées. Au démarrage,
GESCO fait une sauvegarde puis met la base à niveau. Les rôles livrés ne changent pas.
