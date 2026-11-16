# SPEC_PHASE_12_LOT_2.md — Enrichir la bibliothèque

**Projet** : ERP de gestion scolaire — Tauri 2 + Rust + React/TypeScript
**Phase** : 12 — e-learning ; **lot 2** : ce que le lot 1 a laissé de l'enrichissement — sons,
import d'une leçon Word, ajout réservé à une classe, échange de leçons entre écoles ; version 2.0.0
**Statut** : v1.0 — proposée le 16 novembre 2026, à soumettre au porteur du projet avant toute
ligne de code ; quatre questions au §12, chacune avec une recommandation
**Documents parents** : `docs/plan-phase-12.md` (§0, §0 bis, §3.4, §5), `SPEC_PHASE_12_LOT_1.md`,
`ETAT_AVANCEMENT.md` (« Phase 12, lot 1 »), `docs/orchestration.md` (passage de relais du
16 novembre 2026)
**ADR proposés avec ce lot** (`docs/adr/`, 16 novembre 2026) : `echange-de-lecons-entre-ecoles`,
`importer-une-lecon-word`, `tables-des-lots-2-a-5`
**Ordre des lots** : 2, puis 4 (portail), puis 3 (quiz), puis 5 (devoirs) — T-87.
**Date** : 16 novembre 2026

---

## 0. Objet et critère de fin

### 0.1 Ce que produit ce lot

Le lot 1 a absorbé l'essentiel de l'ancien lot 2 : l'école ajoute déjà chapitres, leçons, sujets,
documents, images et vidéos, et améliore les leçons livrées. Restent quatre choses, qui servent
aussi le portail du lot 4 :

- **des sons** dans une leçon : prononciation en anglais, lecture d'un texte, consigne dite par le
  maître ;
- **importer une leçon depuis Word**, pour le maître qui a ses fiches dans Word (T-124) ;
- **réserver un ajout à certaines classes** d'un niveau : la leçon que le maître de la 9e A écrit
  pour sa classe (T-125) ;
- **échanger des leçons avec une autre école** : un fichier `.gles` sur une clé ou par WhatsApp,
  reçu en brouillons que la direction valide (T-126).

### 0.2 Décisions déjà prises qui s'appliquent

- Toutes celles du §0.2 de `SPEC_PHASE_12_LOT_1.md`, en particulier : la leçon livrée reste
  intacte ; l'enseignant de la matière modifie, la direction valide avant que les élèves voient
  (T-89) ; « la flexibilité sans incohérence » ; aucun texte affiché ne cite de source.
- **Aucun texte affiché ne dit qu'une leçon a été relue ou non** (T-116, « nulle part »).
- **Plus aucun cours n'est écrit ni repris** dans les sessions de code (15 novembre 2026).
- La 2.0 garde tout son périmètre : ce lot y entre.
- Méthode de travail : à chaque étape, fmt, clippy, tsc et ESLint, puis commit et push ; tous les
  essais (suite Rust complète, vitest, knip, Playwright, captures) une fois, à la dernière étape.

### 0.3 Critère de fin, vérifiable

1. Un son MP3 de 3 Mo et un son M4A s'ajoutent à une leçon ; ils se lisent dans la leçon et en mode
   classe, sur le poste principal et sur un poste relié ; un son de 12 Mo est refusé avec un
   message clair ; un fichier renommé `.mp3` qui n'en est pas un est refusé.
2. Après sauvegarde et restauration sur une école vide, les sons sont là ; à l'impression, un son
   devient la ligne « Son : <titre> ».
3. Un `.docx` d'essai (titres, gras, liste, tableau, deux images, une formule Word) devient un
   brouillon dont l'aperçu montre les titres, la liste, le tableau et les images ; la formule est
   remplacée, pour l'auteur seul, par « Formule à retaper ici » ; un `.doc` est refusé avec « Enregistrez
   ce fichier au format .docx ».
