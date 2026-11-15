# SPEC_LOTS_C_D.md — Service en ligne et portail des parents

**Projet** : ERP de gestion scolaire — Tauri 2 + Rust + React/TypeScript
**Phase** : 11, lots C (service en ligne : sauvegarde sur le serveur de l'éditeur, envoi des données
destinées aux parents, serveur) et D (portail des parents) ; version 2.0.0
**Statut** : v1.0 — proposée le 15 novembre 2026, à valider par le porteur du projet ; aucune ligne
de code écrite
**Documents parents** : `docs/plan-phase-11.md` (§2.4, §2.5, §4 décisions 5 à 9, T-85, T-86, §7),
`PROJECT_BLUEPRINT.md` (ADR-001, 002, 003, 009, 017 ; §3.9, §3.12, §15.2, §17, §24.3, §26),
`docs/orchestration.md` (14 et 15 novembre 2026), `SPEC_PHASE_5.md` §9 (code d'accès),
`SPEC_PHASE_11_A.md` (plusieurs postes), `SPEC_SAUVEGARDE_HORS_ECOLE.md` (copie hors de l'école)
**ADR de cette spécification** (statut proposé) : `docs/adr/2026-11-15-hebergement-service-en-ligne.md`,
`docs/adr/2026-11-15-donnees-qui-montent.md`, `docs/adr/2026-11-15-identite-des-parents.md`,
`docs/adr/2026-11-15-portail-application-web.md`
**Date** : 15 novembre 2026

---

## 0. Objet et critère de fin

### 0.1 Ce que produisent ces deux lots

Aujourd'hui GESCO ne parle à aucun serveur. Toutes les données vivent sur le poste principal de
l'école ; les parents reçoivent des SMS, et peuvent demander leur solde par SMS avec leur code
d'accès.

À la fin des lots C et D :
- une école abonnée **relie son GESCO au service en ligne de l'éditeur**, avec sa licence ;
- elle peut y **envoyer une copie de sa sauvegarde**, déjà chiffrée, que GESCO envoie lui-même,
  avec reprise après coupure ; l'éditeur ne peut pas la lire ; sur un ordinateur neuf, les douze
  mots suffisent pour la récupérer et la restaurer (C1) ;
- si l'école allume le **portail des parents**, GESCO envoie chaque jour, pour les seuls élèves
  dont un responsable a un code d'accès, ce que le parent doit voir, déjà calculé (C2) ;
- un **serveur** reçoit, range et montre ces données ; une seule personne l'exploite (C3) ;
- un parent **ouvre le portail sur son téléphone**, le relie une fois avec son numéro et son code
  d'accès, et voit pour chacun de ses enfants : le reste à payer et les échéances, les reçus, les
  absences, les bulletins publiés, même sans réseau (D1 à D3) ;
- si l'école le veut, le parent **écrit à l'école**, qui répond (D4, éteint à la livraison).

Rien de tout cela n'est nécessaire au travail de l'école. Une école sans abonnement, sans Internet
ou qui n'allume rien ne voit aucune différence.

### 0.2 Décisions déjà prises qui s'appliquent

- **Tout « en ligne » entre dans la 2.0** (porteur, 14 novembre 2026) : C1, C2, C3 et D1 à D4.
  La sauvegarde par dossier synchronisé reste la voie sans abonnement
  (`SPEC_SAUVEGARDE_HORS_ECOLE.md`). Ordre : lot B, phase 9, sauvegarde hors de l'école,
  phase 12, puis les lots C et D.
- **Ce qu'un parent voit** (T-85, 10 novembre 2026) : ce que donne le SMS (reste à payer et
  prochaines échéances, absences, moyenne du dernier bulletin publié), plus les bulletins publiés
  et les reçus, **en lecture seule**. Les sanctions et les appréciations se disent en face.
  Écrire à l'école vient seulement si une école le demande : D4 est construit, éteint à la
  livraison (§7.4).
- **Le poste de l'école reste la source de vérité** (ADR-001). Le serveur est une copie montante,
  jamais l'inverse (§15.2, maître unique).
- **La licence ne bloque jamais** l'édition Essentielle ni l'accès aux données (§26.5). Seuls les
  modules connectés s'éteignent, après 90 jours de grâce (`licence.rs`, `GRACE_MILLIS`).
- **La sauvegarde et la restauration ne sont jamais payantes** (§26.2) : la voie par dossier
  synchronisé est gratuite ; une archive déjà envoyée au serveur reste récupérable après la fin
  de l'abonnement (§4.5).
- **« La flexibilité sans incohérence »** (porteur, 14 novembre 2026) : rien ne bloque, rien ne
  fait remarquer ce qui manque ; les chiffres et les documents sont toujours justes. Ici : un
  chiffre en ligne est celui de l'école au moment de l'envoi, et il est daté.
- **Interface sans sources** : aucun texte affiché, au bureau comme dans le portail, ne cite un
  numéro de question, de spécification, d'ADR ou une date de décision.
- **Préalable à la mise en service réelle** (§24.3) : avis écrit d'un juriste sur les données de
  mineurs en ligne. Tout se construit et s'essaie avant cet avis, contre un serveur d'essai ;
  aucun parent réel n'entre avant.
- **Lot A** : seul le poste principal envoie et récupère ; les commandes du service en ligne sont
  réservées au poste principal, comme la sauvegarde.
- R-21 : toute commande passe par `Application::executer`. R-14 : rien ne se supprime dans la
  base de l'école ; ce qui s'arrête se désactive ou se révoque.

### 0.3 Critère de fin, vérifiable

Tous les essais tournent contre un serveur d'essai lancé sur le poste de développement, sauf les
points 12 à 14.

1. **Archive envoyée et reprise** : une sauvegarde de l'école d'essai part vers le serveur ;
   coupée au milieu, elle reprend là où elle s'est arrêtée, sans renvoyer ce qui était arrivé.
   Son état ne devient « envoyée » qu'après que le serveur a confirmé l'empreinte complète.
2. **Récupération** : sur une seconde instance neuve, avec le numéro de licence et les douze mots
   seuls, l'archive se télécharge et se restaure. Avec des mots faux, rien ne se télécharge. Le
   serveur ne détient rien qui permette de lire l'archive.
3. **Mêmes chiffres** : pour chaque élève de l'école d'essai dont un responsable a un code
   actif, la vue envoyée donne le même reste à payer, les mêmes échéances, les mêmes reçus, les
   mêmes absences et les mêmes moyennes que les écrans de l'école au moment de l'envoi. Un essai
   le vérifie élève par élève.
4. **Minimisation** : aucun élève sans responsable ayant un code actif n'est sur le serveur ; aucune
   vue ne contient de date de naissance, d'adresse, de photo, de sanction, d'appréciation, de note
   non publiée (essai sur le contenu de toutes les vues).
5. **Envoi sûr** : un envoi coupé au milieu reprend ; rien en double, rien de perdu ; un second envoi
   sans changement n'envoie aucune vue ; le volume affiché est celui qui est parti.
6. **Ce qui disparaît** : un élève parti, un code révoqué, une année close disparaissent du serveur
   à l'envoi suivant ; les téléphones reliés avec un code révoqué sont coupés.
7. **Entrer** : un parent relie son téléphone avec son numéro et son code et ne voit que ses
   enfants. Essai d'intrusion simple, écrit et rejoué : code faux (le sixième dans la journée est
   refusé même juste), numéro d'un autre, jeton d'un autre parent, adresse de la vue d'un autre
   élève, jeton expiré, requête sans HTTPS : tout est refusé, avec la même réponse pour un numéro
   inconnu et un code faux.
