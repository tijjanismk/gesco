# SPEC_PHASE_0_5.md — Socle technique

**Projet** : ERP de gestion scolaire — Tauri 2 + Rust + React/TypeScript
**Phase** : 0.5 — socle technique
**Statut** : v1.2 — spécification exécutable
**Documents parents** : `PROJECT_BLUEPRINT.md` v0.18, `ARCHITECTURE_RULES.md` v0.3
**Date** : 27 août 2026, révisée le 4 octobre 2026 (§17)

---

## 0. Objet et critère de fin

### 0.1 Ce que produit cette phase

Une application qui **ne fait rien de métier** mais qui démarre, s'authentifie, journalise, sauvegarde et se restaure. Sept briques qu'il est impossible d'ajouter proprement après coup (modification M1 du blueprint).

À l'issue de la phase 0.5, il n'existe ni élève, ni classe, ni note, ni facture. Il existe un socle sur lequel ces choses peuvent être posées sans jamais avoir à revenir en arrière.

### 0.2 Critère de fin, vérifiable

La phase est terminée quand ces onze assertions sont vraies, chacune démontrée par un test automatisé ou une manipulation reproductible :

1. L'application démarre sur une machine Windows 10 1809 vierge, WebView2 absent, via l'installeur.
2. Un premier lancement crée la base chiffrée et demande la création du compte administrateur.
3. Un utilisateur s'authentifie, sa session expire après inactivité, et il la rouvre par code PIN en moins de 3 secondes.
4. Une commande refusée faute de permission renvoie une erreur, y compris si elle est appelée directement depuis la console du webview.
5. Toute mutation écrit une entrée d'audit et un événement `outbox` dans la même transaction ; couper l'alimentation en cours d'opération ne laisse jamais l'un sans l'autre.
6. La commande de vérification d'intégrité de la chaîne d'audit passe, et échoue si l'on modifie une ligne à la main dans le fichier.
7. Un numéro de séquence attribué puis abandonné ne crée pas de trou.
8. Un document PDF de test est produit par le moteur documentaire, enregistré avec son empreinte et sa version de modèle.
9. Une sauvegarde est produite, chiffrée, avec son manifeste ; sa restauration sur une machine vierge, **avec pour seuls intrants le fichier et la phrase de secours** (R-57), redonne une base identique, empreinte comparée.
10. Le type `Montant` et le type `Note` interdisent toute conversion depuis un flottant, à la compilation.
11. Les types TypeScript sont générés depuis Rust, et modifier une structure Rust casse la compilation du frontend.

Tant qu'une seule est fausse, la phase 2 ne commence pas.

---

## 1. Espace de travail et découpage

### 1.1 Crates

```
crates/
├── gesco-core       # noyau métier pur — aucune E/S (R-03)
├── gesco-db         # repositories, migrations, cloisonnement
├── gesco-services   # orchestration, transactions, effets de bord
├── gesco-docs       # moteur documentaire Typst
├── gesco-sync       # outbox, consommation (inactive en 0.5)
├── gesco-sms        # canal SMS (inactive en 0.5)
├── gesco-app        # binaire Tauri — commandes, gardes
└── gesco-server     # binaire axum du serveur cloud, phase 11 (inactif en 0.5, crate créée)
```

`gesco-sync`, `gesco-sms` et `gesco-server` sont créées vides en phase 0.5. Elles n'ont pas de code, mais leur existence force à poser les frontières dès maintenant plutôt qu'à découper un monolithe plus tard.

Le mode LAN (ADR-014) n'est **pas** `gesco-server` : il est embarqué dans `gesco-app`, un seul binaire pour le poste serveur et les postes clients. `gesco-server` ne sert que le cloud de l'édition Connectée (blueprint §6).

### 1.2 Liste blanche de dépendances de `gesco-core`

Vérifiée en intégration continue (R-03). Sont autorisées uniquement les crates sans effet de bord : types, sérialisation, erreurs, dates en tant que **types seuls**.

Sont interdites, et la CI échoue si elles apparaissent : tout accès base, réseau, système de fichiers, processus, et **toute lecture d'horloge**. Le temps entre par paramètre.

