# GESCO 1.2.0 — notes de version

Version du 18 octobre 2026. Elle ouvre la version 2 avec les SMS aux familles, le guichet et la
remise des documents ; elle rend la barre latérale modulaire et ajoute l'import du programme.

## Ce qui est nouveau

**Modules de l'école.** En tête de **Paramètres → Établissement**, une case par module : messages
aux familles par SMS, cantine, personnel et paie, dépenses et trésorerie, emploi du temps,
discipline. Un module éteint quitte le menu et le tableau de bord pour tout le monde ; ses
données restent. Les SMS et la cantine sont éteints à l'installation, les autres allumés.

**Messages aux familles par SMS (à allumer).** Par une clé modem GSM USB et la puce prépayée de
l'école, sans internet. Trois messages partent d'eux-mêmes, chacun réglable : le bulletin à
retirer, la relance d'impayé, l'absence du jour. Les convocations et les avis de sanction sont
préparés puis relus et envoyés par une personne. Les SMS partent aux heures réglées, en français
sans accents. Chaque numéro sait s'il est joignable ; un parent refuse un type de message sans
perdre les autres. Le crédit de la puce est surveillé : sous le seuil, les SMS s'arrêtent et la
direction est prévenue. Écran **Communication → Messages**.

**Guichet.** **Scolarité → Guichet** : une recherche, et toute la famille sur un écran — ce que
doit chaque enfant, les bulletins à remettre, les convocations, les absences récentes, les SMS
envoyés. On encaisse, remet, justifie sans changer d'écran.

**Remise des bulletins.** Un bulletin publié est à remettre ; il se remet à un responsable, à
l'élève ou à une personne nommée, et se signe. Les bulletins jamais retirés sont listés avec ce
que doit la famille. Écran **Scolarité → Remise des bulletins**.

**Convocations.** Convoquer un responsable, imprimer le billet avec son coupon, noter la
réception, un report, puis la suite. Écran **Scolarité → Convocations**.

**Codes d'accès et consultation par SMS.** Au guichet, un responsable reçoit un code d'accès,
affiché une seule fois avec son billet. Si l'école ouvre la consultation, le parent écrit SOLDE,
ABS ou NOTE suivi de son code et reçoit la réponse par SMS. Un numéro inconnu, un code faux
répété ou un quota dépassé ne coûtent pas de crédit.

**Import du programme.** En bas de **Paramètres → Établissement**, section repliée « Importer le
programme des matières (Excel) » : un modèle qui contient le programme actuel, à corriger dans
Excel puis à réimporter. Seuls les niveaux du fichier sont récrits ; une matière inconnue est
créée.

**Rôles.** Le Secrétaire remet les bulletins et les codes d'accès et voit la file des SMS ; le
Caissier remet les bulletins ; le Surveillant général tient les convocations et relit les
messages préparés ; la Consultation seule voit la file des SMS sans rien envoyer.

## À savoir avant de compter sur les SMS

Les SMS ont été essayés avec une clé simulée. Avec la clé et la puce de l'école, faites d'abord
un essai : un SMS d'essai depuis Communication → Messages, la lecture du crédit, puis un SOLDE
envoyé depuis un téléphone de parent. Les modèles de clés essayés seront notés au fur et à
mesure.

## Mettre à jour une école en 1.0 ou 1.1

Installez la 1.2.0 par-dessus la version en place : les données sont gardées. Au démarrage,
GESCO fait une sauvegarde puis met la base à niveau ; les rôles livrés reçoivent les nouveaux
droits. Le menu gagne le guichet, la remise des bulletins et les convocations ; les messages
aux familles n'y paraissent que si l'école les allume. L'entrée « Cantine » n'y paraît plus que
si la cantine est allumée (dans Finance → Trésorerie ou dans les modules de l'école).