8. **Téléphone** : le portail s'utilise sur un écran de 320 pixels de large, contrastes du projet
   respectés ; premier chargement de 150 Ko compressés au plus ; sans réseau, il montre le dernier
   état avec sa date ; « Se déconnecter » efface tout ce que le téléphone gardait.
9. **Serveur arrêté** : l'école inscrit, encaisse, imprime, sauvegarde sans aucun message
   bloquant ; les envois reprennent seuls au retour du serveur.
10. **Abonnement fini** : après l'échéance et la grâce, les envois cessent, le portail de cette
    école ne montre plus de données, rien d'autre ne change dans GESCO ; l'archive déjà envoyée se
    récupère encore (§4.5).
11. **Journaux** : aucun numéro de téléphone, code, jeton, nom d'élève ou montant n'apparaît dans
    les journaux du serveur (essai qui parcourt les journaux après tout le scénario).
12. **Panne jouée** : sur une machine neuve, le serveur est réinstallé par le script du dépôt et
    restauré depuis la copie de nuit en moins de quatre heures ; la procédure est jouée une fois,
    chronométrée, et écrite.
13. **Une école qui n'allume rien** ne voit aucun changement dans ses écrans, et GESCO n'ouvre
    aucune connexion vers Internet pour elle (essai : aucun appel réseau sans service relié).
14. **Mise en service réelle** seulement quand le porteur a fourni l'avis écrit du juriste,
    l'hébergeur et le nom de domaine (§15), et qu'une école pilote l'a essayée avec quelques
    parents.

---

## 1. Périmètre

**Inclus** : compte de l'école chez l'éditeur, relié par la licence Connectée ; copie de la
sauvegarde sur le serveur, reprise, confirmation, récupération sur un ordinateur neuf ; calcul et
envoi des vues destinées aux parents ; purge du journal `outbox` ; serveur `gesco-server`, ses bases,
sa copie de nuit, sa surveillance, sa console d'administration, ses scripts d'installation et de
restauration ; portail des parents (entrer, voir, hors ligne, écrire à l'école) ; droits ; écrans ;
manuel ; guide d'exploitation du serveur ; essais.

**Exclu** :
- Travailler à distance sur GESCO (le directeur chez lui, deux sites) : le serveur ne reçoit que
  des vues, il ne fait pas tourner GESCO.
- Le téléphone des maîtres, le portail des élèves et l'e-learning en ligne (phase 12, mode en
  ligne) : ils pourront partir du même serveur et du même socle de portail, plus tard.
- Les notifications poussées sur le téléphone (ADR « portail ») ; WhatsApp Business ; le paiement en
  ligne ; la justification d'absence en ligne.
- Les PDF des bulletins et des reçus dans le portail, sauf réponse contraire à T-153.
- Le transfert de données entre écoles d'une même organisation (Q-36).
- Toute écriture du parent autre qu'un message (D4).

## 2. Ce que l'application possède déjà

Relevé dans le code le 15 novembre 2026, sans compilation.

| Pièce | Où | Ce qu'on en fait ici |
|---|---|---|
| Licence signée Ed25519, `Edition { Essentielle, Connectee }`, `echeance`, grâce de 90 jours, `modules_connectes_actifs` | `gesco-core/src/core/licence.rs` | Lue telle quelle : elle allume ou éteint le service. Le format ne change pas (`deny_unknown_fields` refuserait un champ nouveau) |
| Code d'accès du responsable : six caractères, alphabet de 32 signes, empreinte Argon2id, un seul actif, révocation, billet imprimé | migration `0047`, `gesco-core/src/notification/consultation.rs`, `consultation.rs`, `documents.rs` | Identité du parent dans le portail (ADR « identité ») ; le billet ajoute l'adresse du portail |
| Consultation par SMS : solde, absences, moyenne, rang au choix de l'école ; cinq codes faux par jour bloquent un numéro | `notification/consultation.rs` | Même règle de blocage ; même réglage du rang |
| Bulletin figé à la publication : `contenu`, `version`, statut `publie` puis `remplace` | migration `0019` | La vue reprend le contenu publié, dernière version seulement |
| Journal `outbox` écrit dans chaque transaction, jamais lu, jamais purgé | migration `0001`, `gesco-db/src/depots/outbox.rs` | Signal « quelque chose a changé », puis purge (§5.5) |
| Archive `.gsv` chiffrée par la clé de l'école, enveloppe de secours (douze mots) incluse | `gesco-services/src/sauvegarde.rs`, ADR du 4 octobre 2026 | C'est ce qui part au serveur (C1) |
| Sortes de destination et états d'envoi, prévus pour une sorte `serveur` | `SPEC_SAUVEGARDE_HORS_ECOLE.md` §6 et §10 | C1 s'y raccorde (§4) |
| Client HTTPS (hyper, rustls) et serveur (axum) du réseau local | `gesco-reseau` | Le client sert d'envoi vers le serveur ; axum sert le serveur |
| Tâche périodique du poste principal (chaque minute) | `gesco-app` | Déclenche les envois dus |
| Frontend React, Vite, couche `api/` | `frontend/` | Seconde entrée pour le portail |

Ce qui n'existe pas : `gesco-server` et `gesco-sync` ont quitté l'espace de travail le 31 octobre
2026, vides. `gesco-server` revient, hors de `default-members` ; `gesco-sync` ne revient pas (ADR
« données qui montent »).

---

## 3. Le compte de l'école et l'abonnement

### 3.1 Relier l'école

1. Sur le poste principal, une personne qui a le droit `online:manage` ouvre
   **Administration › Service en ligne** et appuie sur « Relier l'école au service en ligne ».
2. GESCO envoie au serveur le fichier de licence de l'école, tel qu'il est installé.
3. Le serveur vérifie la signature avec la clé publique de l'éditeur, l'édition Connectée et
   l'échéance (grâce comprise). Si tout va, il crée le compte de l'école, nommé par l'identifiant
   de la licence, et rend un **secret de compte** tiré au hasard (256 bits). Il n'en garde que
   l'empreinte.
4. GESCO garde ce secret **dans la base chiffrée de l'école**, comme le certificat du réseau local :
   une école restaurée sur un autre ordinateur reste reliée sans rien refaire.
5. Chaque appel de l'école au serveur porte ce secret et la version du protocole.

Une licence qui n'est pas Connectée, ou échue au-delà de la grâce, est refusée avec un message
simple : « Cette licence ne comprend pas le service en ligne. » Rien d'autre ne change.