*Motif* : sans cette interdiction, un test de calcul de moyennes finit par dépendre de la date du jour, et devient rouge un matin sans raison.

### 1.3 Sens de dépendance

```
gesco-app ─→ gesco-services ─→ gesco-core
                    │              ↑
                    └─→ gesco-db ──┘
gesco-services ─→ gesco-docs
```

Le graphe Cargo interdit tout cycle, ce qui rend R-04 vérifiable à la compilation plutôt qu'en revue.

---

## 2. Types fondamentaux

À définir avant toute autre chose, dans `gesco-core/src/commun/`.

### 2.1 `Id`

Enveloppe d'un ULID. Génération côté client (ADR-004). Typée par entité — un `Id<Eleve>` ne doit pas être assignable à un `Id<Classe>`.

*Motif* : sur un système comptant une centaine de tables, l'inversion de deux identifiants dans un appel est une erreur fréquente et silencieuse. Le typage la rend impossible à la compilation, pour un coût nul.

### 2.2 `Montant`

Entier signé 64 bits, unité le franc CFA (ADR-005, R-11).

**Aucune conversion depuis ou vers un flottant n'existe.** Pas de méthode `from_f64`, pas d'implémentation de conversion. L'absence est délibérée : ce n'est pas une recommandation, c'est une impossibilité.

Opérations exposées : addition, soustraction, multiplication par un entier, division entière **retournant explicitement quotient et reste**, comparaison, formatage. Aucune division retournant un résultat unique.

Arrondi commercial au multiple de 5 ou 25 : fonction séparée, paramétrée, jamais appliquée implicitement.

### 2.3 `Note`

Entier signé 32 bits, unité le centième de point (R-11bis). 13,50 vaut `1350`.

Mêmes interdictions que `Montant`. Opérations : addition, multiplication par un entier, et une fonction de **troncature à deux décimales** implémentée par division entière (RC-1). Les sommes et produits intermédiaires (Σ note × pondération × coefficient, RC-6) sont calculés en `i64`, puis ramenés en `i32` au moment de la troncature ; un débordement est une erreur, jamais une valeur silencieusement fausse.

Le type porte séparément sa valeur et son barème, et refuse toute comparaison entre notes de barèmes différents sans normalisation explicite.

### 2.4 `Horodatage`

UTC en interne, affichage en heure locale. Jamais lu depuis l'horloge à l'intérieur de `gesco-core`.

### 2.5 `Erreur`

Une énumération d'erreurs métier nommées par domaine, distincte des erreurs techniques. Toute erreur destinée à l'utilisateur porte une clé de traduction, jamais un message en dur (R-39).

---

## 3. Conventions d'événements `outbox`

**C'est le point à figer avant la première écriture de code** (ADR-009, §25.2 du blueprint). Une convention changée après un an de production ne se rattrape pas.

### 3.1 Structure

Chaque événement porte : identifiant ULID, type, version du schéma d'événement, entité et identifiant concernés, établissement, acteur, horodatage, identifiant de poste, charge utile, identifiant de corrélation, et un marqueur de consommation.

### 3.2 Nommage

**Nom au participe passé, décrivant un fait accompli**, jamais une intention ni une opération technique.

| Correct | Incorrect | Pourquoi |
|---|---|---|
| `InscriptionValidee` | `CreerInscription` | Un événement est un fait, pas un ordre |
| `VersementEncaisse` | `PaiementMisAJour` | Le fait métier, pas la mutation technique |
| `BulletinPublie` | `BulletinChange` | « Changé » ne dit pas quoi ni pourquoi |
| `AbsenceConstatee` | `InsertAbsence` | Vocabulaire métier, pas SQL |
| `SanctionPrononcee` | `SanctionCreee` | Le verbe du domaine |

### 3.3 Versionnement

Chaque type d'événement porte un numéro de version. **Un événement publié ne change jamais de forme** : on crée `VersementEncaisse.v2` et on conserve la capacité de lire `v1`.

*Motif* : le journal est rejouable et sert de piste d'audit secondaire. Modifier la forme d'un événement passé rendrait l'historique illisible.

### 3.4 Charge utile

Minimale et **auto-suffisante**. L'événement contient ce qu'il faut pour être compris sans consulter la base — nom de l'élève, montant, période — mais pas l'entité entière.

