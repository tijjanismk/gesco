# Versions et branches

Le dépôt est sur GitHub (`tijjanismk/gesco`, public) ; chaque version y est publiée avec son
installeur et le manuel d'utilisation en PDF.

- `1.2.x` : la version 1.2 qui est dans les écoles (SMS aux familles, guichet, modules de
  l'école). Elle ne reçoit que des corrections. Chaque correction livrée porte une étiquette :
  `v1.2.1`, `v1.2.2`… et son installeur.
- `1.1.x` : la version 1.1, gardée telle qu'elle est sortie ; une école en 1.1 installe la 1.2
  par-dessus.
- `1.0.x` : la version 1.0, gardée telle qu'elle est sortie. Une école en 1.0 installe la 1.1
  par-dessus ; une correction ne se fait plus que sur `1.2.x`.
- `developpement` : la suite de la version 2 (rapports et statistiques quand les formulaires
  officiels seront là, e-learning). Toute correction faite sur `1.2.x` y est reportée aussitôt
  (`git merge 1.2.x`), pour qu'elle ne revienne pas dans la version suivante.
- `main` : la dernière version publiée ; elle avance à chaque étiquette.

Pour corriger la version en service : `git switch 1.2.x`, corriger, tester, monter la version
(1.2.1) dans `Cargo.toml`, `crates/gesco-app/tauri.conf.json` et `frontend/package.json`,
valider, étiqueter, construire l'installeur ; puis `git switch developpement` et
`git merge 1.2.x`.

Jamais dans le dépôt : `target/`, la clé privée de l'éditeur, les licences émises.
