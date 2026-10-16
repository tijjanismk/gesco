# SPEC_PHASE_2.md — Élèves, classes, inscriptions

**Projet** : ERP de gestion scolaire — Tauri 2 + Rust + React/TypeScript
**Phase** : 2 — référentiel des niveaux et des classes, élèves, responsables, inscriptions, import
**Statut** : v1.3 — spécification exécutable, engagée le 7 octobre 2026
**Documents parents** : `PROJECT_BLUEPRINT.md` v0.20 (§2.3 UC-01, §3.5, §3.6, §8.2, §8.3, §10.1, §14, §19.2, §19.3, §24.4, §24.7, §25.5, §25.6, §25.9), `ARCHITECTURE_RULES.md` v0.3, `SPEC_PHASE_1.md` v1.4, `docs/questions-terrain.md` (T-01 à T-10)
**Date** : 7 octobre 2026

---

## 0. Objet et critère de fin

### 0.1 Ce que produit cette phase

Le livrable du blueprint (§22.2) : **UC-01, inscrire un élève de bout en bout**, sans sa partie financière. À la fin de la phase, une secrétaire inscrit un nouvel élève devant son parent : identité, responsables, classe, validation, fiche d'inscription imprimée. Une école importe ses élèves depuis son fichier Excel au lieu de les ressaisir.

La tarification, l'échéancier, le premier versement et le reçu d'UC-01 arrivent avec la phase 3 (§0.3).

### 0.2 Entrée en phase 2 malgré la phase 0.5 non validée

`SPEC_PHASE_0_5.md` §0.2 interdit d'entrer en phase 2 tant qu'une assertion de la phase 0.5 est fausse. Deux le sont : la base n'est pas chiffrée (SQLCipher reporté le 7 octobre 2026) et l'essai sur machine vierge reste à faire. Le 7 octobre 2026, le porteur du projet a validé la phase 1 et demandé de commencer la phase 2. Cette décision lève l'interdiction pour le développement, pas pour la livraison : **aucune école ne reçoit GESCO avec des données d'élèves tant que la base n'est pas chiffrée et que l'essai sur machine vierge n'a pas réussi.** Ce sont des données de mineurs (§3.12, R-24).

**Décision du porteur du projet, 10 octobre 2026** : pour les phases restantes, la base reçoit un chiffrement simple, et l'essai sur une machine Windows 10 vierge est abandonné. La condition de livraison devient : aucune école ne reçoit GESCO avec des données d'élèves tant que la base n'est pas chiffrée. **Chiffrement actif le 10 octobre 2026** : SQLite3 Multiple Ciphers remplace SQLite, la base naît chiffrée avec la clé de données, une base en clair se chiffre à sa première ouverture (`docs/adr/2026-10-10-chiffrement-de-la-base.md`).

Q-40 (durée des phases) reste ouverte ; le temps réel de chaque étape est noté dans `ETAT_AVANCEMENT.md`.

### 0.3 Inscription validée sans facture jusqu'à la phase 3

R-17 veut qu'une inscription validée et sa facture naissent dans la même transaction. La facturation n'existe pas en phase 2. Décision du porteur du projet du 7 octobre 2026 : en phase 2, la validation ne crée pas de facture. **En phase 3, la facture rejoint la transaction de validation**, et les inscriptions validées auparavant reçoivent leur facture par une montée des bases installées, sur le modèle de la montée de composition des rôles (`SPEC_PHASE_1.md` §5.6). Aucune école réelle n'utilise GESCO avant la phase 3 (§22.4 : pilote à la fin de la phase 3).

### 0.4 Critère de fin, vérifiable

La phase est terminée quand ces douze assertions sont vraies, chacune démontrée par un test automatisé ou par le scénario de bout en bout (§12) :

