# SPEC_PHASE_6.md — Personnel ; enseignement technique et professionnel

**Projet** : ERP de gestion scolaire — Tauri 2 + Rust + React/TypeScript
**Phase** : 6 — registre et dossier du personnel, engagements (permanents et vacataires), état mensuel des heures, absences et congés du personnel ; enseignement technique et professionnel (CAP, BT)
**Statut** : v1.0 — spécification engagée le 14 octobre 2026 après les réponses du porteur du projet à T-39 à T-48 ; Q-63 à Q-72 proposées avec leur recommandation (§14)
**Documents parents** : `PROJECT_BLUEPRINT.md` v0.23 (§8.2, §8.7, §9.2, §9.3, §10.5, §11, §20 RH), `ARCHITECTURE_RULES.md` v0.3 (R-11, R-14, R-17, R-20), `SPEC_PHASE_4.md` v1.0, `docs/questions-terrain.md` (T-39 à T-48)
**Date** : 14 octobre 2026

---

## 0. Objet et critère de fin

### 0.1 Ce que produit cette phase

Le livrable du blueprint (§22.2) : **le dossier du personnel complet**. Ordre des phases décidé le 14 octobre 2026 : 6, 7, 8 et 10, puis 5 et 9 ; la paie (phase 7) s'appuie sur ce que cette phase enregistre. S'y ajoute, à la demande du porteur du projet (T-48), **l'enseignement technique et professionnel** : CAP et BT, avec des semestres ou des trimestres.

À la fin de la phase :
- tout le personnel de l'école a sa fiche, dans des catégories et des fonctions que l'école choisit ;
- chaque enseignant de la phase 4 est un membre du personnel, ses affectations inchangées ;
- chacun a son engagement de l'année : salaire de base pour le personnel permanent, taux de l'heure pour un vacataire, souvent fonctionnaire de l'État ;
- chaque mois, l'état des heures de chaque vacataire est arrêté et visé par la direction ;
- les absences, retards, congés et permissions du personnel sont notés ;
- les montants ne se voient que par qui en a le droit ;
- une école technique ou un centre de formation inscrit ses élèves en CAP ou en BT, dans ses filières, et produit des bulletins semestriels ou trimestriels.

### 0.2 Décisions déjà prises qui s'appliquent

- **Souplesse** (T-39) : aucune liste fermée de catégories ou de fonctions ; GESCO propose, l'école renomme, ajoute, retire.
- **Fonctionnaires de l'État** (T-40) : pas de contrat de travail, un engagement de vacation pour l'année ; l'indication « fonctionnaire » et son numéro, facultatif ; **pas de service d'origine**.
- **Heures des vacataires** (T-41) : un état par mois, heures prévues et heures faites, visé par la direction ; payé en phase 7.
- **Engagement du personnel permanent** (T-42) : écrit ou non ; un engagement à durée déterminée est signalé deux mois avant son terme.
- **Présence** (T-43) : absences et retards seulement, pas de pointage.
- **Congés** (T-44) : demandés, accordés ou refusés par la direction ; décompte des jours acquis désactivé par défaut.
- **Droits** (T-46) : la direction gère tout, le comptable voit les montants, le secrétariat tient les fiches sans les montants, chaque enseignant voit sa fiche.
- **Cantine** (T-47) : module activable, en phase 8 avec les dépenses ; le personnel de cuisine est dans le registre dès cette phase.
- **CAP et BT** (T-48) : inclus, avec des semestres ou des trimestres au choix de l'école.
- Un vacataire peut enseigner dans plusieurs écoles, chacune ne voyant que lui (T-24). Aucune règle fiscale ou sociale écrite en dur (§8.7) : la paie les paramètre en phase 7.
- R-14 : rien ne se supprime ; un membre qui part est marqué parti, un engagement se termine, une absence s'annule.

### 0.3 Critère de fin, vérifiable

La phase est terminée quand ces dix assertions sont vraies, chacune démontrée par un test automatisé ou par un scénario Playwright :

