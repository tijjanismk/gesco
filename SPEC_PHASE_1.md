# SPEC_PHASE_1.md — Administration de l'établissement

**Projet** : ERP de gestion scolaire — Tauri 2 + Rust + React/TypeScript
**Phase** : 1 — authentification, rôles, établissement, année scolaire, paramètres, audit
**Statut** : v1.4 — phase livrée le 7 octobre 2026, à valider
**Documents parents** : `PROJECT_BLUEPRINT.md` v0.18 (§8.1, §10.8, §11, §12, §17.1, §19.2, §22.2, §24.2), `ARCHITECTURE_RULES.md` v0.3, `SPEC_PHASE_0_5.md` v1.2
**Date** : 7 octobre 2026

---

## 0. Objet et critère de fin

### 0.1 Ce que produit cette phase

Le livrable du blueprint (§22.2) : **connexion, gestion des utilisateurs et des rôles**. À la fin de la phase, une école peut s'organiser seule dans GESCO : sa fiche est à jour et s'imprime sur ses documents, son année scolaire est ouverte, chaque personne du poste a son compte, son rôle et son code PIN, et le journal d'audit se lit sans aide.

Aucune règle de scolarité, de finance ou de RH n'entre dans cette phase. Elle reste dans le domaine CORE.

### 0.2 Point de départ : ce que la phase 0.5 a déjà livré

| Sujet | Déjà en place | Reste pour la phase 1 |
|---|---|---|
| Authentification | Argon2id, verrouillage progressif après échecs, changement exigé à la première connexion, PIN, verrouillage par inactivité, bascule d'utilisateur | Politique de mot de passe complète (§6), invitation à définir un PIN, déverrouillage sans PIN (§7) |
| Utilisateurs | Créer, modifier le nom, activer/désactiver, attribuer un rôle | Retirer un rôle, réinitialiser le mot de passe, déverrouiller un compte, garde-fous sur le dernier administrateur (§4) |
| Rôles | Dix rôles livrés ; Administrateur complet, les neuf autres vides | Composition des rôles livrés, rôles personnalisés, édition des permissions (§5) |
| Établissement | Fiche saisie au premier démarrage, paramètres modifiables | Fiche modifiable, logo, effet sur les documents (§2) |
| Année scolaire | Table `annee_scolaire`, unicité de l'année courante en base | Création, activation, affichage (§3) |
| Audit | Journal chaîné, vérification, liste des N dernières lignes | Filtres, noms lisibles, détail avant/après (§8) |

### 0.3 Ce que la phase 0.5 laisse ouvert

La phase 0.5 n'est pas validée : l'assertion 2 (base chiffrée) est fausse depuis le report de SQLCipher (7 octobre 2026), et l'essai sur machine vierge (assertions 1 à 3) attend de la place disque ou un autre ordinateur. `SPEC_PHASE_0_5.md` §0.2 interdit d'entrer en **phase 2** tant qu'une assertion est fausse, pas en phase 1 : cette phase reste dans le domaine CORE, qu'aucune de ces deux réserves ne touche.

**Conséquence** : la phase 1 peut se terminer, mais la phase 2 ne commence qu'après la validation de la phase 0.5.

### 0.4 Critère de fin, vérifiable

La phase est terminée quand ces dix assertions sont vraies, chacune démontrée par un test automatisé ou par le scénario de bout en bout (§10) :

