# Optimisations relevées le 31 octobre 2026

Relecture de `main` (version 1.2.5) en lecture seule, sans compilation. Les lignes renvoient à
la branche 1.2.x (commit e023ad1). Les durées sont des estimations : rien n'a été mesuré sur un
vrai poste. Les constats marqués « vérifié » ont été relus dans le code.

Ordre conseillé : d'abord le disque (§1), pour pouvoir compiler à nouveau ; puis, en une seule
compilation, le démarrage, la fermeture, la mémoire des PDF, les index et la saisie des notes
(§2), avec les tranches d'impayés réglables déjà décidées.

## 1. Compilation et disque

D-1 à D-4 sont faits le 31 octobre 2026 : compilation complète et 501 tests depuis un `target`
vide, qui pèse ensuite 2,6 Go.

| N° | Constat | Correction | Effort | Gain estimé |
|---|---|---|---|---|
| D-1 | `crates/gesco-services/tests/` : 45 fichiers, donc 45 exécutables de test d'environ 140 Mo chacun (vérifié : 45 fichiers). | Un seul binaire : `autotests = false`, `[[test]] name = "services"`, `tests/services/main.rs` qui déclare les modules ; `use crate::commun::*`. Idem pour les 2 tests de gesco-db. Garder `gesco-docs --test moteur` à part. Mettre à jour README, `docs/matrice-permissions.md`. | ~1 h | ~6 Go et 5 à 15 min par vérification |
| D-2 | Chaque montée de version recrée tous les artefacts du dépôt dans `target/debug/deps` sans effacer les anciens ; l'élagage se fait à la main. | Script `outils/elaguer-target.ps1` : `cargo clean -p` des crates du dépôt seulement. | 15 min | ~0,5 à 6 Go par version |
| D-3 | README.md:18 lance `cargo test --release -p gesco-docs --test moteur` : tout typst recompilé en release (`lto = "thin"`, `codegen-units = 1`). | Le laisser à la CI (déjà dans `ci.yml`) et avant un `tauri build`. | 5 min | quelques Go, 10 à 20 min |
| D-4 | `gesco-server` (`fn main() {}`, vérifié) et `gesco-sync` (1 ligne) sont compilés, avec axum. | Les retirer de l'espace de travail jusqu'à la phase 11. | 10 min | ~0,3 Go, ~1 min |
| D-5 | Profils `dev` et `test` différents : les crates du dépôt sont compilées deux fois (tests, puis `gesco-app` pour Playwright). | Aligner les profils pour les crates du dépôt ; mesurer avant (le 18 octobre, sans incrémental, gesco-app a pris 58 min). | 30 min à 2 h | 0,5 à 1,5 Go |
| D-6 | Lieur MSVC `link.exe` (échecs LNK1180 et LNK1140 par manque de mémoire). | Essayer `rust-lld` pour les tests d'abord. | 10 min | liens 2 à 5 fois plus rapides |
| D-7 | Dépendances en `opt-level = 0` en dev. | `opt-level = 1` pour `"*"`. | 1 ligne | exécutables 20 à 40 % plus petits |
| D-8 | `-j 2` ou `-j 4` tapé à la main. | `.cargo/config.toml` avec `[build] jobs = 4`. | 2 min | fiabilité |
| D-9 | Hors du dépôt : toolchain Rust 1.97.1 inutilisée (695 Mo), `target` de gescom (3,2 Go), store pnpm (2,1 Go), anciens installeurs 1.2.4 et poste-essai (451 Mo), navigateurs Playwright de Python (719 Mo). | Nettoyage après accord du porteur. | 10 min | 4 à 7 Go |
| D-10 | sqlx avec la fonctionnalité `json`, peut-être inutile. | La retirer ; le compilateur dira si elle sert. | 5 min | faible |

Écartés : Playwright (pas de navigateur téléchargé pour GESCO, captures réécrites sous le même
nom), `split-debuginfo`, `debug` et `incremental` en test (déjà réglés), doublons de Cargo.lock
(viennent de tauri et typst).

## 2. Ce que l'école sentira sur un vieux PC

