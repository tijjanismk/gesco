# PROJECT_BLUEPRINT.md

**Projet** : ERP de gestion scolaire — maternelle, fondamental, lycée (cursus malien)
**Stack** : Tauri 2 + Rust (édition 2024) + React 19/TypeScript, dernières versions stables (§5.4)
**Statut** : v0.23 — phases 1 et 2 validées ; phase 3 engagée (`SPEC_PHASE_3.md`) ; Q-48 à Q-53 tranchées (§24.9) ; Q-45 à Q-47 appliquées par défaut ; Q-39 et Q-40 ouvertes (§24.8)
**Auteur** : Tidiani
**Dernière révision** : 7 octobre 2026

---

## 0. Comment lire ce document

Ce document est la **source de vérité fonctionnelle et technique** du projet. Aucune ligne de code ne doit être écrite sans qu'elle se rattache à une section d'ici.

Trois règles d'usage :

1. **Rien n'est figé tant que ce n'est pas un ADR.** Les sections descriptives peuvent évoluer librement. Les décisions numérotées `ADR-XXX` (§4) ne changent que par une révision explicite, datée, avec le motif.
2. **Les questions ouvertes sont en §24.** Tant qu'une question de cette liste n'est pas tranchée, le module concerné ne se code pas.
3. **Ce document ne contient pas de code.** Ni schéma SQL exécutable, ni signature de fonction. Il contient des entités, des règles et des contraintes.

Le compagnon obligatoire de ce document est `ARCHITECTURE_RULES.md`, qui contient les règles opposables pendant le développement.

---

## 1. Vision globale et périmètre

### 1.1 Ce que le produit est

Un **ERP scolaire installé localement** dans l'établissement, qui fonctionne intégralement sans connexion internet, et qui gère quatre domaines dans un seul système cohérent :

| Domaine | Responsabilité |
|---|---|
| **CORE** | Utilisateurs, permissions, établissements, années scolaires, paramètres, audit, documents, sauvegarde |
| **SCOLARITÉ** | Élèves, responsables, classes, inscriptions, matières, évaluations, absences, emploi du temps, bulletins |
| **RH** | Personnel, organisation, contrats, présence, congés, paie |
| **FINANCE** | Tarification, facturation, encaissements, caisse, dépenses, budgets, rapports |

Un cinquième domaine, **E-LEARNING**, se greffe sur SCOLARITÉ en fin de parcours (§16).

### 1.2 Ce que le produit n'est pas

Délimiter par la négative évite la dérive de périmètre :

- **Ce n'est pas une application SaaS.** Chaque établissement possède ses données, sur sa machine, et le poste reste la source de vérité (ADR-001). Le serveur de l'édition Connectée (§26.3) héberge des répliques en lecture de plusieurs écoles — il est donc multi-tenant côté serveur et soumis au cloisonnement d'ADR-006 et à l'analyse APDP (§3.12) — mais il n'est jamais une dépendance du fonctionnement nominal.
- **Ce n'est pas un logiciel de comptabilité générale.** Pas de plan comptable SYSCOHADA complet, pas de bilan, pas de liasse fiscale. On produit une comptabilité de trésorerie exploitable par la direction et par un comptable externe.
- **Ce n'est pas une plateforme de visioconférence.** L'e-learning est asynchrone : contenus, quiz, devoirs. Pas de classe virtuelle temps réel.
- **Ce n'est pas un système de gestion pour l'université.** Pas de crédits ECTS, pas d'UE capitalisables, pas d'inscriptions pédagogiques à la carte.

### 1.3 Proposition de valeur, par ordre de force

C'est ce qui doit guider les arbitrages de priorité :

1. **Le recouvrement.** Une école privée à Bamako perd de l'argent parce qu'elle ne sait pas, à un instant donné, qui doit combien. C'est le déclencheur d'achat le plus fort.
2. **Le bulletin.** Sortir 400 bulletins conformes en une heure au lieu de trois semaines de travail manuel. C'est le déclencheur d'adhésion du personnel.
3. **La continuité.** Retrouver le dossier d'un élève de 2019 en dix secondes, y compris après un changement de directeur.
4. **Les états administratifs.** Produire ce que le CAP et la CPS/SE demandent sans ressaisie.

Tout ce qui ne sert aucune de ces quatre finalités est reporté.

---

## 2. Acteurs et cas d'usage

### 2.1 Acteurs internes (utilisateurs du logiciel)

| Acteur | Fréquence d'usage | Compétence informatique attendue | Besoin dominant |
|---|---|---|---|
| **Promoteur / Fondateur** | Hebdomadaire | Faible, souvent sur téléphone | Trésorerie, impayés, effectifs |
| **Directeur / Directrice** | Quotidienne | Moyenne | Pilotage pédagogique, discipline, validation |
| **Directeur des études** | Quotidienne | Moyenne | Emplois du temps, notes, conseils de classe |
| **Secrétaire / Censeur** | Intensive | Word et Excel uniquement | Inscriptions, saisie, impression |
| **Caissier / Comptable** | Intensive | Faible à moyenne | Encaissements, reçus, état de caisse |
| **Enseignant** | Périodique (fin de période) | Très variable | Saisie de notes, appel, appréciations |
| **Surveillant général** | Quotidienne | Faible | Absences, retards, sanctions |
| **Gestionnaire RH** | Mensuelle | Moyenne | Contrats, présence, paie |

Point critique : **le secrétaire est l'utilisateur principal en volume horaire**, et c'est celui dont la compétence informatique est la plus limitée. L'ergonomie se conçoit pour lui, pas pour le directeur.

### 2.2 Acteurs externes (consommateurs de sorties)

| Acteur | Interaction | Canal |
|---|---|---|
| **Parent / Tuteur** | Consultation notes, absences, situation financière | SMS, WhatsApp, portail web (tardif) |
| **Élève** | Contenus e-learning, devoirs | Portail LAN ou web |
| **CAP** | Listes de classe, états mensuels, PV | Documents papier et Excel |
| **Académie d'Enseignement** | Remontées consolidées | Documents |
| **CPS/SE** | Fiche d'enquête scolaire annuelle | Formulaire, Excel |
| **CNECE** | Listes d'inscription DEF / BAC | Formulaire |
| **Comptable externe** | Journal de recettes et dépenses | Excel |

### 2.3 Cas d'usage critiques (ceux qui doivent être irréprochables)

Ces sept parcours concentrent le risque produit. Ils sont les cibles prioritaires des tests end-to-end :

- **UC-01 — Inscrire un nouvel élève** : identité, responsables, classe, tarification, échéancier, premier versement, reçu, fiche d'inscription imprimée. En moins de 5 minutes, sans quitter l'écran.
- **UC-02 — Encaisser un versement** : rechercher l'élève, voir la situation, saisir le montant, imputer sur les échéances, imprimer le reçu numéroté. En moins de 60 secondes.
- **UC-03 — Saisir les notes d'une composition** : une classe de 90 élèves, une matière, saisie au clavier sans souris, tolérante aux absents. En moins de 10 minutes.
- **UC-04 — Générer les bulletins d'une classe** : calcul, rangs, appréciations, validation, PDF en lot, impression. Sans intervention technique.
- **UC-05 — Sortir la liste des impayés** : par classe, par tranche d'ancienneté, exportable, avec les numéros de téléphone des payeurs.
- **UC-06 — Clôturer une année et démarrer la suivante** : décisions de passage, réinscriptions en masse, report des impayés, archivage.
- **UC-07 — Restaurer après un sinistre** : disque mort, ordinateur volé. Remonter la base sur une autre machine à partir de la sauvegarde, sans perte financière.

**UC-07 est le cas d'usage le plus sous-estimé et le plus dangereux du projet.** Il se conçoit au jour 1, pas en phase 10.

---

## 3. Contraintes structurantes du contexte malien

Cette section n'est pas de la couleur locale. **Chaque contrainte ci-dessous produit une décision d'architecture**, référencée en regard.

### 3.1 Énergie avant réseau

Les coupures d'électricité sont plus fréquentes et plus dommageables que les coupures internet. Une coupure au milieu d'une transaction financière peut corrompre une base SQLite.

→ **Conséquences** : mode WAL activé, `synchronous = FULL` sur les écritures financières, transactions courtes et jamais interactives, point de sauvegarde automatique déclenché par événement (pas seulement par horloge), vérification d'intégrité au démarrage. Recommandation commerciale : vendre la solution avec un onduleur, ou privilégier un ordinateur portable dont la batterie fait office d'onduleur.

### 3.2 Connectivité intermittente et facturée au volume

La donnée mobile est chère et lente. Une synchronisation naïve consomme le forfait de l'école en une journée.

→ **Conséquences** : ADR-001 (local-first), synchronisation différentielle compressée, jamais automatique en heures ouvrées, compteur de volume consommé visible par l'utilisateur, médias jamais synchronisés par internet (§16.4).

### 3.3 Parc matériel ancien

Windows 10 majoritaire, parfois 8.1, 4 Go de RAM, disques mécaniques. **C'est le principal risque technique du choix Tauri** : Tauri 2 dépend de WebView2, qui n'est pas préinstallé sur les Windows 10 anciens et n'est pas disponible sur Windows 7.

→ **Conséquences** : ADR-016. L'installeur doit embarquer le *bootstrapper hors ligne* de WebView2 (~130 Mo) puisqu'on ne peut pas compter sur un téléchargement. Un audit du parc matériel doit être fait **avant** de confirmer la stack sur un client donné. Voir §23, risque RA-01.

### 3.4 Effectifs de classe élevés

Des classes de 60 à 100 élèves, parfois davantage dans le public. Une composition sur un niveau représente plusieurs milliers de notes.

→ **Conséquences** : la saisie de notes est une grille type tableur avec navigation clavier, pas un formulaire par élève. Les calculs de moyennes et de rangs sont matérialisés (§8.4), recalculables, jamais recalculés à la volée à l'affichage.

### 3.5 Identité des élèves peu fiable

Dates de naissance approximatives ou issues d'un jugement supplétif. Homonymie très forte : plusieurs élèves peuvent partager nom, prénom et année de naissance. Orthographes variables d'un document à l'autre (Mohamed / Mahamadou / Mahamane).