Une licence renouvelée s'installe comme aujourd'hui ; GESCO la renvoie au serveur au prochain
appel. Un même compte ne sert qu'à une licence ; une seconde école a sa propre licence.

### 3.2 Ce que l'abonnement allume

GESCO lit `modules_connectes_actifs` de la licence :
- **vrai** : le service peut être relié, la copie vers le serveur et l'envoi des vues partent ;
- **faux** (pas de licence Connectée, ou grâce passée) : rien ne part ; l'écran le dit en une
  phrase neutre (« L'abonnement au service en ligne a pris fin le 12 mars 2027. ») ; la
  récupération d'une archive reste possible (§4.5) ; tout le reste de GESCO marche comme avant.

Le serveur applique la même règle de son côté avec la licence qu'il a reçue, pour ne pas dépendre
de l'horloge du poste. Une seule formule d'abonnement couvre la sauvegarde et le portail, sauf
réponse contraire à T-151.

### 3.3 Le serveur injoignable

Pas d'Internet, serveur arrêté, horloge du poste fausse, pare-feu : rien ne bloque. L'envoi en
cours s'arrête, sera repris au prochain passage, et l'écran Service en ligne dit l'heure du dernier
envoi réussi. Une horloge du poste qui empêche la connexion sécurisée est nommée comme telle
(« L'heure de cet ordinateur semble fausse : réglez-la pour que les envois reprennent. »).

---

## 4. C1 — Sauvegarde sur le serveur de l'éditeur

### 4.1 Raccord avec la copie par dossier synchronisé

`SPEC_SAUVEGARDE_HORS_ECOLE.md` crée la « copie hors de l'école » et prévoit C1 en son §10. C1
**ajoute une sorte de destination**, `serveur`, et **reprend toutes les règles de cette
spécification** sans les redire :
- même section de l'écran **Sauvegardes › Copie hors de l'école** ; une école peut avoir le dossier
  synchronisé, le serveur, ou les deux ;
- **mêmes copies** : seules la copie à la demande et la copie du rythme partent (§3.2 de cette
  spécification) ;
- **même rythme**, partagé : à la demande (livré), chaque semaine, chaque mois ;
- **même politique de conservation** que le dossier synchronisé (les 4 dernières semaines et les
  12 derniers mois), appliquée par le serveur ;
- **mêmes états**, dans la même colonne `etat_envoi` ; un seul ajout, `en_cours` (envoi commencé,
  pas fini) ;
- **même bandeau** : une copie confirmée sur le serveur compte comme une copie hors de l'école ;
- **même règle** : n'y part que l'archive `.gsv` déjà chiffrée, jamais la base, jamais la clé.

Si l'une de ces règles change dans `SPEC_SAUVEGARDE_HORS_ECOLE.md`, C1 suit le changement.

### 4.2 Ce que C1 fait en propre

- **C'est GESCO qui envoie**, et non le logiciel d'un fournisseur. L'archive part par morceaux d'un
  mégaoctet. Chaque morceau confirmé n'est plus renvoyé. Coupure, fermeture de GESCO, ordinateur
  éteint : l'envoi reprend au prochain passage de la tâche périodique, au morceau suivant.
- **Confirmation** : à la fin, le serveur calcule l'empreinte de l'archive reçue et la renvoie.
  Égale à celle du poste : état « envoyée ». Différente : l'archive est effacée du serveur, état
  « envoi en échec », nouvel essai au passage suivant. « Envoyée » est donc toujours une certitude.
- **Volume** : l'écran affiche la taille de l'archive avant l'envoi et ce qui reste à envoyer
  pendant l'envoi. Pour l'école d'essai, environ 4,4 Mo par copie.
- **En arrière-plan** : l'envoi ne gêne pas le travail ; il ne s'arrête pas pour une saisie.
  Une seule archive à la fois ; une nouvelle copie due remplace une copie dont l'envoi n'a pas
  commencé.
- **Place sur le serveur** : la conservation suit §4.1 ; une archive plus grande que la limite par
  école (§13.4) est refusée avec un message clair, sans effet sur les autres destinations.

### 4.3 Récupérer sur un ordinateur neuf

Dans l'assistant de restauration existant, au premier démarrage d'un ordinateur neuf, une
troisième voie s'ajoute : « Récupérer depuis le service en ligne ».

1. La personne tape le **numéro de licence** de l'école (imprimé sur la licence et sur la fiche
   du service, §9.1) et les **douze mots**.
2. GESCO demande au serveur l'enveloppe de secours la plus récente du compte. Cette enveloppe est
   la clé de l'école enfermée par les douze mots : elle ne se lit pas sans eux, le serveur peut
   donc la remettre sans risque.
3. GESCO ouvre l'enveloppe avec les douze mots, en tire une **preuve de récupération** (dérivée de
   la clé de l'école, jamais la clé elle-même) et la présente au serveur, qui n'en garde qu'une
   empreinte, déposée au moment où l'école s'est reliée.
4. Preuve juste : le serveur liste les archives du compte ; la personne choisit (la plus récente
   par défaut) ; GESCO la télécharge, avec reprise, puis la restaure par le chemin existant.
5. Après la restauration, l'école est de nouveau reliée (le secret de compte était dans la base).

Douze mots faux : rien ne se télécharge. Les essais de preuve sont comptés : cinq faux par jour et
par compte, puis attente jusqu'au lendemain, et une alerte au porteur.

À chaque nouvelle phrase de secours, GESCO renvoie la nouvelle enveloppe au serveur avec la
copie suivante. Une archive plus ancienne garde l'enveloppe de son époque : la restaurer demande
les douze mots de son époque, comme aujourd'hui.

**Procédure de secours** : école qui a perdu ses douze mots. L'éditeur ne peut rien lire, et ne
peut donc rien restaurer ; il le dit. C'est la raison d'être du certificat imprimé.

Ce mécanisme touche à l'authentification : il s'écrit dans un ADR court (« compte de l'école et
récupération ») avant l'étape 3 du §14.

### 4.4 Ce que l'éditeur peut voir

Le nom de l'école (dans la licence), la date et la taille de chaque archive, son manifeste en clair
(versions de GESCO). Rien sur les élèves, le personnel ou l'argent. Le guide le dit.

### 4.5 Après la fin de l'abonnement

Les envois s'arrêtent. Les archives déjà sur le serveur restent récupérables par les douze mots
pendant douze mois après la fin de la grâce (T-157), puis sont effacées. L'écran Service en ligne
donne la date d'effacement.

---

## 5. C2 — Ce qui monte pour les parents

### 5.1 La vue

Une **vue par élève**, document au format versionné dont les types vivent dans
`gesco-core::en_ligne`, calculé dans `gesco-services` par les fonctions qui servent les écrans de
l'école (ADR « données qui montent »). Elle contient, pour l'année scolaire en cours (pendant les
vacances : la dernière année close, jusqu'à l'ouverture de la suivante) :

| Rubrique | Contenu | D'où il vient |
|---|---|---|
| Élève | Prénoms et nom, classe, nom de l'école | Fiche et inscription |
| Paiements | Reste à payer ; échéances à venir (date, montant) ; total versé | Même calcul que la fiche finance de l'élève |
| Reçus | Pour chaque versement : numéro du reçu, date, montant, mode de paiement, frais réglés ; un versement annulé apparaît comme annulé, sans motif | Caisse |
| Absences | Date, demi-journée ou séance, justifiée ou non ; retards s'ils sont saisis | Même lecture que la fiche de l'élève |
| Bulletins | Pour chaque période publiée de l'année, la dernière version publiée : moyennes par matière, moyenne générale, rang si l'école le montre par SMS ; sans appréciation ni décision (T-152) | Contenu figé du bulletin |
| Livret du jardin | Pour chaque période publiée : compétences et niveau atteint ; sans appréciation générale (T-152) | Contenu figé du livret |
| Date | Jour et heure du calcul | Poste principal |

Rien d'autre. Jamais : date de naissance, matricule, adresse, photo, autre responsable, sanction,
convocation, appréciation, note non publiée, information de santé ou de handicap, emploi du temps.

Une **vue par responsable** : son identifiant, ses numéros, l'empreinte de son code d'accès, la
liste de ses enfants dans cette école, et, si D4 est allumé, les réponses de l'école à ses messages.

### 5.2 Qui monte

Seuls montent les élèves dont **au moins un responsable a un code d'accès actif**, dans une école
qui a **allumé le portail**. Un responsable sans code ne monte pas. Un parent qui ne veut pas du
portail n'a qu'à ne pas demander de code, ou à le faire révoquer : c'est le geste déjà connu.

### 5.3 Quand

- **Chaque jour à l'heure réglée** (livrée : 17 h 30) si GESCO est ouvert sur le poste principal et
  qu'Internet répond ; sinon au premier passage de la tâche périodique après cette heure.
- **À la demande** : « Envoyer maintenant ».
- **À la publication d'un bulletin ou d'un livret** : GESCO tente aussitôt d'envoyer les vues des
  élèves concernés, pour que le parent qui reçoit le SMS trouve son bulletin.
- Le réglage « seulement à la demande » existe pour l'école qui veut tout maîtriser (T-158).

### 5.4 Comment

1. GESCO calcule les vues de tous les élèves concernés et leur empreinte (SHA-256 du document).
2. Il les compare à l'empreinte de la dernière vue acceptée par le serveur (table locale, §8.1).
3. Il envoie les vues différentes par **lots compressés** de 256 Ko au plus. Le serveur remplace la
   vue entière : un lot renvoyé ne crée pas de double.
4. Le dernier lot porte l'**inventaire** : la liste des élèves et des responsables présents. Le
   serveur efface ce qui n'y est plus (élève parti, code révoqué, année close) et coupe les
   téléphones reliés avec un code qui n'existe plus.
5. Le serveur répond par ce qui descend : nombre de téléphones reliés et dernière consultation par
   responsable ; messages des parents (D4).
6. GESCO note l'envoi au journal des envois (date, nombre de vues, volume, issue).

Un lot qui échoue se renvoie tel quel au passage suivant. Si le serveur a perdu ses données (panne,
§13.3), il le dit, GESCO oublie ses empreintes et renvoie tout.

### 5.5 Le journal `outbox`

- Après un envoi réussi, les lignes du journal antérieures au calcul sont marquées consommées. Elles
  servent seulement à savoir s'il y a lieu de recalculer avant l'heure.
- Les lignes consommées depuis plus de 90 jours sont purgées.
- Pour une école qui n'a jamais relié le service, les lignes de plus d'un an sont purgées. La trace
  des actions reste dans `journal_audit`, qui ne bouge pas.
- La purge passe une fois par jour, par petits paquets, hors d'une opération qui demande d'être seul.

### 5.6 Volume

Estimation à mesurer à l'étape 5 sur l'école d'essai : quelques kilo-octets compressés par élève,
moins d'un mégaoctet et demi au premier envoi d'une école de 500 élèves, quelques dizaines de
kilo-octets les jours ordinaires. L'écran montre le volume réel de chaque envoi.

---

## 6. C3 — Le serveur

### 6.1 Forme

Un binaire `gesco-server`, crate du dépôt **hors de `default-members`**, construit par la CI Linux.
Il dépend de `gesco-core` (types des vues, licence, code d'accès) et jamais de `gesco-services`.
Derrière Caddy, qui tient le certificat HTTPS ; il n'écoute que sur la machine elle-même (ADR
« hébergement »).

### 6.2 Ce qu'il sert

| Pour qui | Quoi |
|---|---|
| L'école (secret de compte) | Relier l'école ; envoyer une archive par morceaux, la confirmer ; envoyer des lots de vues et l'inventaire ; recevoir ce qui descend |
| Un ordinateur neuf (numéro de licence, puis preuve) | Enveloppe de secours ; liste et téléchargement des archives |
| Le parent (jeton d'appareil) | Les fichiers du portail ; relier un téléphone ; ses enfants ; leurs vues ; se déconnecter ; écrire (D4) |
| Tout le monde | Une adresse de santé qui répond « ça marche », sans rien dire d'autre |

Chaque requête de l'école porte la version du protocole. Le serveur accepte la version courante
et la précédente ; au-delà, il répond « mettez GESCO à jour pour continuer les envois », que GESCO
affiche sans rien bloquer d'autre.

### 6.3 Données du serveur

- **Registre** : comptes d'écoles (identifiant de licence, nom, licence reçue, empreinte du secret
  de compte, empreinte de la preuve de récupération, état, dates) ; journal d'administration.
- **Une base par école** : vues des élèves et des responsables (numéros sous forme d'empreinte à
  clé, empreinte du code chiffrée), téléphones reliés (empreinte du jeton, dates), compteurs
  d'essais, messages des parents (D4), journal des envois reçus.
- **Archives** : fichiers `.gsv` par école, avec leur empreinte et leur date.
- Bases chiffrées au repos ; clés hors des bases (§12.4).

### 6.4 Administration

Par une commande sur la machine, à travers SSH, jamais par le web :
`gesco-server admin ecoles` (liste, état, dernier envoi, place prise), `ecole <licence>` (détail),
`suspendre` et `reprendre` (impayé, demande de l'école), `effacer` (fin de contrat, après les délais
du §4.5 et de T-157), `rapport` (le rapport du jour, §13.2), `essai-restauration` (§13.3). Chaque
commande est écrite au journal d'administration.

---

## 7. D — Le portail des parents

### 7.1 D1 — Entrer

- **Adresse** : un sous-domaine du nom de domaine de l'éditeur (T-159), en HTTPS seulement. Elle
  est imprimée sur le billet du code d'accès quand l'école a allumé le portail.
- **Relier le téléphone** : le parent tape son numéro (indicatif du Mali prérempli, modifiable) et
  son code (six caractères, en deux groupes, majuscules ou minuscules). Le téléphone reçoit un jeton
  d'appareil ; le code ne se retape plus (ADR « identité »).
- **Refus** : même phrase pour un numéro inconnu et un code faux (« Numéro ou code incorrect. »).
  Cinq codes faux dans la journée bloquent le numéro jusqu'au lendemain (« Trop d'essais pour ce
  numéro. Réessayez demain, ou demandez un nouveau code à l'école. »), comme pour le SMS.
- **Plusieurs écoles** : un parent qui a des enfants dans deux écoles reliées tape le second code
  par « Ajouter un enfant » ; ses enfants des deux écoles s'affichent ensemble.
- **Durée** : 180 jours, prolongés à chaque visite ; cinq téléphones au plus par responsable.

### 7.2 D2 — Voir

- **Mes enfants** : une carte par enfant (prénoms, nom, classe, école), marquée « Nouveau » si
  quelque chose a changé depuis la dernière visite.
- **Pour chaque enfant**, trois parties : **Paiements** (le reste à payer en grand, les échéances à
  venir, la liste des reçus), **Absences**, **Bulletins** (une ligne par période publiée).
- **Un reçu** et **un relevé de bulletin** s'ouvrent en page lisible sur un téléphone, avec les
  mêmes chiffres que le papier.
- **Date des données**, en tête de chaque page, toujours : « Données de l'école du vendredi
  14 novembre à 17 h 32 ». C'est la seule indication d'âge ; elle n'est jamais une alerte.
- **Rien n'est écrit** sur ce qui n'est pas là : un enfant sans absence n'a pas de partie Absences
  vide qui le souligne, seulement « Aucune absence cette année. » ; un enfant sans bulletin publié
  n'a pas de partie Bulletins.
- **Lecture seule** : aucun bouton qui modifie quoi que ce soit, sauf « Écrire à l'école » (D4).

### 7.3 D3 — Hors ligne et avis

- Le service worker garde l'application et la dernière vue reçue de chaque enfant. Sans réseau,
  le portail s'ouvre et montre ce dernier état, avec sa date et la mention « Sans réseau ».
- « Se déconnecter » efface tout ce que le téléphone garde et coupe le jeton au serveur.
- **Installer** : sur Android, le portail propose l'ajout à l'écran d'accueil après la première
  visite réussie, une seule fois ; une page « Mettre sur l'écran d'accueil » montre le geste sur
  iPhone. Le portail marche aussi sans installation.
- **Avis** : pas de notification poussée (ADR « portail »). Quand le portail est allumé, le SMS de
  publication du bulletin, que l'école envoie déjà, porte en plus l'adresse courte du portail,
  **seulement si le texte tient dans le même nombre de SMS** ; sinon il part tel qu'aujourd'hui.

### 7.4 D4 — Écrire à l'école (éteint à la livraison)

Un réglage de l'école, éteint à la livraison, l'allume.
- Le parent écrit un message court (500 caractères au plus, texte seul, pas de pièce jointe) depuis
  la page d'un enfant. Trois messages par jour au plus par responsable.
- Le message descend à l'école au prochain envoi et apparaît dans **Messages des parents**, avec
  le nom du responsable et de l'enfant.
- L'école répond par un texte court ; la réponse monte avec la vue du responsable au prochain
  envoi. Elle peut aussi répondre par téléphone ou SMS, et marquer le message « traité ».
- Ni le message ni la réponse ne se modifient ; rien ne se supprime à l'école. Le serveur efface
  un message 30 jours après que l'école l'a reçu.
- Le portail dit : « L'école lit les messages pendant ses heures d'ouverture. »
- Éteindre D4 n'efface rien à l'école ; le bouton disparaît du portail au prochain envoi.

---

## 8. Données nouvelles

### 8.1 Dans la base de l'école (migration prise à l'étape 1)

- **Paramètres** : `en_ligne.portail` (éteint, livré), `en_ligne.heure_envoi` (17 h 30, livré),
  `en_ligne.envoi_a_la_demande_seulement` (faux), `en_ligne.messages_parents` (éteint, livré).
  Validés dans `gesco-core` ; modification auditée.
- **Compte en ligne** (une ligne) : secret de compte, date de la liaison, adresse du serveur
  utilisée, dernière licence envoyée. Dans la base chiffrée.
- **`envoi_en_ligne`** : par élève et par responsable, l'empreinte de la dernière vue acceptée, sa
  version de format, la date. Peut être vidée sans dommage (tout se renvoie).
- **`journal_envoi_en_ligne`** : date, sorte (vues ou archive), nombre de vues, volume, issue.
  Ajout seul.
- **Table `sauvegarde`** : la sorte `serveur` et l'état `en_cours` s'ajoutent aux contrôles posés
  par `SPEC_SAUVEGARDE_HORS_ECOLE.md` ; position du dernier morceau confirmé.
- **`message_parent`** (D4) : identifiant venu du serveur, responsable, élève, texte, reçu le,
  traité le, traité par ; **`reponse_parent`** : message, texte, écrite le, par. Ajout seul.
- **Responsable** : nombre de téléphones reliés et date de dernière consultation, mis à jour par ce
  qui descend.
- **Adresse du serveur** : constante de GESCO, remplaçable par un réglage du poste pour les essais
  (jamais par un écran).

### 8.2 Commandes nouvelles

Toutes réservées au poste principal, sauf la lecture de l'état.
`en_ligne_etat`, `en_ligne_relier`, `en_ligne_regler`, `en_ligne_envoyer`,
`en_ligne_journal`, `en_ligne_recuperer_lister` et `en_ligne_recuperer` (assistant de
restauration), `messages_parents_lister`, `message_parent_repondre`, `message_parent_traiter`.
La destination `serveur` se règle par la commande existante des destinations, la copie à la demande
par celle de `SPEC_SAUVEGARDE_HORS_ECOLE.md`.

---

## 9. Écrans

### 9.1 Au bureau

1. **Administration › Service en ligne** (droit `online:manage`)
   - *Compte* : relié ou non ; nom de l'école ; abonnement jusqu'au … ; bouton « Relier l'école » ;
     bouton « Imprimer la fiche du service » (numéro de licence, adresse du portail, rappel que
     les douze mots servent aussi à récupérer la sauvegarde en ligne ; jamais les douze mots).
   - *Portail des parents* : interrupteur ; adresse ; heure d'envoi ; « seulement à la demande » ;
     dernier envoi réussi (date, élèves en ligne, volume) ; bouton « Envoyer maintenant » ; envoi en
     cours avec ce qui reste.
   - *Messages des parents* : interrupteur.
   - *Journal des envois* : les trente derniers.
   - Sur un poste relié : lecture seule, sans bouton.
2. **Sauvegardes › Copie hors de l'école** : la destination « Serveur du service en ligne »
   s'ajoute, avec les états communs et la progression de l'envoi en cours.
3. **Assistant de restauration** : la voie « Récupérer depuis le service en ligne » (§4.3).
4. **Fiche du responsable**, panneau du code d'accès : « Portail : 2 téléphones reliés, dernière
   consultation le 12 novembre. » Rien si le portail est éteint.
5. **Billet du code d'accès** : quand le portail est allumé, l'adresse du portail et une phrase :
   « Ce code ouvre aussi le portail des parents. Gardez-le pour vous. »
6. **Messages des parents** (droit `parent_messages:manage`, visible seulement si D4 est allumé) :
   liste par date, non traités en tête ; ouvrir, répondre, marquer traité.
7. **Tableau de bord** : une seule ligne neutre, seulement si le portail est allumé et que le
   dernier envoi réussi a plus de sept jours : « Les parents voient les données du 3 novembre. »

### 9.2 Dans le portail (téléphone)

1. **Entrée** : numéro, code, « Entrer » ; lien « Où trouver mon code ? » (« Sur le billet remis par
   l'école. Si vous l'avez perdu, demandez-en un nouveau à l'école. »).
2. **Mes enfants** : cartes ; menu (Ajouter un enfant, Mettre sur l'écran d'accueil, Se
   déconnecter, Aide).
3. **Enfant** : en-tête (prénoms, nom, classe, date des données) ; Paiements ; Absences ;
   Bulletins ; « Écrire à l'école » si D4.
4. **Reçu**, **Relevé du bulletin**, **Livret** : pages simples, chiffres alignés, montants en
   francs CFA (`fr-ML`).
5. **Écrire à l'école** et le fil des messages (D4).
6. **Service arrêté pour cette école** (abonnement fini) : « Ce service n'est plus proposé par
   l'école. » Aucune donnée.

Lisibilité : niveau AA du projet, texte de 16 pixels au moins, cibles de 44 pixels, aucun sens porté
par la seule couleur, écran de 320 pixels sans défilement de côté (skill `lisibilite`).

### 9.3 Pour le porteur

Pas d'écran : la console `gesco-server admin` (§6.4) et le rapport du jour (§13.2).

---

## 10. Règles

1. **Rien ne bloque.** Aucune action de l'école n'attend le serveur. Un envoi qui échoue ne montre
   qu'un état et une heure, jamais une fenêtre qui interrompt.
2. **Un chiffre en ligne est un chiffre de l'école.** Le serveur ne calcule rien ; il range et montre
   les vues que le poste principal a calculées avec ses propres fonctions.
3. **Toujours daté.** Chaque page du portail porte la date de calcul des données qu'elle montre.
4. **Le moins possible.** Ne monte que ce que la vue décrit (§5.1), pour les seuls élèves dont un
   responsable a un code actif, dans une école qui a allumé le portail.
5. **Maître unique.** L'école écrit, le serveur lit. Ne descendent que des données créées au
   serveur qui ne se modifient jamais (messages des parents, compteurs de consultation).
6. **Ce qui s'arrête à l'école s'arrête en ligne** au prochain envoi : élève parti, code révoqué,
   portail éteint (tout le portail de l'école est vidé), abonnement fini.
7. **Une copie n'est « envoyée » que confirmée** par l'empreinte calculée au serveur.
8. **Récupérer est toujours possible** pendant le délai du §4.5, abonnement fini compris.
9. **Le même code pour le SMS et le portail**, avec la même règle de blocage ; le code ne quitte
   jamais l'école en clair.
10. **Aucune source affichée**, au bureau comme dans le portail.
11. **Rien de nouveau à l'école qui n'allume rien** : pas de connexion sortante, pas d'écran
    nouveau ailleurs que dans l'administration.

## 11. Droits

Deux permissions nouvelles, nommées selon `docs/matrice-permissions.md` :
- `online:manage` : relier l'école, régler et allumer le portail, envoyer maintenant, lire le
  journal des envois, imprimer la fiche du service — administrateur et directeur à la livraison
  (étape 1) ;
- `parent_messages:manage` : lire les messages des parents, répondre, marquer traité —
  administrateur, directeur et secrétariat à la livraison (étape 9).

Inchangées : la destination `serveur` et la copie à la demande suivent `core:backup:create` ; la
récupération suit `core:backup:restore` ; le panneau du responsable suit `guardians:access_code`.
Les maîtres n'ont aucune de ces permissions.

---

## 12. Sécurité

### 12.1 Données de mineurs

- **Ce qui sort de l'école** est la vue du §5.1, rien d'autre ; pour la sauvegarde, une archive que
  l'éditeur ne peut pas lire.
- **Isolation** : une base par école au serveur ; chaque requête du portail vérifie que l'élève
  demandé appartient au responsable du jeton, dans la base de son école. Un essai d'intrusion le
  vérifie pour chaque adresse.
- **Pas de traceur** : aucun script, police, image ou mesure d'audience venus d'ailleurs dans le
  portail ; politique de contenu stricte ; aucun témoin autre que le jeton d'appareil.
- **Journaux** sans numéro, code, jeton, nom ni montant ; adresses Internet gardées 30 jours
  seulement, pour les limites d'essais.
- **Effacement** : vues effacées dès qu'elles ne sont plus dans l'inventaire ; tout le compte d'une
  école effacé après les délais du §4.5 et de T-157 ; la copie de nuit suit avec son propre délai.
- **Information des parents** : une page « Vos données » du portail dit qui garde quoi, pour
  combien de temps, et à qui s'adresser. Son texte vient de l'avis du juriste (§15).
- **Rôles** : l'école est responsable des données ; l'éditeur les garde pour elle. Le contrat entre
  eux et les obligations envers l'APDP sont l'affaire du juriste (§15).

### 12.2 Codes exposés à Internet

- Le code ne sert qu'une fois par téléphone ; le jeton d'appareil (256 bits, témoin réservé au
  serveur, HTTPS seulement, illisible par le code de la page) fait le reste.
- Cinq codes faux par numéro et par jour ; vingt essais par heure par adresse Internet ; alerte au
  porteur au-delà d'un seuil global d'échecs par heure ; deux vérifications Argon2id à la fois au
  plus.
- Numéros rangés sous forme d'empreinte à clé ; empreintes des codes chiffrées par une clé du
  serveur : la base volée seule ne permet ni de lire les numéros, ni d'essayer les codes hors ligne.
- Même réponse pour un numéro inconnu et un code faux.
- Un code révoqué coupe ses téléphones au prochain envoi ; l'école peut envoyer aussitôt.
- Le secret de compte de l'école et la preuve de récupération : 256 bits, gardés en empreinte au
  serveur ; cinq preuves fausses par jour et par compte.

### 12.3 Canal

- HTTPS partout, certificat reconnu d'Internet (Caddy). GESCO vérifie le certificat avec la liste
  d'autorités qu'il embarque (webpki-roots), mise à jour à chaque version de GESCO.
- Aucun port ouvert à l'école : c'est toujours GESCO qui appelle.

### 12.4 Le serveur

- Accès SSH par clé seulement ; pas de mot de passe ; pare-feu : HTTPS et SSH seulement.
- Mises à jour de sécurité du système automatiques ; `gesco-server` dans un compte de service sans
  droits, avec les protections de systemd.
- Clés du serveur (chiffrement des bases, empreinte des numéros, chiffrement des empreintes de
  codes) dans des fichiers lisibles du seul compte de service, hors des bases ; un double chiffré
  chez le porteur, hors du serveur.
- Copie de nuit chiffrée, chez un second fournisseur, avec une clé que le serveur ne garde pas en
  clair à côté.
- **En cas de fuite** : procédure écrite dans le guide d'exploitation (couper le service, changer
  les clés, prévenir les écoles et l'autorité dans les délais que donnera le juriste).

---

## 13. Exploitation par une seule personne

### 13.1 Principe

**Le serveur peut tomber sans que rien ne casse à l'école.** Les écoles travaillent ; les parents
voient le dernier état gardé sur leur téléphone, daté. Le porteur n'a donc pas d'astreinte la nuit :
l'engagement écrit est un **rétablissement en 48 heures ouvrées** (T-160), pas une disponibilité
parfaite.

Le serveur ne garde presque rien qu'on ne puisse reconstruire : les vues reviennent des écoles au
prochain envoi, les téléphones se relient de nouveau avec le code. Seules les archives (C1) et les
messages des parents non encore descendus n'existent qu'au serveur : ce sont eux que la copie de
nuit protège.

### 13.2 Surveillance

- **De l'extérieur** : un service de surveillance (au choix du porteur) appelle l'adresse de santé
  toutes les cinq minutes et prévient le porteur par SMS ou courriel.
- **Rapport du jour**, envoyé chaque matin au porteur : place sur le disque, copie de nuit faite
  ou non, date du dernier essai de restauration, nombre d'écoles et d'envois reçus, refus et
  erreurs, échecs d'entrée, jours restants du certificat.
- **Alertes immédiates** : disque à 80 %, copie de nuit manquée, certificat à moins de 14 jours,
  pic d'échecs d'entrée, erreurs en série.

### 13.3 Panne et restauration

- **Scripts du dépôt** (`outils/serveur/`) : installer une machine neuve (système, Caddy, compte
  de service, pare-feu, `gesco-server`) ; restaurer depuis la copie de nuit ; mettre à jour le
  binaire.
- **Procédure de panne** écrite dans le guide d'exploitation, pas à pas : machine perdue, disque
  plein, certificat refusé, base abîmée, fuite.
- **Essai de restauration** de la copie de nuit chaque trimestre, sur une machine jetable ; la
  date du dernier essai est dans le rapport du jour.
- **Objectif** : serveur rétabli en moins de quatre heures de travail du porteur ; au plus une
  journée de données du serveur perdue (les écoles renvoient leurs vues seules).

### 13.4 Mises à jour et capacité

- Une mise à jour du serveur se fait en remplaçant le binaire et en le relançant (quelques
  secondes d'arrêt) ; le serveur accepte la version de GESCO courante et la précédente (§6.2).
- Limites par école, réglées dans la console : place pour les archives (16 copies, soit
  environ 70 Mo pour l'école d'essai, une limite de 500 Mo livrée), lots de vues par jour.
- Essai de charge avant la mise en service : cent écoles simulées, mille parents qui entrent
  en même temps ; temps de réponse mesurés sur la plus petite machine envisagée.

### 13.5 Ce que coûte le service

À chiffrer par le porteur avec l'hébergeur choisi (§15) : une petite machine virtuelle, la copie de
nuit chez un second fournisseur, le nom de domaine, le service de surveillance. Ces coûts sont fixes
jusqu'à plusieurs centaines d'écoles (à vérifier par l'essai de charge) ; c'est ce que l'abonnement
couvre (§26.3).

---

## 14. Ordre de réalisation

Après la phase 12, comme l'a fixé le porteur. Plusieurs étapes par session, commit et push après
chacune, relais au-delà d'environ 150 000 jetons ; une seule session compile à la fois. Par étape :
fmt, clippy, tsc, ESLint ; les essais s'écrivent avec le code et se lancent tous ensemble à
l'étape 11, avec l'essai Playwright global (règle du 14 novembre 2026).

Les étapes 1 à 9 se font **entièrement sur le poste de développement**, contre un serveur d'essai
lancé localement : elles n'attendent ni hébergeur, ni domaine, ni juriste.

1. **Serveur minimal et compte de l'école.** Crate `gesco-server` hors de `default-members`, construite
   par la CI Linux ; registre et base par école ; relier l'école par la licence ; secret de compte ;
   adresse de santé ; côté GESCO : client HTTPS vers Internet (webpki-roots, ADR de dépendance au
   jour même), écran Service en ligne (partie Compte), droit `online:manage`, migration. Critère 13.
2. **Copie de la sauvegarde vers le serveur.** Sorte `serveur`, envoi par morceaux, reprise,
   confirmation par empreinte, conservation au serveur, état `en_cours`, raccord à la section
   « Copie hors de l'école ». Critères 1 et 9.
3. **Récupérer sur un ordinateur neuf.** ADR « compte de l'école et récupération » d'abord ;
   enveloppe et preuve, voie de l'assistant de restauration, deux instances. Critère 2.
4. **Les vues.** Types `gesco-core::en_ligne`, calcul dans `gesco-services`, empreintes, table
   `envoi_en_ligne` ; essai « mêmes chiffres qu'à l'écran » ; essai de minimisation ; mesure de la
   taille sur l'école d'essai. Critères 3 et 4.
5. **L'envoi.** Lots, reprise, inventaire et effacements, heure d'envoi, à la demande, envoi à la
   publication, ce qui descend, journal des envois, licence (échéance, grâce), purge de `outbox`,
   partie Portail de l'écran. Critères 5, 6 et 10.
6. **Portail : entrer.** Entrée Vite `frontend/portail/`, manifeste, entrée, jeton d'appareil,
   limites d'essais, politique de contenu, page « Vos données » (texte provisoire), billet du code
   avec l'adresse. Critère 7.
7. **Portail : voir.** Mes enfants, Paiements, Absences, Bulletins, reçu, relevé, livret ; date des
   données ; « Nouveau » ; lisibilité à 320 pixels ; mesure du premier chargement. Critère 8 (partie
   écran).
8. **Hors ligne et avis.** Service worker, dernier état, « Se déconnecter » qui efface,
   installation, adresse dans le SMS de bulletin. Critère 8 (partie hors ligne).
9. **Écrire à l'école.** Messages, réponses, écran Messages des parents, droit
   `parent_messages:manage`, interrupteur éteint à la livraison.
10. **Exploitation.** Chiffrement des bases du serveur, clés hors des bases, copie de nuit,
    surveillance et rapport du jour, console d'administration, scripts d'installation et de
    restauration, guide d'exploitation, procédure de panne. Critères 11 et 12 sur une machine
    jetable.
11. **Livraison.** Tous les essais (suite Rust complète, `gesco-server` compris, vitest, knip),
    essai Playwright global (bureau et portail sur un téléphone simulé), essai d'intrusion simple,
    essai de charge ; manuel, notes de version, captures ; **puis**, quand le porteur a fourni ce
    que demande le §15 : mise en service sur la vraie machine, école pilote avec quelques parents.
    Critère 14.

Les étapes 2 et 3 (sauvegarde) et les étapes 4 et 5 (vues) ne dépendent que de l'étape 1 et
touchent des fichiers différents. Les étapes 6 à 9 dépendent de l'étape 5. L'étape 10 peut
commencer après l'étape 1.

---

## 15. Ce que seul le porteur peut fournir

Rien de cette liste ne retarde les étapes 1 à 10 ; tout est nécessaire avant la mise en service
réelle (étape 11).

1. **L'avis écrit d'un juriste** qui connaît la loi malienne sur les données personnelles : pays
   d'hébergement permis ; déclaration ou autorisation auprès de l'APDP, et par qui (l'école,
   l'éditeur, les deux) ; contrat entre l'école et l'éditeur pour la garde des données ; texte de la
   page « Vos données » et des conditions d'utilisation du portail ; durées de conservation ;
   conduite à tenir en cas de fuite (qui prévenir, en combien de temps) ; s'il faut l'accord écrit
   des parents ou si le code remis au guichet suffit.
2. **L'hébergeur** de la machine virtuelle et le **second fournisseur** de la copie de nuit, choisis
   après l'avis du juriste ; les comptes ouverts au nom de l'éditeur, avec son moyen de paiement.
3. **Le tarif** de l'abonnement Connectée (montant, durée, par tranche d'élèves ou non), et la façon
   dont l'école paie.
4. **Le nom de domaine** de l'éditeur, enregistré à son nom, et le nom du portail tel que les parents
   le verront (T-159).
5. **Les moyens d'alerte** : l'adresse électronique ou le numéro qui reçoit les alertes et le
   rapport du jour ; le choix du service de surveillance extérieure.
6. **La clé SSH** du porteur (créée sur son poste) et un endroit sûr, hors du serveur, pour le
   double des clés du serveur et de la copie de nuit.
7. **Une école pilote** qui a Internet, accepte l'abonnement et l'essai, et **quelques parents**
   volontaires avec des téléphones variés (Android d'entrée de gamme, iPhone) ; un téléphone
   Android d'entrée de gamme pour les essais du porteur.
8. **Les licences Connectée** de l'école pilote, émises par l'outil de licence.
9. **Une personne de secours**, si le porteur le souhaite, qui saurait suivre la procédure de panne
   s'il est indisponible longtemps (facultatif).

## 16. Questions au porteur du projet

À la suite des questions des spécifications écrites en même temps : numérotées à partir de T-150
pour ne pas les croiser. Chacune a une recommandation ; les étapes 1 à 10 commencent sur les
recommandations.

### T-150 — Où héberger

Le serveur garde des données d'élèves mineurs.
- a) chez un hébergeur hors d'Afrique, si le juriste le permet (prix bas, grande fiabilité) ;
- b) chez un hébergeur au Mali ou dans la sous-région ;
- c) le juriste décide, puis on choisit.

*Recommandation* : c). L'installation est la même partout (une machine virtuelle sous Debian) ; on
ne choisit l'hébergeur qu'avec l'avis du juriste en main.

### T-151 — Une seule formule d'abonnement

- a) un seul abonnement Connectée : sauvegarde en ligne et portail ensemble ;
- b) deux : sauvegarde en ligne seule, ou sauvegarde et portail.

