# Versions et branches

Le dépôt est local : il n'a pas de dépôt distant, rien n'est poussé.

- `1.0.x` : la version 1.0 qui est dans les écoles. Elle ne reçoit que des corrections. Chaque
  correction livrée porte une étiquette : `v1.0.1`, `v1.0.2`… et son installeur.
- `developpement` : la suite (SMS, rapports, modules suivants), qui deviendra la 1.1. Toute
  correction faite sur `1.0.x` y est reportée aussitôt (`git merge 1.0.x`), pour qu'elle ne
  revienne pas dans la version suivante.
- `main` : la dernière version publiée ; elle avance à chaque étiquette.

Pour corriger la V1 : `git switch 1.0.x`, corriger, tester, monter la version (1.0.1) dans
`Cargo.toml`, `crates/gesco-app/tauri.conf.json` et `frontend/package.json`, valider, étiqueter,
construire l'installeur ; puis `git switch developpement` et `git merge 1.0.x`.

Jamais dans le dépôt : `target/`, la clé privée de l'éditeur, les licences émises.