1. L'administrateur crée un utilisateur et lui attribue un rôle livré ; connecté, cet utilisateur ne voit que les sections que son rôle autorise, et toute autre commande, même appelée directement, est refusée par la garde.
2. Les dix rôles livrés portent leur composition par défaut (§5.2) ; la matrice rôle × permission est générée et un test échoue si elle diverge du code.
3. Un rôle personnalisé se crée, se modifie et se supprime ; le rôle Administrateur ne se modifie ni ne se supprime ; chaque changement de rôle ou de permission est audité avec sa valeur avant et après.
4. Un compte désactivé ne se connecte plus et ses sessions ouvertes sont fermées ; un mot de passe réinitialisé par l'administrateur exige un changement à la connexion suivante ; le dernier administrateur actif ne peut être ni désactivé, ni privé de son rôle.
5. La politique de mot de passe paramétrée s'applique à la création, au changement et à la réinitialisation.
6. Passer d'un utilisateur à l'autre par l'écran verrouillé et le code PIN prend moins de 3 secondes, mesuré par le scénario de bout en bout.
7. La fiche de l'établissement modifiée (nom, coordonnées, logo, mentions légales) apparaît dans l'en-tête du document suivant ; la modification est auditée.
8. Une année scolaire se crée en préparation, s'active, devient l'année courante et s'affiche dans le bandeau ; les transitions interdites (§3.2) sont refusées, chacune par un test.
9. Le journal d'audit se filtre par utilisateur, entité, opération et période, affiche le nom de l'utilisateur et le détail avant/après, et sa chaîne se vérifie toujours.
10. Le scénario Playwright couvre chaque écran de la phase ; ses captures sont produites et montrées au porteur du projet.

---

## 1. Périmètre

**Inclus** : fiche d'établissement, année scolaire (préparation et activation), cycle de vie des comptes, rôles et permissions, politique de mot de passe, finitions de la session rapide (RA-13), journal d'audit lisible, navigation du §19.2 limitée aux sections existantes, essais de bout en bout avec captures.

**Exclu** :
- Clôture et archivage d'année (§10.8 du blueprint) : phase 10, UC-06. La permission `academic_year:close` existe déjà, l'écran non.
- Périodes académiques : rattachées au niveau (§8.1), elles attendent le référentiel de la phase 2.
- Portée ressource (§11.3) : la garde garde son implémentation qui accorde tout, jusqu'aux classes et matières de la phase 2.
- Campus : le schéma est multi-établissement, l'interface mono (Q-02). La table `campus` reste sans écran.
- Archivage signé du journal par exercice (§12.4) : avec la clôture, phase 10.

---

## 2. Établissement

### 2.1 Fiche

Champs modifiables : nom, statut (public / privé / communautaire / medersa), code de l'académie, code du CAP, adresse, téléphones (liste), mentions légales imprimées, logo. L'organisation propriétaire reste fixée à sa création.

*Règle* : `code_cap` et `code_academie` restent facultatifs en phase 1, mais l'écran signale leur absence : ils deviennent indispensables aux remontées CPS/SE (§8.1 du blueprint).

### 2.2 Logo

Image PNG ou JPEG, 1 Mo au plus, reconnue à ses premiers octets. **Elle est gardée dans la base**, dans deux colonnes de la table `etablissement` (migration 0005), et non comme `PieceJointe` sur disque : elle voyage ainsi avec chaque sauvegarde, et sera chiffrée avec la base le jour où SQLCipher sera actif. L'audit et l'événement n'en portent que l'empreinte SHA-256 et la taille. Elle entre dans l'en-tête commun des modèles Typst (`modeles/commun/entete.typ`). Un document déjà produit garde son logo d'origine : l'empreinte et la version du modèle du `Document` le prouvent (§8.1).

### 2.3 Commandes

| Commande | Permission | Effet |
|---|---|---|
| `etablissement_lire` | connectée | Fiche courante, pour le bandeau et les documents |
| `etablissement_modifier` | `core:settings:update` | Audit avant/après, événement `FicheEtablissementRectifiee` |
| `etablissement_logo_definir` | `core:settings:update` | Pièce jointe, audit, événement `LogoEtablissementRemplace` |

---

## 3. Année scolaire

### 3.1 Modèle

Table existante `annee_scolaire` : `libelle` (« 2026-2027 »), `debut`, `fin`, `statut`, `est_courante`. Une seule année courante par établissement, garantie par un index unique partiel (migration 0001).

*Règles* :
- le libellé suit la forme `AAAA-AAAA`, la seconde année valant la première plus un ;
- `debut` < `fin`, et l'année dure au plus 14 mois ;
- deux années non supprimées d'un même établissement ne se chevauchent pas.