→ **Conséquences majeures sur le modèle** :
- **Aucune contrainte d'unicité** sur le triplet (nom, prénom, date de naissance). Ce serait un bug garanti.
- Un **matricule interne** généré par le système est obligatoire et fait autorité.
- Un champ `date_naissance_estimee` (booléen) accompagne toute date de naissance.
- La recherche doit être **tolérante aux fautes** (phonétique et distance d'édition), pas une simple correspondance exacte.
- Un mécanisme de **détection et fusion de doublons** est nécessaire dès la phase 2.

### 3.6 Paiement fractionné et par des tiers

La scolarité se paie rarement en une fois. Le payeur n'est souvent ni le père ni la mère : un oncle, un grand frère, un employeur, une ONG. Les arrangements informels sont courants.

→ **Conséquences** :
- **Le payeur est une relation distincte du responsable légal.** Un élève peut avoir un responsable légal qui ne paie pas et un payeur qui n'a aucune autorité légale.
- Un versement s'**impute partiellement sur plusieurs échéances** — d'où une table d'imputation dédiée, jamais un simple montant sur la facture.
- Les **remises et exonérations doivent être enregistrables et justifiables**. Si le logiciel ne permet pas de tracer un arrangement, le personnel le contournera en dehors du système, et les données deviendront fausses.

### 3.7 Monnaie sans subdivision

Le franc CFA ne se divise pas en pratique. La plus petite coupure utile est de 5 F.

→ **Conséquences** : ADR-005. Tous les montants sont des **entiers signés 64 bits en francs CFA**. Aucun flottant nulle part, à aucun étage, y compris dans le TypeScript. Les règles d'arrondi (au multiple de 5 ou de 25) sont paramétrables par établissement et appliquées uniquement dans le noyau métier.

### 3.8 Calendrier scolaire instable

Grèves, fermetures, rentrées décalées. Le découpage en trimestres n'est pas garanti et varie entre établissements.

→ **Conséquences** : aucune date ni nombre de périodes en dur. Le nombre de périodes, leur type (trimestre ou semestre), leurs bornes et leurs pondérations sont des données de paramétrage par année scolaire et par niveau.

### 3.9 WhatsApp plutôt que portail

Le taux d'usage de WhatsApp dépasse très largement celui d'un portail web dédié. Un parent n'installera pas une application pour consulter trois notes par trimestre.

→ **Conséquences** : le portail parent est **repositionné très tard** dans la roadmap (§22). La communication passe d'abord par SMS et par génération de liens WhatsApp. Voir aussi §3.10.

### 3.10 Coût du SMS

Les passerelles SMS locales facturent au message. Sur 500 élèves, une campagne de relance mensuelle représente un coût récurrent que l'école refusera.

→ **Conséquences** : prévoir **deux implémentations** derrière une même interface `PasserelleSms` :
- une passerelle HTTP (agrégateur), simple mais payante et dépendante d'internet ;
- un **modem GSM USB piloté en commandes AT** depuis Rust, avec une puce prépayée de l'école — coût marginal très inférieur, et fonctionne sans internet.

La seconde est un différenciateur commercial réel. Elle impose que l'application desktop soit le point d'envoi, ce qui renforce ADR-001.

### 3.11 Mobile money sans API accessible

Orange Money et Wave n'exposent pas d'API marchand facilement accessible à un éditeur indépendant. Passer par un agrégateur suppose un contrat et des volumes.

→ **Conséquences** : **ne pas bloquer le produit là-dessus.** Le mode de paiement est un attribut du versement, saisi manuellement avec la référence de transaction. Une interface `FournisseurPaiement` est définie dès maintenant mais reste sans implémentation. Un import CSV du relevé marchand permet le rapprochement, ce qui couvre 90 % du besoin réel pour un dixième du travail.

### 3.12 Cadre juridique

Le Mali dispose d'une loi relative à la protection des données à caractère personnel et d'une autorité de contrôle (APDP). Le produit traite des données de mineurs, catégorie sensible.

→ **Conséquences** : chiffrement de la base au repos, journal d'audit, politique de conservation explicite, et — si un hébergement cloud est proposé — vérification des obligations déclaratives. À faire valider par un juriste avant toute commercialisation d'une offre hébergée. Voir §24, Q-11.

### 3.13 Modèle économique et piratage

L'abonnement mensuel prélevé passe mal culturellement et techniquement. La licence perpétuelle avec maintenance annuelle est mieux acceptée.

→ **Conséquences** : mécanisme de licence hors ligne, signature Ed25519 liée à une empreinte machine. **Accepter d'emblée que ce soit contournable** : investir dans le support, les mises à jour et les modèles de documents personnalisés plutôt que dans une protection sophistiquée qui pénalisera surtout les clients honnêtes lors d'un changement de machine.

---

## 4. Décisions architecturales (ADR)

Format condensé : décision, motif, conséquence, alternative écartée.

---

**ADR-001 — Architecture local-first : le poste de l'établissement est la source de vérité**

*Décision.* L'application desktop Tauri installée dans l'établissement détient la base de données de référence. Aucun serveur n'est requis pour un fonctionnement nominal complet. Un serveur de synchronisation existe en option et n'est qu'une **réplique en lecture** enrichie de quelques files d'écriture restreintes.

*Motif.* §3.1, §3.2, §3.13. Une architecture serveur imposerait un coût d'hébergement récurrent en FCFA et une dépendance internet incompatibles avec la cible.

*Conséquence.* La notion de « le backend est la source de vérité » du prompt initial se traduit ici par : **le noyau Rust est la source de vérité, le webview est un client non fiable.** Toute règle métier et toute vérification de permission s'exécutent en Rust, jamais en TypeScript.

*Alternative écartée.* Backend Laravel ou Node hébergé, avec client web. Écartée pour coût récurrent et dépendance réseau.

---

**ADR-002 — Noyau métier en Rust pur, partagé entre desktop et serveur**

*Décision.* Une crate `gesco-core` sans dépendance à Tauri, à SQLite ni à HTTP contient l'intégralité des règles métier : calcul de moyennes, décisions de passage, tarification, imputation des versements, calcul de paie, transitions d'états.

*Motif.* Le jour où le portail parent ou un serveur de synchronisation calcule une moyenne, il doit obtenir **exactement** le même résultat que le poste de l'école. Deux implémentations divergeront, c'est mécanique.

*Conséquence.* `gesco-core` reçoit des données en entrée et retourne des résultats ; il ne lit ni n'écrit lui-même. Cela le rend testable sans base de données, ce qui est la condition d'une couverture de tests sérieuse sur le calcul financier.

---

**ADR-003 — SQLite chiffré en local, PostgreSQL côté serveur, derrière une abstraction**

*Décision.* SQLite via SQLCipher pour le poste, PostgreSQL pour le serveur de synchronisation optionnel. L'accès aux données passe par des interfaces `Repository` définies dans une crate `gesco-db`.

*Motif.* SQLite est le seul choix raisonnable pour un déploiement sans administrateur système. Le chiffrement est requis par §3.12 et par le risque de vol de matériel.

*Conséquence.* Éviter les particularismes SQL de SQLite dans le code métier. Deux jeux de migrations à maintenir, ce qui est acceptable car le serveur ne porte qu'un sous-ensemble du schéma.

---

**ADR-004 — Identifiants techniques ULID, séparés des références métier**

*Décision.* Toute clé primaire est un ULID (ou UUIDv7) généré côté client. Les références métier — matricule élève, numéro de reçu, numéro de facture, matricule employé — sont des colonnes distinctes, avec leur propre logique de numérotation.

*Motif.* Un entier auto-incrémenté rend la synchronisation multi-poste impossible sans collision. Par ailleurs le prompt initial exige explicitement cette séparation (règle 15).

*Conséquence.* Les ULID sont triables chronologiquement, ce qui préserve la localité d'index malgré leur taille. La numérotation métier est séquentielle, sans trou, réinitialisée par exercice, et gérée par un service de numérotation transactionnel unique (§8.1).

---

**ADR-005 — Montants en entiers de francs CFA**

*Décision.* Type `i64`, unité le franc CFA. Interdiction du flottant sur toute la chaîne, y compris en TypeScript où les montants transitent en nombre entier.

*Motif.* §3.7. Une erreur d'arrondi sur un encaissement détruit la confiance dans le logiciel de façon irréversible.

*Conséquence.* Un type dédié dans `gesco-core` porte les opérations autorisées et les règles d'arrondi. Toute division produit explicitement un reste, jamais un arrondi silencieux.

---

**ADR-006 — Multi-établissement présent dans le schéma dès l'origine, activé plus tard**

*Décision.* La hiérarchie `Organisation → Établissement → Campus` existe dans le modèle de données dès la première migration. Toute table métier porte une colonne `etablissement_id`. L'interface n'expose qu'un seul établissement tant que la fonctionnalité n'est pas activée.

*Motif.* Ajouter une dimension de cloisonnement à un schéma existant impose de réécrire toutes les requêtes et toutes les règles de permission. C'est le refactoring le plus coûteux et le plus risqué qu'on puisse s'infliger. Le coût de l'anticipation est marginal ; le coût du rattrapage est prohibitif.

*Conséquence.* Le cloisonnement est appliqué par une couche unique de filtrage dans `gesco-db`, jamais laissé à la discrétion de chaque requête.

---

**ADR-007 — Types TypeScript générés depuis Rust**

*Décision.* Les structures de données échangées entre le noyau Rust et le frontend sont déclarées **uniquement en Rust**, et les types TypeScript correspondants sont générés automatiquement (`specta` / `tauri-specta`).

*Motif.* Toute duplication manuelle des types dérive. Un champ renommé côté Rust doit casser la compilation TypeScript, pas produire un `undefined` en production.

*Conséquence.* Le frontend ne définit jamais à la main une interface représentant une entité métier. La génération est une étape de build, vérifiée en intégration continue.

---

**ADR-008 — Moteur documentaire unique pour tous les PDF**

*Décision.* Un seul service de génération documentaire, alimenté par des **modèles Typst**, produit reçus, factures, bulletins, certificats, attestations, fiches de paie et rapports. Génération intégralement en Rust, jamais dans le webview.

*Motif.* Le prompt initial l'exige (§15), et l'expérience le confirme : un générateur par module produit sept mises en page incohérentes et sept bugs d'impression distincts. Par ailleurs générer 400 bulletins dans un webview est une impasse de performance.

*Conséquence.* **Le modèle de bulletin devient une donnée, pas du code.** C'est un point commercial décisif : chaque école veut *son* bulletin, à l'identique. Un moteur de modèles est ce qui rend cela possible sans recompilation.

---

**ADR-009 — Journal d'événements sortants (`outbox`) pour la synchronisation**

*Décision.* Toute mutation métier écrit, dans la même transaction, une ligne dans une table `outbox` append-only décrivant l'événement. La synchronisation consomme cette file.

*Motif.* Comparer deux bases pour détecter les différences est coûteux et fragile. Un journal ordonné est simple, rejouable et sert aussi de piste d'audit.

*Conséquence.* La transaction métier et l'écriture du journal sont atomiques. Cela impose que toute écriture passe par la couche service — d'où la règle 4 de `ARCHITECTURE_RULES.md`.

---

**ADR-010 — Suppression logique par défaut, suppression physique interdite sur le financier**

*Décision.* Les entités métier portent `supprime_le` et `supprime_par`. Les entités financières validées (versement, facture, dépense, bulletin publié, ligne de paie) ne se suppriment **jamais** : elles s'annulent par une écriture inverse tracée.

*Motif.* Un caissier qui peut supprimer un encaissement peut détourner de l'argent sans laisser de trace. C'est le risque de fraude interne le plus élevé du produit.

*Conséquence.* L'interface ne propose pas « Supprimer » sur ces objets, mais « Annuler », avec motif obligatoire et permission distincte.

---

**ADR-011 — Machines à états explicites, transitions centralisées en Rust**

*Décision.* Les processus importants (§10) sont modélisés par des états nommés et une table de transitions autorisées, implémentée dans `gesco-core`. Aucun booléen d'état dispersé.

*Motif.* Exigence du prompt (§13). Un booléen `est_valide` ne dit ni qui a validé, ni depuis quel état, ni si le retour en arrière est permis.

*Conséquence.* Chaque transition déclare ses préconditions, son acteur autorisé, ses effets de bord et sa réversibilité. Les tests de transitions sont générés à partir de cette table.

---

**ADR-012 — Année scolaire comme dimension de partition universelle**

*Décision.* Classe, inscription, tarification, période, programme, affectation d'enseignement et résultat portent tous une référence à l'année scolaire. Les données ne sont jamais écrasées d'une année sur l'autre : elles sont dupliquées et versionnées.

*Motif.* Un coefficient change, un tarif augmente, une classe est renommée. Sans partition par année, l'historique devient faux rétroactivement — un bulletin réimprimé trois ans plus tard afficherait les coefficients actuels.

*Conséquence.* La clôture d'année (UC-06) est une opération de première classe, avec verrouillage : une année clôturée devient lecture seule, sauf déverrouillage par une permission spécifique et tracée.

---

**ADR-013 — Deux systèmes d'évaluation coexistants**

*Décision.* Le système supporte deux modes d'évaluation, choisis par niveau : **notation chiffrée** (fondamental, lycée) et **observation de compétences** (maternelle, et éventuellement 1er cycle en approche par compétences). Ce ne sont pas deux variantes d'un même modèle mais deux sous-systèmes distincts, réunis par une interface commune de production de bulletin.

*Motif.* Forcer la maternelle dans un modèle de notes produit des bulletins absurdes. Forcer le lycée dans un modèle de compétences produit une saisie inutilisable à 90 élèves par classe.

*Conséquence.* Le paramétrage porte un champ `mode_evaluation` au niveau du `Niveau`. Le moteur documentaire sélectionne le modèle de bulletin correspondant. Voir §8.4.

---

**ADR-014 — Multi-poste par mode serveur local sur le réseau de l'établissement**

*Décision.* L'application embarque un serveur HTTP local (axum) activable. Un poste est désigné « serveur » et détient la base ; les autres postes fonctionnent en clients légers sur le réseau local de l'école. Un seul binaire, un basculement par configuration.

*Motif.* Une base SQLite partagée par fichier réseau se corrompt. Installer PostgreSQL dans une école sans informaticien n'est pas réaliste. Le mode serveur local résout les deux.

*Conséquence.* La couche d'accès du frontend est abstraite dès le départ : soit une commande Tauri locale, soit un appel HTTP au poste serveur. **Cette abstraction doit exister dès la phase 1**, sinon elle est impossible à introduire ensuite.

---

**ADR-015 — Internationalisation dès l'origine, français seul livré**

*Décision.* Aucune chaîne de caractères visible n'est écrite en dur. Toutes passent par un catalogue clé/valeur. Une seule locale, `fr-ML`, est livrée initialement.

*Motif.* Le curriculum bilingue emploie les langues nationales au premier cycle. Ajouter l'i18n à un produit mature suppose de reprendre plusieurs milliers de chaînes.

*Conséquence.* Coût quasi nul au départ, option ouverte. La locale porte aussi les formats de date, de nombre et de monnaie.

---

**ADR-016 — Cible système : Windows 10 version 1809 ou supérieure, 64 bits**

*Décision.* Support officiel de Windows 10 (1809+) et Windows 11, 64 bits. L'installeur embarque le bootstrapper WebView2 hors ligne. Windows 7, 8.1 et les systèmes 32 bits sont hors périmètre.

*Motif.* §3.3. C'est la contrainte imposée par Tauri 2 et WebView2.

*Conséquence.* **Un audit du parc doit précéder chaque signature client.** Si une part significative du marché cible s'avère hors périmètre, c'est le choix de Tauri lui-même qui doit être réexaminé — voir risque RA-01 (§23). Une version Linux est techniquement accessible à faible coût et peut servir de repli sur du matériel ancien.

---

**ADR-017 — Portail parent en application web progressive, pas en natif**

*Décision.* Le portail parent (édition Connectée, §26) est une **PWA** : application web progressive installable sur l'écran d'accueil, avec cache hors ligne par service worker et notifications push. Aucun développement React Native, Swift ou Kotlin n'est engagé.

*Motif.* Trois raisons, dans l'ordre de poids.

1. **Un frontend de plus est un produit de plus.** Le natif ajoute une bibliothèque de composants, une navigation, une chaîne de publication, un compte Apple à 99 $ par an, un compte Google, des cycles de validation, et un délai entre le correctif et le téléphone du parent. Pour un développeur seul dont le risque dominant est déjà le périmètre (RA-03), c'est le mauvais arbitrage.
2. **L'usage ne le justifie pas.** Neuf consultations par an et par parent (Q-22) ne valent pas une place sur un téléphone où le stockage est compté. Un parent n'installe pas une application pour consulter des notes — c'est le constat qui avait déjà repoussé le portail lui-même (§3.9).
3. **Aucune capacité native n'est requise.** Ni géolocalisation en arrière-plan, ni accès matériel profond. La PWA couvre l'intégralité du besoin, et sur un parc majoritairement Android elle est strictement supérieure : installation sans magasin, mise à jour instantanée, un seul code.

*Conséquence.* Le portail réutilise la bibliothèque de composants du frontend desktop. Les notifications push passent par le service worker, ce qui en fait un `Canal` supplémentaire (§25.3) sans architecture nouvelle.

*Alternative écartée.* React Native. À réexaminer uniquement si un besoin natif réel apparaît — aucun n'est identifié à ce jour.

---

## 5. Architecture technique

### 5.1 Vue en couches

```
┌──────────────────────────────────────────────────────────┐
│  PRÉSENTATION — React 19 + TypeScript (WebView)          │
│  Rendu, navigation, saisie, validation de confort        │
│  ⚠ Zone non fiable : aucune règle métier, aucune         │
│    décision de permission                                │
└──────────────────────────────────────────────────────────┘
                   ↕  commandes Tauri  |  HTTP (mode LAN)
┌──────────────────────────────────────────────────────────┐
│  FRONTIÈRE — gesco-app (Rust)                            │
│  Exposition des commandes, sérialisation, garde de       │
│  permission, ouverture de transaction, journal d'audit   │
└──────────────────────────────────────────────────────────┘
┌──────────────────────────────────────────────────────────┐
│  SERVICES MÉTIER — gesco-services (Rust)                 │
│  Orchestration, transactions, workflows, effets de bord  │
│  (documents, SMS, outbox)                                │
└──────────────────────────────────────────────────────────┘
┌──────────────────────────────────────────────────────────┐
│  NOYAU MÉTIER — gesco-core (Rust pur)                    │
│  Entités, invariants, calculs, machines à états          │
│  Aucune E/S. Testable sans base de données.              │
└──────────────────────────────────────────────────────────┘
┌──────────────────────────────────────────────────────────┐
│  PERSISTANCE — gesco-db (Rust)                           │
│  Repositories, migrations, cloisonnement établissement    │
└──────────────────────────────────────────────────────────┘
                              ↓
              SQLite + SQLCipher  |  PostgreSQL (serveur)
```

La règle de dépendance est stricte et unidirectionnelle : **une couche ne connaît jamais celle qui est au-dessus d'elle**. `gesco-core` ne connaît personne.

### 5.2 Choix techniques

| Besoin | Choix | Justification |
|---|---|---|
| Enveloppe applicative | Tauri 2 | Binaire léger, faible empreinte mémoire, adapté au parc §3.3 |
| Langage noyau | Rust | Sûreté mémoire, absence d'exception silencieuse, exactitude arithmétique |
| Base locale | SQLite + SQLCipher | Zéro administration, chiffrement au repos |
| Accès données | `sqlx` | Requêtes vérifiées à la compilation |
| Migrations | `sqlx migrate` | Versionnées, ordonnées, dans le dépôt |
| Serveur LAN et sync | `axum` | Léger, même écosystème async |
| Ponts de types | `specta` / `tauri-specta` | ADR-007 |
| Génération PDF | `typst` embarqué | Modèles textuels lisibles, composition rapide, ADR-008 |
| Export tableur | `rust_xlsxwriter` | Sans dépendance externe |
| SMS local | `serialport` + commandes AT | §3.10 |
| Mots de passe | `argon2id` | Standard actuel |
| Signature de licence | `ed25519-dalek` | §3.13 |
| UI | React 19 + Vite + TypeScript, dernières versions stables | Choix de l'utilisateur (4 octobre 2026) : toutes les dépendances à leur dernière version stable au démarrage, figées ensuite par le fichier de verrouillage |
| État serveur | TanStack Query | Cache, invalidation, mutations optimistes |
| Grilles de données | TanStack Table + TanStack Virtual | Saisie de notes à fort volume §3.4, matériel ancien §3.3 |
| Formulaires | React Hook Form + Zod | Validation de confort côté client |
| Composants | shadcn/ui sur Radix UI | Choix de l'utilisateur. Radix gère le clavier et l'accessibilité ; le thème shadcn est entièrement redéfini par les jetons du §19.6 |
| Tableaux de bord | Tremor, version composants copiés dans le dépôt (sur Radix et Recharts) | Choix de l'utilisateur. Le paquet npm `@tremor/react` (3.18.7, janvier 2025) exige React 18 : il est écarté au profit des composants copiés, compatibles avec la pile récente (à confirmer en phase 0.5). **Attention** : ces composants reposent sur la palette Tailwind par défaut et sur des valeurs arbitraires, toutes deux interdites au §19.6 ; les copier revient à les réécrire sur les jetons, coût à budgéter à l'étape 12 de la phase 0.5. Ce que l'on garde réellement de Tremor : la composition Radix + Recharts et les motifs de tableau de bord |
| Style | Tailwind 4 | Thème par défaut remplacé par les jetons du §19.6 |
| Icônes | Lucide (fourni avec shadcn/ui), avec parcimonie | Le texte d'abord ; icône seule seulement si universelle |
| Polices | Fira Sans, Fira Sans Condensed, Andika, Charis SIL, embarquées | Couverture du bambara vérifiée (§19.6), aucun appel réseau |

### 5.3 Sur Typst plutôt que HTML pour les PDF

Choix contre-intuitif qui mérite justification. Générer un PDF depuis le webview (impression navigateur ou bibliothèque JS) pose trois problèmes : la mise en page dépend de la version de WebView2 installée, la génération en lot de plusieurs centaines de documents sature la mémoire, et le résultat n'est pas reproductible d'une machine à l'autre. Typst compile en Rust, en dehors du webview, produit un rendu identique partout, et ses modèles restent modifiables sans recompiler l'application — ce qui est la condition d'ADR-008.

### 5.4 Versions : toujours les plus récentes

Consigne de l'utilisateur (4 octobre 2026) : Rust et le frontend démarrent sur les dernières versions stables, puis sont figés par `Cargo.lock`, `rust-toolchain.toml` et `pnpm-lock.yaml`. Toute montée de version ultérieure est volontaire et passe par la CI.

| Élément | Version stable relevée le 4 octobre 2026 |
|---|---|
| Rust (toolchain) | 1.99.0, édition 2024, fixée dans `rust-toolchain.toml` |
| `tauri` | 2.12.1 (la 3.0 n'existe qu'en alpha : écartée) |
| `sqlx` | 0.9.0 |
| `axum` / `tokio` | 0.8.9 / 1.53.2 |
| `typst` | 0.15.1 |
| `rust_xlsxwriter` / `calamine` | 0.99.1 / 0.36.1 |
| `serialport` | 4.10.1 |
| `argon2` / `ed25519-dalek` / `sha2` | 0.6.0 / 3.0.0 / 0.11.0 |
| `ulid` | 3.0.0 |
| `proptest` / `insta` | 1.11.0 / 1.49.0 |
| `specta` / `tauri-specta` | 2.0.0-rc.25 (voir ci-dessous) |

**Seule exception à « stable uniquement »** : la dernière version stable de `tauri-specta` (1.0.2) vise Tauri 1. Pour Tauri 2, seule la branche 2.0 existe, encore en version candidate (rc.25, mai 2026). Elle est retenue, épinglée exactement (`=2.0.0-rc.25`), et passée en 2.0 stable dès sa sortie. ADR-007 en dépend.

Ce tableau sert de plancher : au démarrage du projet, on reprend la dernière version stable de chaque crate si elle est plus récente.

### 5.5 Aucun signe de « vibe coding »

Consigne de l'utilisateur (4 octobre 2026). Le code doit se lire comme écrit par une équipe soigneuse, pas comme généré puis laissé en l'état. Ces règles valent pour Rust, TypeScript, SQL, Typst, la CI et la documentation, en plus des interdits visuels du §19.6.

**Code**
1. Pas de commentaire qui paraphrase le code (`// incrémente i`, `// appelle la fonction`). Un commentaire explique un pourquoi, une règle métier ou renvoie au blueprint (`// RC-6 : troncature, §8.4bis`).
2. Pas de code mort, commenté ou de démonstration : pas de `fn exemple()`, de composant `Demo`, de route `/test`, de `console.log` ou de `println!` de mise au point.
3. Pas de `TODO`, `FIXME`, `XXX`, `unimplemented!()`, `todo!()` ni de valeur bouchon en dehors d'une branche de travail. Une tâche restante va au suivi (ETAT_AVANCEMENT.md), pas dans le code.
4. Pas de noms génériques : `data`, `result2`, `handleClick2`, `utils.ts` fourre-tout, `helpers.rs`, `Manager`, `Service` sans métier. On nomme avec le vocabulaire du domaine : `Inscription`, `Versement`, `moyenne_periode`.
5. Pas de `unwrap()` ni `expect()` hors des tests ; pas de `any`, `as unknown as`, `@ts-ignore` ni `#[allow(...)]` sans justification écrite à côté.
6. Pas de duplication : une même logique écrite deux fois est factorisée.
7. Pas de dépendance ajoutée « au cas où » : toute dépendance est utilisée, et `cargo machete` / `knip` le vérifient.

**Dépôt et documentation**
8. Pas de fichier d'exemple ou de modèle laissé en place : logo Vite, `App.css` d'origine, `README` du gabarit, `.env.example` vide, captures d'écran fictives.
9. README sobre et factuel : à quoi sert le logiciel, comment le construire, comment lancer les tests. Pas d'émojis, pas de badges décoratifs, pas de ton commercial (« ultra-rapide », « révolutionnaire »).
10. Messages de commit courts et précis, en français, au présent (`Ajoute l'imputation des versements par échéance`). Pas de « Update file », pas d'émojis.
11. Pas de texte de remplissage (lorem ipsum, « John Doe », « Acme ») ni de données inventées en dehors du jeu de données de référence (§21.3).
12. Les messages d'erreur et libellés sont écrits pour l'utilisateur de l'école, en français simple (§19.1), jamais du jargon technique ni des messages génériques.

**Contrôles automatiques** : `cargo clippy` avec `-D warnings` et les lints `clippy::unwrap_used`, `clippy::expect_used`, `clippy::todo`, `clippy::dbg_macro`, `clippy::print_stdout` ; ESLint avec `no-console`, `@typescript-eslint/no-explicit-any`, `@typescript-eslint/ban-ts-comment` ; une recherche en CI qui fait échouer la construction si `TODO`, `FIXME`, `lorem` ou un émoji apparaît dans le code source ; `cargo machete` et `knip` pour les dépendances et exports inutilisés.

---

## 6. Architecture logicielle (structure des dossiers)

Structure adaptée de celle du prompt initial. Elle est réorganisée pour refléter la réalité d'un projet Rust en espace de travail Cargo, et non celle d'un monorepo Node.

```
gesco/
├── PROJECT_BLUEPRINT.md
├── ARCHITECTURE_RULES.md
├── docs/
│   ├── adr/                       # une décision = un fichier daté
│   ├── modele-donnees/
│   ├── workflows/
│   └── manuel-utilisateur/
│
├── crates/
│   ├── gesco-core/                # noyau métier pur, sans E/S
│   │   ├── src/
│   │   │   ├── commun/            # Montant, Ulid, Periode, erreurs
│   │   │   ├── core/              # établissement, année, permissions
│   │   │   ├── scolarite/
│   │   │   │   ├── eleve/
│   │   │   │   ├── inscription/
│   │   │   │   ├── academique/    # niveaux, matières, programmes
│   │   │   │   ├── evaluation/    # notes ET compétences
│   │   │   │   ├── assiduite/
│   │   │   │   ├── emploi_du_temps/
│   │   │   │   └── bulletin/      # moteur de calcul et de décision
│   │   │   ├── rh/
│   │   │   │   ├── personnel/
│   │   │   │   ├── contrat/
│   │   │   │   ├── conge/
│   │   │   │   └── paie/          # moteur de calcul paramétrable
│   │   │   ├── finance/
│   │   │   │   ├── tarification/
│   │   │   │   ├── facturation/
│   │   │   │   ├── encaissement/  # règles d'imputation
│   │   │   │   ├── caisse/
│   │   │   │   ├── depense/
│   │   │   │   └── budget/
│   │   │   └── elearning/
│   │   └── tests/
│   │
│   ├── gesco-db/                  # repositories, migrations
│   │   └── migrations/
│   ├── gesco-services/            # orchestration, transactions
│   ├── gesco-docs/                # moteur documentaire + modèles Typst
│   │   └── modeles/
│   │       ├── bulletin/
│   │       ├── recu/
│   │       ├── facture/
│   │       ├── certificat/
│   │       └── paie/
│   ├── gesco-sync/                # outbox, réconciliation
│   ├── gesco-sms/                 # passerelle HTTP + modem GSM
│   ├── gesco-app/                 # binaire Tauri ; embarque le serveur LAN axum (ADR-014)
│   └── gesco-server/              # binaire axum du serveur cloud (phase 11) — jamais le mode LAN
│
├── frontend/
│   ├── src/
│   │   ├── app/                   # routage, mise en page, garde d'accès
│   │   ├── modules/               # miroir des modules du noyau
│   │   ├── composants/            # bibliothèque partagée
│   │   ├── api/                   # abstraction Tauri | HTTP (ADR-014)
│   │   ├── types/                 # ⚠ GÉNÉRÉ — ne jamais éditer
│   │   └── i18n/
│   └── package.json
│
└── outils/
    ├── donnees-initiales/         # référentiel malien préchargé
    └── installeur/
```

Deux points d'attention :

- `frontend/src/types/` est **entièrement généré** (ADR-007). Toute modification manuelle est écrasée au build et doit être rejetée en revue.
- Le mode LAN (ADR-014, §15.1) est **embarqué dans `gesco-app`** : le poste serveur est le même binaire que les postes clients, activé par configuration. `gesco-server` est le binaire du serveur cloud de l'édition Connectée (§15.2, §26.3). Les gestionnaires HTTP communs aux deux vivent dans `gesco-services`, pas dans l'un des binaires.
- `outils/donnees-initiales/` contient le référentiel du système éducatif malien — ordres, cycles, niveaux, séries, matières types. C'est ce qui permet à une nouvelle école d'être opérationnelle en une heure au lieu d'une semaine de paramétrage. **C'est un actif produit, pas un fichier de test.**

---

## 7. Cartographie des modules

| Module | Domaine | Dépend de | Phase |
|---|---|---|---|
| `core.auth` | CORE | — | 1 |
| `core.rbac` | CORE | auth | 1 |
| `core.etablissement` | CORE | — | 1 |
| `core.annee_scolaire` | CORE | etablissement | 1 |
| `core.parametres` | CORE | etablissement | 1 |
| `core.audit` | CORE | auth | 1 |
| `core.numerotation` | CORE | etablissement | 1 |
| `core.documents` | CORE | — | 1 |
| `core.sauvegarde` | CORE | — | 1 |
| `scolarite.academique` | SCOLARITÉ | annee_scolaire | 2 |
| `scolarite.eleve` | SCOLARITÉ | core | 2 |
| `scolarite.responsable` | SCOLARITÉ | eleve | 2 |
| `scolarite.classe` | SCOLARITÉ | academique | 2 |
| `scolarite.inscription` | SCOLARITÉ | eleve, classe | 2 |
| `finance.tarification` | FINANCE | academique | 3 |
| `finance.facturation` | FINANCE | inscription, tarification | 3 |
| `finance.encaissement` | FINANCE | facturation, caisse | 3 |
| `finance.caisse` | FINANCE | core | 3 |
| `scolarite.evaluation` | SCOLARITÉ | inscription, academique | 4 |
| `scolarite.assiduite` | SCOLARITÉ | inscription | 4 |
| `scolarite.bulletin` | SCOLARITÉ | evaluation, assiduite | 4 |
| `scolarite.remise_convocation` | SCOLARITÉ | bulletin, responsable (§8.5bis) | 5 |
| `notification.*` | TRANSVERSE | outbox, responsable (§25.3, §25.4) | 5 |
| `core.guichet` | CORE | encaissement, remise_convocation, assiduite (§19.5) | 5 |
| `rh.personnel` | RH | core | 6 |
| `rh.contrat` | RH | personnel | 6 |
| `rh.assiduite` | RH | personnel | 6 |
| `rh.conge` | RH | personnel | 6 |
| `rh.paie` | RH | contrat, rh.assiduite | 7 |
| `finance.depense` | FINANCE | caisse | 8 |
| `finance.budget` | FINANCE | depense | 8 |
| `scolarite.emploi_du_temps` | SCOLARITÉ | classe, personnel | 8 |
| `core.reporting` | CORE | tous | 9 |
| `core.licence` | CORE | etablissement (§26.5) | 10 |
| `sync.*` | TRANSVERSE | outbox | 11 |
| `portail.parent` | EXTERNE | sync, identité responsable (§25.5) | 11 |
| `elearning.*` | E-LEARNING | classe, inscription, academique | 12 |

Les numéros de phase sont ceux du §22.2, qui fait foi. Le tableau avait gardé la numérotation antérieure à l'insertion du SMS en phase 5 (M5).

---

## 8. Modèle métier

Convention de lecture : `PK` clé primaire, `FK` clé étrangère, `†` historisé, `⊗` suppression logique interdite (annulation uniquement).

### 8.1 CORE

**Organisation / Établissement / Campus**
Hiérarchie de cloisonnement (ADR-006). `Etablissement` porte `code_academie`, `code_cap`, `statut` (public / privé / communautaire / medersa), logo, coordonnées, mentions légales imprimées sur les documents.
*Contrainte* : `code_cap` et `code_academie` sont indispensables aux remontées CPS/SE. Les prévoir dès le départ évite une migration de données chez tous les clients.

**AnneeScolaire** † — `libelle` (« 2026-2027 »), bornes, `statut` (préparation / active / clôturée / archivée), `est_courante`.
*Règle* : une seule année courante par établissement. Une année clôturée est en lecture seule (ADR-012).

**PeriodeAcademique** — rattachée à l'année et au niveau, `rang`, `type` (mensuelle / trimestre / semestre — mensuelle au 1er cycle, Q-22), bornes, `ponderation`, `verrouillee`.
*Règle* : le verrouillage d'une période gèle la saisie des notes. Le déverrouillage exige une permission distincte et est audité.

**Utilisateur / Role / Permission / RolePermission / UtilisateurRole** — voir §11.

**JournalAudit** ⊗ — voir §12.

**SequenceNumerotation** — `(etablissement, type_document, annee)` → compteur. Un service transactionnel unique attribue les numéros.
*Règle* : les numéros de reçus et de factures doivent être **séquentiels sans trou**. Un numéro attribué puis abandonné doit produire un document annulé, pas un trou. C'est une exigence de contrôle, pas un confort.

**Document** — métadonnées de tout PDF produit : type, entité rattachée, chemin, empreinte SHA-256, date, producteur, version du modèle.
*Règle* : la version du modèle utilisé est conservée. Sans elle, réimprimer un bulletin de 2024 avec le modèle 2026 produit un document différent de l'original — ce qui est inacceptable pour une pièce administrative.

**PieceJointe** — stockage adressé par contenu (SHA-256), déduplication automatique.

### 8.2 SCOLARITÉ — référentiel académique

**OrdreEnseignement** — Préscolaire, Fondamental, Secondaire général.
**Cycle** — 1er cycle (6 ans), 2e cycle (3 ans), Lycée (3 ans).
**Niveau** — `code`, `rang`, `age_theorique`, `mode_evaluation` (notes / compétences, ADR-013), `bareme_max`, `diplome_terminal` (DEF, BAC ou aucun).
**Serie** — applicable aux seuls niveaux du lycée. Filières en vigueur (confirmées le 3 octobre 2026), qui remplacent celles des programmes 2011-2013 (A, SH, SE…) :

| Niveau | Séries |
|---|---|
| 10e | Tronc commun (aucune série) |
| 11e | 11S (sciences), 11SES (sciences économiques et sociales), 11LL (langues et littérature) |
| 12e | TSE (sciences exactes), TSExp (sciences expérimentales), TSEco (sciences économiques), TSS (sciences sociales), TLL (langues et littérature) |

La série Arts et lettres (TAL), encore présente dans les programmes de 2013, n'existe plus et n'est pas reprise.
**Matiere** — `code`, `libelle`, `libelle_court` (pour la largeur des colonnes du bulletin), `groupe` (littéraire / scientifique / autre).

**ProgrammeMatiere** † — `(annee, niveau, serie?, matiere)` → `coefficient`, `volume_horaire_hebdo`, `bareme`, `obligatoire`.
*Note* : c'est ici qu'atterrissent les données de la DNP. `serie` est nullable pour les niveaux sans série. Le coefficient dépend de la série — un même enseignement de mathématiques n'a pas le même poids en série A et en série SE.

**RegleEvaluation** † — par année et niveau : nombre de périodes, pondération devoirs / composition, moyenne de passage, seuil de rattrapage, mode de calcul du rang, arrondi.
*Règle* : ces valeurs varient d'un établissement à l'autre. Aucune ne doit être écrite en dur.

**Classe** — `(annee, etablissement, niveau, serie?)`, `libelle` (« 9ème A », « 12TSE »), `professeur_principal`, `salle_principale`. Pas d'effectif maximal (T-05, 7 octobre 2026).

**AffectationEnseignement** — `(annee, classe, matiere, personnel)`. Détermine qui a le droit de saisir quelles notes (§11.3).

### 8.3 SCOLARITÉ — personnes et inscriptions

**Personne** — socle commun à l'élève, au responsable et à l'employé : nom, prénoms, sexe, date de naissance, `date_naissance_estimee`, lieu de naissance, nationalité, photo, téléphones (plusieurs), adresse.
*Contrainte majeure* : **aucun index unique** sur (nom, prénoms, date de naissance) — §3.5. Prévoir en revanche un index de recherche phonétique.

**Eleve** — `personne` FK, `matricule_interne` (unique par établissement, fait autorité), `matricule_national` (nullable), `statut`.

**Responsable** — `personne` FK, `profession`, `employeur`.

**EleveResponsable** — table de liaison portant : `lien` (père / mère / tuteur / autre), `est_responsable_legal`, `est_payeur`, `recoit_notifications`, `ordre_priorite`.
*Règle décisive* (§3.6) : **responsable légal et payeur sont deux qualités indépendantes.** Un élève peut avoir plusieurs payeurs. Un payeur peut n'avoir aucune autorité légale.

**Inscription** † ⊗ — `(annee, eleve, classe)`, date, `type` (nouvelle / réinscription / transfert entrant), `statut` (active / transférée / abandonnée / radiée / achevée), `redoublant`, `decision_fin_annee` (admis / redouble / exclu / rattrapage / non statué), `classe_precedente`.

**L'inscription est la table pivot du système.** Notes, absences, sanctions, factures, versements et résultats référencent tous `inscription_id`, jamais `eleve_id`. Trois raisons :
- l'historique par année devient trivial et exact ;
- un changement de classe en cours d'année ne perturbe rien ;
- les données financières d'une année restent attachées à cette année, même si l'élève est réinscrit ailleurs.

**HistoriqueScolaire** — années antérieures à l'informatisation, saisies librement.

### 8.4 SCOLARITÉ — évaluation

Deux sous-systèmes (ADR-013).

**A. Notation chiffrée**

**Evaluation** — `(periode, classe, matiere)`, `type` (interrogation / devoir / composition / examen blanc), date, `bareme`, `ponderation`, `saisie_par`, `statut` (brouillon / saisie / validée / publiée).

**Note** ⊗ — `(evaluation, inscription)` → `valeur` **nullable**, `absent`, `justifie`, `dispense`.
*Règle non négociable* : un élève absent a une valeur nulle et un drapeau `absent`, **jamais un zéro**. Confondre les deux fausse toutes les moyennes et c'est l'erreur la plus fréquente dans ce type de logiciel. Le traitement de l'absent (exclu du calcul, ou compté zéro si non justifié) est une règle paramétrable, appliquée au calcul et non à la saisie.

**AppreciationMatiere** — `(periode, inscription, matiere)` → texte, auteur.

**ResultatPeriode** — table matérialisée : `(inscription, periode, matiere?)` → moyenne, rang, effectif, mention, `calcule_le`, `version_regle`.
*Règle* : jamais saisie à la main, toujours recalculable intégralement à partir des notes. La version de règle utilisée est conservée pour garantir la reproductibilité d'un bulletin ancien.

**B. Observation de compétences**

**DomaineApprentissage** — langage, motricité, socialisation, découverte du monde, expression artistique.
**Competence** — `(domaine, niveau)` → libellé, ordre.
**Observation** — `(inscription, competence, periode)` → `niveau_acquisition` (acquis / en cours / non acquis), date, observateur, commentaire.

**C. Bulletin (commun aux deux)**

**Bulletin** ⊗ — `(inscription, periode)`, `statut` (brouillon / calculé / validé / publié), moyenne générale, rang, décision, appréciation générale, `document_id`, validé par, date.
*Règle* : un bulletin publié est immuable. Une correction produit une nouvelle version, l'ancienne restant consultable et marquée comme remplacée.

### 8.4bis Moteur de calcul des moyennes

Section ajoutée le 27 août 2026, sur règles métier confirmées.

#### Principe directeur

**Le moteur reproduit l'arithmétique que l'enseignant fait à la main.** Ce n'est pas une coquetterie : un professeur vérifiera toujours une moyenne au crayon, et si l'écran affiche 12,33 quand le cahier donne 12,34, c'est le logiciel qui aura tort aux yeux de l'école. La conformité au calcul manuel prime sur l'élégance mathématique — d'autant qu'ici les deux coïncident.

#### Régimes par niveau

| Rang | Cycle | Barème du bulletin | Coefficients de matière |
|---|---|---|---|
| 1 à 6 | Fondamental, 1er cycle | /10 | Non |
| 7 à 9 | Fondamental, 2e cycle | /20 | Oui |
| 10 à 12 | Lycée | /20 | Oui |

Le `rang` de `Niveau` est continu de 1 à 12 sur toute la scolarité.

#### Types d'évaluation par cycle

Le catalogue des types d'évaluation est **propre à chaque cycle**, paramétrable, et porté par `RegleEvaluation`.

| Rang | Types d'évaluation | Rythme | Barèmes usuels |
|---|---|---|---|
| 1 à 6 | Composition seule — **pas de devoir** | Mensuelle | **/10** (Q-21) |
| 7 à 9 | Devoir, Composition | Devoirs en cours de période, composition en fin | 20 et 40 |
| 10 à 12 | Devoir, Composition | Idem | 20 et 40 |

*Conséquence sur le calcul* : aucune. Trois compositions sur 10 au 1er cycle donnent Σ barèmes = 30, divisé par le barème du niveau (10) = 3. La formule générale produit la moyenne simple attendue, sans branche particulière. La distinction entre cycles est entièrement portée par le paramétrage.

*Conséquence sur l'interface* : l'écran de création d'évaluation ne propose que les types autorisés au niveau concerné. Un enseignant de 4e année ne doit pas pouvoir créer un « devoir » qui n'existe pas dans son cycle.

#### Niveau 1 — moyenne d'une matière sur une période

```
moyenne_matière  =  Σ (note_i × pondération_i)
                   ─────────────────────────────────────
                   Σ (barème_i × pondération_i) ÷ barème_niveau
```

`pondération_i` vaut 1 par défaut. Elle existe pour l'école qui voudrait pondérer explicitement sans modifier le barème de correction ; avec tous les poids à 1, la formule se réduit à la pratique courante.

**Vérification sur la pratique confirmée** — un devoir sur 20, une composition sur 40, poids à 1 :

```
(devoir + composition) ÷ (60 ÷ 20)  =  (devoir + composition) ÷ 3
```

C'est exactement l'opération faite à la main. **Le barème porte la pondération** : une composition sur 40 compte double sans qu'aucun champ de poids n'intervienne.

*Conséquence sur la flexibilité* : deux devoirs, trois devoirs, un barème /50, une composition comptant triple — tout se règle en paramétrant les barèmes des évaluations. Le moteur ne change pas.

#### Niveau 2 — moyenne générale de la période

```
moyenne_générale  =  Σ (moyenne_matière × coefficient_matière)
                     ────────────────────────────────────────
                              Σ coefficient_matière
```

Au 1er cycle, tous les coefficients valent 1 (`RegleEvaluation.utilise_coefficients = false`). Une moyenne pondérée à poids unitaires étant une moyenne simple, **le moteur conserve une seule formule**. L'absence de coefficient est un fait d'affichage — colonne masquée au bulletin, champ masqué au paramétrage — jamais une branche de calcul.

*Motif* : deux formules distinctes divergent silencieusement au bout d'un an de maintenance.

#### Niveau 3 — moyenne annuelle

```
moyenne_annuelle  =  Σ (moyenne_générale_p × pondération_période_p)
                     ──────────────────────────────────────────────
                              Σ pondération_période_p
```

`PeriodeAcademique.ponderation` permet à une école de faire compter le troisième trimestre davantage.

#### Règles de calcul — non négociables

**RC-1 — Troncature à deux décimales, jamais d'arrondi.** 13,359 devient **13,35**, pas 13,36. On coupe, on ne regarde pas le chiffre suivant. Cette règle s'applique à toute valeur affichée ou imprimée : moyenne de matière, moyenne générale, moyenne annuelle.

*Point de vigilance* : la troncature exacte est **impossible en virgule flottante**. Le résultat d'une division en `f64` peut valoir 13,359999999 ou 13,360000001 selon l'ordre des opérations, et la troncature donnerait alors 13,35 ou 13,36 de façon imprévisible. D'où RC-6.

**RC-6 — Notes et moyennes en arithmétique entière.** Les notes sont stockées en **centièmes, sur des entiers** : une note de 13,50 vaut `1350`. Tous les calculs intermédiaires se font en arithmétique entière, sur un type suffisamment large pour absorber les produits sans débordement. La troncature intervient au dernier moment, par division entière.

**Aucun flottant n'intervient dans la chaîne de calcul des notes, à aucun étage**, y compris dans les types générés vers TypeScript et dans les exports. C'est l'extension de R-11 au domaine pédagogique, et pour le même motif : deux machines doivent produire le même bulletin, au centième près, indéfiniment.

**RC-7 — Calcul en points bruts.** Aucune valeur intermédiaire n'est tronquée. La valeur pleine précision de la moyenne de matière est celle qui alimente la moyenne générale ; la troncature n'intervient qu'à l'affichage et à l'impression.

*Exception confirmée (Q-20, 3 octobre 2026)* : la moyenne de matière **est tronquée à deux décimales avant d'alimenter la moyenne générale**, comme le fait l'enseignant au crayon. Paramètre `tronquer_avant_report`, par établissement, **vrai par défaut**. Avec ce réglage, la valeur reportée dans la moyenne générale est la moyenne de matière telle qu'imprimée au bulletin ; RC-7 ne s'applique pleinement qu'aux établissements qui le désactivent. La troncature reste une division entière (RC-6).

**RC-2 — La note est stockée telle qu'écrite par le correcteur.** Avec le barème de son évaluation. La normalisation vers le barème du niveau intervient au calcul, jamais à la saisie. Ramener à la saisie détruirait la note d'origine et rendrait toute recorrection impossible.

**RC-3 — L'absent n'est pas un zéro.** Rappel de §8.4. Une note absente est exclue du numérateur **et du dénominateur**, sauf si le paramètre de traitement des absences non justifiées impose le zéro — auquel cas elle entre dans les deux.

**RC-4 — La version de règle est conservée.** Tout `ResultatPeriode` enregistre la version des règles de calcul appliquées (R-28). Un bulletin réimprimé trois ans plus tard doit être identique à l'original.

**RC-5 — Reproductibilité intégrale.** Recalculer une année archivée à partir des notes doit redonner exactement les bulletins d'origine, au centième près. C'est un test automatisé, pas une intention (§21.2).

### 8.5 SCOLARITÉ — assiduité et discipline

**Absence** — `(inscription)`, date, `granularite` (journée / demi-journée / séance), `seance?`, motif, `justifiee`, justificatif, saisie par.
**Retard** — même structure, avec durée.
**Sanction** ⊗ — `(inscription)`, type (avertissement / blâme / exclusion temporaire / conseil de discipline), date, durée, motif, décideur.

### 8.5bis Remise des documents et convocations

Section ajoutée le 27 août 2026, sur observation du fonctionnement réel.

#### Le constat

Le canal de communication effectif entre l'école et le parent est **l'enfant**. C'est un canal sans accusé de réception, et dont le porteur a un intérêt propre dans le message qu'il transporte — une convocation annonçant une sanction arrive rarement.

Par ailleurs, **le parent doit se déplacer** : il vient payer, et il vient signer le bulletin. Cette visite n'est pas une friction à supprimer, c'est le **point de convergence** de la scolarité, de la finance et de la communication.

*Conséquence sur la conception de la notification* : le SMS ne transporte pas le contenu, il transporte **l'avis**. « Bulletin de mars disponible, passez le retirer et le signer » est plus court, n'expose aucune donnée sensible, et déclenche la visite au cours de laquelle le parent réglera aussi sa scolarité. **Le canal de notification est d'abord un levier de recouvrement, ensuite un canal d'information.**

#### `RemiseDocument`

`(document, inscription)` → `statut` (à remettre / remis / signé / non retiré), date de remise, **remis à** (responsable identifié / élève / tiers), nom du signataire, date de signature, agent ayant effectué la remise.

**Règle** : un bulletin publié n'est pas un bulletin remis. Les deux états sont distincts et suivis séparément.

**Liste actionnable** : « bulletins non retirés » à J+7 après publication. Cette liste recoupe très largement celle des impayés — ce n'est pas une coïncidence, c'est le même parent qui évite le même déplacement. Les deux listes se croisent dans un même écran.

#### `Convocation`

`(inscription)` → motif, date d'émission, date de rendez-vous, émetteur, `canal_transmission` (**élève** / responsable en main propre / SMS / `wa.me`), `accuse_reception` (confirmé / **transmis non confirmé** / sans réponse), suite donnée (honorée / non honorée / reportée).

**Règle** : une convocation confiée à l'élève est enregistrée comme **transmise, non confirmée**. C'est la vérité du canal, et c'est ce qui permet de décider de la doubler par SMS quand l'enjeu le justifie — convocation disciplinaire, conseil de classe, situation d'impayé bloquante.

*Ce champ unique règle un problème réel* : l'école ne sait aujourd'hui pas distinguer un parent qui ignore une convocation d'un parent qui ne l'a jamais reçue.

#### Effet sur §25.4 — consultation par SMS entrant

Le périmètre exposé est restreint en conséquence :

| Donnée | Exposée par SMS | Motif |
|---|---|---|
| Solde et prochaine échéance | **Oui** | Déclenche la visite et le paiement |
| Absences de la période | **Oui** | Actionnable par le parent |
| Document disponible à retirer | **Oui** | Objet même du dispositif |
| Moyenne générale | Paramétrable, **activé** par défaut | Utile, peu sensible |
| **Rang** | Paramétrable, **désactivé** par défaut | Donnée comparative, source de tension entre familles ; et le bulletin le porte de toute façon |
| Appréciations | **Non** | Nuancées, elles se lisent en contexte et se commentent de vive voix |
| Sanctions | **Non** | §25.6 — jamais par canal automatique |

### 8.6 SCOLARITÉ — emploi du temps


**Salle** — capacité, type, équipement.
**CreneauHoraire** — grille de référence de l'établissement.
**Seance** — `(classe, matiere, personnel, salle, jour, creneau)`, avec période de validité.
*Règle* : la détection de conflits (enseignant, salle, classe) est une fonction du noyau, pas une contrainte de base de données. Elle doit signaler sans nécessairement bloquer — les écoles connaissent des situations transitoires que le logiciel ne doit pas rendre impossibles à enregistrer.

### 8.7 RH

**Employe** — `personne` FK, `matricule`, `categorie` (enseignant / administratif / direction / technique / appui), `date_embauche`, `statut`.
**Departement / Poste / Fonction** — organisation, avec responsable hiérarchique.
**Diplome / Qualification** — dossier de l'employé.
**Contrat** † ⊗ — type (CDI / CDD / vacation / prestation), bornes, `remuneration_base`, `statut`, renouvellements liés.
**PresenceEmploye** — pointage ou déclaratif, heures effectuées, heures supplémentaires.
**Conge** — type, bornes, statut (demandé / validé / refusé / annulé), solde.
**SoldeConge** — par employé et par exercice.

**Paie** — architecture délibérément générique (§5 du prompt) :
- **ElementPaie** : catalogue paramétrable — salaire de base, prime, indemnité, retenue, avance, heure supplémentaire. Chaque élément porte son `mode_calcul` (fixe / taux / formule / manuel), son assiette et son sens (gain ou retenue).
- **BulletinPaie** ⊗ : `(employe, periode)`, statut, brut, retenues, net, `document_id`.
- **LigneBulletinPaie** ⊗ : élément, base, taux, montant.

*Règle explicitement demandée* : **aucune règle fiscale ou sociale n'est écrite en dur.** L'INPS, l'ITS et toute cotisation sont des `ElementPaie` paramétrés, avec date d'effet. Le moteur applique des éléments, il ne connaît aucun pays.

### 8.8 FINANCE

**GrilleTarifaire** † — `(annee, etablissement)`, libellé.
**LigneTarifaire** — `(grille, niveau, serie?)` → `type_frais` (inscription / scolarité / cantine / transport / tenue / examen / autre), montant, `obligatoire`, `periodicite`.

**Facture** ⊗ — `(inscription)`, numéro, date, statut (brouillon / émise / partiellement réglée / soldée / annulée), total.
**LigneFacture** ⊗ — type de frais, désignation, montant.
**Echeance** — `(facture)`, libellé, montant dû, date d'échéance, statut.

**Reduction** ⊗ — `(inscription)`, type (bourse / demi-bourse / fratrie / enfant du personnel / arrangement), taux ou montant, motif **obligatoire**, autorisée par.
*Règle* (§3.6) : la remise doit être saisissable facilement et tracée strictement. Un système trop rigide sera contourné hors logiciel, ce qui est bien pire qu'une remise enregistrée.

**Versement** ⊗ — numéro de reçu, `(inscription)`, date, montant, `mode` (espèces / Orange Money / Wave / Moov / chèque / virement), référence externe, `payeur` (FK Responsable, nullable — le payeur peut être un tiers non enregistré, auquel cas nom libre), encaissé par, session de caisse, `annule`, motif d'annulation.

**ImputationVersement** ⊗ — `(versement, echeance)` → montant.
*Règle centrale* : un versement se répartit sur plusieurs échéances, partiellement. C'est cette table qui porte la vérité de « qui doit combien ». La règle d'imputation par défaut (la plus ancienne échéance d'abord) est dans le noyau et surchargeable manuellement.

**SessionCaisse** ⊗ — ouverture, clôture, fonds de caisse initial, total théorique, total compté, écart, motif d'écart, opérateur.
*Règle* : aucun encaissement en dehors d'une session ouverte. La clôture avec écart exige un motif. C'est le mécanisme anti-fraude le plus efficace du module.

**MouvementCaisse** ⊗ — entrée, sortie, transfert, justificatif.
**Fournisseur / CategorieDepense / Depense** ⊗ — avec workflow de validation (§10).
**Exercice / Budget / LigneBudget** — prévisions, réalisations, écarts.

### 8.9 Index critiques

À définir dès la première migration, sous peine de dégradation rapide sur des bases de plusieurs milliers d'élèves :

- `inscription (annee_id, classe_id)` — affichage de classe, toutes les listes
- `inscription (eleve_id, annee_id)` — parcours de l'élève
- `note (evaluation_id)` — saisie et calcul
- `note (inscription_id)` — bulletin
- `versement (inscription_id, date)` — situation financière
- `echeance (statut, date_echeance)` — état des impayés
- `absence (inscription_id, date)` — statistiques d'assiduité
- `personne` — index de recherche textuelle tolérante (§3.5)
- `journal_audit (entite, entite_id, horodatage)` — consultation d'historique

---

## 9. Relations entre modules

Le prompt insiste, à juste titre, sur ce point. Voici les chaînes qui doivent rester cohérentes de bout en bout.

### 9.1 Chaîne scolarité → finance

```
Élève → Inscription → [Niveau + Série] → Grille tarifaire → Lignes tarifaires
      → Réductions appliquées → Facture → Échéancier
      → Versement → Imputation → Reçu → Mouvement de caisse
```

**Point de couplage à surveiller** : la finance a besoin du niveau et de la série pour tarifer, mais **ne doit jamais lire directement les tables de scolarité**. Elle interroge un service `Scolarite::contexte_tarifaire(inscription)` qui retourne uniquement ce dont elle a besoin. Sans cette discipline, les deux domaines deviennent inséparables et le module RH suivra.

### 9.2 Chaîne RH → finance

```
Employé → Contrat → Éléments de paie → Bulletin de paie
        → Paiement du salaire → Dépense → Mouvement de caisse
```

**Décision à prendre** (§24, Q-06) : le versement d'un salaire crée-t-il automatiquement une dépense ? Recommandation : oui, mais par un événement explicite, jamais par un déclencheur de base de données invisible.

### 9.3 Chaîne pédagogique

```
Employé → Enseignant → Affectation d'enseignement → (Classe, Matière)
        → Séances → Évaluations → Notes → Résultats de période → Bulletin
```

**Point de couplage** : l'affectation d'enseignement détermine les permissions de saisie (§11.3). C'est le seul endroit où RH influence les droits d'accès en scolarité.

### 9.4 Chaîne d'assiduité

```
Séance → Appel → Absences → Statistiques de période → Bulletin
                          → Alerte SMS au responsable
                          → Éventuel impact sur la décision de passage
```

### 9.5 Chaîne transverse d'audit

Toute mutation, quel que soit le domaine :

```
Commande → Garde de permission → Service → Transaction {
    mutation métier
  + écriture du journal d'audit
  + écriture outbox
} → Effets de bord post-commit (document, SMS, notification)
```

**Les effets de bord ne sont jamais dans la transaction.** Envoyer un SMS depuis une transaction ouverte est une garantie de blocage.

### 9.6 Règle générale d'interaction

Un module n'accède jamais aux tables d'un autre module. Il passe par le service public de ce module. La conséquence pratique : `gesco-core` déclare pour chaque domaine une interface publique restreinte, et tout ce qui n'y figure pas est privé. Cette discipline est ce qui rendra possible, dans deux ans, d'extraire le module e-learning ou le portail parent sans démonter le reste.

---

## 10. Workflows et machines à états

Format : `ÉTAT → ÉTAT [acteur / permission] (condition)`.

### 10.1 Inscription

```
BROUILLON → VALIDÉE      [Secrétariat / enrollments:validate]
                          (élève identifié, responsable légal présent ; détail `SPEC_PHASE_2.md` §5.3)
BROUILLON → ABANDONNÉE   [Secrétariat / enrollments:cancel] (motif obligatoire, Q-47)
VALIDÉE   → TRANSFÉRÉE   [Direction / enrollments:transfer]
VALIDÉE   → ABANDONNÉE   [Secrétariat / enrollments:cancel] (motif obligatoire)
VALIDÉE   → RADIÉE       [Direction / enrollments:expel] (décision disciplinaire liée)
VALIDÉE   → ACHEVÉE      [Système] (à la clôture d'année)
```
*Effet de bord de la validation* : génération de la facture et de l'échéancier. **Toujours dans la même transaction** — une inscription validée sans facture est une perte de recette silencieuse.

### 10.2 Facture

```
BROUILLON → ÉMISE                    [Secrétariat / invoices:issue] (attribue le numéro)
ÉMISE     → PARTIELLEMENT_RÉGLÉE     [Système] (première imputation)
          → SOLDÉE                   [Système] (reste à payer nul)
ÉMISE     → ANNULÉE                  [Direction / invoices:cancel]
                                      (impossible si un versement est imputé)
```

### 10.3 Versement

```
SAISI → VALIDÉ    [Caisse / payments:create] (session de caisse ouverte)
VALIDÉ → ANNULÉ   [Direction / payments:cancel]
                   (motif obligatoire, session du jour uniquement,
                    contre-passation générée, reçu marqué annulé)
```
*Règle* : jamais de suppression (ADR-010). L'annulation au-delà du jour même exige une permission supérieure et déclenche une alerte à la direction.

### 10.4 Note et bulletin

```
Évaluation :  BROUILLON → SAISIE → VALIDÉE → PUBLIÉE
              [Enseignant]  [Enseignant]  [Direction des études]  [Direction]

Bulletin :    BROUILLON → CALCULÉ → VALIDÉ → PUBLIÉ
                          [Système]  [Conseil de classe]  [Direction]
```
*Règles* : une note validée n'est plus modifiable par l'enseignant. Un bulletin publié est immuable ; une correction crée une version. La publication déclenche la génération PDF et, si activé, la notification aux responsables.

### 10.5 Congé

```
DEMANDÉ → VALIDÉ    [Responsable hiérarchique / leave:approve] (solde suffisant)
        → REFUSÉ    [Responsable hiérarchique] (motif obligatoire)
VALIDÉ  → ANNULÉ    [RH] (avant la date de début, restitue le solde)
```

### 10.6 Paie

```
BROUILLON → CALCULÉE → VALIDÉE → PAYÉE
            [Système]   [Direction / payroll:validate]   [Finance]
```
*Règle* : une paie validée n'est plus recalculable. Une erreur se corrige par un élément de régularisation sur la période suivante, jamais par modification rétroactive.

### 10.7 Dépense

```
DEMANDÉE → APPROUVÉE → PAYÉE     [Demandeur → Direction → Caisse]
         → REJETÉE               (motif obligatoire)
```

### 10.8 Année scolaire

```
PRÉPARATION → ACTIVE → CLÔTURÉE → ARCHIVÉE
```
*La clôture est le workflow le plus complexe du système.* Elle enchaîne : arrêt des décisions de passage, verrouillage des périodes, gel des bulletins, arrêté des comptes, report des impayés sur l'année suivante, préparation des réinscriptions, sauvegarde d'archive.

Elle mérite un document de conception dédié et une procédure de reprise en cas d'interruption : une clôture qui s'arrête à mi-chemin à cause d'une coupure de courant doit être reprenable, pas recommencée.

### 10.9 Session de caisse

```
OUVERTE   → CLÔTURÉE    [Caisse / cashbox:close]
                         (total compté saisi ; écart non nul → motif obligatoire)
CLÔTURÉE  → RAPPROCHÉE  [Direction ou Comptable / cashbox:reconcile]
```
*Règles* : une seule session OUVERTE par caisse et par opérateur ; aucun versement ni mouvement hors d'une session OUVERTE (§8.8) ; une session ne se rouvre pas — une correction après clôture passe par un `MouvementCaisse` de régularisation daté du jour, tracé. L'ouverture (`cashbox:open`) exige la saisie du fonds de caisse initial. Ajouté le 4 octobre 2026 : la section §8.8 décrivait la session comme le mécanisme anti-fraude le plus efficace du module sans en fixer les transitions.

### 10.10 Remise (réduction)

```
PROPOSÉE  → ACCORDÉE   [Direction / invoices:discount] (motif obligatoire, type de remise)
          → REFUSÉE    [Direction] (motif obligatoire)
ACCORDÉE  → ANNULÉE    [Direction / invoices:discount]
                        (contre-passation sur l'échéancier ; impossible si le solde
                         deviendrait négatif après versements déjà imputés)
```
*Règles* : le secrétariat propose, la direction accorde (`invoices:discount`), avec le type de remise du §8.8 et un motif obligatoire ; `Reduction.autorisee_par` porte l'acteur de la transition ACCORDÉE. L'effet d'ACCORDÉE est le recalcul de l'échéancier dans la même transaction (R-17). Ajouté le 4 octobre 2026 : §3.6 et §8.8 exigeaient que la remise soit facile à saisir et strictement tracée, sans dire qui l'accorde ni ce qu'elle produit.

---

## 11. RBAC

### 11.1 Structure

Conforme au prompt : `Utilisateur → UtilisateurRole → Role → RolePermission → Permission`. Jamais de champ `role` unique sur l'utilisateur.

### 11.2 Nommage des permissions

Format `domaine:ressource:action`, en anglais pour rester stable et compact :

```
core:users:create        core:users:read        core:users:update
core:roles:assign        core:settings:update   core:audit:read
core:backup:create       core:backup:restore

students:create          students:read          students:update
students:delete          students:merge

enrollments:create       enrollments:validate   enrollments:transfer
enrollments:cancel       enrollments:expel

grades:create            grades:update          grades:validate
grades:publish           grades:unlock

reports:generate         reports:publish

invoices:issue           invoices:cancel        invoices:discount
payments:create          payments:cancel        payments:refund
cashbox:open             cashbox:close          cashbox:reconcile
expenses:create          expenses:approve       expenses:pay

employees:read           employees:update       contracts:manage
leave:request            leave:approve
payroll:calculate        payroll:validate       payroll:pay

academic_year:manage     academic_year:close      academic_year:reopen

attendance:create        attendance:update      attendance:justify
sanctions:create         sanctions:update       sanctions:cancel
convocations:create      convocations:update    convocations:close
documents:deliver        documents:reprint      templates:manage
periods:lock             periods:unlock
notifications:configure  notifications:send     notifications:send_manual
import:students          import:staff           import:grades
import:statements        export:full
licence:read             licence:install
```

Le catalogue ci-dessus est complété le 4 octobre 2026 (assiduité, discipline, convocations, remise et réimpression de documents, verrouillage de période, notification, imports, portabilité §25.10, licence §26.5). Il se remplit par phase, chaque module déclarant ses permissions dans son propre fichier ; la matrice exhaustive du §21.1 est régénérée à chaque ajout. `notifications:send_manual` couvre les messages que le moteur prépare sans jamais envoyer seul (sanctions, décisions disciplinaires, §25.6). `grades:unlock` reste la permission de déverrouillage d'une note validée ; `periods:unlock` est celle du déverrouillage d'une période (§8.1), distincte et auditée.

### 11.3 Portée : la permission ne suffit pas

C'est le point le plus souvent manqué. Un enseignant possède `grades:create`, mais **uniquement pour les couples (classe, matière) où il est affecté**. Une permission seule autoriserait tout enseignant à noter toute classe.

Chaque vérification combine donc trois dimensions :

```
permission  ∧  portée_établissement  ∧  portée_ressource
```

La portée ressource est calculée par le domaine concerné : `Scolarite::classes_autorisees(utilisateur)` retourne l'ensemble effectif. La vérification est centralisée dans la couche frontière (§5.1) et **jamais dupliquée dans le frontend** — l'interface masque des boutons pour le confort de l'utilisateur, elle ne protège rien.

### 11.4 Rôles prédéfinis

Administrateur, Directeur, Directeur des études, Secrétaire, Caissier, Comptable, Enseignant, Surveillant, Gestionnaire RH, Consultation seule. Modifiables, avec un rôle Administrateur non supprimable.

### 11.5 Risques spécifiques au contexte scolaire

- **Le mot de passe partagé.** Dans une petite école, la tentation d'un compte unique « secrétariat » est forte, et elle détruit toute la traçabilité. Contre-mesure : rendre la création d'utilisateur triviale, l'ouverture de session rapide (code PIN court sur un poste physiquement sécurisé), et afficher en permanence le nom de l'utilisateur connecté.
- **Le collègue qui note à la place d'un autre.** Contre-mesure : portée ressource stricte (§11.3) et audit sur toute saisie de note.
- **L'accès aux données financières par le personnel pédagogique.** Contre-mesure : séparation nette des rôles Enseignant et Caissier ; aucun rôle ne cumule `grades:*` et `payments:*` par défaut.

---

## 12. Audit et traçabilité

### 12.1 Structure du journal

`JournalAudit` : horodatage, utilisateur, rôle actif, établissement, entité, identifiant d'entité, opération, **valeur avant** (JSON), **valeur après** (JSON), contexte (module et écran d'origine), poste et empreinte machine, identifiant de corrélation.

### 12.2 Chaînage cryptographique

Chaque ligne du journal porte l'empreinte SHA-256 de la ligne précédente. Une modification a posteriori d'une ligne rend toute la chaîne incohérente et détectable par une commande de vérification.

*Motif.* Sur un poste local, l'utilisateur a un accès physique au fichier de base (§3.13). Sans chaînage, un journal d'audit ne prouve rien. Avec chaînage, il devient un argument commercial auprès d'un promoteur qui veut se prémunir contre la fraude de son propre personnel — ce qui est une préoccupation réelle et exprimée.

### 12.3 Entités à traçabilité renforcée

Audit systématique, non désactivable, conservation permanente : versement, remboursement, facture, remise, dépense, session de caisse, bulletin de paie, note validée, bulletin publié, inscription, toute suppression logique, toute modification de rôle ou de permission, tout accès au module de sauvegarde et restauration.

### 12.4 Conservation

Le journal ne se purge pas automatiquement. Il s'archive à la clôture d'année, par exercice, dans un fichier signé. Une purge manuelle exige une permission dédiée et laisse — évidemment — une trace dans le journal courant.

---

## 13. Stratégie documentaire

### 13.1 Principe

Un service unique `gesco-docs` (ADR-008). Chaîne de production :

```
Type de document + Données + Modèle (version)
        → Composition Typst
        → PDF
        → Enregistrement dans Document (empreinte, version du modèle)
        → Aperçu, impression ou export
```

### 13.2 Modèles

Un modèle Typst est un fichier texte, versionné, avec un jeu de variables déclaré. Trois niveaux de personnalisation :

1. **Modèle de base** livré avec le produit ;
2. **Variante d'établissement** (logo, en-tête, mentions, couleurs) sans toucher au modèle ;
3. **Modèle sur mesure** pour une école qui veut reproduire exactement son bulletin historique.

**Le niveau 3 est un service facturable, et c'est probablement le meilleur argument de vente du produit.** Une école qui retrouve son bulletin au millimètre près n'a plus de raison de résister au changement.

### 13.3 Génération en lot

Les bulletins d'un niveau entier peuvent représenter plusieurs centaines de documents. La génération est parallélisée en Rust, avec une barre de progression, une possibilité d'annulation et une reprise après interruption.

### 13.4 Documents couverts

Reçu, facture, échéancier, relevé de compte élève, bulletin (notes et compétences), relevé de notes, certificat de scolarité, attestation de fréquentation, liste de classe, fiche d'inscription, procès-verbal de conseil de classe, contrat de travail, fiche de paie, état de caisse, rapport financier, états CPS/SE, certificat de transfert et relevé des moyennes des périodes suivies (Q-32), convocation imprimée (§8.5bis), fiche de code d'accès du responsable (§25.5), certificat de clé de secours (§17.3).

---

## 14. Import / Export

### 14.1 Import

Processus obligatoire en quatre temps, pour tout import :

```
Dépôt du fichier → Analyse et correspondance des colonnes
                 → Validation ligne par ligne
                 → Aperçu avec rapport d'erreurs
                 → Confirmation explicite → Application transactionnelle
```

Aucun import ne s'applique directement. L'aperçu distingue les lignes valides, les lignes en avertissement (doublon probable, §3.5) et les lignes en erreur. L'utilisateur peut n'appliquer que les lignes valides.

Imports prévus : élèves, personnel, notes (grille par classe et matière), relevé marchand mobile money pour rapprochement (§3.11).

**L'import des élèves est le premier obstacle commercial.** Une école qui doit ressaisir 600 élèves n'adoptera pas le logiciel. L'import depuis un fichier Excel désordonné — qui est la réalité — doit être robuste, tolérant et pédagogique. C'est un module à ne pas sous-doter.

### 14.2 Export

Excel et CSV sur toute liste affichée, sans exception. C'est non négociable : les utilisateurs ont besoin de retravailler les données dans Excel, et un logiciel qui les en empêche est perçu comme une prison. Export PDF via le moteur documentaire.

---

## 15. Synchronisation et multi-poste

### 15.1 Multi-poste sur le réseau local (ADR-014)

Un poste serveur détient la base ; les autres sont des clients HTTP sur le réseau de l'école. Un seul binaire, un basculement par configuration. Découverte du serveur par mDNS ou adresse saisie.

*Conséquence sur le frontend* : la couche `frontend/src/api/` abstrait « commande Tauri locale » et « appel HTTP ». **Elle doit exister dès la phase 1.** L'introduire après coup imposerait de reprendre chaque appel de l'application.

### 15.2 Synchronisation cloud (optionnelle, phase 9)

Modèle **maître unique** : l'établissement écrit, le cloud lit. Cela évite délibérément toute résolution de conflit complexe.

- **Montant** (école → cloud) : consommation de l'`outbox` (ADR-009), envoi différentiel compressé, uniquement les données destinées aux parents.
- **Descendant** (cloud → école) : files restreintes uniquement — messages de parents, soumissions e-learning. Ces objets ne sont jamais modifiés par l'école, donc aucun conflit possible par construction.

Contraintes issues de §3.2 : synchronisation manuelle ou planifiée hors heures ouvrées, volume affiché, reprise après interruption, **aucun média transmis par internet**.

### 15.3 Identifiants et horodatage

ULID générés localement (ADR-004), `mis_a_jour_le` et `identifiant_poste` sur toute ligne synchronisable. Si un jour un mode multi-maître devient nécessaire, ces colonnes en sont le prérequis — les ajouter plus tard sur une base en production serait très coûteux.

---

## 16. Module e-learning

Positionné en phase 11. C'est le seul module réellement dépendant des programmes officiels — d'où l'intérêt d'engager la démarche auprès de la DNP dès maintenant, en parallèle du développement du reste.

### 16.1 Modèle

**Cours** — `(matiere, niveau, serie?)`, titre, description, ordre, auteur, statut (brouillon / publié), visibilité (classe / niveau / établissement).
**Chapitre** — `(cours)`, titre, ordre.
**Lecon** — `(chapitre)`, titre, ordre, type (texte / vidéo / audio / PDF / quiz / exercice), contenu Markdown, média.
**Media** — adressé par contenu (SHA-256), type MIME, taille, disponibilité hors ligne.
**Quiz / Question / Reponse** — QCM, vrai-faux, texte court, appariement.
**DevoirEnLigne** — `(cours, classe)`, date limite, barème, consignes.
**Soumission** — `(devoir, inscription)`, contenu, fichiers, date, note, correcteur.
**Progression** — `(inscription, lecon)`, statut, temps passé, dernière visite.

*Ajouts du 3 octobre 2026 (étude des programmes)* :

**ProgrammeOfficiel** — texte officiel source : `code` (`F1-1994`, `CN1-2011`…), `ordre`, `cycle`, `niveaux` couverts, `discipline?`, `titre`, `date_publication`, `approche` (objectifs / compétences), `fait_foi` (booléen), fichier d'origine (`PieceJointe`). Plusieurs versions d'un même programme coexistent ; une seule fait foi par niveau et discipline.
**ObjectifProgramme** — `(programme, niveau, matiere)`, `code`, `libelle`, `type` (compétence / objectif / contenu), `parent?` (hiérarchie), ordre. Un `Cours` ou une `Lecon` peut s'y rattacher, ce qui rend la couverture du programme mesurable.
**Champ `langue`** — sur `Cours`, `Lecon`, `Media` et `Question`, code de langue (`fr` par défaut, `bm` pour le bambara). Posé dès la création des tables pour que les contenus en bambara, texte et voix, n'exigent aucune migration (ADR-015).

*Portée* : ces entités servent **uniquement** le module e-learning ; aucun écran de gestion (bulletins, emplois du temps, notes) ne les référence. Le référentiel de la scolarité (`Matiere`, `ProgrammeMatiere`) reste la source des horaires, barèmes et coefficients.

### 16.2 Articulation avec la scolarité

Le lien avec les notes est **optionnel et explicite**. Une note de devoir en ligne peut alimenter le système d'évaluation via une `Evaluation` marquée `source = elearning`. Elle n'y entre jamais automatiquement : c'est un enseignant qui décide de la reporter. Sans quoi le bulletin devient imprévisible.

### 16.3 Distribution des contenus — le vrai problème

Un module e-learning qui suppose une connexion internet par élève est inutilisable dans le contexte cible. Trois modes, par ordre de priorité :

1. **Réseau local (prioritaire).** Le poste serveur (ADR-014) sert le portail e-learning sur le Wi-Fi de l'école. Les élèves se connectent avec leur téléphone. Aucun internet. Coût nul. C'est le mode par défaut.
2. **Hors ligne par clé USB.** Export d'un paquet de contenus signé, réimportable sur un autre poste ou distribué aux élèves. Réponse aux fermetures d'établissement.
3. **Cloud.** PWA avec cache par service worker, uniquement là où la connectivité le permet. Contrainte héritée du §3.2 : les médias ne partent jamais du poste par internet ; en mode cloud, un contenu est soit textuel (leçons, quiz), soit porté par un média téléversé par l'enseignant depuis une connexion qui le supporte, hors du poste de l'école. Le point est à préciser avant la phase 12.

Le stockage adressé par contenu rend la déduplication et la synchronisation incrémentale des médias efficaces dans les trois modes.

### 16.4 Contrainte de contenu

Produire les contenus est un travail éditorial, pas un travail de développement. Le module doit donc être conçu comme un **outil d'édition pour les enseignants** avant d'être une bibliothèque de contenus. Vendre du contenu clé en main suppose un partenariat pédagogique qui dépasse le périmètre de ce document.

### 16.5 Décisions du 3 octobre 2026

1. **Élèves en PWA, pas d'APK.** ADR-017 s'étend au portail élève : même PWA, servie sur le Wi-Fi de l'école (§16.3, mode 1) ou en ligne.
2. **Versions de programmes les plus récentes.** Quand le dossier contient une version d'origine et une version retouchée (français, sciences physiques du 2e cycle), la retouchée fait foi (`ProgrammeOfficiel.fait_foi`).
3. **1re et 2e année livrées d'abord**, sur la base du curriculum niveau 1 de 2011 (26 h 30 par semaine, dont 12 h de LC, 7 h de SMT, 3 h de DP, 1 h 30 de SH, 1 h 30 d'Arts et 1 h 30 de récréation). Le curriculum compte 14 unités d'apprentissage mensuelles (UA 1 à 7 en 1re, 8 à 14 en 2e). SMT, SH, Arts et DP sont enseignés en langue nationale ; le français entre en 2e année dans le domaine LC.
4. **Chaque domaine du curriculum 2011 devient une `Matiere`** (Q-23) et l'on retient **50 séances par semaine** (Q-24).
5. **Volumes horaires du 1er cycle (Q-37, remplace Q-25)** : la grille du curriculum 2011 s'applique **par défaut** de la 1re à la 6e année (5 domaines, 26 h 30 par semaine). Aucune grille officielle n'a été trouvée pour la 3e à la 6e, dans le dossier comme en ligne : de la 3e à la 6e elle est marquée « non officielle ». Ces volumes sont **indicatifs** : chaque école les modifie, ou choisit les disciplines de 1994, sans aucune contrainte de gestion — dans les écoles rurales qui manquent de maîtres, l'horaire réel s'écarte souvent de la grille. Cette souplesse vaut pour les **horaires seulement** : toutes les autres règles (barèmes, effectifs, permissions, facturation…) restent appliquées.
6. **Pas de coefficients au 1er cycle** (Q-16 confirmée).
7. **Langue nationale : bambara (bamanankan).** Français livré d'abord ; le bambara entre par le champ `langue` (§16.1). La synthèse et la reconnaissance vocales (TTS, STT) sont traitées à part. Le bambara saisi dans d'anciennes polices doit passer par une table de conversion vers Unicode avant import.

