# GESCO 2.0 — essais à la main de la livraison à plusieurs postes

Phase 11, lot A, étape 6. Chaque ligne : ce qu'on fait → ce qu'on doit voir. La partie A a été
jouée sur le poste de développement le 12 novembre 2026. Les parties B et C sont **à jouer par le
porteur du projet** : elles demandent de répondre à Windows en personne, ou deux ordinateurs.

Notez pour chaque ligne : vu, pas vu, ou ce qui diffère. Un écart se rapporte avec le fichier
`journal-technique.txt` du poste (`%APPDATA%\ml.gesco.poste`).

---

## A. Joué sur une seule machine, sans toucher à Windows (12 novembre 2026)

| N° | Faire | Attendu | Résultat |
|---|---|---|---|
| A-1 | Essais automatiques de la règle (`cargo test -p gesco-reseau pare_feu`) | La règle lue est reconnue pour le bon programme et le bon port, malgré un chemin accentué ; un autre port ou un autre programme ne vaut pas ; le script ne nomme que les réseaux privés | Vu |
| A-2 | Scénario Playwright « la direction allume le réseau de l'école », sur la vraie application, pare-feu simulé qui refuse la première demande | Réseau allumé, avertissement « Le pare-feu de Windows n'est pas ouvert à GESCO » et bouton ; après le bouton, « Le pare-feu de Windows laisse entrer les postes reliés » | Vu (capture 162) |
| A-3 | Même scénario, réseau simulé public | Avertissement sur le réseau public avec le chemin du réglage de Windows ; « Vérifier de nouveau » le retire une fois le réseau privé | Vu (capture 163) |
| A-4 | Scénarios Playwright des postes, deux GESCO sur la même machine | Un second GESCO se relie, travaille, garde sa saisie pendant une coupure, reprend, est révoqué | Vu |
| A-5 | Construction de l'installeur (`pnpm installeur`) avec le crochet de désinstallation | `makensis` compile le crochet : les variables de Tauri qu'il lit existent toujours | Vu |
| A-6 | Installeur d'essai jetable (`makensis`) qui joue le crochet dans ses quatre cas, sur un dossier jetable | Case cochée et base présente : case ignorée ; case cochée sans base : gardée ; mise à jour : rien ne change ; case non cochée : rien ne change | Vu |
| A-7 | Lecture du pare-feu de ce poste sans droits d'administrateur (`netsh advfirewall firewall show rule`) | La lecture répond sans rien demander ; une règle absente rend un code d'échec | Vu |

Non joué sur ce poste : la pose réelle de la règle (elle demande de répondre « Oui » à
Windows), l'installation et la désinstallation de la 2.0.0, deux ordinateurs.

---

## B. À jouer par le porteur du projet, sur un seul ordinateur

Avec l'installeur `GESCO_2.0.0_x64-setup.exe`. Faites d'abord une sauvegarde de l'école sur la
clé USB si GESCO est déjà installé sur cet ordinateur.

| N° | Faire | Attendu |
|---|---|---|
| B-1 | Installer la 2.0.0 par-dessus la 1.2.7, lancer GESCO, se connecter | L'école est là, avec ses données ; aucun écran nouveau n'est imposé ; « Postes de l'école » paraît dans Paramètres pour la direction |
| B-2 | Paramètres → Postes de l'école | « Le réseau est éteint : GESCO ne sert que cet ordinateur. » et, sous le bouton, la phrase qui annonce la question de Windows |
| B-3 | Cliquer sur « Activer le réseau » | La fenêtre de Windows « Voulez-vous autoriser cette application à apporter des modifications à votre appareil ? » paraît (au besoin dans la barre des tâches) |
| B-4 | Répondre « Non » | « Le réseau est allumé », et l'avertissement « Le pare-feu de Windows n'est pas ouvert à GESCO » avec son bouton. Noter si Windows affiche en plus sa propre fenêtre « Le Pare-feu Windows Defender a bloqué… » |
| B-5 | Cliquer sur « Ouvrir le pare-feu de Windows », répondre « Oui » | « Le pare-feu de Windows laisse entrer les postes reliés, sur les réseaux privés seulement. » |
| B-6 | Ouvrir « Pare-feu Windows Defender avec fonctions avancées de sécurité » → Règles de trafic entrant | Une seule règle « GESCO postes relies » : Autoriser, TCP, port local 47800, profil Privé, programme `…\AppData\Local\GESCO\gesco-app.exe` |
| B-7 | Si le réseau de l'ordinateur est « public » dans Windows : regarder l'écran | L'avertissement sur le réseau public. Passer le réseau en « privé » (Paramètres de Windows → Réseau et Internet → propriétés du réseau), puis « Vérifier de nouveau » : l'avertissement disparaît |
| B-8 | Éteindre le réseau, le rallumer | Aucune question de Windows la seconde fois ; le pare-feu reste dit ouvert |
| B-9 | Fermer GESCO, le rouvrir, revenir à « Postes de l'école » | Réseau allumé, pare-feu ouvert, aucune question de Windows |
| B-10 | Paramètres de Windows → Applications → GESCO → Désinstaller, **en cochant** « Supprimer les données de l'application » | Un message dit que les données de l'école restent sur l'ordinateur. Après la désinstallation, le dossier `%APPDATA%\ml.gesco.poste` et son fichier `gesco.db` sont toujours là |
| B-11 | Réinstaller la 2.0.0, lancer GESCO | L'école est là ; le réseau est allumé ; le pare-feu est dit ouvert (la règle est restée) |
| B-12 | Journal technique (`%APPDATA%\ml.gesco.poste\journal-technique.txt`) | Une ligne « pare-feu ouverture demandee » par demande, avec `ouvert=false` après le « Non » et `ouvert=true` après le « Oui » |

