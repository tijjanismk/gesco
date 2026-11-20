# SPEC_DOCUMENTS_ECOLE.md — Les documents que l'école délivre

**Projet** : ERP de gestion scolaire — Tauri 2 + Rust + React/TypeScript
**Phase** : documents de l'école, phase à part, codée après la phase 12 ; version 2.x
**Statut** : v1.1 — recommandations de T-172 à T-186 validées par le porteur du projet le
20 novembre 2026 ; à coder après la phase 12
**Documents parents** : choix du porteur du 20 novembre 2026 (onze documents) ;
`SPEC_PHASE_2.md` (élèves, inscriptions, matricules, photos) ; `SPEC_PHASE_4.md` §9 (départs,
exclusions, certificat de transfert) ; `SPEC_PHASE_5.md` (convocations, remise des bulletins) ;
`SPEC_PHASE_6.md` (personnel, engagements) ; `SPEC_PHASE_7.md` (paie) ;
`SPEC_VERSION_1_1.md` (discipline) ; `SPEC_PHASE_9.md` (fiche de l'établissement, examens,
rapports, signataire) ; `SPEC_PHASE_10.md` (décisions de fin d'année) ; `PROJECT_BLUEPRINT.md`
§13.2 (modèles), §25.9 (vérification par le QR) ; ADR « Actes délivrés et vérification par le
QR » du 20 novembre 2026
**Date** : 20 novembre 2026

---

## 0. Objet et critère de fin

### 0.1 Ce que produit cette phase

Chaque jour, le secrétariat écrit à la main des certificats, des attestations, des listes. Il
recopie des noms, des dates, des classes que GESCO connaît déjà. Il se trompe parfois. Et quand
un papier revient, personne ne peut dire s'il est vrai.

À la fin de la phase, GESCO imprime onze documents de plus :

| N° | Document | Pour qui | Numéroté |
|---|---|---|---|
| 1 | Certificat de scolarité, ou de fréquentation | un élève | oui, `CSC` |
| 2 | Attestation d'inscription | un élève | oui, `AIN` |
| 3 | Carte d'élève | un élève, ou toute une classe | non, mais vérifiable |
| 4 | Exeat, ou certificat de radiation | un élève qui part | oui, `CTR` ou `RAD` |
| 5 | Attestation de passage, attestation de réussite | un élève | oui, `APA` ou `ARE` |
| 6 | Lettre d'avertissement ou de blâme | les parents d'un élève | oui, `LDI` |
| 7 | Tableau d'honneur | une classe, une période | non |
| 8 | Liste de classe, liste d'appel | une classe | non |
| 9 | Registre matricule | l'école | non |
| 10 | Procès-verbal du conseil de classe | une classe, une période | oui, `PVC` |
| 11 | Attestation de travail, certificat de travail, attestation de salaire | un membre du personnel | oui, `ATT`, `CTT`, `ASA` |

Et deux choses servent à tous :
- un **en-tête administratif malien** commun (§3.1) ;
- un écran **Vérifier un document** : on lit le QR d'un papier, ou on tape sa référence, et
  GESCO dit si l'école l'a bien délivré, à qui, quand, et s'il est toujours valable (§4).

### 0.2 Décisions déjà prises qui s'appliquent

- **Les onze documents** (20 novembre 2026) : retenus par le porteur, à coder après la phase 12.
- **La flexibilité sans incohérence** (14 novembre 2026) : un document ne bloque jamais l'école,
  et il ne fait jamais de remarque. Mais ses chiffres et ses mentions sont toujours justes. Il
  n'affiche rien que les données ne justifient pas.
- **Ne pas bloquer, ne pas le mentionner** (12 novembre 2026) : un impayé, un dossier incomplet,
  une paie que l'école ne tient pas dans GESCO ne bloquent aucun document et n'y figurent pas.
- **Interface sans sources** : aucun texte affiché ou imprimé ne cite un numéro de question, de
  spécification, de règle ou une date de décision.
- R-14 : rien ne se supprime. Un acte délivré par erreur s'annule avec un motif ; son numéro reste.
- R-21 : toute commande passe par `Application::executer`. R-45 : chaque document garde la version
  de son modèle. R-46 : les numéros sont séquentiels, sans trou, par exercice.
- Plusieurs postes (phase 11, lot A) : tout se fait depuis un poste relié ; le PDF voyage par son
  contenu.

### 0.3 Ce qui « juste » veut dire ici

Un document imprime une phrase seulement si la donnée qui la porte existe et dit la même chose.
Trois conséquences, appliquées partout :

1. **Pas de donnée, pas de phrase.** Une date de naissance inconnue fait disparaître « né(e) le
   … » ; elle n'est pas remplacée par des points, ni par « non renseigné ».
2. **Pas de décision, pas d'attestation.** GESCO ne propose pas d'imprimer une attestation de
   passage pour un élève dont la décision de fin d'année n'est pas « admis ». Le bouton n'est pas
   là ; à sa place, l'écran montre la décision telle qu'elle est (« Décision : non statuée »). Ce
   n'est pas un refus : c'est l'état de l'élève.
3. **Pas de chiffre retouché.** Une moyenne, un rang, un effectif, un salaire viennent du calcul
   ou de la paie validée. Pour les changer, on corrige la donnée, puis on imprime de nouveau.

### 0.4 Critère de fin, vérifiable

1. Sur l'école d'essai de trois ans, chacun des onze documents s'imprime depuis son écran, en
   PDF, en A4 (A5 où le §5 le permet), avec l'en-tête administratif et le nom de l'école.
2. Chaque document numéroté reçoit le numéro suivant de son type ; deux postes qui impriment en
   même temps n'obtiennent jamais le même numéro, et il n'y a pas de trou.
3. Chaque document numéroté réimprimé est identique, au bit près, au premier, même si l'élève a
   changé de classe ou de nom depuis. Le mot « Duplicata » s'ajoute seulement si l'école l'a
   choisi (T-176).
4. Le QR de chaque document numéroté, lu par la douchette ou tapé à l'écran Vérifier, retrouve
   le document, son bénéficiaire, sa date et son état. Un document annulé est montré « annulé le
   …, motif … ». Un papier dont la référence n'existe pas est montré « inconnu de l'école ».
   La carte d'élève, vérifiée, dit si l'élève est inscrit aujourd'hui, et dans quelle classe.
5. Sur une école qui vient d'être créée, sans élève ni personnel, les listes et le registre
   s'impriment vides ; les documents individuels ne sont simplement pas proposés. Aucun message
   d'erreur.
6. Sur une école où manquent des dates de naissance, des lieux, des photos, des parents, des
   numéros de matricule national, des fonctions, toute la paie : chaque document s'imprime ; la
   phrase qui dépend d'une donnée absente disparaît ; aucun document ne dit qu'il manque quelque
   chose.
7. Un essai indépendant compare, pour chaque document, chaque mention imprimée à la base :
   classe, année, dates, décision, moyenne, rang, sanction, fonction, dates d'emploi, salaires.
   Zéro écart.
8. Aucune attestation de passage pour un élève non admis ; aucune attestation de réussite sans
   résultat « admis » à l'examen ; aucun certificat de travail pour un membre encore présent ;
   aucune lettre pour une sanction annulée ; aucune attestation de salaire au-delà de ce que la
   paie validée ou l'engagement disent.
9. Le tableau d'honneur et le procès-verbal du conseil de classe donnent, pour chaque élève, la
   même moyenne et le même rang que son bulletin publié.
10. Un impayé ne change rien à aucun document de cette phase, ni à sa possibilité, ni à son
    contenu.
11. Un compte qui n'a pas le droit d'un document ne voit pas son bouton et la commande le refuse
    (§11).
12. Une école en 2.0 qui installe cette version ne voit rien changer dans ses écrans ni dans ses
    documents existants ; le certificat de transfert garde ses numéros ; les essais des phases
    précédentes passent sans changement.

## 1. Périmètre

**Inclus** : les onze documents ; l'en-tête administratif ; le signataire et le cachet en
réglage ; la numérotation et le registre des actes délivrés ; l'annulation d'un acte ; l'écran
Vérifier un document ; la saisie du conseil de classe ; les seuils du tableau d'honneur ; les
droits ; le manuel, l'école d'essai, les essais.

**Exclu** :
- La vérification en ligne par un tiers, sans passer par l'école. Le QR garde aujourd'hui la
  référence seule (T-174) ; une adresse en ligne viendra avec le service en ligne, dans une
  version du modèle.
- L'impression sur des cartes en PVC. GESCO imprime les cartes sur papier A4, à découper et à
  plastifier (T-177).
- Les diplômes et attestations officiels de l'État (CEP, DEF, baccalauréat, CAP, BT). L'école
  atteste ce qu'elle sait, elle ne remplace pas le diplôme (T-179).
- Le conseil de discipline et ses procès-verbaux.
- Le contrat de travail et la lettre d'engagement : ils existent déjà (phase 6).
- Tout document qui dirait qu'un élève ou un parent doit de l'argent, ou qu'il est « libre de
  tout engagement ».

## 2. Ce que l'application possède déjà

Relevé fait sur le dépôt au 20 novembre 2026 (migrations 0001 à 0066).

| Ce qui existe | Où | Sert à |
|---|---|---|
| Moteur Typst, catalogue de modèles embarqués, surcharge par l'école | `crates/gesco-docs/src/catalogue.rs`, `moteur.rs` | tous les documents |
| En-tête commun : logo, nom, adresse, téléphones, mentions de pied ; papier A4 ou A5 | `modeles/commun/entete.typ` | à compléter (§3.1) |
| En-tête administratif par lignes (académie, CAP…) | `modeles/rapport_ecole` | modèle de l'en-tête malien |
| Registre des documents produits : chemin, empreinte, modèle, version, auteur, poste | table `document` | réimpression à l'identique, vérification |
| QR de vérification : `GESCO:DOC:<identifiant>`, ajouté à tout modèle qui déclare `qr` | `gesco-docs/src/qr.rs`, `documents.rs` | tous les documents vérifiables |
| Numérotation sans trou, par type et par exercice ; types `CTR` et `CSC` déjà réservés | `core/numerotation.rs`, table de compteurs | numéros des actes |
| Trace d'un numéro annulé | modèle `numero_annule` | annulation d'un acte |
| Certificat de transfert numéroté, réimprimable | table `certificat_transfert`, modèle | exeat (§5.4) |
| Élève : matricule interne, matricule national, acte de naissance, école d'origine, année de recrutement | `eleve` | tous les documents d'élève |
| Personne : noms, sexe, date et lieu de naissance (parfois estimée), nationalité, **photo** | `personne` | cartes, certificats |
| Responsables et lien (père, mère, tuteur, autre) | `eleve_responsable` | filiation, lettres aux parents |
| Inscription : classe, date, type, statut (validée, transférée, abandonnée, radiée, achevée), redoublant, décision de fin d'année, date et motif de départ, destination | `inscription` | scolarité, passage, radiation, registre |
| Exclusions temporaires et définitives | `exclusion` | radiation |
| Sanctions : avertissement, blâme, retenue, convocation ; décidées par ; période ; annulation motivée | `sanction` | lettre aux parents, procès-verbal |
| Bulletins publiés : moyenne, rang, mention, effectif, premier, dernier, moyenne de classe, moyenne et rang annuels, sanctions du conseil | `bulletin`, `ContenuBulletin` | tableau d'honneur, passage, procès-verbal |
| Échelle des mentions (Excellent, Très bien, Bien…) | `pedagogie/regle.rs` | tableau d'honneur |
| Résultats aux examens par élève (inscrit, présent, admis, mention) | `resultat_examen` | attestation de réussite |
| Enseignants et affectations par classe | `affectation_enseignement` | procès-verbal (présents proposés) |
| Personnel : matricule, fonction, arrivée, départ, fonctionnaire, numéro INPS, diplômes | `membre_personnel` | documents du personnel |
| Engagements : nature, début, fin, fonction, rémunération, statut | `engagement` | travail, salaire |
| Paie du mois validée : brut, net, bulletin figé | `paie_mois`, `bulletin_paie` | attestation de salaire |
| Fiche de l'établissement : commune, cercle, région, académie, CAP, statut | `etablissement`, `valeur_fiche` | en-tête, « Fait à » |
| Qualité du signataire d'un rapport | `rapport_ecole.qualite_signataire` | modèle du réglage du signataire |
| Permissions `documents:deliver`, `documents:reprint`, `students:read`, `sanctions:*`, `employees:read`, `contracts:read`, `payroll:*` | `docs/matrice-permissions.md` | droits (§11) |

Ce qui manque : l'en-tête administratif commun, le signataire des actes, le registre des actes
délivrés avec leur état, l'écran de vérification, la saisie du conseil de classe, les seuils du
tableau d'honneur. Le §10 les décrit ; l'ADR du 20 novembre 2026 fixe les tables.

## 3. Ce qui est commun aux documents

### 3.1 En-tête administratif

Les actes officiels de l'école portent l'en-tête malien en deux colonnes :

| À gauche | À droite |
|---|---|
| MINISTÈRE DE L'ÉDUCATION NATIONALE | RÉPUBLIQUE DU MALI |
| ACADÉMIE D'ENSEIGNEMENT DE … | Un Peuple – Un But – Une Foi |
| CENTRE D'ANIMATION PÉDAGOGIQUE DE … (fondamental) | |
| Nom de l'école, logo | |

- Le ministère s'écrit tel que l'école le règle (son intitulé change avec les gouvernements) ;
  GESCO livre « Ministère de l'Éducation nationale ».
- Une ligne dont la donnée manque (pas de CAP renseigné, par exemple) disparaît ; les autres
  remontent. L'en-tête ne montre jamais « … » ni de case vide.
- Le CAP ne s'imprime que pour le fondamental et le jardin ; au lycée et au technique, l'académie
  suffit (T-173).
- L'école peut couper l'en-tête administratif, document par document (une école privée qui
  préfère son propre papier à en-tête). Le nom de l'école reste.
- L'en-tête court de `entete.typ` (logo, nom, adresse, téléphones) sert aux listes et au
  registre, qui sont des documents internes.

### 3.2 Signataire, cachet, lieu et date

- **Signataire** : un réglage de l'école donne, pour chaque famille de documents (élèves,
  discipline, personnel, conseil de classe), la qualité imprimée (« Le Directeur », « La
  Directrice », « Le Proviseur », « Le Censeur », « Le Promoteur », « Le Directeur des études »)
  et, si l'école le veut, le nom. Par défaut : « Le Directeur », sans nom. Le secrétariat peut
  changer la qualité au moment d'imprimer ; ce choix est gardé sur l'acte.