*Motif* : un consommateur qui doit interroger la base pour comprendre un événement recrée un couplage entre modules que §25.2 vise précisément à éviter.

### 3.5 Catalogue initial

Aucun événement métier n'existe en phase 0.5 puisqu'il n'y a pas de métier. Trois événements techniques servent à valider la mécanique de bout en bout :

`UtilisateurCree`, `RoleAttribue`, `SauvegardeEffectuee`.

Les étapes 10 et 11 en ajoutent trois, techniques eux aussi : `DocumentCompose` (un document est composé et enregistré), `SauvegardeRestauree` (écrit dans la base restaurée), `PhraseSecoursRegeneree`. « Produit » ne termine pas en `-é` : `DocumentCompose` respecte la convention du §3.2 vérifiée par le test du noyau.

Le catalogue complet se remplit phase par phase, chaque module déclarant ses événements dans son propre fichier.

---

## 4. Entités du domaine CORE

### 4.1 Colonnes obligatoires

Toute table métier porte, sans exception (R-12, R-13) :

`id` (ULID) · `etablissement_id` · `cree_le` · `cree_par` · `mis_a_jour_le` · `mis_a_jour_par` · `identifiant_poste` · `supprime_le` · `supprime_par`

Les tables de référentiel national partagé (ordres, cycles, séries) sont dispensées de `etablissement_id`, au titre de la dérogation prévue à R-12.

### 4.2 Tables

**`organisation`** — nom, propriétaire, licence.

**`etablissement`** — organisation, nom, `statut` (public / privé / communautaire / medersa), `code_academie`, `code_cap`, adresse, téléphones, logo, mentions légales imprimées, devise, arrondi commercial appliqué.

**`campus`** — établissement, nom, adresse. Une seule ligne par défaut.

**`annee_scolaire`** — établissement, libellé, bornes, `statut` (préparation / active / clôturée / archivée), `est_courante`.
*Contrainte* : une seule année courante par établissement, garantie en base.

**`periode_academique`** — année, **niveau** (nullable en 0.5, le référentiel académique arrivant en phase 2), rang, `type` (mensuelle / trimestre / semestre), bornes, pondération, `verrouillee`.
*Note* : le rattachement au niveau est ce qui permet neuf périodes au 1er cycle et trois au-delà dans la même année (Q-22).

**`utilisateur`** — établissement, `personne_id` (nullable en 0.5), identifiant de connexion, empreinte Argon2id du mot de passe, empreinte du code PIN, `actif`, `doit_changer_mot_de_passe`, dernière connexion, compteur d'échecs, verrouillé jusqu'à.

**`role`** — établissement, code, libellé, `systeme` (non supprimable).

**`permission`** — code (format `domaine:ressource:action`), libellé, domaine. Table de référence, non modifiable par l'utilisateur.

**`role_permission`** — role, permission.

**`utilisateur_role`** — utilisateur, role, portée optionnelle sérialisée.

**`parametre`** — établissement, clé, valeur, type, portée, modifiable par l'utilisateur ou non.

**`sequence_numerotation`** — établissement, type de document, année, compteur courant, format.

**`document`** — établissement, type, entité et identifiant rattachés, chemin, empreinte SHA-256, taille, **identifiant et version du modèle**, produit le, produit par.

**`piece_jointe`** — empreinte SHA-256 (adressage par contenu), type MIME, taille, chemin, compteur de références.

**`journal_audit`** — voir §5. Sans `supprime_le` : la suppression n'existe pas.

**`outbox`** — voir §3. Sans `supprime_le`.

**`sauvegarde`** — horodatage, destination, taille, empreinte, version de schéma, version applicative, résultat, dernier test de restauration.

### 4.3 Index de la phase 0.5

`utilisateur (etablissement_id, identifiant_connexion)` unique · `annee_scolaire (etablissement_id, est_courante)` · `journal_audit (entite, entite_id, horodatage)` · `journal_audit (horodatage)` · `outbox (consomme_le, id)` · `document (entite, entite_id)` · `piece_jointe (empreinte)` unique · `sequence_numerotation (etablissement_id, type_document, annee)` unique.