### 3.2 Transitions de la phase 1

```
PRÉPARATION ──activer──► ACTIVE (courante)
```

| Depuis | Action | Vers | Condition |
|---|---|---|---|
| — | créer | Préparation | Aucun chevauchement |
| Préparation | modifier | Préparation | Libellé et bornes seulement |
| Préparation | activer | Active, courante | Aucune autre année active dans l'établissement |
| Préparation | supprimer | — (suppression logique) | Jamais activée |
| Active | toute action | — | **Refusée** en phase 1 : seule la clôture (phase 10) en sort |
| Clôturée, Archivée | toute action | — | Refusée (lecture seule, ADR-012) |

Une seconde année peut être créée en préparation pendant qu'une autre est active, pour préparer la rentrée. Elle ne s'active qu'après la clôture de la première, donc pas avant la phase 10.

### 3.3 Commandes

| Commande | Permission | Effet |
|---|---|---|
| `annees_lister` | connectée | Toutes les années de l'établissement |
| `annee_creer` | `academic_year:manage` (Q-42) | Audit, événement `AnneeScolairePreparee` |
| `annee_modifier` | `academic_year:manage` | Audit avant/après |
| `annee_activer` | `academic_year:manage` | Audit, événement `AnneeScolaireOuverte`, sauvegarde préalable, comme pour les opérations à risque du §11.1 de la spec 0.5 |
| `annee_supprimer` | `academic_year:manage` | Suppression logique, audit |

L'année courante s'affiche dans le bandeau, à côté du nom de l'établissement. Tant qu'aucune année n'est active, un avertissement permanent le signale.

---

## 4. Utilisateurs

### 4.1 Cycle de vie

| Action | Permission | Règle |
|---|---|---|
| Créer | `core:users:create` | Identifiant unique dans l'établissement, mot de passe initial conforme à la politique (§6), changement exigé à la première connexion |
| Modifier le nom affiché | `core:users:update` | Audité |
| Désactiver | `core:users:update` | Ferme immédiatement ses sessions ouvertes ; il ne se connecte plus |
| Réactiver | `core:users:update` | Audité |
| Réinitialiser le mot de passe | `core:users:update` | Nouveau mot de passe initial, changement exigé, PIN effacé, sessions fermées |
| Déverrouiller après échecs | `core:users:update` | Remet le compteur d'échecs à zéro |
| Attribuer un rôle | `core:roles:assign` | Audité |
| Retirer un rôle | `core:roles:assign` | Audité |

Un utilisateur ne se supprime pas : il se désactive. Ses lignes d'audit doivent rester attribuables (§12).

### 4.2 Garde-fous

- Le dernier administrateur actif ne peut être ni désactivé, ni privé du rôle Administrateur, y compris par lui-même.
- Un utilisateur ne modifie pas ses propres rôles.
- La réinitialisation du mot de passe d'un administrateur exige d'être soi-même administrateur.

### 4.3 Mot de passe oublié

Sur un poste unique sans messagerie, la seule voie est la réinitialisation par un administrateur. Si le dernier administrateur oublie son mot de passe, la récupération passe par la phrase de secours : restauration de la dernière sauvegarde sur le poste, puis réinitialisation. La procédure est décrite dans l'aide de l'écran de connexion. Aucune porte dérobée n'existe.

---

## 5. Rôles et permissions

### 5.1 Libellés

Chaque permission reçoit un libellé français dans le catalogue `fr-ML`, sous une clé égale à son code (`core:users:create` → « Créer des utilisateurs »). L'écran les regroupe par domaine : Administration, Élèves, Inscriptions, Notes, Bulletins, Facturation, Encaissements, Caisse, Dépenses, Personnel, Paie, Année scolaire, Assiduité, Discipline, Documents, Notifications, Imports et exports, Licence. Un test exige un libellé pour toute permission du catalogue, comme pour les paramètres.

### 5.2 Composition par défaut des rôles livrés

