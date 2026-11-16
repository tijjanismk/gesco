# SPEC_PHASE_12_LOT_4.md — Le portail de l'élève sur le Wi-Fi de l'école

**Projet** : ERP de gestion scolaire — Tauri 2 + Rust + React/TypeScript
**Phase** : 12 — e-learning ; **lot 4** : le poste principal sert la bibliothèque aux téléphones et
aux ordinateurs des élèves ; codes des élèves ; suivi par les maîtres ; version 2.0.0
**Statut** : v1.0 — proposée le 16 novembre 2026, à soumettre au porteur du projet avant toute
ligne de code ; sept questions au §15, chacune avec une recommandation
**Documents parents** : `docs/plan-phase-12.md` (§0, §0 bis E-13, §1.4, §3.6, §4 T-87 et T-91,
§5 décisions 1 à 3 et 14), `SPEC_PHASE_12_LOT_1.md`, `SPEC_PHASE_12_LOT_2.md`,
`SPEC_PHASE_11_A.md` (serveur et postes reliés), `SPEC_LOTS_C_D.md` (portail des parents, même
façon de faire), `docs/orchestration.md` (passage de relais du 16 novembre 2026)
**ADR proposés avec ce lot** (`docs/adr/`, 16 novembre 2026) :
`portail-des-eleves-separe-de-la-gestion`, `portail-des-eleves-en-clair-sur-le-wifi`,
`identite-de-l-eleve`, `charge-du-portail-des-eleves`, `tables-des-lots-2-a-5`
**Ordre des lots** : 2, puis ce lot 4, puis 3 (quiz), puis 5 (devoirs) — T-87 : le portail passe
devant les quiz.
**Date** : 16 novembre 2026

---

## 0. Objet et critère de fin

### 0.1 Ce que produit ce lot

T-87 : les élèves suivront les cours « tout cela à la fois » — sur les postes de l'école, sur leur
téléphone à l'école, sur le téléphone de la maison. Le lot 1 a donné le PDF à emporter ; ce lot
donne la lecture sur le Wi-Fi de l'école.

À la fin du lot :
- la direction allume « Les élèves lisent les cours sur leurs appareils » ; GESCO affiche
  l'adresse à donner aux élèves et imprime une affiche de classe avec un code QR ;
- le secrétariat imprime, classe par classe, les **billets** des élèves (matricule et code) ;
- l'élève ouvre l'adresse dans le navigateur de son téléphone ou d'un ordinateur de la salle
  informatique, entre avec son matricule et son code — ou lit sans code — et trouve les leçons de
  sa classe, rangées par matière et chapitre, avec formules, figures, images, sons et vidéos ; il
  enregistre une leçon en PDF pour la maison ;
- quand le suivi est allumé (T-91, à la livraison), le maître voit qui a ouvert et fini quelles
  leçons dans ses classes ; la direction règle ou éteint ce suivi ; rien ne bloque jamais
  l'élève ni le maître ;
- la gestion ne ralentit pas pendant qu'une classe entière lit.

### 0.2 Décisions déjà prises qui s'appliquent

- **T-87** (11 novembre 2026) : tous les supports à la fois ; le portail passe devant les quiz.
- **T-91** (11 et 14 novembre 2026) : le suivi des élèves par leurs maîtres est allumé à la
  livraison, réglable par chaque direction, et ne bloque jamais ni l'élève ni le maître. Son
  réglage `bibliotheque.suivi_eleves` existe depuis le lot 1 (montré, allumé, T-122).
- **T-92** : compris dans la licence ; aucun réglage de licence nouveau.
- **E-13** : par le portail, aucune commande de gestion ne répond ; l'élève n'a accès qu'aux
  contenus de sa classe ; on ne garde de lui que son matricule et son avancement ; aucune note
  n'entre au bulletin sans un geste du maître.
- **T-118** : une leçon livrée est visible des élèves dès l'installation ; **T-116** : aucun texte
  ne dit qu'une leçon a été relue ou non.
- **« La flexibilité sans incohérence »** : ne jamais bloquer, ne pas faire remarquer ce qui
  manque, chiffres justes. **Aucun texte affiché ne cite de source.**
