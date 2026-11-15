# SPEC_SAUVEGARDE_HORS_ECOLE.md — Sauvegarde hors de l'école

**Projet** : ERP de gestion scolaire — Tauri 2 + Rust + React/TypeScript
**Lot** : sauvegarde hors de l'école, par dossier synchronisé, sans abonnement ; version 2.0.0
**Statut** : v1.0 — proposée le 15 novembre 2026 ; T-130 à T-133 attendent le porteur du projet (§12)
**Documents parents** : `docs/plan-phase-11.md` (§2.4, lot C, étape C1 ; §7, demande du 11 novembre
2026 ; §7.4, T-95 à T-97), `docs/orchestration.md` (décisions des 14 et 15 novembre 2026),
`SPEC_PHASE_0_5.md` (§11, sauvegarde), `SPEC_PHASE_11_A.md` (§3.4, commandes réservées au poste
principal), ADR du 4 octobre 2026 (archive de sauvegarde), ADR du 10 octobre 2026 (chiffrement de
la base), ADR du 15 novembre 2026 (copie hors de l'école)
**Date** : 15 novembre 2026

**Numéros des questions** : T-130 et suivants. T-116 à T-129 restent libres pour la phase 12, dont
la spécification s'écrit en même temps.

---

## 0. Objet et critère de fin

### 0.1 Ce que produit ce lot

Aujourd'hui, une école qui perd en même temps son ordinateur et sa clé USB (vol, incendie,
inondation) perd toutes ses données. Ce lot met une copie de la sauvegarde hors de l'école, sur un
compte en ligne qui appartient à l'école.

À la fin du lot :
- l'écran **Sauvegardes** a une section **Copie hors de l'école** ;
- GESCO **propose les dossiers Google Drive, OneDrive et Dropbox** qu'il trouve sur le poste ;
  l'école peut aussi choisir n'importe quel autre dossier ;
- l'école choisit le **rythme** : à la demande, chaque semaine ou chaque mois ; jamais chaque jour ;
- un bouton **Envoyer une copie maintenant** dépose tout de suite une copie, et dit avant combien
  de mégaoctets partiront par Internet ;
- **n'y part que l'archive `.gsv`, déjà chiffrée**. Jamais la base en clair, jamais la clé, jamais
  les douze mots ;
- GESCO **vérifie, sans bloquer, que la dernière copie est bien arrivée** dans le dossier, et dit,
  quand Windows le sait, si le logiciel du fournisseur l'a envoyée ;
- le **guide d'installation** et le **manuel** expliquent comment installer le logiciel du
  fournisseur, choisir le dossier, vérifier l'envoi et restaurer depuis le fichier téléchargé ;
- une école qui ne s'en sert pas ne voit rien changer, et rien ne le lui fait remarquer.

### 0.2 Décisions déjà prises qui s'appliquent

- **T-96** (porteur du projet, 14 novembre 2026) : un dossier synchronisé (Google Drive, OneDrive,
  Dropbox) sur le compte de l'école. N'y part que la sauvegarde déjà chiffrée, jamais la base en
  clair ; la clé ne part jamais avec. L'application vérifie, sans bloquer, que la dernière copie est
  bien arrivée.