Composition retenue le 7 octobre 2026 (Q-44). Les permissions des modules futurs sont attribuées dès maintenant : la matrice est complète et testée une fois pour toutes, et un module livré plus tard trouve ses rôles déjà prêts.

| Rôle | Permissions |
|---|---|
| **Administrateur** | Toutes (système, ni modifiable ni supprimable) |
| **Directeur** | `academic_year:manage` ; `core:users:read`, `core:audit:read`, `core:backup:create`, `licence:read` ; `students:read` ; `enrollments:validate`, `enrollments:expel` ; `grades:validate`, `grades:publish`, `grades:unlock` ; `reports:generate`, `reports:publish` ; `periods:lock`, `periods:unlock` ; `expenses:approve` ; `employees:read`, `leave:approve`, `payroll:validate` ; `sanctions:create`, `sanctions:update`, `sanctions:cancel` ; `convocations:create`, `convocations:close` ; `notifications:send_manual` ; `academic_year:close` |
| **Directeur des études** | `students:read` ; `grades:create`, `grades:update`, `grades:validate` ; `reports:generate` ; `periods:lock` ; `attendance:create`, `attendance:update`, `attendance:justify` ; `sanctions:create` ; `convocations:create`, `convocations:update` ; `import:grades` |
| **Secrétaire** | `students:create`, `students:read`, `students:update`, `students:merge` ; `enrollments:create`, `enrollments:transfer`, `enrollments:cancel` ; `attendance:create`, `attendance:justify` ; `convocations:create`, `convocations:update`, `convocations:close` ; `documents:deliver`, `documents:reprint` ; `reports:generate` ; `import:students` |
| **Caissier** | `students:read` ; `invoices:issue` ; `payments:create` ; `cashbox:open`, `cashbox:close` ; `documents:reprint` |
| **Comptable** | `invoices:issue`, `invoices:cancel`, `invoices:discount` ; `payments:cancel`, `payments:refund` ; `cashbox:reconcile` ; `expenses:create`, `expenses:pay` ; `import:statements` ; `core:audit:read` |
| **Enseignant** | `students:read` ; `grades:create`, `grades:update` (portée : ses classes et matières, phase 2) ; `attendance:create` |
| **Surveillant** | `students:read` ; `attendance:create`, `attendance:update`, `attendance:justify` ; `sanctions:create`, `sanctions:update` ; `convocations:create`, `convocations:update` |
| **Gestionnaire RH** | `employees:read`, `employees:update`, `contracts:manage` ; `leave:request`, `leave:approve` ; `payroll:calculate`, `payroll:pay` ; `import:staff` |
| **Consultation seule** | `students:read`, `employees:read`, `core:audit:read`, `licence:read` |

*Secrétaire et notes* : `grades:create` n'est pas attribué tant que Q-39 n'est pas tranchée (phase 4). Si sa recommandation est retenue, il s'ajoute par une montée de composition (§5.6).

*Contrainte permanente* (§11.5) : aucun rôle livré hors Administrateur ne cumule `grades:*` et `payments:*`. Elle est déjà testée et le reste.

*Constat* : le catalogue n'a pas de permission de lecture pour les notes, les factures ni les encaissements. La « Consultation seule » ne voit donc rien de la scolarité ni des finances. Les permissions de lecture s'ajoutent avec les modules (phases 3 et 4), et la matrice avec elles.

### 5.3 Rôles personnalisés

- **Créer** un rôle vide, ou copier un rôle existant comme point de départ.
- **Modifier** son libellé et ses permissions, cochées par domaine.
- **Supprimer** un rôle personnalisé (suppression logique) seulement si aucun utilisateur actif ne le porte.
- Un rôle livré autre qu'Administrateur se modifie (§11.4 du blueprint) mais ne se supprime pas. Un bouton rétablit sa composition par défaut.
- Un rôle qui cumulerait `grades:*` et `payments:*` est refusé (Q-43). Une école où une seule personne fait tout lui attribue deux rôles : le cumul reste nominatif et audité.
- **On n'accorde que ce que l'on détient** — vaut aussi pour l'attribution d'un rôle à un utilisateur (§4.1) : une création, une modification ou un rétablissement qui ajoute une permission que l'auteur ne détient pas lui-même est refusé. Sans cette règle, un utilisateur qui compose les rôles sans être administrateur pourrait s'en fabriquer un plus puissant que le sien.
- Le code d'un rôle personnalisé dérive de son libellé (`perso_agent_d_accueil`) et n'est jamais réemployé, même après suppression : un rôle recréé sous le même nom reçoit un suffixe. La suppression retire aussi les attributions restantes des comptes inactifs.

