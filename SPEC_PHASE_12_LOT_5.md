# SPEC_PHASE_12_LOT_5.md — Devoirs à rendre

**Projet** : ERP de gestion scolaire — Tauri 2 + Rust + React/TypeScript
**Phase** : 12 — e-learning ; **lot 5** : le maître donne un devoir, l'élève le rend par le portail
(texte ou photos de sa copie) ou sur papier, le maître corrige et, s'il le décide, reporte les
notes au bulletin ; version 2.0.0
**Statut** : v1.0 — proposée le 16 novembre 2026, à soumettre au porteur du projet avant toute
ligne de code ; quatre questions au §14, chacune avec une recommandation
**Documents parents** : `docs/plan-phase-12.md` (§0 bis E-13, §3.7, §4 T-91, §5 décision 12),
`SPEC_PHASE_12_LOT_1.md`, `SPEC_PHASE_12_LOT_4.md`, `SPEC_PHASE_4.md` (évaluations, notes,
validation, bulletins), `SPEC_PHASE_11_A.md` §7 (saisie des notes par les maîtres),
`docs/orchestration.md` (passage de relais du 16 novembre 2026)
**ADR proposés avec ce lot** (`docs/adr/`, 16 novembre 2026) : `report-d-une-note-au-bulletin`,
`copies-photographiees`, `tables-des-lots-2-a-5` ; s'appuie sur les ADR du portail (lot 4)
**Ordre des lots** : 2, 4, 3, puis ce lot 5 — il suppose le portail du lot 4 fait.
**Date** : 16 novembre 2026

---

## 0. Objet et critère de fin

### 0.1 Ce que produit ce lot

T-91 a gardé les devoirs à rendre comme un lot à part, que ce lot réalise. À la fin :

- le maître **donne un devoir** à une classe dans sa matière : consignes (avec formules et
  images), leçon liée, date limite, barème, façon de rendre ; la feuille du devoir s'imprime pour
  les élèves sans appareil ;
- l'élève entré avec son code voit **« Mes devoirs »** sur le portail et **rend** un texte, ou
  jusqu'à cinq photos de sa copie, réduites par son téléphone ; il peut rendre de nouveau tant que
  le devoir n'est pas clos ; après la date, son rendu est accepté et marqué « en retard » (T-143) ;
- un élève qui rend **sur papier** est noté par le maître dans le même écran (T-146) ;
- le maître **corrige** : il ouvre chaque copie, met une note et un commentaire, puis **rend les
  copies corrigées** : chaque élève voit sa note et le commentaire sur le portail (T-145) ;
- le maître **reporte au bulletin** d'un geste : une évaluation « devoir » ordinaire est créée, en
  saisie, que la direction valide comme les autres ; sans ce geste, rien n'entre au bulletin ;
- après la clôture de l'année, la direction peut **libérer la place** des photos de cette année
  (T-144), les notes restant.

### 0.2 Décisions déjà prises qui s'appliquent

- **T-91** : les devoirs à rendre forment un lot à part ; le suivi ne bloque jamais.
- **E-13 et blueprint §16.2** : aucune note n'entre au bulletin sans un geste du maître.
- **T-87** : tous les supports ; donc aussi le devoir sur papier, sans appareil.
- **Lot A, étape 5** : la saisie des notes par les maîtres est possible mais éteinte à la
  livraison (`notes.saisie_maitres`) ; voir T-134 (lot 4) pour les comptes des maîtres.
- « La flexibilité sans incohérence » : ne jamais bloquer, chiffres et documents toujours justes ;
  aucune source affichée.
- R-14 : rien ne se supprime ; seule exception bornée, le contenu des photos après la clôture
  (ADR « copies photographiées »).
- Méthode : fmt, clippy, tsc et ESLint à chaque étape ; tous les essais à la dernière.

### 0.3 Critère de fin, vérifiable

1. **Donner** : un maître de mathématiques de 9e A donne un devoir à la 9e A ; il ne peut pas en
   donner un en biologie ni à une classe qui n'est pas la sienne ; la feuille du devoir s'imprime,
   vérifiée en image.
2. **Voir** : sur le portail, les élèves de 9e A entrés avec leur code voient le devoir dans « Mes
   devoirs » ; ceux de 9e B ne le voient pas ; un élève sans code lit « Pour rendre un devoir,
   entre avec ton code. » et peut toujours lire les leçons.