---

## 17. Sécurité

### 17.1 Authentification

Argon2id, politique de mot de passe configurable, changement obligatoire à la première connexion, verrouillage temporaire après échecs répétés, session avec expiration d'inactivité, déverrouillage rapide par code PIN sur poste de confiance (§11.5).

### 17.2 Autorisation

Vérifiée exclusivement dans la couche frontière Rust (§5.1, §11.3). Le frontend n'implémente aucune décision de sécurité.

### 17.3 Données au repos

SQLCipher. **Révision du 4 octobre 2026 (Q-38, tranchée le même jour : recommandation retenue).** La version précédente dérivait la clé d'un secret d'établissement *et* d'un secret machine : une sauvegarde chiffrée ainsi est indéchiffrable dès que la machine disparaît, ce qui rend UC-07 impossible par construction.

Schéma retenu :

- une **clé de données** aléatoire de 256 bits, générée au premier lancement, chiffre la base SQLCipher et les pièces jointes ; elle ne quitte jamais le poste en clair ;
- la clé de données est conservée sous **deux enveloppes** : une *enveloppe machine* (clé du trousseau du système, déverrouillage transparent au quotidien) et une *enveloppe de secours* (clé dérivée par Argon2id d'une **phrase de secours** de douze mots, générée à l'installation, affichée une seule fois, imprimée par le moteur documentaire sous forme de *certificat de clé de secours* que la direction conserve hors du poste) ;
- la sauvegarde embarque la base chiffrée et l'enveloppe de secours, **jamais l'enveloppe machine** : la restaurer sur une autre machine n'exige que la phrase de secours ;
- la phrase se régénère depuis une session administrateur avec mot de passe, l'opération est auditée et réimprime le certificat ; l'ancienne phrase cesse immédiatement d'ouvrir la clé du poste et les sauvegardes suivantes (les archives antérieures, voir Q-41) ;
- le mot de passe d'un utilisateur ne protège jamais la base : il ouvre une session, pas la clé.