- Portail des parents (`SPEC_LOTS_C_D.md`) : même socle technique (React du dépôt, entrée Vite à
  part), mêmes règles d'écriture ; il est en ligne, celui-ci est sur le Wi-Fi.
- Méthode : fmt, clippy, tsc et ESLint à chaque étape ; tous les essais à la dernière étape ; un
  seul Playwright d'ensemble.

### 0.3 Critère de fin, vérifiable

1. **Rien de la gestion** : un essai envoie au port du portail chacune des routes du serveur des
   postes et toutes les commandes du contrat ; aucune ne répond autrement que « introuvable ».
   L'essai échoue si une route du portail n'est pas dans la liste écrite du §3.
2. **Portail éteint** : rien n'écoute sur ses ports ; allumé, il écoute seulement les adresses du
   réseau privé.
3. **Entrer** : un code juste fait entrer l'élève, qui voit son prénom et sa classe ; un code faux,
   un matricule inconnu, un élève sorti ou non réinscrit reçoivent la même réponse ; cinq codes
   faux dans la journée bloquent ce matricule jusqu'au lendemain, et « Lire sans code » reste
   possible.
4. **Sa classe seulement** : un élève de 9e A voit les leçons visibles de la 9e année non retirées,
   plus les ajouts réservés à la 9e A, et rien de la 9e B réservé, rien d'un autre niveau ; un
   brouillon, une version soumise, une note pour le maître, une liste de faits ne sont jamais dans
   une réponse du portail (essai sur le texte des réponses).
5. **Visible tout de suite** : la direction publie une version ; l'élève qui recharge la leçon voit
   la nouvelle version ; un contenu retiré disparaît au rechargement.
6. **Suivi** : un élève entré avec son code ouvre une leçon puis appuie sur « J'ai fini » ; le
   maître de la classe le voit dans le suivi dans les dix secondes ; suivi éteint, rien n'est
   écrit ; un élève sans code n'est jamais compté.
