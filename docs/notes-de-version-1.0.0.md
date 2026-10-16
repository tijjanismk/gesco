# GESCO 1.0.0 — notes de version

Version du 16 octobre 2026. Première version stable : elle va dans les écoles. Elle ne reçoit
ensuite que des corrections (1.0.1, 1.0.2…), pendant que le développement continue sur une
ligne à part.

## Ce que fait GESCO 1.0.0

**Socle.** Installation sur un poste Windows, sans internet ; base chiffrée ; comptes, rôles et
droits ; journal d'audit ; sauvegardes automatiques et sur clé USB, phrase de secours ; licence
libre numérotée, attachée au poste (sans licence, tout marche, avec un bandeau).

**Scolarité.** Cycles du jardin au lycée et enseignement technique ; élèves, responsables,
inscriptions et réinscriptions, transferts, abandons ; classes ; import Excel des élèves ;
fiches et certificats imprimés.

**Notes, absences, bulletins.** Matières et programmes, évaluations, saisie et import des notes,
moyennes et rangs, bulletins validés et publiés ; relevé des absences et justifications ;
fin d'année : moyenne de passage, décisions validées par l'école, clôture, année suivante.

**Finances des familles.** Grilles de frais, factures et échéanciers, remises, encaissement au
guichet avec reçu, caisse et sessions, annulations et régularisations, impayés par ancienneté,
arriérés reportés sur l'année suivante.

**Personnel et paie.** Registre du personnel, engagements, vacations et heures, absences et
congés ; éléments de paie, calcul du mois, bulletins de paie, état de paie à signer, état des
retenues ; salaires payés en une ou plusieurs fois, avances remboursées par retenue.

**Dépenses et trésorerie (nouveau).**
- Dépenses par catégorie, payées d'un geste sous le seuil, après l'accord de la direction
  au-dessus ; justificatif joint ; annulation motivée.
- Fournisseurs et leurs factures, payées en plusieurs fois, et ce que l'école leur doit.
- Comptes de l'école (caisse, banque, mobile money) avec leur solde, transferts entre eux,
  livre de chaque compte coché contre le relevé.
- Salaires et avances devenus des dépenses « Salaires ».
- Journal de trésorerie du mois et budget de l'année, exportés en Excel.

**Emploi du temps (nouveau).** Journée de chaque cycle, horaire de canicule du 1er cycle,
séances placées à la main ; un enseignant à deux endroits ou une salle prise deux fois est
signalé ; impression par classe et par enseignant.

**Cantine (nouveau, à activer).** Abonnés du jour d'après les factures, repas pointés si l'école
les compte, liste du jour imprimée, bilan du mois.

**Tableau de bord.** Effectifs, encaissements et impayés, scolarité, personnel, paie, trésorerie,
selon les droits de chacun.

## Ce qui viendra après la 1.0

Les SMS aux familles, les rapports et statistiques, puis les modules suivants : ils arriveront
dans les versions 1.1 et au-delà. La comptabilité générale (SYSCOHADA) attendra une école qui la
demande.

## Mettre à jour une école en 0.11.0

Installer la 1.0.0 par-dessus. Les données sont gardées ; au démarrage, GESCO fait une
sauvegarde, met la base à niveau, donne aux rôles livrés leurs nouveaux droits (trésorerie,
emploi du temps, cantine) et range les salaires déjà payés dans les dépenses.