3. **Rendre** : un élève rend trois photos de 4 Mo prises par un téléphone ; chacune arrive
   réduite à 1 600 pixels au plus et sans métadonnée (essai : une photo avec position GPS ressort
   sans elle) ; un fichier qui n'est ni JPEG ni PNG est refusé ; une sixième photo est refusée avec
   un message simple.
4. **Rendre de nouveau** : le dernier rendu compte ; les précédents restent dans l'historique du
   rendu ; après la date limite, le rendu est accepté et marqué « en retard » ; après « Clore »,
   l'élève lit « Ce devoir est clos » et ne peut plus rendre.
5. **Sur papier** : le maître marque « Rendu sur papier » et note l'élève ; la note compte comme
   les autres.
6. **Corriger et rendre les copies** : notes et commentaires ne sont visibles des élèves qu'après
   « Rendre les copies corrigées » ; un élève ne voit jamais la copie, la note ni le commentaire
   d'un autre (essai sur chaque route).
7. **Report** : sans le geste « Reporter au bulletin », aucune évaluation ni note n'est créée
   (bulletins de l'école d'essai recalculés avant et après) ; avec le geste, une évaluation
   « devoir » en saisie, de même barème, porte les notes ; un élève qui n'a pas rendu n'a pas de
   note ; le bulletin suit les règles de calcul existantes.
8. **Report de nouveau** : une note changée après le report est signalée sur l'écran du devoir ;
   « Reporter de nouveau » met à jour l'évaluation en saisie ; validée, l'évaluation ne bouge pas
   et l'écran renvoie au déverrouillage existant.
9. **Bulletin** : aucun texte du bulletin ne dit « en ligne » ; l'écran des évaluations montre
   « Devoir rendu par les élèves » et le lien vers le devoir.
10. **Libérer la place** : sur une année close, le bouton vide le contenu des photos ; les rendus,
    les notes, les commentaires et les évaluations restent ; l'écran montre « Photo retirée pour
    libérer de la place » ; le geste est au journal ; il est refusé sur une année ouverte.
11. **Sauvegarde** : après sauvegarde et restauration sur une école vide, devoirs, rendus et photos
    (non libérées) sont là.
12. **Migration** : une base 1.2 et une base 2.0 mises à niveau ont les colonnes `origine` et
    `devoir_id` sur `evaluation`, leurs évaluations marquées `saisie`, et leurs bulletins
    inchangés.
13. Module ou portail éteint : les essais des phases précédentes passent sans changement ; portail
    éteint, les devoirs « sur papier » restent utilisables.

---

## 1. Périmètre

**Inclus** : devoir (écrire, donner, imprimer, clore) ; « Mes devoirs » et rendu sur le portail
(texte, photos réduites) ; rendu sur papier ; correction, commentaires, rendre les copies ;
report au bulletin ; libérer la place des photos ; migration, droits, écrans, essais, manuel.

**Exclu** :
- Le rendu depuis la maison : le portail local ne s'atteint qu'à l'école (ADR « en clair sur le
  Wi-Fi ») ; le portail en ligne viendra plus tard.
- Les fichiers autres que des photos (Word, PDF) dans un rendu : un élève qui a un ordinateur
  tape son texte dans le champ.
- La correction automatique, la détection de copie entre élèves, l'annotation dessinée sur la
  photo.
- Les rappels et notifications aux élèves ou aux parents (le SMS de la phase 5 n'est pas branché
  ici).
- Le report d'un score de quiz (T-140).

---

## 2. Ce que l'application possède déjà

| Couture | Où | Ce qu'elle apporte au lot |
|---|---|---|
| Évaluations et notes | `evaluation`, `note` (migration 0017), règles de la phase 4, validation, déverrouillage | Le report crée une évaluation ordinaire |
| Saisie des notes par les maîtres | `notes.saisie_maitres`, portée par classe et matière | Qui peut reporter |
| Affectations | `affectation_enseignement` | Classes et matières d'un maître |
| Portail des élèves | Lot 4 : routes, session par code, file d'écritures, bornes | « Mes devoirs », rendre, voir sa note |
| Format des leçons et éditeur | Lot 1 : Markdown, LaTeX, images, aperçu | Consignes du devoir |
| Fichiers dans la base | Reconnaissance par les premiers octets, rangement par empreinte | Photos des rendus |
| Impression | Modèles Typst | Feuille du devoir, liste des notes |
| Clôture d'année | Phase 3 | Ce qui vit par année ; libérer la place après la clôture |