4. Une leçon de l'école réservée à la 9e A n'apparaît, pour les élèves, que dans ce qui sert la
   9e A (essai sur la fonction de visibilité qu'utilisera le portail) ; dans le catalogue du
   personnel, elle porte la marque « 9e A seulement ».
5. L'école A exporte un chapitre de trois leçons, dont une version de l'école d'une leçon livrée ;
   l'école B (même paquet, matières renommées) l'importe : deux leçons de l'école et un brouillon
   de la leçon livrée, rangés sous les matières de B ; rien n'est visible des élèves avant
   publication.
6. Le même fichier importé une seconde fois n'ajoute rien, et le résumé le dit ; un fichier dont un
   octet a changé est refusé entier ; un fichier qui contient du HTML ou un SVG avec un script
   arrive filtré et nettoyé.
7. Une école en 2.0 (lot 1) qui reçoit ce lot garde ses images, documents, leçons et versions :
   l'essai de la reconstruction de `fichier_bibliotheque` compare lignes, index et déclencheurs
   avant et après.
8. Module éteint : rien de ce lot ne paraît ; les essais des phases précédentes passent sans
   changement.

---

## 1. Périmètre

**Inclus** : sons (ajout, lecture, mode classe, impression, sauvegarde, poste relié) ; import d'un
`.docx` en brouillon ; portée d'un ajout de l'école (niveau entier ou classes choisies) ; export
et import `.gles` ; migration ; essais, manuel, notes de version.

**Exclu** :
- L'enregistrement d'un son par le micro du poste (T-127) : on ajoute un fichier enregistré au
  téléphone.
- La conversion des formules de Word en LaTeX (ADR « importer une leçon Word »).
- La publication directe des leçons reçues (ADR « échange ») ; l'échange par Internet (lot C).
- La mesure de la couverture du programme par les cours d'une école (« ne pas faire remarquer ce
  qui manque »).
- Les vidéos dans le fichier d'échange : le bouton « Copier les vidéos vers une clé » du lot 1
  existe.

---

## 2. Ce que l'application possède déjà

| Couture | Où | Ce qu'elle apporte au lot |
|---|---|---|
| Fichiers de l'école | `fichier_bibliotheque`, `fichier_contenu` (migration 0061), `bibliotheque/fichiers.rs` : reconnaissance par les premiers octets, bornes, rangement par empreinte | Les sons s'y ajoutent comme une troisième nature |
| Vidéos par morceaux | Route réservée aux postes appairés dans `gesco-reseau` | Lecture d'un son depuis un poste relié |
| Format des leçons | `pulldown-cmark`, arbre de blocs, éléments `image` et `video` | Un élément `son` de plus |
| Paquet `.gbib` | Lecture et vérification dans le noyau, `flate2` | Base du fichier `.gles` |
| Repère commun | Codes nationaux, `correspondance_discipline` | Ranger une leçon reçue sous les matières de l'école |
| Circuit des versions | `version_contenu`, un brouillon par leçon, historique, comparaison | Une leçon reçue suit ce circuit |
| Imports Excel | `calamine` 0.36.1, qui compile `zip` 8.6.0 et `quick-xml` 0.42.0 | Lecture du `.docx` sans crate nouvelle |
| Inscriptions | `inscription`, classes de l'année | Portée d'un ajout réservé à une classe |

---

## 3. Sons

- **Bouton Son** dans l'éditeur, à côté d'Image et de Vidéo ; il écrit l'appel du son dans le
  texte, comme pour une image (`son:<empreinte>` et un titre).
- **Formats** : MP3, M4A (AAC), OGG (Vorbis ou Opus, comme les messages vocaux de WhatsApp), WAV ;
  reconnus par leurs premiers octets, jamais par l'extension ; **10 Mo au plus** (T-127), soit
  une dizaine de minutes en MP3.
- **Rangement** : comme une image, dans la base chiffrée, une fois par empreinte, donc dans la
  sauvegarde.
- **Lecture** : un lecteur simple (lecture, pause, avancer), titre au-dessus ; en mode classe, un
  grand bouton de lecture ; depuis un poste relié, le son passe par la route par morceaux des
  vidéos.
- **Impression et PDF** : la ligne « Son : <titre> ».
- **Absent** (fichier retiré) : la leçon s'ouvre sans lui ; le personnel voit « Son non
  disponible », les élèves ne voient rien.

---

## 4. Importer une leçon depuis Word

Règles : ADR « importer une leçon Word ». Sous réserve de T-124.

- **Où** : « Importer depuis Word » à l'ajout d'une leçon (nouvelle leçon) et dans l'éditeur
  d'une leçon (nouveau brouillon qui remplace le texte du brouillon, après confirmation).
- **Ce qui passe** : titres (styles « Titre 1 » à « Titre 3 », noms anglais compris), paragraphes,
  gras, italique, listes à puces et numérotées, tableaux simples, images (rangées comme celles du
  bouton Image, bornes du lot 1).
