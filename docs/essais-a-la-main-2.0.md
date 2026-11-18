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
direction. La petite section y compte quarante enfants, observés au 1er trimestre, et leurs
livrets du 1er trimestre sont publiés. L'école d'essai vit le 14 octobre 2026 : elle n'a rien du
2e trimestre.

| N° | Faire | Attendu |
|---|---|---|
| D-1 | Paramètres → Compétences du jardin | Un onglet par section ; la liste livrée ; renommer, monter, retirer puis remettre une compétence fonctionnent |
| D-2 | Notes → classe « PS », 2e trimestre, domaine Langage | La grille des quarante enfants, avec les photos ; aucune case remplie ; aucun compteur |
| D-3 | « Toute la colonne… » → Acquis sur une compétence, puis A, E, N sur quelques cases | Le niveau suit aussitôt ; le curseur descend ; Suppr vide la case |
| D-4 | Commentaire et appréciation d'un enfant, quitter le champ | « Enregistré à … » |
| D-5 | Bulletins → classe « PS », 1er trimestre, Ouvrir un enfant | Ses domaines, la colonne du 1er trimestre |
| D-6 | Imprimer ce livret, puis l'imprimer en noir et blanc ou le photocopier | Une page A4 lisible : niveaux en lettres et ronds noirs ; ni case de photo vide, ni « non observé » |
| D-7 | Remise des bulletins → classe « PS », 1er trimestre | Les livrets à remettre ; une remise et une signature fonctionnent |
| D-8 | Paramètres → Enseignants → classe « PS » | « Lui confier la classe » ; la maîtresse figure parmi les maîtres de la classe |


## E. Statistiques et rapports de l'école, à jouer par le porteur du projet sur l'école d'essai

Sur l'école d'essai, connecté comme direction, puis comme secrétaire et comme caissier.

| N° | Faire | Attendu |
|---|---|---|
| E-1 | Général → Statistiques | Les effectifs au 14 octobre 2026 ; le total de « Élèves par classe » égale le nombre d'élèves inscrits |
| E-2 | Changer la date d'arrêt au 1er octobre 2026 | Les chiffres changent ; « Chiffres arrêtés au 1er octobre 2026 » |
| E-3 | Cliquer un chiffre | La liste des élèves qu'il compte ; un nom mène à la fiche |
| E-4 | « Exporter tout l'onglet en Excel », ouvrir le classeur | Une feuille par tableau, sans la ligne discrète des fiches hors colonnes |
| E-5 | Onglet Tableau libre : élèves, classe en lignes, sexe en colonnes ; puis personnel par statut ; puis les rubriques de la fiche | Les tableaux se composent aussitôt ; le total compte aussi les fiches sans sexe |
| E-6 | « Données pour l'enquête », ouvrir le classeur | Quatorze feuilles, de « I. Identification » à « XIII. Observations », dans l'ordre ; « À remplir à la main » pour ce que GESCO ne tient pas |
| E-7 | Onglet Établissement : répondre à une rubrique, ajouter une rubrique de l'école | La réponse est gardée ; la rubrique ajoutée paraît dans le tableau libre et dans le classeur de l'enquête |
| E-8 | Général → Inventaire : saisir les salles de classe, « Reprendre l'an dernier » | Les lignes se gardent ; l'onglet Locaux et équipements les montre |
| E-9 | Général → Rapports : préparer le rapport de rentrée, rédiger une partie, aperçu, arrêter | Le PDF a les tableaux remplis ; arrêté, il ne bouge plus ; rouvrir crée une version 2 |
| E-10 | Fiche d'un élève → Fiche scolaire | Le PDF s'ouvre ; aucune case ne dit qu'une donnée manque |
| E-11 | Se connecter comme caissier | Ni Statistiques, ni Inventaire, ni Rapports dans le menu |

## F. La bibliothèque de cours, à jouer par le porteur du projet sur l'école d'essai

Sur l'école d'essai, connecté comme direction, puis comme secrétaire. Un second ordinateur relié
pour F-8.