---

## 5. Journal d'audit et chaînage

### 5.1 Contenu

Horodatage · utilisateur · rôle actif · établissement · entité · identifiant d'entité · opération · **valeur avant** · **valeur après** · module et écran d'origine · poste et empreinte machine · identifiant de corrélation · **empreinte de la ligne précédente** · **empreinte de la ligne courante**.

### 5.2 Chaînage

L'empreinte de chaque ligne est calculée sur l'ensemble de ses champs **plus l'empreinte de la ligne précédente**. La première ligne d'un établissement chaîne sur une valeur d'amorçage dérivée de son identifiant.

Une commande `verifier_integrite_audit` recalcule la chaîne et signale la première rupture. Elle est exposée à l'utilisateur, exécutée au démarrage sur les cent dernières lignes, et intégralement dans les tests.

*Motif* : sur un poste local, l'utilisateur a accès au fichier de base. Sans chaînage, le journal ne prouve rien. Avec, une modification manuelle devient détectable — ce qui est un argument commercial direct auprès du promoteur (§12.2 du blueprint).

### 5.3 Redaction

Les mots de passe, empreintes, codes PIN et codes d'accès responsables ne figurent **jamais** dans les valeurs avant et après (R-25). Les structures concernées implémentent un affichage masqué, ce qui rend l'oubli impossible plutôt qu'improbable.

---

## 6. Sécurité de session

### 6.1 Authentification

Argon2id, paramètres calibrés pour environ 250 ms sur le matériel cible — assez lent pour résister au calcul de dictionnaire, assez rapide pour ne pas irriter un utilisateur qui se reconnecte trente fois par jour.

Verrouillage temporaire progressif après échecs successifs. Changement obligatoire à la première connexion.

### 6.2 Session rapide — exigence RA-13

Rappel du contexte : un poste unique, plusieurs personnes qui s'y succèdent (§24.2 du blueprint). **Si changer d'utilisateur prend plus de trois secondes, un compte unique s'imposera dans la semaine et tout le dispositif d'audit perdra sa valeur.**

Mécanique :

- Verrouillage automatique après inactivité courte, durée paramétrable, deux minutes par défaut.
- Déverrouillage par **code PIN** de 4 à 6 chiffres, propre à chaque utilisateur.
- Le PIN ne déverrouille que la session en cours ; il ne remplace pas le mot de passe à l'ouverture de l'application ni pour les opérations sensibles.
- Bascule directe d'utilisateur depuis l'écran verrouillé, sans repasser par l'écran d'accueil.
- **Nom de l'utilisateur actif affiché en permanence**, en évidence, dans le bandeau.
- Toute opération financière ou de validation réaffiche le nom de l'utilisateur avant confirmation.

*Ce n'est pas de l'ergonomie. C'est le mécanisme qui donne sa valeur à l'ensemble du dispositif d'audit.*

### 6.3 Chiffrement au repos et clé de secours

*Révisé le 4 octobre 2026 — Q-38, tranchée le même jour : recommandation retenue.* La v1.0 dérivait la clé d'un secret d'établissement et d'un secret machine, puis constatait que perdre les deux rendait la base irrécupérable. C'était contradictoire avec l'assertion 9 : une sauvegarde chiffrée par une clé liée à la machine ne se restaure pas sur une machine vierge.

**Clé de données.** 256 bits aléatoires, générés au premier lancement (étape 7, avec le compte administrateur). Elle chiffre SQLCipher et les pièces jointes. Elle n'est jamais écrite en clair, ni journalisée (R-25).

**Deux enveloppes de la clé de données**, stockées côte à côte dans un fichier `cles.json` hors de la base :

| Enveloppe | Protégée par | Usage |
|---|---|---|
| Machine | Clé aléatoire conservée dans le trousseau du système (DPAPI sur Windows) | Déverrouillage transparent à chaque démarrage sur le poste d'origine |
| Secours | Clé dérivée par Argon2id d'une **phrase de secours** de douze mots, générée à l'installation | Restauration sur une autre machine (UC-07), perte du trousseau, réinstallation du système |

