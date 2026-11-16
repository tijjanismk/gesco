# SPEC_PHASE_12_LOT_3.md — Quiz et exercices

**Projet** : ERP de gestion scolaire — Tauri 2 + Rust + React/TypeScript
**Phase** : 12 — e-learning ; **lot 3** : les quiz des leçons deviennent interactifs, sur le
portail, sur les postes et en classe ; quiz écrits par l'école ; révision d'un chapitre ; quiz
sur papier ; résultats dans le suivi ; version 2.0.0
**Statut** : v1.0 — proposée le 16 novembre 2026, à soumettre au porteur du projet avant toute
ligne de code ; quatre questions au §13, chacune avec une recommandation
**Documents parents** : `docs/plan-phase-12.md` (§0, §0 bis E-6 et E-13, §3.5, §4 T-91 et T-94),
`SPEC_PHASE_12_LOT_1.md`, `SPEC_PHASE_12_LOT_4.md`, `docs/orchestration.md` (passage de relais
du 16 novembre 2026)
**ADR proposés avec ce lot** (`docs/adr/`, 16 novembre 2026) : `format-des-quiz`,
`tables-des-lots-2-a-5` ; s'appuie sur `charge-du-portail-des-eleves` et `identite-de-l-eleve`
**Ordre des lots** : 2, 4, puis ce lot 3, puis 5 — ce lot suppose le portail du lot 4 fait.
**Date** : 16 novembre 2026

---

## 0. Objet et critère de fin

### 0.1 Ce que produit ce lot

Chaque leçon livrée finit par un quiz de cinq questions, écrit à neuf pour GESCO, dont chaque
mauvaise réponse correspond à une vraie erreur d'élève, expliquée. Aujourd'hui ces quiz se lisent
comme du texte. À la fin du lot :

- l'élève **passe le quiz** d'une leçon sur le portail : une question par écran, la correction et
  l'explication tout de suite (T-139), son score à la fin, autant d'essais qu'il veut ;
- il **révise un chapitre** : dix questions tirées des quiz des leçons du chapitre (T-141) ;
- sur un poste de l'école, le maître ou l'élève **s'entraîne** sans nom ; en **mode classe**, le
  maître projette une question, la classe répond à voix haute, il montre la réponse ;
- le maître ou la direction **écrit un quiz** : dans une leçon, ou comme quiz d'un chapitre ; il
  **importe** des questions depuis Excel ;
- le **quiz sur papier** s'imprime : questions seules, et feuille des réponses pour le maître ;
- le maître voit dans le suivi **qui a fait quel quiz**, le score du premier essai et du meilleur,
  et les questions les plus manquées de sa classe (T-142) ; aucun score n'entre au bulletin
  (T-140).

### 0.2 Décisions déjà prises qui s'appliquent

- **Les quiz sont écrits à neuf, jamais tirés des anciennes bases** (décision du porteur ; T-94
  sans objet) ; ni les 15 129 questions au gabarit, ni aucune autre question collectée n'entre.
- **Plus aucun cours n'est écrit ni repris** dans les sessions de code (15 novembre 2026) : ce lot
  rend interactifs les quiz qui existent, sans les réécrire.
- **T-91** : le suivi est allumé, réglable, ne bloque jamais. **E-13** : aucune note n'entre au
  bulletin sans un geste du maître.
- **T-116** : aucun texte ne dit qu'un quiz ou une leçon a été relu.
- « La flexibilité sans incohérence » ; aucune source affichée.
- Méthode : fmt, clippy, tsc et ESLint à chaque étape ; tous les essais à la dernière.

### 0.3 Critère de fin, vérifiable

1. **Les quiz livrés sont reconnus** : sur le paquet, l'analyse reconnaît au moins 1 950 des 1 990
   questions des 398 leçons (relevé du 16 novembre 2026 : 392 leçons à la forme habituelle) ; le
   tableau de couverture du paquet donne, par classe et matière, le nombre de questions reconnues,
   égal à un décompte indépendant ; les leçons dont une question n'est pas reconnue sont listées
   pour l'intégration des cours.