1. Un établissement neuf ou une base installée porte les quinze niveaux livrés (jardin, 1er cycle, 2e cycle, lycée) et les huit séries du lycée ; seuls les niveaux des cycles que l'école enseigne sont actifs ; l'école renomme un niveau ou le désactive, jamais n'en supprime.
2. Une classe se crée dans l'année courante, rattachée à un niveau et, au lycée en 11e et 12e, à une série ; une série est refusée hors de ces niveaux, et exigée à ces niveaux.
3. Une fiche élève se crée avec un matricule interne généré selon les règles de l'école, sans trou, ou saisi par l'école et unique ; deux élèves de même nom, prénoms et date de naissance coexistent.
4. Un responsable déjà saisi se retrouve par son téléphone ou son nom et se rattache à un autre élève sans ressaisie ; ses qualités (responsable légal, payeur, reçoit les SMS, vient chercher l'enfant) sont propres à chaque lien.
5. Les téléphones sont enregistrés sous forme canonique E.164 ; un numéro malien saisi sous trois formes différentes donne la même valeur.
6. La recherche rapproche Traoré et Traore, Keïta et Keita, Coulibaly et Kouloubaly, Mohamed et Mahamadou ; elle propose les doublons probables à la création d'une fiche et ne fusionne jamais rien d'elle-même.
7. Une inscription passe de brouillon à validée seulement si l'identité est complète, qu'un responsable légal joignable existe, que le matricule national est saisi en 9e et au lycée, et que la photo existe au jardin ; chaque condition manquante est refusée par un test.
8. Un élève n'a qu'une inscription vivante (brouillon ou validée) par année ; un changement de classe en cours d'année reste dans le même niveau et la même série, sauf décision motivée de la direction (Q-35), et l'historique le garde.
9. La fiche d'inscription imprimée porte l'en-tête de l'école, l'année, la classe, l'identité, le matricule, les responsables si le paramètre de l'école le veut, les signatures et un QR code de vérification.
10. Deux fiches en doublon se fusionnent : inscriptions et responsables passent sur la fiche conservée, l'autre reste lisible et désignée comme absorbée ; la fusion est auditée avant et après.
11. Un fichier Excel d'élèves s'importe en quatre temps (correspondance des colonnes, validation ligne par ligne, aperçu avec doublons probables et erreurs, confirmation) ; une sauvegarde précède l'application ; seules les lignes valides peuvent être appliquées.
12. Chaque liste de la phase s'exporte en Excel et en CSV (R-41), et le scénario Playwright couvre chaque écran de la phase, captures montrées au porteur du projet.

---

## 1. Périmètre

**Inclus** : niveaux et séries, classes de l'année, personnes, élèves, responsables et leurs liens, matricule interne, téléphones canoniques, recherche tolérante et recherche globale, doublons probables et fusion, inscription (brouillon, validation, abandon, changement de classe), photo, fiche d'inscription imprimée avec QR, import Excel des élèves, export Excel de toute liste.

**Exclu** :
- Matières, coefficients, programmes, règles d'évaluation, périodes : début de la phase 4, où ils servent (décision du 7 octobre 2026).
- Tarification, facture, échéancier, versement, reçu : phase 3 (§0.3).
- Transfert sortant et certificat de transfert (Q-32, Q-33, Q-34) : phase 4, avec le relevé des moyennes qu'il accompagne. Radiation : avec la discipline. Achèvement : clôture d'année, phase 10.
- Réinscription en masse et passage automatique d'un élève d'une année à l'autre (T-09) : clôture d'année, phase 10. La phase 2 pose seulement le paramètre.
- Portée ressource de la garde (§11.3) : elle commence avec les notes (phase 4). La garde garde son implémentation qui accorde tout.
- Code d'accès des responsables (§25.5), joignabilité et consentement des numéros (§25.6) : phase 5, avec le canal SMS.
- Timeline de l'élève (§25.7), écran guichet (§19.5) : phase 5.
- Évaluation par compétences du jardin : reportée (Q-03 révisée).

---

## 2. Niveaux et séries

### 2.1 Niveaux livrés

Le référentiel national est livré, pas saisi. Chaque établissement en reçoit sa copie, qu'il renomme ou désactive (les libellés « restent modifiables par l'école », T-04). On ne crée ni ne supprime de niveau.

| Cycle | Codes | Libellés livrés | Évaluation | Diplôme terminal |
|---|---|---|---|---|
| Jardin d'enfants | PS, MS, GS | Petite section, Moyenne section, Grande section | compétences | aucun |
| 1er cycle | 1 à 6 | 1re année … 6e année | notes | aucun |
| 2e cycle | 7 à 9 | 7e année … 9e année | notes | DEF en 9e |
| Lycée | 10 à 12 | 10e année, 11e année, 12e année | notes | BAC en 12e |