**Phrase de secours.** Douze mots tirés d'une liste française de 2 048 mots embarquée (entropie 132 bits), affichée une seule fois à l'écran, puis imprimée par le moteur documentaire sous forme de *certificat de clé de secours* (modèle Typst de l'étape 10 : nom de l'établissement, date, identifiant d'enveloppe, les douze mots numérotés, consigne de conservation). L'application exige la confirmation « j'ai imprimé et rangé le certificat hors du poste » et affiche un avertissement permanent tant qu'elle ne l'a pas.

**Régénération.** Depuis une session administrateur, mot de passe redemandé : nouvelle phrase, nouvelle enveloppe de secours, ancienne enveloppe détruite, certificat réimprimé, opération auditée (§12.3 du blueprint : accès au module de sauvegarde). Les archives antérieures embarquent l'ancienne enveloppe et restent ouvrables par l'ancienne phrase jusqu'à leur rotation (Q-41, recommandation appliquée par défaut). Le mot de passe d'un utilisateur ne protège jamais la base : il ouvre une session, pas la clé.

**Ce que la sauvegarde embarque** : la base chiffrée, les pièces jointes chiffrées et l'enveloppe de secours. **Jamais l'enveloppe machine.** C'est ce qui rend R-57 vrai par construction.

*Vérification* : assertion 9 et test « Sauvegarde » du §14.

---

## 7. Garde de permission

### 7.1 Point de passage unique

Toute commande exposée passe par la garde avant d'atteindre son service (R-21). La déclaration de permission est **portée par la commande elle-même** et non par une table de correspondance séparée, qui dériverait.

### 7.2 Trois dimensions

```
permission  ∧  portée_établissement  ∧  portée_ressource
```

En phase 0.5, seules les deux premières sont actives : la portée ressource n'a pas d'objet avant l'existence des classes et des matières. **Le point d'extension existe dès maintenant**, avec une implémentation qui accorde tout, pour que la phase 2 n'ait qu'à le renseigner.

### 7.3 Test d'exhaustivité

Un test énumère toutes les commandes exposées et échoue si l'une d'elles ne déclare pas de permission. C'est ce qui rend R-21 vérifiable automatiquement au lieu de reposer sur l'attention du relecteur.

### 7.4 Rôles livrés

Administrateur (système, non supprimable), Directeur, Directeur des études, Secrétaire, Caissier, Comptable, Enseignant, Surveillant, Gestionnaire RH, Consultation seule.

Créés vides en phase 0.5, hormis Administrateur. Ils se remplissent à mesure que les permissions apparaissent.

**Contrainte permanente** : aucun rôle livré ne cumule `grades:*` et `payments:*` (§11.5 du blueprint).

---

## 8. Transactions et effets de bord

### 8.1 Unité d'exécution

Une commande métier ouvre une transaction, applique la mutation, écrit l'audit, écrit l'`outbox`, valide (R-17). Les trois écritures sont indissociables.

### 8.2 Effets de bord

Génération de document, envoi de message, écriture de fichier, appel réseau : **après validation, jamais pendant** (R-18). Ils sont collectés pendant la transaction et exécutés après.

### 8.3 Durabilité

Mode WAL. `synchronous = FULL` sur les transactions marquées critiques — financières et d'audit —, `NORMAL` ailleurs.

Vérification d'intégrité au démarrage. Transactions courtes, jamais interactives (R-19).

*Motif* : §3.1 du blueprint. Les coupures de courant sont plus fréquentes que les coupures réseau, et plus destructrices.

---

## 9. Numérotation

Service transactionnel unique. Format paramétrable, par exemple `REC-2026-000147`.

**Séquence sans trou** (R-46). Un numéro attribué puis abandonné produit un document **annulé**, jamais un trou. Réinitialisation par exercice selon paramètre.

L'attribution se fait dans la transaction de l'opération, sous verrou sur la ligne de séquence.

---

## 10. Moteur documentaire

### 10.1 Périmètre en phase 0.5

Un modèle Typst de test, produit un PDF, l'enregistre avec son empreinte et sa version de modèle. Aucun document métier.

L'objectif est de valider la chaîne complète — composition, écriture, enregistrement, aperçu, impression — avant d'avoir des données réelles.

### 10.2 Contraintes à valider dès maintenant

- Composition en Rust, hors webview (R-44).
- Génération en lot parallélisée, avec progression, annulation et reprise.
- **Cible de performance : 400 documents en moins de 2 minutes** sur le matériel cible (§24.6, bulletin mensuel).
- Version du modèle enregistrée sur chaque document (R-45).
- Variante d'établissement — logo, en-tête, mentions — sans modification du modèle.

### 10.3 Structure d'un modèle

Un modèle déclare son jeu de variables. Le service refuse de composer si une variable requise manque, plutôt que de produire un document avec un trou — un bulletin incomplet imprimé en 400 exemplaires est un incident coûteux.

---

## 11. Sauvegarde et restauration

**Le module le plus important de la phase**, et le plus négligé dans ce type de projet. Rappel : machine unique, aucune autre copie vivante (RA-12).

### 11.1 Déclenchement

À la fermeture de l'application · à intervalle configurable · **avant toute opération à risque** : clôture d'année, import en masse, migration de schéma, restauration.

### 11.2 Contenu

Base chiffrée, pièces jointes, modèles de documents personnalisés, paramètres, **l'enveloppe de secours de la clé de données** (§6.3, jamais l'enveloppe machine), et un **manifeste** portant version de schéma, version applicative, établissement, horodatage, empreinte, identifiant de l'enveloppe de secours embarquée. Le manifeste rappelle en clair que la restauration demande la phrase de secours du certificat.