L'application avertit en permanence tant que l'impression du certificat n'a pas été confirmée, au même titre que l'absence de destination de sauvegarde déconnectable (§18.1).

### 17.4 Mode serveur local

Le mode LAN expose une surface réseau. TLS avec certificat auto-signé généré à l'installation, jeton d'appairage pour enrôler un poste client, écoute limitée aux interfaces locales, et journalisation des connexions.

### 17.5 Risques spécifiques à une application scolaire

| Risque | Gravité | Contre-mesure |
|---|---|---|
| Fraude sur les encaissements | Très élevée | Session de caisse, annulation tracée, chaînage d'audit, suppression interdite |
| Modification de notes après publication | Élevée | Immuabilité, versionnage, audit renforcé |
| Vol de l'ordinateur | Élevée | Chiffrement au repos, sauvegarde externe |
| Fuite de données de mineurs | Élevée | Chiffrement, RBAC, minimisation des exports |
| Compte partagé | Moyenne | Ouverture de session sans friction, nom affiché en permanence |
| Perte de base par coupure | Très élevée | WAL, sauvegardes fréquentes, vérification d'intégrité |
| Rançongiciel sur le poste | Élevée | Sauvegarde hors ligne, copie déconnectée obligatoire |

---

## 18. Sauvegarde et restauration