---

## C. À jouer par le porteur du projet, sur deux ordinateurs

**Ce qu'il faut** : deux ordinateurs sous Windows 10 ou 11 (64 bits) sur le même réseau (le
Wi-Fi d'une box ou d'un petit routeur suffit, sans Internet), l'installeur de la 2.0.0 sur une
clé USB, une montre. Appelons **P** le poste principal (celui qui a l'école) et **R** l'autre.
Suivre `docs/installation-plusieurs-postes.md` et noter tout endroit où le guide ne dit pas ce
que l'écran montre.

### C1. Relier

| N° | Faire | Attendu |
|---|---|---|
| C-1 | Sur P : installer la 2.0.0, ouvrir l'école (ou la créer), déclarer le réseau « privé » dans Windows, puis Postes de l'école → Activer le réseau → « Oui » | Réseau allumé, pare-feu ouvert, une adresse du type 192.168.x.x, une empreinte |
| C-2 | Sur R : installer la 2.0.0, premier écran → « Rejoindre le poste principal de l'école », saisir l'adresse de P, « Chercher le poste principal » | « Un poste principal répond à l'adresse… en version 2.0.0 », et la même empreinte que sur P |
| C-3 | Sur R : saisir une adresse où il n'y a rien (192.168.x.250) | « Poste principal injoignable » en cinq secondes environ, rien n'est relié |
| C-4 | Sur P : « Afficher un code ». Sur R : nommer le poste « Caisse », saisir le code, relier | R montre l'écran de connexion de l'école sous le bandeau « Caisse » ; P le liste dans « Postes reliés » |
| C-5 | Sur R : dossier `%APPDATA%\ml.gesco.poste` | `poste.json` et le journal technique ; **ni `gesco.db` ni `cles.json`** |

### C2. Travailler

| N° | Faire | Attendu |
|---|---|---|
| C-6 | Sur R : se connecter avec un compte de l'école, inscrire un élève, encaisser un versement, imprimer le reçu | Tout marche comme sur P ; le reçu s'ouvre sur R ; sur P, le journal porte « Caisse » sur ces écritures |
| C-7 | Sur R : exporter une liste en Excel | Le classeur s'enregistre sur le disque de R |
| C-8 | Sur P et R : ouvrir la fiche de l'établissement ; corriger sur P et enregistrer ; corriger autre chose sur R et enregistrer | R est refusé : « modifiée entre-temps », avec le nom de la personne et l'heure ; sa saisie reste à l'écran |
| C-9 | Travailler dix minutes sur R (recherche d'élèves, fiches, notes, versements) | Chaque écran répond en moins d'une seconde ; noter tout geste plus lent, et lequel |
| C-10 | Sur P : fermer GESCO alors que R a une session | P prévient qu'un poste a une session ouverte et demande confirmation |

### C3. Coupures

| N° | Faire | Attendu |
|---|---|---|
| C-11 | Sur R : commencer une saisie sans enregistrer. Sur P : fermer GESCO. Sur R : enregistrer | Bandeau « liaison coupée », message d'erreur, la saisie reste à l'écran |
| C-12 | Sur P : rouvrir GESCO | R retrouve la liaison seul, en quelques secondes ; il faut se reconnecter (les sessions vivent sur P) |
| C-13 | Couper le Wi-Fi de R une minute, puis le remettre | Même comportement : coupée, puis rétablie sans rien toucher |
| C-14 | Sur P : mettre l'ordinateur en veille, le réveiller | R reprend seul ; noter le temps que cela prend |
| C-15 | Éteindre puis rallumer le routeur | Les deux reprennent ; noter si l'adresse de P a changé (écran « Postes de l'école »). Si oui, suivre le §9 du guide : R doit quitter puis se relier |

### C4. Pare-feu et réseau public

| N° | Faire | Attendu |
|---|---|---|
| C-16 | Sur P : passer le réseau en « public » dans Windows, « Vérifier de nouveau » ; sur R : un geste | P affiche l'avertissement du réseau public ; R dit « poste principal injoignable » |
| C-17 | Sur P : repasser le réseau en « privé », « Vérifier de nouveau » | L'avertissement disparaît ; R reprend seul |
| C-18 | Sur P : supprimer la règle « GESCO postes relies » dans le pare-feu de Windows, rouvrir « Postes de l'école » | « Le pare-feu de Windows n'est pas ouvert à GESCO » ; R est injoignable. « Ouvrir le pare-feu de Windows » → « Oui » : R reprend |

### C5. Versions, révocation, désinstallation

| N° | Faire | Attendu |
|---|---|---|
| C-19 | Si un installeur d'une autre version est disponible : l'installer sur R seulement | R est refusé avec la version attendue ; remettre la même version le fait reprendre, sans le relier de nouveau |
| C-20 | Sur P : « Révoquer » le poste Caisse | R ne travaille plus et le dit ; « Quitter le poste principal » le ramène au premier écran |
| C-21 | Relier R de nouveau (nouveau code) | R travaille ; l'ancienne ligne reste, révoquée |
| C-22 | Sur P : « Renouveler la clé du réseau » | R est refusé (empreinte différente) ; il se relie de nouveau avec un nouveau code |
| C-23 | Sur R : désinstaller GESCO en cochant « Supprimer les données de l'application » | Aucun message (R n'a pas de base) ; le dossier `%APPDATA%\ml.gesco.poste` de R est effacé. Sur P, révoquer le poste |
| C-24 | Sur P : désinstaller en cochant la même case | Le message dit que les données restent ; `gesco.db` est toujours là |

