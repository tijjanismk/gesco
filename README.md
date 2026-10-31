# GESCO

ERP de gestion scolaire pour les établissements maliens — scolarité, finance, ressources humaines — installé sur le poste de l'école et fonctionnant sans connexion.

## Construire

```
cargo build --workspace --exclude gesco-app
cargo build -p gesco-app   # sur Windows 10 1809+, WebView2 requis
```

## Tester

```
cargo test --workspace --exclude gesco-app
cargo clippy --workspace --exclude gesco-app --all-targets -- -D warnings
node outils/ci/verifier-liste-blanche.mjs
cargo test --release -p gesco-docs --test moteur   # cible de 400 documents en moins de 2 minutes
```

Les tests d'intégration de `gesco-services` et de `gesco-db` forment un seul exécutable par crate
et tournent dans un seul processus ; si la mémoire manque, `RUST_TEST_THREADS=4`. Un seul fichier :
`cargo test -p gesco-services --test services <fichier>`.

Essais de bout en bout sur l'application réelle (21 scénarios), avec captures d'écran et chronomètre
UC-01 dans `frontend/e2e/captures/` (`gesco-app` compilé en développement : `cargo build -p gesco-app` après
`pnpm build`) :

```
cd frontend
pnpm e2e
```

## Frontend

```
cd frontend
pnpm install --frozen-lockfile
pnpm verifier && pnpm lint && pnpm test && pnpm build
```

Les types de `frontend/src/types/` sont générés depuis Rust :
`GESCO_REGENERER_TYPES=1 cargo test -p gesco-services --test services types_typescript`.

## Documents

`PROJECT_BLUEPRINT.md` décrit le produit et ses décisions ; `ARCHITECTURE_RULES.md` les règles opposables ; `SPEC_PHASE_0_5.md`, `SPEC_PHASE_1.md` et `SPEC_PHASE_2.md` les phases engagées ; `ARCHITECTURE.md` la carte du code ; `ETAT_AVANCEMENT.md` l'avancement ; `docs/matrice-permissions.md` les rôles livrés ; `docs/questions-terrain.md` les questions de terrain et leurs réponses.