C'est **UC-07**, le cas d'usage le plus critique et le plus souvent négligé. Traité en phase 1, pas en phase 10.

### 18.1 Stratégie

- **Automatique** : à la fermeture de l'application, à intervalle configurable, et **avant toute opération à risque** (clôture d'année, import en masse, migration de schéma).
- **Rotation** : conservation glissante — 7 quotidiennes, 4 hebdomadaires, 12 mensuelles.
- **Destinations multiples** : disque local, clé USB, dossier réseau, cloud si disponible. **Au moins une destination doit être physiquement déconnectable** — c'est la seule protection réelle contre le rançongiciel.
- **Chaque sauvegarde** est chiffrée par la clé de données (§17.3), compressée, accompagnée de son empreinte et d'un manifeste (version du schéma, version applicative, date, établissement, identifiant de l'enveloppe de secours). **Elle ne dépend d'aucun secret présent sur la machine d'origine** (R-57) : la phrase de secours suffit à la restaurer ailleurs.

### 18.2 Restauration

Assistant guidé, sans compétence technique requise. Saisie de la phrase de secours quand l'enveloppe machine est absente (nouvelle machine, UC-07). Vérification d'empreinte, contrôle de compatibilité de version, migration automatique si la sauvegarde est plus ancienne que l'application, restauration dans une base temporaire puis bascule atomique, puis création d'une nouvelle enveloppe machine sur le poste de destination.

### 18.3 Test de restauration

**Une sauvegarde jamais restaurée n'est pas une sauvegarde.** L'application effectue mensuellement une restauration de vérification dans un espace temporaire et alerte l'utilisateur en cas d'échec. Cette fonction évite le scénario classique : trois ans de sauvegardes silencieusement corrompues, découvertes le jour du sinistre.

---

## 19. UX et architecture de l'information

### 19.1 Principes, dans l'ordre

1. **Densité assumée.** Les utilisateurs viennent d'Excel. Un tableau dense les rassure ; une interface aérée avec trois champs par écran les ralentit.
2. **Le clavier avant la souris.** La saisie de notes et l'encaissement se font au clavier de bout en bout. Toute étape imposant la souris coûte des minutes par jour.
3. **L'impression est une fonctionnalité de premier plan.** Pas un menu secondaire.
4. **Vocabulaire local.** « Composition », « moyenne de classe », « rang », « compo », « inscription ». Pas de jargon logiciel importé.
5. **Aucun écran modal profond.** Une modale dans une modale est une impasse pour un utilisateur non technique.

### 19.2 Navigation

Barre latérale à sections repliables, correspondant aux modules autorisés :

```
Tableau de bord
Scolarité      → Élèves · Inscriptions · Classes · Notes · Absences · Bulletins
Finance        → Encaissements · Impayés · Caisse · Dépenses · Rapports
RH             → Personnel · Contrats · Congés · Paie
Paramètres     → Établissement · Année · Niveaux · Matières · Tarifs · Utilisateurs
Outils         → Import/Export · Sauvegarde · Journal d'audit
```

### 19.3 Recherche globale

Un champ unique accessible au clavier, cherchant simultanément élèves, responsables, employés, reçus et factures. Tolérante aux fautes (§3.5). **C'est le raccourci le plus utilisé du logiciel** : le parcours réel d'un caissier commence toujours par « où est cet élève ».

### 19.4 Motifs récurrents

Liste filtrable avec export systématique · Fiche à onglets · Panneau latéral pour les actions rapides · Grille de saisie en masse · Assistant multi-étapes pour inscription, clôture et import.

---

### 19.5 L'écran guichet

Écran spécifique, issu de l'observation du fonctionnement réel (§8.5bis) : **le parent qui se présente au comptoir est le moment où tout converge.** Il vient payer, il vient signer le bulletin, et c'est la seule occasion fiable de lui transmettre quoi que ce soit.

Une recherche unique sur le nom de l'élève, et l'agent voit tout ce qui concerne cette famille, en un seul écran :

- **Situation financière** — solde, échéances, prochaine date, historique des versements
- **Documents en attente de remise** — bulletins non retirés, certificats demandés
- **Bulletins à signer** — avec enregistrement de la signature sur place
- **Convocations en cours** — dont celles marquées *transmises non confirmées* (§8.5bis)
- **Absences récentes** non justifiées
- **Fratrie** — les autres enfants du même responsable, avec leur solde

Les actions se font depuis cet écran, sans navigation : encaisser, imprimer un reçu, remettre un document, enregistrer une signature, justifier une absence.

*Motif* : sans cet écran, l'agent doit ouvrir quatre modules pendant que le parent attend debout. Avec lui, une visite règle tout — et le parent qui vient signer un bulletin repart en ayant payé, ce qui est précisément l'effet recherché.

**La ligne « fratrie » n'est pas un détail** : un responsable ayant trois enfants dans l'école ne veut pas faire la queue trois fois, et l'école a tout intérêt à encaisser les trois scolarités pendant qu'il est là.

### 19.6 Identité visuelle

Décisions de l'utilisateur du 4 octobre 2026 : shadcn/ui, Radix UI et Tremor ; dernières versions des paquets ; **bleu comme couleur principale, avec mode sombre** ; e-learning au style d'une extension LMS WordPress (§19.7). Échantillon visuel : https://claude.ai/artifact/H7Ebb7aNAmgBrHUsoePuZi

**Thème.** Les variables CSS de shadcn/ui (`--primary`, `--background`, `--card`, `--destructive`…) et les couleurs des graphiques Tremor sont toutes alimentées par les jetons ci-dessous. Aucune couleur de la palette Tailwind par défaut n'est utilisée.

| Jeton | Clair | Sombre | Rôle |
|---|---|---|---|
| `fond` | `#F1F3F7` | `#0F1828` | Fond de l'application |
| `surface` | `#FFFFFF` | `#18233A` | Grilles, formulaires, cartes |
| `marine` | `#16233F` | `#0A1220` | Barre latérale |
| `encre` | `#1E2533` | `#E3E8F2` | Texte, valeurs calculées |
| `muet` | `#56607A` | `#9AA6BF` | Texte secondaire |
| `primaire` (bleu) | `#1F4BB8` | `#8FB0FF` | Action principale, liens, focus, valeurs saisies |
| `rouge` | `#B4232B` | `#FF8E8A` | Erreur, note sous la moyenne, impayé |
| `vert` | `#2D6A3E` | `#8FD19E` | Soldé, admis, leçon terminée |
| `ocre` | `#8A5300` | `#E8B460` | À relancer, en attente |
| `sur-marine` | `#E3E8F2` | `#E3E8F2` | Texte de la barre latérale (ajouté le 4 octobre 2026, étape 12 : `encre` clair sur `marine` clair était illisible) |

Tous les couples texte / fond dépassent 6:1 dans les deux thèmes (écrans anciens, §3.3), vérifié le 4 octobre 2026. Le mode sombre suit le réglage du système, avec un choix manuel dans les préférences de l'utilisateur.

**Règle des encres**, conservée car compatible avec le bleu principal : dans les grilles, ce qu'une personne a saisi s'affiche en bleu, ce que le logiciel a calculé en noir (`encre`), ce qui demande correction en rouge souligné.

**Polices**, embarquées dans l'application. Critère éliminatoire : afficher ɛ ɔ ɲ ŋ Ɛ Ɔ Ɲ Ŋ (§16.5, point 7), vérifié sur les fichiers le 4 octobre 2026.
- Fira Sans : interface du logiciel de bureau ; Fira Sans Condensed avec chiffres tabulaires : grilles denses.
- Andika (SIL) : portail élève et e-learning, conçue pour l'apprentissage de la lecture.
- Charis SIL : bulletins et documents Typst.
- Écartées : IBM Plex Sans, Public Sans, Figtree (sans ɛ ɔ ɲ), Lexend (sans ɛ ɔ), Atkinson Hyperlegible Next (sans aucune des lettres), Source Sans 3 (sans Ɲ). Inter les couvre mais reste la police par défaut de trop d'applications.

**Interdits**, vérifiés en revue ; les deux premiers sont bloqués par l'outillage :
1. couleurs de la palette Tailwind par défaut (`bg-indigo-500`…) : seuls les jetons existent ;
2. valeurs arbitraires (`p-[13px]`, couleur en dur), refusées par le linter ;
3. dégradés, verre dépoli ;
4. émojis dans l'interface ;
5. libellés en capitales espacées ;
6. animations d'apparition : seules les réponses à une action sont animées ;
7. texte centré dans les écrans de gestion (§19.1) ;
8. messages vagues (« Oups ! ») : une erreur dit ce qui ne va pas et comment corriger ;
9. toasts pour tout : l'état s'affiche là où l'on agit (« Enregistré à 10 h 42 »).

### 19.7 E-learning au style LMS

