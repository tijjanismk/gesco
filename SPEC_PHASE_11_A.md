# SPEC_PHASE_11_A.md — Plusieurs postes sur le réseau de l'école

**Projet** : ERP de gestion scolaire — Tauri 2 + Rust + React/TypeScript
**Phase** : 11, lot A — un poste principal et des postes reliés sur le réseau local de l'école ; version 2.0.0
**Statut** : v1.5 — acceptée par le porteur du projet le 10 novembre 2026, avec les trois choix du §12 et le nombre de postes porté par la licence (§4.4) ; étape 1 validée le 11 novembre 2026, étape 2 validée le 11 novembre 2026, étape 3 réalisée le 11 novembre 2026 et présentée pour validation ; étape 4, première partie (même fiche, écrans qui se relisent, opérations qui demandent d'être seul) réalisée le 11 novembre 2026, les lenteurs et la mesure restant à faire
**Documents parents** : `docs/plan-phase-11.md` (T-80 à T-86, répondues le 10 novembre 2026), les quatre ADR du 10 novembre 2026 dans `docs/adr/` (serveur du réseau local, poste relié, sécurité du réseau local, modification simultanée), `PROJECT_BLUEPRINT.md` (ADR-014, §15.1, §17.4, §26.2), `docs/adr/2026-10-04-frontiere-commande-unique.md`, `SPEC_PHASE_4.md` (comptes des enseignants, portée des notes), `docs/optimisations-2026-10-31.md`
**Date** : 10 novembre 2026

---

## 0. Objet et critère de fin

### 0.1 Ce que produit ce lot

Une école qui a plusieurs ordinateurs travaille sur les mêmes données en même temps : un **poste
principal** garde la base, les autres, les **postes reliés**, s'y branchent par le réseau de
l'école, sans Internet. Une école qui n'a qu'un ordinateur ne voit rien changer.

À la fin du lot :
- la direction active le réseau sur le poste principal et relie un poste en recopiant un code ;
- la secrétaire inscrit, le caissier encaisse et imprime ses reçus, le surveillant relève les
  absences, chacun sur son poste, en même temps ;
- ce qui passe sur le réseau est chiffré ; un poste perdu ou volé se coupe en un clic et ne
  contient aucune donnée de l'école ;
- le journal dit de quel poste vient chaque écriture ;
- deux personnes qui modifient la même fiche ne s'écrasent plus en silence ;
- quand le poste principal est éteint, les postes reliés le disent clairement et reprennent seuls
  à son retour ;
- l'école qui le veut ouvre la saisie des notes aux maîtres, chacun dans ses classes.

### 0.2 Décisions déjà prises qui s'appliquent

- **Ordre et version** (T-80) : ce lot d'abord ; version 2.0.0, branche `developpement`, après
  l'étiquette de la 1.2.7 et la mise à jour de `developpement`.
- **Dix postes** (T-81) : le lot se construit et se mesure pour dix postes en même temps.
- **Maîtres** (T-82) : ils pourront saisir leurs notes sur un poste de l'école, dans leurs classes
  et leurs matières ; possibilité **bloquée à la livraison**. Pas de téléphone personnel.
- **Poste principal éteint** (T-83) : les autres attendent. Aucun poste ne travaille seul pour se
  réconcilier ensuite.
- **Serveur** : axum, dans une crate `gesco-reseau` (ADR du 10 novembre 2026).
- **Poste relié** : le même GESCO installé, dont le binaire relaie les commandes ; l'écran garde
  son transport (ADR du 10 novembre 2026).
- **Sécurité** : certificat fabriqué par le poste principal, empreinte retenue à l'appairage,
  secret propre à chaque poste (ADR du 10 novembre 2026).
- **Même fiche** : refus si elle a changé depuis sa lecture (ADR du 10 novembre 2026).
- **Licence et postes** : deux formules, postes compris ou vendus par poste, portées par les
  modules de la licence ; un poste relié à l'essai quand la licence ne dit rien (ADR du
  10 novembre 2026).
- R-21 : la garde reste dans `Application::executer`, seul point de passage. R-14 : rien ne se
  supprime ; un poste se révoque.

### 0.3 Critère de fin, vérifiable

1. Chaque commande du contrat donne par le réseau le même résultat et le même refus que par la
   commande locale ; un essai parcourt tout le contrat par les deux chemins.