Toute modification de rôle ou de permission est auditée avec la liste avant et après (§12.3 du blueprint) et émet `RoleRecompose`.

### 5.4 Commandes

| Commande | Permission |
|---|---|
| `roles_lister`, `permissions_catalogue` | `core:users:read` |
| `role_creer`, `role_modifier`, `role_supprimer`, `role_retablir` | `core:roles:assign` |
| `role_retirer` (d'un utilisateur) | `core:roles:assign` |

### 5.5 Matrice générée

`docs/matrice-permissions.md` liste chaque rôle livré et ses permissions. Un test la régénère et échoue si le fichier versionné diffère, à la manière des types TypeScript (R-37). C'est la « matrice exhaustive et générée » du §21.1.

### 5.6 Montée de composition des bases installées

Les bases créées en phase 0.5 portent neuf rôles livrés vides, et un Administrateur sans les permissions apparues depuis (`academic_year:manage`). La colonne `role.composition_livree` (migration 0004) note la version de composition déjà appliquée à chaque rôle livré ; la version courante est une constante du noyau.

À l'ouverture de la base, après les migrations, tout rôle livré en retard **reçoit les permissions de sa composition qui lui manquent**, puis sa version est mise à jour. La montée est additive : elle ne retire jamais une permission, ce qui respecte les rôles livrés que l'administrateur a modifiés. Chaque rôle complété est audité (avant et après) et émet `RoleRecompose`, avec le système pour auteur. La sauvegarde qui précède toute migration de schéma (R-48) la précède aussi.

Une phase qui ajoute des permissions à la composition incrémente la version : les bases installées les reçoivent à leur ouverture suivante.

---

## 6. Politique de mot de passe

Paramètres de l'établissement, modifiables dans l'écran Établissement :

| Paramètre | Défaut | Règle |
|---|---|---|
| `mot_de_passe.longueur_min` | 8 | Existe déjà ; borne basse 8, non modifiable en dessous |
| `mot_de_passe.refuser_identifiant` | vrai | Le mot de passe ne contient pas l'identifiant |
| `mot_de_passe.refuser_courants` | vrai | Refus des mots de passe d'une liste embarquée de mots de passe courants (mille entrées) |
| `connexion.echecs_avant_verrouillage` | 3 | Seuil du verrouillage progressif (30 s, puis doublé à chaque échec, une heure au plus) ; borné entre 3 et 10 |

La longueur minimale est bornée entre 8 et 64 ; une valeur hors bornes est refusée à l'enregistrement. La liste des mots de passe courants compte environ 130 entrées de huit caractères au moins (suites, mots de passe génériques, mots du contexte scolaire malien), et non mille : faute de source vérifiée à embarquer, elle a été écrite à la main et s'allongera avec l'usage. Les bases installées reçoivent les nouveaux paramètres à l'ouverture, sans écrasement des valeurs choisies.

Pas d'expiration périodique du mot de passe : sur un poste unique, elle pousse à l'écrire sur un papier collé à l'écran.

---

## 7. Session rapide — finitions (RA-13)

La mécanique de la 0.5 reste ; trois manques constatés à l'usage le 7 octobre 2026 :

1. **Invitation au PIN.** Après le changement de mot de passe exigé, l'écran propose de définir un PIN. On peut refuser, mais un avertissement personnel reste affiché tant qu'aucun PIN n'existe.
2. **Écran verrouillé sans PIN.** Pour un utilisateur sans PIN, l'écran verrouillé demande directement le mot de passe, au lieu du message « Aucun code PIN n'est défini ».
3. **Mesure des 3 secondes.** Le scénario de bout en bout chronomètre la bascule entre deux utilisateurs, de l'écran verrouillé à l'écran de travail (assertion 6).
4. **Essais de PIN limités** (ajout du 7 octobre 2026). La phase 0.5 acceptait un nombre illimité de PIN faux, ce qui rend un PIN de quatre chiffres devinable. Au troisième PIN faux sur une session verrouillée, la session est fermée et seul le mot de passe permet de se reconnecter. Un mot de passe faux à l'écran verrouillé compte comme un échec de connexion (verrouillage progressif du compte).

---

## 8. Journal d'audit

- **Filtres** : utilisateur, entité, opération, période (du … au …). Pagination par 200 lignes.
- **Lisibilité** : le nom affiché de l'utilisateur au lieu de son identifiant technique ; les entités et opérations par leur libellé français.
- **Détail** : un clic sur une ligne affiche les valeurs avant et après, champ par champ.
- **Export** : le CSV existant suit les filtres.
- La vérification de la chaîne reste disponible et intacte.

Commande `audit_lister` étendue (filtres, page) ; permission inchangée, `core:audit:read`.

---

## 9. Navigation

Conforme au §19.2 du blueprint, limitée aux sections qui existent :

```
Tableau de bord · Mon compte
Paramètres  → Établissement · Année scolaire · Utilisateurs · Rôles
Outils      → Sauvegarde · Journal d'audit · Licence
```

Chaque section reste masquée si la commande qui la nourrit est refusée ; la garde Rust reste seule juge (R-21).

---

## 10. Essais de bout en bout et captures

Le scénario Playwright de la 0.5 (`frontend/e2e/`) s'étend à chaque étape de la phase. Chaque assertion du §0.4 qui touche un écran a son test ; chaque nouvel écran a sa capture dans `frontend/e2e/captures/`, montrée au porteur du projet à la fin de l'étape.

Scénarios à ajouter :
- un administrateur crée une secrétaire, lui attribue son rôle ; la secrétaire se connecte et ne voit que ses sections ;
- bascule chronométrée entre les deux par l'écran verrouillé ;
- création et activation de l'année 2026-2027, visible dans le bandeau ;
- modification de la fiche et du logo, puis document d'essai qui les porte ;
- création d'un rôle personnalisé, refus du cumul notes et caisse ;
- désactivation d'un compte pendant que sa session est ouverte.

---

## 11. Événements de la phase

Faits accomplis au participe passé singulier (spec 0.5 §3.2), versionnés en v1, vérifiés par le test du noyau :

| Événement | Fait | Charge utile |
|---|---|---|
| `FicheEtablissementRectifiee` | La fiche de l'établissement a changé | Champs changés, avant et après |
| `LogoEtablissementRemplace` | Un nouveau logo est en place | Empreinte de la pièce jointe |
| `AnneeScolairePreparee` | Une année est créée en préparation | Libellé, bornes |
| `AnneeScolaireOuverte` | Une année devient active et courante | Libellé |
| `RoleCree` | Un rôle personnalisé existe | Code, libellé, permissions |
| `RoleSupprime` | Un rôle personnalisé est supprimé | Code, libellé |
| `RoleRecompose` | Les permissions d'un rôle ont changé | Permissions ajoutées et retirées |
| `RoleRetire` | Un rôle est retiré à un utilisateur | Utilisateur, rôle |
| `CompteDesactive` / `CompteReactive` | Un compte est désactivé ou réactivé | Utilisateur, sessions fermées |
| `MotDePasseReinitialise` | Un administrateur a réinitialisé un mot de passe | Utilisateur, auteur ; jamais le mot de passe (R-25) |

---

## 12. Tests de la phase

| Cible | Nature |
|---|---|
| Transitions d'année scolaire | Toutes, autorisées et interdites, générées (§21.1) |
| Matrice rôle × permission | Générée, comparée au fichier versionné |
| Garde | Pour chaque rôle livré, chaque commande exposée : autorisée ou refusée selon la matrice |
| Garde-fous administrateur | Dernier administrateur : désactivation, retrait de rôle, auto-modification |
| Politique de mot de passe | Chaque règle, aux trois points d'entrée (création, changement, réinitialisation) |
| Audit | Chaque mutation de la phase écrit audit et `outbox` dans la même transaction (assertion 5 de la 0.5) |
| i18n | Libellé pour chaque permission et chaque nouveau paramètre |
| Bout en bout | Scénarios du §10, avec captures |

---

## 13. Ordre de réalisation

Chaque étape est livrable et testable avant la suivante, avec ses captures.

1. Libellés des permissions, composition des rôles livrés, matrice générée
2. Écran Rôles : rôles personnalisés, édition des permissions, rétablissement
3. Cycle de vie des comptes : désactivation, réinitialisation, déverrouillage, retrait de rôle, garde-fous
4. Politique de mot de passe
5. Session rapide : invitation au PIN, écran verrouillé sans PIN, bascule chronométrée
6. Fiche d'établissement et logo dans les documents
7. Année scolaire : création, activation, bandeau
8. Journal d'audit lisible et filtrable
9. Navigation finale et scénario de bout en bout complet

Le temps réel de chaque étape est noté dans `ETAT_AVANCEMENT.md`, comme pour la 0.5 (Q-40).

---

## 14. Questions à trancher

| Id | Question | Recommandation | Bloque |
|---|---|---|---|
| **Q-42** | Quelle permission pour créer et activer une année scolaire ? Le catalogue n'a que `academic_year:close` et `academic_year:reopen` | **Ajouter `academic_year:manage`** (créer, modifier, activer, supprimer une année en préparation), attribuée à l'Administrateur et au Directeur. Alternative écartée : `core:settings:update`, trop large pour un acte qui ouvre une année entière | **Tranchée le 7 octobre 2026 : recommandation retenue** |
| **Q-43** | Un rôle personnalisé peut-il cumuler `grades:*` et `payments:*` ? | **Non, refusé comme pour les rôles livrés.** Une petite école où une seule personne fait tout lui attribue deux rôles ; le cumul reste alors visible, nominatif et audité, au lieu d'être caché dans un rôle | **Tranchée le 7 octobre 2026 : recommandation retenue** |
| **Q-44** | La composition par défaut du §5.2 convient-elle ? | Retenue telle que proposée ; `grades:create` du rôle Secrétaire attend Q-39 (phase 4) | **Tranchée le 7 octobre 2026 : composition retenue** |

---

## 15. Journal des révisions

| Version | Date | Modification |
|---|---|---|
| v0.1 | 2026-10-07 | Création, à valider |
| v1.4 | 2026-10-07 | Étapes 7 à 9 livrées : sauvegarde avant ouverture d'une année (`MotifSauvegarde::AvantOuvertureAnnee`) ; journal paginé avec auteurs et entités (§8) ; ordre de la barre latérale (§9). Les dix assertions sont démontrées (`ETAT_AVANCEMENT.md`) |
| v1.3 | 2026-10-07 | Étape 6 : logo gardé dans la base plutôt que comme pièce jointe sur disque (§2.2) |
| v1.2 | 2026-10-07 | Étapes 3 à 5 : attribution soumise à « on n'accorde que ce que l'on détient » (§5.3) ; bornes et liste des mots de passe courants (§6) ; essais de PIN limités (§7) |
| v1.1 | 2026-10-07 | Étape 2 : règle « on n'accorde que ce que l'on détient », codes des rôles personnalisés, événement `RoleSupprime` (§5.3, §11) |
| v1.0 | 2026-10-07 | Q-42, Q-43 et Q-44 tranchées selon les recommandations ; libellés de permission sous la clé du code (§5.1) ; `grades:create` du Secrétaire suspendu à Q-39 (§5.2) ; montée de composition des bases installées (§5.6) |