*Recommandation* : a). Une seule chose à vendre et à expliquer ; la licence le porte déjà sans
changer de format. Le tarif peut suivre le nombre d'élèves.

### T-152 — Le bulletin dans le portail

- a) un relevé : moyennes par matière, moyenne générale, rang si l'école le montre par SMS ; sans les
  appréciations ni la décision du conseil ; pour le jardin, les compétences sans l'appréciation
  générale ;
- b) le bulletin entier, appréciations comprises, puisque le parent le reçoit sur papier.

*Recommandation* : a), qui suit votre réponse sur ce que voit un parent (« les appréciations se
disent en face »). Les chiffres sont ceux du papier ; le papier reste le document de l'école.

### T-153 — Les PDF

Faut-il pouvoir télécharger le PDF du bulletin et du reçu dans le portail ?
- a) non : des pages lisibles sur téléphone, avec les mêmes chiffres ;
- b) oui, en plus.

*Recommandation* : a) pour la 2.0. Un PDF A4 se lit mal sur un téléphone, pèse sur le forfait de
l'école à l'envoi, et le parent a déjà le papier. On l'ajoutera si l'école pilote le demande.

### T-154 — Entrer dans le portail

- a) numéro et code une fois par téléphone, puis le téléphone reste relié 180 jours, prolongés à
  chaque visite ;