| N° | Faire | Attendu |
|---|---|---|
| F-1 | Scolarité → Bibliothèque ; ouvrir 9e → Mathématiques → un chapitre → une leçon | Formules et figures lisibles ; A− et A+ changent la taille ; Mode classe en grand, Échap en sort |
| F-2 | Chercher « racine carree » | « La racine carrée d'un nombre positif » en tête, les mots surlignés, en moins d'une seconde |
| F-3 | Ouvrir la plus longue leçon de terminale TSE (le problème du baccalauréat) | Ouverte en moins de 2 secondes |
| F-4 | Retirer… → la matière ; puis Retirés → Remettre | La confirmation compte les leçons ; la matière disparaît puis revient |
| F-5 | Écrire → Commencer un brouillon, ajouter une phrase et une image, Enregistrer, Publier | La version de l'école est visible ; l'historique la montre et la compare à la livrée |
| F-6 | Ajouter une vidéo MP4 courte, puis la lire dans le cours et en mode classe | La vidéo se lit ; la ligne sur la sauvegarde paraît à la première vidéo |
| F-7 | Imprimer… → Leçon complète, puis Sans les corrigés, puis Tout le chapitre | Trois PDF lisibles ; le second n'a pas la section des corrigés |
| F-8 | Sur le poste relié : ouvrir la même leçon et lire la vidéo | Ouverte en moins de 3 secondes ; la vidéo se lit |
| F-9 | Se connecter comme secrétaire | La bibliothèque se lit ; ni Écrire, ni Retirer…, ni Faits à vérifier |
| F-10 | Paramètres → Modules : éteindre la bibliothèque, puis la rallumer | Elle disparaît du menu et du tableau de bord, puis revient telle quelle |
| F-11 | Enregistrer une consigne de 30 secondes avec un téléphone (MP3 ou M4A) ; l'ajouter à une leçon par Son, publier ; Écouter, puis Mode classe | Le son se lit, avec pause et avancer ; en mode classe, un grand bouton Écouter |
| F-12 | Sur le poste relié : ouvrir la leçon de F-11 et Écouter | Le son se lit |
| F-13 | Ajouter… → Leçon → Ajouter depuis une fiche Word, avec une vraie fiche d'un maître (titres, tableau, images, une formule) | Le brouillon montre les titres, le tableau et les images ; la formule est signalée à retaper |
| F-14 | Une leçon de l'école → Pour qui → cocher la 9e A seulement | Le catalogue porte « 9e A seulement » |
| F-15 | Exporter la matière de F-11 et F-13 sur une clé ; sur un autre GESCO (une autre école), Paramètres → Importer des leçons d'une autre école ; puis Publier les leçons reçues | Les leçons arrivent en brouillons sous les bonnes matières ; le son et les images sont là ; un second import n'ajoute rien |

### F bis. Les cours sur de vrais téléphones

Sur l'école d'essai, poste principal branché au routeur de l'école (ou à un routeur de maison).
Trois appareils si possible : un Android récent, un Android ancien (Android 8 ou 9), un iPhone.
Wi-Fi sans Internet de préférence, pour voir ce que voient les élèves.

| N° | Faire | Attendu |
|---|---|---|
| F-16 | Paramètres → Portail des élèves → cocher « Les élèves lisent les cours sur leurs appareils » ; répondre « Oui » à Windows si la ligne du pare-feu paraît | L'adresse s'affiche en grand (`http://…`) |
| F-17 | « Imprimer l'affiche de classe » ; viser son code QR avec chaque téléphone | La page « Les cours de ton école » s'ouvre |
| F-18 | Classes → 9e A → « Billets des élèves sans code » | Dix billets par page, lisibles, le code QR lisible |
| F-19 | Sur chaque téléphone : entrer avec un billet ; puis un code faux | Le prénom et la classe ; puis « Matricule ou code incorrect… » |
| F-20 | Ouvrir une leçon avec formules et figures ; A+ puis A− ; tourner le téléphone | Tout se lit sans défiler de côté |
| F-21 | Une leçon avec un son, puis une avec une vidéo : lire, avancer, mettre en pause | Le son et la vidéo se lisent et s'avancent |
| F-22 | « Enregistrer en PDF », puis ouvrir le fichier sur le téléphone | La leçon complète, lisible hors du Wi-Fi |
| F-23 | « J'ai fini cette leçon » ; au bureau, Bibliothèque → Suivi des élèves → 9e A → Par élève | L'élève y est, la leçon finie, dans les dix secondes |
| F-24 | Ajouter la page à l'écran d'accueil ; la rouvrir le lendemain | L'icône GESCO ; la page s'ouvre (le code est demandé de nouveau après deux heures sans activité) |
| F-25 | Éteindre le poste principal pendant qu'un téléphone lit, puis le rallumer | « Beaucoup d'élèves sont connectés… », puis la lecture reprend seule |
| F-26 | Trente téléphones (une classe) ouvrent la même leçon ; pendant ce temps, encaisser un paiement au bureau | L'encaissement reste rapide ; les téléphones lisent, au pire après « Réessaie dans un instant » |