7. **Charge** (mesurée à l'étape 6, sur le poste du porteur) : soixante élèves simulés lisent et
   marquent des leçons pendant qu'un poste relié encaisse et qu'un autre saisit des notes ; un
   encaissement reste sous une seconde ; une leçon s'ouvre en moins de 2 secondes ; au-delà des
   bornes, l'élève lit « Beaucoup d'élèves sont connectés. Réessaie dans un instant. ».
8. **Téléphone** : chaque écran se lit sur 320 pixels de large, contrastes du projet tenus, cibles
   de 44 pixels ; une leçon avec formules et figures s'affiche ; un son et une vidéo se lisent et
   s'avancent ; le PDF d'une leçon se télécharge.
9. **Billets et affiche** : les billets d'une classe de 60 élèves s'impriment en PDF, vérifiés en
   image ; réimprimer remet de nouveaux codes et les anciens ne marchent plus.
10. **Journal** : billets produits, code remis, matricule bloqué, portail allumé ou éteint sont au
    journal d'audit ; aucun code ni jeton n'y figure, ni dans les journaux du serveur.
11. **Essai du porteur** avec de vrais téléphones (essais à la main, partie de ce lot) : entrer,
    lire, son, vidéo, PDF, raccourci sur l'écran d'accueil.
12. Module éteint ou portail éteint : les essais des phases précédentes passent sans changement.

---

## 1. Périmètre

**Inclus** : serveur du portail ; codes des élèves, billets, affiche ; entrée avec ou sans code,
session ; catalogue, chapitre, lecteur de leçon, PDF, tableau « Mon avancement » ; suivi par les
maîtres et la direction ; réglages ; bornes de charge et mesure ; règle de pare-feu ; guide
d'installation (Wi-Fi des élèves), manuel, essais à la main.

**Exclu** :
- Les quiz interactifs (lot 3) : jusqu'au lot 3, la section « Quiz » d'une leçon s'affiche comme
  du texte, les réponses repliées sous « Voir les réponses ».
- Les devoirs à rendre (lot 5).
- L'installation véritable et la lecture hors ligne (ADR « en clair sur le Wi-Fi ») ; le portail
  en ligne (édition Connectée, après le lot C).
- Les notifications sur le téléphone.
- L'accès depuis la maison au portail local : la maison passe par le PDF.

---

## 2. Ce que l'application possède déjà

| Couture | Où | Ce qu'elle apporte au lot |
|---|---|---|
| Serveur du réseau local | `crates/gesco-reseau` (axum 0.8.9), port 47 800 en TLS, trois routes | Un second routeur dans la même crate |
| Origine d'un appel | `gesco-services/src/origine.rs` (`Locale`, `Relie`) | Une origine `Portail` de plus |
| Règle de pare-feu | `gesco-reseau/src/pare_feu.rs`, installeur | Les ports du portail |
| Bibliothèque | Catalogue, versions visibles, retraits, fichiers par empreinte, vidéos par morceaux (lot 1) ; portée par classe (lot 2) | Ce que lit l'élève |
| Format des leçons | Arbre de blocs du noyau, KaTeX 0.18.11 au frontend, figures SVG | Même rendu sur le téléphone |
| Impression | Modèles Typst de la leçon, formes « complète » et « sans les corrigés » | PDF à emporter depuis le portail |
| Code des responsables | Migration 0047 : six caractères, Argon2id, essais comptés, billet | Modèle du code des élèves |
| Code QR | `qrcode` 0.14.1 dans `gesco-docs` (rendu SVG) | Adresse du portail sur l'affiche et les billets |
| Élèves et inscriptions | Matricule, classe de l'année, sorties | Qui peut entrer, quelle classe |
| Affectations des maîtres | `affectation_enseignement` | Ce que chaque maître suit |
| Réglage du suivi | `bibliotheque.suivi_eleves` (lot 1) | Allume ou éteint le suivi |
| Comptes des maîtres | `notes.saisie_maitres` : éteint, un compte de maître n'ouvre pas de session | Voir T-134 |

---

## 3. Le serveur du portail

Règles : ADR « portail des élèves séparé de la gestion » et « en clair sur le Wi-Fi ».

- **Allumer** : réglages de la bibliothèque, « Les élèves lisent les cours sur leurs appareils »
  (éteint à la livraison, T-129). Allumé sur un poste qui n'est pas principal, ou sans le serveur
  du réseau local : l'écran dit en une ligne ce qu'il faut allumer d'abord, avec le bouton.
- **Port** : 80 s'il est libre, sinon 47 801 ; le port retenu et l'adresse
  (`http://<adresse du poste principal>[:port]`) sont affichés avec leur code QR. Si l'adresse du
  poste change (routeur), l'écran le dit et propose de réimprimer l'affiche ; le guide explique
  comment fixer l'adresse dans le routeur.
- **Routes**, et rien d'autre :

| Route | Ce qu'elle fait |
|---|---|
| `GET /` et fichiers du frontend des élèves | Pages, compressées, en cache |
| `POST /eleve/api/entrer` | Matricule et code ; crée la session |
| `POST /eleve/api/lire-sans-code` | Niveau (et série) choisis ; session sans identité |
| `POST /eleve/api/sortir` | Fin de session |
| `GET /eleve/api/catalogue` | Matières, parties, chapitres, contenus visibles pour la classe ou le niveau |
| `GET /eleve/api/contenu/{cle}` | Arbre de blocs de la version visible |
| `GET /eleve/api/fichier/{empreinte}` | Figure, image, son ou vidéo appelé par un contenu visible ; requêtes partielles |
| `GET /eleve/api/pdf/{cle}` | PDF « leçon complète » de la version visible (T-138) |
| `POST /eleve/api/avancement` | Leçon ouverte, leçon finie |
| `GET /eleve/api/mon-avancement` | Tableau de l'élève |

  Les lots 3 et 5 ajoutent leurs routes à cette liste, et à l'essai qui la vérifie.
- **Fonctions du service `portail`** : une par route, sous `Origine::Portail` ; elles ne lisent
  que des versions visibles, non retirées, du niveau de la session, dans la portée des ajouts
  réservés ; un fichier n'est servi que s'il est appelé par un contenu visible pour la session.
- **Réponses** : jamais de nom de famille, de date de naissance, de note, d'absence, de
  responsable ; jamais de note pour le maître, de liste de faits, de brouillon.
- **En-têtes** : politique de contenu stricte (aucun script, police ou image venus d'ailleurs),
  pas de mise en cache des réponses de l'API, cache permanent des fichiers par empreinte.

---

## 4. L'élève entre

Règles : ADR « identité de l'élève ».

- **Codes** : six caractères, alphabet des codes des responsables, un seul actif par élève,
  empreinte Argon2id (paramètres légers). Valable tant que l'élève est inscrit dans une classe de
  l'année en cours ; il le reste d'une année à l'autre.
- **Billets** : depuis la liste d'une classe, « Codes du portail » : pour les élèves sans code, ou
  pour tous ; le PDF met dix billets par page A4 (prénom, nom, classe, matricule, code, adresse et
  code QR) ; une ligne dit que réimprimer remplace les codes. Depuis la fiche d'un élève :
  « Code du portail : remis le … » et « Remettre un code ».
- **Affiche de classe** : A4, « Les cours sur ton téléphone », l'adresse en grand, son code QR,
  trois étapes (« Connecte-toi au Wi-Fi de l'école », « Ouvre l'adresse », « Entre ton matricule
  et ton code ») ; depuis les réglages du portail.
- **Écran d'entrée** : matricule, code, « Entrer » ; en dessous, « Lire sans code ». Message d'un
  refus : « Matricule ou code incorrect. Demande ton code au secrétariat. » ; matricule bloqué :
  « Trop d'essais pour ce matricule aujourd'hui. Tu peux lire sans code. ».
- **Lire sans code** (T-128) : choix du niveau, et de la série au lycée, parmi ceux de l'école ;
  une ligne dit « Ton avancement n'est pas gardé ». Ni avancement, ni tentative, ni devoir.
- **Session** : témoin `HttpOnly`, `SameSite=Strict` ; fin à « Sortir » et après la durée sans
  activité (T-135, 2 heures par défaut, réglable) ; case « Ce n'est pas mon téléphone » à l'entrée,
  qui limite la session à la fermeture du navigateur.
- **Suivi éteint** : l'entrée par code reste possible (le lot 5 en aura besoin), rien de
  l'avancement n'est écrit, « Mon avancement » n'est pas montré.

---

## 5. Les écrans de l'élève

Une entrée Vite à part, `frontend/eleve/`, construite à part du bureau, sans `@tauri-apps` ;
React et KaTeX du dépôt ; police **Andika 7.000** (SIL, licence libre OFL, sortie le 2 juin 2025,
vérifiée sur GitHub le 16 novembre 2026), régulière et grasse, en woff2. Premier chargement :
300 Ko compressés au plus, polices comprises (ADR « charge »).

1. **Entrer** (§4).
2. **Mes matières** : une carte par matière du niveau, avec son nombre de leçons et, pour un élève
   suivi, « 12 finies » ; la dernière leçon ouverte en tête (« Reprendre »).
3. **Chapitres d'une matière** : parties et chapitres dans l'ordre du programme ; chaque contenu
   avec sa nature (« Leçon », « Sujet du DEF 2024 », « Corrigé », « Document ») et, pour un élève
   suivi, une coche « finie ».
4. **Leçon** : texte, formules, figures, images, sons, vidéos (T-137) ; taille du texte A−, A+ ;
   les corrigés et la sous-section « Réponses » du quiz repliés sous « Voir les corrigés » et
   « Voir les réponses » ; en bas, « J'ai fini cette leçon » (élève suivi) et « Enregistrer en
   PDF » ; un sujet renvoie à son corrigé ; un document PDF de l'école s'ouvre dans le lecteur du
   navigateur.
5. **Mon avancement** (élève suivi) : par matière, leçons finies sur leçons visibles, dernières
   leçons ouvertes.

Règles d'écriture : tutoiement, phrases courtes, aucun mot technique (« session », « serveur »),
aucune source, aucun état de relecture. Un contenu sans texte n'est pas montré. Une vidéo ou un son
absent n'est pas montré.

---

## 6. Le suivi

- **Ce qui s'écrit** (élève entré avec son code, suivi allumé) : leçon ouverte (première et
  dernière fois), leçon finie (« J'ai fini cette leçon »), avec l'empreinte de la version lue ;
  par année scolaire. Écritures groupées toutes les cinq secondes (ADR « charge »).
- **Ce que voit le maître** (Bibliothèque, onglet « Suivi des élèves », ses classes et ses
  matières) : par leçon, combien d'élèves l'ont ouverte et finie ; par élève, ses leçons finies et
  sa dernière visite ; trié par chapitre ; imprimable. Les quiz s'y ajoutent au lot 3.
- **Grain** (T-136) : « par élève » à la livraison ; la direction peut choisir « par classe
  seulement » (nombres sans noms) ou éteindre le suivi.
- **La direction** voit toutes les classes.
- **Éteindre** : rien de nouveau ne s'écrit ; ce qui a été écrit reste (rien ne se supprime) mais
  n'est plus montré ; rallumer le remontre.
- **Jamais au bulletin**, jamais d'alerte ni de rappel à l'élève ; aucun classement d'élèves.

---

## 7. Charge

Règles et nombres de départ : ADR « charge du portail des élèves ».
- Catalogue et arbres de blocs en mémoire, recalculés à chaque publication, retrait, import ou
  installation de paquet ; fichiers servis par empreinte avec cache permanent.
- 48 requêtes simultanées pour les pages et l'API, 4 pour les vidéos, 2 pour les PDF ; au-delà,
  le message d'attente, et la page se relance seule.
- PDF fabriqué une fois par version visible, gardé dans un dossier de travail hors sauvegarde.
- Vidéos et sons par requêtes partielles ; jamais lus en entier en mémoire.

---

## 8. Données nouvelles

Migration **0064** (ou la suivante libre), ADR « tables des lots 2 à 5 » :

| Table | En bref |
|---|---|
| `code_eleve` | Élève, empreinte, actif, remis le, par, révoqué le ; un seul actif par élève |
| `essai_code_eleve` | Matricule, jour, nombre d'essais faux |
| `avancement_lecon` | Élève, année, contenu, empreinte de la version, ouvert le (premier, dernier), fini le |

Réglages : `portail.eleves` (booléen, éteint) ; `portail.suivi_grain` (`eleve` ou `classe`,
`eleve`) ; `portail.duree_session_min` (entier, 120) ; `portail.videos` (booléen, allumé, T-137) ;
`portail.pdf` (booléen, allumé, T-138). `bibliotheque.suivi_eleves` existe.

Fichiers : dossier de travail des PDF du portail dans le dossier de données, hors sauvegarde,
vidé sans risque.

---

## 9. Règles

1. **Un téléphone n'atteint jamais la gestion** : routeur à part, liste fermée, essai.
2. **L'élève ne voit que des versions visibles de son niveau**, retraits respectés, ajouts réservés
   respectés.
3. **Ne jamais bloquer** : code oublié ou bloqué, l'élève lit sans code ; suivi éteint, tout se
   lit ; serveur chargé, message d'attente et relance seule ; fichier absent, la leçon s'ouvre.
4. **On ne garde de l'élève que son identifiant, son code (empreinte) et son avancement.**
5. **Le suivi ne classe pas, ne relance pas, ne note pas.**
6. **Chaque geste du personnel est tracé** : portail allumé ou éteint, billets, code remis, grain
   du suivi changé.
7. **La gestion d'abord** : les bornes protègent la caisse et la saisie.

---

## 10. Droits

Deux permissions nouvelles, dans la composition livrée suivante (18 si rien ne l'a prise avant),
écrites dans `docs/matrice-permissions.md` par l'essai qui la génère :

| Permission | Ce qu'elle permet | Rôles à la livraison |
|---|---|---|
| `portal:codes` | Produire les billets, remettre un code, voir si un élève a un code | Administrateur, Directeur, Directeur des études, Secrétaire |
| `library:follow` | Voir le suivi des élèves ; portée des affectations pour un Enseignant | Administrateur, Directeur, Directeur des études, Enseignant |

Allumer le portail, régler le suivi, sa durée, les vidéos, le PDF et imprimer l'affiche relèvent
de `library:manage`.

**Comptes des maîtres** (T-134) : aujourd'hui, un compte qui n'a que le rôle Enseignant n'ouvre
pas de session tant que « Les maîtres saisissent leurs notes » est éteint. Le suivi par les
maîtres ne se voit donc que si la direction ouvre leurs comptes ; sinon la direction le voit et
l'imprime pour eux. La recommandation de T-134 sépare les deux réglages.

---

## 11. Écrans du personnel

- **Réglages de la bibliothèque**, section « Portail des élèves » : interrupteur, adresse et code
  QR, « Imprimer l'affiche », durée de session, vidéos, PDF, grain du suivi.
- **Classe** : bouton « Codes du portail » ; **fiche de l'élève** : état du code, « Remettre un
  code ».
- **Bibliothèque** : onglet « Suivi des élèves » (classe, matière, par leçon ou par élève,
  imprimer).
- **Tableau de bord** : rien de nouveau.
- Lisibilité : skill du projet.

---

## 12. Livraison

Skill `livraison` à l'étape 7 :
- règle de pare-feu pour les ports 80 et 47 801, réseau privé seulement, posée et retirée par
  l'installeur ; un port 80 pris ne bloque rien (47 801) ;
- le portail ne démarre jamais sur un poste relié ;
- guide d'installation, section « Le portail des élèves » : Wi-Fi des élèves séparé quand le
  routeur le permet, adresse fixe du poste principal, affiche, billets, combien d'élèves un
  routeur ordinaire tient, ce que voit un téléphone sans Internet ;
- manuel : « Ouvrir les cours aux élèves », « Remettre les codes », « Suivre ses élèves » ;
- essais à la main du porteur avec de vrais téléphones (Android récent et ancien, un iPhone si
  possible).

---

## 13. Ordre de réalisation

Des étapes courtes ; une session en fait plusieurs, commite et pousse après chacune, passe la main
au-delà d'environ 150 000 jetons. Une seule session compile. Élaguer `target` avant toute grosse
compilation.

1. **Serveur du portail** : second routeur dans `gesco-reseau`, ports, écoute privée, réglage
   `portail.eleves`, `Origine::Portail`, liste des routes et son essai (non lancé), page d'essai
   « GESCO répond » ; **le porteur ouvre cette page depuis un vrai téléphone** (dix minutes) pour
   confirmer ce que suppose l'ADR « en clair sur le Wi-Fi ».
2. **Codes et entrée** : migration 0064, codes, essais comptés, sessions en mémoire, billets et
   affiche (Typst, code QR), lire sans code, permissions `portal:codes` et `library:follow`.
3. **Service du portail** : catalogue, contenu, fichiers, PDF, visibilité par niveau, classe et
   retraits, cache en mémoire et son invalidation.
4. **Écrans de l'élève** : entrée Vite `frontend/eleve/`, Andika, cinq écrans, téléphone d'abord.
5. **Suivi** : avancement groupé, onglet « Suivi des élèves », grain, impression ; comptes des
   maîtres selon T-134.
6. **Charge** : bornes, requêtes partielles, message d'attente ; outil de mesure (soixante élèves
   simulés, deux postes) et chiffres écrits dans l'ADR « charge ».
7. **Essais et livraison du lot** : suite Rust complète, vitest, knip ; critères du §0.3 ;
   Playwright d'ensemble (écran de 360 pixels : entrer, lire, son, PDF, suivi du maître) avec
   captures montrées au porteur ; skill `livraison` ; guide, manuel, notes de version, essais à la
   main ; école d'essai refaite (portail allumé, une classe avec billets).

Les étapes 3 et 4 peuvent avancer ensemble une fois la liste des routes fixée.

---

## 14. Ce que ce lot prépare

- Lot 3 : routes `/eleve/api/quiz…`, tentatives écrites par la même file groupée, résultats dans
  l'onglet « Suivi des élèves ».
- Lot 5 : routes des devoirs et des photos, élève entré avec son code seulement.

---

## 15. Questions au porteur du projet

Numérotées T-128, T-129 et T-134 à T-138 (T-130 à T-133 sont prises par la sauvegarde hors de
l'école). Chacune a une recommandation ; le lot peut commencer sur les recommandations.

### T-128 — Lire sans code

Un élève a oublié son billet, ou la direction a éteint le suivi. Peut-il lire quand même ?
- a) oui : « Lire sans code », il choisit son niveau ; rien de lui n'est gardé ;
- b) non : sans code, pas de lecture.

*Recommandation* : a). « Ne jamais bloquer » : un élève renvoyé devant une leçon qu'il pourrait
lire en PDF n'apprend rien ; seul son suivi manque ce jour-là. Étape 2.

