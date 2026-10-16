# ARCHITECTURE_RULES.md

**Projet** : ERP de gestion scolaire — Tauri 2 + Rust + React/TypeScript
**Statut** : v0.3
**Dernière révision** : 4 octobre 2026
**Document parent** : `PROJECT_BLUEPRINT.md`

---

## Préambule

Ce document contient les règles **opposables** du projet. Il ne décrit pas ce que fait le logiciel — c'est le rôle du blueprint. Il décrit ce qui est autorisé et ce qui ne l'est pas pendant le développement.

Trois principes d'usage :

- **Une règle se cite en revue.** « Cela viole R-14 » doit suffire à rejeter une modification, sans débat.
- **Une règle se modifie, elle ne se contourne pas.** Si une règle bloque un besoin légitime, on amende la règle par une décision datée et motivée. On ne fait pas d'exception silencieuse.
- **Une règle sans moyen de vérification est un vœu.** Chaque règle indique comment elle est contrôlée : compilation, test automatisé, ou revue humaine.
- **Une règle garde son numéro.** On ajoute à la fin, on ne renumérote jamais : les numéros sont cités dans le code et les revues.

Le préfixe `R-` désigne une règle de ce document, et rien d'autre. Les risques du blueprint (§23) portent `RA-`, les règles de calcul des moyennes `RC-`, les questions `Q-`. Jusqu'au 4 octobre 2026, risques et règles partageaient `R-xx`, ce qui rendait « R-13 » ambigu ; la collision est levée côté blueprint (v0.17).

Codes de sévérité :

| Code | Signification |
|---|---|
| **BLOQUANT** | Une infraction interdit la fusion. Aucune exception. |
| **STRICT** | Une infraction interdit la fusion, sauf dérogation écrite dans le journal des dérogations (§14). |
| **ATTENDU** | Une infraction doit être justifiée en revue et documentée dans le code. |

---

## 1. Séparation des responsabilités

### R-01 — Le frontend ne contient aucune règle métier — BLOQUANT

Aucun calcul de moyenne, de montant, de solde, de rang, d'ancienneté ou d'éligibilité en TypeScript. Aucune décision de transition d'état. Aucune détermination de ce qu'un utilisateur a le droit de faire.

Le frontend peut : afficher, formater, valider la forme d'une saisie pour le confort, masquer un bouton.

**Test de la règle** : si une valeur affichée à l'écran ne provient pas d'une réponse du noyau Rust mais d'un calcul JavaScript, la règle est violée.

*Vérification* : revue. Toute occurrence d'arithmétique sur des montants ou des notes dans `frontend/` est un signal d'alerte.

### R-02 — Le noyau Rust est la source de vérité — BLOQUANT

Le webview est traité comme un client **non fiable**, au même titre qu'un navigateur distant. Toute donnée entrante est revalidée en Rust, quelle qu'ait été la validation côté client.

*Motif* : ADR-001. Sur un poste local, un utilisateur peut ouvrir les outils de développement du webview et appeler les commandes directement.

*Vérification* : revue + tests de permissions.

### R-03 — `gesco-core` n'effectue aucune entrée/sortie — BLOQUANT

La crate `gesco-core` ne dépend ni de Tauri, ni de `sqlx`, ni de `reqwest`, ni du système de fichiers, ni de l'horloge système. Elle reçoit des données, retourne des résultats.

Les dates et heures sont **passées en paramètre**, jamais lues depuis `SystemTime` à l'intérieur du noyau. Sans cela, aucun test déterministe n'est possible sur les calculs liés aux périodes, aux échéances ou à l'ancienneté.

*Vérification* : compilation. Le `Cargo.toml` de `gesco-core` est vérifié en intégration continue contre une liste blanche de dépendances.

### R-04 — Les dépendances entre couches sont unidirectionnelles — BLOQUANT

```
app → services → core
        ↓
       db → core
```