Colonnes : `code`, `libelle`, `cycle`, `ordre` (1 à 15, ordre d'affichage), `mode_evaluation`, `diplome_terminal`, `actif`. Le cycle porte les règles : matricule national au lycée (Q-18), photo et personnes qui viennent chercher l'enfant au jardin (T-02, T-10).

### 2.1 bis Cycles enseignés

Précision du porteur du projet, 7 octobre 2026 : « il y a des écoles qui ont seulement le premier cycle, d'autres le premier et le deuxième cycle, et d'autres aussi le lycée ». L'école coche ses **cycles enseignés** au premier démarrage (au moins un, `referentiel.aucun_cycle`), puis les change d'un geste dans « Niveaux et séries » (`cycles_definir`, `academic_year:manage`) : les niveaux des cycles cochés sont actifs, les autres désactivés, chacun audité. Seuls les niveaux actifs reçoivent des classes, donc des inscriptions. Une base installée avant la phase 2 reçoit tous ses niveaux actifs ; l'école décoche ensuite ce qu'elle n'enseigne pas.

### 2.2 Séries

Les huit filières du §8.2 : en 11e, `S`, `SES`, `LL` ; en 12e, `TSE`, `TSExp`, `TSEco`, `TSS`, `TLL`. La 10e est un tronc commun, sans série. Une série se renomme ou se désactive.

### 2.3 Bases installées

Les niveaux et séries sont posés au premier démarrage. Une base créée avant la phase 2 les reçoit à son ouverture, par `mise_a_niveau::completer_referentiel`, sur le modèle des rôles et des paramètres : ajout seul, audité, auteur Système.

### 2.4 Commandes

| Commande | Permission | Effet |
|---|---|---|
| `niveaux_lister` | connectée | Niveaux et séries de l'établissement |
| `cycles_definir` | `academic_year:manage` | Active les niveaux des cycles enseignés, désactive les autres ; audit |
| `niveau_modifier` | `academic_year:manage` | Libellé, actif ; audit avant/après |
| `serie_modifier` | `academic_year:manage` | Libellé, actif ; audit avant/après |

*Permission* : le catalogue n'a pas de permission pour le référentiel et les classes. `academic_year:manage` (Q-42) les couvre : organiser les niveaux et les classes, c'est préparer l'année. Voir Q-45.

---

## 3. Classes

### 3.1 Modèle

`classe` : `annee_id`, `niveau_id`, `serie_id` (facultative), `libelle`. Une classe appartient à une année (R-16) : la 7e A de 2026-2027 n'est pas celle de 2027-2028.

*Règles* :
- le libellé est libre et unique dans l'année : « 7e A », « 10CG », « 12TSE » (T-04) ;
- la série est exigée en 11e et 12e, refusée ailleurs, et doit appartenir au niveau ;
- un niveau ou une série désactivé ne reçoit pas de nouvelle classe ;
- **aucun effectif maximal** (T-05) : l'écran affiche l'effectif, rien ne bloque ni n'avertit ;
- une classe qui a reçu une inscription ne se supprime pas ; une classe vide se supprime logiquement ;
- une classe se crée dans une année en préparation ou active, jamais dans une année clôturée.

Professeur principal et salle attendent le personnel (phase 6) et l'emploi du temps (phase 8).

### 3.2 Commandes

| Commande | Permission | Effet |
|---|---|---|
| `classes_lister` | connectée | Classes d'une année, avec leur effectif |
| `classe_creer` | `academic_year:manage` | Audit, événement `ClasseCreee` |
| `classe_modifier` | `academic_year:manage` | Libellé ; audit avant/après |
| `classe_supprimer` | `academic_year:manage` | Classe vide seulement ; suppression logique, audit |

---

## 4. Personnes, élèves, responsables

### 4.1 Personne

Socle commun (§8.3) : `nom`, `prenoms`, `sexe` (M, F), `date_naissance`, `date_naissance_estimee`, `lieu_naissance`, `nationalite`, `adresse`, `telephones`, photo. **Aucun index unique** sur nom, prénoms et date de naissance (§3.5).

La forme normalisée du nom (sans accents ni casse) et sa clé phonétique (§6) ne sont pas stockées : la recherche les calcule en mémoire, ce qui permet d'affiner l'algorithme sans migration.

Une date de naissance est une date civile (`AAAA-MM-JJ`), pas un instant : elle ne dépend pas du fuseau. Elle est obligatoire pour un élève (§24.4), facultative pour un responsable.

### 4.2 Élève

`personne_id`, `matricule_interne`, `matricule_national`, `acte_naissance` (numéro, date, nature : acte ou jugement supplétif), `ecole_origine`, `fusionne_dans`.

**Matricule interne** (T-01, révisé le 7 octobre 2026 à la demande du porteur du projet : « manuel ou autogénéré selon des règles, pour plus de flexibilité ») : l'école choisit dans ses réglages.
- **Saisi par l'école** (`matricule.saisie_manuelle`) : la secrétaire tape le numéro de ses registres ; GESCO vérifie seulement qu'il n'est pas vide, tient en 30 caractères et n'appartient à aucun autre élève (fiches absorbées comprises). Une faute de frappe se corrige, auditée.
- **Généré par GESCO** (par défaut) selon les règles de l'école : préfixe (`matricule.prefixe`, lettres et chiffres, dix au plus), année d'entrée sur 4, 2 ou 0 chiffres (`matricule.chiffres_annee`), numéro sur 3 à 8 chiffres (`matricule.chiffres`), numérotation qui repart à 1 chaque année ou continue (`matricule.remise_annuelle` ; sans année dans le matricule, elle continue toujours). Par défaut `2026-00042` ; par exemple `ASK-26-042`. Le numéro est attribué par le service de numérotation dans la transaction de création, sans trou ; un matricule déjà pris (saisi à la main ou produit avant un changement de règles) fait passer au suivant. L'année d'entrée est la première année du libellé de l'année courante (`2026` pour 2026-2027). Un matricule généré ne change jamais, même après une fusion.

**Matricule national** (décision du même jour, « sauf celui du DEF ») : le numéro donné par l'État aux examens, DEF puis BAC, se recopie tel quel et ne se génère jamais. Facultatif à la saisie, il est exigé pour valider une inscription **dès la 9e** (niveau du DEF) et au lycée ; unique dans l'établissement quand il est renseigné (index partiel).

**Champs de la fiche** (T-02) — exigés à la création : nom, prénoms, sexe, date de naissance avec la case « estimée » (jamais dans le futur). Le lieu de naissance, obligatoire selon T-02, est exigé à la **validation de l'inscription** (§5.3) plutôt qu'à la création : un import dont le fichier n'a pas de lieu crée les fiches, la secrétaire le complète avant de valider. Facultatifs : nationalité (Malienne par défaut), acte de naissance ou jugement supplétif, école d'origine, adresse, téléphones, photo. La classe et les responsables appartiennent à l'inscription et aux liens, pas à la fiche.

### 4.3 Responsable et liens

`responsable` : `personne_id`, `profession`, `employeur`. Pour un responsable, seuls nom, prénoms et un téléphone sont requis.

`eleve_responsable` (T-03) : `lien` (père, mère, tuteur, autre), `est_responsable_legal`, `est_payeur`, `recoit_notifications`, `vient_chercher`, `ordre_priorite`. Les qualités sont indépendantes (§3.6) et propres au lien : la mère de deux enfants peut être payeuse pour l'un et pas pour l'autre.

De un à trois responsables par élève en général (T-03), sans plafond technique. Au jardin, deux à trois personnes qui viennent chercher l'enfant (T-02) : voir §7.2 et Q-46.

**Réutilisation** (§25.5) : à l'ajout d'un responsable, la saisie d'un téléphone ou d'un nom propose les responsables existants ; en choisir un crée le lien sans nouvelle personne. Un responsable n'est jamais dupliqué par la fratrie.

### 4.4 Téléphones

Stockés en E.164 (§25.6), saisis sous n'importe quelle forme :
- huit chiffres → numéro malien, `+223XXXXXXXX` ;
- `00223…` ou `+223…` suivi de huit chiffres → même forme ;
- `+` suivi de 8 à 15 chiffres → numéro étranger conservé ;
- tout autre saisie est refusée avec le message `telephone.invalide`.

Espaces, points et tirets sont ignorés. L'affichage regroupe par paires (`+223 76 12 34 56`).

### 4.5 Photo

Facultative, sauf au jardin où elle est exigée pour valider (T-10). PNG ou JPEG de 100 Ko au plus, reconnue à ses premiers octets comme le logo (`SPEC_PHASE_1.md` §2.2), gardée dans la base : elle voyage avec la sauvegarde. Le frontend la réduit avant l'envoi (cadrage, 480 px de côté au plus, JPEG) ; le noyau revérifie format et taille (R-02). Prise à la webcam ou importée d'un fichier.

### 4.6 Commandes

| Commande | Permission | Effet |
|---|---|---|
| `eleves_rechercher` | `students:read` | Recherche tolérante (§6), page de résultats |
| `eleve_lire` | `students:read` | Fiche, responsables, inscriptions |
| `eleve_creer` | `students:create` | Matricule attribué ; audit ; `FicheEleveCreee` |
| `eleve_modifier` | `students:update` | Audit avant/après ; `FicheEleveRectifiee` |
| `eleve_photo_definir` | `students:update` | Photo ou retrait ; audit de l'empreinte et de la taille |
| `eleve_doublons_probables` | `students:read` | Fiches proches d'une identité en cours de saisie |
| `eleves_fusionner` | `students:merge` | §8 |
| `responsables_rechercher` | `students:read` | Par téléphone ou nom |
| `responsable_rattacher` | `students:update` | Nouveau ou existant ; audit ; `ResponsableRattache` |
| `responsable_modifier` | `students:update` | Personne et qualités du lien ; audit avant/après |
| `responsable_detacher` | `students:update` | Lien retiré ; audit ; `ResponsableDetache` |

---

## 5. Inscriptions

### 5.1 Modèle

`inscription` (§8.3, §24.7) : `annee_id`, `eleve_id`, `classe_id`, `date_inscription`, `type` (nouvelle, réinscription, transfert entrant), `statut`, `redoublant`, `decision_fin_annee` (non statué), `classe_precedente_id`, `date_arrivee`, `etablissement_origine`, `motif_abandon`, et les colonnes de départ de §24.7 (`date_depart`, `etablissement_destination`, `motif_depart`), posées dès maintenant et remplies en phase 4.

L'inscription est la table pivot (§8.3) : les modules suivants référencent `inscription_id`, jamais `eleve_id`. **Jamais de `DELETE`** (R-14).

Le type est déduit, pas saisi : réinscription si l'élève a une inscription validée d'une année antérieure, transfert entrant si une école d'origine est indiquée en cours d'année, nouvelle sinon.

### 5.2 Transitions de la phase 2

```
BROUILLON ──valider──► VALIDÉE ──abandonner──► ABANDONNÉE
    └────────abandonner──────────────────────────┘
```

| Depuis | Action | Vers | Permission | Condition |
|---|---|---|---|---|
| — | créer | Brouillon | `enrollments:create` | Année courante active ; aucune autre inscription vivante de l'élève dans l'année |
| Brouillon | modifier | Brouillon | `enrollments:create` | Classe, date, redoublant, provenance |
| Brouillon | valider | Validée | `enrollments:validate` | §5.3 |
| Brouillon | abandonner | Abandonnée | `enrollments:cancel` | Motif obligatoire |
| Validée | changer de classe | Validée | `enrollments:transfer` | §5.4 |
| Validée | abandonner | Abandonnée | `enrollments:cancel` | Motif obligatoire |
| Abandonnée | toute action | — | — | Refusée : une nouvelle inscription se crée |

`BROUILLON → ABANDONNÉE` n'est pas au §10.1 du blueprint ; il évite de supprimer un brouillon (R-14). Le blueprint est amendé (R-51). Transférée, radiée et achevée sont dans la table des transitions du noyau dès maintenant, sans commande (§1).

Une inscription se crée et se valide en une seule action à l'écran quand l'utilisateur détient les deux permissions : c'est le parcours de la secrétaire devant le parent (UC-01). L'écran enchaîne les deux commandes, chacune gardée par sa permission ; si la validation est refusée, le brouillon reste et le motif du refus s'affiche.

### 5.3 Conditions de validation

Vérifiées par le noyau (R-30), chacune avec sa clé de message :
1. identité complète : nom, prénoms, sexe, date de naissance, lieu de naissance (`inscription.identite_incomplete`) ;
2. au moins un responsable légal (`inscription.responsable_legal_absent`, §10.1) ;
3. au moins un responsable avec un téléphone (`inscription.telephone_absent`, T-02) ;
4. en 9e (niveau du DEF) et au lycée, matricule national renseigné (`inscription.matricule_national_absent`, Q-18 révisée) ;
5. au jardin, photo présente (`inscription.photo_absente`, T-10) et au moins une personne qui vient chercher l'enfant (`inscription.personne_autorisee_absente`, T-02) ;
6. classe de l'année de l'inscription, niveau et série actifs.

*Avertissements, sans blocage* : moins de deux personnes qui viennent chercher un enfant du jardin (Q-46) ; inscription après la date d'alerte de l'école (Q-31, paramètre `inscription.alerte_apres_jours`, 0 par défaut : jamais) ; doublon probable de la fiche (§6.3).

« Classe non pleine » (§10.1) disparaît : aucune classe n'a d'effectif maximal (T-05). Le blueprint est amendé.

### 5.4 Changement de classe

En cours d'année, une inscription validée change de classe dans le **même niveau et la même série** (Q-35). Un changement de niveau ou de série exige `enrollments:validate` en plus et un motif écrit : c'est une décision de direction, tracée. L'historique est dans le journal d'audit (avant, après, motif) et dans `classe_precedente_id`. Événement `ClasseChangee`.

### 5.5 Commandes

| Commande | Permission | Effet |
|---|---|---|
| `inscriptions_lister` | `students:read` | Par année et classe, avec filtres de statut |
| `inscription_creer` | `enrollments:create` | Brouillon ; audit ; `InscriptionPreparee` |
| `inscription_modifier` | `enrollments:create` | Brouillon seulement ; audit avant/après |
| `inscription_valider` | `enrollments:validate` | Conditions §5.3 ; audit ; `InscriptionValidee` |
| `inscription_abandonner` | `enrollments:cancel` | Motif ; audit ; `InscriptionAbandonnee` |
| `inscription_changer_classe` | `enrollments:transfer` | §5.4 ; audit ; `ClasseChangee` |
| `inscription_fiche_produire` | `students:read` | §9 |

---

## 6. Recherche

### 6.1 Normalisation

Pour chercher, chaque nom complet est réduit en Rust (R-01) : majuscules, accents retirés (`Keïta` → `KEITA`, `Traoré` → `TRAORE`), ponctuation et espaces multiples ramenés à un espace.

### 6.2 Proximité

Un nom candidat est retenu si, mot à mot :
- sa forme normalisée est identique ou commence par le mot cherché ;
- ou sa **clé phonétique** est identique : `C`, `K` et `Q` se confondent, `OU` et `U`, `PH` et `F`, `DJ` et `DI` devant une voyelle et `J` (Diallo, Jallo), `CH` et `S`, `Y` et `I`, `H` muet, lettres doublées simplifiées, `E` final retiré ;
- ou la **distance d'édition** entre les deux clés reste sous un seuil fixé par la plus courte (aucune faute jusqu'à 3 lettres, 1 jusqu'à 5, 2 au-delà) : Coulibaly et Kouloubaly ;
- ou les deux prénoms appartiennent à une même classe d'**équivalences** (Mohamed, Mahamadou, Mahamane, Mamadou ; Fatoumata, Fatimata, Fatouma ; Oumar, Omar…), livrée avec le produit.

Les résultats sont classés : identique, puis préfixe, puis équivalence et phonétique, puis distance d'édition. La recherche couvre nom, prénoms et matricules ; un matricule saisi en entier trouve sa fiche directement.

Le calcul se fait en mémoire sur les personnes de l'établissement : 2 000 élèves et leurs responsables se parcourent en quelques millisecondes. Une recherche répond en moins de 200 ms sur 2 000 noms en compilation optimisée, celle du poste de l'école (mesuré par un test).

*Équivalences complétées par l'école* (T-07) : la liste livrée couvre les variantes courantes. Une table et un écran pour l'enrichir s'ajouteront quand l'usage le demandera.

### 6.3 Doublons probables

À la création d'une fiche, à l'import et sur demande : deux fiches sont des doublons probables si leurs noms sont proches au sens du §6.2 **et** leurs dates de naissance identiques ou distantes de moins d'un an (dates estimées comprises). La liste est proposée ; rien n'est fusionné sans un geste humain (T-07).

### 6.4 Recherche globale

Un champ dans le bandeau, ouvert par `Ctrl+K` (§19.3, R-42) : élèves et responsables en phase 2, puis reçus et factures en phase 3. Entrée ouvre la fiche, flèches pour choisir, Échap pour fermer : le parcours se fait au clavier seul.

---

## 7. Règles du jardin d'enfants

Q-03 révisée le 7 octobre 2026 : le jardin s'inscrit dès la phase 2, sans évaluation.

### 7.1 Ce qui change au jardin

- photo exigée à la validation (T-10) ;
- case « vient chercher l'enfant » sur chaque lien de responsable ;
- pas de matricule national, pas de série ;
- `mode_evaluation = competences` : aucun bulletin avant la phase 11.

### 7.2 Personnes qui viennent chercher l'enfant

T-02 : « deux à trois responsables pour le jardin, ceux qui viennent le récupérer ». Une validation sans aucune personne qui vient chercher l'enfant est refusée ; avec une seule, l'écran avertit sans bloquer. Voir Q-46.

---

## 8. Fusion de doublons

Deux fiches élèves désignées par l'utilisateur (`students:merge`) : l'une est **conservée**, l'autre **absorbée**.

- Les inscriptions de l'absorbée passent sur la conservée. Si les deux ont une inscription vivante la même année, la fusion est refusée (`fusion.inscriptions_concurrentes`) : il faut d'abord abandonner l'une des deux.
- Les liens de responsables de l'absorbée s'ajoutent à la conservée ; un responsable commun garde les qualités de la conservée.
- Les champs vides de la conservée reçoivent ceux de l'absorbée ; les champs remplis des deux côtés gardent la valeur de la conservée, l'écran montre les deux avant de confirmer.
- L'absorbée n'est pas supprimée : `fusionne_dans` désigne la conservée, elle sort des recherches, son matricule reste réservé et redirige vers la conservée.
- Audit des deux fiches avant et après, événement `EleveFusionne`.

---

## 9. Fiche d'inscription imprimée

Modèle Typst `fiche_inscription` (T-08) : en-tête commun avec le logo, année, classe, identité et matricule, photo si elle existe, responsables (paramètre `fiche_inscription.afficher_responsables`, vrai par défaut), date, signatures du parent et du secrétariat, QR code de vérification.

**QR code** (§25.9) : il encode la référence du document (`GESCO:DOC:<id>`). Le moteur documentaire le compose en SVG (`qrcode`, sans la pile d'images) et le joint à la composition ; le webview n'en produit pas (R-44). Seuls les modèles qui déclarent la variable `qr` le reçoivent, avec `reference` : le moteur refuse toute variable qu'un modèle ne connaît pas. Le même mécanisme servira aux reçus de la phase 3.

La fiche ne s'imprime que pour une inscription validée (`inscription.fiche_non_validee`).

Les frais de l'année y entreront en phase 3. La fiche se produit à la validation et se réimprime ; chaque production est un `Document` avec empreinte et version de modèle (R-45).

---

## 10. Import des élèves

### 10.1 Parcours

Assistant en quatre temps (§14.1, R-47) :

1. **Dépôt** d'un fichier `.xlsx` ou `.csv`. Lu en Rust (`calamine`, ajouté à la liste blanche).
2. **Correspondance des colonnes** : reconnues par leur titre (« Nom », « Prénom(s) », « Date de naissance », « Né(e) le », « Sexe », « Classe », « Téléphone du parent »…), proposées à l'écran, corrigeables. Deux dispositions acceptées (T-06) : une feuille par classe (le nom de la feuille désigne la classe) ou une seule feuille avec une colonne « classe ».
3. **Validation ligne par ligne**, puis **aperçu** : lignes valides, lignes en avertissement (doublon probable dans le fichier ou dans la base, date estimée), lignes en erreur avec leur motif. Une colonne qui réunit nom et prénoms est découpée selon une règle affichée (le premier mot est le nom, ou le dernier), à valider à l'écran.
4. **Confirmation** : une sauvegarde automatique précède l'application (R-48), puis les lignes retenues s'appliquent en une transaction. Chaque élève importé reçoit son matricule, son responsable s'il est fourni, et une inscription **en brouillon** dans la classe indiquée ; la validation se fait ensuite, classe par classe, quand les conditions du §5.3 sont réunies.

Les classes absentes de l'année sont signalées, jamais créées par l'import.

### 10.2 Données

Rien n'est lu en JavaScript : le dialogue du système rend un chemin, que le noyau lit (révision du 9 octobre 2026 : le base64 initialement prévu doublait la taille en mémoire pour rien ; même mécanisme que la restauration). Le fichier lu reste en mémoire le temps de l'assistant, pour son déposant seul. Taille maximale 10 Mo, 5 000 lignes.

### 10.3 Commandes

| Commande | Permission | Effet |
|---|---|---|
| `import_eleves_analyser` | `import:students` | Feuilles, colonnes, correspondance proposée |
| `import_eleves_valider` | `import:students` | Aperçu ligne par ligne avec la correspondance choisie |
| `import_eleves_appliquer` | `import:students` | Sauvegarde, puis transaction ; audit ; `ImportElevesApplique` |

---

## 11. Export Excel

La dérogation à R-41 (export CSV seul) devait se résorber « avant le premier écran métier ». C'est ici. Une commande générique `exporter_xlsx` reçoit le titre, les en-têtes et les lignes déjà affichés et rend un classeur produit en Rust (`rust_xlsxwriter`, ajouté à la liste blanche). Le composant `Tableau` propose « Excel » à côté de « CSV » sur toutes les listes de l'application. La dérogation est close.

Permission : connectée. La commande ne lit rien en base ; elle met en forme ce que l'écran montre déjà à l'utilisateur.

---

## 12. Essais de bout en bout et captures

Le scénario Playwright s'étend à chaque étape. Scénarios à ajouter :
- l'administrateur renomme un niveau et crée les classes de l'année ;
- **UC-01 chronométré** : la secrétaire inscrit un nouvel élève du lycée avec deux responsables et imprime la fiche, en moins de 5 minutes au clavier ;
- inscription au jardin refusée sans photo, puis acceptée ;
- recherche de « Kouloubaly » qui trouve « Coulibaly » ; recherche globale au clavier ;
- un responsable existant rattaché à un frère ;
- doublon proposé à la création, puis fusion ;
- import d'un fichier de quinze élèves avec deux erreurs et un doublon ;
- export Excel d'une liste.

---

## 13. Modèle de données

Migrations, dans l'ordre des étapes :

| Migration | Tables |
|---|---|
| 0006 référentiel | `niveau`, `serie`, `classe` |
| 0007 personnes | `personne` (photo comprise), `eleve`, `responsable`, `eleve_responsable` |
| 0008 inscriptions | `inscription` |

Toutes portent `etablissement_id` et les colonnes de traçabilité (R-12, R-13), des ULID (R-10). Les photos sont des colonnes BLOB exclues des requêtes de liste.

---

## 14. Événements de la phase

| Événement | Fait | Charge utile |
|---|---|---|
| `ClasseCreee` | Une classe existe dans une année | Année, niveau, série, libellé |
| `FicheEleveCreee` | Une fiche élève existe | Matricule |
| `FicheEleveRectifiee` | L'identité d'un élève a changé | Champs changés |
| `ResponsableRattache` / `ResponsableDetache` | Un lien élève–responsable est créé ou retiré | Élève, responsable, qualités |
| `InscriptionPreparee` | Un brouillon d'inscription existe | Élève, classe, année |
| `InscriptionValidee` | Une inscription est validée | Élève, classe, type — **consommé par la facturation en phase 3** |
| `InscriptionAbandonnee` | Une inscription est abandonnée | Motif |
| `ClasseChangee` | Un élève change de classe | Ancienne, nouvelle, motif |
| `EleveFusionne` | Deux fiches n'en font plus qu'une | Conservée, absorbée |
| `ImportElevesApplique` | Un import est appliqué | Nombre de lignes, fichier, empreinte |

---

## 15. Rôles

La validation d'une inscription appartient au rôle **Secrétaire** (décision du 7 octobre 2026 : « des écoles n'ont parfois pas de secrétaire, mais comme c'est un rôle » — une école sans secrétaire l'attribue à qui fait ce travail). `enrollments:validate` rejoint sa composition ; la version de composition passe à 2 et les bases installées le reçoivent à leur ouverture (`SPEC_PHASE_1.md` §5.6). Le Directeur garde la validation.

---

## 16. Tests de la phase

| Cible | Nature |
|---|---|
| Transitions d'inscription | Table exhaustive, autorisées et interdites |
| Conditions de validation | Une par condition du §5.3, refus et acceptation |
| Matricule | Sans trou, unique, inchangé par la fusion |
| Téléphones | Formes maliennes et étrangères, refus |
| Recherche | Variantes du §6.2, classement ; 2 000 élèves sous 200 ms |
| Doublons et fusion | Proposés, jamais automatiques ; fusion refusée sur inscriptions concurrentes |
| Import | Les deux dispositions, colonne nom et prénoms réunis, erreurs, sauvegarde préalable |
| Garde | Chaque nouvelle commande refusée sans sa permission |
| Audit | Chaque mutation écrit audit et `outbox` dans la même transaction |
| Bout en bout | Scénarios du §12, captures |

---

## 17. Ordre de réalisation

Chaque étape est livrable et testable avant la suivante, avec ses captures.

1. Niveaux, séries, classes ; composition des rôles en version 2
2. Personnes, élèves, responsables : fiche, matricule, téléphones, photo
3. Recherche tolérante, doublons probables, recherche globale
4. Inscriptions : brouillon, validation, abandon, changement de classe ; assistant UC-01
5. Fiche d'inscription imprimée et QR code
6. Fusion de doublons
7. Export Excel
8. Import des élèves
9. Navigation finale, scénario UC-01 chronométré, captures

---

## 18. Questions à trancher

Appliquées par défaut selon leur recommandation, comme Q-41 : elles ne bloquent pas le code, la réponse du porteur du projet peut les changer.

| Id | Question | Recommandation | Bloque |
|---|---|---|---|
| **Q-45** | Quelle permission pour renommer les niveaux et créer les classes ? | **`academic_year:manage`** : organiser niveaux et classes, c'est préparer l'année ; Administrateur et Directeur. Pas de permission nouvelle | Non, appliquée par défaut |
| **Q-46** | Au jardin, faut-il au moins deux personnes qui viennent chercher l'enfant ? | **Au moins une pour valider, avertissement en dessous de deux** : bloquer à deux empêcherait d'inscrire l'enfant d'une mère seule | Non, appliquée par défaut |
| **Q-47** | Un brouillon d'inscription s'abandonne-t-il ? | **Oui, `BROUILLON → ABANDONNÉE` avec motif**, plutôt qu'une suppression (R-14) | Non, appliquée par défaut |

---

## 19. Journal des révisions

| Version | Date | Modification |
|---|---|---|
| v1.6 | 2026-10-09 | Remarque du porteur du projet : le sexe d'un responsable « père » ou « mère » découle du lien, ne se demande plus à l'écran et s'impose dans le noyau (§4.3) |
| v1.5 | 2026-10-09 | Décision du porteur du projet après validation : le nom complet s'affiche prénoms puis nom, partout (écrans, listes, exports, fiche imprimée `fiche_inscription` v2) ; les champs et les colonnes suivent cet ordre ; les listes restent triées par nom de famille |
| v1.4 | 2026-10-09 | Import : le dialogue du système rend un chemin lu par le noyau, plus de base64 (§10.2) ; neuf étapes livrées, assertions démontrées (`ETAT_AVANCEMENT.md`) |
| v1.3 | 2026-10-07 | Matricule interne saisi par l'école ou généré selon ses règles ; matricule national exigé dès la 9e (§4.2, §5.3) |
| v1.2 | 2026-10-07 | Étape 2 : lieu de naissance exigé à la validation et non à la création (§4.2) ; clés de recherche calculées en mémoire, sans colonnes (§4.1) ; table des équivalences reportée (§6.2, §13) |
| v1.1 | 2026-10-07 | Cycles enseignés choisis au premier démarrage et modifiables (§2.1 bis) |
| v1.0 | 2026-10-07 | Création. Périmètre et réponses du porteur du projet du même jour : validation par le rôle Secrétaire, référentiel limité aux niveaux, séries et classes, jardin en sections, facture ajoutée en phase 3, entrée en phase 2 avant la validation de la phase 0.5 |
