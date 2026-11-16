# Importer une leçon depuis Word : lecture du `.docx` par `zip` et `quick-xml`, déjà compilés pour les imports Excel

**Date** : 16 novembre 2026
**Statut** : proposé le 16 novembre 2026
**Précise** : ADR « format des leçons » (15 novembre 2026) ; étape 2.1 de
`docs/plan-phase-12.md` §3.4 (« édition trop lourde pour un maître peu à l'aise au clavier »)
**Spécification** : `SPEC_PHASE_12_LOT_2.md` (sous réserve de T-124)

## Contexte

- Beaucoup de maîtres ont leurs fiches de préparation dans Word ; les retaper dans l'éditeur du
  lot 1 (Markdown et LaTeX) les décourage.
- Un `.docx` est une archive zip de fichiers XML (`word/document.xml`, styles, images).
- `calamine` (imports Excel) compile déjà `zip` 8.6.0 et `quick-xml` 0.42.0 : ils sont dans
  `Cargo.lock` et sur le disque de compilation. Sur crates.io, le 16 novembre 2026 : `zip` 9.6.0
  (sortie le 15 novembre 2026, trop récente), `quick-xml` 0.42.0 (22 août 2026).
- Les formules de Word (OMML) n'ont pas d'équivalent simple en LaTeX.

## Options

| | 1. Lecture du `.docx` dans le noyau, par `zip` et `quick-xml` | 2. Une crate de conversion Word dédiée | 3. Pas d'import : copier-coller dans l'éditeur |
|---|---|---|---|
| Apprentissage | Moyen : une partie du format Word | Faible | Nul |
| Exploitation | Rien | Une dépendance de plus à suivre | Rien |
| Maturité | Briques très répandues | Crates peu suivies pour la lecture | — |
| Pour un agent de code | Bon : essais sur de vrais fichiers | Moyen | — |
| Plus gros risque | Une mise en forme rare mal rendue | Une crate abandonnée | Le maître renonce |

## Décision

Le noyau lit un `.docx` avec `zip` et `quick-xml`, déclarés en dépendances directes aux versions
que `Cargo.lock` contient déjà, et le convertit en texte de leçon dans un brouillon : titres,
paragraphes, gras, italique, listes, tableaux et images ; une formule de Word devient, pour
l'auteur seul, une note « Formule à retaper ici ».

| Brique | Version | Sortie | Suite prévue |
|---|---|---|---|
| zip | 8.6.0 (déjà dans `Cargo.lock`) | — | Suit `calamine` ; la série 9 attend que `calamine` y passe |
| quick-xml | 0.42.0 (déjà dans `Cargo.lock`) | 22 août 2026 | Suit `calamine` |

Vérifié le 16 novembre 2026 sur crates.io et dans `Cargo.lock`. Aucune crate nouvelle n'entre
dans la compilation.

Ce que la décision fixe : un `.docx` de 20 Mo au plus ; les styles « Titre 1 » à « Titre 3 » (et
leurs noms anglais) donnent les titres ; les images passent par les bornes et le rangement du
lot 1 (une image de plus de 5 Mo est laissée de côté, et l'auteur le lit sous l'aperçu) ; rien du
fichier n'est interprété comme du HTML ; les macros, champs, commentaires et révisions sont
ignorés ; l'ancien format `.doc` n'est pas lu (« Enregistrez ce fichier au format .docx »).

## Justification

Les deux briques sont déjà compilées pour les imports Excel : l'import Word ne coûte ni disque ni
temps de compilation, seulement du code essayable sur de vrais fichiers. Une crate dédiée
ajouterait une dépendance pour une conversion dont on n'utilise qu'une petite partie.

## Conséquences

Positives : un maître part de sa fiche ; aucune dépendance nouvelle à suivre.

Négatives : une mise en page riche (colonnes, zones de texte) se perd ; les formules se retapent ;
`zip` reste sur la série 8 tant que `calamine` y est.

## Contre-argument conservé

Le copier-coller depuis Word dans l'éditeur garderait le texte sans aucun code. Il n'a pas gagné
parce qu'il perd les titres, les tableaux et les images, qui font l'essentiel d'une fiche.

## Conditions de reconsidération

- Les maîtres importent surtout des fichiers pleins de formules : regarder une conversion OMML vers
  LaTeX.
- `calamine` passe à `zip` 9 : suivre la même version.