- **Cachet** : un espace vide à gauche de la signature, de 40 mm. GESCO n'imprime pas d'image de
  cachet ni de signature (T-175).
- **« Fait à … le … »** : le lieu est la commune de la fiche de l'établissement ; s'il manque,
  la phrase devient « Fait le … ». La date est celle du jour de l'impression, en toutes lettres
  (`date_longue`), jamais une date choisie : un acte ne s'antidate pas.
- **Formule finale** des certificats et attestations : « En foi de quoi, le présent certificat
  (la présente attestation) est délivré(e) pour servir et valoir ce que de droit. »

### 3.3 Accords et formules

- Le sexe de la personne accorde le texte : « né », « née » ; « l'élève », « inscrit »,
  « inscrite » ; « M. », « Mme ». Sexe inconnu : la forme double « né(e) », « inscrit(e) ».
- Civilité : « M. » ou « Mme » pour un adulte ; pas de civilité pour un élève.
- Les noms s'impriment NOM en capitales, prénoms en minuscules accentuées : « TRAORÉ Aminata ».
- Les montants en francs CFA, en chiffres et en lettres pour l'attestation de salaire
  (`formules.rs` a déjà le nombre en lettres des reçus).
- Les classes s'écrivent comme dans GESCO (« 7e année A », « 10e Lettres »).

