# Le format d'une leçon : Markdown restreint et LaTeX, lu une fois par le noyau, affiché par KaTeX, imprimé par Typst

**Date** : 15 novembre 2026
**Statut** : proposé le 15 novembre 2026 — décision 4 de `docs/plan-phase-12.md` §5, lot 1 de la phase 12 (`SPEC_PHASE_12_LOT_1.md`)

## Contexte

La bibliothèque livrée avec GESCO est écrite. Relevé du 15 novembre 2026 sur
`gesco-bibliotheque\contenus` : 398 leçons (7e : 75, 8e : 65, 9e : 208, terminale : 50),
41 sujets officiels du DEF et du BAC et leurs corrigés, 285 figures SVG, 18 Mo en tout.

- Toutes sont en Markdown : titres, gras et italique, listes, tableaux (les 398 leçons en ont),
  citations (313 leçons), images appelées par `![description](figure.svg)` (311 appels).
  Aucune balise HTML.
- Les formules sont en LaTeX, `$…$` dans la ligne et `$$…$$` seul sur sa ligne, limitées à ce
  que KaTeX sait afficher (décision du porteur du 12 novembre 2026). Le relevé compte environ
  cent commandes réelles (`\frac`, `\times`, `\sqrt`, `\vec`, `\widehat`, `\overrightarrow`,
  `\lim`, `\int`, `\mathbb`, lettres grecques…) et deux environnements, `cases` et `array`.
- Les figures de géométrie sont des SVG produits par programme (décision du 13 novembre 2026).
- Le porteur a demandé le 14 novembre 2026 que les cours soient améliorables dans
  l'application : l'école écrit sa version d'une leçon, avec formules, images, vidéos et
  exercices. Les maîtres écrivent aussi des leçons nouvelles.
- La leçon doit s'afficher à l'écran (poste principal et postes reliés), s'imprimer et
  s'enregistrer en PDF (T-87 : « tout cela à la fois »), plus tard se servir aux téléphones
  (lot 4). Tout fonctionne sans Internet.
- Ce qui existe : le moteur de documents Typst 0.15.1 (`gesco-docs`), qui imprime bulletins et
  rapports avec la police Charis SIL ; il n'a pas de police mathématique. Le frontend n'a
  aucune bibliothèque de Markdown ni de formules (React 19, TanStack Query, Tailwind).
- Les maîtres sont souvent peu à l'aise au clavier ; beaucoup écrivent leurs formules au
  tableau, pas en LaTeX.

## Options

| | 1. Markdown restreint et LaTeX, comme la bibliothèque | 2. Blocs structurés (JSON) et éditeur par blocs | 3. Éditeur visuel (HTML, ProseMirror ou TipTap) |
|---|---|---|---|
| Apprentissage | Faible pour l'équipe ; pour un maître, une barre de boutons et un aperçu côte à côte | Moyen : un modèle de blocs à inventer | Faible pour un maître, élevé pour l'équipe |
| Exploitation | Un seul format pour les leçons livrées et celles de l'école ; textes lisibles dans la base et dans la sauvegarde | Les 398 leçons livrées à convertir et à garder converties | Deux formats (livré en Markdown, école en HTML) ; HTML à nettoyer à chaque enregistrement |
| Maturité | CommonMark et KaTeX sont stables et très répandus | Rien d'existant : tout à écrire | Bibliothèques mûres mais lourdes, nombreuses dépendances |
| Pour un agent de code | Bon : un analyseur, un arbre, deux rendus | Moyen | Mauvais : formules, tableaux et impression à recoudre |
| Plus gros risque | Un maître qui ne veut pas écrire de LaTeX | Un format maison qui vieillit mal | Une impression qui ne ressemble pas à l'écran ; du HTML collé depuis le web |

Écartées sans tableau : imprimer depuis la fenêtre de l'application (impression du navigateur
intégré) — le rendu dépend du poste et ne se vérifie pas par un essai, alors que les bulletins
sont vérifiés en image ; convertir le LaTeX par la crate `mitex` (0.2.4, aucune sortie depuis le
17 juin 2024) — abandonnée, et elle suppose un paquet Typst téléchargé.

## Décision

Option 1. Une leçon, livrée ou écrite par l'école, est un texte Markdown restreint avec des
formules LaTeX du sous-ensemble de KaTeX. Le noyau Rust le lit une seule fois, avec
`pulldown-cmark`, en un arbre de blocs ; l'écran affiche cet arbre (React, formules par KaTeX),
l'impression le traduit en Typst (formules par une conversion du sous-ensemble LaTeX vers les
mathématiques de Typst, écrite dans `gesco-docs`).