## G. La sauvegarde hors de l'école, à jouer par le porteur du projet

Sur le poste principal, connecté en administrateur, avec un vrai compte : au moins OneDrive et
Google Drive. Le compte et le logiciel du fournisseur se préparent avec le guide d'installation
(« Une copie hors de l'école »).

| N° | Faire | Attendu |
|---|---|---|
| G-1 | Outils → Sauvegarde, avant d'installer quoi que ce soit | La section « Copie hors de l'école » ne montre que « Choisir un autre dossier… », sans message ; aucune ligne dans le bandeau |
| G-2 | Connecter OneDrive au compte de l'école, rouvrir l'écran | « OneDrive — …\OneDrive\GESCO » proposé ; rien n'est choisi |
| G-3 | Choisir, puis « Envoyer une copie maintenant » | Le volume est affiché avant ; un fichier `.gsv` apparaît dans `OneDrive\GESCO` ; état « Déposée dans le dossier » ou « En attente d'envoi » |
| G-4 | Attendre une heure, ordinateur connecté à Internet, rouvrir l'écran | L'état passe à « Envoyée » si Windows le connaît ; sinon il reste « Déposée dans le dossier » (noter lequel) |
| G-5 | Sur le site de OneDrive, depuis le téléphone | Le fichier est dans `GESCO`, daté du jour, de la taille affichée |
| G-6 | Mêmes gestes G-2 à G-5 avec Google Drive pour ordinateur | « Google Drive — G:\Mon Drive\GESCO » proposé ; mêmes résultats ; noter l'état atteint |
| G-7 | Rythme « Chaque semaine » | La prochaine copie est annoncée sept jours après la dernière ; l'ordre de grandeur par mois s'affiche |
| G-8 | Effacer la copie dans le dossier, attendre la vérification suivante (ou redémarrer GESCO) | « N'est plus dans le dossier » ; une ligne dans le bandeau ; elle disparaît après « Envoyer une copie maintenant » |
| G-9 | Déconnecter le lecteur `G:` (quitter Google Drive), « Envoyer une copie maintenant » | La sauvegarde sur l'ordinateur et sur la clé réussit ; aucune fenêtre d'erreur ; « N'a pas pu être déposée » |
| G-10 | Télécharger la dernière copie depuis le site, restaurer dans un dossier de données neuf avec les douze mots | L'école revient entière (élèves, paiements, notes) |
| G-11 | Plus tard : G-10 sur un autre ordinateur | Même résultat |
| G-12 | Sur un poste relié | L'écran des sauvegardes n'existe pas |

## H. Commencer GESCO en cours d'année, à jouer par le porteur du projet sur l'école d'essai

Sur l'école d'essai, au deuxième mois de l'année, connecté en administrateur, puis en caissier.
Le parcours est celui du chapitre `manuel-2.0-reprise.md`. Les montants sont ceux du cahier de
l'école ; notez ceux que vous tapez.

| N° | Faire | Attendu |
|---|---|---|
| H-1 | Paramètres → Années : l'année active, avec sa vraie date de rentrée | L'année est active ; ses périodes couvrent la rentrée |
| H-2 | Scolarité → Élèves → Importer la liste des élèves | Le champ « Dans l'école depuis le » montre la rentrée ; une date hors de l'année est refusée dès l'aperçu |
| H-3 | Valider une classe entière (plan par défaut), puis ouvrir la fiche d'un élève | Facture avec toutes ses tranches, la première comprise ; inscription datée de la rentrée |
| H-4 | Finance → Payé avant GESCO, avant toute caisse | « Arrêté avant GESCO : à ce jour » ; une ligne par élève validé ; la phrase des brouillons s'il en reste |
| H-5 | Taper un montant dans la case d'un élève, puis Entrée, et ainsi de suite sur cinq élèves | Chaque ligne s'enregistre, le reste baisse d'autant, le curseur passe à l'élève suivant ; les totaux en tête suivent |
| H-6 | Taper « 75 000 F », puis « 75.000 » sur un autre élève | Les deux sont lus 75 000 F |
| H-7 | Taper un montant plus grand que le dû | Accepté ; la colonne « À rembourser » montre l'excédent ; le total à rembourser en tête aussi |
| H-8 | Vider la case d'un élève qui a un montant, Entrée | GESCO demande le motif, « Saisie erronée » proposé ; après confirmation, le reste remonte |
| H-9 | « Télécharger le modèle Excel », l'ouvrir dans Excel | Les élèves validés de la sélection, le dû de l'année, la colonne « Payé avant GESCO » remplie pour ceux déjà saisis ; une feuille « Mode d'emploi » |
| H-10 | Remplir la colonne pour une classe, avec une faute exprès (un montant « beaucoup », un élève deux fois), puis « Choisir le fichier rempli » | L'aperçu montre chaque ligne : valides, à vérifier, en erreur, ignorées, avec le total qui sera enregistré ; rien n'est encore écrit |
| H-11 | « Importer » | Une sauvegarde est faite ; le bilan compte les montants enregistrés et les lignes refusées ; les refusées s'exportent en Excel |
| H-12 | Réimporter le même fichier | Toutes les lignes déjà écrites reviennent « Inchangé » |
| H-13 | Fiche d'un élève, section « Frais » | « Payé avant GESCO : … (arrêté à ce jour) » avec la note ; colonne « Dont avant GESCO » dans les échéances ; Modifier, Annuler, Historique |
| H-14 | Changer le plan de cet élève | Accepté ; le reste total ne bouge pas |
| H-15 | Élève qui a payé plus que sa facture, et dont un frais facultatif manque : « Ajouter un frais » | Les frais facultatifs absents sont proposés avec leurs échéances ; après l'ajout, « À rembourser au parent » baisse d'autant |
| H-16 | Trésorerie : soldes d'ouverture de la banque et du mobile money ; ouvrir la première caisse avec l'argent du tiroir | Dans « Payé avant GESCO », l'en-tête devient « avant le » jour de la caisse |
| H-17 | Élève avec un excédent : « Rembourser » | Dépense préremplie (catégorie « Remboursements aux parents », parent payeur, excédent, libellé avec le nom de l'élève) ; payée en espèces, la caisse perd exactement ce montant ; « À rembourser » baisse d'autant |
| H-18 | Rembourser plus que ce qui reste à rendre | Refusé : rien n'est remboursé deux fois |
| H-19 | Baisser le payé avant GESCO de cet élève sous ce qui lui a été rendu | Refusé, avec le minimum |
| H-20 | Encaisser un versement pour un élève qui a un payé avant GESCO | Le reçu porte le reste juste et ne parle pas du payé avant GESCO |
| H-21 | Imprimer le relevé de compte de cet élève | Ligne « Payé avant GESCO » en tête des paiements, avec sa date d'arrêt ; remboursements et reste à rendre s'il y a lieu |
| H-22 | Journal des encaissements du mois | Ligne « Payé avant GESCO — hors caisse » sous les totaux ; les totaux ne comptent que la caisse ; l'export Excel porte la ligne « Hors des totaux » |
| H-23 | Budget de l'année | Ligne « Payé avant GESCO (hors caisse) » sans prévu ; le total des recettes réalisées la compte ; les catégories non |
| H-24 | Journal de trésorerie du mois, livres des comptes, état de caisse | Rien pour le payé avant GESCO ; seul le remboursement payé y entre |
| H-25 | Connecté en caissier | L'entrée « Payé avant GESCO » n'existe pas ; sur la fiche, la ligne se lit sans Modifier ni Annuler |
| H-26 | Relevé du jour d'un jour d'octobre ; une évaluation datée d'octobre | Les élèves importés y sont ; les absences et les notes s'enregistrent |
| H-27 | Une fois l'année close | La liste se lit, les cases sont grisées, « Année close : lecture seule » |

## Ce qu'il faut rapporter

1. Les lignes « pas vu », avec ce qui s'est affiché à la place.
2. La fenêtre de Windows au moment d'activer le réseau : son texte exact, et si elle est venue
   devant GESCO ou derrière.
3. Si Windows a affiché sa propre fenêtre de pare-feu, à quel moment.
4. Les temps de C-9, C-12 et C-14.
5. Le journal technique de P et de R.
6. Pour G-4 et G-6 : l'état atteint par chaque fournisseur (« Envoyée » ou « Déposée dans le
   dossier »).
7. Pour F-16 à F-26 : le modèle et la version de chaque téléphone, et ce que chacun a affiché en
   se connectant à un Wi-Fi sans Internet.
8. Pour H-5 à H-12 : le temps de saisie d'une classe à l'écran et par le modèle Excel, et les
   lignes que l'aperçu a mises en erreur sans raison.