### T-129 — Le portail à la livraison

- a) éteint : la direction l'allume quand elle est prête (Wi-Fi, billets) ;
- b) allumé dès que le serveur du réseau local est allumé.

*Recommandation* : a). Il ouvre un port à tout appareil du Wi-Fi ; c'est à la direction de le
décider, et il ne sert à rien avant que les billets soient remis. Étape 1.

### T-134 — Les comptes des maîtres et le suivi

Aujourd'hui, un maître n'ouvre son compte que si « Les maîtres saisissent leurs notes » est allumé
(éteint à la livraison). Le suivi, l'écriture des leçons et, au lot 5, les devoirs passent par ce
compte.
- a) deux réglages : « Les maîtres ont leur compte (bibliothèque, suivi, devoirs) » et « … et
  saisissent leurs notes » ; le premier peut être allumé sans le second ;
- b) un seul réglage, comme aujourd'hui : sans la saisie des notes, la direction voit le suivi et
  l'imprime pour les maîtres.

*Recommandation* : a), le premier réglage éteint à la livraison comme aujourd'hui. Une école qui
garde la saisie des notes au secrétariat peut quand même donner aux maîtres leur suivi et leurs
leçons. Étape 5.

### T-135 — Le téléphone partagé

Combien de temps un élève reste-t-il entré sans rien faire ?
- a) 2 heures, réglable, avec la case « Ce n'est pas mon téléphone » qui le fait sortir à la
  fermeture du navigateur ;