1. Un membre du personnel s'enregistre avec une fonction choisie dans la liste de l'école ou écrite librement ; les catégories et les fonctions se renomment et se retirent sans toucher aux fiches existantes.
2. À la mise à niveau, chaque enseignant existant devient un membre du personnel ; ses affectations et son compte restent les mêmes.
3. Un engagement de vacation porte le taux de l'heure et les heures prévues par semaine ; un engagement permanent porte le salaire de base ; un membre n'a jamais deux engagements en cours qui se chevauchent ; le renouvellement crée un nouvel engagement lié au précédent.
4. Un engagement à durée déterminée qui finit dans moins de soixante jours apparaît au tableau de bord.
5. L'état des heures d'un mois se calcule à partir des engagements de vacation ; une fois visé, il ne change plus que par une réouverture motivée, tracée.
6. Une absence du personnel pendant un congé accordé est refusée ; un congé se demande, s'accorde ou se refuse (motif obligatoire) et s'annule avant son début.
7. Un compte sans le droit de voir les montants reçoit les fiches et les engagements sans aucun montant, du service aux types TypeScript ; un enseignant ne lit que sa propre fiche.
8. Toute écriture sur le personnel est auditée avec la valeur avant et après, montants compris.
9. Une école crée une classe de CAP ou de BT dans une filière à elle, choisit semestres ou trimestres, inscrit, note et publie les bulletins de la classe en PDF.
10. Les tests de la phase 4 passent sans changement : le 1er cycle, le 2e cycle et le lycée gardent leurs mois et leurs trimestres, jamais de semestre.

## 1. Périmètre

**Inclus** : registre du personnel, catégories et fonctions, dossier (pièces, diplômes, numéro INPS, numéro de fonctionnaire, moyen de paiement) ; rattachement des enseignants ; engagements permanents et de vacation, renouvellement, terme, lettre d'engagement imprimable ; état mensuel des heures des vacataires ; absences et retards du personnel ; congés et permissions, décompte facultatif ; droits et rôles en version 7 ; bloc « Personnel » du tableau de bord ; enseignement technique et professionnel (cycle, niveaux, filières, périodes, règles, matières, bulletins).

**Exclu** :
- Paie, bulletins de paie, retenues et cotisations : phase 7. Cette phase enregistre les montants de base et les heures, sans rien calculer de la paie.
- Emploi du temps, séances et préremplissage de l'état des heures : phase 8. Cantine : phase 8 (T-47).
- Import du personnel depuis un fichier (`import:staff`) : plus tard, si une école le demande.
- Évaluation du personnel, carrière, formation continue.
- Examens nationaux du CAP et du BT, stage suivi au jour le jour : seule la note de stage entre dans la moyenne.

---

# Partie A — Personnel

## 2. Membre du personnel

**MembrePersonnel** — `personne` (la `Personne` de la phase 2 : nom, prénoms, sexe, date de naissance, téléphones, adresse), `matricule` (attribué `P-0001`, `P-0002`… ; l'école peut le remplacer par le sien, unique dans l'établissement), `categorie`, `fonction` (texte), `arrive_le`, `parti_le` (avec motif), `fonctionnaire` (vrai ou faux) et `numero_fonctionnaire` facultatif, `numero_inps` facultatif, `personne_a_prevenir` (nom et téléphone), `paiement` (espèces ; mobile money avec numéro ; banque avec intitulé et numéro de compte), `utilisateur` facultatif (compte lié).

**Catégorie** — liste de l'école ; livrées : *Enseignement*, *Direction et administration*, *Appui*. **Fonction** — liste de l'école, rattachée à une catégorie ; livrées : directeur, directeur des études, censeur, secrétaire, comptable, caissier, surveillant, enseignant, maître, gardien, agent d'entretien, chauffeur, cuisinière. Une fonction se renomme ou se retire ; une fiche garde le texte de sa fonction. Une fonction peut aussi s'écrire librement sur la fiche (T-39).