Choix de l'utilisateur : le portail élève reprend les codes des extensions LMS de WordPress, sur le modèle de Tutor LMS et LearnDash. Mêmes jetons et même mode sombre que le reste ; police Andika. Cinq écrans, rattachés au modèle du §16.1 :

| Écran | Contenu | Entités |
|---|---|---|
| Catalogue de cours | Cartes de cours filtrables par matière et niveau, avec image, enseignant, nombre de leçons et avancement | `Cours` |
| Page de cours | Description, puis le programme en accordéon : chapitres dépliables, leçons avec type (texte, vidéo, audio, quiz) et coche de fin | `Cours`, `Chapitre`, `Lecon`, `ObjectifProgramme` |
| Lecteur de leçon | Contenu au centre, barre latérale avec le programme et la progression, boutons « Leçon précédente » et « Leçon suivante », « Marquer comme terminée » | `Lecon`, `Media`, `Progression` |
| Quiz | Une question par écran, minuteur facultatif, correction et score à la fin | `Quiz`, `Question`, `Reponse` |
| Tableau de bord élève | Cours en cours avec pourcentage d'avancement, devoirs à rendre, derniers résultats | `Progression`, `DevoirEnLigne`, `Soumission` |

Les cartes et les barres de progression sont ici la norme du genre. Le sélecteur de langue (`fr`, `bm`) s'affiche sur la page de cours dès qu'une version bambara existe (champ `langue`, §16.1).

---

## 20. Dashboards



**Promoteur / Direction générale** — effectif total et par niveau, évolution des inscriptions, recettes du mois, impayés totaux et par ancienneté, taux de recouvrement, solde de caisse, masse salariale, alertes.

**Direction pédagogique** — moyennes par classe et par matière, taux de réussite, classes en difficulté, absentéisme, saisies de notes en retard, préparation des conseils de classe.

**Finance** — encaissements du jour et du mois par mode de paiement, état de caisse ouverte, échéances à venir sur sept jours, top des impayés, dépenses en attente de validation.

**RH** — effectif par catégorie, contrats arrivant à échéance sous soixante jours, absences du jour, congés en attente, masse salariale prévisionnelle.

**Enseignant** — mes classes, mon emploi du temps du jour, saisies attendues avec échéance, appels non faits.

Contrainte de performance : chaque dashboard doit s'afficher en moins d'une seconde sur le matériel cible (§3.3), donc sur agrégats précalculés et non sur requêtes en temps réel.

---

## 21. Stratégie de tests

### 21.1 Priorisation par le risque

| Niveau | Cible | Couverture attendue |
|---|---|---|
| **Unitaire — `gesco-core`** | Calculs financiers, moyennes, imputation, paie, transitions | **Très élevée. Non négociable.** |
| **Propriétés** | Invariants : somme des imputations ≤ montant, aucun montant négatif, moyenne dans le barème | Sur tous les calculs monétaires et de notes |
| **Intégration — `gesco-db`** | Migrations, transactions, cloisonnement établissement | Élevée |
| **Permissions** | Matrice rôle × permission × portée | **Exhaustive et générée** |
| **Workflows** | Toutes transitions, autorisées et interdites | **Exhaustive et générée** |
| **Bout en bout** | UC-01 à UC-07 | Les sept cas critiques |
| **Non-régression** | Reproduction de chaque bug corrigé | Systématique |
| **Charge** | 1500 élèves, 5 années d'historique, 200 000 notes | Avant chaque livraison majeure |

### 21.2 Tests financiers spécifiques

Le module financier justifie des tests que les autres n'exigent pas :
- **Réconciliation** : pour tout élève, somme des imputations = somme des versements non annulés.
- **Étanchéité de la caisse** : total théorique = fonds initial + entrées − sorties, sur toute session.
- **Idempotence** : rejouer deux fois le même encaissement ne double pas la recette.
- **Reproductibilité** : recalculer intégralement les résultats d'une année archivée redonne exactement les bulletins d'origine.

### 21.3 Jeu de données de référence

Un établissement fictif complet — trois niveaux, six classes, 400 élèves, une année de notes et de paiements — sert de base à tous les tests d'intégration et de démonstration commerciale. Construit en phase 2, maintenu ensuite.

---

## 22. Roadmap

Ordre globalement conservé, avec quatre modifications justifiées.

Les phases ont un ordre, pas encore de durée : aucune estimation d'effort ni date cible pour le jalon commercial (§22.4) n'est écrite, alors que RA-03 est le risque le plus probable du projet. C'est l'objet de Q-40 (§24.8).

### 22.1 Modifications par rapport au plan initial

**M1 — Ajout d'une phase 0.5 « socle technique ».**
Espace de travail Cargo, migrations, audit, numérotation, moteur documentaire, sauvegarde et restauration, abstraction d'accès (ADR-014), génération de types (ADR-007). *Motif* : ces sept briques sont impossibles à introduire proprement après coup. Les traiter comme des fonctionnalités tardives est la cause d'échec la plus fréquente sur ce type de projet.

**M2 — Sauvegarde et restauration remontée de la phase 10 à la phase 0.5.**
*Motif* : §3.1 et UC-07. Le premier client réel produira des données irremplaçables dès le premier jour. Livrer sans stratégie de restauration éprouvée, c'est accepter de perdre une école entière sur une panne de disque.

**M3 — Portail parent repoussé après le durcissement.**
*Motif* : §3.9. Le canal SMS et WhatsApp couvre l'essentiel du besoin pour une fraction du coût. Le portail est un confort tardif, pas un besoin structurant.

**M4 — E-learning en dernière position, préparation engagée maintenant.**
*Motif* : dépend des classes, des inscriptions et des matières, et des programmes officiels dont l'obtention est un délai externe. Engager la démarche administrative aujourd'hui et développer en dernier est l'ordonnancement optimal.

### 22.2 Phases

| Phase | Contenu | Livrable vérifiable |
|---|---|---|
| **0** | Ce document, ARCHITECTURE_RULES, questions §24 tranchées | Blueprint validé |
| **0.5** | Socle technique (M1) | Application vide qui démarre, journalise, sauvegarde et restaure |
| **1** | Auth, RBAC, établissement, année, paramètres, audit | Connexion, gestion des utilisateurs et des rôles |
| **2** | Référentiel académique, élèves, responsables, classes, inscriptions, import | **UC-01** — inscrire un élève de bout en bout |
| **3** | Tarification, facturation, échéanciers, encaissements, caisse, reçus | **UC-02 et UC-05** — encaisser et suivre les impayés |
| **4** | Évaluations, notes chiffrées, absences, calculs, bulletins | **UC-03 et UC-04** — saisir et produire les bulletins |
| **5** | Moteur de notification (§25.3), canal SMS par modem GSM, relances d'impayés, notification des bulletins, remise des documents et convocations (§8.5bis), écran guichet (§19.5), puis consultation par SMS entrant (§25.4) | Relance automatisée du recouvrement + guichet + consultation parent sans serveur |
| **6** | Personnel, organisation, contrats, présence, congés | Dossier RH complet |
| **7** | Paie paramétrable | Bulletin de paie |
| **8** | Dépenses, fournisseurs, budgets, emploi du temps | Comptabilité de trésorerie |
| **9** | Rapports, dashboards, états CPS/SE et CAP | Remontées administratives sans ressaisie |
| **10** | Durcissement, performance, tests de charge, installeur, licence | **UC-06 et UC-07** — clôture d'année et restauration |
| **11** | Synchronisation cloud et portail parent (Q-11, **à implémenter**, révision du 3 octobre 2026). *Conditionnelles* : mode LAN (Q-13), évaluation par compétences (Q-03) | Offre hébergée en service ; le reste déclenché par un besoin client avéré |
| **12** | E-learning (M4) | Contenus, quiz, devoirs |

### 22.3 Révision du 27 août 2026

Après arbitrage des questions §24, trois modifications :

**M5 — SMS remonté de la phase 9 à la phase 5.** Le module se retrouve juste après les bulletins et juste après la finance, c'est-à-dire au moment où l'école dispose enfin de deux choses à communiquer : une situation d'impayés et des résultats. C'est également le module au meilleur rapport valeur/effort de toute la roadmap — quelques centaines de lignes de pilotage de modem contre un effet direct sur le recouvrement, qui est le premier point de valeur du §1.3.

**M6 — Phase 11 devient conditionnelle.** Mode LAN, synchronisation, portail parent et évaluation par compétences ne sont plus planifiés mais *armés* : les coutures existent dans le schéma et dans l'architecture, l'implémentation attend un client qui la demande. Aucune de ces quatre fonctionnalités ne coûtera plus cher pour avoir été différée.

**M7 — L'évaluation par compétences sort de la phase 4.** Voir §24.1.

Effet net : la trajectoire jusqu'au premier produit vendable raccourcit, sans qu'aucune porte ne se ferme.

### 22.4 Jalon commercial

**La fin de la phase 4 constitue le premier produit vendable.** Élèves, finances et bulletins couvrent les quatre points de valeur du §1.3. Les phases 5 à 8 sont de l'approfondissement.

Recommandation forte : **chercher un établissement pilote dès la fin de la phase 3**, avec un engagement de gratuité contre retours. Six mois d'usage réel révéleront plus de défauts de conception que six mois de spécification supplémentaire.

---

## 23. Risques architecturaux

Les risques portent le préfixe `RA-` ; le préfixe `R-` est réservé aux règles d'`ARCHITECTURE_RULES.md`. Les deux listes étaient numérotées `R-xx` jusqu'à la v0.16, ce qui rendait « R-13 » ambigu (risque du compte partagé ou règle des colonnes d'horodatage).

| Id | Risque | Prob. | Impact | Traitement |
|---|---|---|---|---|
| **RA-01** | ~~WebView2 indisponible sur le parc (§3.3)~~ | — | — | **CLOS le 27/08/2026** (Q-01). Parc vérifié compatible. Maintenir le bootstrapper hors ligne dans l'installeur |
| **RA-12** | **Perte totale des données d'une école — machine unique, aucune copie vivante (§24.2)** | Moyenne | **Critique** | Sauvegarde en phase 0.5, destination physiquement déconnectable obligatoire, test de restauration mensuel automatique (§18.3) |
| **RA-13** | **Compte unique partagé sur le poste, audit vidé de sa valeur (§24.2)** | **Très élevée** | Élevé | Changement d'utilisateur en moins de 3 secondes, code PIN, verrouillage par inactivité, utilisateur actif affiché en permanence. **Phase 1** |
| **RA-02** | Corruption de base par coupure de courant | Élevée | Critique | WAL, transactions courtes, sauvegardes, vérification d'intégrité, préconisation d'onduleur |
| **RA-03** | Périmètre trop large, produit jamais livré | **Très élevée** | Critique | Jalon commercial fin de phase 4. Refuser toute fonctionnalité hors §1.3 avant. **C'est le risque le plus probable de ce projet.** |
| **RA-04** | Divergence des règles entre desktop et serveur | Moyenne | Élevé | ADR-002, noyau unique partagé |
| **RA-05** | Performance dégradée sur grandes classes | Moyenne | Élevé | Agrégats matérialisés, index (§8.9), tests de charge |
| **RA-06** | Modèle de bulletin non conforme aux attentes de l'école | Élevée | Élevé | ADR-008, moteur de modèles, collecte de bulletins réels dès maintenant |
| **RA-07** | Rejet par les utilisateurs peu à l'aise avec l'outil | Moyenne | Élevé | §19, pilote précoce, formation incluse dans l'offre |
| **RA-08** | Complexité de la paie sous-estimée | Élevée | Moyen | Architecture générique (§8.7), aucune règle en dur, phase 6 isolée |
| **RA-09** | Synchronisation plus complexe que prévu | Moyenne | Moyen | Maître unique (§15.2), refus assumé du multi-maître |
| **RA-10** | Développeur unique — facteur d'autobus | Élevée | Élevé | Documentation, tests, ADR. Ce document en est la première mesure. |
| **RA-11** | Contenus e-learning inexistants faute de production éditoriale | Élevée | Moyen | §16.4 — positionner l'outil comme éditeur pour enseignants |

---

## 24. Décisions arrêtées

Questions tranchées le 27 août 2026. Une décision ne se modifie que par révision datée de ce tableau.

| Id | Question | Décision | Effet |
|---|---|---|---|
| **Q-01** | Parc compatible WebView2 ? (RA-01) | **Oui, vérifié** | ADR-016 confirmé. **RA-01 clos.** Stack Tauri définitive |
| **Q-02** | Mono ou multi-établissement ? | Schéma multi, interface mono | ADR-006 appliqué |
| **Q-03** | Maternelle dans le premier périmètre ? | **Non — fondamental et lycée d'abord** ; révisée le 7 octobre 2026 : le jardin d'enfants **s'inscrit** dès la phase 2, seule son évaluation reste reportée | §24.1 |
| **Q-04** | Une facture par inscription ou par échéance ? | Une facture, plusieurs échéances | §8.8 |
| **Q-05** | Frais annexes sur la même facture ? | Oui, lignes distinctes | §8.8 |
| **Q-06** | Le salaire versé crée-t-il une dépense ? | Oui, par événement explicite (jamais par déclencheur, R-15) | §9.2 |
| **Q-07** | Rang par classe ou par niveau ? | Paramétrable, par classe par défaut | `RegleEvaluation` |
| **Q-08** | Absence non justifiée = zéro ? | Paramétrable, exclue du calcul par défaut | `RegleEvaluation` |
| **Q-09** | Changement de classe en cours d'année ? | Oui, avec historique | §8.3 |
| **Q-10** | Report des impayés sur l'année suivante ? | Oui, avec traçabilité de l'origine | §10.8 |
| **Q-11** | Offre hébergée à court terme ? | ~~Peut-être plus tard~~ → **On l'implémente** (révision du 3 octobre 2026) | §24.3 |
| **Q-12** | Licence perpétuelle ou abonnement ? | Perpétuelle + maintenance annuelle | §3.13 |
| **Q-13** | Combien de postes par école ? | **Un seul poste** | §24.2 |
| **Q-14** | Passerelle SMS ou modem GSM ? | **Modem GSM d'abord** | §3.10 |
| **Q-15** | Barème du bulletin par niveau ? | **/10 rangs 1-6, /20 rangs 7-12** | §8.4bis |
| **Q-16** | Coefficients de matière au 1er cycle ? | **Non** — `utilise_coefficients = false` | §8.4bis |
| **Q-17** | Formule de moyenne de matière ? | **Σ notes ÷ (Σ barèmes ÷ barème_niveau)** | §8.4bis |
| **Q-18** | Matricule national au lycée ? | **Connu dès l'inscription** — obligatoire et unique au lycée, vide au fondamental | §8.3 |
| **Q-19** | Date de naissance disponible à l'inscription ? | **Oui, toujours exigée** — obligatoire. Saisie progressive abandonnée | §24.4 |

### 24.1 Conséquences de Q-03 — maternelle reportée

**Révision du 7 octobre 2026** (questions de terrain T-02, T-03 et T-10) : le jardin d'enfants entre en phase 2 pour l'**inscription** — niveaux du jardin, deux à trois responsables qui viennent chercher l'enfant (case « vient chercher l'enfant »), photo obligatoire. Son évaluation par compétences reste reportée comme ci-dessous ; aucun bulletin du jardin avant la phase 11.

**Conservé** : le champ `mode_evaluation` sur `Niveau` (ADR-013) et la sélection du modèle de bulletin correspondant dans le moteur documentaire.

**Reporté** : les entités `DomaineApprentissage`, `Competence` et `Observation` (§8.4B), ainsi que les écrans de saisie d'observations.

**Effet sur la phase 4** : le moteur de bulletin ne traite que la notation chiffrée. C'est un allègement substantiel — la phase 4 était la plus lourde de la roadmap.

*Motif du maintien du champ* : ajouter une discriminante de mode d'évaluation à un moteur de bulletin déjà écrit suppose de le réécrire. La garder coûte une colonne et un `match` à une seule branche.

### 24.2 Conséquences de Q-13 — poste unique

**Reporté** : le mode serveur LAN (ADR-014), déplacé de la phase 9 vers la phase 11, conditionné à l'apparition d'un besoin réel.

**Conservé sans négociation** : la couche d'abstraction d'accès `frontend/src/api/` (R-38). Elle coûte une indirection aujourd'hui ; l'introduire après coup imposerait de reprendre chaque appel de l'application.

**Deux effets qu'il ne faut pas manquer** :

1. **La machine unique devient un point de défaillance unique.** Il n'existe aucune autre copie vivante des données de l'école. La sauvegarde n'est plus une précaution, c'est la seule chose qui sépare le client d'une perte totale. Le calendrier de la phase 0.5 s'en trouve renforcé, pas allégé, et la destination physiquement déconnectable (§18.1) devient obligatoire et non recommandée.

2. **Le risque de compte partagé (§11.5) devient dominant.** Sur un poste unique, secrétaire, caissier et direction se succèdent sur le même clavier. Si changer d'utilisateur prend plus de trois secondes, personne ne le fera, un compte unique s'imposera, et tout le journal d'audit perdra sa valeur — y compris le chaînage cryptographique (§12.2), qui prouvera fidèlement que « secrétariat » a tout fait.

   **Conséquence de conception** : le changement d'utilisateur rapide n'est pas un confort d'ergonomie, c'est une exigence de sécurité. Verrouillage automatique après inactivité courte, déverrouillage par code PIN, nom de l'utilisateur actif affiché en permanence et de façon visible. **À traiter en phase 1, pas en phase 10.**

### 24.3 Conséquences de Q-11 — offre hébergée retenue

*Révision du 3 octobre 2026.* Q-11 passe de « peut-être plus tard » à **« on l'implémente »**. La décision est documentée ici ; aucun code n'est engagé à ce stade.

**Ce qui change** :

- La synchronisation cloud (§15.2) et le portail parent en PWA (ADR-017) quittent le statut de « couture armée » : ils deviennent une phase planifiée (phase 11, §22.2), qui forme l'édition Connectée (§26.3).
- L'analyse de conformité APDP (§3.12) **redevient un prérequis** : elle doit être validée par un juriste avant la mise en service de l'offre hébergée, pas après.
- Le choix de l'hébergeur, le modèle de tarification cloud et le périmètre exposé aux parents deviennent des décisions à prendre avant la phase 11. Aucun n'a d'effet sur le schéma.

**Ce qui ne change pas** : ADR-001. Le poste de l'établissement reste la source de vérité ; le cloud est une copie montante destinée aux parents et à l'e-learning en mode cloud (§16.3), jamais l'inverse.

**Conservé sans négociation** : les identifiants ULID (ADR-004), les colonnes `mis_a_jour_le` et `identifiant_poste` (R-13), et l'écriture dans l'`outbox` (ADR-009). Ce sont les prérequis de la synchronisation qui sont impossibles à ajouter rétroactivement ; ils restent posés dès la phase 0.5.

### 24.4 Conséquences de Q-15 à Q-19

**Saisie progressive abandonnée.** Elle avait été envisagée pour absorber les dossiers incomplets. La date de naissance étant systématiquement exigée à l'inscription, elle n'a plus d'objet. `Personne.date_naissance` devient obligatoire pour un élève.

**Le drapeau `date_naissance_estimee` est conservé.** Une date issue d'un jugement supplétif reste approximative, et cette approximation doit apparaître sur les documents destinés aux examens.

**Matricule.** `Eleve.matricule_interne` reste obligatoire partout et fait autorité en interne ; depuis le 7 octobre 2026, l'école le saisit elle-même ou le fait générer selon ses règles (préfixe, année, longueur, remise à zéro ; `SPEC_PHASE_2.md` §4.2). `Eleve.matricule_national`, donné par l'État aux examens, se recopie et ne se génère jamais ; il devient obligatoire **dès la 9e (DEF)** et aux rangs 10 à 12 (révision du 7 octobre 2026), avec index d'unicité partiel — appliqué aux seules valeurs renseignées.

**Le moteur de calcul est unique.** Une seule formule de moyenne de matière, une seule formule de moyenne générale, quel que soit le niveau. Les différences entre cycles sont portées par des paramètres — `bareme`, `utilise_coefficients` — jamais par des branches de code.

### 24.5 Décisions du 3 octobre 2026