---

## 3. Donner un devoir

- **Où** : Pédagogie → **Devoirs** (liste par classe et matière), et « Donner un devoir sur cette
  leçon » dans la barre d'une leçon.
- **Champs** : classe (parmi celles de ses affectations pour un maître), matière, titre, consignes
  (éditeur du lot 1 : texte, formules, images ; pas de vidéo), leçon liée (facultative), date et
  heure limites (fin de journée par défaut), barème (sur 20 par défaut ; de 10 à 100), façon de
  rendre : « Texte », « Photos de la copie », « Texte ou photos » (par défaut), « Sur papier
  seulement ».
- **États** : brouillon (le maître seul le voit) → **donné** (visible des élèves de la classe sur
  le portail, imprimable) → **clos** (plus de rendu ; geste du maître, jamais automatique).
- **Donner aussi à une autre classe** : copie du devoir pour une autre classe du maître, avec ses
  propres rendus.
- **Feuille du devoir** : PDF A4 avec en-tête de l'école, classe, matière, titre, consignes, date
  limite, barème ; pour les élèves sans appareil.
- Aucune validation par la direction : un devoir est un acte du maître, comme une évaluation.

---

## 4. Rendre

Sur le portail, élève entré avec son code (lot 4) ; règles des photos : ADR « copies
photographiées ».

- **« Mes devoirs »** (écran 7 du portail) : les devoirs donnés à sa classe, triés par date
  limite ; chacun avec son état : « À rendre », « Rendu le … », « En retard », « Corrigé ».
- **Écran du devoir** : consignes, leçon liée (lien), date limite ; selon la façon de rendre, un
  champ de texte (le brouillon se garde dans le navigateur du téléphone tant qu'il n'est pas
  rendu) et « Ajouter une photo » (appareil photo ou galerie), cinq au plus (T-144), chacune
  réduite par la page à 1 600 pixels au plus avant l'envoi, avec un aperçu et « Retirer » ;
  « Rendre mon devoir », puis « Ton devoir est rendu le 12 mars à 10 h 40. ».
- **Rendre de nouveau** : possible tant que le devoir n'est pas clos ; le dernier rendu compte ;
  les précédents restent.
- **Après la date limite** (T-143) : le rendu est accepté et marqué « en retard », pour le maître
  seulement ; l'élève lit « Rendu après la date limite ».
- **Sans code** : « Pour rendre un devoir, entre avec ton code. » ; les leçons restent lisibles.
- **Serveur chargé** : un rendu s'écrit tout de suite (ADR « charge ») ; s'il ne passe pas, la page
  garde le texte et les photos et propose « Réessayer ».
- **Devoir « sur papier seulement »** : l'élève voit les consignes, et « À rendre sur papier à ton
  maître ».

---

## 5. Corriger

- **Écran du devoir** (maître) : la liste des élèves de la classe avec leur état (rendu, en retard,
  sur papier, pas rendu), la note et le commentaire ; filtres « À corriger », « Pas rendu ».
- **Copie** : le texte rendu, les photos en grand (agrandir, tourner, suivante), l'historique des
  rendus de l'élève ; note (sur le barème, avec la même précision que la saisie existante) et
  commentaire ; « Élève suivant ».
- **Rendu sur papier** (T-146) : sur la ligne d'un élève, « Rendu sur papier », puis la note.
- **Rendre les copies corrigées** (T-145) : les notes et commentaires deviennent visibles des
  élèves sur le portail ; une note changée ensuite est visible aussitôt.
- **Imprimer** : liste des notes de la classe pour le devoir.

---

## 6. Reporter au bulletin

Règles : ADR « report d'une note de devoir au bulletin ».
- Bouton **Reporter au bulletin** sur l'écran du devoir, pour qui a `grades:create` dans la portée
  de la classe et de la matière ; une confirmation donne la période (celle de la date limite par
  défaut), le barème, la pondération (1 par défaut) et le nombre de notes qui entreront.
- Crée une évaluation « devoir » **en saisie**, libellée du titre du devoir, `origine` « devoir en
  ligne », liée au devoir ; les élèves sans note restent sans note.
