# Architecture — GESCO

Carte de l'état du code au 9 octobre 2026 : phase 0.5 (étape 13 en cours : SQLCipher reporté, essai sur machine vierge à faire), phase 1 validée, phase 2 livrée et en attente de validation. Les documents de référence font foi ; cette carte les résume.

## Stack

- Noyau : Rust, édition 2024, toolchain épinglée en 1.99.0 (`rust-toolchain.toml`).
- Poste : Tauri 2.12 (`gesco-app`), une seule commande exposée, `executer` (ADR du 4 octobre 2026) ; dialogues du système d'ouverture et d'enregistrement, qui ne rendent qu'un chemin.
- Base : SQLite en WAL par `sqlx` 0.9 ; SQLCipher prévu, pas encore compilé (voir `ALERTES.md`).
- Documents : Typst 0.15 embarqué dans `gesco-docs`, police Charis SIL embarquée ; QR codes SVG (`qrcode`) ; classeurs Excel (`rust_xlsxwriter`) ; lecture des tableurs (`calamine`).
- Frontend : React 19.3, Vite 8, TypeScript 6.0, Tailwind 4.3 aux jetons du §19.6, TanStack
  Query, polices Fira Sans et Fira Sans Condensed embarquées, catalogue i18n `fr-ML`.
- Types TypeScript générés depuis Rust par `specta` (ADR-007), vérifiés par un test.
- Essais de bout en bout : Playwright 1.63 pilote la vraie application par le port de débogage de WebView2 ; 21 scénarios, 49 captures dans `frontend/e2e/captures/`.

## Crates et sens des dépendances

```
gesco-app ──► gesco-services ──► gesco-core
                    ├──────────► gesco-db ──► gesco-core
                    └──────────► gesco-docs
gesco-sms ──► gesco-core
```

`gesco-server` (serveur cloud) et `gesco-sync` (consommation de l'outbox), deux squelettes vides,
sont retirés de l'espace de travail le 31 octobre 2026 : ils étaient compilés à chaque fois pour
rien. Ils reviendront à la phase 11 ; l'historique git garde leur point de départ.

| Crate | Rôle | État |
|---|---|---|
| `gesco-core` | Règles pures, sans E/S (R-03). Socle : types fondamentaux, permissions et composition des rôles livrés (version 2), politique de mot de passe, année scolaire, audit chaîné, événements, numérotation, clé de données et enveloppes, phrase de secours, manifeste et rotation des sauvegardes, licence signée Ed25519. Scolarité (`scolarite/`) : niveaux, séries, cycles et classes ; personnes, élèves, responsables et qualités des liens ; téléphones canoniques ; matricule interne (règles de l'école) ; recherche tolérante et doublons probables ; machine à états des inscriptions et conditions de validation ; fusion de fiches ; lecture d'un fichier d'élèves | Phases 0.5, 1 et 2 |
| `gesco-db` | Migrations (0001 socle, 0002 numéros abandonnés, 0003 licence, 0004 composition des rôles livrés, 0005 logo, 0006 référentiel, 0007 personnes, 0008 inscriptions), dépôts filtrés par établissement, recherche paginée du journal, unité de travail, instantané (`VACUUM INTO`), empreinte du contenu | Phases 0.5, 1 et 2 |
| `gesco-docs` | Catalogue de modèles Typst (`essai`, `certificat_phrase`, `fiche_inscription`), composition avec pièces jointes, QR code, variante d'établissement, lot parallèle avec reprise, classeur Excel d'une liste (`tableur`) | Phases 0.5 et 2 |
| `gesco-services` | Noyau applicatif, authentification et session rapide, garde, comptes, rôles, mise à niveau des bases installées, années scolaires, fiche d'établissement, numérotation, documents, clés, archive, sauvegarde, restauration, administration et journal, licence ; scolarité : référentiel, élèves et responsables, inscriptions, fusion, export Excel, import des élèves ; frontière et contrat des commandes (82) | Phases 0.5, 1 et 2 |
| `gesco-app` | Binaire Tauri : fenêtre, commande `executer`, trousseau Windows, empreinte machine, tâches périodiques, mode d'essai (`GESCO_DOSSIER_ESSAI`) ; installeur NSIS configuré | Phase 0.5 |
| `outils/licence` (`gesco-licence`) | Outil de l'éditeur, jamais livré : clés de signature, émission des licences | Phase 0.5 |
| `gesco-sms` | Frontière posée, inactive | — |

## Frontend

`frontend/src/api/` est le seul accès au noyau (R-38) : transport Tauri actif, transport HTTP
déclaré, dialogues du système (`fichiers.ts`). `frontend/src/types/` est généré (R-37).
`frontend/src/i18n/` porte le catalogue ; `frontend/src/modules/` les écrans : `accueil`,
`administration`, `sauvegarde`, `scolarite` (niveaux et cycles, classes, élèves et fiche,
responsables, inscriptions et assistant UC-01, fusion, import) ; `frontend/src/app/` l'aiguillage,
la session, la recherche globale (`Ctrl+K`) et la navigation filtrée par permissions. Le composant
`Tableau` exporte toute liste en Excel et en CSV (R-41).

## Points d'entrée

- `crates/gesco-app/src/main.rs` — application du poste.
- Serveur cloud (`gesco-server`) : retiré le 31 octobre 2026, il reviendra à la phase 11.
- `frontend/src/main.tsx` — interface.

## Documents de référence

- `PROJECT_BLUEPRINT.md` v0.22 — le produit, ses décisions, Q-01 à Q-47.
- `ARCHITECTURE_RULES.md` v0.3 — règles R-01 à R-57 et dérogations (§14).
- `SPEC_PHASE_0_5.md` v1.2, `SPEC_PHASE_1.md`, `SPEC_PHASE_2.md` v1.4 — spécifications des phases engagées.
- `docs/adr/` — décisions datées (R-50) ; `docs/questions-terrain.md` — questions de terrain et réponses.
- `ETAT_AVANCEMENT.md` — avancement ; `ALERTES.md` — écarts à surveiller ; `DOMAINE.md` — règles
  métier cibles.
