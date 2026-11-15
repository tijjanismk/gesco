# SPEC_PHASE_12_LOT_1.md — La bibliothèque de cours dans GESCO

**Projet** : ERP de gestion scolaire — Tauri 2 + Rust + React/TypeScript
**Phase** : 12 — e-learning ; **lot 1** : la bibliothèque livrée dans GESCO, retirer, ajouter, améliorer les cours, publier, imprimer et enregistrer en PDF ; version 2.0.0
**Statut** : v1.0 — proposée le 15 novembre 2026, à soumettre au porteur du projet avant toute ligne de code ; huit questions au §17, chacune avec une recommandation
**Documents parents** : `docs/plan-phase-12.md` (§0, §0 bis, §3.3, §3.4, §4, §5), `docs/orchestration.md` (décisions des 12 au 15 novembre 2026), `gesco-bibliotheque\DECISIONS.md` et `contenus\REPARTITION.md` (hors du dépôt), `SPEC_PHASE_11_A.md` (plusieurs postes), `SPEC_PHASE_4.md` (affectations des maîtres), `SPEC_PHASE_9.md` (modèle de cette spécification)
**ADR proposés avec ce lot** (`docs/adr/`, tous du 15 novembre 2026) :
`format-des-lecons`, `stockage-des-contenus`, `tables-de-la-bibliotheque`, `retirer-un-contenu`,
`paquet-de-contenus`, `repere-commun-des-contenus`, `programme-officiel`,
`licence-et-module-bibliotheque` (décisions 4 à 9, 11 et 13 du plan)
**Date** : 15 novembre 2026

---

## 0. Objet et critère de fin

### 0.1 Ce que produit ce lot

GESCO est livré avec une bibliothèque de cours écrite pour lui, selon le programme malien :
398 leçons de 7e, 8e, 9e année et de terminale, 41 sujets officiels du DEF et du BAC avec leurs
corrigés. Aujourd'hui, cette bibliothèque est dans un dossier hors de l'application.

À la fin du lot :
- l'école ouvre GESCO et trouve, dans **Bibliothèque**, les cours rangés par classe, matière,
  partie et chapitre, pour les seuls niveaux qu'elle a ; elle lit une leçon, avec ses formules et
  ses figures, un sujet et son corrigé ; elle cherche par mot ;
- la direction **retire** ce qu'elle ne veut pas, à cinq grains (contenu, chapitre, matière,
  niveau, paquet), et le **remet** quand elle veut ;
- un maître ou la direction **ajoute** des chapitres, des leçons, des documents et des sujets ;
- un maître **améliore** une leçon livrée : la leçon livrée reste intacte, l'école en écrit sa
  version ; chaque modification est tracée ; on peut comparer et revenir à la version livrée ;
- la direction **valide** avant que les élèves voient ; elle peut donner nommément à un maître le
  droit de publier lui-même ; elle marque une leçon **relue** ;
- les leçons reçoivent des **images**, des **documents** et des **vidéos** de l'école ; deux
  guides courts aident à filmer et à montrer une vidéo ;
- toute leçon, tout sujet, tout corrigé et tout chapitre **s'imprime ou s'enregistre en PDF**,
  pour la classe, pour la maison, pour WhatsApp ;
- une **mise à jour** de la bibliothèque ne touche jamais ce que l'école a retiré, ajouté ou
  écrit.

### 0.2 Décisions déjà prises qui s'appliquent

- **La bibliothèque est rédigée pour GESCO** (11 novembre 2026). Rien n'est recopié, sauf les
  énoncés officiels du DEF et du BAC (14 novembre 2026), dont les corrigés sont écrits à neuf.
- **Tout ce qui est écrit entre dans GESCO, sans ordre de classes** (14 novembre 2026) : 7e, 8e,
  9e, terminale et sujets, matières incomplètes comprises. L'école complète dans l'application.
  Cela règle T-88.
- **Formules en LaTeX affichées par KaTeX** (12 novembre 2026) ; **figures de géométrie en SVG**
  (13 novembre 2026).
- **Vidéos** (12 novembre 2026) : ajoutées par l'école, fichiers locaux hors de la base, lues sans
  Internet et depuis les postes reliés, jamais dans la sauvegarde ; GESCO n'en fournit aucune ;
  un bouton les copie vers une clé et les reprend, annoncé à l'ajout de la première ; une vidéo
  absente ne bloque rien ; MP4 seulement, avertissement sans refus au-delà d'une taille
  conseillée ; deux guides courts (filmer une leçon avec un téléphone ; ajouter la vidéo et la
  montrer en classe).
- **Cours améliorables dans l'application** (14 novembre 2026) : la leçon livrée reste intacte,
  l'école en fait sa version ; chaque modification est tracée ; retour possible à la version
  livrée ; une mise à jour de la bibliothèque ne touche jamais la version de l'école.
- **T-89** (14 novembre 2026) : l'enseignant de la matière modifie, la direction valide avant que
  les élèves voient ; la direction peut donner nommément à un maître le droit de publier.
- **T-91** (14 novembre 2026) : le suivi des élèves par leurs maîtres est allumé à la livraison,
  réglable par chaque direction, et ne bloque jamais. Il sert à partir du lot 4 ; son réglage est
  posé dans ce lot.