- **Reporter de nouveau** met à jour l'évaluation tant qu'elle est en saisie ; validée, l'écran
  renvoie au déverrouillage existant.
- **Maîtres sans saisie des notes** (`notes.saisie_maitres` éteint) : la direction ou le
  secrétariat reporte depuis l'écran du devoir (T-134 pour les comptes des maîtres).
- Ensuite, tout suit les règles de la phase 4 : validation par la direction, calcul, bulletin.

---

## 7. Libérer la place des photos

- Réglages des devoirs, pour la direction : par année **close**, la place prise par les photos et
  « Libérer la place des photos de cette année » ; confirmation (« Les photos des copies de
  2025-2026 seront retirées. Les notes et les commentaires restent. »).
- Effet : le contenu des photos est vidé, la ligne et l'empreinte restent, `libere_le` est rempli ;
  l'écran de la copie montre « Photo retirée pour libérer de la place ». Au journal d'audit.
- Refusé sur l'année en cours ; jamais automatique.

---

## 8. Données nouvelles

Migration **0066** (ou la suivante libre), ADR « tables des lots 2 à 5 » :

| Table | En bref |
|---|---|
| `devoir` | Année, classe, matière, auteur, titre, consignes, leçon liée, date limite, barème, façon de rendre, état, copies rendues le |
| `rendu_devoir` | Devoir, élève, numéro, origine (portail ou papier), texte, rendu le, en retard, note, commentaire, noté le, par |
| `fichier_rendu` | Empreinte, contenu, taille, sorte, libéré le |
| `photo_rendu` | Rendu, ordre, empreinte |
| `evaluation` | Colonnes `origine` (`saisie` par défaut, `devoir_en_ligne`) et `devoir_id` |

Réglages : aucun nouveau. Fichiers : aucun hors de la base.

---

## 9. Règles

1. **Aucune note n'entre au bulletin sans le geste « Reporter au bulletin »**, puis la validation
   ordinaire.
2. **L'élève ne voit que ses devoirs, ses rendus, ses notes** ; jamais ceux d'un autre.
3. **Ne jamais bloquer** : rendu en retard accepté ; rendu sur papier noté ; serveur chargé, le
   rendu se garde et se réessaie ; portail éteint, le devoir sur papier marche.
4. **Rien ne se supprime** : rendus précédents gardés ; seule la place des photos d'une année close
   peut être libérée, par la direction, au journal.
5. **Aucune métadonnée de photo n'est gardée.**
6. **Chaque geste du personnel est tracé** : devoir donné, clos, copies rendues, note changée après
   le rendu des copies, report, place libérée.

---

## 10. Droits

Une permission nouvelle, dans la composition livrée suivante (19 si le lot 4 a pris la 18) :

| Permission | Ce qu'elle permet | Rôles à la livraison |
|---|---|---|
| `homework:manage` | Écrire, donner, imprimer, clore un devoir ; corriger ; noter un rendu sur papier ; rendre les copies. Portée : classes et matières de ses affectations pour un Enseignant | Administrateur, Directeur, Directeur des études, Enseignant |

Reporter au bulletin : `grades:create` et sa portée. Libérer la place des photos :
`library:manage`. Voir les devoirs d'une classe sans les corriger : `library:follow`.

---

## 11. Écrans

- **Pédagogie → Devoirs** : liste par classe et matière, état, nombre de rendus ; « Donner un
  devoir ».
- **Devoir** (maître) : consignes, liste des élèves, copie, rendre les copies, reporter, imprimer.
- **Évaluations** (existant) : marque « Devoir rendu par les élèves » et lien.
- **Réglages des devoirs** (direction) : place des photos par année, libérer.
- **Portail** : « Mes devoirs », écran du devoir, rendu, note et commentaire.
- Lisibilité : skill du projet ; sur le portail, le bouton « Rendre mon devoir » fait au moins
  44 pixels de haut et l'état se dit en mots, pas seulement en couleur.

---

## 12. Ordre de réalisation

Des étapes courtes ; une session en fait plusieurs, commite et pousse après chacune, passe la main
au-delà d'environ 150 000 jetons. Une seule session compile.

1. **Migration 0066** : tables, colonnes de `evaluation`, déclencheurs ; mise à niveau d'une base
   1.2 et 2.0 ; permission `homework:manage` et composition des rôles.