- **Ce qui est permis** : titres de trois niveaux, paragraphes, gras, italique, listes
  numérotées ou non, tableaux, citations, filets, formules `$…$` et `$$…$$`, images par leur
  nom (SVG, PNG, JPEG), et une ligne propre à GESCO pour une vidéo, `![titre](video:<empreinte>)`.
  Le HTML brut n'est jamais interprété : il s'affiche comme du texte.
- **Une seule lecture, deux rendus** : la commande qui ouvre une leçon renvoie l'arbre, pas le
  Markdown ; le frontend n'analyse rien. L'aperçu de l'éditeur appelle la même commande.
- **Conversion pour l'impression** : une table des commandes LaTeX connues, bâtie sur le relevé
  de la bibliothèque (environ cent commandes, `cases`, `array`). Une formule qui contient une
  commande inconnue s'imprime telle qu'elle est écrite, en caractères à chasse fixe ; elle ne
  bloque jamais l'impression. L'éditeur le signale à son auteur sous l'aperçu, sans refuser
  l'enregistrement.
- **Police des formules imprimées** : New Computer Modern Math, embarquée dans
  `gesco-docs/polices` comme Charis SIL.

| Brique | Version retenue | Vérifiée le 15 novembre 2026 | Support |
|---|---|---|---|
| `pulldown-cmark` (crate, MIT) | 0.13.4 (20 mai 2026) | Dernière : 0.13.6 du 30 octobre 2026, trop récente ; 0.13.5 du 21 octobre 2026. Revérifier le jour de l'étape : la 0.13.6 aura alors plus d'un mois | Projet actif (six sorties en 2026), pas de politique de fin de vie publiée ; relais : la dernière 0.13.x |
| `katex` (npm, MIT) | 0.19.1 (23 octobre 2026) si elle a un mois révolu le jour de l'étape (23 novembre 2026), sinon 0.18.11 (1er octobre 2026) | Dernière : 0.19.1 | Projet actif, pas de politique de fin de vie ; polices KaTeX livrées avec le paquet, servies sans Internet |
| New Computer Modern Math (police, GUST Font License) | 8.1.1 (22 juin 2026) | Version CTAN | Police stable ; la même famille sert de police mathématique par défaut à Typst |
| Typst | 0.15.1, déjà en place | Inchangé | Déjà suivi par `gesco-docs` |

## Justification

1. Les 398 leçons et les 82 fichiers de sujets existent déjà dans ce format : aucune
   conversion, et la leçon de l'école part du texte livré, sans perte.
2. Un seul analyseur, dans le noyau : l'écran, l'impression et plus tard le portail lisent le
   même arbre ; ce qui s'affiche est ce qui s'imprime.
3. Le noyau filtre le texte une fois : pas de HTML, pas de lien vers l'extérieur, pas de
   chemin de fichier. Le portail des élèves (lot 4) hérite de cette garde.
4. L'impression passe par Typst, comme les bulletins : elle se vérifie par des essais en image,
   identique sur tous les postes.
5. La conversion LaTeX vers Typst est bornée par ce que la bibliothèque emploie vraiment ; un
   essai convertit toutes les formules livrées et doit n'en laisser aucune de côté.
6. Une seule dépendance npm (KaTeX) et une seule crate (pulldown-cmark).
7. Le maître peu à l'aise reçoit des boutons (titre, liste, tableau, formule, image, vidéo) qui
   écrivent le balisage pour lui, et un aperçu immédiat.

## Conséquences

Positives : un format ouvert, lisible tel quel dans la sauvegarde ; leçons livrées et leçons de
l'école traitées par le même code ; l'impression ne dépend pas du poste ; rien ne s'exécute
depuis le texte d'une leçon.

Négatives : écrire une formule demande d'apprendre un peu de LaTeX, ou de passer par les
boutons ; KaTeX et la conversion vers Typst ne couvrent pas exactement le même ensemble, d'où
la règle d'une formule imprimée telle quelle ; l'aperçu passe par une commande, donc par le
réseau sur un poste relié (attente de l'ordre d'une demi-seconde, à mesurer) ; les polices
KaTeX (environ 1 Mo) et la police mathématique (moins de 1 Mo) grossissent l'installeur.

## Contre-argument conservé

Un éditeur visuel est ce qu'un maître attend : il voit sa leçon comme elle sera, sans balises.
Il n'a pas gagné parce qu'il crée un second format à côté des 398 leçons livrées, qu'il colle du
HTML venu d'ailleurs, et que l'impression n'y ressemblerait qu'à peu près. Les boutons et
l'aperçu côte à côte donnent l'essentiel de son confort.

## Conditions de reconsidération

- Si l'essai du porteur ou d'une école montre que les maîtres n'écrivent pas de leçon parce que
  le balisage les arrête.
- Si plus de 1 % des formules livrées ne passent pas la conversion vers Typst.
- Si KaTeX cesse d'être maintenu (aucune sortie pendant dix-huit mois).
- Si le lot 4 demande un rendu que l'arbre de blocs ne porte pas.