2. **Le score est juste** : essais du calcul pour chaque sorte de question (choix unique, plusieurs
   bonnes réponses, vrai ou faux, réponse courte, appariement), avec les cas limites : aucune
   réponse, réponse courte « 3.5 » pour « 3,5 », majuscules et accents, espaces, appariement
   partiel (faux), choix multiples incomplets (faux).
3. **Rien ne bloque** : une question sans réponse ou avec une réponse qui n'existe pas reste du
   texte, et l'auteur seul le lit sous l'aperçu ; un quiz sans aucune question reconnue s'affiche
   comme avant.
4. **Portail** : un élève entré avec son code passe le quiz de « La racine carrée » ; il voit
   chaque correction avec son explication et son score à la fin ; sa tentative est dans le suivi
   de son maître dans les dix secondes ; un élève sans code, ou suivi éteint : le quiz se passe,
   rien n'est écrit.
5. **Versions** : la direction publie une version de l'école qui change la question 2 ; les
   tentatives passées gardent leur score et la question telle qu'elle était posée ; les questions
   les plus manquées ne mêlent pas les deux versions.
6. **Réviser le chapitre** : les dix questions viennent des seules leçons visibles et non retirées
   du chapitre, pour la classe de l'élève ; un chapitre qui en a moins de dix les donne toutes.
7. **Poste et mode classe** : un quiz se passe de bout en bout sur le poste principal et sur un
   poste relié, sans rien écrire ; en mode classe, « Montrer la réponse » affiche la bonne réponse
   et l'explication.
8. **Écrire** : le bouton Question écrit le gabarit ; un quiz de chapitre ajouté par l'école suit
   le circuit de publication ; un import Excel de vingt lignes, dont deux fausses, écrit dix-huit
   questions dans un brouillon et dit, ligne par ligne, pourquoi les deux autres n'y sont pas.
9. **Papier** : « Quiz seul » et « Réponses du quiz » s'impriment pour une leçon et pour un
   chapitre, vérifiés en image ; « Quiz seul » ne contient aucune réponse.
10. **Bulletin** : aucune tentative ne change aucune note ni aucun bulletin (essai sur l'école
    d'essai, bulletins recalculés avant et après).
11. **Migration** : la reconstruction de `contenu` garde lignes, index, déclencheurs et recherche
    (un mot trouvé avant l'est après).
12. Module ou portail éteint : les essais des phases précédentes passent sans changement.

---

## 1. Périmètre

**Inclus** : analyse des quiz ; quiz de chapitre ajouté par l'école ; bouton Question ; import
Excel ; passer un quiz sur le portail, sur un poste, en mode classe ; réviser un chapitre ; quiz
sur papier ; tentatives et résultats dans le suivi ; migration ; essais, manuel.

**Exclu** :
- Les exercices notés et la correction des exercices ouverts : les exercices et leurs corrigés se
  lisent (corrigés repliés sur le portail, lot 4) ; le travail noté passe par les devoirs (lot 5).
- Le report d'un score de quiz au bulletin (T-140).
- Une banque de questions partagée et les examens blancs tirés au hasard (ADR « format des
  quiz », conditions de reconsidération).
- La réécriture des six leçons dont le quiz n'a pas la forme habituelle : elle relève de
  l'intégration des cours, après le code de la phase 12.
- Toute question tirée des anciennes bases.

---

## 2. Ce que l'application possède déjà

| Couture | Où | Ce qu'elle apporte au lot |
|---|---|---|
| Analyse des leçons | `pulldown-cmark`, arbre de blocs du noyau (lot 1) | Un bloc « quiz » de plus |
| Section « Réponses » | `Lecon::sans_les_corriges` (lot 1, étape 9) | La forme « sans les corrigés » retire déjà les réponses |
| Circuit des versions | Brouillon, soumission, publication, historique, comparaison | Un quiz suit sa leçon ; un quiz de chapitre suit le même circuit |
| Mode classe | Plein écran du lot 1 | Une question à la fois, « Montrer la réponse » |
| Portail des élèves | Lot 4 : routes, session, file d'écritures groupées, suivi | Routes du quiz, tentatives, résultats |
| Imports Excel | `calamine` 0.36.1, analyse ligne par ligne des imports existants | Import des questions |
| Impression | Modèles Typst des leçons, KaTeX et conversion des formules | « Quiz seul », « Réponses du quiz » |
| Outil du paquet | `outils/bibliotheque`, tableau de couverture | Compte des questions reconnues |

---

## 3. La convention des quiz

Règles complètes : ADR « format des quiz ». En bref, ce qu'un maître écrit (et ce que les leçons
livrées ont déjà) :

```
**Question 1.** √(16 + 9) est égal à :
A) 7  B) 5  C) 25  D) 12

**Question 2.** Pour tout nombre a, √(a²) = a.
A) Vrai  B) Faux

**Question 3.** Combien font 7 / 2 ?

### Réponses

1. **B.** √25 = 5. La réponse 7 vient de √16 + √9 : c'est le piège de la somme.
2. **B.** Pour a = −3, √9 = 3 et non −3 : √(a²) est la valeur absolue de a.
3. **« 3,5 » ou « 7/2 ».** On partage 7 en deux.
```

Plusieurs bonnes réponses : « **A, C.** » ; appariement : choix « A) … » et « 1) … », réponse
« **A-2, B-1, C-3.** ». Une question vaut un point ; plusieurs réponses ou un appariement ne
valent le point que s'ils sont entièrement justes.