2. Une écriture faite depuis un poste relié porte le nom de ce poste dans le journal d'audit et
   dans les événements, jamais celui du poste principal.
3. Un poste qui n'a pas été relié, ou qui a été révoqué, est refusé ; ses sessions sont fermées.
4. Rien de lisible ne circule sur le réseau ; un poste relié refuse un poste principal dont
   l'empreinte n'est pas celle qu'il a retenue.
5. Un code d'appairage ne sert qu'une fois, expire, et s'annule après plusieurs essais faux.
6. Une session ouverte sur un poste ne vaut pas depuis un autre.
7. Un poste relié d'une autre version que le poste principal est refusé avec un message clair.
8. Depuis un poste relié : inscrire un élève, encaisser un versement, imprimer un reçu, exporter
   une liste en Excel, importer un fichier d'élèves.
9. Les commandes qui dépendent du disque du poste principal (§3.4) sont refusées depuis un poste
   relié, et leurs écrans n'y paraissent pas.
10. Poste principal éteint : chaque poste relié l'affiche, ne perd pas la saisie en cours à
    l'écran, et reprend sans intervention quand il revient.
11. Deux modifications croisées de la même fiche : la seconde est refusée, nomme la personne et
    l'heure, garde la saisie.
12. Dix postes simulés sur l'école d'essai de trois ans : les temps de réponse sont mesurés et
    tiennent la cible fixée à l'étape 4.
13. Saisie des maîtres éteinte : un compte de maître n'entre pas. Allumée : il ne lit ni n'écrit
    hors de ses classes, commande par commande.
14. Sans mention dans la licence, un seul poste se relie ; avec `postes:N`, N postes ; avec
    `postes:illimites`, autant qu'on veut. Au-delà, « Relier un poste » est refusé avec le nombre
    couvert ; un poste déjà relié n'est jamais coupé, même licence échue ou retirée.
15. Une école en 1.2 installe la 2.0 par-dessus : poste seul, réseau éteint, rien ne change ; les
    essais des phases précédentes passent sans changement.

## 1. Périmètre

**Inclus** : rôle du poste ; serveur du poste principal ; appairage, révocation, journal des
connexions ; relais du poste relié ; fichiers échangés par leur contenu ; commandes réservées au
poste principal ; coupure et reprise ; contrôle des versions ; refus des modifications croisées ;
opérations qui demandent d'être seul ; lenteurs qui gênent à plusieurs ; réglage de la saisie par
les maîtres ; écran « Postes de l'école » ; installeur, pare-feu, guide, manuel.

**Exclu** :
- Navigateur ou téléphone comme poste (ADR « poste relié ») ; saisie depuis le téléphone des
  maîtres.
- Recherche automatique du poste principal sur le réseau : l'adresse ou le nom se saisit une fois.
- Poste relié qui travaille sans le poste principal (T-83).
- Message poussé par le poste principal vers les écrans : les écrans se relisent (§6.2).
- Plusieurs établissements sur un même poste principal ; service en ligne ; portail (lots C et D).
- Les prix des deux formules : ils sont l'affaire du porteur et n'entrent pas dans le logiciel.

## 2. Rôle du poste