- **T-87** (11 novembre 2026) : tous les supports à la fois ; donc la leçon en PDF à emporter.
- **T-92** (11 novembre 2026) : compris dans la licence.
- **T-93 et T-94** : sans objet (aucun manuel d'éditeur, aucun quiz fabriqué au gabarit).
- **« La flexibilité sans incohérence »** (14 novembre 2026) : ne jamais bloquer, ne pas faire
  remarquer ce qui manque, chiffres et documents toujours justes.
- **Interface sans sources** : aucun texte affiché ou imprimé ne cite un numéro de question, de
  spécification, une date de décision, un site, un livre ou un auteur.
- **Relecture par la direction de chaque école** (11 novembre 2026) : aucune leçon n'a été relue
  par un enseignant, et GESCO ne le prétend jamais.
- R-14 : rien ne se supprime. R-21 : toute commande passe par `Application::executer`. R-27 :
  journal d'audit chaîné.
- **Lot A** : tout écran de ce lot marche depuis un poste relié ; les fichiers voyagent par leur
  contenu, 20 Mo au plus par fichier ; deux personnes qui écrivent le même texte ne s'écrasent pas
  (refus si le texte a changé depuis sa lecture).
- **Méthode de travail** (14 novembre 2026) : pendant les étapes, fmt, clippy, tsc et ESLint
  seulement, puis commit et push ; tous les essais (suite Rust complète, vitest, knip,
  Playwright) une seule fois, à la dernière étape.

### 0.3 Critère de fin, vérifiable

1. Le paquet fabriqué par `outils/bibliotheque` depuis `gesco-bibliotheque\contenus` contient
   toutes les leçons et tous les sujets prévus au §3 ; son tableau de couverture donne, par
   classe et matière, le nombre de chapitres, de leçons, de sujets et de corrigés, égal à un
   décompte indépendant des fichiers.
2. Aucun texte du paquet ne contient « Source : », « .verif.py », « Image à chercher »,
   « SPEC », un numéro de question, un chemin de disque ni une adresse web : un essai lit tout le
   texte affichable du paquet et n'en trouve aucun.
3. Un paquet dont un octet a changé est refusé entier, et la bibliothèque déjà installée reste.
4. Sur une école neuve qui a les niveaux 7, 8, 9 et 12 (séries TSE et TSExp), le catalogue montre
   les 398 leçons et les 41 sujets ; sur une école qui n'a que le 2e cycle, il ne montre que la
   7e, la 8e, la 9e et les sujets du DEF.
5. Toute formule livrée s'affiche par KaTeX sans erreur, et se convertit pour l'impression : un
   essai convertit toutes les formules du paquet et n'en laisse aucune imprimée telle qu'écrite.
6. La plus grosse leçon livrée s'ouvre en moins de 2 secondes sur le poste principal et en moins
   de 3 secondes depuis un poste relié ; une recherche répond en moins d'une seconde.
7. Une matière retirée disparaît du catalogue, de la recherche et de l'impression par chapitre ;
   elle revient d'un geste avec ses versions de l'école ; un paquet mis à jour qui ajoute une
   leçon à cette matière ne la fait pas paraître.
8. Un maître affecté aux mathématiques de 9e écrit une version de la leçon « La racine carrée » ;
   il ne peut pas écrire une version d'une leçon de biologie ; sa version n'est visible des élèves
   (mode classe, PDF pour les élèves) qu'une fois publiée par la direction.
9. La direction donne au maître le droit de publier : il publie sa leçon suivante lui-même ; la
   direction retire ce droit : il ne peut plus.
10. L'historique d'une leçon montre chaque enregistrement, chaque soumission, chaque décision,
    avec qui et quand ; deux versions se comparent ligne à ligne ; « Revenir à la version livrée »
    rend visible le texte livré, et l'historique le garde.
11. Une mise à jour du paquet qui change le texte d'une leçon livrée ne change pas la version
    publiée par l'école ; une leçon que l'école n'a pas touchée prend le texte nouveau.
12. Deux personnes ouvrent le même brouillon ; la seconde qui enregistre reçoit « Cette leçon a été
    modifiée depuis que vous l'avez ouverte » et ne perd pas son texte (il reste dans l'éditeur).
13. Une image de 6 Mo est refusée avec un message clair ; une image de 4 Mo et un PDF de 15 Mo
    s'ajoutent ; après sauvegarde et restauration sur une école vide, ils sont là.
14. Une vidéo MP4 ajoutée se lit sur le poste principal et sur un poste relié ; supprimée du
    dossier, la leçon s'ouvre sans elle ; copiée vers un dossier puis reprise sur une autre
    école, elle se relit ; elle n'est pas dans la sauvegarde.
15. Une leçon, un sujet, un corrigé et un chapitre entier s'impriment en PDF, vérifiés en image
    comme les bulletins ; la version « sans les corrigés » n'a pas la section des corrigés.
16. Module éteint : la bibliothèque disparaît de la barre latérale, du tableau de bord et de la
    recherche ; les essais des phases précédentes passent sans changement ; rallumé, tout revient.
17. Une école en 1.2 qui installe la 2.0 trouve la bibliothèque allumée et ses écrans existants
    inchangés.

---

## 1. Périmètre

**Inclus** : format du paquet, outil de fabrication, signature et contrôles ; installation et mise
à jour du paquet ; module de l'école, droits, réglages ; catalogue, lecture d'une leçon, d'un
sujet, d'un corrigé, d'un document ; recherche plein texte ; mode classe (plein écran) ; retrait
et remise ; chapitres, leçons, documents et sujets ajoutés par l'école ; versions de l'école,
historique, comparaison, retour à la version livrée ; soumission, validation, renvoi, droit de
publier, relecture ; images, documents et vidéos de l'école, copie des vidéos vers une clé, deux
guides ; impression et PDF ; correspondance des disciplines ; école d'essai, manuel, notes de
version, essais.

**Exclu** :
- Le portail des élèves sur le Wi-Fi (lot 4), leur identité, leur suivi : le réglage du suivi est
  posé, il ne commande rien.
- Les quiz interactifs et les exercices notés (lot 3) : les questions de quiz des leçons livrées
  se lisent et s'impriment comme du texte, avec leurs réponses dans les corrigés.
- Les devoirs à rendre (lot 5).
- L'export des leçons d'une école vers une autre école : place réservée, après la 2.0 si une école
  le demande. La sauvegarde, elle, emporte tout ce que l'école a écrit.
- Les contenus en bambara : le champ `langue` est posé (`fr`), rien n'est livré.
- La mesure de la couverture du programme par les cours d'une école.
- Les mises à jour du paquet par Internet (lot C de la phase 11).