---

## 4. Écrire un quiz

- **Dans une leçon** : la section « Quiz » ; le bouton **Question** de l'éditeur écrit le gabarit
  d'une question et, s'il n'y en a pas, la sous-section « Réponses ». Sous l'aperçu, l'auteur
  voit le quiz tel que l'élève le passera (« Essayer ce quiz ») et les questions qui ne seront pas
  proposées, avec la raison.
- **Quiz d'un chapitre** : « Ajouter un quiz » sous un chapitre ; contenu de nature « Quiz », sans
  autre section ; même circuit que les leçons de l'école (brouillon, soumission, publication,
  portée par classe du lot 2).
- **Importer depuis Excel** : un modèle à télécharger (colonnes : question, choix A à F, bonne
  réponse, explication) ; chaque ligne lue est écrite dans le brouillon selon la convention ; les
  lignes refusées sont listées avec leur raison (« Ligne 7 : la bonne réponse D n'est pas parmi
  les choix »), comme les imports existants.
- **Quiz livrés** : rien à écrire ; ils sont reconnus dans le texte du paquet.

---

## 5. Passer un quiz

- **Sur le portail** (écran 6 du portail) : depuis une leçon (« Faire le quiz ») ou un chapitre
  (« Réviser le chapitre ») ; une question par écran, grande police, formules et figures ; choix
  par de grands boutons, réponse courte dans un champ, appariement par des listes à choisir ;
  « Vérifier », puis « Juste » ou « Pas tout à fait », la bonne réponse et l'explication
  (T-139) ; « Question suivante » ; à la fin, « 4 sur 5 », la liste des questions avec un signe, et
  « Recommencer ». Aucun chronomètre, aucun classement.
- **Sur un poste de l'école** : bouton « S'entraîner » dans la barre de la leçon ; même
  déroulement, rien n'est écrit, aucun nom demandé.
- **En mode classe** : la question en grand ; « Montrer la réponse » affiche la bonne réponse et
  l'explication ; flèches pour avancer.
- **Réviser le chapitre** (T-141) : dix questions tirées au hasard parmi les questions reconnues
  des leçons visibles et non retirées du chapitre, pour la classe de l'élève ; toutes s'il y en a
  moins de dix. Sur le portail et sur un poste.