- **T-95** (14 novembre 2026) : tout « en ligne » entre dans la 2.0, y compris le lot C (sauvegarde
  sur un serveur de l'éditeur, envoi des données, serveur) et le lot D (portail des parents). Ce lot
  ne couvre que la voie par dossier synchronisé, sans abonnement. Le §10 dit comment l'étape C1 s'y
  ajoutera.
- **T-97** (11 novembre 2026) : dans le même installeur que le réseau local et l'e-learning,
  version 2.0.0, branche `developpement`.
- **Rythme** (11 novembre 2026) : « pas de sauvegarde automatique, ou il choisit hebdomadaire ou
  mensuel ». La copie hors de l'école ne part donc jamais chaque jour. Les sauvegardes du poste et
  de la clé gardent leur rythme.
- **La flexibilité sans incohérence** (14 novembre 2026) : ne jamais bloquer, ne pas faire
  remarquer ce qui manque, chiffres et documents toujours justes.
- **Interface sans sources** : aucun texte affiché ne cite un numéro de question, de spécification
  ou une date de décision.
- **Lot A** (`SPEC_PHASE_11_A.md` §3.4) : sauvegarde, destinations, test et restauration sont
  réservés au poste principal ; les tâches de fond n'y tournent que là.
- **Phase 12** (12 novembre 2026) : les vidéos ajoutées par l'école ne font jamais partie de la
  sauvegarde. Elles ne partent donc pas non plus dans la copie hors de l'école.
- **Méthode de travail** (14 novembre 2026) : par étape, fmt, clippy, tsc, ESLint, commit et push ;
  les essais écrits avec le code sont lancés tous ensemble à la fin, avec l'essai Playwright global.

### 0.3 Critère de fin, vérifiable

1. Sur un poste où OneDrive est installé, la section propose le dossier OneDrive. L'école le
   choisit, appuie sur « Envoyer une copie maintenant » : un fichier `.gsv` apparaît dans le
   sous-dossier `GESCO` de OneDrive, avec la même empreinte que l'archive du poste.
2. Après trente jours simulés au rythme « chaque semaine », le dossier contient quatre ou cinq
   copies, et seulement des fichiers `.gsv`. Au rythme « à la demande », il n'en contient aucune
   sans un geste de l'école.
3. Les sauvegardes faites à la fermeture, à l'intervalle réglé, avant un import, une facturation,
   une migration, une restauration, l'ouverture ou la clôture d'une année ne vont pas dans le
   dossier hors de l'école. Elles continuent d'aller dans les autres destinations.
4. Un essai cherche dans le dossier hors de l'école l'en-tête d'une base SQLite, les octets de la
   clé de l'école et les douze mots : il n'en trouve aucun.
5. Dossier absent (logiciel désinstallé, lecteur `G:` disparu, disque plein) : la sauvegarde du
   poste et celle de la clé réussissent, la fermeture n'attend pas, aucune fenêtre d'erreur ne
   s'ouvre. L'historique dit « n'a pas pu être déposée ».
6. Une copie effacée du dossier après son dépôt : la vérification suivante dit « n'est plus dans le
   dossier ». Une copie que le fournisseur a libérée du disque (disponible en ligne seulement) n'est
   jamais téléchargée par une vérification.
7. Sur un vrai compte OneDrive, la dernière copie passe de « en attente d'envoi » à « envoyée »
   (essai à la main). Avec un fournisseur dont Windows ne connaît pas l'état, elle reste
   « déposée dans le dossier », jamais « envoyée ».
8. La copie téléchargée depuis le site du fournisseur se restaure avec les douze mots dans un
   dossier de données neuf, et la base restaurée a la même empreinte de contenu que celle du poste.
   Le porteur n'a qu'un ordinateur : l'essai se fait sur la même machine, dans un dossier de données
   neuf ; l'essai sur une autre machine est écrit dans les essais à la main.
9. Le nettoyage du dossier hors de l'école ne supprime que des copies que GESCO y a lui-même
   déposées pour cette école. Un autre fichier, ou la copie d'une autre école dans le même dossier,
   n'est jamais touché.
10. Depuis un poste relié, la section n'apparaît pas et ses commandes sont refusées.
11. Une école en 1.2 qui avait ajouté à la main un dossier Google Drive comme destination ordinaire
    passe en 2.0 sans que rien change : ce dossier reçoit toujours les mêmes sauvegardes.
12. Les essais des phases précédentes passent sans changement ; les destinations et la rotation
    existantes ne changent pas.

## 1. Périmètre

**Inclus** : une nouvelle sorte de destination, « dossier synchronisé » ; détection des dossiers
Google Drive, OneDrive et Dropbox ; rythme propre ; envoi à la demande avec la taille ; nettoyage
propre au dossier ; vérification en trois niveaux et état d'envoi ; signal en cas d'échec (selon
T-130) ; écran ; manuel, guide d'installation, essais à la main ; notes de version.

**Exclu** :
- GESCO qui se connecte lui-même au compte de l'école (Google, Microsoft, Dropbox). Non retenu
  (T-96, ADR du 15 novembre 2026).
- Le serveur de l'éditeur, l'envoi par GESCO lui-même, l'abonnement : étape C1 du lot C (§10).
- L'envoi d'une partie seulement de l'archive (envoi différentiel). Chaque copie est une archive
  entière ; elle est petite (environ 4 Mo pour l'école d'essai le 15 novembre 2026).
- Un second chiffrement : l'archive est déjà chiffrée par la clé de l'école.
- Installer ou régler le logiciel du fournisseur : c'est le guide qui l'explique.
- Créer le compte de l'école chez le fournisseur : le guide le conseille, GESCO ne le vérifie pas.
- Les vidéos de la phase 12 : jamais dans la sauvegarde, donc jamais dans la copie.

## 2. Ce que l'application possède déjà

Relevé fait dans le code le 15 novembre 2026.

| Élément | Où | Ce qu'il fait |
|---|---|---|
| Archive `.gsv` | `gesco-services/src/archive.rs` | Base et modèles sur mesure, compressés, chiffrés par blocs d'un mébioctet avec la clé de l'école. Le manifeste reste lisible sans clé : nom de l'école, date, motif, versions, empreintes, et la clé enfermée par les douze mots (enveloppe de secours, Argon2id) |
| Destinations | `gesco-services/src/sauvegarde.rs`, paramètre `sauvegarde.destinations` | Le dossier du poste, toujours en premier, puis des dossiers ajoutés par l'école, chacun marqué « amovible » ou non |
| Copie vers chaque destination | `ServiceSauvegarde::archiver` | L'archive s'écrit dans le dossier du poste, puis se copie ailleurs par un fichier `.partiel` renommé ; l'empreinte est vérifiée ; une destination absente donne une ligne « échouée » sans arrêter les autres |
| Rotation | `appliquer_rotation`, `PolitiqueRotation` | Garde 7 jours, 4 semaines et 12 mois par dossier ; lit le manifeste de chaque archive pour ne toucher que celles de l'école |
| Déclencheurs | `MotifSauvegarde` | Manuelle, planifiée (intervalle réglé, 24 heures à la livraison), fermeture, avant migration, restauration, ouverture ou clôture d'année, import, facturation. Toutes vont aujourd'hui vers toutes les destinations |
| Historique | table `sauvegarde` | Une ligne par destination et par sauvegarde : date, chemin, taille, empreinte, versions, réussie ou échouée, dernier test |
| Avertissements | `ServiceSauvegarde::avertissements` | Bandeau : certificat non confirmé, test de restauration échoué, aucune destination amovible, mot de passe réinitialisé par la phrase |
| Restauration | `restauration.rs`, assistant | Depuis n'importe quel fichier `.gsv`, avec la clé du poste ou les douze mots, y compris au premier démarrage d'un nouvel ordinateur |
| Droits | `core:backup:create`, `core:backup:restore` | Sauvegarder, régler les destinations : administrateur et directeur. Restaurer : administrateur |
| Poste principal | `SPEC_PHASE_11_A.md` §3.4 | Sauvegarde, destinations, test et restauration refusés depuis un poste relié |

Conséquence, déjà notée au §7.1 du plan : un dossier Google Drive ajouté à la main comme
destination ordinaire marche dès la 1.2. Mais il reçoit toutes les sauvegardes, chaque jour et à
chaque fermeture, ce qui consomme le forfait ; GESCO ne sait pas si la copie est partie ; et la
rotation lit chaque archive, ce qui peut la faire télécharger.

## 3. La copie hors de l'école

### 3.1 Une sorte de destination de plus

Une destination a désormais une **sorte** parmi trois :
- **dossier** : un dossier du poste ou du réseau, comme aujourd'hui ;
- **support amovible** : clé USB, disque externe, comme aujourd'hui ;
- **dossier synchronisé** : un dossier que le logiciel d'un fournisseur envoie sur un compte en
  ligne. C'est la copie hors de l'école.

Le dossier du poste reste toujours la première destination. L'école peut avoir un ou plusieurs
dossiers synchronisés (par exemple Google Drive et OneDrive). Les réglages venus de la 1.2 se lisent
sans changement : `amovible` vrai donne « support amovible », faux donne « dossier ».

### 3.2 Quelles sauvegardes partent hors de l'école

Un dossier synchronisé ne reçoit que deux sortes de copies :
- **la copie à la demande** : le bouton « Envoyer une copie maintenant » fait une sauvegarde
  manuelle, qui va vers toutes les destinations, dossiers synchronisés compris ;
- **la copie du rythme** : quand le rythme choisi le demande (§3.3), la tâche périodique du poste
  principal fait une sauvegarde planifiée, qui va vers toutes les destinations, dossiers
  synchronisés compris.

Toutes les autres sauvegardes (fermeture, intervalle réglé, avant migration, restauration, import,
facturation, ouverture ou clôture d'année) et le bouton « Sauvegarder maintenant » ne vont **pas**
vers les dossiers synchronisés. Le bouton « Sauvegarder maintenant » garde donc son sens d'avant.

### 3.3 Le rythme

Un seul rythme pour tous les dossiers synchronisés, choisi par l'école :
- **à la demande** : rien ne part sans un geste. Réglage livré (T-131) ;
- **chaque semaine** : une copie est due quand la dernière copie bien déposée a sept jours ou plus,
  ou quand il n'y en a pas encore ;
- **chaque mois** : une copie est due quand la dernière copie bien déposée date du même jour du mois
  précédent ou d'avant (le 31 mars devient le 30 avril, puis le 31 mai).

Une copie due part au premier passage de la tâche périodique (chaque minute) tant que GESCO est
ouvert sur le poste principal. Elle ne part jamais à la fermeture : l'ordinateur s'éteint souvent
juste après, avant que le logiciel du fournisseur ait pu l'envoyer. Si GESCO reste fermé une
semaine, la copie part à la prochaine ouverture ; il n'y a pas de rattrapage des semaines passées.

Une copie à la demande compte comme une copie du rythme : elle repousse la suivante.

### 3.4 Où et sous quel nom

- Pour un dossier trouvé (§3.6), GESCO propose un sous-dossier `GESCO` à la racine du compte (par
  exemple `C:\Users\…\OneDrive\GESCO`). Il le crée s'il n'existe pas.
- Pour un dossier choisi à la main, la copie va dans ce dossier même.
- Le nom est celui des autres archives : `gesco-AAAAMMJJ-HHMMSS-XXXXXX.gsv`.
- La copie s'écrit en `.partiel`, puis prend son nom ; c'est déjà ainsi.

### 3.5 Le nettoyage du dossier

Un dossier synchronisé a sa propre politique : **les 4 dernières semaines et les 12 derniers mois**,
une copie par semaine et par mois, soit 16 copies au plus (T-132). Pour l'école d'essai, environ
70 Mo sur le compte.

Le nettoyage ne lit jamais les fichiers du dossier : il part des lignes de la table `sauvegarde`
qui disent ce que GESCO y a déposé pour cette école. Il ne supprime donc que ses propres copies, et
ne fait jamais télécharger un fichier libéré du disque. La copie la plus récente n'est jamais
supprimée. Le fournisseur garde en général les fichiers supprimés quelques jours dans sa corbeille ;
le guide le dit.

### 3.6 Trouver les dossiers des fournisseurs

GESCO cherche, sur le poste principal et pour le compte Windows qui l'utilise :
- **OneDrive** : variables d'environnement `OneDrive`, `OneDriveConsumer`, `OneDriveCommercial`, et
  les comptes déclarés dans le registre de OneDrive ;
- **Google Drive** : le lecteur de Google Drive et son dossier « Mon Drive » (ou « My Drive »), ou
  le dossier en miroir sous le profil ;
- **Dropbox** : le fichier `info.json` de Dropbox (dans `%APPDATA%\Dropbox` ou
  `%LOCALAPPDATA%\Dropbox`) ;
- **les autres** : les racines de synchronisation déclarées à Windows, avec le nom que leur donne
  le fournisseur.

Les endroits exacts se vérifient à l'étape 3 sur un vrai poste. La lecture du registre passe par
`reg.exe`, comme pour l'empreinte du poste : pas de code `unsafe`. Trouver un dossier n'est jamais
une condition : « Choisir un autre dossier » marche toujours, avec n'importe quel fournisseur.

Le dossier appartient au compte Windows. Si le poste principal sert à plusieurs comptes Windows, la
copie n'est déposée que lorsque GESCO tourne sous le compte qui a le logiciel du fournisseur ;
ailleurs, elle donne « n'a pas pu être déposée ». Le guide le dit.

## 4. Vérifier que la copie est arrivée

Selon l'ADR du 15 novembre 2026, trois niveaux, du plus sûr au moins sûr.

1. **Au dépôt** : la copie est écrite, renommée, et son empreinte est égale à celle de l'archive du
   poste. Sinon : « n'a pas pu être déposée ».
2. **Ensuite, sans rien télécharger** : présence du fichier et taille, lues dans ses métadonnées.
   Le contenu n'est jamais relu. Fichier absent ou taille différente : « n'est plus dans le
   dossier ».
3. **L'état d'envoi, quand Windows le connaît** : GESCO lit, par une commande PowerShell lancée sans
   fenêtre, la propriété que Windows tient pour les fichiers synchronisés. Elle dit si le fichier
   est à jour sur le compte, en attente, ou en erreur. Quand Windows ne dit rien, ou que la lecture
   échoue, l'état reste « déposée dans le dossier ».

**Quand** : quelques minutes après le dépôt, une heure après, puis au démarrage du poste principal
et une fois par jour, tant que la dernière copie n'est pas « envoyée ». Seule la dernière copie de
chaque dossier synchronisé se vérifie.

**États affichés** (un seul à la fois, pour chaque copie) :

| État | Sens |
|---|---|
| Envoyée | Windows dit que le fichier est à jour sur le compte |
| En attente d'envoi | Windows dit que le fichier n'est pas encore envoyé |
| Envoi en échec | Windows dit que le logiciel du fournisseur n'a pas pu l'envoyer |
| Déposée dans le dossier | La copie est intacte dans le dossier ; Windows ne dit rien de l'envoi |
| N'a pas pu être déposée | Dossier absent, plein ou refusé ; copie interrompue |
| N'est plus dans le dossier | Fichier effacé ou modifié depuis le dépôt |

Jamais « envoyée » sans que Windows l'ait dit. Rien de ces états ne bloque quoi que ce soit.

## 5. Ce qui ne part jamais

- **La base en clair** : seul le fichier `.gsv` est copié. GESCO n'écrit rien d'autre dans un
  dossier synchronisé, ni journal, ni fichier d'état, ni export.
- **La clé de l'école** : elle ne quitte pas le coffre de Windows. L'archive contient seulement la
  clé enfermée par les douze mots (enveloppe de secours), ce qui permet de restaurer sur un autre
  ordinateur. Sans les douze mots, l'archive ne se lit pas, même pour le fournisseur.
- **Les douze mots et leur certificat** : ils ne sont jamais écrits par GESCO. L'écran et le guide
  disent de ne jamais ranger le certificat, ni sa photo, dans ce dossier ou sur ce compte.
- **Ce que le fournisseur peut voir** : le nom du fichier (une date), sa taille, et le manifeste en
  clair (nom de l'école, date, versions de GESCO). Aucune donnée d'élève, de personnel ou de
  finance. Le guide le dit.

## 6. Données nouvelles

- **Destination** : un champ `sorte` (`dossier`, `amovible`, `synchronise`). L'ancien champ
  `amovible` reste lu pour les réglages de la 1.2.
- **Paramètre** `sauvegarde.hors_ecole.rythme` : `demande` (livré), `semaine`, `mois`. Validé dans
  `gesco-core` comme les autres paramètres ; sa modification est auditée.
- **Table `sauvegarde`**, par la migration suivante (numéro pris à l'étape 1) :
  - `sorte TEXT NOT NULL DEFAULT 'dossier'`, avec `CHECK (sorte IN ('dossier', 'amovible',
    'synchronise'))` ; les lignes existantes prennent `dossier` ;
  - `etat_envoi TEXT`, vide pour les autres sortes, avec `CHECK (etat_envoi IN ('deposee',
    'en_attente', 'envoyee', 'echec_envoi', 'non_deposee', 'absente'))` ;
  - `etat_verifie_le INTEGER`, instant de la dernière vérification.
  Les noms ne parlent pas de dossier : l'étape C1 ajoutera une sorte `serveur` et reprendra les mêmes
  états (§10).
- **Commandes nouvelles**, toutes réservées au poste principal :
  - `sauvegarde_dossiers_synchronises` : les dossiers trouvés sur le poste (§3.6) ;
  - `sauvegarde_hors_ecole_regler` : le rythme ;
  - `sauvegarde_hors_ecole_envoyer` : la copie à la demande ;
  - `sauvegarde_hors_ecole_etat` : la dernière copie de chaque dossier synchronisé, son état, la
    taille de la dernière sauvegarde, la date de la prochaine copie due.
  Les destinations se règlent par la commande existante `sauvegarde_destinations_definir`.

## 7. Écrans

### 7.1 Sauvegardes — section « Copie hors de l'école »

Placée entre l'historique et les destinations, sur le poste principal seulement.

- **Une explication courte**, toujours visible : « Une copie chiffrée de la sauvegarde part dans
  un dossier que Google Drive, OneDrive ou Dropbox envoie sur le compte de l'école. Sans les douze
  mots, personne ne peut la lire, pas même le fournisseur. Elle s'ajoute à la clé USB, elle ne la
  remplace pas. »
- **Dossier** : les dossiers trouvés, en choix (« OneDrive — C:\Users\…\OneDrive\GESCO ») ; « Choisir
  un autre dossier… » ; « Retirer ». Rien n'est choisi à la livraison. Sans dossier trouvé, seule
  la ligne « Choisir un autre dossier… » apparaît, sans message.
- **Rythme** : trois choix, « À la demande », « Chaque semaine », « Chaque mois », avec en dessous :
  « Chaque copie pèse environ 4,3 Mo » (taille de la dernière sauvegarde) et, pour la semaine et le
  mois, l'ordre de grandeur par mois (« environ 17 Mo par mois »).
- **Envoyer une copie maintenant** : au-dessus du bouton, « Environ 4,3 Mo partiront par
  Internet. » Pas de fenêtre de confirmation. Le bouton n'apparaît que lorsqu'un dossier est choisi.
- **Dernière copie** : date, taille, état (§4), et, au rythme semaine ou mois, la date de la
  prochaine copie.
- **Une ligne de rappel** : « Ne rangez jamais le certificat des douze mots dans ce dossier ni sur
  ce compte. »

### 7.2 Historique et destinations

- L'historique montre la sorte de chaque ligne et, pour un dossier synchronisé, son état.
- La liste des destinations montre la sorte de chacune, en trois choix (dossier, support amovible,
  dossier synchronisé), à la place de la case « amovible ». Une école peut ainsi passer un dossier
  Google Drive ajouté en 1.2 en « dossier synchronisé » ; GESCO ne le fait jamais à sa place.

### 7.3 Bandeau

Selon la recommandation de T-130 :
- rien pour une école qui n'a pas de dossier synchronisé, ni au rythme « à la demande » quand elle
  n'envoie rien ;
- une ligne dans le bandeau, seulement quand l'école a choisi un dossier et que la dernière copie
  « n'a pas pu être déposée », « n'est plus dans le dossier », est en « envoi en échec », ou reste
  « en attente d'envoi » plus de sept jours : « La dernière copie hors de l'école n'est pas arrivée
  sur le compte. » Un clic ouvre la section. La ligne disparaît dès qu'une copie suivante est
  déposée ou envoyée.

L'avertissement existant « aucune destination amovible » ne change pas : un dossier synchronisé ne
remplace pas la clé, parce qu'un rançongiciel qui chiffre le poste peut atteindre ce dossier.

### 7.4 Manuel et guide d'installation

- **Guide d'installation**, nouvelle partie « Mettre une copie hors de l'école » : créer un compte
  au nom de l'école (pas celui d'un employé qui peut partir), installer le logiciel du fournisseur,
  s'y connecter, choisir le dossier dans GESCO, envoyer une copie, vérifier sur le site du
  fournisseur que le fichier est là ; ce qu'il faut savoir du forfait (OneDrive suspend l'envoi sur
  une connexion limitée ; un partage de connexion du téléphone peut l'être) ; la corbeille du
  fournisseur ; plusieurs comptes Windows sur le poste principal.
- **Manuel 2.0**, chapitre « Sauvegarde hors de l'école » : la section, le rythme, les états, et
  restaurer depuis le fichier téléchargé (télécharger le `.gsv` depuis le site du fournisseur, puis
  l'assistant de restauration avec les douze mots, sur un ordinateur neuf ou non).
- **Essais à la main** (`docs/essais-a-la-main-2.0.md`) : sur un vrai compte, au moins OneDrive et
  Google Drive : choisir, envoyer, voir l'état passer à « envoyée » ou rester « déposée »,
  télécharger, restaurer dans un dossier de données neuf ; puis, plus tard, sur un autre ordinateur.

## 8. Règles

1. **Rien ne bloque.** Un dossier absent, plein ou refusé, une vérification qui échoue, PowerShell
   bloqué : la sauvegarde du poste et celle de la clé réussissent, la fermeture n'attend pas, aucune
   fenêtre d'erreur ne s'ouvre. Seuls l'historique, la section et, selon T-130, le bandeau le disent.
2. **Une erreur dans un dossier synchronisé ne fait jamais échouer une sauvegarde.** Ni le dépôt,
   ni le nettoyage, ni la vérification. Aujourd'hui une erreur de rotation fait échouer toute la
   sauvegarde : pour un dossier synchronisé, elle est ignorée et réessayée à la copie suivante.
3. **Ne pas faire remarquer ce qui manque.** Une école sans dossier synchronisé ne reçoit ni
   rappel, ni avertissement, ni compteur. La section reste à sa place dans l'écran Sauvegardes, sans
   message.
4. **Jamais d'état faux.** « Envoyée » seulement quand Windows le dit ; « déposée » quand on ne sait
   pas. Une copie n'est jamais déclarée bien déposée sans empreinte vérifiée.
5. **Rien d'autre que l'archive chiffrée** dans un dossier synchronisé (§5).
6. **Jamais chaque jour.** Seuls la copie à la demande et le rythme semaine ou mois écrivent dans un
   dossier synchronisé.
7. **Ne rien télécharger pour vérifier ni pour nettoyer** : métadonnées et table `sauvegarde`
   seulement.
8. **Ne toucher qu'à ses propres copies** : le nettoyage ne supprime que les fichiers que la table
   dit déposés par GESCO pour cette école.
9. **Rien ne change sans l'école** : un réglage de la 1.2 garde son comportement ; GESCO ne
   transforme jamais une destination en dossier synchronisé, et ne choisit aucun dossier à la place
   de l'école.
10. **Poste principal seulement.** Les commandes sont refusées depuis un poste relié, comme les
    autres commandes de sauvegarde.
11. **Tout passe par `Application::executer`** (R-21) ; le réglage du rythme, des destinations et
    chaque copie sont audités comme les sauvegardes d'aujourd'hui.

## 9. Droits

Aucune permission nouvelle. Les commandes suivent celles de la sauvegarde :
- choisir un dossier, régler le rythme, envoyer une copie, voir l'état : `core:backup:create`
  (administrateur et directeur à la livraison) ;
- restaurer depuis une copie téléchargée : `core:backup:restore` (administrateur), comme toute
  restauration ; au premier démarrage d'un ordinateur neuf, l'assistant existant, avec les douze
  mots.

## 10. Plus tard : le serveur de l'éditeur (lot C, étape C1)

T-95 fait entrer l'étape C1 dans la 2.0, après la phase 12. Elle n'est pas spécifiée ici. Voici
comment elle s'ajoutera à ce lot, sans le défaire :

- **Une sorte de plus** : `serveur`, à côté de `synchronise`, dans la même section « Copie hors de
  l'école ». Une école pourra avoir l'une, l'autre ou les deux.
- **Les mêmes règles** : n'y part que l'archive `.gsv` déjà chiffrée ; jamais la base, jamais la
  clé ; le même rythme (à la demande, chaque semaine, chaque mois) ; rien ne bloque ; jamais d'état
  faux.
- **Les mêmes états**, dans la même colonne `etat_envoi`. La différence : c'est GESCO qui envoie,
  avec reprise après coupure et volume affiché, et le serveur confirme l'empreinte reçue. « Envoyée »
  y sera donc toujours connu.
- **Ce que C1 apporte en propre** et qu'il faudra spécifier : compte de l'école chez l'éditeur,
  licence Connectée (échéance, grâce), envoi hors des heures de classe, reprise après coupure,
  restauration depuis le serveur, et les préalables que seul le porteur fournit (hébergeur, tarif,
  avis d'un juriste sur les données de mineurs en ligne).
- **Ce que ce lot prépare** : les noms de sorte et d'état ne parlent pas de dossier ; la copie du
  rythme et la copie à la demande passent par un seul point du service, où une destination
  `serveur` s'ajoutera ; la vérification passe par un trait, où le serveur remplacera Windows.

## 11. Ordre de réalisation

Plusieurs étapes par session, commit et push après chacune, relais au-delà d'environ 150 000
jetons. Par étape : fmt, clippy, tsc, ESLint. Les essais s'écrivent avec le code et se lancent tous
ensemble à l'étape 4. Une seule session compile à la fois.

1. **Sorte et rythme, dans le service.** Champ `sorte` des destinations et lecture des réglages de
   la 1.2 ; paramètre du rythme ; choix des destinations selon le motif (§3.2) ; copie due (§3.3,
   horloge simulée) ; nettoyage propre au dossier, depuis la table, sans lecture des fichiers ;
   erreurs du dossier synchronisé sans effet sur la sauvegarde ; migration de la table `sauvegarde` ;
   commandes `sauvegarde_hors_ecole_regler` et `sauvegarde_hors_ecole_envoyer`, réservées au poste
   principal. Essais écrits : critères 2, 3, 4, 5, 9, 11, 12.
2. **Vérification et état d'envoi.** Niveaux 2 et 3 (§4) ; trait de lecture de l'état dans
   `gesco-services`, lecture simulée pour les essais, lecture par PowerShell dans `gesco-app` ;
   calendrier des vérifications ; commande `sauvegarde_hors_ecole_etat` ; ligne du bandeau selon
   T-130. Sur le poste du porteur, relever ce que Windows dit pour OneDrive, Google Drive et
   Dropbox, et l'écrire dans l'ADR du 15 novembre 2026 (conditions de reconsidération). Essais
   écrits : critères 6 et 7 (partie simulée).
3. **Détection et écran.** Détection des dossiers (§3.6) et commande
   `sauvegarde_dossiers_synchronises` ; section « Copie hors de l'école », sorte dans la liste des
   destinations et dans l'historique ; textes en `fr-ML`, sans source citée ; section absente d'un
   poste relié. Essais écrits : critères 1 et 10 ; vitest de la section.
4. **Livraison.** Manuel, guide d'installation, essais à la main, notes de version 2.0.0,
   `ETAT_AVANCEMENT.md` ; école d'essai avec un dossier synchronisé simulé (un simple dossier) ;
   scénario Playwright de la section et captures. Puis tous les essais (suite Rust complète,
   vitest, knip, Playwright), corrections, commit. L'essai sur un vrai compte (critères 7 et 8)
   revient au porteur, sur sa machine, avec le guide.

Les étapes 2 et 3 ne dépendent que de l'étape 1 ; elles touchent des fichiers différents (service
d'un côté, écran de l'autre) et peuvent se suivre dans n'importe quel ordre.

## 12. Questions au porteur du projet

À la suite de T-97. Chacune a une recommandation ; le lot peut commencer sur les recommandations.

### T-130 — Ce que l'application dit quand la copie n'arrive pas

Quand l'école a choisi un dossier et que la dernière copie n'est pas arrivée (dossier absent,
envoi en échec, en attente depuis plus d'une semaine), faut-il une ligne dans le bandeau, ou
seulement l'état dans l'écran Sauvegardes ? Et pour une école qui n'a choisi aucun dossier ?

*Recommandation* : une ligne dans le bandeau dans ces seuls cas, qui disparaît d'elle-même dès
qu'une copie suivante arrive ; rien pour une école sans dossier, ni au rythme « à la demande » quand
elle n'envoie rien. C'est ce que l'école a choisi qu'on vérifie ; ce qu'elle n'a pas choisi, on ne
le lui fait pas remarquer.

### T-131 — Le rythme livré

Quand l'école choisit un dossier, le rythme est-il « à la demande » (rien ne part sans un geste),
ou faut-il proposer « chaque semaine » d'emblée ?

*Recommandation* : « à la demande » à la livraison, comme vous l'avez dit le 11 novembre ; les trois
choix sont côte à côte avec la taille d'une copie et l'ordre de grandeur par mois, pour que l'école
choisisse en connaissance de cause.

### T-132 — Combien de copies garder sur le compte

*Recommandation* : les 4 dernières semaines et les 12 derniers mois, soit 16 copies au plus (environ
70 Mo pour l'école d'essai, loin des 5 Go gratuits de OneDrive ou des 15 Go de Google Drive). Fixe
dans la 2.0, sans réglage de plus.

### T-133 — Le fournisseur des écoles

Les écoles ont-elles plutôt un compte Google (téléphones Android), Microsoft ou Dropbox ? Le compte
est-il au nom de l'école ou à celui du directeur ou du promoteur ?

*Recommandation* : le guide présente Google Drive en premier, puis OneDrive, puis Dropbox, et
conseille un compte au nom de l'école dont le promoteur garde l'accès. GESCO marche de la même
façon avec les trois et avec tout autre dossier ; il ne vérifie pas à qui est le compte.

## 13. Journal des révisions

- v1.0, 15 novembre 2026 : première rédaction, d'après `docs/plan-phase-11.md` (§2.4, §7), les
  réponses du porteur à T-95, T-96 et T-97, et le code de sauvegarde au 15 novembre 2026.