**Diplôme ou qualification** — intitulé, délivré par, année ; gardés sur la fiche. **Pièce du dossier** — un PDF ou une image de 2 Mo au plus, gardé dans la base chiffrée avec son intitulé, sa nature (pièce d'identité, diplôme, engagement signé, autre) et son empreinte SHA-256 ; il se réenregistre hors de GESCO à l'identique ; une pièce se retire du dossier sans quitter la base.

**Enseignant** (phase 4) — l'enseignant et le membre partagent la même personne : c'est le lien, sans colonne nouvelle. À la mise à niveau, chaque enseignant existant devient un membre de la catégorie *Enseignement* (fonction *maître* s'il a été affecté au 1er cycle, *enseignant* sinon) ; ses affectations ne bougent pas (Q-55, Q-63). Un enseignant créé dans « Enseignants » entre aussi au registre. Un membre devient enseignant d'un clic, sans ressaisie, avec son compte s'il en a un.

*Règles* : un membre parti garde sa fiche, ses engagements et son historique ; l'enseignant qu'il était cesse d'être actif ; son compte se désactive dans « Utilisateurs ». Un membre parti peut revenir. Une catégorie retirée reste sur les fiches qui la portent sans en recevoir de nouvelles ; une fonction retirée sort de la liste proposée, les fiches gardent leur texte.

## 3. Engagement

**Engagement** † — `membre`, `nature`, `debut`, `fin` (facultative sauf à durée déterminée), `fonction`, `remuneration`, `statut`, `precedent` (l'engagement renouvelé), `note`.

| Nature | Rémunération | Fin |
|---|---|---|
| Durée indéterminée | salaire de base mensuel | sans |
| Durée déterminée | salaire de base mensuel | exigée |
| Vacation | taux de l'heure, heures prévues par semaine | fin de l'année scolaire par défaut |
| Stage | indemnité mensuelle, éventuellement nulle | exigée |
| Bénévole | aucune | facultative |

États : **en cours → terminé** (à sa fin, ou avant avec un motif) ; **en cours → rompu** (motif obligatoire). Table exhaustive des transitions, comme pour les inscriptions.

*Règles* :
- deux engagements en cours d'un même membre ne se chevauchent pas ; un changement de salaire ou de taux est un nouvel engagement qui succède au précédent à une date, jamais une modification rétroactive (la paie de la phase 7 en dépend) ;
- **renouveler** reprend la nature, la fonction et la rémunération pour la période suivante : l'année scolaire suivante pour une vacation, autant de mois pour un engagement de mois entiers (du 1er juin au 30 novembre, puis du 1er décembre au 31 mai), autant de jours sinon ; tout se modifie avant d'enregistrer ;
- **terminer** avant la fin prévue exige un motif ; **rompre** aussi ; le dernier jour tombe entre le début et la fin prévue ;
- un engagement à durée déterminée ou un stage qui finit dans moins de soixante jours est signalé (tableau de bord, fiche) ;
- montants en francs entiers (`Montant`), comme toute la finance.

**Lettre d'engagement** — document imprimable (modèle `engagement`, A4 par défaut, format réglable comme les autres documents), signée hors du logiciel ; jamais exigée (T-40, T-42).

## 4. État des heures des vacataires

**HeuresVacation** — une ligne par engagement de vacation et par mois : heures prévues, heures faites, observation, état, visa (qui, quand), motif du dernier déverrouillage. Elle se crée à la première saisie ; l'état du mois rassemble les lignes de toutes les vacations qui le couvrent.

- Les heures prévues du mois se proposent d'après l'engagement : heures par semaine × nombre de semaines de cours du mois (Q-66) ; la direction les corrige.
- Les heures faites se saisissent en heures et quarts d'heure (`12`, `12,5`, `12,25`) ; un écart avec les heures prévues se commente (cours manqués, rattrapages).
- États : **en saisie → visée → en saisie** (déverrouillage motivé) **→ visée**. On vise une vacation seule ou tout le mois, chaque ligne ayant ses heures faites. Une ligne visée ne change plus ; la phase 7 paie les heures visées au taux de l'engagement. Le montant des heures faites (taux × heures, tronqué au franc) s'affiche dès la saisie.
- Écran mensuel : tous les vacataires du mois sur une grille, au clavier comme la saisie des notes (Entrée descend) ; l'état imprimable d'un vacataire ou de tous (modèle `etat_heures`).

## 5. Absences et retards du personnel

**AbsencePersonnel** — `membre`, `date`, `nature` (absence d'une journée, d'une demi-journée, retard en minutes), `motif`, `justifiee`, `annulee`.

*Règles* : jour à venir refusé ; une absence pendant un congé accordé est refusée (le congé suffit) ; une absence se justifie après coup ; une absence saisie par erreur s'annule avec un motif (R-14). Pour un vacataire, les cours manqués se notent plutôt dans l'état des heures (T-43). Une seule absence non annulée par membre et par jour : la corriger, c'est l'annuler puis la noter de nouveau. Un retard se compte de 1 à 240 minutes ; aucun jour hors de la présence du membre (avant son arrivée, après son départ).

## 6. Congés et permissions

**Conge** — `membre`, `type`, `debut`, `fin`, `jours`, `motif`, `statut`, `decide_par`, `motif_refus`. **Types** : congé annuel, permission d'absence, congé de maladie, congé de maternité, autre (libellé libre) ; l'école renomme et ajoute des types.

États (§10.5 du blueprint) : **demandé → accordé** ou **refusé** (motif obligatoire) ; **accordé → annulé** avant son début. La direction (`leave:approve`) décide ; le secrétariat peut enregistrer une demande (`leave:request`). Une demande ne s'annule pas : la direction la refuse, avec un motif. Deux congés demandés ou accordés d'un même membre ne se chevauchent pas ; un congé peut se noter après coup (une permission prise la veille) ; il ne dépasse pas un an. Un type de congé se retire sans quitter les congés qui le portent ; chaque type dit s'il se déduit du solde (le congé annuel livré, seul).

*Jours* : comptés du lundi au vendredi, le samedi compté si l'école le règle (Q-68). *Décompte* : désactivé par défaut (T-44) ; activé, chaque membre acquiert chaque mois un nombre de jours réglé par l'école (2,5 par défaut) et un congé annuel accordé au-delà du solde est signalé, sans être bloqué. Le solde au premier jour d'un congé : les jours acquis par mois entier de présence depuis l'arrivée, moins les jours des congés accordés qui se déduisent du solde ; sans date d'arrivée sur la fiche, pas de solde. Les trois réglages (`conge.samedi_compte`, `conge.decompte_actif`, `conge.jours_par_mois`) sont dans les paramètres de l'école.

## 7. Droits

| Droit | Ce qu'il permet |
|---|---|
| `employees:read` | lire le registre et les fiches, **sans montants** |
| `employees:update` | créer et modifier les fiches, le dossier, les absences |
| `contracts:read` | voir les montants des engagements et des états des heures (nouveau, Q-67) |
| `contracts:manage` | créer, renouveler, terminer les engagements ; viser et rouvrir les états des heures |
| `leave:request` / `leave:approve` | enregistrer une demande de congé / l'accorder, la refuser |

Rôles livrés en **version 7** (Q-67) : le Directeur reçoit `employees:update`, `contracts:read`, `contracts:manage`, `leave:request` ; le Comptable `employees:read` et `contracts:read` ; le Secrétaire `employees:read`, `employees:update`, `leave:request` ; le Gestionnaire RH `contracts:read` en plus des siens. Un compte lié à un membre lit **sa propre fiche** et ses engagements, montants compris, sans aucun droit (comme la portée des notes de la phase 4).

*Règle* : les montants sont retirés **dans le service**, pas dans l'écran : un compte sans `contracts:read` reçoit des engagements dont la rémunération est absente. Concrètement : `employees:read` ouvre en lecture les engagements d'une fiche et l'état des heures ; sans `contracts:read`, la rémunération d'un engagement arrive « masquée » (un engagement bénévole reste « sans rémunération », il n'a rien à cacher), l'état des heures sans taux ni montant, le journal d'audit avec la rémunération masquée. La lettre d'engagement et l'état imprimé des heures, qui portent les montants, exigent `contracts:read` ; conclure, clôturer, saisir et viser restent à `contracts:manage`. Une rémunération masquée ne s'enregistre jamais. La fiche personnelle se lit dans « Mon compte ».

## 8. Tableau de bord

Bloc « Personnel », selon les droits : effectif par catégorie, vacataires du mois et états des heures restant à viser, engagements qui finissent dans moins de soixante jours, absences du jour, congés demandés en attente, personnel en congé aujourd'hui. Il paraît avec `employees:read`, même sans année ouverte, et ne porte aucun montant ; les heures à viser comptent les vacations du mois en cours, et celles du mois précédent encore à viser sont signalées.

---

# Partie B — Enseignement technique et professionnel

## 9. Cycle, niveaux et filières

**Cycle** — nouveau cycle **Enseignement technique et professionnel** (`technique`), après le lycée, dans les écoles qui l'activent ; il ne change rien aux quatre autres.

**Niveaux livrés** (Q-69) : CAP 1re année et CAP 2e année (diplôme terminal *CAP*) ; BT 1re à 4e année (diplôme terminal *BT* en dernière année). Comme pour les autres cycles, l'école renomme et désactive les années qu'elle n'enseigne pas ; durées à confirmer avec une école de ce type (T-48).

**Filières** — les séries de la phase 2, étendues aux niveaux du cycle technique ; **aucune filière livrée** : l'école crée les siennes (électricité, mécanique automobile, génie civil, comptabilité, secrétariat…), avec un code et un libellé. Une classe porte son niveau et sa filière (« CAP 1 Électricité »). Une filière se crée sur les années où elle s'enseigne (une série par année) ; son code s'écrit en majuscules, dix caractères au plus (`ELEC`, `MECA-AUTO`) ; elle ne se crée que sur les années du CAP et du BT, les séries du lycée restant celles du référentiel national. Une année qui a des filières exige la filière de chaque classe nouvelle, comme une année du lycée sa série. Les années ouvertes reçoivent aussitôt le programme par défaut de la filière.

**Ouverture** — à l'installation, l'école coche le cycle parmi ceux qu'elle enseigne ; une base installée avant la phase 6 reçoit les années du CAP et du BT fermées, et l'école ouvre le cycle dans « Niveaux et séries ».

**Matricule national** : facultatif dans ce cycle (Q-70).

## 10. Périodes, règles et matières

- **Périodes** (T-48) : au choix de l'école pour chaque niveau du cycle : **deux semestres** (défaut : octobre-janvier, février-juin), trois trimestres, ou deux trimestres (Q-71). Le semestre n'existe que dans ce cycle ; ailleurs, rien ne change : au 2e cycle et au lycée, un semestre demandé est refusé. Le découpage se choisit dans « Pédagogie », niveau par niveau.
- **Règles d'évaluation** : celles du lycée par défaut : barème /20, coefficients, devoirs et composition, (note de classe + 2 × composition) ÷ 3, mentions, moyenne annuelle imprimée en dernière année de chaque diplôme.
- **Matières** (Q-72) : la pratique et le stage sont des matières à part entière, avec leur coefficient ; livrées, à ajouter aux programmes : *Technologie*, *Dessin technique*, *Travaux pratiques d'atelier*, *Stage en entreprise*, *Législation*, *Gestion et économie d'entreprise*, *Hygiène et sécurité*, en plus des matières générales (français, anglais, mathématiques, physique-chimie). Le programme se règle par niveau **et filière**, avec le mécanisme du lycée. Codes : `TECH` (la technologie du 1er cycle), `DTECH`, `TPA`, `STAGE`, `LEGIS`, `GEST`, `HSE`. Le programme par défaut d'une filière porte les quatre matières générales, sans coefficient ; la pratique et le stage s'y ajoutent selon la filière.

## 11. Bulletins et documents

Le bulletin du 2e cycle et du lycée sert tel quel ; sa période s'écrit « 1er semestre ». Les réglages du bulletin imprimé (colonnes et lignes) existent pour ce cycle comme pour les autres. La fiche d'inscription, le certificat de transfert et le relevé des moyennes portent le niveau et la filière. Ils l'écrivent après la classe : « CAP 1 Électricité (CAP 1re année, Électricité) ». Le matricule national figure sur le certificat de transfert et le relevé quand l'élève en a un, comme au lycée.

---

## 12. Modèle de données

Une migration par étape. 0022 (étape 1) : `categorie_personnel` et `fonction_personnel` (code stable des livrées), `membre_personnel` (une personne, un matricule et un compte uniques ; paiement et diplômes en JSON), `piece_dossier` (contenu dans la base chiffrée). 0023 (étape 2) : `engagement` (versions liées, jamais supprimé). 0024 (étape 3) : `heures_vacation`, une ligne par vacation et par mois, l'état du mois étant leur ensemble. 0025 (étape 4) : `absence_personnel`, `type_conge` (code stable des livrés), `conge`. 0026 (étape 6) : `niveau` et `regle_evaluation` reconstruites à l'identique, contrôles élargis au cycle `technique` et aux diplômes `cap` et `bt` (la vérification des clés étrangères est reportée à la fin de la transaction) ; la période `semestre` était admise depuis la phase 0.5 ; niveaux livrés et filières sont des lignes, posées par le service. Rien ne se supprime (R-14) : statuts finaux et déclencheurs de non-suppression comme ailleurs.

## 13. Événements

`MembreEnregistre` (création ou modification), `MembreParti` (départ ou retour), `ReferentielPersonnelModifie`, `PieceDossierAjoutee`, `PieceDossierRetiree`, `EngagementConclu`, `EngagementTermine`, `EngagementRompu`, `EtatHeuresSaisi`, `EtatHeuresVise`, `EtatHeuresDeverrouille`, `AbsencePersonnelNotee`, `AbsencePersonnelJustifiee`, `AbsencePersonnelAnnulee`, `CongeDemande`, `CongeAccorde`, `CongeRefuse`, `CongeAnnule`.

## 14. Questions de conception

| Id | Question | Recommandation | Étape |
|---|---|---|---|
| **Q-63** | L'enseignant de la phase 4 et le membre du personnel ? | **Deux fiches liées** : le membre porte le dossier et les engagements, l'enseignant garde ses affectations et sa portée ; la mise à niveau crée un membre pour chaque enseignant | 1 |
| **Q-64** | Matricule du personnel ? | `P-0001` attribué dans l'ordre, remplaçable par celui de l'école, unique dans l'établissement | 1 |
| **Q-65** | Changer un salaire ou un taux ? | **Un nouvel engagement à une date**, jamais une modification de l'ancien ; la paie d'un mois passé reste juste | 2 |
| **Q-66** | Heures prévues d'un mois ? | Heures par semaine × semaines de cours du mois (4 proposé, corrigé par la direction) ; l'emploi du temps de la phase 8 les calculera | 3 |
| **Q-67** | Montants cachés : comment ? | Droit nouveau `contracts:read` ; montants retirés par le service ; rôles en version 7 (§7) | 5 |
| **Q-68** | Jours d'un congé ? | Du lundi au vendredi ; samedi compté si l'école le règle ; pas de jours fériés en phase 6 | 4 |
| **Q-69** | Niveaux du cycle technique ? | CAP 1re et 2e année, BT 1re à 4e année, désactivables ; durées à confirmer sur les captures | 6 |
| **Q-70** | Matricule national en CAP et BT ? | Facultatif | 6 |
| **Q-71** | Périodes du cycle technique ? | Deux semestres par défaut ; trois ou deux trimestres au choix, niveau par niveau | 6 |
| **Q-72** | Pratique et stage dans la moyenne ? | Des matières avec leur coefficient, sans mécanisme nouveau | 6 |

## 15. Essais de bout en bout et captures

- la direction règle ses fonctions, enregistre une secrétaire, un gardien et une cuisinière ;
- un enseignant de la phase 4 retrouvé dans le registre, avec ses affectations ;
- un fonctionnaire engagé en vacation (taux de l'heure, 6 heures par semaine), puis son état des heures d'octobre, saisi, commenté et visé ;
- un engagement à durée déterminée qui finit bientôt, signalé au tableau de bord ; renouvellement ;
- une permission demandée, accordée ; une absence refusée pendant ce congé ;
- la secrétaire ouvre une fiche : aucun montant ; le comptable : les montants ;
- une classe « CAP 1 Électricité » en semestres : programme, inscriptions, notes, bulletins publiés du 1er semestre.

## 16. Tests de la phase

| Cible | Nature |
|---|---|
| Transitions | Engagement, état des heures, congé : tables exhaustives |
| Chevauchement | Deux engagements en cours d'un membre refusés ; succession à une date acceptée |
| Calcul | Heures prévues ; jours d'un congé (avec et sans samedi) ; solde quand le décompte est actif |
| Droits | Montants absents sans `contracts:read`, du service aux types ; fiche personnelle lisible sans droit ; refus hors droits |
| Mise à niveau | Enseignants existants devenus membres, affectations intactes, base de la phase 4 rouverte |
| Audit | Avant et après sur chaque écriture, montants compris |
| Technique | Calendrier en semestres ; bulletins semestriels ; non-régression des cycles existants |
| Bout en bout | Scénarios du §15, captures |

## 17. Ordre de réalisation

1. Registre du personnel : catégories, fonctions, fiches, dossier et pièces ; rattachement des enseignants ; écran « Personnel »
2. Engagements : permanents et vacation, succession, renouvellement, terme et alertes ; lettre d'engagement
3. État mensuel des heures des vacataires ; grille au clavier ; état imprimable
4. Absences et retards du personnel ; congés et permissions ; décompte facultatif
5. Droits et rôles en version 7, fiche personnelle, montants retirés ; tableau de bord « Personnel »
6. Enseignement technique et professionnel : cycle, niveaux CAP et BT, filières, semestres ou trimestres, règles, matières
7. CAP et BT de bout en bout : inscriptions, notes, bulletins semestriels
8. Navigation, captures, école d'essai, validation

## 18. Journal des révisions

| Version | Date | Modification |
|---|---|---|
| v1.0 | 2026-10-14 | Création après les réponses du porteur du projet du 14 octobre 2026 : souplesse des catégories et fonctions (T-39), pas de service d'origine (T-40), recommandations retenues (T-41 à T-47), CAP et BT inclus avec semestres ou trimestres (T-48) ; spécification engagée |