---

## 2. Ce que l'application possède déjà

Relevé fait le 15 novembre 2026 sur `developpement` (migrations 0001 à 0060), en lecture seule.

| Couture | Où | Ce qu'elle apporte au lot |
|---|---|---|
| Point de passage unique | `Application::executer` | Les commandes de la bibliothèque s'ajoutent comme celles des autres modules |
| Modules de l'école | `gesco-core/src/core/module.rs`, sept modules | Un huitième, `Bibliotheque` |
| Rôles et permissions | `RoleSysteme`, composition livrée version 16 | Quatre permissions nouvelles (§14) |
| Affectations des maîtres | `enseignant`, `affectation_enseignement` | Qui écrit quelles matières, dans quels niveaux |
| Référentiel | Niveaux (`7`, `8`, `9`, `12`…), séries (`TSE`, `TSExp`…), matières (`MATH`, `PC`, `SVT`…) aux codes livrés | Le repère commun des contenus livrés |
| Pièces dans la base chiffrée | `piece_depense`, `piece_dossier` : `contenu BLOB`, empreinte SHA-256 | Images et documents de l'école |
| Sauvegarde `.gsv` | Base et modèles sur mesure | Emporte le travail de l'école sans changer de format |
| Journal d'audit chaîné | `journal_audit` | Trace de chaque geste |
| Moteur de documents | `gesco-docs`, Typst 0.15.1, police Charis SIL, rendu SVG pour les aperçus | Impression des leçons ; il manque une police mathématique |
| Fichiers par le réseau local | `gesco-reseau`, 20 Mo par fichier, 32 Mo par appel | Images et documents depuis un poste relié ; les vidéos demandent une route par morceaux |
| Recherche plein texte | FTS5 compilé dans SQLite3 Multiple Ciphers (`vendor/libsqlite3-sys/build.rs`) | Recherche dans le texte des leçons |
| Signature de l'éditeur | `ed25519-dalek` 3.0.0, `outils/licence` | Signature du paquet, avec une clé à part |
| Compression | `flate2` 1.1.10 (archive de sauvegarde) | Compression du paquet |
| Frontend | React 19, TanStack Query, Tailwind ; aucune bibliothèque de Markdown ni de formules | KaTeX à ajouter (ADR « format des leçons ») |

---

## 3. Ce qui est livré