---

## 6. Quiz sur papier

Dans « Imprimer ou PDF » d'une leçon, d'un quiz de chapitre et d'un chapitre entier, deux formes de
plus :
- **Quiz seul** : les questions numérotées et leurs choix, une ligne pour la réponse courte, sans
  aucune réponse ; en-tête de l'école, classe, matière, « Nom : ……… ».
- **Réponses du quiz** : pour le maître, la bonne réponse et l'explication de chaque question.
Les questions non reconnues s'impriment telles qu'elles sont écrites.

---

## 7. Résultats dans le suivi

- **Ce qui s'écrit** (élève entré avec son code, suivi allumé, sur le portail seulement) : une
  tentative par quiz passé jusqu'au bout, avec l'empreinte de la version, chaque réponse donnée et
  si elle est juste ; par la file d'écritures groupées du lot 4. Un quiz abandonné en route n'est
  pas écrit.
- **Ce que voit le maître** (onglet « Suivi des élèves », colonne « Quiz » ; T-142) : par quiz,
  combien d'élèves l'ont fait, le score moyen du premier essai, les trois questions les plus
  manquées (avec leur énoncé, version par version) ; par élève, le score du premier essai et le
  meilleur. Au grain « par classe seulement », les nombres sans noms.
- **Jamais au bulletin** ; jamais de classement ; rien n'est montré à un autre élève.

---

## 8. Données nouvelles

Migration **0065** (ou la suivante libre), ADR « tables des lots 2 à 5 » :

| Table | En bref |
|---|---|
| `contenu` (reconstruite) | Nature `quiz` en plus ; lignes, index, déclencheurs et index plein texte gardés |
| `tentative_quiz` | Élève, année, contenu (ou chapitre pour une révision), empreinte de la version, origine, commencée le, finie le, score, sur |
| `reponse_quiz` | Tentative, contenu, empreinte, numéro de la question, réponse donnée, juste |

Aucun réglage nouveau (le suivi et son grain sont ceux du lot 4).

---

## 9. Règles

1. **Les quiz sont écrits à neuf** ; aucune question collectée n'entre, ni par le paquet, ni par
   un import de l'éditeur.
2. **Un quiz suit sa leçon** : même version, même circuit, même retrait, même portée.
3. **Ne jamais bloquer** : une question mal écrite reste du texte ; un quiz se passe sans code ;
   le serveur chargé met la tentative en file, sans erreur pour l'élève.
4. **Un score ne juge pas** : pas de chronomètre, pas de classement, pas de bulletin.
5. **Les résultats passés restent justes** : une tentative garde la question telle qu'elle a été
   posée.
6. **Chaque geste du personnel est tracé** ; une tentative d'élève ne va pas au journal d'audit
   (elle est sa propre trace).

---

## 10. Droits

Aucune permission nouvelle : écrire et importer relèvent de `library:edit` ; passer un quiz sur un
poste, de `library:read` ; voir les résultats, de `library:follow` (lot 4) ; publier un quiz de
chapitre, de `library:publish` ou du droit de publier donné à un maître.

---

## 11. Écrans

- **Éditeur** : bouton Question ; « Essayer ce quiz » ; « Importer des questions depuis Excel ».
- **Leçon** (bureau) : « S'entraîner » ; en mode classe, navigation des questions.
- **Chapitre** : « Ajouter un quiz », « Réviser le chapitre ».
- **Portail** : écran du quiz, écran du score ; « Faire le quiz » sur la leçon, « Réviser le
  chapitre » sur le chapitre ; dans « Mon avancement », les derniers scores de l'élève.
- **Suivi des élèves** : colonne et détail des quiz.
- Lisibilité : skill du projet ; les boutons de réponse font au moins 44 pixels de haut ; juste et
  faux ne se distinguent jamais par la couleur seule (signe et mot).

---

## 12. Ordre de réalisation