- b) 30 minutes.

*Recommandation* : a). Un cours dure une à deux heures ; la case couvre le téléphone emprunté et
l'ordinateur de la salle informatique. Étape 2.

### T-136 — Ce que le maître voit du suivi

- a) par élève (qui a lu quoi), la direction pouvant passer à « par classe seulement » ou éteindre ;
- b) par classe seulement (nombres, sans noms).

*Recommandation* : a). C'est le suivi que T-91 demande : le maître aide l'élève qui n'a rien
ouvert. La direction garde la main. Étape 5.

### T-137 — Les vidéos de l'école sur les téléphones

- a) oui, quatre à la fois au plus, la direction pouvant les couper ;
- b) non : les vidéos se regardent en classe (mode classe) seulement.

*Recommandation* : a). Une vidéo d'une leçon est ce que l'élève revoit le plus volontiers ; la
borne de quatre protège le Wi-Fi et se règle après la mesure. Étape 3.

### T-138 — Le PDF depuis le téléphone

- a) oui, la leçon complète, pour la garder et la lire à la maison sans Wi-Fi ;
- b) non : le PDF reste au bureau, le maître le partage par WhatsApp.

*Recommandation* : a). C'est la seule façon pour l'élève d'emporter la leçon chez lui (T-87) ; le
PDF est fabriqué une fois par version et gardé. Étape 3.

---

## 16. Journal des révisions

- v1.0, 16 novembre 2026 : première rédaction, d'après `docs/plan-phase-12.md` §3.6 et §5, les
  décisions du porteur (T-87, T-91, T-92) et ce que les lots 1 et 2 apportent ; cinq ADR
  (`portail-des-eleves-separe-de-la-gestion`, `portail-des-eleves-en-clair-sur-le-wifi`,
  `identite-de-l-eleve`, `charge-du-portail-des-eleves`, `tables-des-lots-2-a-5`) ; questions
  T-128, T-129, T-134 à T-138.