Relevé du 15 novembre 2026 dans `gesco-bibliotheque\contenus` (le dossier `_fusion-2026-11-13`
n'est pas livré).

| Classe | Matière (code de référence) | Leçons |
|---|---|---|
| 7e année | Mathématiques (`MATH`) | 32, 121 figures |
| 7e année | Physique-chimie (`PC`) | 28 |
| 7e année | Biologie (`SVT`) | 15 |
| 8e année | Mathématiques (`MATH`) | 30 sur 44 prévues, 50 figures |
| 8e année | Physique-chimie (`PC`) | 20 sur 23 prévues, 13 figures |
| 8e année | Biologie (`SVT`) | 15 |
| 9e année | Mathématiques (`MATH`) | 31 |
| 9e année | Physique-chimie (`PC`) | 26 |
| 9e année | Biologie (`SVT`) | 22 |
| 9e année | Français (`FR`) | 46 |
| 9e année | Histoire-géographie (`HG`) | 20 |
| 9e année | Anglais (`ANG`) | 41 |
| 9e année | ECM (`ECM`) | 22 |
| Terminale TSE | Mathématiques (`MATH`) | 29 sur 38 prévues (dont AN-09, voir T-123), 93 figures |
| Terminale TSE et TSExp | Physique-chimie (`PC`) | 20 sur 59 prévues, 23 figures |
| **Total** | | **398 leçons** |

Sujets officiels, chacun avec son corrigé écrit pour GESCO : **39 du DEF** (sessions 2021 à 2026,
sept matières) et **2 du BAC** (mathématiques 2025, TSE et TSExp). 285 figures SVG en tout ;
94 listes de faits à cocher pour la relecture (biologie, histoire-géographie, ECM, français…).

Ce qui accompagne les leçons et que le paquet traite (ADR « paquet de contenus ») :
- **notes de travail** : 71 leçons ont des lignes « Image à chercher », 9 une section « Figures
  qui manquent » ; elles deviennent des notes visibles du seul personnel qui écrit (« Image à
  ajouter ici : … »), jamais des élèves ni de l'impression ;
- **listes de faits à cocher** : jointes à leur leçon comme liste de relecture, visibles du
  personnel qui écrit et de la direction, imprimables à part ;
- **en-têtes** : l'état, la source et les liens de fichiers sont retirés du texte ; le titre, le
  classement, la nature, l'examen, la session et « programme à confirmer » passent au manifeste ;
- **non livrés** : `.verif.py`, `.fig.py`, `ETAT.md`, consignes de rédaction.

Les leçons des sciences de 9e encore en texte simple (79 le 14 novembre 2026) se livrent telles
quelles : leurs formules s'affichent comme du texte. Leur conversion en LaTeX est une question
au porteur (T-123).

---

## 4. Le paquet et son installation

### 4.1 Le paquet

Format et contrôles : ADR « paquet de contenus ». En bref : un fichier `.gbib`, un manifeste JSON
signé en Ed25519 par une clé de l'éditeur propre aux paquets, les fichiers compressés ; un seul
paquet aujourd'hui, `bibliotheque-mali`, version `2026.11.1`.

L'outil `outils/bibliotheque` (membre du dépôt, hors de la suite par défaut) fabrique le paquet et
son tableau de couverture ; il refuse un contenu sans chapitre ou sans nature, une image appelée
qui manque, une formule hors du sous-ensemble convertible, un texte affichable qui porte une
marque interdite. La clé privée reste hors du dépôt, comme celle des licences.

### 4.2 Installation

- **À l'installation de GESCO** : l'installeur dépose le paquet dans `bibliotheque\` du dossier de
  données ; à la première ouverture, GESCO vérifie la signature, remplit l'index
  (`chapitre_programme`, `contenu`, `recherche_bibliotheque`) et pose la correspondance des
  disciplines par code exact.
- **Mise à jour** : la direction choisit « Installer une mise à jour de la bibliothèque » et
  désigne un fichier `.gbib` sur une clé. GESCO vérifie la signature, la version (plus récente
  que celle installée ; une version plus ancienne est refusée avec « Cette bibliothèque est plus
  ancienne que celle installée »), la version minimale de GESCO et la place sur le disque, copie
  le fichier, puis rafraîchit l'index. Une copie interrompue laisse l'ancien paquet en place :
  le nouveau n'est activé qu'une fois copié et vérifié.
- **Ce qu'une mise à jour change** : les chapitres et contenus livrés, par leur clé ; un contenu
  qui n'est plus dans le paquet est marqué « plus livré » et reste lisible s'il a une version de
  l'école, sinon il quitte le catalogue. Elle ne touche jamais : les retraits, les chapitres et
  contenus de l'école, les versions, les relectures, la correspondance des disciplines, les
  droits de publier.
- **Depuis un poste relié** : le choix du fichier se fait sur le poste ; le paquet voyage par son
  contenu s'il fait 20 Mo ou moins, sinon l'écran demande de l'installer depuis le poste principal.
- **Après une restauration** sur un autre ordinateur : l'index se refait depuis le paquet installé
  par l'installeur ; si le paquet de la sauvegarde est plus récent que celui de l'ordinateur,
  l'écran le dit en une ligne et propose d'installer la mise à jour.
- **Paquet absent ou refusé** : la bibliothèque montre les seuls contenus de l'école ; aucun autre
  écran ne change.

---

## 5. Lire

### 5.1 Catalogue

- Arborescence : **niveau** (ceux que l'école a, actifs) → **série** au lycée → **matière**
  (libellé de l'école par la correspondance, sinon libellé de référence) → **partie** →
  **chapitre** → contenus, dans l'ordre du programme. Les chapitres et contenus de l'école se
  rangent parmi les livrés, à la place qu'elle leur a donnée.
- Chaque contenu montre son titre et sa nature : « Leçon », « Sujet du DEF 2024 », « Corrigé »,
  « Document ». Pour le personnel qui écrit, deux marques discrètes : « Version de l'école » (la
  version visible est celle de l'école) et la marque de relecture (T-116).
- Filtres : « Mes matières » (pour un maître : celles de ses affectations de l'année),
  « Version de l'école seulement », « En attente de validation » (pour qui valide).
- Le maître arrive par défaut sur « Mes matières » ; la direction sur tout.

### 5.2 Lecture

- Une leçon s'affiche depuis l'arbre de blocs que renvoie le noyau (ADR « format des leçons ») :
  titres, paragraphes, listes, tableaux, citations, formules par KaTeX, figures SVG, images et
  vidéos de l'école. Police de lecture : celle de l'application ; taille réglable par deux boutons
  (A−, A+), retenue par poste.
- Un sujet renvoie à son corrigé, et l'inverse, par un bouton.
- Un document de l'école (PDF) s'ouvre dans l'afficheur du système ; une image s'affiche dans la
  page.
- Barre de la leçon : Imprimer ou PDF (§11), Mode classe, et pour qui en a le droit : Écrire ma
  version, Historique, Retirer.
- **Mode classe** : la version visible en plein écran, grande police, sans barre ni menu, pour un
  projecteur ou un écran tourné vers la classe ; Échap pour sortir.
- Ce qui s'affiche : par défaut la **version visible** (§8.1). Le personnel qui écrit peut basculer
  sur « Version livrée » et, s'il en a un, sur son brouillon.

### 5.3 Recherche

Un champ en tête de la bibliothèque cherche dans les titres et le texte des versions visibles
(FTS5), sur les niveaux de l'école, sans les contenus retirés ; résultats groupés par niveau et
matière, avec le chapitre et un extrait. Accents et majuscules ignorés.

---

## 6. Retirer et remettre

Règles : ADR « retirer un contenu ».
- Bouton **Retirer** sur un contenu, un chapitre, une matière d'un niveau (avec sa série au lycée),
  un niveau ; « Retirer tout le paquet » dans les réglages de la bibliothèque. Une confirmation
  dit ce qui sera caché (« 31 leçons et 5 sujets ne seront plus montrés ; vous pourrez les
  remettre »).
- Liste **Retirés** : chaque retrait, son grain, qui, quand, et le bouton **Remettre**. Un contenu
  caché par deux retraits (lui-même et son chapitre) le dit : « Aussi caché par le retrait du
  chapitre … ».
- Les ajouts de l'école se retirent de la même façon.
- Seule la direction retire (`library:manage`).

---

## 7. Ajouter

- **Chapitre** : sous un niveau, une série au besoin et une matière de l'école ; titre, partie
  (choisie parmi celles de la matière ou nouvelle), place dans l'ordre (avant ou après un
  chapitre existant).
- **Leçon** : sous un chapitre, livré ou de l'école ; titre ; texte écrit dans l'éditeur (§8.3),
  qui propose au départ le plan des leçons livrées (L'essentiel, Les méthodes, Exemples résolus,
  Les erreurs à ne pas faire, Exercices, Corrigés, Quiz), que l'auteur garde ou efface.
- **Sujet** : sous un chapitre « Sujets du DEF » ou « Sujets du BAC » (créé au besoin) ; examen,
  série au lycée, session ; énoncé écrit, ou photos et PDF joints (décision du 14 novembre 2026 :
  les sujets que la bibliothèque n'a pas pu lire, l'école les ajoute) ; corrigé lié, facultatif.
- **Document** : un ou plusieurs fichiers (PDF, images) sous un chapitre, avec un titre et une
  phrase de présentation.
- Tout ajout naît **brouillon** et suit le circuit du §9. Un maître ajoute dans les niveaux et les
  matières de ses affectations ; la direction partout.

---

## 8. Améliorer une leçon

### 8.1 Version livrée, version de l'école, version visible

- La **version livrée** est le texte du paquet installé. Elle n'est jamais modifiée.
- Une **version de l'école** est un texte écrit dans GESCO, parti de la version livrée ou d'une
  version de l'école antérieure.
- La **version visible** est ce que voient les élèves (mode classe, PDF pour les élèves, plus tard
  le portail) et le personnel qui n'écrit pas : la dernière version publiée de l'école, sinon la
  version livrée.
- Une leçon ajoutée par l'école n'a pas de version livrée : sa version visible est sa dernière
  version publiée ; avant la première publication, elle n'est visible que du personnel qui écrit
  et de qui valide.

### 8.2 Le circuit d'une version

Brouillon → Soumise → Publiée, ou Renvoyée (retour au brouillon avec un commentaire).
- **Un brouillon à la fois par leçon.** Il appartient à son auteur ; un autre maître de la même
  matière peut le reprendre (« Reprendre ce brouillon »), ce qui s'écrit au journal.
- Chaque **enregistrement** du brouillon s'écrit au journal d'audit (avant et après) : c'est la
  trace demandée. Une version **soumise** ne change plus.
- **Revenir à la version livrée** : une version spéciale, sans texte, qui suit le même circuit ;
  publiée, elle rend visible le texte livré. L'historique garde tout.
- **Une mise à jour du paquet** ne change rien à la version visible de l'école. Dans l'éditeur et
  l'historique seulement, une ligne discrète dit « La bibliothèque a une version plus récente de
  cette leçon », avec un bouton Comparer ; l'auteur peut partir de la nouvelle version livrée
  pour un nouveau brouillon.

### 8.3 L'éditeur

- Deux colonnes : à gauche le texte (Markdown et LaTeX), à droite l'aperçu, rafraîchi une demi-
  seconde après la dernière frappe, rendu par la même commande que la lecture.
- Barre de boutons qui écrivent le balisage : Titre, Gras, Italique, Liste, Liste numérotée,
  Tableau, Formule dans la ligne, Formule seule, Image, Vidéo, Note pour le maître (une citation).
- Sous l'aperçu, seulement pour l'auteur : les formules que l'impression montrera telles qu'elles
  sont écrites (« Cette formule s'imprimera telle qu'elle est écrite : … »), et les notes « Image à
  ajouter ici » venues de la bibliothèque. Rien n'empêche d'enregistrer.
- Boutons : Enregistrer, Soumettre à la direction (ou Publier, pour qui en a le droit),
  Abandonner le brouillon (le brouillon passe à « abandonné », gardé dans l'historique).
- Conflit : si le brouillon a été enregistré ailleurs depuis son ouverture, l'enregistrement est
  refusé avec « Cette leçon a été modifiée depuis que vous l'avez ouverte » ; le texte reste dans
  l'éditeur, avec un bouton pour voir l'autre version.

### 8.4 Historique et comparaison

- Liste chronologique : versions (numéro, état, auteur, dates), décisions (qui a publié, renvoyé,
  avec le commentaire), relectures, retraits et remises de la leçon, installations de paquet qui
  l'ont changée.
- **Comparer** deux versions quelconques, livrée comprise : lignes retirées et ajoutées, côte à
  côte, par une comparaison ligne à ligne écrite dans le noyau (aucune dépendance).
- **Repartir de cette version** : crée un brouillon depuis une version ancienne.

---

## 9. Publier et relire

- **Soumettre** : l'auteur soumet son brouillon ; il paraît dans « En attente de validation » de
  la direction, avec un compteur sur l'entrée Bibliothèque pour qui valide.
- **Valider** (`library:publish`) : la direction ouvre la version soumise, la compare à la version
  visible, puis **Publie** ou **Renvoie** avec un commentaire. Publier rend la version visible
  aussitôt et passe la précédente à « remplacée ».
- **Droit de publier donné à un maître** : dans les réglages de la bibliothèque, la direction
  choisit un ou plusieurs maîtres nommément (« Peuvent publier eux-mêmes ») ; un maître ainsi
  désigné publie sans soumettre, **dans les niveaux et matières de ses affectations** (T-117) ;
  le retrait du droit vaut aussitôt. Chaque don et chaque retrait s'écrit au journal.
- **Relire** : sur toute version visible, qui a le droit de publier peut **Marquer relue**. La
  marque vise le texte relu (son empreinte) : si une autre version devient visible, ou si une
  mise à jour change le texte livré non modifié, la marque ne vaut plus pour le nouveau texte.
  La liste de relecture (faits à cocher) s'ouvre à côté de la leçon et s'imprime.
- La direction publie et relit sans passer par la soumission.

---

## 10. Images, documents et vidéos

Règles de stockage : ADR « stockage des contenus ».
- **Images** (SVG, PNG, JPEG ; 5 Mo au plus) et **documents** (PDF ; 20 Mo au plus) : ajoutés par
  le bouton Image ou à l'ajout d'un document, rangés dans la base chiffrée par leur empreinte,
  gardés une fois, dans la sauvegarde. Un SVG est nettoyé à l'ajout (aucun script, aucun lien
  externe) ; refusé s'il ne se nettoie pas.
- **Vidéos** (MP4) : bouton Vidéo dans l'éditeur ; le fichier est copié dans `videos\` du poste
  principal, nommé par son empreinte ; au-delà de la taille conseillée (T-121), un avertissement :
  « Cette vidéo est lourde : elle prendra de la place et pourra être lente sur les autres postes.
  Ajouter quand même ? ». Jamais de refus pour la taille.
- **Première vidéo** : une fenêtre dit une fois que les vidéos ne sont pas dans la sauvegarde et
  montre le bouton **Copier les vidéos vers une clé** ; le même bouton et **Reprendre des vidéos
  depuis une clé** sont dans les réglages de la bibliothèque. Reprendre ne copie que les fichiers
  dont l'empreinte est connue de l'école et absente du dossier.
- **Vidéo absente** : la leçon s'ouvre sans elle ; le personnel voit à sa place « Vidéo non
  disponible sur cet ordinateur » ; les élèves et l'impression ne voient rien.
- **Depuis un poste relié** : une vidéo se lit par morceaux sur une route du réseau local réservée
  aux postes appairés ; une vidéo ajoutée depuis un poste relié est envoyée au poste principal par
  morceaux, puis vérifiée par son empreinte.
- **Imprimer une leçon avec vidéo** : la vidéo devient une ligne « Vidéo : <titre> ».
- **Deux guides courts dans l'application**, ouverts depuis le bouton Vidéo et depuis l'aide :
  « Filmer une leçon avec un téléphone » (tenir le téléphone à l'horizontale, filmer le tableau et
  les mains plutôt que les visages, dix minutes au plus, lumière de face, son près du maître,
  enregistrer en MP4) et « Ajouter une vidéo à une leçon et la montrer en classe » (bouton Vidéo,
  mode classe, plein écran, copier vers une clé). Les mêmes textes sont dans le manuel.

---

## 11. Imprimer et enregistrer en PDF

- **Ce qui s'imprime** : une leçon, un sujet, un corrigé, une liste de relecture, un chapitre
  entier (ses leçons visibles dans l'ordre, non retirées). Toujours la **version visible**, sauf
  pour le personnel qui écrit, qui peut imprimer son brouillon (marqué « Brouillon » en filigrane).
- **Deux formes** pour une leçon ou un chapitre (T-120) : « Leçon complète » et « Sans les
  corrigés » (la section des corrigés et les réponses du quiz retirées, quand la leçon les a sous
  ses titres habituels).
- **Mise en page** : A4, police Charis SIL, formules par Typst avec la police New Computer Modern
  Math (ADR « format des leçons »), figures SVG à leur taille, tableaux sur la largeur, une leçon
  commence sur une page. En-tête : nom de l'école, niveau, matière ; pied : titre et numéro de
  page. Aucune autre mention (ni « rédigé pour GESCO », ni état de relecture).
- **Enregistrer en PDF** : boîte d'enregistrement du système (clé, dossier, bureau), nom proposé
  « Classe — Matière — Titre.pdf ». Depuis un poste relié, le PDF revient par son contenu.
- Une formule hors du sous-ensemble s'imprime telle qu'elle est écrite ; jamais de refus.

---

## 12. Données nouvelles

### 12.1 Tables

Une migration, la suivante libre (0061 aujourd'hui, ou la suivante si la sauvegarde hors de
l'école la prend avant). Détail et règles : ADR « tables de la bibliothèque » et « repère
commun ».

| Table | En bref |
|---|---|
| `bibliotheque_paquet` | Paquets installés |
| `chapitre_programme`, `chapitre_serie` | Chapitres livrés et de l'école, séries d'un chapitre |
| `contenu` | Leçons, sujets, corrigés, documents |
| `version_contenu` | Versions de l'école et leur circuit |
| `relecture_contenu` | Marques « relue » |
| `retrait_bibliotheque` | Retraits et remises |
| `fichier_bibliotheque` | Images et documents de l'école, dans la base chiffrée |
| `video_bibliotheque` | Vidéos de l'école (le fichier est hors de la base) |
| `droit_publication` | Maîtres qui peuvent publier eux-mêmes |
| `correspondance_discipline` | Sous quelle matière de l'école se range une discipline de référence |
| `recherche_bibliotheque` | Index plein texte FTS5 |

Aucune n'a d'année scolaire ; aucune ligne ne se supprime (déclencheurs).

### 12.2 Réglages

- `module.bibliotheque` (booléen, allumé) ;
- `bibliotheque.suivi_eleves` (booléen, allumé ; sert au lot 4) ;
- `bibliotheque.taille_video_conseillee` (entier, en Mo ; T-121).

### 12.3 Fichiers

`bibliotheque\` (paquets) et `videos\` (vidéos) dans le dossier de données du poste principal.
Le désinstalleur ne les efface pas, comme la base (risque de livraison à vérifier à l'étape 10).

---

## 13. Règles

1. **Le livré ne se modifie jamais.** Ni par l'école, ni par une mise à jour de ses versions à
   elle. Une mise à jour remplace le paquet ; elle ne touche ni retraits, ni ajouts, ni versions,
   ni relectures, ni correspondance, ni droits.
2. **Rien ne se supprime.** Retirer cache ; abandonner un brouillon le garde ; un contenu « plus
   livré » reste lisible tant que l'école en a une version.
3. **Les élèves ne voient que des versions visibles** : livrées non retirées, ou publiées par
   l'école. Jamais un brouillon, une version soumise ou renvoyée, une note pour le maître, une
   liste de relecture, une marque de relecture.
4. **Ne jamais bloquer.** Une formule mal formée s'affiche telle qu'elle est écrite ; une vidéo
   absente laisse la leçon s'ouvrir ; une vidéo lourde s'ajoute ; un paquet absent laisse
   l'école travailler avec ses ajouts ; une matière sans correspondance garde ses contenus sous
   le libellé de référence.
5. **Ne pas faire remarquer ce qui manque.** Aucun chapitre vide livré ; aucun compteur de leçons
   manquantes ; aucune mention « incomplet » ; les notes de travail de la bibliothèque ne sont
   vues que du personnel qui écrit.
6. **Aucune source affichée.** Ni site, ni livre, ni auteur, ni date de rédaction, ni « vérifié
   par un programme ». Un énoncé officiel dit ce qu'il est : examen, matière, session.
7. **GESCO ne prétend jamais qu'une leçon a été relue** si l'école ne l'a pas marquée relue.
8. **Chaque geste est tracé** au journal d'audit : enregistrement, soumission, publication,
   renvoi, retour à la version livrée, relecture, retrait, remise, droit de publier donné ou
   retiré, paquet installé, correspondance changée, vidéo ajoutée.
9. **Un maître écrit dans ses matières** : celles de ses affectations de l'année en cours, à
   travers la correspondance des disciplines. Une affectation retirée lui retire l'écriture, pas
   ses versions passées.
10. **Le texte d'une leçon est filtré par le noyau** : aucun HTML interprété, aucun lien vers
    l'extérieur, aucun chemin de fichier ; une image ou une vidéo n'est appelée que par une
    empreinte connue de l'école ou un nom de fichier du paquet.
11. **Module éteint, rien ne change ailleurs** : aucune commande de la bibliothèque n'est
    proposée, les données restent.

---

## 14. Droits

Quatre permissions nouvelles, dans la composition livrée suivante (17 aujourd'hui, ou la suivante
si la sauvegarde hors de l'école en ajoute avant), écrites dans `docs/matrice-permissions.md` par
l'essai qui la génère :

| Permission | Ce qu'elle permet | Rôles à la livraison |
|---|---|---|
| `library:read` | Lire la bibliothèque, chercher, mode classe, imprimer la version visible | Administrateur, Directeur, Directeur des études, Secrétaire, Enseignant, Surveillant général, Consultation seule |
| `library:edit` | Écrire des brouillons, ajouter chapitres, leçons, sujets, documents, images et vidéos, soumettre ; voir les versions livrées, les notes pour le maître et les listes de relecture. Portée : les matières et niveaux de ses affectations pour un Enseignant ; tout pour les autres rôles | Administrateur, Directeur, Directeur des études, Enseignant |
| `library:publish` | Valider, publier, renvoyer, revenir à la version livrée, marquer relue, donner et retirer le droit de publier | Administrateur, Directeur, Directeur des études |
| `library:manage` | Retirer et remettre, installer un paquet, régler la correspondance des disciplines et le suivi, copier et reprendre les vidéos | Administrateur, Directeur |

Le droit de publier donné nommément à un maître n'est pas une permission de rôle : c'est une
ligne de `droit_publication`, lue en plus de la portée de ses affectations. Le Caissier, le
Comptable et le Gestionnaire RH n'ont aucune permission de la bibliothèque à la livraison ;
l'administrateur peut la leur donner.

---

## 15. Écrans

- **Bibliothèque** (barre latérale, sous Pédagogie) : recherche en tête ; catalogue à gauche,
  contenu à droite ; filtres ; pour qui valide, l'onglet « En attente de validation » avec son
  compteur.
- **Leçon** : lecture, barre de la leçon, bascule version visible, livrée, brouillon.
- **Éditeur** : deux colonnes, barre de boutons, notes de l'auteur sous l'aperçu.
- **Historique** : liste, comparaison côte à côte.
- **Retirés** : liste et remise.
- **Réglages de la bibliothèque** (onglet des réglages, `library:manage`) : paquet installé et sa
  version, Installer une mise à jour, Retirer tout le paquet ; correspondance des disciplines ;
  « Peuvent publier eux-mêmes » (visible aussi de qui a `library:publish`) ; suivi des élèves ;
  taille conseillée des vidéos ; Copier les vidéos vers une clé, Reprendre des vidéos.
- **Deux guides** (fenêtre d'aide), **mode classe** (plein écran).
- **Tableau de bord** : pour qui valide, une tuile « Leçons à valider » quand il y en a ; rien
  d'autre.
- Tous les écrans suivent le skill de lisibilité du projet : contrastes, focus visible, clavier,
  libellés ; la lecture d'une leçon reste lisible à 200 % d'agrandissement.

---

## 16. Ordre de réalisation

Des étapes courtes ; une session en fait plusieurs, commite et pousse après chacune, et passe la
main au-delà d'environ 150 000 jetons (règles du 14 novembre 2026). Une seule session compile à
la fois. Pendant les étapes : fmt, clippy, tsc, ESLint ; tous les essais à l'étape 10.

1. **Format des leçons dans le noyau.** `pulldown-cmark` ajouté ; analyse du Markdown restreint
   en arbre de blocs, filtre (pas de HTML, pas de lien externe) ; conversion LaTeX vers les
   mathématiques de Typst pour le sous-ensemble relevé ; police New Computer Modern Math dans
   `gesco-docs/polices`. Essais écrits : toutes les leçons et tous les sujets de la bibliothèque
   s'analysent, toutes leurs formules se convertissent.
2. **Paquet.** Format `.gbib` (lecture et vérification dans le noyau) ; outil
   `outils/bibliotheque` : en-têtes, retraits de texte, notes pour le maître, listes de
   relecture, contrôles et refus, signature, tableau de couverture ; paire de clés des paquets
   (clé privée hors du dépôt) ; premier paquet `2026.11.1` fabriqué hors du dépôt.
3. **Tables, module, droits.** Migration, déclencheurs, module `Bibliotheque` et réglages,
   quatre permissions et composition des rôles, mise à niveau d'une base 1.2 et 2.0.
4. **Installer.** Installation à la première ouverture, mise à jour depuis un fichier, refus d'un
   paquet abîmé ou plus ancien, activation après copie, index et correspondance, réindexation
   après restauration ; installeur qui dépose le paquet.
5. **Lire.** Catalogue, filtres, lecture d'une leçon (KaTeX ajouté au frontend, polices locales),
   sujet et corrigé, document, recherche FTS5, mode classe, taille du texte ; depuis un poste
   relié.
6. **Retirer.** Cinq grains, liste des retraits, remise, effet sur catalogue, recherche et
   impression par chapitre ; réglages de la correspondance des disciplines.
7. **Écrire.** Éditeur et aperçu, brouillon unique et compteur de révision, chapitres, leçons,
   sujets et documents de l'école, historique, comparaison ligne à ligne, repartir d'une version,
   retour à la version livrée, ligne « version plus récente » après une mise à jour.
8. **Publier.** Soumettre, valider, renvoyer, compteur et tuile, droit de publier nommé, portée du
   maître, relecture et liste de relecture.
9. **Images, documents, vidéos, impression.** Images et documents dans la base, nettoyage des SVG,
   bornes ; vidéos : dossier, route par morceaux dans `gesco-reseau`, avertissement de taille,
   première vidéo, copie vers une clé et reprise, vidéo absente ; les deux guides ; modèle Typst
   de la leçon, du sujet, du corrigé, de la liste de relecture et du chapitre, forme « sans les
   corrigés ».
10. **Essais et livraison.** Suite Rust complète, vitest, knip, Playwright de la bibliothèque
    (catalogue, lecture, recherche, retrait, écriture, validation, PDF, vidéo) avec captures
    montrées au porteur ; essais des critères du §0.3 ; mesures de temps (critère 6) ; skill
    `livraison` sur l'installeur (paquet déposé, dossiers `bibliotheque\` et `videos\` gardés à
    la désinstallation) ; école d'essai refaite avec quelques versions de l'école, une leçon en
    attente, un retrait, une vidéo courte ; manuel, notes de version de la 2.0.0, essais à la
    main ; validation du porteur.

Les étapes 5 et 6 ne dépendent pas l'une de l'autre ; les vidéos (étape 9) peuvent se faire en
dernier sans rien retirer au reste.

---

## 17. Questions au porteur du projet

Numérotées de T-116 à T-123, dans la plage T-116 à T-129 que la spécification de la sauvegarde
hors de l'école laisse à la phase 12. Chacune a une
recommandation ; le lot peut commencer sur les recommandations, les étapes concernées étant
indiquées.

### T-116 — Où se voit « pas encore relu »

Aucune leçon livrée n'a été relue par un enseignant. GESCO doit-il le montrer, et à qui ?
- a) au personnel qui écrit et à la direction seulement, dans le catalogue et l'historique ;
- b) aussi aux élèves, sur la leçon et dans le PDF ;
- c) nulle part.

*Recommandation* : a). La direction sait ce qu'elle a à relire ; un élève ou un parent n'a pas à
douter d'une leçon que l'école a laissée visible. Étape 5.

### T-117 — Le droit de publier d'un maître : quelle portée

Quand la direction donne à un maître le droit de publier lui-même, ce droit vaut :
- a) pour toutes les matières et classes où il enseigne ;
- b) pour une ou plusieurs matières choisies par la direction.

*Recommandation* : a). Un seul geste, et la portée suit ses affectations de l'année ; b) peut
venir plus tard si une école le demande. Étape 8.

### T-118 — Une leçon livrée non relue : visible des élèves tout de suite ?

- a) oui : la bibliothèque est utilisable dès l'installation ; la direction retire ce qu'elle ne
  veut pas ;
- b) non : une leçon livrée n'est visible des élèves qu'une fois marquée relue ou publiée.

*Recommandation* : a). C'est ce que veut une bibliothèque livrée prête ; b) laisserait 398 leçons
invisibles tant que personne ne les a relues, et le PDF pour les élèves vide. Étape 5.

### T-119 — Le programme entier, chapitres vides compris

La bibliothèque ne livre que les chapitres qui ont une leçon ou un sujet (ADR « programme
officiel »). Faut-il aussi livrer les chapitres du programme qui n'en ont pas encore, pour que
l'école les voie et y ajoute ses cours ?

*Recommandation* : non pour la 2.0. Les intitulés viendraient d'un relevé du programme qu'aucun
enseignant n'a relu (dont 339 lignes de langues non contrôlées), et des matières entières
s'afficheraient vides. L'école ajoute ses chapitres où elle veut. Étape 2.

### T-120 — Ce que le PDF « pour emporter » contient

- a) deux formes au choix : « Leçon complète » et « Sans les corrigés » ;
- b) une seule forme, la leçon complète.

*Recommandation* : a). La leçon complète sert à l'élève qui révise seul à la maison ; la forme sans
corrigés sert au maître qui donne les exercices à chercher. Étape 9.

### T-121 — La taille conseillée d'une vidéo

Au-delà, GESCO avertit sans refuser. Quelle taille ?

*Recommandation* : 200 Mo, soit une vingtaine de minutes filmées par un téléphone en qualité
moyenne ; réglable par la direction. Étape 9.

### T-122 — Le suivi des élèves avant le portail

Le réglage du suivi (T-91) est posé dans ce lot mais ne sert qu'au lot 4. Faut-il le montrer dès
maintenant dans les réglages de la bibliothèque ?

*Recommandation* : oui, allumé, avec une ligne neutre « Le suivi servira quand les élèves liront
les cours sur leurs appareils ». La direction le connaît avant d'en avoir besoin. Étape 3.

### T-123 — Deux reprises de la bibliothèque avant le premier paquet

La rédaction est arrêtée sur décision du porteur (14 novembre 2026). Deux points touchent le
premier paquet :
- 79 leçons de sciences de 9e sont encore en texte simple : leurs formules s'affichent comme du
  texte, pas par KaTeX ;
- la leçon AN-09 des mathématiques de TSE a un programme de vérification qui s'arrête sur une
  erreur : ses résultats ne sont pas tous recalculés (exigence E-16).

Que fait-on ?
- a) une session courte convertit les 79 leçons en LaTeX (en relançant leurs vérifications) et
  reprend AN-09 avant le premier paquet ;
- b) on livre les 79 leçons telles quelles, et AN-09 n'est pas livrée tant qu'elle n'est pas
  reprise ;
- c) on livre tout tel quel.

*Recommandation* : a), en parallèle des étapes 1 à 4, qui n'en dépendent pas ; à défaut b). c)
livrerait une leçon dont un résultat n'est pas vérifié. Étape 2.

---

## 18. Journal des révisions

- v1.0, 15 novembre 2026 : première rédaction, d'après `docs/plan-phase-12.md`, les décisions du
  porteur des 11 au 15 novembre 2026 et l'état de la bibliothèque relevé le même jour ; huit ADR
  proposés ; questions T-116 à T-123.