| N° | Constat | Correction | Effort |
|---|---|---|---|
| P-1 | `PRAGMA integrity_check` complet à chaque démarrage (vérifié : `gesco-db/src/base.rs:194`, `frontiere.rs:1334`) ; 10 à 60 s sur une grosse base avant que la fenêtre réponde. | `quick_check` au démarrage ; la vérification complète reste faite chaque mois (`sauvegarde.rs:510`). | petit |
| P-2 | Sauvegarde complète à chaque fermeture, même sans modification (vérifié : `frontiere.rs:1408`) ; hachages et copies bloquants. | La sauter si la dernière sauvegarde est récente et que la chaîne d'audit n'a pas bougé ; E/S dans `spawn_blocking`. | petit |
| P-3 | Le cache mémoire de Typst (`comemo`) n'est jamais vidé (vérifié : aucun `comemo` dans le dépôt) ; la mémoire grossit toute la journée. | `comemo::evict(10)` après chaque composition. | petit |
| P-4 | Chaque saisie de note charge toutes les fiches élèves de l'école pour vérifier l'appartenance à la classe (`evaluations.rs:474`, `inscriptions.rs:195`, recherche linéaire). Même chemin : appel, conduite, bulletins. | Requête de dépôt « inscriptions validées de la classe » (index `inscription_classe` existant) ; `HashMap` dans `lister`. | petit à moyen |
| P-5 | Dernière empreinte d'audit lue à chaque écriture par `ORDER BY rowid DESC LIMIT 1` sans index adapté (vérifié : seuls `journal_audit_entite` et `journal_audit_horodatage`). À confirmer par `EXPLAIN QUERY PLAN`. | `CREATE INDEX journal_audit_chaine ON journal_audit(etablissement_id)`. | petit, migration |
| P-6 | Index manquants : versements par date (`versements.rs:367`), absences et relevés du jour (`tableau.rs:91/98/114`), évaluations par année (`tableau.rs:129`), `mouvement_caisse.versement_id`, paiements du personnel. | Une migration de 5 ou 6 index ; `PRAGMA optimize` à la fermeture. | petit, migration |
| P-7 | Tableau de bord : 2 000 à 3 000 requêtes à l'ouverture (impayés, heures des vacataires, noms du personnel). | Voir P-8, P-9, P-10. | — |
| P-8 | Impayés : 3 requêtes par facture ouverte (`recouvrement.rs:247`), plus un chargement complet des élèves par année. Sert aussi au tableau de bord et aux relances SMS. | 3 requêtes par année, regroupées par facture. Comparer avant/après sur l'école d'essai. | moyen |
| P-9 | Heures des vacataires : l'emploi du temps entier rechargé pour chaque vacataire, deux fois sur le tableau de bord (`heures.rs:154/159`). | Charger le contenu une fois ; un comptage sans calcul des minutes pour le tableau. | petit |
| P-10 | Noms du personnel lus un par un (`conges.rs:248`, `paie_mois.rs:194/516`, `documents_paie.rs:161`…). | `personnel::membres_nommes` (une jointure). | petit |
| P-11 | Recherche d'élève : sans délai de frappe, recopiée sur 7 écrans sous 7 clés de cache ; le noyau relit toutes les fiches à chaque lettre (`eleves.rs:402`). | Hook `useRechercheEleves` avec 250 ms de délai, composant `ChoixEleve` ; côté noyau, lire les inscriptions des seuls élèves trouvés. | petit à moyen |
| P-12 | Remplissage des appréciations (ajouté le 31 octobre) : un appel `appreciation_saisir` par case, jusqu'à ~400 transactions. | Commande `appreciations_remplir` en une transaction. | moyen |
| P-13 | Chaque appréciation saisie recharge toute la classe (`Bulletins.tsx`, `surChange`). | Mettre à jour le cache avec le bulletin rendu (`setQueryData`), comme Notes.tsx. | petit |
| P-14 | Convocations chargent toutes les inscriptions de l'année ; la clé `["inscrits"]` n'est jamais invalidée. | `inscription_id` dans `EleveTrouve` ; une seule clé. | petit |
| P-15 | Cache de l'écran : `staleTime` à 0 et écran remonté à chaque clic du menu ; `session_courante` demandée à 25 endroits. | `staleTime` long pour les données de référence ; hooks `useSessionCourante`, `useAnneeCourante`. Corriger P-14 avant. | moyen |
| P-16 | `Tableau` affiche toutes les lignes (jusqu'à 9 000 cellules). | Pagination de 100 lignes ; les exports gardent tout. | petit à moyen |
| P-17 | Tout l'écran dans un seul fichier JS de 947 Ko. | `React.lazy` par écran. | moyen |
| P-18 | Bulletins : calcul de la classe fait deux fois par clic ; une requête par évaluation et par élève (`bulletins.rs:247/360/612/686`). | Réutiliser le classement ; requêtes par classe et période. | moyen |
| P-19 | Vérification Argon2 pendant que le verrou d'écriture est tenu (`authentification.rs:53`). | Vérifier hors transaction, dans `spawn_blocking`. | petit |
| P-20 | Mise à niveau des données refaite à chaque lancement (`mise_a_niveau.rs`, des centaines de requêtes). | La sauter si la version n'a pas changé (comme `COMPOSITION_ROLES_LIVRES`). | petit |
| P-21 | SMS : appels série bloquants dans la tâche asynchrone, connexion tenue pendant l'envoi (jusqu'à 30 s), `AT` toutes les 5 s. | `spawn_blocking` ou fil dédié au modem ; `verifier()` seulement après une erreur. | moyen |
| P-22 | Clé de la base passée comme phrase : dérivation à chaque nouvelle connexion (supposé). | `idle_timeout(None)` et `max_lifetime` long sur les pools. | petit |