### C6. La mesure, sur un vrai réseau

Sur P, depuis le dépôt, sur une **copie** de l'école d'essai : `gesco-remplir mesurer <dossier>`
(voir `SPEC_PHASE_11_A.md` §6.5). L'outil simule dix postes depuis la machine où il tourne ; pour
mesurer le réseau, le plus simple est de chronométrer à la main sur R dix encaissements et dix
recherches (C-9) et de noter le plus lent. Cible : moins d'une seconde dans 95 cas sur 100.

---

## D. Le jardin d'enfants, à jouer par le porteur du projet sur l'école d'essai

Sur l'école d'essai (`Documents\gesco-ecole-essai`, ouverte par son lanceur), connecté comme
direction. La petite section y compte quarante enfants, observés au 1er et au 2e trimestre, et
leurs livrets sont publiés.

| N° | Faire | Attendu |
|---|---|---|
| D-1 | Paramètres → Compétences du jardin | Un onglet par section ; la liste livrée ; renommer, monter, retirer puis remettre une compétence fonctionnent |
| D-2 | Notes → classe « PS », 3e trimestre, domaine Langage | La grille des quarante enfants, avec les photos ; aucune case remplie ; aucun compteur |
| D-3 | « Toute la colonne… » → Acquis sur une compétence, puis A, E, N sur quelques cases | Le niveau suit aussitôt ; le curseur descend ; Suppr vide la case |
| D-4 | Commentaire et appréciation d'un enfant, quitter le champ | « Enregistré à … » |
| D-5 | Bulletins → classe « PS », 2e trimestre, Ouvrir un enfant | Ses domaines, une colonne pour le 1er et une pour le 2e trimestre |
| D-6 | Imprimer ce livret, puis l'imprimer en noir et blanc ou le photocopier | Une page A4 lisible : niveaux en lettres et ronds noirs ; ni case de photo vide, ni « non observé » |
| D-7 | Remise des bulletins → classe « PS », 1er trimestre | Les livrets à remettre ; une remise et une signature fonctionnent |
| D-8 | Paramètres → Enseignants → classe « PS » | « Lui confier la classe » ; la maîtresse figure parmi les maîtres de la classe |


## Ce qu'il faut rapporter

1. Les lignes « pas vu », avec ce qui s'est affiché à la place.
2. La fenêtre de Windows au moment d'activer le réseau : son texte exact, et si elle est venue
   devant GESCO ou derrière.
3. Si Windows a affiché sa propre fenêtre de pare-feu, à quel moment.
4. Les temps de C-9, C-12 et C-14.
5. Le journal technique de P et de R.