Questions Q-20 à Q-25, tranchées le 3 octobre 2026 (Q-23 et Q-24 lors de l'étude des programmes, les autres en réponse à la liste des points ouverts).

| Id | Question | Décision | Effet |
|---|---|---|---|
| **Q-20** | La moyenne de matière est-elle tronquée avant d'alimenter la moyenne générale ? | **Oui** — `tronquer_avant_report = vrai` par défaut | §8.4bis, RC-7 |
| **Q-21** | Au 1er cycle, quel barème pour la composition mensuelle ? | **/10** | §8.4bis. La formule s'y adapte sans modification |
| **Q-22** | Au 1er cycle, bulletin mensuel ou trimestriel ? | **Mensuel — neuf périodes** | §24.6 |
| **Q-23** | Dans le curriculum 2011, un domaine (LC, SMT, SH, Arts, DP) devient-il une `Matiere` ? | **Oui** — c'est le grain auquel les horaires sont donnés | §8.2, §16.5 |
| **Q-24** | Curriculum 2011 : 53 séances par semaine annoncées, 50 au détail par domaine. Lequel fait foi ? | **50**, le détail par domaine | §16.5 |
| **Q-25** | Volumes horaires de la 3e année (aucune grille officielle dans le dossier, l'emploi du temps fourni n'étant qu'un exemple) ? | **Reprendre ceux de la 4e année** (confirmé) | §16.5 |
| **Q-37** | Horaires du 1er cycle de la 3e à la 6e année, et force de la grille ? | **Grille du curriculum 2011 par défaut sur tout le 1er cycle, indicative, jamais bloquante** : aucun contrôle ne refuse un emploi du temps ou une classe qui s'en écarte. Souplesse limitée aux horaires. Remplace Q-25 | §16.5, `ProgrammeMatiere` |

*Coefficients au 1er cycle* : Q-16 reste en vigueur. Aucun document ne donne de coefficient pour les rangs 1 à 6, ce qui est cohérent avec `utilise_coefficients = false` ; les coefficients du référentiel du 1er cycle restent donc vides, et ce n'est pas une donnée manquante.

**Plus aucune question pédagogique ne bloque la phase 4.** Les questions de §24.7 (arrivée en cours d'année, transferts) sont tranchées elles aussi.

### 24.6 Conséquences de Q-22 — bulletin mensuel au 1er cycle

`PeriodeAcademique` étant rattachée à l'année **et au niveau**, une même année scolaire porte neuf périodes pour les rangs 1 à 6 et trois pour les rangs 7 à 12. Le modèle n'évolue pas.

**Volume documentaire.** Pour 400 élèves au 1er cycle, 3 600 bulletins par an au lieu de 1 200. La génération en lot (§13.3) devient une cible de performance mesurée : **400 bulletins en moins de 2 minutes sur le matériel cible**, avec barre de progression et reprise après interruption. Typst compilant en Rust hors du webview, la marge est confortable — mais l'objectif entre dans les tests de charge (§21.1).

**Volume de notification.** Neuf notifications de bulletin par an et par parent au lieu de trois, soit 3 600 SMS annuels pour ce seul motif sur 400 élèves. Chez un agrégateur à 20 FCFA, 72 000 FCFA par an que l'école refusera. **Confirmation définitive de Q-14** : le modem GSM n'est pas une préférence technique, c'est la condition de viabilité de la notification.

**Charge de saisie.** Neuf campagnes de saisie de notes par an au 1er cycle. L'exigence de saisie clavier rapide (R-42) s'en trouve renforcée : ce qui coûte trente secondes de trop par classe se paie neuf fois.

**Levé le 3 octobre 2026.** Q-20 et Q-21, qui bloquaient la phase 4, sont tranchées (§24.5).

### 24.7 Décisions — arrivée en cours d'année et transfert entre lycées

*Tranchées le 3 octobre 2026*, à la demande de l'utilisateur, en suivant la logique du blueprint et en privilégiant la flexibilité : quand c'est raisonnable, un **paramètre d'établissement avec une valeur par défaut** plutôt qu'une règle figée. Elles prolongent Q-09 (changement de classe en cours d'année, avec historique), Q-18 (matricule national au lycée) et RC-3 (l'absent n'est pas un zéro).

**A. Élève qui arrive en cours d'année**

| Id | Question | Décision | Effet | Phase |
|---|---|---|---|---|
| **Q-26** | Que facture-t-on à un élève inscrit en cours d'année ? | **Paramètre `facturation_arrivee_en_cours`**, par établissement : `echeances_restantes` (**défaut** : frais d'inscription entiers, scolarité sur les seules échéances non échues à la date d'inscription), `prorata_mensuel` ou `plein_tarif`. Toute autre réduction passe par une remise tracée (§3.6) | Génération de l'échéancier à la validation (§10.1) | Phase 3 |
| **Q-27** | Comment compter les périodes antérieures à l'arrivée ? | **Elles sont « non suivies »** : exclues du numérateur et du dénominateur de la moyenne annuelle, comme une note absente (RC-3). Les pondérations des périodes suivies ne sont pas renormalisées à la main : la formule s'en charge | §8.4bis, `moyenne_annuelle` | Phase 4 |
| **Q-28** | Reprend-on les notes de l'ancienne école ? | **Oui, seulement les moyennes de période, sur présentation d'un relevé officiel**, saisies comme `MoyenneReportee` (source = externe, établissement d'origine, pièce jointe). Jamais les notes de détail | Nouvelle entité, §8.4 | Phase 4 |
| **Q-29** | L'élève est-il classé (rang) à sa période d'arrivée ? | **Paramètre `classer_arrivee_en_cours`, vrai par défaut** : l'élève est classé dès qu'il a une moyenne sur la période, avec la mention « arrivé le … » au bulletin ; rang annuel sur les périodes suivies, même mention. À faux, il figure « non classé » jusqu'à la période suivante | Bulletin, §13 | Phase 4 |
| **Q-30** | Qui décide du passage d'un élève qui n'a suivi qu'une partie de l'année ? | **Le conseil de classe, jamais le calcul seul** : la décision proposée est `non statué` tant qu'il n'a pas tranché | `decision_fin_annee`, UC-06 | Phase 10 |
| **Q-31** | Faut-il une date limite d'inscription en cours d'année ? | **Non, un paramètre d'établissement qui avertit sans bloquer** (calendrier instable, §3.8) | `RegleEvaluation` ou paramètres d'année | Phase 2 |

**B. Transfert entre lycées (et entre écoles)**

| Id | Question | Décision | Effet | Phase |
|---|---|---|---|---|
| **Q-32** | Quels documents produit le transfert sortant ? | **Un certificat de transfert numéroté et un relevé des moyennes des périodes suivies**, sans rang pour la période en cours. Les deux portent le matricule national au lycée (Q-18) | Moteur documentaire, §13 | Phase 4 |
| **Q-33** | Un impayé bloque-t-il le certificat de transfert ? | **Non par défaut : avertissement et mention au dossier.** Paramètre d'établissement pour exiger un solde nul, la direction pouvant passer outre avec motif tracé | `enrollments:transfer`, audit | Phase 3 |
| **Q-34** | Que deviennent les échéances d'un élève transféré ? | **Les échéances échues restent dues ; les échéances non échues sont annulées par contre-passation**, jamais supprimées (R-14). Aucun remboursement automatique : il passe par une décision tracée | §8.8, §10.2 | Phase 3 |
| **Q-35** | À l'arrivée au lycée, l'élève peut-il changer de série ? | **En cours d'année, même niveau et même série obligatoires.** Un changement de série est une décision de direction, motivée et tracée (Q-09) | `Inscription`, `Serie` | Phase 2 |
| **Q-36** | Entre deux établissements d'une même organisation (ADR-006), les données suivent-elles ? | **Pas avant la synchronisation (phase 11)** : un poste par école (Q-13), donc transfert par documents et `MoyenneReportee`, comme pour une école extérieure | ADR-006, §15.2 | Phase 11 |

**Ajouts au modèle** (décidés, à poser dans les migrations de la phase concernée) :
- `Inscription` : `date_arrivee`, `etablissement_origine` (texte libre), `date_depart`, `etablissement_destination` (texte libre), `motif_depart`.
- `MoyenneReportee` — `(inscription, periode, matiere?)`, valeur et barème (`Note`), `source = externe`, `etablissement_origine`, `PieceJointe` du relevé. Elle entre dans la moyenne annuelle comme une moyenne de période ordinaire.
- `CertificatTransfert` — document numéroté par le service de numérotation (ADR-004), émis à la transition `VALIDÉE → TRANSFÉRÉE`.
- Paramètres d'établissement : `facturation_arrivee_en_cours` (Q-26), `classer_arrivee_en_cours` (Q-29), `alerte_inscription_tardive` (Q-31), `solde_nul_pour_transfert` (Q-33, faux par défaut).

*Écarts avec les recommandations initiales* : Q-26 et Q-29 deviennent des paramètres d'établissement (valeur par défaut = la recommandation) ; les neuf autres sont retenues telles quelles.

### 24.8 Questions ouvertes — revue du 4 octobre 2026

Questions issues de la revue de cohérence (§28, v0.17) et de l'étape 11 de la phase 0.5. Q-38 est tranchée ; Q-41 est implémentée selon sa recommandation, à confirmer ; Q-39 et Q-40 restent ouvertes.

| Id | Question | Recommandation | Effet | Bloque |
|---|---|---|---|---|
| **Q-38** | Comment une sauvegarde se restaure-t-elle sur une autre machine, alors que la clé de la base dépendait d'un secret machine (§17.3 v0.16) ? | **Clé de données sous deux enveloppes, dont une enveloppe de secours protégée par une phrase de douze mots imprimée à l'installation** (§17.3). La sauvegarde embarque l'enveloppe de secours, jamais l'enveloppe machine. Alternative écartée : dériver la clé du seul mot de passe administrateur — un mot de passe oublié ou un administrateur parti rendraient la base irrécupérable, et le mot de passe changerait à chaque rotation de personnel | §17.3, §18, R-57, SPEC_PHASE_0_5 §6.3 et §11 | Tranchée le 4 octobre 2026 : recommandation retenue (consigne « on implémente ») |
| **Q-39** | Sur un poste unique (Q-13), qui saisit physiquement les notes ? UC-03, §11.3 et §11.5 supposent l'enseignant au clavier ; neuf campagnes par an au 1er cycle (Q-22) rendent la file d'attente devant un seul écran irréaliste | **Trois canaux, tous tracés** : (1) le secrétariat saisit depuis le relevé papier signé de l'enseignant, qui reste la pièce source et est joint à l'évaluation (`PieceJointe`) — le rôle Secrétaire reçoit `grades:create` sur tout l'établissement ; (2) l'enseignant saisit lui-même quand il le peut, sous la portée du §11.3 ; (3) import d'une grille Excel remplie par l'enseignant (§14.1). Dans les trois cas `Evaluation.saisie_par` et `Note` portent l'auteur réel de la saisie **et** l'enseignant correcteur. La contrainte « aucun rôle ne cumule `grades:*` et `payments:*` » (§11.5) tient : le secrétaire n'encaisse pas | §8.4, §11.3, §11.4, §14.1, UC-03 | Phase 4 |
| **Q-40** | Quelle durée pour chaque phase, et quelle date cible pour le jalon commercial (fin de phase 4) ? | **Écrire un ordre de grandeur par phase avant d'entrer en phase 2**, en semaines de travail effectif, et utiliser la phase 0.5 comme étalon : son coût réel mesuré (étapes 1 à 13) calibre les suivantes. Fixer la date du pilote (§22.4) à partir de cette mesure, pas d'une intention. Tant que ce chiffrage n'existe pas, toute fonctionnalité hors §1.3 est refusée sans discussion | §22.2, §22.4, RA-03 | Phase 2 |
| **Q-41** | Régénérer la phrase de secours doit-il rendre illisibles les sauvegardes antérieures ? | **Non, recommandation appliquée par défaut le 4 octobre 2026.** L'ancienne phrase cesse d'ouvrir la clé du poste et les sauvegardes suivantes ; les archives antérieures embarquent l'ancienne enveloppe et restent ouvrables avec elle, jusqu'à leur disparition par rotation (douze mois au plus, §18.1). Alternative écartée par défaut : changer aussi la clé de données à chaque régénération, ce qui impose de rechiffrer la base et rend les archives antérieures inutilisables sans l'ancienne phrase | §17.3, §18, R-57 | Rien (à confirmer) |
| **Q-42** | Quelle permission pour créer et activer une année scolaire ? | **`academic_year:manage`**, ajoutée au catalogue (§11.2), pour l'Administrateur et le Directeur | §8.1, §10.8, `SPEC_PHASE_1.md` §3 | Tranchée le 7 octobre 2026 : recommandation retenue |
| **Q-43** | Un rôle personnalisé peut-il cumuler `grades:*` et `payments:*` ? | **Non**, comme les rôles livrés (§11.5) ; une même personne reçoit deux rôles, le cumul reste nominatif et audité | §11.4, §11.5, `SPEC_PHASE_1.md` §5.3 | Tranchée le 7 octobre 2026 : recommandation retenue |
| **Q-45** | Quelle permission pour renommer les niveaux et créer les classes ? | **`academic_year:manage`** (Q-42), sans permission nouvelle | `SPEC_PHASE_2.md` §2.4, §3.2 | Non : appliquée par défaut le 7 octobre 2026 |
| **Q-46** | Au jardin, combien de personnes qui viennent chercher l'enfant faut-il pour valider ? | **Au moins une, avertissement en dessous de deux** (T-02) | `SPEC_PHASE_2.md` §7.2 | Non : appliquée par défaut |
| **Q-47** | Un brouillon d'inscription s'abandonne-t-il ? | **Oui, `BROUILLON → ABANDONNÉE` avec motif** (R-14) | §10.1 | Non : appliquée par défaut |
| **Q-44** | Composition par défaut des rôles livrés ? | **Matrice de `SPEC_PHASE_1.md` §5.2** ; `grades:create` du Secrétaire suspendu à Q-39 | §11.4, `SPEC_PHASE_1.md` §5.2 | Tranchée le 7 octobre 2026 : composition retenue |

---

### 24.9 Décisions du 9 octobre 2026 — phase 3

Tranchées par le porteur du projet, recommandations retenues (détail : `SPEC_PHASE_3.md` §16) :

| Id | Question | Décision |
|---|---|---|
| **Q-48** | Permission pour saisir les frais | `fees:manage`, nouvelle : Administrateur, Directeur, Comptable ; rôles livrés en version 3 |
| **Q-48 bis** | Trousseau partagé par toutes les installations d'un compte Windows | Corrigé en étape 0 de la phase 3 : une entrée par clé de données |
| **Q-49** | Validation sans grille de frais | Refusée ; tarif à zéro possible |
| **Q-50** | Factures des inscriptions validées en phase 2 | Commande de rattrapage de la direction, avec aperçu et sauvegarde |
| **Q-51** | Facture en brouillon | Non : émise à la validation ; §10.2 amendé |
| **Q-52** | Échéances visées par une remise | Les dernières non soldées, en remontant |
| **Q-53** | Annulation d'un versement d'une session clôturée | Par la direction, régularisation dans sa session ouverte, signalée |

Réponses de terrain qui modifient le blueprint : la scolarité se paie selon un **plan choisi par le parent** parmi ceux de l'école (T-12, §8.8) ; une **session de caisse peut durer plusieurs jours** (T-21, §10.9).

Réponses anticipées pour la phase 4 (T-22 à T-25, `docs/questions-terrain.md`) : **Q-39 tranchée en partie, la direction saisit les notes** à partir du relevé de l'enseignant (le Directeur recevra `grades:create`) ; plusieurs enseignants peuvent être affectés à une même matière d'une classe ; un vacataire intervient dans plusieurs écoles ; au 1er cycle, un maître reçoit toutes les matières de sa classe, une matière pouvant être confiée à un autre.

## 25. Extensions armées

### 25.1 Le critère de tri

Toutes les fonctionnalités futures ne se valent pas. Certaines s'ajoutent quand on veut, d'autres deviennent hors de prix si on ne les a pas anticipées. Le critère est le **rapport d'asymétrie** entre le coût d'anticipation et le coût de rattrapage.

| Rapport | Décision |
|---|---|
| Coût de rattrapage ≈ coût d'anticipation | Ne rien faire. Construire le jour où le besoin existe. |
| Rattrapage 5 à 10 fois plus cher | Poser la couture. Ne pas implémenter. |
| Rattrapage impossible sans migration à risque sur données de production | **Poser la couture maintenant, sans discussion.** |

Une couture n'est pas une fonctionnalité désactivée. C'est une colonne, une interface, une convention de nommage ou un point d'extension — quelque chose qui ne fait rien aujourd'hui et qui rend possible demain.

Ce qui suit est classé du rapport le plus fort au plus faible.

---

### 25.2 Événements métier nommés dans l'`outbox`

**Couture** : l'`outbox` (ADR-009) enregistre des **événements sémantiques** — `BulletinPublie`, `VersementEncaisse`, `AbsenceConstatee`, `SanctionProncee`, `InscriptionValidee` — et non des différences de lignes anonymes. Chaque événement porte son type, l'entité concernée, l'horodatage, l'acteur, et une charge utile minimale.

**Coût aujourd'hui** : une discipline de nommage. Aucune ligne de code supplémentaire, puisque l'écriture dans l'`outbox` est déjà obligatoire (R-17).

**Coût du rattrapage** : reconstruire l'historique d'événements à partir de différences de lignes est impossible. On ne retrouve pas *pourquoi* une ligne a changé.

**Ce que ça débloque** : le moteur de notification (§25.3), le portail parent, la timeline élève (§25.7), toute intégration future, et le rejeu d'historique. Ces quatre consommateurs lisent le même flux au lieu d'être quatre implémentations parallèles.

**C'est la couture au plus fort rapport d'asymétrie de tout le projet.**

---

### 25.3 Canal de notification abstrait, avec file persistante

**Couture** : le moteur de notification de la phase 5 sépare trois choses que l'on confond spontanément :

```
Événement (§25.2) → Règle de notification → Message (fait + destinataire)
                                                   ↓
                                          File persistante
                                                   ↓
                                    Canal : SMS | WhatsApp | portail | vocal
```

Un seul canal est implémenté — le modem GSM. L'interface `Canal` existe, avec une seule implémentation.

**Coût aujourd'hui** : quasi nul. La **file persistante avec réessai est de toute façon obligatoire** pour le modem : plus de crédit, réseau absent, modem débranché, message trop long. Sans file durable, un SMS perdu l'est définitivement, et personne ne le sait.

**Coût du rattrapage** : ajouter un deuxième canal à un moteur qui appelle directement le modem suppose de reprendre chaque point d'envoi.

**Ce que ça débloque** : WhatsApp Business le jour où c'est accessible, l'email, la notification du portail, et la messagerie vocale (§25.8).

---

### 25.3bis Contraintes opérationnelles du canal SMS

Section ajoutée le 27 août 2026, après relevé des tarifs.

#### Pourquoi le modem, chiffres à l'appui

L'API SMS d'Orange Mali est facturée par paquets, de 3 500 FCFA les 100 SMS à 800 000 FCFA les 50 000, avec une validité de 30 à 45 jours (portail développeur, tarifs 2024, à revérifier).

Pour une école de 400 élèves au 1er cycle, avec bulletin mensuel :

| Motif | Volume annuel |
|---|---|
| Notification de bulletin | 3 600 |
| Relances d'impayés | ~2 000 |
| Absences | variable, ~1 500 |
| **Total** | **~7 000** |

Au tarif du paquet correspondant à ce rythme mensuel — environ 30 à 35 FCFA l'unité — cela représente **plus de 200 000 FCFA par an**. Aucune école privée malienne ne validera cette ligne budgétaire pour un logiciel qu'elle a déjà payé.

Deux caractéristiques du tarif aggravent le cas dans ce contexte précis :

- **La validité de 30 jours** ne correspond pas au profil d'usage d'une école, qui est en pics : massif à la sortie des bulletins, quasi nul entre. Un paquet acheté pour un pic et non consommé est perdu.
- **Le tarif dégressif ne devient intéressant qu'à 10 000 SMS**, volume qu'une école seule n'atteint pas.

#### Contraintes du modem GSM

Le modem résout le coût mais introduit six contraintes, toutes gérables si elles sont anticipées.

**C-1 — Le matériel doit exposer un port série AT.** Une part des clés 3G/4G modernes fonctionne en mode routeur — elles se présentent comme une carte réseau et n'exposent aucun port AT. Elles sont inutilisables ici. **Tester un exemplaire avant tout achat en quantité**, et documenter les modèles validés.

**C-2 — Débit limité.** Un modem GSM envoie de l'ordre de 5 à 10 SMS par minute. Une campagne de 400 bulletins prend donc environ une heure. Conséquence : la file d'attente (§25.3) est **obligatoire et strictement asynchrone**. L'utilisateur lance la campagne et continue à travailler ; il n'attend jamais.

**C-3 — Les accents doublent le coût.** Un SMS en alphabet GSM-7 contient 160 caractères. Un seul caractère accenté bascule le message en UCS-2, qui n'en contient que **70**. « Votre enfant était absent » devient deux SMS au lieu d'un.

*Conséquence de conception* : le canal SMS **translittère systématiquement** vers GSM-7 avant envoi — « eleve », « etait », sans accents. C'est l'usage normal du SMS de toute façon. L'écran de composition affiche en permanence le nombre de segments, comme un téléphone. Ce seul point divise la consommation par deux.

**C-4 — Risque de blocage pour envoi en masse.** Une carte SIM prépayée qui émet plusieurs centaines de SMS en une heure peut être considérée comme émettrice de spam et suspendue par l'opérateur. **C'est le risque le plus sérieux de cette approche.**

*Atténuation* : limitation de débit paramétrable, étalement de la campagne sur plusieurs heures, fenêtre d'envoi excluant la nuit, et — pour un client important — déclaration de l'usage à l'opérateur. Prévoir également un basculement manuel vers l'API en cas de suspension.

**C-5 — Le crédit s'épuise silencieusement.** Un prépayé vide ne produit pas d'erreur explicite : les messages échouent, et personne ne le sait avant qu'un parent se plaigne.

*Conséquence de conception* : interrogation périodique du solde par USSD, seuil d'alerte, et **blocage de la file avec notification à la direction** plutôt qu'échec silencieux. Un compteur de consommation mensuelle avec plafond paramétrable protège du dérapage.

**C-6 — Hors-réseau.** Les parents sont répartis entre opérateurs. Une SIM unique paie le tarif hors-réseau pour une partie des envois. Un second modem avec une SIM d'un autre opérateur est possible, mais complique l'exploitation. À n'envisager qu'après mesure de la répartition réelle sur un client pilote.

#### Architecture retenue

Le canal SMS reste **une implémentation parmi d'autres derrière l'interface `Canal`** (§25.3). Deux implémentations sont prévues :

1. **Modem GSM** — par défaut, coût marginal, fonctionne sans internet.
2. **API opérateur** — repli, pour les gros volumes ponctuels ou en cas de suspension de la SIM.

Le choix est un paramètre d'établissement, et la file d'attente est commune. Une école peut router les bulletins par le modem et les urgences par l'API.

*Motif* : c'est précisément ce que l'abstraction de canal rendait possible. Sans elle, un changement d'opérateur ou une suspension de SIM imposerait de reprendre chaque point d'envoi.

---

### 25.4 Consultation par SMS entrant


C'est l'extension qui transforme la notification en consultation, **sans serveur, sans internet, sans portail**.

**Principe** : le modem GSM reçoit aussi. Un responsable envoie un code au numéro de l'école ; le poste répond automatiquement.

```
Parent  →  "NOTE 4B2X7"      →  Moyenne de la période, rang, absences
Parent  →  "SOLDE 4B2X7"     →  Reste à payer, prochaine échéance
Parent  →  "ABS 4B2X7"       →  Absences de la période, dont non justifiées
```

**Coût pour l'école** : un SMS sortant par requête. Aucune infrastructure au-delà du modem déjà présent en phase 5.

**Points à trancher avant implémentation** :

- **Authentification.** Le matricule seul ne suffit pas : n'importe qui pourrait interroger n'importe quel élève. Un **code d'accès confidentiel** est remis au responsable à l'inscription, distinct du matricule, révocable, régénérable. Le contrôle croise le code et le numéro appelant.
- **Périmètre exposé.** Notes, moyennes, absences et solde financier. **Ni sanctions, ni appréciations, ni décisions de conseil de classe** — voir §25.6.
- **Limitation de débit.** Un numéro qui envoie trente requêtes par jour vide le crédit de l'école. Quota quotidien par numéro.
- **Journalisation.** Toute consultation entrante est auditée : qui a demandé quoi, depuis quel numéro.

**Statut** : extension de la phase 5, à implémenter une fois le canal sortant stabilisé.

---

### 25.5 Identité de responsable réutilisable

**Couture** : le code d'accès du §25.4 n'est pas un gadget SMS. C'est une **identité de responsable**, avec un identifiant, un secret, un état d'activation et une date de dernière utilisation.

**Coût aujourd'hui** : une table et un code imprimé sur la fiche d'inscription.

**Coût du rattrapage** : si le portail arrive un jour avec son propre système de comptes, il faut ré-enrôler plusieurs centaines de responsables, un par un, sans canal fiable pour leur transmettre leurs identifiants. C'est le genre de tâche qui fait échouer un lancement de portail.

**Ce que ça débloque** : le portail parent réutilise l'identité existante. Le responsable a déjà son code depuis deux ans.

---

### 25.6 Qualité des numéros et consentement

Deux colonnes, un impact disproportionné.

**Normalisation E.164.** Les numéros sont stockés sous forme canonique, jamais tels que saisis. Au Mali les gens changent souvent de numéro et en possèdent plusieurs. Sans normalisation, la même personne existe sous trois formes et reçoit trois SMS — ou aucun.

**État de joignabilité.** Chaque numéro porte un état — non vérifié, vérifié, invalide, injoignable — alimenté par les accusés de remise du modem. **Sans cela, une campagne de relance brûle silencieusement du crédit sur des numéros morts**, et l'école conclut que le logiciel ne sert à rien.

**Consentement.** Un indicateur par responsable et par type de message. Un parent qui demande à ne plus recevoir de relances doit pouvoir être retiré, immédiatement, sans qu'on désactive aussi son bulletin. C'est une exigence de la loi sur les données personnelles, et surtout une exigence de bon sens commercial.

**Sensibilité par type de message.** Les sanctions et les décisions disciplinaires ne partent **jamais** automatiquement. Le moteur prépare le message, un humain décide de l'envoyer. Une exclusion annoncée par robot avant que le surveillant général ait parlé au parent est un incident que l'école reprochera au logiciel, à juste titre.

---

### 25.7 Timeline de l'élève

**Couture** : dès lors que les événements sont nommés (§25.2), un écran chronologique unique par élève devient presque gratuit — inscription, notes, absences, sanctions, versements, documents émis, dans un seul fil.

**Coût aujourd'hui** : un écran de lecture, aucune donnée supplémentaire.

**Pourquoi c'est stratégique** : c'est l'écran qui vend le produit en démonstration. Un directeur qui voit trois ans de la vie d'un élève défiler en une seconde comprend immédiatement ce qu'il achète — bien mieux qu'avec une liste de fonctionnalités. C'est aussi exactement ce que le portail parent affichera un jour.

---

### 25.8 Séparation du fait et de sa formulation

**Couture** : un événement porte des **données**, pas une phrase. La formulation appartient au canal et à la locale (ADR-015).

```
AbsenceConstatee { eleve, date, demi_journee, justifiee: false }
        ↓
SMS français   : « Votre enfant X était absent le 12/03, matinée, non justifiée. »
SMS bambara    : (même fait, autre catalogue)
Portail        : ligne dans la timeline
Vocal          : séquence audio composée
```

**Coût aujourd'hui** : nul, puisque l'i18n est déjà décidée.

**Ce que ça débloque, et qui compte ici** : une part réelle des parents ne lit pas le français avec aisance. Un SMS en bambara, ou à terme un appel vocal automatique composé à partir de fragments enregistrés, atteint une population qu'un SMS français n'atteint pas. Si le message est fabriqué comme une chaîne de caractères au moment de l'envoi, cette option est fermée pour toujours.

Le canal vocal reste spéculatif et n'est pas planifié. La couture, elle, ne coûte rien.

---

### 25.9 Vérifiabilité des documents

**Couture** : chaque document produit porte déjà son empreinte SHA-256 et sa référence (§8.1). Il suffit d'imprimer un **QR code** encodant cette référence sur les documents officiels — reçu, bulletin, certificat de scolarité, attestation.

**Coût aujourd'hui** : quelques lignes dans les modèles Typst.

**Ce que ça règle** : le faux reçu. Un élève présente une quittance falsifiée, l'école ne peut pas trancher sans fouiller ses registres. Avec le QR, la vérification est immédiate — et le jour où un service en ligne existe, un tiers peut vérifier un certificat de scolarité sans appeler l'école.

**C'est un argument commercial direct auprès du promoteur**, qui se préoccupe de la fraude interne bien plus qu'on ne le dit ouvertement.

---

### 25.10 Portabilité des données

**Couture** : une commande d'export intégral des données d'un établissement, dans un format ouvert et documenté.

**Coût aujourd'hui** : faible, puisque construit sur la couche de dépôt existante.

**Pourquoi le faire tôt** : c'est une obligation probable au titre de la protection des données, mais surtout un **argument de vente**. Une école qui sait qu'elle peut partir n'a pas peur de venir. Le verrouillage par la difficulté de sortie est une stratégie perdante sur un marché où la réputation circule vite et de bouche à oreille.

---

### 25.11 Ce qu'on n'arme délibérément pas

Par symétrie, il faut nommer ce qu'on refuse d'anticiper, pour que la liste précédente ne s'allonge pas indéfiniment :

- **Reconnaissance biométrique pour l'appel.** Rattrapage aussi coûteux que l'anticipation. Aucune couture.
- **Application mobile native pour les parents.** Le portail web couvre le besoin ; une application n'ajoute rien qu'un lien WhatsApp ne fasse déjà.
- **Comptabilité générale SYSCOHADA.** Hors périmètre (§1.2). Si le besoin apparaît, c'est un module séparé alimenté par les exports.
- **Classe virtuelle temps réel.** Hors périmètre, et incompatible avec la connectivité cible.
- **Intelligence artificielle prédictive sur le décrochage.** Rien à anticiper : les données historiques suffiront le jour venu, et elles sont déjà conservées.

---

## 26. Éditions du produit

### 26.1 Principe de découpage

Le marché n'est pas homogène : certaines écoles ont une connexion et un budget récurrent, la plupart n'ont ni l'un ni l'autre. Deux éditions y répondent.

**Un seul binaire, un seul code.** Les éditions sont des **modules activés par la licence signée**, jamais deux applications distinctes. Deux bases de code divergent en dix-huit mois, et un développeur seul ne maintient pas deux produits.

**Règle de découpage** : ce qui distingue les éditions est exclusivement **ce qui coûte de l'argent récurrent à l'éditeur** — hébergement, messages facturés par un tiers, stockage hors site. Rien d'autre.

*Motif.* Retirer une fonction du cœur métier pour forcer la montée en gamme est une stratégie perdante sur un marché où la réputation circule de bouche à oreille. Une école qui se sent rançonnée le dira à dix autres. À l'inverse, facturer un service dont le coût est réel et vérifiable se défend en négociation sans difficulté.

### 26.2 Édition Essentielle

**Modèle** : licence perpétuelle, maintenance annuelle optionnelle (Q-12). Aucun coût récurrent pour l'éditeur, donc aucun abonnement obligatoire.

**Contenu — le produit complet, pas une démonstration bridée** :

CORE intégral · Scolarité intégrale · Finance intégrale · RH et paie · Moteur documentaire et modèles personnalisés · **Sauvegarde et restauration** · Rapports et états CPS/SE · Notification SMS par modem GSM · Liens `wa.me` · Consultation par SMS entrant (§25.4) · Mode LAN si le besoin apparaît.

**Interdiction permanente** : la sauvegarde et la restauration ne sont **jamais** une fonction payante. Sur une machine unique sans autre copie vivante (RA-12), en faire un levier commercial reviendrait à monnayer le risque de perte totale du client.

### 26.3 Édition Connectée

**Modèle** : abonnement annuel, dont le montant couvre un coût réel et justifiable.

**Contenu additionnel** :

Synchronisation cloud · Portail parent en PWA (ADR-017) · Canal WhatsApp Business API · Sauvegarde hors site · E-learning en mode cloud.

**Cible** : les écoles privées de Bamako disposant déjà d'une connexion permanente. Pas les autres — leur proposer l'édition Connectée serait leur vendre une dépendance qu'elles ne peuvent pas honorer.

### 26.4 Modules optionnels

Disponibles dans les deux éditions, facturés séparément : **multi-établissement** (par établissement supplémentaire, ADR-006), **modèles de documents sur mesure** (§13.2, niveau 3).

### 26.5 Mécanique de licence

Licence signée Ed25519, vérifiée **hors ligne et en Rust** (jamais dans le frontend, R-02). Elle porte l'établissement, l'édition, les modules actifs, l'échéance éventuelle et l'empreinte machine.

Quatre règles, toutes destinées à protéger le client plutôt que l'éditeur :

1. **L'expiration d'un abonnement ne bloque jamais l'accès aux données ni les fonctions de l'édition Essentielle.** Elle désactive uniquement les modules connectés. Une école qui cesse de payer garde son logiciel et ses données ; elle perd le portail.
2. **Période de grâce généreuse.** Une école dont la connexion tombe, ou dont le virement tarde, ne doit pas se retrouver bloquée un lundi matin de rentrée.
3. **Aucune vérification en ligne obligatoire.** La licence est un fichier signé, pas un appel serveur. C'est la condition du fonctionnement local-first (ADR-001).
4. **Changement de machine possible sans intervention de l'éditeur** dans une limite raisonnable. Le contournement de licence est accepté comme un risque assumé (§3.13) ; punir un client honnête dont le disque a lâché coûte bien plus cher qu'un piratage.

### 26.6 Effet sur la roadmap

Aucun décalage. L'édition Essentielle correspond aux phases 0.5 à 10, déjà planifiées. L'édition Connectée correspond à la phase 11, déjà conditionnelle (M6). Le découpage commercial était donc **déjà porté par l'architecture** — c'est le bénéfice direct de la discipline des coutures (§25.1).

---

## 27. Prochaine étape

La spécification du socle existe (`SPEC_PHASE_0_5.md`) ; les étapes 1 à 12 de la phase 0.5 sont livrées, l'étape 13 (installeur) reste à faire. Deux prérequis restent ouverts :

**Trancher Q-39 et Q-40** (§24.8) avant la phase 2, et confirmer Q-41.

**Collecter les documents réels.** Bulletins vierges et remplis, fiches d'inscription, reçus, listes de classe, fiche CPS. Trois écoles au minimum. Ces documents déterminent le modèle de données autant que le moteur documentaire (risque RA-06), et c'est le seul intrant qu'aucune décision de conception ne peut remplacer.

Cette collecte n'est pas bloquante pour la phase 0.5 : le socle technique ne dépend d'aucun document métier. Elle devient bloquante à l'entrée de la phase 2.

**Révision du 7 octobre 2026 (décision du porteur du projet).** La collecte des documents de trois écoles est abandonnée comme prérequis. Elle est remplacée par des **questions posées au porteur du projet**, qui connaît les pratiques des écoles : chaque champ, document ou règle de terrain encore incertain devient une question écrite, et sa réponse datée fait foi au même titre qu'une décision du §24. Les questions de la phase 2 sont rassemblées dans `docs/questions-terrain.md`. Le risque RA-06 (modèle de données ou documents mal ajustés au terrain) reste ouvert : il se réduit question par question, puis à l'arrivée d'un établissement pilote (§22.4).

**Contenu de la spécification du socle (phase 0.5), livré dans `SPEC_PHASE_0_5.md`** :

- entités complètes du domaine CORE, avec champs, contraintes et index ;
- plan de migrations initiales ;
- matrice de permissions par rôle, avec portées (§11.3) ;
- format exact du journal d'audit et mécanique de chaînage (§12.2) ;
- procédure de sauvegarde, de restauration et de test de restauration (§18) ;
- mécanique de changement d'utilisateur rapide, exigée par RA-13 (§24.2) ;
- conventions de nommage, découpage des crates, mise en place de la génération de types ;
- **conventions de nommage des événements de l'`outbox` (§25.2)** — à figer avant la première écriture, car c'est la couture au plus fort rapport d'asymétrie du projet.

Coutures du §25 à poser dès les phases 0.5 à 2, dans l'ordre : événements nommés (§25.2), normalisation E.164 et consentement (§25.6), identité de responsable (§25.5), QR de vérification dans les modèles (§25.9).

Le code suit `ETAT_AVANCEMENT.md` ; aucune règle métier n'est écrite tant que la phase 0.5 n'est pas validée (§0.2 de la spécification).

---

## 28. Journal des révisions

| Version | Date | Modification |
|---|---|---|
| v0.5 et antérieures | 27 août 2026 | Q-01 levée, Q-02 à Q-14 tranchées (§24). Ajout de §8.4bis (moteur de calcul), §8.5bis (remise et convocations), §25.3bis (contraintes SMS). M5 à M7 (§22.3) |
| v0.10 | 3 octobre 2026 | Q-15 à Q-25 tranchées (§24, §24.5) ; Q-11 révisée (offre hébergée retenue, §24.3) ; filières du lycée (§8.2) ; référentiel des programmes dans l'e-learning (§16.1, §16.5) |
| v0.11 | 3 octobre 2026 | §24.7 : Q-26 à Q-36 ouvertes, avec recommandation |
| v0.12 | 3 octobre 2026 | Q-26 à Q-36 tranchées (§24.7) ; Q-26 et Q-29 deviennent des paramètres d'établissement |
| v0.13 | 3 octobre 2026 | Q-37 : grille du curriculum 2011 par défaut sur tout le 1er cycle, indicative, souplesse limitée aux horaires (remplace Q-25) |
| v0.14 | 4 octobre 2026 | §5.2 et §19.6 : React Aria Components, thème Tailwind vidé, règle des encres, polices compatibles bambara, interdits visuels |
| v0.15 | 4 octobre 2026 | Retour à shadcn/ui + Radix UI + Tremor (composants copiés), React 19, bleu principal et mode sombre, e-learning au style LMS (§19.7). Polices et contrastes de la v0.14 conservés |
| v0.16 | 4 octobre 2026 | Rust 1.99 édition 2024 et dernières versions stables (§5.4, exception `tauri-specta` 2.0 rc) ; aucun signe de vibe coding (§5.5) |
| v0.17 | 4 octobre 2026 | Revue de cohérence : titres §5 et §8.5 ajoutés ; §22.3/§22.4 et §24.1–§24.7 remis dans l'ordre ; risques renommés `RA-xx` (§23) ; §6 et ADR-014 réconciliés (mode LAN dans `gesco-app`, `gesco-server` réservé au cloud) ; §7 réaligné sur §22.2, module de notification et guichet ajoutés ; `mensuelle` ajouté aux types de période (§8.1) ; catalogue de permissions complété (§11.2) ; machines à états de la session de caisse et de la remise (§10.9, §10.10) ; §1.2 reformulé pour le serveur de l'édition Connectée ; §13.4 complété ; coût de réécriture des composants Tremor noté (§5.2) ; clé de secours indépendante de la machine (§17.3, §18) sous réserve de Q-38 ; **Q-38 à Q-40 ouvertes (§24.8)** ; journal des révisions déplacé ici (§28) |
| v0.18 | 4 octobre 2026 | Étapes 10 à 12 de la phase 0.5 : Q-38 tranchée (recommandation retenue) ; Q-41 ajoutée et appliquée par défaut (anciennes archives et phrase régénérée, §17.3, §24.8) ; jeton `sur-marine` (§19.6) |
| v0.22 | 7 octobre 2026 | Q-18 révisée : matricule national exigé dès la 9e (DEF) ; matricule interne saisi par l'école ou généré selon ses règles (§24.4) |
| v0.21 | 7 octobre 2026 | Phase 1 validée par le porteur du projet ; phase 2 spécifiée et engagée avant la validation de la phase 0.5, sans livraison à une école tant que la base n'est pas chiffrée (`SPEC_PHASE_2.md` §0.2) ; référentiel de la phase 2 limité aux niveaux, séries et classes (matières, programmes et périodes en phase 4) ; §8.2 sans effectif maximal (T-05) ; §10.1 amendé (Q-47) ; Q-45 à Q-47 ajoutées (§24.8) |
| v0.20 | 7 octobre 2026 | Questions de terrain T-01 à T-10 répondues (`docs/questions-terrain.md`) ; Q-03 révisée : le jardin d'enfants s'inscrit dès la phase 2, son évaluation reste reportée (§24.1) |
| v0.19 | 7 octobre 2026 | Collecte des documents réels remplacée par des questions au porteur du projet (§27) ; phase 1 spécifiée (`SPEC_PHASE_1.md` v1.0) ; Q-42 à Q-44 ajoutées et tranchées (§24.8) ; `academic_year:manage` au catalogue (§11.2) |