## 3. Croissance avec les années

| N° | Constat | Correction | Effort |
|---|---|---|---|
| C-1 | Les PDF s'accumulent dans `donnees/documents` sans limite et ne sont jamais relus ; le logo (jusqu'à 1 Mo) est mis en pleine résolution dans chaque reçu. | Réduire le logo à ~400 px à l'enregistrement (ADR : nouvelle dépendance) ; politique de purge à décider avec le porteur (valeur de preuve). | petit / moyen |
| C-2 | L'empreinte de contenu de la sauvegarde charge les tables entières en mémoire, photos en hexadécimal (~600 Mo pour une grande école). | Lecture en flux, blobs hachés tels quels ; versionner le calcul d'empreinte. | moyen |
| C-3 | Paie : chaque saisie recalcule tout le personnel et relit tous les bulletins validés de chaque membre (`paie_mois.rs:708`). | Recalcul du seul membre ; cumuls en une requête. Couvrir par les cas de référence. | moyen |
| C-4 | Trésorerie : chaque compte relit tous les mouvements depuis le début (`livres.rs:466`). | Charger une fois ; plus tard, solde d'ouverture par année. | petit / gros |
| C-5 | Audit : chaque bulletin recalculé écrit son contenu complet avant et après. | Ne garder que l'empreinte ou les champs changés. | moyen |
| C-6 | Chaque document relit la fiche, le logo et les réglages (`documents.rs:117-216`). | Cache de la variante dans `AtelierDocuments`. | petit |
| C-7 | Paramètres enregistrés un par un (`LargeursColonnes.tsx:83`, `ReglagesBulletin.tsx:59`) : sauvegarde non atomique. | Commande `parametres_definir` qui prend une liste. | petit |
| C-8 | Permissions relues en base à chaque commande (`garde.rs:67`) et 20 fois ailleurs. Gain faible. | Cache par session ; à faire en dernier (sécurité). | moyen |

## Petits gains d'écran

- Polices : 56 fichiers livrés (cyrillique, grec, `.woff`) ; importer le latin et le latin étendu
  seulement (vérifier ɛ, ɔ, ɲ). Gain ~0,8 Mo sur l'installeur.
- PDF en base64 décodé octet par octet (`ApercuPdf.tsx`) ; boucle simple, ou octets bruts côté noyau.
- `invalidateQueries()` global dans `Niveaux.tsx:109`.
- Code mort : `Moteur::composer_lot` (`gesco-docs/src/lot.rs`) n'est appelé nulle part.

## Déjà bien

Bulletins d'une classe en un seul PDF ; polices réduites aux glyphes utilisés ; moteur Typst
partagé ; absences et imports en une transaction ; file SMS légère ; tableau de bord qui rend ses
connexions ; dépendances de l'écran légères.

## À mesurer avant d'agir

Sur la base de l'école d'essai : la durée de `integrity_check`, le pic de mémoire d'une
sauvegarde, la mémoire du processus après 200 compositions, et `EXPLAIN QUERY PLAN` de la
dernière empreinte d'audit.