### 11.3 Rotation et destinations

Sept quotidiennes, quatre hebdomadaires, douze mensuelles.

Destinations multiples. **Au moins une destination physiquement déconnectable est obligatoire** — clé USB ou disque externe. C'est la seule protection réelle contre un rançongiciel, et l'application avertit tant qu'aucune n'est configurée.

### 11.4 Restauration

Assistant guidé, sans compétence technique. Si le poste possède l'enveloppe machine correspondante, aucune saisie ; sinon l'assistant demande la phrase de secours (UC-07). Vérification d'empreinte, contrôle de compatibilité, migration automatique si la sauvegarde est antérieure, restauration en base temporaire puis bascule atomique, puis création d'une nouvelle enveloppe machine sur ce poste. Trois phrases erronées consécutives imposent une attente progressive ; la phrase n'est jamais journalisée (R-25).

### 11.5 Test de restauration automatique

Mensuel, dans un espace temporaire, avec alerte en cas d'échec (R-49).

*Motif* : le scénario à éviter est classique et fatal — trois ans de sauvegardes silencieusement corrompues, découvertes le jour du sinistre. Une sauvegarde jamais restaurée n'est pas une sauvegarde.

---

## 12. Frontend

### 12.1 Périmètre en phase 0.5

Écran de connexion · écran verrouillé avec bascule d'utilisateur · squelette de navigation avec sections masquées selon permissions · gestion des utilisateurs et des rôles · paramètres d'établissement · consultation du journal d'audit · écran de sauvegarde et restauration.

### 12.2 Couche d'accès

`frontend/src/api/` abstrait commande Tauri locale et appel HTTP (R-38, ADR-014). Une seule implémentation active, l'autre déclarée.

*Rappel* : bien que le mode LAN soit reporté en phase 11 (Q-13), cette couche reste obligatoire. L'introduire après coup imposerait de reprendre chaque appel de l'application.

### 12.3 Génération de types

Étape de build. La CI échoue si les types générés diffèrent de ceux versionnés (R-37). Modifier une structure Rust doit casser la compilation TypeScript, pas produire un `undefined` en production.

### 12.4 Internationalisation

Catalogue en place dès le premier écran, locale `fr-ML` seule (R-39, ADR-015). Un test échoue si une chaîne visible n'y passe pas.

---

## 13. Installeur

- Bootstrapper WebView2 **hors ligne** embarqué (§3.3 du blueprint). Le téléchargement pendant l'installation n'est pas une hypothèse tenable.
- Installation sans droits administrateur si possible.
- Détection d'une installation existante et migration de schéma au démarrage.
- Cible : Windows 10 1809 et supérieur, 64 bits (ADR-016).
- Licence Ed25519 liée à une empreinte machine, vérifiée hors ligne.