2. **Donner** : service et écrans du devoir, portée du maître, états, copie vers une autre classe,
   feuille du devoir (Typst).
3. **Rendre** : routes du portail (`/eleve/api/devoirs`, `/eleve/api/devoir/{id}`,
   `/eleve/api/rendu`, `/eleve/api/rendu/{id}/photo/{ordre}`) ajoutées à la liste et à son essai ;
   réduction des photos dans la page ; vérification et nettoyage des JPEG dans le noyau ; rendu de
   nouveau ; en retard ; écrans « Mes devoirs ».
4. **Corriger** : écran du devoir, copie, rendu sur papier, rendre les copies, liste des notes.
5. **Reporter** : création et mise à jour de l'évaluation, marque dans l'écran des évaluations,
   règles de validation.
6. **Libérer la place** : écran, geste, journal.
7. **Essais et livraison du lot** : suite Rust complète, vitest, knip ; critères du §0.3 ;
   Playwright d'ensemble (donner, rendre trois photos depuis un écran de 360 pixels, corriger,
   rendre les copies, reporter, valider, bulletin) avec captures montrées au porteur ; manuel
   (« Les devoirs à rendre ») ; notes de version ; essais à la main (photo prise par un vrai
   téléphone, lisible une fois réduite) ; école d'essai refaite avec un devoir corrigé et reporté.

Les étapes 2 et 3 peuvent avancer ensemble une fois les tables posées.

---

## 13. Ce qui reste après ce lot pour la 2.0.0

La phase 12 est alors entière. Restent, dans l'ordre du passage de relais du 16 novembre 2026 :
l'intégration des cours de la bibliothèque (crédit des auteurs des images selon l'ADR « paquet
de contenus » ; les six leçons dont le quiz n'a pas la forme habituelle, relevées au lot 3) ; les
lots C et D ; l'installeur officiel, le guide et les essais à la main sur deux machines.

---

## 14. Questions au porteur du projet

Numérotées T-143 à T-146. Chacune a une recommandation ; le lot peut commencer sur les
recommandations.

### T-143 — Un devoir rendu après la date limite

- a) accepté, marqué « en retard » pour le maître, tant que le maître n'a pas clos le devoir ;
- b) refusé dès la date passée.

*Recommandation* : a). Le Wi-Fi était coupé, le poste principal éteint, l'élève malade : refuser
bloquerait pour une raison que l'élève ne commande pas. Le maître décide de ce que vaut le retard,
et clôt le devoir quand il veut. Étape 3.

### T-144 — Les photos des copies : combien, et combien de temps

- a) cinq photos au plus par rendu ; gardées dans la base ; après la clôture de l'année, la
  direction peut libérer leur place, les notes restant ;
- b) cinq photos, gardées pour toujours ;
- c) trois photos, gardées pour toujours.

*Recommandation* : a). Cinq pages couvrent un devoir de lycée ; une grande école qui utilise
beaucoup les devoirs ajoute quelques gigaoctets par an à sa sauvegarde, que la direction peut
libérer une fois les réclamations passées. Étapes 3 et 6.

### T-145 — L'élève voit-il sa note et le commentaire du maître ?

- a) oui, quand le maître « rend les copies corrigées » ;
- b) non : la note se donne en classe, le portail montre seulement « Corrigé ».

*Recommandation* : a). Le commentaire écrit est ce qui fait progresser ; le geste « rendre les
copies » laisse le maître choisir le moment, comme en classe. Étape 4.

### T-146 — Noter dans GESCO un devoir rendu sur papier

- a) oui : sur l'écran du devoir, « Rendu sur papier » et la note, qui compte comme les autres ;
- b) non : un devoir sur papier se note dans une évaluation ordinaire.

*Recommandation* : a). Dans une classe, une partie des élèves n'a pas de téléphone (T-87) : un
même devoir doit pouvoir mêler rendus par le portail et copies papier, et un seul report au
bulletin. Étape 4.

---

## 15. Journal des révisions

- v1.0, 16 novembre 2026 : première rédaction, d'après `docs/plan-phase-12.md` §3.7 et §5
  (décision 12), les décisions du porteur (T-87, T-91) et les règles de la phase 4 ; trois ADR
  (`report-d-une-note-au-bulletin`, `copies-photographiees`, `tables-des-lots-2-a-5`) ; questions
  T-143 à T-146.