### 3.4 Numéro, QR, réimpression, annulation

- Un **acte** est un document numéroté qui atteste quelque chose. Chaque acte reçoit un numéro
  `PREFIXE-EXERCICE-COMPTEUR` (par exemple `CSC-2026-000014`), réglable comme les autres
  numéros, et un QR qui encode la référence du document.
- La référence du document s'imprime en clair sous le QR, pour qu'on puisse la dicter au
  téléphone.
- **Réimprimer** un acte ressort le même PDF, au bit près, depuis le registre des documents. Il
  ne recalcule rien.
- **Délivrer de nouveau** (l'élève a perdu son certificat) crée un nouvel acte, avec un nouveau
  numéro, sur les données du jour. L'ancien reste valable, sauf si on l'annule.
- **Annuler** un acte demande un motif ; son numéro reste pris ; la vérification le montre
  annulé. Un acte remplacé par un nouveau (erreur de date corrigée) est montré « remplacé par
  CSC-2026-000015 ».
- Les **documents non numérotés** (listes, registre, tableau d'honneur, cartes) se produisent et
  se gardent au registre des documents ; seule la carte porte un QR.

### 3.5 Ce que fait une donnée absente

| Donnée absente | Effet sur le document |
|---|---|
| Date de naissance | la mention « né(e) le … » disparaît ; dans un tableau, case vide |
| Date estimée (« vers 2014 ») | s'imprime « né(e) vers 2014 » |
| Lieu de naissance | « à … » disparaît |
| Photo | la carte garde un cadre vide pour coller une photo |
| Matricule national | la mention disparaît |
| Père, mère | la ligne de filiation disparaît ; le registre laisse la case vide |
| Responsable | la lettre s'adresse « Aux parents de l'élève … » |
| Commune de l'école | « Fait le … » |
| Fonction du personnel | impossible : la fonction est obligatoire sur la fiche |
| Date d'arrivée du personnel | on prend le début du premier engagement ; sinon « depuis le » disparaît et la phrase devient « est employé(e) dans notre établissement en qualité de … » |
| Paie validée | l'attestation de salaire prend l'engagement en cours (§5.11) |

Rien d'autre ne change. Aucune de ces absences n'est signalée sur le papier.

## 4. Vérifier un document

### 4.1 Le geste

Écran **Vérifier un document**, dans le menu Documents et sur l'accueil du secrétariat. Une seule
case. On y lit le QR avec une douchette (elle tape comme un clavier), ou on tape la référence,
ou le numéro de l'acte (`CSC-2026-000014`). GESCO montre :

- **Délivré par l'école** : type, numéro, date, bénéficiaire (nom, classe ou fonction),
  signataire, poste et compte qui l'a produit ; l'aperçu du PDF original, pour le comparer au
  papier ;
- **Annulé** : en plus, la date, le motif, et l'acte qui le remplace s'il y en a un ;
- **Inconnu de l'école** : aucune référence ni aucun numéro ne correspond.

Pour la **carte d'élève**, GESCO montre en plus l'état d'aujourd'hui : « inscrit(e) en 7e année
A », ou « n'est plus inscrit(e) depuis le 15 janvier 2027 », sans motif.

### 4.2 Règles

- La vérification ne fait que lire ; elle est journalisée (qui a vérifié quoi, quand).
- Elle ne dit rien de plus que le papier : pas de moyenne, pas de sanction, pas de frais.
- Le QR ne porte aucune donnée personnelle : seulement `GESCO:DOC:<identifiant>`. Une photo du
  papier qui circule ne révèle rien de plus que ce qui est imprimé.
- Un document produit dans une autre école GESCO est « inconnu de l'école » : chaque école ne
  vérifie que les siens (T-174).

## 5. Les documents, un par un

Pour chacun : à quoi il sert, ce qu'il imprime et d'où vient chaque donnée, quand il est
proposé, qui peut l'imprimer, comment il se vérifie.

### 5.1 Certificat de scolarité, ou de fréquentation

**Sert à** : prouver qu'un élève suit les cours (allocations familiales, bourse, dossier
administratif, voyage).

**Texte** : « Le Directeur soussigné certifie que l'élève **TRAORÉ Aminata**, née le 2 mars 2014
à Kati, matricule 2024-0153, fille de Moussa TRAORÉ et de Fatoumata DIARRA, est régulièrement
inscrite en classe de **7e année A** pour l'année scolaire **2026-2027** et fréquente
l'établissement depuis le 2 octobre 2026. »

| Mention | Source |
|---|---|
| Nom, prénoms, sexe, naissance | `personne` |
| Matricule, matricule national | `eleve` |
| Filiation | responsables de lien père et mère ; facultative, réglage (T-178) |
| Classe, année | inscription validée de l'année en cours |
| « fréquente depuis le » | date d'inscription de l'année ; ou, si l'élève est dans l'école depuis plusieurs années sans interruption, la date de sa première inscription (« fréquente l'établissement depuis l'année scolaire 2022-2023 ») |

**Titre** : « Certificat de scolarité » par défaut ; l'école peut choisir « Certificat de
fréquentation ». Même document, même numéro `CSC`.

**Proposé** : pour un élève dont l'inscription de l'année en cours est validée. Pour un élève
parti, ou une année close, le même écran propose un **certificat de scolarité pour une année
passée** : « a été inscrit(e) en classe de … pour l'année scolaire 2025-2026, du … au … ». La
période imprimée est celle des dates connues ; jamais « est inscrit » au présent pour un élève
parti.

**Droits** : `documents:deliver` (secrétariat, direction).
**Vérification** : QR ; montre l'élève, la classe, l'année attestées.

### 5.2 Attestation d'inscription

**Sert à** : prouver qu'un élève est inscrit pour l'année, souvent avant la rentrée ou avant le
premier jour de classe (dossier de bourse, d'état civil, de transport).

**Différence avec le certificat de scolarité** : elle dit « est inscrit(e) », pas « fréquente ».
Elle peut être délivrée dès que l'inscription est validée, même avant le premier jour de
l'année.

**Texte** : « … certifie que l'élève …, est inscrit(e) en classe de 7e année A au titre de
l'année scolaire 2026-2027, depuis le 14 septembre 2026. »

**Proposé** : inscription validée, de l'année en cours ou de l'année qui se prépare. Pas pour un
brouillon. Une inscription transférée ou abandonnée ne la propose plus.

**Ce qu'elle ne dit pas** : rien sur les frais, même « réglés ». Une école qui veut attester un
paiement imprime le reçu ou le relevé de compte, qui existent.

**Droits** : `documents:deliver`.
**Vérification** : QR ; montre l'inscription attestée et son état d'aujourd'hui.

### 5.3 Carte d'élève

**Format** : format carte bancaire (85,6 × 54 mm), au recto seul ; dix cartes par feuille A4, à
découper (T-177). Impression pour un élève ou pour toute une classe.

| Sur la carte | Source |
|---|---|
| En-tête court : nom et logo de l'école, « Carte d'élève », année scolaire | `etablissement`, année |
| Photo, 25 × 30 mm | `personne.photo` ; cadre vide si absente |
| NOM, prénoms | `personne` |
| Né(e) le | si connue |
| Matricule | `eleve.matricule` ; le matricule national au lycée s'il existe |
| Classe | inscription validée de l'année |
| Téléphone d'un responsable | facultatif, réglage, éteint par défaut (T-178) |
| QR | référence du document de la carte |
| Signature et cachet | petit espace, « Le Directeur » |

**Proposé** : pour un élève à l'inscription validée de l'année. Imprimée pour une classe, la
feuille ne prend que les élèves inscrits ce jour-là, dans l'ordre de la liste de classe.

**Validité** : l'année scolaire imprimée. Un changement de classe dans l'année ne rend pas la
carte fausse pour la vérification, qui donne la classe d'aujourd'hui ; l'école réimprime si elle
veut.

**Pas de numéro d'acte** : une carte n'atteste rien d'autre que l'inscription, que la
vérification montre à jour. Elle porte quand même son QR et entre au registre des documents.

**Droits** : `documents:deliver`.
**Vérification** : QR ; montre l'élève, sa photo, et s'il est inscrit aujourd'hui (§4.1).

### 5.4 Exeat, ou certificat de radiation

Deux situations, deux documents, jamais pour le même départ.

**Exeat (départ pour une autre école)**. GESCO imprime déjà le **certificat de transfert**
numéroté `CTR` quand un élève est transféré. L'exeat est ce même acte : l'école choisit, dans ses
réglages, son titre, « Certificat de transfert » ou « Exeat ». Même numéro, même table, même
modèle (version 2, qui ajoute l'en-tête administratif et le signataire). Ainsi un même départ
n'a jamais deux actes de numéros différents (T-180).

**Certificat de radiation (l'élève quitte sans aller dans une école connue)**. Nouvel acte,
numéro `RAD`.

**Texte** : « … certifie que l'élève …, inscrit(e) en classe de 8e année B pour l'année
scolaire 2026-2027, a été radié(e) des registres de l'établissement à compter du 15 janvier
2027. »

| Mention | Source |
|---|---|
| Classe, année | l'inscription quittée |
| Date de radiation | `inscription.date_depart` ; pour une exclusion définitive, `exclusion.du` |
| Motif | **non imprimé par défaut** ; l'école peut imprimer le motif d'abandon (« départ de la famille ») ; jamais celui d'une exclusion (T-181) |

**Proposé** : pour une inscription au statut abandonnée ou radiée. Pour une exclusion définitive
révoquée (l'élève est revenu), le certificat déjà délivré est annulé d'office, avec le motif
« exclusion révoquée », et la vérification le montre.

**Droits** : `enrollments:transfer` (comme le certificat de transfert).
**Vérification** : QR ; montre l'élève, la classe quittée, la date.

### 5.5 Attestation de passage, attestation de réussite

**Attestation de passage** (numéro `APA`). « … certifie que l'élève …, inscrit(e) en classe de
7e année A pour l'année scolaire 2025-2026, a été déclaré(e) **admis(e) en classe de 8e année**
par le conseil des maîtres (de classe). »

| Mention | Source |
|---|---|
| Classe et année | inscription de l'année concernée |
| Admis en classe de | niveau suivant de `niveau` (ordre des niveaux) ; au lycée, sans série si la série de l'an prochain n'est pas connue |
| Moyenne annuelle, rang | **facultatifs**, réglage ; imprimés seulement si le bulletin de la dernière période est publié et porte la moyenne annuelle (T-182) |

**Proposé** : seulement si `decision_fin_annee` vaut « admis ». Pour « redouble », « exclu »,
« rattrapage » ou « non statué », l'écran montre la décision, sans bouton. Pour la classe
terminale d'un cycle (6e, 9e, 12e), où « admis » veut dire « a fini le cycle », la phrase devient
« a achevé avec succès la classe de 9e année ». Après le rattrapage, quand la décision devient
« admis », le bouton apparaît.

**Attestation de réussite** (numéro `ARE`). « … certifie que l'élève …, inscrit(e) en classe de
9e année pour l'année scolaire 2025-2026, a été déclaré(e) admis(e) à l'examen du **Diplôme
d'études fondamentales (DEF)**, session de 2026, avec la mention Bien. »

| Mention | Source |
|---|---|
| Examen | `resultat_examen.examen` : entrée en 7e, DEF, baccalauréat, CAP, BT |
| Session | année civile de fin de l'année scolaire |
| Mention | `resultat_examen.mention`, si elle existe |

**Proposé** : seulement si `resultat_examen.admis` vaut vrai pour cet élève. Une école qui n'a
saisi que des totaux par sexe n'a pas de résultat par élève : l'attestation n'est pas proposée,
et l'écran des examens permet de saisir le résultat de l'élève.

Cette attestation est celle de l'école. Elle ne remplace pas le relevé ni le diplôme délivrés par
l'administration (T-179).

**Droits** : `documents:deliver`.
**Vérification** : QR ; montre la décision ou le résultat attesté et son état d'aujourd'hui (si
la décision a changé depuis, la vérification le dit : « décision modifiée depuis : redouble »).

### 5.6 Lettre d'avertissement ou de blâme aux parents

**Sert à** : informer la famille d'une sanction, et garder la trace qu'elle a été envoyée.

**Texte** (exemple de l'avertissement) :

« À Monsieur Moussa TRAORÉ, parent de l'élève TRAORÉ Aminata, 7e année A.

Objet : avertissement.

Monsieur, nous avons le regret de vous informer que votre fille a reçu le 12 février 2027 un
**avertissement** du conseil de classe pour le motif suivant : *bavardages répétés*. Nous vous
prions de bien vouloir lui en parler et de veiller avec nous à son comportement. Nous restons à
votre disposition pour en parler.

Veuillez agréer, Monsieur, l'expression de nos salutations distinguées. »

Puis un coupon détachable, comme celui de la convocation : « Je soussigné(e) …, parent de …,
reconnais avoir reçu la lettre n° LDI-2027-000003. Date et signature. »

| Mention | Source |
|---|---|
| Destinataire | responsable principal de l'élève (lien, civilité selon le sexe) ; sinon « Aux parents de l'élève » |
| Nature | `sanction.nature` : avertissement ou blâme |
| Date, motif, décidée par | `sanction` |
| Formule | texte livré, un par nature ; l'école peut le réécrire dans ses réglages (sans toucher aux mentions venues de la sanction) |

**Proposé** : depuis la fiche de la sanction, pour un avertissement ou un blâme non annulé. Une
lettre par sanction ; la réimprimer ressort la même. Une sanction annulée après l'envoi : la
lettre est annulée d'office, motif « sanction annulée ».

Les retenues et les convocations ont déjà leur billet (convocation) ; elles n'ont pas de lettre.

**Droits** : `sanctions:create` (direction, directeur des études, surveillant général).
**Vérification** : QR ; montre la sanction, et si elle est annulée.

### 5.7 Tableau d'honneur

**Sert à** : afficher et récompenser les meilleurs élèves d'une classe à la fin d'une période.

**Trois distinctions**, de la plus haute à la plus basse : **Félicitations**,
**Encouragements**, **Tableau d'honneur**. Chaque élève en reçoit au plus une.

**Seuils** : en pour cent du barème, pour que le même réglage serve aux notes sur 10 et sur 20.
Livrés : Tableau d'honneur à 60 % (12 sur 20, 6 sur 10), Encouragements à 70 % (14 sur 20),
Félicitations à 80 % (16 sur 20). L'école les règle, par cycle (T-183).

**Condition de conduite** (réglage, allumé par défaut) : un avertissement ou un blâme non annulé
de la période retire la distinction. La note de conduite, quand l'école l'a activée, peut aussi
être exigée (seuil réglable, éteint par défaut).

**Source** : la moyenne de la période sur le **bulletin publié** de l'élève. Un élève sans
bulletin publié n'apparaît pas. Les élèves partis avant la fin de la période n'apparaissent pas.

**Deux impressions** :
- **l'affiche de la classe** : A4 portrait, titre, classe, période, les trois listes dans
  l'ordre du rang, avec la moyenne ; la liste vide s'omet ; si aucun élève n'a de distinction,
  l'affiche dit seulement le titre et « Classe de … — 1er trimestre » (T-184) ;
- **le diplôme individuel**, facultatif : A5 paysage, « Tableau d'honneur décerné à … » ; sans
  numéro, avec le QR du document.

**Au bulletin** : la distinction peut s'imprimer sur le bulletin, à côté des sanctions du
conseil, avec une nouvelle version du modèle de bulletin. Elle ne s'ajoute qu'aux bulletins
publiés après ce réglage ; un bulletin déjà publié ne change jamais (T-184).

**Droits** : `reports:generate` (direction, directeur des études, secrétariat).
**Vérification** : le diplôme individuel porte un QR ; l'affiche non.

### 5.8 Liste de classe et liste d'appel

**Liste de classe** : en-tête court, classe, année, date, enseignant titulaire ou professeur
principal s'il est connu. Colonnes : n° d'ordre, matricule, NOM et prénoms, sexe, date de
naissance, « R » pour un redoublant. Pied : effectif, garçons, filles.

Options à l'impression : colonnes en plus (âge, téléphone du responsable, nouveau ou ancien),
colonnes vides à titre libre (pour des notes à la main), A4 portrait ou paysage, export Excel.

**Liste d'appel** : mêmes lignes, puis une grille de cases vides : un mois, une colonne par jour
de classe (les jours sans cours selon le calendrier de l'école sont grisés), ou une semaine en
demi-journées. En bas de chaque colonne, une ligne « Absents » vide.

**Qui est sur la liste** : les élèves dont l'inscription dans cette classe est validée à la date
de la liste (règle 6 de la phase 9 : date d'inscription passée, pas encore parti). Ordre
alphabétique par nom, puis prénoms. Un élève arrivé après la date n'y est pas ; un élève parti
avant non plus. Le total est toujours le nombre de lignes.

**Droits** : `students:read`. Un enseignant imprime les listes de ses classes seulement (portée
par classe, existante). Pas de numéro, pas de QR.

### 5.9 Registre matricule

**Sert à** : tenir le grand registre de l'école, où chaque élève entré a une ligne, dans l'ordre
de son matricule. Il se relie et se garde ; l'inspection le demande.

**Colonnes** (A4 paysage, ou A3 si l'imprimante le permet) :

| Colonne | Source |
|---|---|
| N° matricule | `eleve.matricule` |
| NOM et prénoms | `personne` |
| Sexe | `personne` |
| Date et lieu de naissance | `personne` |
| Filiation : père, mère | responsables de lien père et mère |
| Date d'entrée | première inscription dans l'école |
| Classe d'entrée | classe de cette inscription |
| École d'origine | `inscription.etablissement_origine` de la première inscription |
| Date de sortie | `date_depart` de la dernière inscription transférée, abandonnée ou radiée ; ou fin de l'année pour une scolarité achevée en classe terminale |
| Classe de sortie | classe de cette inscription |
| Motif de sortie | « transfert vers … », « abandon », « radiation », « fin de cycle » ; jamais le motif d'une exclusion |
| Observations | vide, pour la main |

**Impression** : par année d'entrée, ou par plage de matricules, ou tout. Chaque feuille porte
« Registre matricule — feuillet n° … » et un pied « Arrêté le … ». Les élèves fusionnés (doublons)
ne s'impriment qu'une fois, sous le matricule gardé.

**Ce qui est juste** : la ligne reflète les données du jour d'impression. Une correction de nom
se voit à la réimpression ; l'ancien feuillet reste au registre des documents. Un élève encore
inscrit a sa date de sortie vide.

**Droits** : `students:read` et `documents:deliver` (secrétariat, direction). Pas de numéro ;
pas de QR.

### 5.10 Procès-verbal du conseil de classe

**Sert à** : garder la trace écrite du conseil de classe d'une période : qui était là, ce qui a
été constaté, ce qui a été décidé pour chaque élève.

**Ce que l'école saisit** (écran Conseil de classe, §9.4) : date et heure, lieu, président,
secrétaire de séance, membres présents (les enseignants de la classe sont proposés, cochés ;
on ajoute délégués des élèves, parents, autres), observations générales, et pour chaque élève
une observation libre courte s'il le faut.

**Ce que GESCO calcule, sans le laisser retoucher** :
- effectif, élèves ayant composé, moyenne de la classe, premier, dernier, nombre et pourcentage
  d'élèves à la moyenne, par sexe — depuis les bulletins publiés de la période ;
- la liste des élèves dans l'ordre du rang : moyenne, rang, mention, distinction (§5.7),
  avertissement ou blâme du conseil de la période ;
- à la dernière période : moyenne et rang annuels, et décision de fin d'année (admis, redouble,
  exclu, rattrapage, non statué).

Les sanctions et les décisions **ne se saisissent pas dans le procès-verbal** : elles se
saisissent dans la discipline et dans les décisions de fin d'année, où l'écran du conseil mène
directement. Le procès-verbal les relit. Ainsi il ne peut pas dire autre chose que les bulletins.

**Arrêter** le procès-verbal le fige et lui donne son numéro `PVC`. Le rouvrir est possible,
journalisé ; l'ancien acte est marqué « remplacé ». Un bulletin publié de nouveau après l'arrêt
(correction de note) : le procès-verbal arrêté ne bouge pas, et l'écran du conseil montre, pour
cette classe, « bulletins modifiés depuis l'arrêt » avec le bouton Rouvrir. Rien n'est imprimé à
ce sujet.

**Signatures** : le président et le secrétaire de séance, au pied.

**Droits** : nouvelle permission `class_council:manage` (direction, directeur des études) ;
lecture du procès-verbal arrêté avec `reports:generate`. Les enseignants le lisent pour leurs
classes s'ils ont `students:read` sur la classe (T-185).
**Vérification** : QR ; montre la classe, la période, la date du conseil, l'état.

### 5.11 Documents du personnel

Tous partent de la fiche du membre (`membre_personnel`) et de ses engagements. Ils ne lisent
jamais le dossier de pièces. Ils ne disent jamais qu'une pièce manque.

**Attestation de travail** (`ATT`). « Le Directeur soussigné atteste que **M. Boubacar
KONÉ**, né le …, est employé dans notre établissement en qualité de **professeur de
mathématiques** depuis le 1er octobre 2023. »

| Mention | Source |
|---|---|
| Civilité, nom, naissance | `personne` |
| Fonction | engagement en cours ; sinon `membre_personnel.fonction` |
| Depuis le | `arrive_le` ; sinon début du premier engagement ; sinon la phrase sans date |
| Nature de l'emploi | facultative (réglage) : « sous contrat à durée indéterminée », « à durée déterminée jusqu'au … », « en qualité de vacataire », « en stage », « à titre bénévole » — d'après l'engagement en cours |
| Numéro de fonctionnaire | jamais par défaut ; l'école peut l'ajouter pour un fonctionnaire |

**Proposée** : pour un membre présent (pas de `parti_le`, ou une date future).

**Certificat de travail** (`CTT`). Remis au départ. « … certifie avoir employé **M. Boubacar
KONÉ** du 1er octobre 2023 au 31 juillet 2027, en qualité de professeur de mathématiques. » S'il
a occupé plusieurs fonctions, une ligne par période, d'après la suite des engagements. Le
certificat ne contient **ni appréciation, ni motif de départ** ; la mention « libre de tout
engagement » s'imprime si l'école la choisit, parce qu'elle dit seulement que l'emploi a pris fin
(T-186).

**Proposé** : pour un membre parti (`parti_le` passée), ou dont tous les engagements sont
terminés ou rompus.

**Attestation de salaire** (`ASA`). Deux formes, choisies par GESCO selon les données :

- **depuis la paie** : si la paie de l'école est validée pour ce membre sur les mois demandés
  (par défaut les trois derniers, au plus douze), l'attestation donne, mois par mois, le salaire
  brut et le salaire net de ses bulletins de paie validés, puis leur moyenne ; en chiffres et en
  lettres pour la moyenne. Un mois sans bulletin validé n'est pas imprimé ; la période annoncée
  est celle des mois imprimés.
- **depuis l'engagement** : si l'école ne tient pas la paie dans GESCO, l'attestation dit « perçoit
  une rémunération mensuelle de 150 000 F CFA » (ou « est rémunéré à l'heure, au taux de 2 500 F
  CFA ») d'après l'engagement en cours. Elle dit « rémunération convenue » et non « salaire net
  perçu ».

**Proposée** : pour un membre présent dont l'engagement en cours a une rémunération, ou qui a au
moins un bulletin de paie validé. Un bénévole n'a pas d'attestation de salaire. Pour un
fonctionnaire, l'attestation ne porte que ce que l'école lui verse elle-même (indemnités,
vacations), jamais le traitement de l'État.

**Droits** : attestation et certificat de travail, `employees:read` et `documents:deliver` ;
attestation de salaire, `contracts:read` (direction, comptable, gestionnaire RH). Un membre ne
peut pas imprimer sa propre attestation depuis son compte enseignant.
**Vérification** : QR ; montre le membre, la fonction, les dates attestées ; pour l'attestation
de salaire, les montants attestés, à la personne qui a le droit `contracts:read` seulement — les
autres voient « attestation de salaire délivrée le … à M. … », sans montant.

## 6. Données que la phase ajoute

L'ADR du 20 novembre 2026 décide de la forme. En résumé :

| Donnée | Où | Pourquoi |
|---|---|---|
| Registre des actes délivrés : type, numéro, document, bénéficiaire (inscription, sanction, membre, conseil), mentions attestées figées, état (valide, annulé, remplacé), motif et date d'annulation, acte de remplacement | table nouvelle `acte_delivre` | vérification, annulation, réimpression |
| Types de numéros `AIN`, `RAD`, `APA`, `ARE`, `LDI`, `PVC`, `ATT`, `CTT`, `ASA` | `TypeDocumentNumerote` (code) | numérotation ; `CSC` et `CTR` existent |
| Conseil de classe : classe, période, date, lieu, président, secrétaire, présents, observations, observations par élève, état (saisi, arrêté), acte | tables nouvelles `conseil_classe`, `conseil_classe_eleve` | procès-verbal |
| Réglages des documents : ministère, en-tête administratif par document, signataire par famille, titres choisis (scolarité ou fréquentation, transfert ou exeat), options (filiation, téléphone sur la carte, motif d'abandon, moyenne sur l'attestation, « libre de tout engagement », duplicata), formules des lettres | `parametre` (existant) | réglages, sans table |
| Seuils et conditions du tableau d'honneur, par cycle | `parametre` | §5.7 |
| Distinction au bulletin | contenu du bulletin, nouvelle version du modèle | §5.7 |

La table `certificat_transfert` reste ; ses actes entrent aussi au registre des actes, sans
changer leur numéro, par une reprise faite à la migration.

## 7. Modèles Typst

Un dossier par modèle dans `crates/gesco-docs/modeles/`, déclaré au catalogue
(`MODELES_INTEGRES`). Chacun déclare `qr` s'il est vérifiable ; le moteur ajoute alors la
référence et le QR (existant).

| Modèle | Version | Papier |
|---|---|---|
| `commun/entete.typ` : ajout de `entete-administratif`, `signature`, `fait-a` | — | — |
| `certificat_scolarite` | 1 | A4 ou A5 |
| `attestation_inscription` | 1 | A4 ou A5 |
| `carte_eleve` | 1 | A4, 10 cartes |
| `certificat_transfert` | **2** (en-tête, signataire, titre « Exeat » au choix) | A4 |
| `certificat_radiation` | 1 | A4 ou A5 |
| `attestation_passage` | 1 | A4 ou A5 |
| `attestation_reussite` | 1 | A4 ou A5 |
| `lettre_sanction` | 1 | A4 |
| `tableau_honneur` | 1 | A4 |
| `diplome_honneur` | 1 | A5 paysage |
| `liste_classe` | 1 | A4 portrait ou paysage |
| `liste_appel` | 1 | A4 paysage |
| `registre_matricule` | 1 | A4 paysage ou A3 |
| `proces_verbal_conseil` | 1 | A4 |
| `attestation_travail` | 1 | A4 |
| `certificat_travail` | 1 | A4 |
| `attestation_salaire` | 1 | A4 |
| `bulletin` | **suivante** (distinction) | inchangé |

Un document déjà produit en version 1 du certificat de transfert se réimprime en version 1 : le
PDF est gardé (R-45). Une école qui a surchargé un modèle garde le sien.

Les textes fixes des modèles ne citent aucune source, aucun article, aucune règle.

## 8. Règles

1. **Un document ne bloque jamais l'école.** Aucun impayé, aucune pièce de dossier, aucune paie
   absente n'empêche un document. Ce qui décide si un document est proposé, c'est seulement que
   ce qu'il atteste soit vrai (§0.3).
2. **Il ne fait aucune remarque.** Aucun papier ne porte « non renseigné », « à compléter »,
   « dossier incomplet », « sous réserve du paiement ».
3. **Chaque mention vient d'une donnée.** Pas de donnée, pas de mention (§3.5).
4. **Les chiffres viennent du calcul.** Moyennes, rangs, effectifs, distinctions : des bulletins
   publiés ; salaires : des bulletins de paie validés, ou de l'engagement, dit comme tel.
5. **Un acte ne change pas.** Il se réimprime à l'identique, s'annule ou se remplace ; il ne se
   modifie pas.
6. **La date d'un acte est le jour de son impression.**
7. **Le motif d'une exclusion et le handicap ne s'impriment sur aucun document de cette phase.**
8. **Une liste compte à une date** : inscription validée, datée de ce jour ou d'avant, pas
   encore partie.
9. **La vérification ne montre que ce que le papier montre**, plus l'état de l'acte et, pour la
   carte, l'inscription d'aujourd'hui.
10. **Chaque délivrance, réimpression, annulation et vérification est journalisée.**

## 9. Écrans

### 9.1 Depuis la fiche de l'élève

Un bouton **Documents** ouvre la liste des documents possibles pour cet élève, chacun avec son
état : « à délivrer », ou « délivré le … (n° …) » avec Réimprimer, Délivrer de nouveau, Annuler.
Un document qui n'est pas possible aujourd'hui n'est pas dans la liste, sauf l'attestation de
passage, remplacée par la ligne « Décision de fin d'année : … » qui mène à l'écran des
décisions.

Avant l'impression, un aperçu montre le document tel qu'il sortira, avec le signataire choisi.

### 9.2 Depuis la classe

Boutons : Liste de classe, Liste d'appel, Cartes d'élève (toute la classe ou une sélection),
Tableau d'honneur (choix de la période), Conseil de classe, Certificats de scolarité en série
(un acte par élève, un seul PDF).

### 9.3 Depuis la fiche du personnel

Bouton **Documents** : Attestation de travail, Certificat de travail, Attestation de salaire
(choix des mois), selon le §5.11 et les droits du compte.

### 9.4 Conseil de classe

Choix de la classe et de la période. En haut, la séance (date, lieu, président, secrétaire,
présents). Au milieu, le tableau calculé de la classe, en lecture, avec un lien par élève vers
ses sanctions et sa décision. En bas, les observations. Boutons : Aperçu, Arrêter, Imprimer,
Rouvrir. Deux personnes qui saisissent le même conseil ne s'écrasent pas (refus si le texte a
changé depuis sa lecture, comme les rapports).

### 9.5 Documents délivrés

Menu Documents : la liste des actes, filtrable par type, date, élève ou membre, état ; on y
réimprime et on y annule. Export Excel. C'est le registre des actes délivrés de l'école.

### 9.6 Vérifier un document

Voir §4. Une case, un résultat, l'aperçu du PDF original.

### 9.7 Réglages des documents

Un onglet dans les réglages de l'école : en-tête administratif, ministère, signataires, titres,
options, formules des lettres, seuils du tableau d'honneur. Chaque option a un aperçu.

## 10. Plusieurs postes, sauvegarde, mise à niveau

- Les numéros se prennent sur le poste principal, sous verrou, comme aujourd'hui ; un poste
  relié qui délivre un acte reçoit le PDF par son contenu.
- La vérification marche depuis un poste relié.
- Les actes et les procès-verbaux entrent dans la sauvegarde ordinaire et dans l'export
  intégral.
- Mise à niveau : une migration crée les tables et reprend les certificats de transfert au
  registre des actes. Rien d'autre ne change pour l'école tant qu'elle n'imprime pas un document
  nouveau.

## 11. Droits

| Document | Imprimer | Annuler |
|---|---|---|
| Certificat de scolarité, attestation d'inscription, carte, attestation de passage et de réussite | `documents:deliver` | `documents:cancel` (nouvelle) |
| Exeat (certificat de transfert), certificat de radiation | `enrollments:transfer` | `documents:cancel` |
| Lettre d'avertissement ou de blâme | `sanctions:create` | d'office à l'annulation de la sanction ; sinon `documents:cancel` |
| Tableau d'honneur, diplôme | `reports:generate` | — |
| Liste de classe, liste d'appel | `students:read` (portée par classe) | — |
| Registre matricule | `students:read` et `documents:deliver` | — |
| Procès-verbal du conseil de classe | saisir, arrêter, rouvrir : `class_council:manage` (nouvelle) ; lire : `reports:generate` | rouvrir |
| Attestation et certificat de travail | `employees:read` et `documents:deliver` | `documents:cancel` |
| Attestation de salaire | `contracts:read` | `documents:cancel` |
| Réimprimer un acte | `documents:reprint` | — |
| Vérifier un document | `documents:verify` (nouvelle) | — |
| Réglages des documents | `templates:manage` | — |

Trois permissions nouvelles, entrées au catalogue avec l'étape qui s'en sert :
- `documents:cancel` : annuler un acte — administrateur, direction ;
- `documents:verify` : vérifier un document — administrateur, direction, directeur des études,
  secrétaire, caissier, surveillant général ;
- `class_council:manage` : tenir le conseil de classe — administrateur, direction, directeur
  des études.

La matrice `docs/matrice-permissions.md` se régénère par son essai.

## 12. Ordre de réalisation

Étapes courtes. Plusieurs étapes par session, un commit par étape, tout vert avant de pousser.
Une seule session compile à la fois. Les étapes 3 à 6 ne dépendent que des étapes 1 et 2 et
peuvent se faire dans n'importe quel ordre.

1. **Socle commun.** En-tête administratif, signataire, « Fait à », formules et accords dans
   `entete.typ` ; réglages des documents ; types de numéros ; table `acte_delivre` et reprise
   des certificats de transfert ; annulation et remplacement ; permissions `documents:cancel` et
   `documents:verify`. Essai : un acte d'essai numéroté, réimprimé au bit près, annulé.
2. **Vérifier un document et Documents délivrés.** Écran de vérification (référence, numéro,
   douchette), état valide, annulé, remplacé, inconnu ; registre des actes ; journal.
3. **Élève, situation de l'année.** Certificat de scolarité (et année passée), attestation
   d'inscription ; bouton Documents de la fiche ; impression en série pour une classe.
4. **Départs et fin d'année.** Certificat de transfert version 2 avec le titre « Exeat »,
   certificat de radiation, annulation d'office à la révocation ; attestation de passage,
   attestation de réussite.
5. **Listes et registre.** Liste de classe, liste d'appel, registre matricule ; exports Excel ;
   portée de l'enseignant.
6. **Carte d'élève.** Planche de dix cartes, photo, QR, vérification de l'inscription du jour.
7. **Discipline.** Lettre d'avertissement ou de blâme, coupon, annulation d'office.
8. **Tableau d'honneur.** Seuils et conditions, affiche, diplôme, distinction au bulletin
   (nouvelle version du modèle).
9. **Conseil de classe.** Tables, écran, calcul, arrêt et réouverture, procès-verbal ;
   permission `class_council:manage`. Dépend de l'étape 8 (distinctions).
10. **Personnel.** Attestation de travail, certificat de travail, attestation de salaire depuis
    la paie ou l'engagement ; droits.
11. **Livraison.** École d'essai complétée (un acte de chaque sorte, dont un annulé) ; outil de
    démonstration ; essai indépendant des mentions (§0.4, point 7) ; essai « école vide » et
    « données manquantes » ; Playwright global ; manuel ; notes de version.

## 13. Essais

- **Mentions** : pour chaque document, un essai relit le PDF produit (texte extrait) et compare
  chaque mention à la base. Le même essai sur une école aux données trouées vérifie qu'aucune
  phrase n'apparaît sans sa donnée et qu'aucun mot de manque n'apparaît (« renseign »,
  « compléter », « manqu », « impayé »).
- **Proposition** : un essai par propriété — pour toute inscription, l'attestation de passage
  est proposée si et seulement si la décision vaut « admis » ; le certificat de travail si et
  seulement si le membre est parti ; etc.
- **Numérotation** : deux postes, cent actes chacun en même temps, aucun doublon, aucun trou.
- **Réimpression** : empreinte SHA-256 identique, après un changement de classe et de nom.
- **Cohérence avec le bulletin** : tableau d'honneur et procès-verbal contre les bulletins
  publiés, élève par élève.
- **Listes** : effectif de la liste à une date égal au décompte de la phase 9 à la même date.
- **Droits** : chaque commande refusée sans sa permission.

## 14. Questions au porteur

Chacune a une recommandation. Le porteur a validé toutes les recommandations le 20 novembre
2026 : la phase se code telle que décrite ici.

### T-172 — Les documents qu'une école délivre le plus

Parmi les onze, lesquels le secrétariat délivre-t-il chaque semaine ? Faut-il les faire en
premier ?

*Recommandation* : garder l'ordre du §12, qui commence par le certificat de scolarité,
l'attestation d'inscription et la vérification : ce sont les plus demandés et ceux qu'on
falsifie le plus.

### T-173 — L'en-tête administratif

Les certificats de votre école portent-ils l'en-tête « Ministère… / Académie… / CAP… » à gauche et
« République du Mali / Un Peuple – Un But – Une Foi » à droite ? Une école privée garde-t-elle cet
en-tête, ou son papier à en-tête seul ?

*Recommandation* : l'en-tête administratif sur les actes, avec le CAP au fondamental et au
jardin seulement ; l'école peut le couper document par document ; l'intitulé du ministère se
règle.

### T-174 — La vérification par un tiers

Une autre école ou une administration qui reçoit un certificat doit-elle pouvoir le vérifier
seule, en ligne, sans appeler l'école ?

*Recommandation* : pas dans cette phase. Le QR garde la référence seule, sans donnée
personnelle ; la vérification se fait à l'école, ou au téléphone en dictant la référence. La
vérification en ligne viendra avec le service en ligne, par une nouvelle version des modèles.

### T-175 — Cachet et signature

Faut-il imprimer une image du cachet et de la signature du directeur ?

*Recommandation* : non. Un espace vide pour le cachet, la qualité et le nom du signataire ; la
signature et le cachet se mettent à la main. Une image imprimée se copie trop facilement et
ôterait sa valeur au papier.

### T-176 — « Duplicata » à la réimpression

Un acte réimprimé doit-il porter « Duplicata » ?

*Recommandation* : non par défaut, pour que la réimpression soit identique au bit près et que la
vérification la reconnaisse ; l'école peut l'allumer dans ses réglages. Pour un papier perdu,
« Délivrer de nouveau » donne un nouvel acte numéroté.

### T-177 — La carte d'élève

Papier A4 à découper et plastifier, ou cartes en PVC sur une imprimante à cartes ? Recto seul ou
recto verso ?

*Recommandation* : papier A4, dix cartes par feuille, recto seul, plastifiées par l'école. Une
imprimante à cartes est rare et chère ; un modèle pour elle pourra venir plus tard sans rien
changer aux données.

### T-178 — Filiation et téléphone

Le certificat de scolarité doit-il porter « fils (fille) de … et de … » ? La carte doit-elle
porter le téléphone d'un parent ?

*Recommandation* : la filiation sur le certificat, allumée par défaut, imprimée seulement quand
le père ou la mère est connu ; le téléphone sur la carte, éteint par défaut (une carte perdue ne
doit pas donner le numéro d'un parent).

### T-179 — L'attestation de réussite à un examen

L'école délivre-t-elle elle-même une attestation de réussite au DEF ou au baccalauréat, en
attendant le diplôme ? Ou est-ce seulement l'académie qui la délivre ?

*Recommandation* : GESCO imprime l'attestation de l'école, qui dit « a été déclaré(e) admis(e)
… session de … », signée du directeur ; elle n'imite pas le document officiel. Si les académies
refusent qu'une école l'imprime, on ne garde que l'attestation de passage.

### T-180 — Exeat et certificat de transfert

Dans vos écoles, l'exeat et le certificat de transfert sont-ils le même papier, ou deux papiers
remis ensemble ?

*Recommandation* : le même acte, dont l'école choisit le titre ; un départ n'a qu'un numéro.
Si ce sont deux papiers, l'exeat devient un acte à part, numéroté `EXE`, au même écran.

### T-181 — Le motif sur le certificat de radiation

Le certificat de radiation doit-il dire pourquoi l'élève est parti ?

*Recommandation* : non par défaut ; l'école peut imprimer le motif d'un abandon (« départ de
la famille ») ; jamais celui d'une exclusion, qui suivrait l'élève dans sa nouvelle école.

### T-182 — Moyenne et rang sur l'attestation de passage

L'attestation de passage doit-elle porter la moyenne annuelle et le rang ?

*Recommandation* : la moyenne oui, le rang non, l'un et l'autre réglables ; imprimés seulement
quand le bulletin de fin d'année est publié.

### T-183 — Les seuils du tableau d'honneur

Quels seuils vos écoles appliquent-elles : tableau d'honneur à 12, encouragements à 14,
félicitations à 16 sur 20 ? Les mêmes au fondamental (notes sur 10) ? Un avertissement ou un blâme
retire-t-il la distinction ?

*Recommandation* : ces trois seuils livrés, en pour cent du barème, réglables par cycle ; un
avertissement ou un blâme de la période retire la distinction.

### T-184 — Le tableau d'honneur au bulletin et sans lauréat

La distinction doit-elle s'imprimer sur le bulletin ? Si aucun élève n'a de distinction, faut-il
imprimer quand même l'affiche ?

*Recommandation* : oui sur le bulletin, à côté des sanctions du conseil, pour les bulletins
publiés après le réglage ; l'affiche sans lauréat s'imprime avec le seul titre, sans phrase qui
le fasse remarquer.

### T-185 — Qui voit le procès-verbal du conseil de classe

Les enseignants de la classe doivent-ils pouvoir lire et imprimer le procès-verbal arrêté ?

*Recommandation* : oui, en lecture, pour leurs classes ; la saisie reste à la direction et au
directeur des études.

### T-186 — Le certificat de travail

Le certificat de travail doit-il porter « libre de tout engagement » ? Et l'attestation de salaire,
le salaire net payé ou la rémunération convenue ?

*Recommandation* : « libre de tout engagement » au choix de l'école, éteint par défaut ; aucune
appréciation, aucun motif de départ. L'attestation de salaire donne le brut et le net des
bulletins de paie validés quand ils existent, sinon la rémunération convenue, dite comme telle.

## 15. Journal des révisions

- v1.0, 20 novembre 2026 : première rédaction, d'après les onze documents retenus par le
  porteur le 20 novembre 2026 et le relevé du dépôt du même jour.
- v1.1, 20 novembre 2026 : recommandations de T-172 à T-186 validées par le porteur ; ADR des
  actes délivrés accepté.