Des étapes courtes ; une session en fait plusieurs, commite et pousse après chacune, passe la main
au-delà d'environ 150 000 jetons. Une seule session compile.

1. **Analyse des quiz** dans le noyau : bloc « quiz », cinq sortes, calcul du score ; essais écrits
   sur toutes les leçons et tous les sujets de la bibliothèque (non lancés) ; compte dans le
   tableau de couverture de l'outil du paquet et liste des leçons à reprendre à l'intégration.
2. **Migration 0065** : reconstruction de `contenu`, tables des tentatives ; essai de la
   reconstruction.
3. **Écrire** : bouton Question, « Essayer ce quiz », notes de l'auteur, quiz de chapitre, import
   Excel.
4. **Passer** : composant de quiz commun au bureau et au portail ; « S'entraîner » sur un poste ;
   mode classe ; routes du portail (`/eleve/api/quiz/{cle}`, `/eleve/api/revision/{chapitre}`,
   `/eleve/api/tentative`) ajoutées à la liste et à son essai ; réviser un chapitre.
5. **Papier** : formes « Quiz seul » et « Réponses du quiz » des modèles Typst.
6. **Suivi** : tentatives par la file groupée, colonne et détail des quiz, grain par classe.
7. **Essais et livraison du lot** : suite Rust complète, vitest, knip ; critères du §0.3 ;
   Playwright d'ensemble (écrire un quiz, le passer sur le portail en écran de 360 pixels, le voir
   dans le suivi, l'imprimer) avec captures montrées au porteur ; manuel (« Les quiz ») ; notes de
   version ; école d'essai refaite avec quelques tentatives.

Les étapes 3 et 5 ne dépendent pas de l'étape 4.

---

## 13. Questions au porteur du projet

Numérotées T-139 à T-142. Chacune a une recommandation ; le lot peut commencer sur les
recommandations.

### T-139 — La correction : après chaque question ou à la fin ?

- a) après chaque question, avec l'explication, puis le score à la fin ;
- b) seulement à la fin, toutes les corrections d'un coup.

*Recommandation* : a). Les explications des quiz livrés nomment l'erreur de l'élève : elles servent
le plus au moment où il vient de la faire. C'est ce que T-91 appelle « quiz corrigés tout de
suite ». Étape 4.

### T-140 — Un score de quiz au bulletin ?

- a) jamais : le quiz est un entraînement, refait autant de fois qu'on veut ;
- b) le maître peut reporter un score, comme une note de devoir (lot 5).

*Recommandation* : a). Un élève qui sait que le quiz compte ne recommence plus et cherche la
réponse chez son voisin ; la note du bulletin passe par les devoirs (lot 5), qui ont un geste du
maître et une copie. Étape 6.

### T-141 — Réviser un chapitre

Faut-il proposer « Réviser le chapitre » : dix questions tirées des quiz des leçons du chapitre ?
- a) oui, sur le portail et sur les postes ;
- b) non : seulement le quiz de chaque leçon.

*Recommandation* : a). Rien à écrire de plus, et c'est l'usage le plus utile avant un devoir ou le
DEF. Étape 4.

### T-142 — Ce que le maître voit des quiz

- a) par élève, le score du premier essai et le meilleur ; par quiz, la moyenne du premier essai et
  les trois questions les plus manquées ;
- b) seulement le meilleur score de chaque élève.

*Recommandation* : a). Le premier essai dit ce que l'élève savait ; le meilleur, ce qu'il a appris ;
les questions les plus manquées disent au maître quoi reprendre en classe. La direction peut passer
au grain « par classe seulement » (T-136). Étape 6.

---

## 14. Journal des révisions

- v1.0, 16 novembre 2026 : première rédaction, d'après `docs/plan-phase-12.md` §3.5, le relevé des
  quiz des 398 leçons fait le même jour (392 à la forme habituelle, 1 990 questions) et les
  décisions du porteur ; deux ADR (`format-des-quiz`, `tables-des-lots-2-a-5`) ; questions T-139 à
  T-142.