- **Ce qui ne passe pas, dit à l'auteur seul sous l'aperçu** : une formule de Word devient « Formule
  à retaper ici » (note pour le maître, jamais vue des élèves ni imprimée) ; une image de plus de
  5 Mo est laissée de côté (« Une image trop lourde n'a pas été reprise ») ; zones de texte,
  colonnes, en-têtes et pieds de page, commentaires, révisions et macros sont ignorés.
- **Bornes** : `.docx` de 20 Mo au plus ; depuis un poste relié, il voyage par son contenu ; un
  `.doc` ou un fichier illisible est refusé avec un message simple.
- Le résultat est un **brouillon** : il suit le circuit du lot 1 (soumission, publication).

---

## 5. Réserver un ajout à certaines classes

Sous réserve de T-125.

- À l'ajout ou dans les propriétés d'un contenu **de l'école** (chapitre, leçon, sujet, document),
  « Pour qui » : « Tout le niveau » (par défaut) ou « Certaines classes », avec les classes du
  niveau de l'année en cours à cocher. Un maître ne coche que ses classes.
- Les contenus **livrés**, et les versions de l'école d'une leçon livrée, restent pour tout le
  niveau : une leçon livrée n'a qu'une version visible par école.
- **Effet** : le catalogue du personnel montre tout, avec la marque « 9e A seulement » ; le mode
  classe et l'impression ne changent pas (le maître choisit ce qu'il montre) ; la fonction du noyau
  « visible pour une classe » , que le portail du lot 4 appellera, ne rend le contenu qu'aux
  classes choisies.
- **Une classe qui n'existe plus** l'année suivante : le contenu revient à « tout le niveau »
  pour la nouvelle année tant que personne ne choisit d'autres classes ; l'ancien choix reste dans
  l'historique.

---

## 6. Échanger des leçons avec une autre école

Règles : ADR « échange de leçons entre écoles ».

- **Exporter** (barre d'un chapitre, d'une matière d'un niveau, ou sélection dans le catalogue) :
  la liste de ce qui partira, puis la boîte d'enregistrement du système ; nom proposé « Leçons —
  Classe — Matière.gles ». Ne part que la version visible de chaque leçon, avec ses images, ses
  documents et ses sons. Ni vidéo, ni historique, ni nom d'auteur, ni nom d'école.
- **Importer** (réglages de la bibliothèque, `library:manage`) : choix du fichier, vérification,
  puis la liste « ce qui arrive et où cela se rangera » ; confirmer ; résumé.
- **Rangement** : une version d'une leçon livrée devient un brouillon de cette leçon ; si un
  brouillon existe, elle entre dans l'historique comme « Version reçue », avec Comparer et
  « Repartir de cette version » ; une leçon de l'école devient une leçon de l'école en brouillon,
  sous un chapitre du même titre (créé au besoin, à la fin de la partie) ; une discipline sans
  correspondance se range sous son libellé de référence.
- **Brouillons reçus** : au nom de la personne qui importe ; ils paraissent dans « Version de
  l'école seulement » et la direction les publie un à un, ou d'un geste « Publier les leçons
  reçues de cet import » (T-126).
- **Le même fichier deux fois** ne double rien (empreinte du texte déjà dans l'historique).

---

## 7. Données nouvelles

Migration **0063** (ou la suivante libre), ADR « tables des lots 2 à 5 » :

| Table | En bref |
|---|---|
| `fichier_bibliotheque` (reconstruite) | Nature `son` en plus ; lignes, index et déclencheurs gardés |
| `contenu_classe` | Contenu de l'école, classe : l'ajout est réservé à ces classes |
| `version_recue` | Contenu, texte, empreinte, import, reçu le |
| `import_lecons` | Empreinte du fichier, quand, qui, nombre de leçons, nombre ignorées |

Aucun réglage nouveau. Aucune permission nouvelle.

---

## 8. Règles

1. Les règles du §13 de `SPEC_PHASE_12_LOT_1.md` s'appliquent toutes.
2. **Rien de reçu n'est visible des élèves sans publication** par l'école qui reçoit.
3. **Rien de l'école ne part sans geste** : l'export ne contient que ce que la personne a choisi,
   jamais un brouillon, jamais un nom.
4. **Un son ou une image de trop ne bloque rien** : l'import Word continue sans l'image, la leçon
   s'ouvre sans le son.
5. **Chaque geste est tracé** : son ajouté, import Word, portée changée, export, import.

---

## 9. Droits

Aucune permission nouvelle : sons, import Word et portée relèvent de `library:edit` (portée des
affectations pour un Enseignant) ; l'export de `library:edit` ; l'import et « Publier les leçons
reçues » de `library:manage` et `library:publish`.

---

## 10. Écrans

- **Éditeur** : boutons Son et « Importer depuis Word » ; notes de l'auteur sous l'aperçu.
- **Propriétés d'un contenu de l'école** : « Pour qui ».
- **Catalogue** : marque « <classe> seulement » ; bouton Exporter sur un chapitre et une matière ;
  sélection pour exporter.
- **Réglages de la bibliothèque** : « Importer des leçons d'une autre école », liste des imports.
- **Historique** : « Version reçue ».
- Lisibilité : skill du projet (contrastes, clavier, focus, 200 %).

---

## 11. Ordre de réalisation

Des étapes courtes ; une session en fait plusieurs, commite et pousse après chacune, passe la main
au-delà d'environ 150 000 jetons. Une seule session compile. Avant toute grosse compilation :
`outils/elaguer-target.ps1 -Incremental`.

1. **Migration 0063** : reconstruction de `fichier_bibliotheque`, tables nouvelles, déclencheurs ;
   essai écrit de la reconstruction (non lancé).
2. **Sons** : reconnaissance des quatre formats, borne, élément `son` du format, lecteur, mode
   classe, impression, route par morceaux pour les postes reliés.
3. **Portée d'un ajout** : « Pour qui », marque du catalogue, fonction « visible pour une
   classe » du noyau, retour à « tout le niveau » à la nouvelle année.
4. **Import Word** (si T-124 dit oui) : `zip` et `quick-xml` en dépendances directes aux versions
   du verrou, conversion, notes de l'auteur, écran.
5. **Échange** : écriture et lecture du `.gles`, filtre, rangement, versions reçues, écran
   d'import, publication des leçons reçues.
6. **Essais et livraison du lot** : suite Rust complète, vitest, knip ; essais des critères du
   §0.3 ; scénario Playwright d'ensemble (son, import Word, export puis import sur une seconde
   école) avec captures montrées au porteur ; manuel (« Ajouter un son », « Importer une fiche
   Word », « Passer des leçons à une autre école ») ; notes de version 2.0.0.

Les étapes 2, 3 et 4 ne dépendent pas l'une de l'autre.

---

## 12. Questions au porteur du projet

Numérotées T-124 à T-127, dans la plage laissée à la phase 12. Chacune a une recommandation ; le
lot peut commencer sur les recommandations.

### T-124 — Importer une leçon depuis Word

Faut-il que GESCO lise une fiche Word (`.docx`) pour en faire une leçon ?
- a) oui, avec les titres, les listes, les tableaux et les images ; les formules sont à retaper ;
- b) non : on écrit dans l'éditeur, ou on joint la fiche comme document PDF.