- b) numéro et code à chaque visite ;
- c) en plus, un code de confirmation par SMS à chaque entrée.

*Recommandation* : a). Le code est moins exposé, le parent n'a rien à retenir, et un téléphone perdu
se coupe en remettant un nouveau code, comme aujourd'hui. c) coûterait un SMS à chaque entrée.

### T-155 — Écrire à l'école

- a) construit, éteint à la livraison ; l'école l'allume si elle veut ;
- b) pas dans la 2.0.

*Recommandation* : a), comme vous l'avez demandé pour la saisie des notes par les maîtres : la
possibilité existe, l'école décide.

### T-156 — Le rang dans le portail

- a) comme pour le SMS : l'école choisit si le rang se montre, un seul réglage pour les deux ;
- b) jamais de rang dans le portail.

*Recommandation* : a). Une seule règle ; l'école garde la main.

### T-157 — Combien de temps garder

Après la fin de l'abonnement :
- a) les archives restent récupérables 12 mois, puis sont effacées ; les vues et les téléphones
  reliés sont effacés au bout de 90 jours ;
- b) tout est effacé à la fin de la grâce.

*Recommandation* : a), sous réserve de l'avis du juriste. Une école qui arrête de payer ne doit pas
perdre sa dernière copie hors de ses murs ; les données des parents, elles, n'ont plus de raison
d'être en ligne.