`core` ne connaît personne. `db` ne connaît pas `services`. `services` ne connaît pas `app`. Le frontend ne connaît que la frontière exposée.

*Vérification* : compilation (le graphe Cargo l'impose si les crates sont correctement découpées).

---

## 2. Frontières entre modules

### R-05 — Aucun module ne lit les tables d'un autre module — BLOQUANT

Le module Finance ne fait pas de requête sur la table `inscription`. Il appelle `Scolarite::contexte_tarifaire(inscription_id)`.

*Motif* : c'est la seule discipline qui permettra, dans deux ans, de faire évoluer un domaine sans en casser trois autres.

*Vérification* : revue. Chaque module déclare son interface publique ; tout le reste est privé au sens du langage.

### R-06 — Aucun module ne contourne le service métier d'un autre — BLOQUANT

Pour annuler un versement, on appelle le service d'encaissement. On ne met pas à jour la ligne directement, même depuis un autre service, même « juste pour cette fois ».

*Motif* : les effets de bord (audit, outbox, recalcul du solde, contre-passation) sont attachés au service. Les contourner produit une base incohérente sans erreur visible.

### R-07 — L'interface publique d'un module est explicite et minimale — ATTENDU

Ce qui n'est pas explicitement public est privé. On n'élargit pas une interface pour la commodité d'un appelant sans se demander si le besoin ne révèle pas une erreur de découpage.

---

## 3. Données et persistance

### R-08 — Toute modification de schéma passe par une migration versionnée — BLOQUANT

Aucun changement de structure appliqué à la main, en développement comme en production. Les migrations sont ordonnées, numérotées, dans le dépôt, et jamais modifiées une fois fusionnées.

Une migration déjà livrée à un client se corrige par une **nouvelle migration**, jamais par édition de l'ancienne.

*Vérification* : test d'intégration rejouant l'intégralité des migrations sur une base vide à chaque exécution de la CI.

### R-09 — Les migrations sont réversibles quand c'est raisonnable — ATTENDU

Une migration fournit son inverse sauf si l'opération est destructrice par nature (suppression de colonne contenant des données). Dans ce cas, l'irréversibilité est déclarée explicitement et la migration exige une sauvegarde préalable.

### R-10 — Les identifiants techniques sont des ULID générés côté client — BLOQUANT

Aucune clé primaire auto-incrémentée sur une table métier. Les références métier (matricule, numéro de reçu, numéro de facture) sont des colonnes distinctes.

*Motif* : ADR-004. Un auto-incrément rend la synchronisation impossible sans collision.

*Vérification* : test d'intégration inspectant le schéma.

### R-11 — Les montants sont des entiers de francs CFA — BLOQUANT

`i64`, unité le franc. **Aucun `f32`, `f64`, `number` flottant ou `Decimal` sur un montant, à aucun étage**, y compris dans les types générés vers TypeScript et dans les exports.

Toute division produit explicitement un quotient et un reste. Aucun arrondi implicite.

*Vérification* : revue + test de propriété sur les opérations monétaires.

### R-11bis — Les notes et moyennes sont des entiers en centièmes — BLOQUANT

Une note de 13,50 est stockée `1350`. Tous les calculs de moyennes se font en arithmétique entière, la troncature à deux décimales intervenant par division entière au dernier moment.

**Aucun flottant dans la chaîne de calcul des notes**, à aucun étage, exports et types TypeScript compris.

*Motif* : la troncature exigée par RC-1 est impossible à réaliser de façon déterministe en virgule flottante. Une division en `f64` peut produire 13,359999999 ou 13,360000001 selon l'ordre des opérations, et deux machines afficheraient alors des bulletins différents.

*Vérification* : revue + test de reproductibilité (RC-5).

### R-12 — Toute table métier porte `etablissement_id` — STRICT

Y compris tant que le multi-établissement n'est pas activé (ADR-006). Le filtrage est appliqué par la couche d'accès, pas par chaque requête.

*Dérogation possible* : tables de référentiel strictement partagées (niveaux, cycles, séries nationales).

### R-13 — Toute table métier porte les colonnes d'horodatage et de traçabilité — STRICT

`cree_le`, `cree_par`, `mis_a_jour_le`, `mis_a_jour_par`, et pour les entités synchronisables `identifiant_poste`.

*Motif* : les ajouter après coup sur une base en production coûte très cher (§15.3 du blueprint).

### R-14 — La suppression physique est interdite sur les entités financières et académiques validées — BLOQUANT

Versement, facture, remise, dépense, session de caisse, bulletin de paie, note validée, bulletin publié, inscription : **jamais de `DELETE`**. Annulation par écriture inverse tracée.

Les autres entités métier utilisent la suppression logique (`supprime_le`, `supprime_par`).

*Motif* : ADR-010. C'est la principale protection contre la fraude interne.

*Vérification* : test d'intégration + revue de toute occurrence de suppression.

### R-15 — Aucun déclencheur de base de données porteur de logique métier — STRICT

Pas de `TRIGGER` qui calcule, met à jour un solde ou propage un changement. La logique est en Rust, visible et testable.

*Motif* : un déclencheur est invisible à la lecture du code et se comporte différemment entre SQLite et PostgreSQL.

### R-16 — Les données historiques sont partitionnées par année scolaire — STRICT

Classe, inscription, tarif, période, programme, affectation, résultat portent une référence d'année. Aucune donnée d'une année antérieure n'est écrasée par une donnée de l'année courante.

*Motif* : ADR-012. Un bulletin réimprimé trois ans plus tard doit être identique à l'original.

---

## 4. Transactions et cohérence

### R-17 — Une opération métier est atomique — BLOQUANT

Mutation métier, journal d'audit et écriture `outbox` sont dans **la même transaction**. Si l'une échoue, tout échoue.

Corollaire : valider une inscription et créer sa facture sont une seule transaction. Une inscription validée sans facture est une perte de recette silencieuse.

### R-18 — Aucun effet de bord externe à l'intérieur d'une transaction — BLOQUANT

Envoi de SMS, écriture de fichier PDF, appel réseau, notification : **après le commit**, jamais pendant.

*Motif* : une transaction qui attend une réponse réseau bloque la base. Sur SQLite, elle bloque toute l'application.

### R-19 — Aucune transaction interactive — BLOQUANT

Une transaction ne reste jamais ouverte en attendant une action de l'utilisateur. Les transactions sont courtes et non interruptibles.

*Motif* : §3.1 du blueprint. Une coupure de courant pendant une transaction longue est un risque de corruption.

### R-20 — Les agrégats matérialisés sont toujours recalculables — STRICT

`ResultatPeriode`, soldes, totaux de caisse et statistiques de dashboard sont des caches. Une commande de recalcul intégral existe pour chacun, et son résultat doit être identique au contenu courant.

*Vérification* : test comparant contenu stocké et recalcul complet sur le jeu de données de référence.

---

## 5. Sécurité et permissions

### R-21 — Les permissions sont vérifiées côté Rust, dans la couche frontière — BLOQUANT

Toute commande exposée passe par la garde de permission avant d'atteindre le service. Aucune commande n'est exposée sans permission déclarée.

Le frontend masque des éléments d'interface pour le confort. **Ce masquage n'est jamais une mesure de sécurité.**

*Vérification* : test automatisé énumérant toutes les commandes exposées et vérifiant que chacune déclare une permission.

### R-22 — La permission ne suffit pas : la portée est vérifiée — BLOQUANT

`permission ∧ portée_établissement ∧ portée_ressource`. Un enseignant possédant `grades:create` ne peut noter que les couples (classe, matière) où il est affecté.

*Vérification* : matrice de tests exhaustive rôle × permission × portée.

### R-23 — Aucun secret dans le dépôt — BLOQUANT

Pas de clé, de mot de passe, de jeton ni de certificat privé versionné, même en environnement de développement, même temporairement.

### R-24 — Les données sensibles ne sortent pas sans nécessité — STRICT

Un export, une réponse de commande ou un journal ne contient que les champs nécessaires à l'usage. Pas de sérialisation d'entité complète « par commodité ».

*Motif* : le produit traite des données de mineurs (§3.12 du blueprint).

### R-25 — Les mots de passe et secrets ne sont jamais journalisés — BLOQUANT

Y compris dans les traces de débogage, les messages d'erreur et le journal d'audit. Les structures les contenant implémentent un affichage masqué.

---

## 6. Audit et traçabilité

### R-26 — Toute mutation d'une entité à traçabilité renforcée est auditée — BLOQUANT

Liste au §12.3 du blueprint. L'audit est **non désactivable** sur ces entités, y compris par un administrateur.

### R-27 — Le journal d'audit est en ajout seul et chaîné — BLOQUANT

Aucune mise à jour ni suppression de ligne. Chaque ligne porte l'empreinte de la précédente.

*Vérification* : commande de vérification d'intégrité de la chaîne, exécutée dans les tests et disponible à l'utilisateur.

### R-28 — Une règle métier ne change jamais silencieusement — BLOQUANT

Modifier une règle de calcul (moyenne, imputation, arrondi, décision de passage) exige :
1. une version de règle incrémentée ;
2. la préservation du comportement antérieur pour les données déjà calculées ;
3. une entrée dans le journal des décisions ;
4. un test de non-régression sur l'ancien comportement.

*Motif* : recalculer un bulletin de 2024 avec les règles de 2026 produit un document faux. C'est inacceptable sur une pièce administrative.

---

## 7. États et workflows

### R-29 — Les processus importants utilisent des états nommés, pas des booléens — STRICT

Pas de `est_valide`, `est_paye`, `est_annule` dispersés. Un champ `statut` avec des valeurs explicites, et une table de transitions autorisées.

*Motif* : ADR-011. Un booléen ne dit ni qui a agi, ni depuis quel état, ni si le retour est permis.

### R-30 — Les transitions sont centralisées dans `gesco-core` — BLOQUANT

Aucun service ni aucune requête ne modifie un champ `statut` directement. On appelle la fonction de transition, qui vérifie la précondition, l'acteur et enregistre la trace.

### R-31 — Chaque transition déclare ses effets de bord — ATTENDU

Documentés à côté de la transition : ce qu'elle crée, ce qu'elle verrouille, ce qu'elle notifie. Un effet de bord non déclaré est un bug de conception.

---

## 8. Tests

### R-32 — Toute nouvelle fonctionnalité est accompagnée de tests — BLOQUANT

Sans exception. Une fonctionnalité sans test est considérée comme non livrée.

### R-33 — Tout calcul financier ou de note est couvert par des tests unitaires dans `gesco-core` — BLOQUANT

Y compris les cas limites : montant nul, versement excédentaire, élève sans note, division non entière, absence non justifiée, remise supérieure au dû.

### R-34 — Tout bug corrigé produit un test de non-régression — BLOQUANT

Le test reproduit le bug avant correction. On écrit le test qui échoue, puis on corrige.

### R-35 — Les invariants financiers sont testés par propriétés — STRICT

Au minimum :
- somme des imputations d'un versement ≤ montant du versement ;
- aucun montant négatif hors contre-passation explicite ;
- total théorique de caisse = fonds initial + entrées − sorties ;
- toute moyenne est comprise entre 0 et le barème ;
- rejouer un encaissement identique ne double pas la recette.

### R-36 — Une modification ne casse pas les autres modules — BLOQUANT

La suite complète passe avant toute fusion. Un test rendu ignoré pour faire passer la CI est une infraction, pas une solution.

---

## 9. Frontend

### R-37 — Les types d'entités métier sont générés, jamais écrits à la main — BLOQUANT

`frontend/src/types/` est un répertoire généré. Toute modification manuelle est rejetée en revue et écrasée au build.

*Motif* : ADR-007.

### R-38 — L'accès aux données passe par la couche d'abstraction — BLOQUANT

Aucun appel direct à `invoke()` de Tauri ni à `fetch()` depuis un composant. Tout transite par `frontend/src/api/`, qui décide entre commande locale et appel HTTP.

*Motif* : ADR-014. Sans cette discipline, le mode serveur local devient impossible à introduire.

### R-39 — Aucune chaîne visible en dur — STRICT

Toute chaîne affichée passe par le catalogue i18n, même si une seule locale est livrée (ADR-015).

*Dérogation possible* : outils de développement et écrans de diagnostic non destinés à l'utilisateur final.

### R-40 — Aucune modale imbriquée — ATTENDU

Une modale ne s'ouvre pas depuis une modale. Utiliser un panneau latéral ou un assistant multi-étapes.

*Motif* : §19.1 du blueprint. C'est une impasse pour un utilisateur non technique.

### R-41 — Toute liste est exportable — STRICT

Chaque écran affichant un tableau propose l'export Excel et CSV. Sans exception.

*Motif* : §14.2. Les utilisateurs viennent d'Excel et un logiciel qui les en prive est rejeté.

### R-42 — Les parcours à fort volume sont utilisables au clavier seul — STRICT

Saisie de notes, encaissement et recherche globale se réalisent de bout en bout sans souris.

*Motif* : §3.4 et §19.1.

---

## 10. Documents et exports

### R-43 — Un seul moteur de génération documentaire — BLOQUANT

Aucun module ne crée son propre générateur de PDF. Tout passe par `gesco-docs`.

*Motif* : ADR-008.

### R-44 — Aucune génération de PDF dans le webview — BLOQUANT

Ni impression navigateur, ni bibliothèque JavaScript. Composition en Rust uniquement.

*Motif* : reproductibilité entre machines et performance sur les lots de plusieurs centaines de documents.

### R-45 — Tout document produit enregistre la version du modèle utilisé — STRICT

Sans elle, une réimpression ultérieure produit un document différent de l'original.

### R-46 — Les numéros de documents officiels sont séquentiels sans trou — BLOQUANT

Reçus et factures. Un numéro attribué puis abandonné donne un document **annulé**, pas un trou dans la séquence. L'attribution est transactionnelle et centralisée.

---

## 11. Import et sauvegarde

### R-47 — Aucun import ne s'applique sans aperçu ni confirmation — BLOQUANT

Analyse, validation, aperçu avec rapport d'erreurs, confirmation explicite, puis application transactionnelle. L'utilisateur peut n'appliquer que les lignes valides.

### R-48 — Une sauvegarde précède toute opération à risque — BLOQUANT

Clôture d'année, import en masse, migration de schéma, restauration. Automatique, sans que l'utilisateur ait à y penser.

### R-49 — La restauration est testée automatiquement — STRICT

Vérification mensuelle par restauration dans un espace temporaire, avec alerte en cas d'échec.

*Motif* : §18.3. Une sauvegarde jamais restaurée n'est pas une sauvegarde.

### R-57 — Une sauvegarde se restaure sans aucun secret présent sur la machine d'origine — BLOQUANT

*Ajoutée le 4 octobre 2026 (v0.3), à la suite de Q-38.* Le fichier de sauvegarde et la phrase de secours imprimée (blueprint §17.3) suffisent à reconstituer la base sur une machine vierge. Ni le trousseau du système, ni un fichier de configuration du poste, ni le mot de passe d'un utilisateur ne sont nécessaires.

*Motif* : UC-07 — disque mort, ordinateur volé. Une sauvegarde dont la clé est liée à la machine qui l'a produite est indéchiffrable précisément le jour où l'on en a besoin.

*Vérification* : test d'acceptation 9 de `SPEC_PHASE_0_5.md` — restauration sur une machine vierge avec, pour seul intrant, le fichier de sauvegarde et la phrase de secours ; empreinte de la base comparée à l'original.

---

## 12. Documentation et processus

### R-50 — Toute décision architecturale importante est documentée — BLOQUANT

Un fichier daté dans `docs/adr/`, au format : décision, motif, conséquence, alternative écartée. Une décision prise en conversation et non écrite n'existe pas.

### R-51 — Le blueprint est mis à jour avant le code, pas après — STRICT

Si une modification contredit `PROJECT_BLUEPRINT.md`, on amende le blueprint dans la même livraison. Un blueprint périmé est pire qu'aucun blueprint : il induit en erreur.

### R-52 — Une question métier sans réponse se pose, elle ne s'invente pas — BLOQUANT

Face à une règle métier non spécifiée, on ne choisit pas « ce qui semble logique ». On ajoute la question à §24 du blueprint et on attend la décision.

*Motif* : une règle inventée est indétectable ; elle produit des résultats plausibles et faux, découverts des mois plus tard par un utilisateur.

### R-53 — Une modification déclare son périmètre d'impact — ATTENDU

Avant de coder : quels fichiers, quels modules, quels effets. Aucune modification opportuniste hors du périmètre annoncé, même si elle paraît évidemment souhaitable — elle fait l'objet d'une autre livraison.

### R-54 — Le code est écrit dans le vocabulaire du domaine, en français — ATTENDU

*Amendée le 4 octobre 2026 (v0.3).* La v0.2 imposait des identifiants en anglais. Le blueprint v0.16 (§5.5, règle 4 : `Inscription`, `Versement`, `moyenne_periode`), la structure des dossiers (§6 : `commun/`, `scolarite/`, `eleve/`), la spécification du socle (`Montant`, `Note`, `Horodatage`) et le code déjà écrit sont tous en français. La règle est alignée sur la pratique décidée, et non l'inverse.

Identifiants, types, fonctions, modules, tables et colonnes portent le **nom que l'école utilise** : `Inscription`, `Versement`, `SessionCaisse`, `moyenne_periode`, `composition`. Chaînes visibles, documentation, ADR, commentaires et messages de commit sont en français.

Trois exceptions, toutes bornées :
1. le **catalogue de permissions** reste en anglais (`grades:create`, blueprint §11.2), parce que ses codes doivent rester courts et stables ;
2. les **noms imposés par l'écosystème** (traits `Display`, `From`, méthodes `new`, `len`, conventions de `sqlx`, `serde`, React) sont conservés tels quels ;
3. les **termes techniques sans usage français établi** (`id`, `hash`, `token`, `outbox`, `payload`) restent en anglais plutôt que de recevoir une traduction forcée.

Pas de franglais dans un même nom : `valider_inscription`, jamais `validateInscription` ni `valider_enrollment`.

*Vérification* : revue. `cargo clippy` et ESLint ne vérifient pas la langue ; une liste des termes du domaine, tenue dans `docs/modele-donnees/glossaire.md`, sert de référence.

---

## 13. Versions et qualité du code

*Section ajoutée le 4 octobre 2026 (v0.3).* Ces deux règles reprennent les consignes des §5.4 et §5.5 du blueprint. Elles avaient été numérotées R-47 et R-48 dans `prompts/PROMPT_ONESHOT_GESCO.md`, numéros déjà pris par l'import et la sauvegarde ; elles reçoivent ici leurs numéros définitifs.

### R-55 — Les dépendances démarrent sur leur dernière version stable, puis sont figées — STRICT

Rust et chaque crate, chaque paquet npm, sont pris à leur dernière version stable au démarrage du projet (plancher : tableau du §5.4 du blueprint), puis figés par `rust-toolchain.toml`, `Cargo.lock` et `pnpm-lock.yaml`. Toute montée de version est volontaire, passe par la CI et apparaît dans un commit dédié.

*Dérogation permanente, documentée* : `tauri-specta` épinglé en `=2.0.0-rc.25` tant que la 2.0 stable n'existe pas (ADR-007 en dépend).

*Vérification* : CI — `cargo outdated` et `pnpm outdated` en rapport, échec si un verrou est absent ou si la toolchain diffère de `rust-toolchain.toml`.

### R-56 — Aucun signe de « vibe coding » — BLOQUANT

Les douze règles du §5.5 du blueprint s'appliquent au code Rust, TypeScript, SQL, Typst, à la CI et à la documentation : pas de commentaire qui paraphrase, pas de code mort ni de démonstration, pas de `TODO` / `FIXME` / `todo!()` / `unimplemented!()` sur la branche principale, pas de nom générique, pas de `unwrap()` / `expect()` hors tests, pas de `any` ni de `@ts-ignore` sans justification écrite, pas de duplication, pas de dépendance inutilisée, pas de fichier de gabarit, pas de texte de remplissage, README sobre, messages de commit en français au présent.

*Vérification* : `cargo clippy -D warnings` avec `clippy::unwrap_used`, `clippy::expect_used`, `clippy::todo`, `clippy::unimplemented`, `clippy::dbg_macro`, `clippy::print_stdout` ; ESLint `no-console`, `@typescript-eslint/no-explicit-any`, `@typescript-eslint/ban-ts-comment` ; recherche CI de `TODO`, `FIXME`, `lorem`, émojis ; `cargo machete` et `knip`.

*Tolérance bornée* : les `todo!()` d'un squelette sont admis sur une branche de travail tant que l'étape correspondante est « en cours » dans `ETAT_AVANCEMENT.md` ; ils doivent avoir disparu avant la validation de l'étape.

---

## 14. Dérogations

Une dérogation à une règle **STRICT** ou **ATTENDU** est consignée ici : date, règle concernée, portée exacte, motif, échéance de résorption.

Une règle **BLOQUANT** ne se déroge pas. Si elle doit céder, c'est la règle qui est amendée, par une décision documentée en `docs/adr/`.

| Date | Règle | Portée | Motif | Résorption |
|---|---|---|---|---|
| 2026-10-04 | R-55 | `typescript` 6.0.3 au lieu de 7.0.2 (dernière stable) | `typescript-eslint` 8.71 exige TypeScript < 6.1 ; sans lui, les contrôles de R-56 (`no-explicit-any`, `ban-ts-comment`) tombent | Montée en 7.x dès qu'une version de `typescript-eslint` la prend en charge |
| 2026-10-04 | R-55 | `specta-typescript` et `specta-serde` épinglés en `=0.0.12` | Seules versions compatibles avec `specta` `=2.0.0-rc.25`, lui-même imposé par `tauri-specta` (dérogation permanente ci-dessus) | Avec `specta` 2.0 stable |
| 2026-10-04 | R-41 | Export CSV seul, sans Excel, sur les tableaux de la phase 0.5 | L'export Excel passe par `rust_xlsxwriter` côté noyau (blueprint §5.2), pas encore intégré | **Close le 2026-10-08** (phase 2, étape 7 : `exporter_xlsx`, bouton Excel sur toutes les listes) |

---

## 15. Journal des révisions

| Version | Date | Modification |
|---|---|---|
| v0.1 | 2026-08-27 | Création |
| v0.2 | 2026-08-27 | Statut porté en en-tête sans entrée de journal ; aucun écart de contenu identifiable avec la v0.1 |
| v0.3 | 2026-10-04 | Préambule : préfixes `R-` / `RA-` / `RC-` / `Q-`, interdiction de renuméroter ; R-54 amendée (code en français, trois exceptions) ; R-55 et R-56 ajoutées (§13, ex-« R-47/R-48 » du prompt) ; R-57 ajoutée (sauvegarde restaurable sans secret machine, Q-38) ; §13→§14 Dérogations, §14→§15 Journal |
