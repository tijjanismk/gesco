# Versions et branches

Le dépôt est local : il n'a pas de dépôt distant, rien n'est poussé.

- `1.1.x` : la version 1.1 qui est dans les écoles. Elle ne reçoit que des corrections. Chaque
  correction livrée porte une étiquette : `v1.1.1`, `v1.1.2`… et son installeur.
- `1.0.x` : la version 1.0, gardée telle qu'elle est sortie. Une école en 1.0 installe la 1.1
  par-dessus ; une correction ne se fait plus que sur `1.1.x`.
- `developpement` : la suite, qui deviendra la version 2 (SMS aux familles, rapports et
  statistiques, e-learning). Toute correction faite sur `1.1.x` y est reportée aussitôt
  (`git merge 1.1.x`), pour qu'elle ne revienne pas dans la version suivante.
- `main` : la dernière version publiée ; elle avance à chaque étiquette.

Pour corriger la version en service : `git switch 1.1.x`, corriger, tester, monter la version
(1.1.1) dans `Cargo.toml`, `crates/gesco-app/tauri.conf.json` et `frontend/package.json`,
valider, étiqueter, construire l'installeur ; puis `git switch developpement` et
`git merge 1.1.x`.

Jamais dans le dépôt : `target/`, la clé privée de l'éditeur, les licences émises.