Un poste est **seul** (comme aujourd'hui), **principal** ou **relié**. Le rôle vit dans un réglage
du poste, à côté de la base et hors d'elle, parce qu'un poste relié n'a pas de base.

- **Seul → principal** : Paramètres, section « Postes de l'école », « Activer le réseau ». Les
  données ne bougent pas. **Principal → seul** : « Éteindre le réseau » ; les postes reliés sont
  refusés, leurs liaisons gardées pour une réactivation.
- **Relié** : au premier lancement, l'écran propose trois chemins au lieu de deux : créer
  l'établissement, restaurer une sauvegarde, **rejoindre le poste principal de l'école**.
  « Quitter le poste principal » efface la liaison et ramène au premier lancement.
- Un poste qui a déjà un établissement ne devient pas relié : il faudrait abandonner sa base, ce
  que GESCO ne fait pas d'un clic.

## 3. Serveur du poste principal

### 3.1 Routes

Dans `gesco-reseau`, démarré par `gesco-app` quand le poste est principal, le réseau activé et la
base ouverte.

- `POST /api/commandes/{nom}` : même entrée, même sortie, mêmes clés d'erreur que la commande
  locale.
- `GET /api/etat` : la version de GESCO, et rien d'autre tant que le poste n'est pas reconnu.
- `POST /api/appairage` : échange du code contre le secret du poste (§4.2).

Port proposé : **47800**, réglable. Corps borné à **32 Mo** (photos, pièces, fichiers d'import).
Le poste relié attend une réponse jusqu'à cinq minutes : un lot de bulletins prend du temps.

### 3.2 Origine d'un appel

`Application::executer` reçoit, en plus du nom, du jeton et de l'entrée, **l'origine** : le poste
local, ou le poste relié reconnu par son secret. L'origine donne le nom de poste écrit dans
l'audit et les événements (R-13), et la session la retient : la garde refuse un jeton présenté
par un autre poste que celui qui a ouvert la session.

### 3.3 Fichiers

Une vingtaine de commandes reçoivent un chemin de fichier : exports Excel, imports (élèves,
personnel, programme, classes et matières, notes, relevé), pièces des dépenses et du dossier du
personnel, livres. Sur un poste relié, ce chemin est celui du poste relié, pas du poste principal.

- Le catalogue des commandes déclare, pour chacune, si elle **lit** ou **écrit** un fichier, et
  dans quel champ.
- Commande qui lit : le binaire du poste relié lit le fichier choisi et envoie son contenu ; le
  poste principal le dépose dans un dossier temporaire de son espace de données, passe ce chemin
  au gestionnaire, puis l'efface.
- Commande qui écrit : le gestionnaire écrit dans le dossier temporaire ; le poste principal
  renvoie le contenu ; le binaire du poste relié l'écrit au chemin choisi.
- Les gestionnaires et les écrans ne changent pas. Un essai vérifie qu'aucune entrée du contrat
  n'a de champ de chemin sans cette déclaration.

### 3.4 Commandes réservées au poste principal

Le catalogue déclare aussi le **lieu** d'une commande. Réservées au poste principal : premier
démarrage ; douze mots, déverrouillage, certificat de la phrase ; sauvegarde, destinations, test
de restauration, restauration ; vérification d'une archive du journal ; licence ; clé modem (ports,
essai, crédit) ; gestion des postes (§4) ; mise à niveau. Depuis un poste relié, elles sont
refusées, et la session courante dit à l'écran le rôle du poste pour qu'il ne les montre pas.

Les tâches de fond (file des SMS, sauvegardes planifiées, relances) ne tournent que sur le poste
principal, puisqu'il est le seul à avoir la base.

## 4. Relier un poste en sûreté

### 4.1 Ce que garde le poste principal

Migration 0050, dans la base chiffrée :

- **Réseau local** — une ligne par établissement : réseau actif, port, certificat et sa clé,
  date de fabrication.
- **Poste relié** — nom donné au poste (unique parmi les postes en service), empreinte de son
  secret, relié le et par qui, dernière connexion, révoqué le et par qui.

Le certificat est fabriqué à la première activation, pour dix ans : la confiance vient de
l'empreinte retenue, pas de la date. Il voyage avec les sauvegardes.

### 4.2 Appairage

1. Sur le poste principal, « Relier un poste » affiche un **code de huit caractères** (sans O, 0,
   I ni 1), valable **dix minutes**, **une seule fois**, annulé après **cinq essais faux**, et
   l'**empreinte courte** du poste principal (quatre groupes de quatre caractères), avec son nom
   d'ordinateur et son adresse.
2. Sur le poste à relier : adresse ou nom du poste principal, nom à donner au poste (le nom de
   l'ordinateur, proposé), code. L'écran montre l'empreinte courte reçue ; on la compare à celle
   du poste principal avant de valider.
3. Le poste principal rend un secret tiré au hasard, que le poste relié range dans le trousseau
   de Windows avec l'adresse et l'empreinte complète. Le poste principal n'en garde que
   l'empreinte.

### 4.3 Ensuite

- Chaque appel porte le secret du poste, puis le jeton de la session.
- **Révoquer** un poste le refuse à l'appel suivant et ferme ses sessions.
- **Renouveler la clé du réseau** refait le certificat : chaque poste doit être relié de nouveau.
- Le poste principal n'accepte que des adresses de réseau privé. Les refus sont comptés par
  adresse : après **dix refus en une minute** (code faux, poste non reconnu), l'adresse n'est
  plus écoutée jusqu'à la fin de la minute (`reseau.trop_d_essais`).
- Au journal d'audit : réseau activé ou éteint, poste relié, poste révoqué, clé renouvelée,
  appairage refusé (code faux, expiré, annulé). Événements `ReseauActive`, `ReseauDesactive`,
  `PosteRelie`, `PosteRevoque`, `CleReseauRenouvelee`.
- Au journal technique du poste principal, et non au journal d'audit : les appels d'un appareil
  non reconnu (secret faux, poste révoqué, aucun secret), avec son adresse. Un appareil inconnu
  n'a pas d'identité à inscrire, et il ne doit pas pouvoir remplir à volonté un journal chaîné
  qui ne s'efface jamais.

### 4.4 Nombre de postes et licence

La licence porte le nombre de postes reliés parmi ses modules, sans changer de format :
`postes:illimites` (formule « postes compris ») ou `postes:N` (formule par poste, N postes reliés
en plus du poste principal). Sans mention — aucune licence, licence libre, licence déjà émise —
**un poste relié**, à l'essai.

- « Relier un poste » est refusé quand le nombre de postes en service atteint ce que la licence
  couvre ; le message dit ce nombre. Un poste révoqué rend sa place.
- Rien d'autre n'est refusé : un poste déjà relié continue de marcher si la licence est échue, à
  réémettre ou retirée.
- Pour ajouter des postes, l'éditeur réémet la licence avec un nombre plus grand ; l'outil
  `gesco-licence` vérifie la forme de la mention.
- L'écran « Postes de l'école » montre le nombre de postes couverts et le nombre en service.

## 5. Le poste relié

- **Relais** : le binaire reçoit la commande de l'écran comme aujourd'hui et la transmet au poste
  principal par le canal chiffré, en vérifiant l'empreinte. Il ne garde ni base ni clé de données.
- **Version** : à la connexion, version différente de celle du poste principal : refus, avec la
  version attendue et la marche à suivre.
- **Coupure** : une commande qui n'atteint pas le poste principal rend l'erreur « poste principal
  injoignable » ; un bandeau reste affiché, le poste réessaie de lui-même, la saisie en cours
  reste à l'écran. Aucune écriture n'est mise de côté pour plus tard.
- **Sessions** : elles vivent dans la mémoire du poste principal. S'il redémarre, chacun se
  reconnecte ; le verrouillage et le code PIN marchent comme sur un poste seul.
- **Journal technique** : chaque poste relié tient le sien, pour les pannes de réseau.
- **Fermeture du poste principal** : s'il reste des postes connectés, il le dit et demande
  confirmation avant de fermer.

## 6. Travailler à plusieurs

### 6.1 Même fiche

Les commandes qui modifient une fiche en bloc reçoivent l'instant de dernière modification lu
avec elle, et ne s'appliquent que s'il n'a pas bougé ; sinon l'erreur « modifiée entre-temps »
nomme la personne et l'heure. Leur liste s'arrête à l'étape 4 par un essai qui parcourt le
contrat. Les grilles (notes, absences, appréciations) gardent la dernière écriture par case ; les
écritures d'argent et les changements d'état ne changent pas.

Liste arrêtée à l'étape 4 (`COMMANDES_DE_FICHE`) : la fiche d'un élève, un responsable vu depuis
son élève, la fiche d'un membre du personnel, une classe, une grille de tarifs en brouillon, la
fiche de l'établissement. L'instant d'une fiche est le plus récent des lignes qui la composent
(l'élève et son identité ; le responsable, son identité et son lien ; le membre et son identité).
Chaque autre commande dont le nom dit qu'elle modifie est classée avec sa raison
(`MODIFICATIONS_SANS_CONTROLE`) : grille de saisie, valeur unique, changement d'état, écriture
d'argent, donnée de référence. Six y sont notées « fiche à protéger », hors de la liste à ce
jour : le compte d'un utilisateur, un rôle, une année scolaire, une inscription, un enseignant,
un fournisseur.

À l'écran, le refus dit qui a modifié la fiche et à quelle heure ; la saisie reste, et « Relire
la fiche » remplace le formulaire par la fiche relue, où la correction se refait.

### 6.2 Écrans qui se relisent

Les listes et les fiches se relisent à l'ouverture, au retour sur l'écran et après un refus. Les
données de référence (classes, matières, réglages) gardent un délai court. Rien n'est poussé par
le poste principal.

Réalisé à l'étape 4 : une liste ou une fiche qui n'est plus à l'écran n'est pas gardée en
mémoire, l'écran la relit avant de l'afficher. Les données de référence (niveaux et séries,
classes, années, paramètres, référentiel du personnel) ne sont pas redemandées pendant trente
secondes, sauf après une écriture de ce poste ou un refus. Revenir à la fenêtre de GESCO depuis
un autre programme ne relit rien : un formulaire en cours de saisie ne doit pas changer sous les
doigts.

### 6.3 Opérations qui demandent d'être seul

Clôture de l'année, restauration, imports, mise à niveau à l'ouverture d'une nouvelle version : le
poste principal montre les postes connectés, demande confirmation, puis refuse les commandes des
autres postes le temps de l'opération, avec un message qui la nomme. Une restauration ferme toutes
les sessions.

Réalisé à l'étape 4 (`COMMANDES_SEULES`) : clôture de l'année, restauration, imports des élèves,
du personnel, du programme et du référentiel. Le temps de la commande, toute commande d'un autre
poste qui demande une session est refusée ; le poste qui a lancé l'opération reste servi. Un
import lancé d'un poste relié écarte de même les autres postes, poste principal compris, sans
confirmation préalable (ce poste ne connaît pas les autres). Les notes d'une évaluation et le
relevé d'un compte s'importent comme une saisie ordinaire. La mise à niveau se fait à
l'ouverture, avant que le réseau ne serve.

### 6.4 Lenteurs à corriger avant de mesurer

Relevées le 31 octobre 2026 : vérification du mot de passe pendant que le verrou d'écriture est
tenu (P-19) ; envois par la clé modem qui bloquent (P-21) ; sauvegarde à la fermeture sans
changement (P-2) ; permissions relues à chaque commande (C-8) ; fiches de toute l'école chargées à
chaque note (P-4).

### 6.5 Mesure

Dix postes simulés, sur l'école d'essai de trois ans, jouent ensemble inscriptions, versements,
notes et lectures. Cible proposée, à confirmer après une première mesure : une commande courante
répond en moins d'une seconde dans 95 cas sur 100.

## 7. Saisie des notes par les maîtres

Ce qui existe depuis la phase 4 : le rôle Enseignant livré, le compte lié à un enseignant, la
saisie limitée à ses classes et ses matières, le relevé des absences de ses classes. Ce que ce lot
ajoute :

- Un réglage de l'école, **« Les maîtres saisissent leurs notes »**, **éteint à la livraison**.
  Éteint, un compte qui n'a que le rôle Enseignant n'ouvre pas de session, avec un message qui
  renvoie à la direction ; la direction saisit tout, comme aujourd'hui.
- Une école déjà installée où un compte de maître a déjà saisi garde la possibilité allumée à la
  mise à niveau.
- **Lecture** : aujourd'hui un compte de maître peut lire les fiches de tous les élèves. Allumé,
  il ne lit que les élèves de ses classes, sans leur situation financière.
- Un essai joue chaque commande du contrat avec un compte de maître : la liste de ce qu'il peut
  faire est écrite, tout le reste est refusé.

## 8. Droits

Nouveau : `workstations:manage` (activer le réseau, relier, révoquer, renouveler la clé), donné au
Directeur et à l'Administrateur. Rôles livrés en version 11 ; les bases installées les reçoivent à
leur ouverture ; matrice des permissions régénérée.

## 9. Écrans

- **Paramètres → « Postes de l'école »** (poste principal) : activer ou éteindre le réseau ;
  le nom, l'adresse, le port et l'empreinte à donner aux postes ; les postes couverts par la
  licence et en service ; « Relier un poste » et son code ; la liste des postes (nom, relié le,
  dernière connexion, session ouverte ou non) ; révoquer ; renouveler la clé du réseau. Éteindre,
  révoquer et renouveler demandent une confirmation. L'écran se relit toutes les dix secondes.
- **Premier lancement** : « Rejoindre le poste principal de l'école ».
- **Bandeau** d'un poste relié : nom du poste, état de la liaison.
- **Poste principal** : avertissement avant de fermer ou de lancer une opération qui demande
  d'être seul.

Aucun texte affiché ne cite une source ; tous passent par le catalogue.

## 10. Livraison

Un seul installeur. Règle du pare-feu de Windows pour le port, sur les réseaux privés seulement,
posée à l'activation du réseau. Mise à jour : le poste principal d'abord, puis chaque poste relié.
Guide d'installation à plusieurs postes, avec le conseil d'un portable comme poste principal et
d'un petit routeur ; manuel ; notes de version. Liste d'essais à la main sur deux vraies machines.

## 11. Ordre de réalisation

1. Le noyau répond par une route, sur la machine elle-même : crate `gesco-reseau`, origine de
   l'appel, lieu et fichiers au catalogue, essai du contrat par les deux chemins.
2. Relier un poste en sûreté : certificat, appairage, secret du poste, révocation, journal, droit
   nouveau, migration 0050, nombre de postes de la licence, écran « Postes de l'école ».
3. Le poste relié : rôle du poste, premier lancement, relais, fichiers, versions, coupure et
   reprise ; essais de bout en bout sur deux instances.
4. Travailler à plusieurs : même fiche, écrans qui se relisent, opérations seules, lenteurs,
   mesure à dix postes.
5. Saisie des notes par les maîtres : réglage, lecture limitée, essai commande par commande.
6. Livraison : pare-feu, installeur, guide, manuel, captures, école d'essai, essai sur deux
   machines, validation.

Chaque étape se termine comme les précédentes : essais verts, captures montrées, accord du
porteur. Une seule compilation complète par étape, `target` élagué avant.

## 12. Ce qu'il faut du porteur du projet

- Pour l'étape 6 : **deux ordinateurs sous Windows** sur le même réseau (le Wi-Fi d'une box
  suffit), pour l'essai réel.
- Avant de vendre la 2.0 : les **prix** des deux formules (postes compris, ou par poste). La
  décision de proposer les deux est prise (§4.4).
- ~~L'accord sur les choix proposés ici et pas encore discutés~~ : acceptés le 10 novembre 2026 —
  le blocage des comptes de maîtres tant que le réglage est éteint (§7), la mise à jour dans
  l'ordre poste principal puis postes reliés avec versions égales (§5), le port 47800 (§3.1).

## 13. Journal des révisions

| Version | Date | Modification |
|---|---|---|
| v1.0 | 2026-11-10 | Création après les réponses du porteur du projet à T-80 à T-86 et son accord sur les quatre ADR du 10 novembre 2026 |
| v1.1 | 2026-11-10 | Spécification acceptée par le porteur du projet, avec le blocage des comptes de maîtres, les versions égales et le port 47800 |
| v1.2 | 2026-11-10 | Décision du porteur du projet sur le prix : les deux formules, et un poste relié à l'essai sans mention dans la licence ; §4.4 et critère 14 ajoutés, la limite par la licence quitte les exclusions |
| v1.3 | 2026-11-11 | Étape 2 réalisée. Événement `ReseauEteint` renommé `ReseauDesactive` (règle des noms d'événements). §4.3 : dix refus par minute et par adresse ; les appels d'un appareil non reconnu vont au journal technique, pas au journal d'audit (écart à la v1.2, à confirmer par le porteur du projet). §9 : contenu de l'écran « Postes de l'école » |
| v1.4 | 2026-11-11 | Étape 3 réalisée. §2 : le réglage du poste ne dit que « relié » ; entre seul et principal, c'est la base qui tranche (réseau allumé ou non), pour qu'une sauvegarde restaurée garde son rôle — écart à confirmer par le porteur du projet. §3.3 : le contenu d'un fichier voyage en base64 dans l'entrée (commande qui lit) ou avec la sortie (commande qui écrit), vingt mégaoctets au plus. §5 : le poste relié dit sa version à chaque appel et le poste principal refuse une version différente avec la version attendue ; « Quitter le poste principal » est offert sans session. §9 : l'avertissement avant une opération qui demande d'être seul passe à l'étape 4 |
| v1.5 | 2026-11-11 | Étape 4, première partie. §6.1 : liste des six fiches protégées, classement des autres commandes de modification, six fiches notées « à protéger ». §6.2 : relecture à l'ouverture, trente secondes pour les données de référence, pas de relecture au retour à la fenêtre. §6.3 : opérations retenues, imports limités à ceux qui touchent toute l'école, import lancé d'un poste relié — choix à confirmer par le porteur du projet. §6.4 et §6.5 (lenteurs, mesure) restent à faire |