**Test d'acceptation** : installation sur une machine Windows 10 vierge, sans WebView2, sans connexion internet.

---

## 14. Tests de la phase

| Cible | Nature |
|---|---|
| `Montant`, `Note` | Unitaires exhaustifs, dont troncature et débordement |
| Migrations | Rejeu intégral sur base vide à chaque exécution CI |
| Chaînage d'audit | Vérification, et détection d'une falsification manuelle |
| Atomicité | Interruption simulée entre mutation, audit et outbox |
| Permissions | Exhaustivité des commandes déclarant une permission |
| Numérotation | Absence de trou sous accès concurrent |
| Sauvegarde | Cycle complet sauvegarde puis restauration, empreintes comparées ; restauration **sans trousseau**, avec la seule phrase de secours (R-57) ; phrase erronée refusée ; régénération de phrase invalidant l'ancienne |
| Documents | Composition, empreinte, version de modèle, lot de 400 |
| Types générés | Écart entre génération et version versionnée |
| i18n | Absence de chaîne visible hors catalogue |

---

## 15. Ordre de réalisation

Chaque étape est livrable et testable avant la suivante.

1. Espace de travail, crates vides, CI, liste blanche de dépendances
2. Types fondamentaux — `Id`, `Montant`, `Note`, `Horodatage`, `Erreur`
3. Migrations initiales, repositories, cloisonnement établissement
4. Conventions d'événements et table `outbox`
5. Journal d'audit et chaînage
6. Mécanique transactionnelle et collecte des effets de bord
7. Authentification, session, code PIN, bascule rapide
8. RBAC et garde de permission
9. Service de numérotation
10. Moteur documentaire et modèle de test
11. Sauvegarde, restauration, test de restauration, clé de données et enveloppes (§6.3), certificat de clé de secours — **ne démarre pas tant que Q-38 n'est pas tranchée**
12. Génération de types, squelette frontend, i18n
13. Installeur et vérification sur machine vierge

**L'étape 4 est la plus importante et la moins visible.** Les conventions d'événements se figent là, et elles ne se rattrapent pas.

---

## 16. Ce qui reste ouvert

Q-20 et Q-21, ouvertes dans la v1.0, sont tranchées depuis le 3 octobre 2026 (blueprint §24.5).

Questions au 4 octobre 2026 (blueprint §24.8) :

| Id | Objet | Effet sur cette phase |
|---|---|---|
| **Q-38** | Clé de secours indépendante de la machine | Tranchée le 4 octobre 2026 : la conception du §6.3 est retenue et implémentée (étape 11) |
| **Q-41** | Phrase régénérée et archives antérieures | Aucun : implémentée selon la recommandation (§6.3), à confirmer |
| **Q-39** | Qui saisit physiquement les notes sur un poste unique | Aucun. Phase 4 |
| **Q-40** | Durée des phases et date du jalon commercial | Aucun sur le contenu. La phase 0.5 sert d'étalon : le temps réel de chaque étape est noté dans `ETAT_AVANCEMENT.md` |

La collecte des documents réels (blueprint §27) reste non bloquante pour cette phase et bloquante à l'entrée de la phase 2.

---

## 17. Journal des révisions

| Version | Date | Modification |
|---|---|---|
| v1.0 | 2026-08-27 | Création |
| v1.1 | 2026-10-04 | Nom de fichier aligné (`SPEC_PHASE_0_5.md`) ; parents mis à jour (blueprint v0.17, règles v0.3) ; `gesco-server` réservé au cloud, mode LAN dans `gesco-app` (§1.1) ; calculs intermédiaires de `Note` en `i64` (§2.3) ; risques cités en `RA-` (§6.2, §11) ; **§6.3 réécrit : clé de données, enveloppes machine et de secours, phrase de douze mots, certificat (Q-38)** ; §11.2, §11.4, §14 et §15 alignés ; assertion 9 renforcée (R-57) ; §16 mis à jour (Q-20/Q-21 tranchées, Q-38 à Q-40) |
| v1.2 | 2026-10-04 | Étapes 10 à 12 livrées : trois événements techniques (§3.5) ; régénération et archives antérieures (§6.3, Q-41) ; Q-38 tranchée (§16) |