*Recommandation* : a). C'est ce qui fera écrire les maîtres qui ont déjà leurs fiches ; aucune
dépendance nouvelle n'entre dans la compilation. Étape 4.

### T-125 — Un ajout réservé à une classe

Un maître de 9e A écrit une leçon pour sa classe seulement. Faut-il pouvoir la réserver à
certaines classes du niveau ?
- a) oui, pour ce que l'école ajoute ; les leçons livrées restent pour tout le niveau ;
- b) non : tout ce que l'école publie est pour tout le niveau.

*Recommandation* : a). Deux maîtres du même niveau n'avancent pas au même rythme ; le portail
(lot 4) en a besoin pour ne pas montrer à la 9e B ce que le maître de la 9e A a écrit pour la
sienne. Étape 3.

### T-126 — Les leçons reçues d'une autre école

- a) elles arrivent en brouillons ; la direction les publie une à une ou d'un geste pour tout
  l'import ;
- b) elles arrivent publiées, visibles des élèves tout de suite.

*Recommandation* : a). C'est la règle de T-89 (la direction valide avant que les élèves voient) ;
le geste « publier tout l'import » évite de cliquer leçon par leçon. Étape 5.

### T-127 — Les sons

- a) fichiers MP3, M4A, OGG ou WAV, 10 Mo au plus, ajoutés depuis le poste (enregistrés au
  téléphone puis copiés) ;
- b) comme a), plus un bouton « Enregistrer ma voix » par le micro du poste.

*Recommandation* : a). Le téléphone enregistre mieux que le micro d'un vieux poste, et les
messages vocaux de WhatsApp (OGG) passent tels quels ; b) pourra venir si les maîtres le
demandent. Étape 2.

---

## 13. Journal des révisions

- v1.0, 16 novembre 2026 : première rédaction, d'après `docs/plan-phase-12.md` §3.4, ce que le
  lot 1 a fait et laissé, et les décisions du porteur des 11 au 16 novembre 2026 ; trois ADR
  proposés (`echange-de-lecons-entre-ecoles`, `importer-une-lecon-word`, `tables-des-lots-2-a-5`) ;
  questions T-124 à T-127.