### T-158 — Quand partent les données des parents

- a) chaque jour à 17 h 30 (heure réglable), à la publication d'un bulletin, et à la demande ;
  « seulement à la demande » possible ;
- b) seulement à la demande.

*Recommandation* : a). Le volume est petit (quelques dizaines de kilo-octets par jour), et un
portail en retard de plusieurs jours sur l'école ferait douter des chiffres.

### T-159 — Le nom du portail et son adresse

Comment les parents l'appelleront-ils, et quelle adresse sur le billet (par exemple
`parents.<votre domaine>`) ?

*Recommandation* : un nom court et une adresse courte, les mêmes pour toutes les écoles ; le nom de
l'école apparaît dans le portail, pas dans l'adresse.

### T-160 — Ce que l'éditeur promet

- a) « rétabli en 48 heures ouvrées », écrit dans le contrat ;
- b) une disponibilité chiffrée (par exemple 99,5 %).

*Recommandation* : a). L'école ne dépend pas du serveur pour travailler ; une seule personne ne peut
pas tenir une promesse de disponibilité chiffrée la nuit et le dimanche.

## 17. Journal des révisions

- v1.0, 15 novembre 2026 : première rédaction, d'après la décision du porteur du 14 novembre 2026
  (« on met tout dans la v2 »), `docs/plan-phase-11.md` et le code au 15 novembre 2026 ; quatre ADR
  proposés le même jour.
