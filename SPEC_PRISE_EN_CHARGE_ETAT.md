# SPEC_PRISE_EN_CHARGE_ETAT.md — Élèves pris en charge par l'État

**Projet** : ERP de gestion scolaire — Tauri 2 + Rust + React/TypeScript
**Objet** : la scolarité d'un élève orienté par l'État est due par l'État, pas par le parent ; le
parent ne doit que le reste ; les versements de l'État, souvent tardifs et groupés, se répartissent
sur les élèves ; l'école suit ce que l'État lui doit et l'imprime pour le réclamer ; version 2.0.0
**Statut** : v1.2 — **prête à coder** le 18 novembre 2026 : T-164 à T-171 toutes tranchées (§14),
T-171 par le porteur (« pas question de frais d'inscription pour l'État »), T-164 par une logique
souple après recherche (le porteur : « regarde sur Internet combien l'État paie pour un élève, ou
une logique flexible »), T-165 à T-170 dans la spécification, le porteur ayant laissé le choix
(« pour la gestion de l'État, débrouille pour ne pas fausser ») ; v1.1 et v1.0 proposées le même
jour
**Décisions qui la fondent** : porteur du projet, 18 novembre 2026, à la question « pour un élève
orienté par l'État, qui paie la scolarité ? » : « L'État, pas le parent » (`docs/orchestration.md`) ;
puis, le même jour : « pas d'inscription et la cantine est le parent s'il l'école possède une et pour
la gestion de l'État débrouille pour ne pas fausser » — un élève pris en charge ne paie pas de frais
d'inscription, la cantine reste due par le parent si l'école en a une, et le reste de la gestion de
l'État est laissé au choix le plus simple qui ne fausse aucun chiffre. GESCO n'est encore installé
dans aucune école : aucune donnée existante à convertir, aucun rôle livré à faire monter
**Documents parents** : `SPEC_PHASE_9.md` §8.2 (case « Prise en charge par l'État », migration
`0056_complements_rapports`), `SPEC_PHASE_3.md` (§4 facture et échéancier, §5 remises, §7
encaissement et imputation, §9 impayés), `SPEC_PHASE_5.md` (relances et consultation par SMS),
`SPEC_PHASE_8.md` (§5 comptes, §7 journal et budget), `SPEC_VERSION_1_1.md` (autres recettes,
migration `0038_recettes`), migration `0053_changement_de_classe` (ajustements et avoirs),
`SPEC_REPRISE_AVANT_GESCO.md` v1.2 (codée juste avant, §11 de ce document),
`SPEC_LOTS_C_D.md` (soldes vus par les parents), principe du porteur « la flexibilité sans
incohérence » (14 novembre 2026)
**ADR proposé avec cette spécification** : `docs/adr/2026-11-18-prise-en-charge-par-l-etat.md`
**Ordre** : fin du lot 4 de la phase 12 → code de la reprise avant GESCO → **code de cette
spécification** → lot 3 (quiz) → lot 5 (devoirs) → tous les essais, une seule fois → installeur
2.0.0 (école d'essai au deuxième mois de l'année)
**Date** : 18 novembre 2026

---

## 0. Objet et critère de fin

### 0.1 Le cas à servir

Chaque année, l'État oriente des élèves admis au DEF vers des lycées et des établissements
techniques, y compris privés, et paie leur scolarité à l'école. Il paie tard (souvent après
plusieurs mois), par un seul virement pour beaucoup d'élèves, parfois pour un montant différent de
ce que l'école attendait. Pendant ce temps :

- le parent ne doit pas être relancé pour une scolarité qui ne le regarde pas ; il ne paie pas
  de frais d'inscription ; il doit le reste (la cantine si l'école en a une, la tenue, le
  transport…) et ses reçus, sa situation, ses relances et ses SMS ne parlent que de cela ;
- l'école doit savoir, élève par élève, classe par classe et en tout, ce que l'État lui doit
  encore, et pouvoir l'imprimer pour le réclamer ;
- l'argent de l'État entre une fois dans la trésorerie, dans le bon compte, au jour où il arrive.

### 0.2 Ce que l'application possède déjà (vérifié dans le code le 18 novembre 2026)

| Besoin | Existe | Où |
|---|---|---|
| Savoir qu'un élève est pris en charge | Case `inscription.prise_en_charge_etat` (lycée et technique, non par défaut), lue par les rapports officiels seulement | migration 0056 ; `gesco-db/src/depots/complements.rs` (`prises_en_charge`, `ecrire_prise_en_charge`) ; `ServiceComplements::regler_eleve` ; grille « Pour les rapports » (`PourLesRapports.tsx`) |
| Cycles concernés | `prise_en_charge_possible` : `lycee`, `technique` | `gesco-services/src/complements.rs` |
| Un seul calcul du reste à payer | `facturation::etat_echeances`, lu par l'encaissement, les remises, les ajustements, le guichet, la consultation par SMS, les messages automatiques, les documents remis, le recouvrement (impayés), les départs (Q-33) | `gesco-services/src/facturation.rs` |
| Un seul point de recalcul | `facturation::recalculer_dans`, appelé par chaque geste qui change des échéances | idem |
| Une recette hors des versements, dans un compte, sans session si ce n'est pas des espèces | `recette_tresorerie` (date, catégorie, payeur, compte, mode, référence, `session_id` facultatif, annulation motivée), comptée au livre du compte, au rapprochement, au journal de trésorerie et au budget par sa catégorie | migration 0038 ; `depots/recettes.rs` ; `journal_tresorerie.rs` |
| Rembourser un parent | Dépense « Remboursements aux parents » liée à l'inscription (`depense.inscription_id`) | `SPEC_REPRISE_AVANT_GESCO.md` §3.6 bis (codée avant celle-ci) |

Ce qui manque :

1. **La case ne change rien à l'argent** : un élève coché doit toute sa scolarité, est relancé
   par SMS et apparaît aux impayés.
2. **Rien ne dit qui doit une échéance** : toute échéance est due par le parent.
3. **Aucun geste pour recevoir l'argent de l'État** et le répartir sur des élèves.
4. **Aucun état de ce que l'État doit**.

### 0.3 Critère de fin, vérifiable

La spécification est réalisée quand ces dix assertions sont vraies, chacune démontrée par un essai
automatisé (§12) :

1. Pour un élève coché, la part de l'État (la scolarité, plus les frais que l'école lui ajoute,
   au montant attendu, §3) et les frais d'inscription (que personne ne doit, T-171) sortent du
   reste à payer du parent ; le parent ne doit que le reste, dont la cantine si l'école en a une.
2. Encaissement, reçu, situation remise au parent, relevé de compte, impayés, relances SMS,
   consultation du solde par SMS, guichet, documents remis, départ (solde exigé) et report de fin
   d'année ne lisent que la part du parent.
3. Cocher ou décocher la case, à tout moment de l'année, ne réécrit ni facture, ni ligne, ni
   échéance, ni imputation de versement ; total de la facture, journal des encaissements et
   journal de trésorerie sont identiques avant et après.
4. Ce que le parent avait déjà payé sur la part de l'État couvre d'abord le reste de sa propre
   part ; le surplus est « à rembourser », par la dépense de remboursement de la reprise ; rien
   n'est remboursé deux fois ; un parent remboursé dont la case est ensuite décochée doit de
   nouveau ce qu'on lui a rendu.
5. Un versement de l'État se répartit sur plusieurs élèves (liste à cocher ou fichier) ; reçu par
   virement ou chèque, il entre dans son compte de banque sans toucher à aucune session de
   caisse ; il entre une seule fois au livre du compte, au journal de trésorerie et au budget.
6. Un versement plus petit que l'attendu laisse un reste dû par l'État ; un versement plus grand
   laisse une part « non répartie » visible, jamais imputée au parent ; la somme répartie ne
   dépasse jamais le montant reçu.
7. L'état de ce que doit l'État (par élève, par classe, en total, échu et à venir) se lit, s'exporte
   en Excel et s'imprime ; ses totaux égalent la somme des restes de l'État des factures et des
   compléments non reçus.
8. La part de l'État est une recette à recevoir : elle n'entre au journal de trésorerie et au
   réalisé du budget qu'à la réception de l'argent ; « à recevoir de l'État » se lit à part, hors
   des totaux réalisés.
9. Toute écriture est auditée avec son événement, en durabilité critique ; rien ne se supprime.
10. Sans aucun élève coché, chaque chiffre de chaque écran et document est identique à celui
    d'avant cette spécification.

---

## 1. Périmètre

**Inclus** : effet financier de la case ; inscription non due ; montant attendu de l'État par élève
(§3) ; écart entre les frais de l'école et le montant de l'État, à la charge de l'école ; cocher,
décocher, modifier, abandonner ce que l'État ne
paiera pas, avec trace ; trop-payé du parent et son remboursement ; versements de l'État, leur
répartition à l'écran ou par fichier, leur annulation ; versement de l'État reçu avant GESCO ;
état de ce que doit l'État, écran, Excel et PDF ; effets sur les rapports de recettes, le budget et
la trésorerie ; case proposée à la réinscription (T-169).

**Exclu** :
- Toute autre forme de tiers payeur (entreprise, ONG, mairie, bourse privée) : le modèle est
  écrit pour l'État ; un second tiers demanderait une table des tiers (ADR, reconsidération).
- La colonne « pris en charge » dans l'import des élèves : la case se coche en masse par la liste
  de la classe (§6.1).
- Un SMS au parent pour lui dire que l'État paie : aucun message nouveau.
- Les bourses de l'État versées aux élèves eux-mêmes : elles ne passent pas par l'école.
- Le calcul des sommes que l'État devrait payer d'après ses propres textes : l'école saisit ou
  règle le montant attendu (§3), GESCO ne le devine pas.

---

## 2. Principes

- **Deux débiteurs pour une facture** : la facture de l'inscription reste une ; chaque échéance
  est partagée entre une **part de l'État** et une **part du parent**. Le partage se calcule à
  chaque lecture depuis la prise en charge de l'élève (R-20) : il ne s'écrit nulle part, donc la
  facture n'est jamais réécrite.
- **Le reste à payer reste le reste du parent** : `EcheanceAffichee.reste` garde son nom et devient
  la part du parent qui reste à payer ; la part de l'État a ses propres colonnes. Tous les écrans
  et documents qui lisent déjà le reste deviennent justes sans être touchés (§4.4).
- **L'argent entre une fois** : l'argent de l'État entre en trésorerie par une recette, au jour de
  sa réception ; la répartition sur les élèves ne fait que dire ce que l'État a payé pour qui. Un
  versement de l'État reçu avant GESCO est déjà dans le solde d'ouverture du compte : il se répartit
  sans recette.
- **Les versements des parents ne bougent pas** : leurs imputations sont imprimées sur les reçus ;
  ce qu'un parent avait payé sur ce qui devient la part de l'État est un trop-payé, reporté ou à
  rembourser, jamais une imputation déplacée.
- **La flexibilité sans incohérence** : rien n'est refusé à cause de l'ordre des gestes (case avant
  ou après la validation, avant ou après les versements du parent, avant ou après la reprise, avant
  ou après le versement de l'État). Seuls sont refusés les gestes qui rendraient un chiffre faux :
  répartir plus que l'argent reçu, rembourser plus que l'excédent.

---

## 3. La prise en charge d'un élève

### 3.1 Modèle

`prise_en_charge_etat` (table nouvelle, ADR) — une ligne par **version** de la prise en charge
d'une inscription :

| Colonne | Sens |
|---|---|
| `inscription_id` | L'inscription de l'année (la finance référence l'inscription, jamais l'élève) |
| `statut` | `en_cours` (case cochée), `terminee` (case décochée), `remplacee` (version ancienne) |
| `montant_attendu` | Ce que l'État doit payer pour cet élève pour l'année (§3.3), entier, ≥ 0 |
| `abandonne` | Ce que l'école renonce à attendre de l'État (§3.6), entier, entre 0 et `montant_attendu` |
| `reference` | Facultative, 60 caractères : numéro de la décision d'orientation, numéro de place au DEF |
| `motif` | Obligatoire pour `terminee` et pour un abandon |
| `remplace_id` | La version qu'elle remplace |
| `cree_le`, `cree_par`, `identifiant_poste` | Habituelles |

**Une seule version courante** (`en_cours` ou `terminee`) par inscription, garantie par un index
unique partiel `(inscription_id) WHERE statut <> 'remplacee'`. Un changement passe l'ancienne en
`remplacee` et crée la nouvelle, liée, dans une transaction ; l'audit garde avant et après. Rien
ne se supprime (déclencheur).

**La case reste la case** : `inscription.prise_en_charge_etat` vaut 1 exactement quand la version
courante est `en_cours`. Le service écrit les deux dans la même transaction ; les rapports
officiels (`complements::prises_en_charge`) continuent de lire la colonne, sans changement.

### 3.2 Qui doit quoi, par nature de frais

Décision du porteur (18 novembre 2026), sans réglage, pour un élève dont la case est cochée :

| Nature de la ligne | Due par | Dans le calcul (§3.4) |
|---|---|---|
| Scolarité | **L'État** | Échéance « à l'État » |
| Inscription | **Personne** : ni le parent ni l'État (réponse du porteur à T-171) | Échéance « non due » : tout son dû est renoncé, à la charge de l'école |
| Cantine | **Le parent**, s'il y a une ligne de cantine sur la facture (l'école qui n'a pas de cantine n'en facture pas) | Échéance ordinaire |
| Tenue, transport, examen, autre | Le parent | Échéance ordinaire |

Ainsi la seule chose que l'école règle est le montant attendu de l'État (§3.3).

**Montant attendu de l'État par niveau et série** (T-164, tranchée) : dans les réglages de l'école
(Finance), visibles seulement si l'école a une classe de lycée ou du technique, un tableau des
niveaux du lycée et du technique de l'année (et de leurs séries), une case par ligne, vide par
défaut ; vide = les frais de l'école. Aide sous le tableau : « Tout ce que l'État verse à l'école
pour un élève orienté de ce niveau (frais scolaires, matériel didactique…). Laissez vide si vous ne
le connaissez pas encore. » Table `montant_etat` (année, niveau, série facultative, montant facultatif),
ADR. Changer ce tableau ne change pas les élèves déjà cochés (chaque version de prise en charge
porte son montant) ; l'écran propose ensuite « Appliquer aux 37 élèves déjà cochés », avec l'aperçu
de ce qui change pour l'État (§6.1).

### 3.3 Montant attendu

**Ce qu'on sait de ce que paie l'État** (recherche du 18 novembre 2026, T-164) : aucun barème
public par élève n'a été trouvé. Le rapport de l'OCLEI sur les subventions aux écoles privées
(2017-2021) dit que l'État y paie, pour ses élèves orientés, des **frais scolaires**, une
**demi-bourse** (matériel didactique) et, pour certains, des **pensions alimentaires** ; ses tableaux
donnent, selon les établissements, de 145 000 F à 181 000 F environ par élève et par an, tout
compris ; la presse cite 96 000 F de frais scolaires plus 30 000 F de demi-bourse. Les états de
paiement sont établis par l'établissement, vérifiés par l'académie, à partir d'une base de données
unique du ministère. Le montant varie donc selon la filière et l'établissement, et peut dépasser la
scolarité de l'école. D'où une logique souple, sans montant livré :

À la case cochée, le montant attendu se propose ainsi :
1. le montant de `montant_etat` pour le niveau et la série de la classe, s'il est réglé ;
2. sinon, **les frais de l'école** : la scolarité de la facture après les remises et les baisses
   (son dû net) ; sans facture encore, la scolarité de la grille en vigueur pour le niveau et la
   série.

Il se corrige ensuite élève par élève ou pour une sélection (« Montant attendu : 126 000 F pour
les 12 élèves cochés »), par le titulaire de la permission (§8), par exemple d'après l'état de
paiement validé par l'académie. Il ne change pas de lui-même : un changement de classe propose le
nouveau montant si la règle en donne un autre (§5.3).

**Plus ou moins que la scolarité de l'école** :
- moins (100 000 F attendus pour 120 000 F de scolarité) : l'écart est renoncé, à la charge de
  l'école (T-165) ;
- plus (126 000 F attendus pour 100 000 F de scolarité, la demi-bourse en plus) : les 100 000 F
  de la scolarité vont sur les échéances, les 26 000 F restants sont un **complément attendu de
  l'État**, suivi avec ce qu'il doit (§9) mais hors de la facture, qui ne le facture pas et ne se
  réécrit pas. Reçu, il n'est pas « reçu en plus » : il était attendu.

### 3.4 Le partage d'une facture (calcul, R-20)

Pour une facture vivante, échéances dues rangées par date puis par rang :

- `net(e)` : dû de l'échéance après remises et baisses, comme aujourd'hui ;
- quand la version courante est `en_cours`, une échéance est **à l'État** si sa ligne est de la
  scolarité, **non due** si elle est de l'inscription (§3.2) ; on dit **couvertes** les échéances
  à l'État et non dues ; `N` = somme des `net` des échéances à l'État ; une version `terminee`
  n'a que des échéances à l'État (la scolarité), aucune non due : l'inscription redevient due par
  le parent ;
- `S` = ce que l'État a payé pour cette inscription : somme des parts actives des versements de
  l'État non annulés (§7).

**Ce que l'État doit sur la facture**, `E`, et **hors facture**, `K` (complément, §3.3) :
- version courante `en_cours` : `A = montant_attendu − abandonne` ; `E = min(A, N)` ;
  `K = A − E` ;
- version courante `terminee` (case décochée) : `E = min(S, N)`, `K = 0` — après la fin de la
  prise en charge, l'État doit exactement ce qu'il a déjà payé ; le reste revient au parent ;
- pas de prise en charge : `E = 0`, `K = 0`.

**Ce que personne ne doit**, `R` (version `en_cours` seulement), à la charge de l'école, jamais du
parent (T-165, tranchée) :
- sur les échéances à l'État, `N − E` : l'écart quand les frais de l'école dépassent le montant
  attendu, et l'abandon (§3.6) ;
- sur les échéances non dues (inscription), tout leur `net`.

**Placement** :
- `part_etat(e)` : `E` placé sur les échéances à l'État **de la plus ancienne à la plus
  récente**, chaque part au plus `net(e)` (l'État est en retard dès la première tranche qu'il ne
  paie pas) ;
- `renonce(e)` : sur les échéances à l'État, `N − E` placé **de la plus récente à la plus
  ancienne** sur ce qui reste (comme une remise) ; sur une échéance non due, `renonce(e) = net(e)` ;
- `part_parent(e) = net(e) − part_etat(e) − renonce(e)` : 0 sur une échéance couverte d'une
  version `en_cours` ; sur une échéance non couverte (cantine, tenue…), `part_parent(e) = net(e)`.

**Côté État** : `S` placé de la plus ancienne à la plus récente sur `part_etat(e)` donne
`paye_etat(e)` ; `reste_etat(e) = part_etat(e) − paye_etat(e)` ; ce qui dépasse la somme des
`part_etat` va ensuite au complément (`paye_complement`, au plus `K`) ; le reste est **reçu en
plus** pour cet élève (§7.5). **Ce que l'État doit encore** pour l'élève = somme des
`reste_etat` + `K − paye_complement`. Le complément n'entre ni dans la facture ni dans son statut.

**Côté parent** (les versements et les avoirs gardent leurs imputations, §3.5) :
- `m(e)` = versements imputés + avoirs utilisés sur l'échéance ;
- **trop-payé** de l'échéance `x(e) = max(0, m(e) − part_parent(e))` ; `X` = leur somme ;
- `B` = remboursé à la famille pour cette inscription (dépenses de remboursement payées, non
  annulées) ; `B_reprise = min(B, reprise active)` ; `B_tp = B − B_reprise` ;
- `B_tp` s'impute d'abord sur les `x(e)`, de la plus ancienne à la plus récente (`b1(e)`) ; ce qui
  dépasse `X` s'impute sur l'argent qui compte pour la part du parent (`b2(e)`), sur les
  échéances couvertes d'abord, de la plus ancienne à la plus récente, puis sur les autres : c'est
  de l'argent rendu alors qu'il était dû, **le parent le doit de nouveau** (case décochée après
  un remboursement) ;
- argent du parent qui compte pour l'échéance : `c(e) = min(m(e), part_parent(e)) − b2(e)` ;
- trop-payé disponible `T = X − somme des b1` ; reprise utilisable `U = reprise − B_reprise` ;
- reste avant report `r0(e) = part_parent(e) − c(e)` ; `T` se place de la plus ancienne à la plus
  récente sur `r0` (`t(e)`, **report du trop-payé**) ; puis la reprise sur ce qui reste (`u(e)`,
  `reprise_imputation`, calculée par `recalculer_dans` comme le prévoit la reprise) ;
- `reste(e) = r0(e) − t(e) − u(e)` : **le reste du parent** ;
- **à rembourser** = `(T − somme des t) + (U − somme des u)`.

Contrôle (essai de propriété, §12) : versements + avoirs utilisés + reprise − remboursé =
somme des `c` + somme des `t` + somme des `u` + à rembourser ; `net(e) = part_etat(e) +
renonce(e) + part_parent(e)` ; aucun montant négatif.

Sans prise en charge ni trop-payé, `part_parent = net`, `X = 0`, `B_tp = 0` : le calcul est
exactement celui d'aujourd'hui et de la reprise (assertion 10).

### 3.5 Le trop-payé du parent (T-166, tranchée)

Un parent a payé l'inscription et la 1re tranche de scolarité avant que la case soit cochée. Ses
versements gardent leurs imputations (le reçu qu'il a en main dit « Inscription », « 1re
tranche ») ; l'inscription n'est plus due et la tranche est à la charge de l'État. Ce qu'il a payé
devient un **trop-payé** :

1. il couvre d'abord ce qui reste de sa propre part (cantine, tenue…), plus ancienne échéance
   d'abord : c'est le report, calculé, sans écriture ;
2. le surplus est **à rembourser au parent**, dans la même ligne « À rembourser au parent » que le
   trop-payé avant GESCO ; le remboursement est la dépense « Remboursements aux parents » liée à
   l'inscription (`SPEC_REPRISE_AVANT_GESCO.md` §3.6 bis), avec ses règles : seuil d'accord,
   espèces par la session de qui paie ou par un compte, annulation motivée ;
3. l'école rembourse quand elle veut : avant ou après que l'État a payé. GESCO montre le montant,
   n'impose aucun moment ;
4. une dépense liée qui dépasse ce qui est à rembourser est refusée
   (`remboursement.superieur_a_l_excedent`, avec le montant possible) : rien n'est remboursé deux
   fois ;
5. un frais ajouté ensuite (cantine oubliée) absorbe d'abord le trop-payé ;
6. la case décochée après un remboursement : le parent doit de nouveau l'inscription, la
   scolarité que l'État n'a pas payée et ce qui lui a été rendu (`b2`, §3.4) ; l'aperçu du
   décochage le dit avant (« Le parent devra 135 000 F, dont 33 000 F déjà remboursés »).
7. l'annulation d'un versement du parent qui laisserait le remboursé plus grand que tout l'argent
   du parent est refusée (`versement.deja_rembourse`, avec le montant en cause) : on annule
   d'abord la dépense de remboursement. C'est le seul refus nouveau côté parent, parce que le
   chiffre deviendrait faux.

### 3.6 Abandonner ce que l'État ne paiera pas

L'État a payé moins que l'attendu et ne paiera pas le reste (règlement définitif de l'année) :
« Ne plus attendre » sur un élève ou une sélection, montant proposé = ce que l'État doit encore,
modifiable vers le bas, motif obligatoire. Nouvelle version avec `abandonne` augmenté. L'abandon
est à la charge de l'école, **jamais du parent**. Il se défait
par une nouvelle version (abandon ramené à 0, motif).

### 3.7 Gestes et règles

| Geste | Effet, dans une seule transaction (durabilité critique) |
|---|---|
| Cocher (pas de version courante, ou `terminee`) | Version `en_cours` (montant proposé §3.3) ; colonne de la case à 1 ; `recalculer_dans` sur la facture s'il y en a une ; audit ; `PriseEnChargeEtatCochee` |
| Modifier (montant attendu, référence) | Ancienne version `remplacee`, nouvelle liée ; recalcul ; audit avant et après ; `PriseEnChargeEtatModifiee` |
| Abandonner (§3.6) | Nouvelle version, `abandonne` augmenté, motif ; recalcul ; audit ; `PriseEnChargeEtatModifiee` |
| Décocher | Motif obligatoire (proposé : « Pas orienté par l'État ») ; nouvelle version `terminee` ; colonne à 0 ; recalcul ; audit ; `PriseEnChargeEtatDecochee` |
| Les mêmes pour une sélection | Une transaction pour toute la sélection, un audit par élève et un pour le lot |

Règles :
- La case n'existe que pour une inscription d'une classe de lycée ou du technique
  (`prise_en_charge_possible`) ; ailleurs elle n'est pas montrée.
- **Inscription sans facture** (brouillon, ou école sans grille) : la version naît quand même ; la
  facture créée à la validation ou au rattrapage porte d'emblée le partage (sa création appelle
  déjà `recalculer_dans` ou le calcul du §3.4 s'applique à la lecture).
- **Année close** : cocher et décocher sont refusés
  (`prise_en_charge.annee_close`) : le parent de l'année close a été soldé ou reporté. **Abandonner
  et corriger le montant attendu restent permis** : ils ne changent que ce que doit l'État, que
  l'école suit encore après la clôture.
- Chaque commande d'écriture porte la date de lecture de l'écran et passe par
  `crate::fraicheur::verifier` sur la version courante et la facture.

---

## 4. Effet sur le calcul et sur les lecteurs du reste

### 4.1 `etat_echeances` et les types affichés

`EcheanceAffichee` gagne : `part_etat`, `renonce`, `part_parent`, `paye_etat`, `reste_etat`. Le
champ `paye` devient l'argent du parent qui compte pour l'échéance (`c + t + u`) ; `reste` le reste
du parent. `net` ne change pas de sens.

`FactureAffichee` gagne : `part_etat`, `renonce`, `paye_etat`, `reste_etat`, `complement_etat`
(`K`), `paye_complement`, `recu_en_plus_etat`,
`trop_paye` (X), `trop_paye_reporte` (somme des `t`) ; `a_rembourser` (créé par la reprise) devient
le total du §3.4 ; `prise_en_charge` : la version courante (statut, montant attendu, abandon,
référence), ou rien.

### 4.2 `recalculer_dans`

- calcule le partage, le côté État, le report du trop-payé, puis réimpute la reprise **sur la part
  du parent seulement** (sur `r0 − t`, au lieu des restes sans reprise) ;
- le statut de la facture se déduit du payé total et du reste total : **soldée** quand le reste du
  parent et le reste de l'État sont nuls tous deux ; un élève dont le parent a tout payé mais
  dont l'État doit encore a une facture « partiellement réglée » ;
- tout geste qui change une donnée du §3.4 l'appelle : les gestes de la finance qui l'appellent
  déjà, plus cocher, modifier, abandonner, décocher, enregistrer, répartir et annuler un versement
  de l'État, payer et annuler une dépense de remboursement ;
- sur une année close, il est appelé par les seuls gestes permis (§3.7, §7.3) : il n'y change que
  le côté de l'État et le statut de la facture.

### 4.3 Encaissement

- L'imputation proposée et la répartition manuelle ne portent que sur la part du parent (elles
  lisent déjà `reste`) ; un versement plus grand que le reste du parent est refusé comme aujourd'hui
  (`versement.superieur_au_reste`) ;
- l'écran Caisse, pour un élève coché, ajoute sous le reste une ligne « Scolarité prise en charge
  par l'État » : le caissier ne la réclame pas ;
- le reçu porte le reste du parent après versement ; il ne parle pas de l'État ;
- l'encaissement pour plusieurs enfants (T-15) : même chose, enfant par enfant.

### 4.4 Lecteurs du reste, devenus justes sans changement

Ils lisent `EcheanceAffichee.reste`, qui est désormais le reste du parent :

| Lecteur | Où | Effet |
|---|---|---|
| Impayés (UC-05) | `recouvrement.rs` | Le parent seulement ; l'élève dont seul l'État est en retard n'y est pas |
| Relances SMS d'impayé | `messages_auto.rs` | Total échu du parent ; une échéance dont la part du parent est payée ne relance plus |
| Consultation du solde par SMS | `consultation.rs` | Solde du parent |
| Guichet | `guichet.rs` | Idem |
| Documents remis | `remises_documents.rs` | « Ce que la famille doit d'échu » : le parent seulement ; un bulletin n'est jamais retenu parce que l'État est en retard |
| Départ, solde exigé (Q-33) | `departs.rs` (`reste_echu`) | Le parent seulement |
| Remises, ajustements | `remises.rs`, `ajustements.rs` | Voir §5 |
| Report des impayés de fin d'année | fin d'année | Le parent seulement ; ce que doit l'État reste sur l'année close (§9.4) |

Deux lecteurs à toucher :
- **liste des abonnés de la cantine** (`gesco-db/src/depots/cantine.rs`), seule requête qui
  calcule un payé en SQL : elle passe par `etat_echeances` pour les inscriptions qui ont une prise
  en charge ou un remboursement (le report du trop-payé n'est pas stocké) ; l'essai du §12 qui
  parcourt les requêtes l'exige ;
- **portail des parents** (`SPEC_LOTS_C_D.md`, pas encore codé) : la vue envoyée prend `reste`,
  donc la part du parent ; la spécification des lots C et D reçoit une ligne de renvoi quand elle
  sera codée.

---

## 5. Effet sur les autres gestes de la finance

### 5.1 Remise

Règle inchangée (dernières échéances d'abord, jamais plus que le reste). Sur la scolarité d'un
élève coché, la borne est le reste de l'État de la ligne plus ce qui en est renoncé (une remise ne
descend pas sous ce que l'État a déjà payé). Le partage se recalcule : la remise réduit d'abord
l'écart renoncé, puis la part de l'État. L'aperçu de la remise le dit. Une remise sur l'inscription
d'un élève coché ne change rien tant que la case reste cochée (l'inscription n'est pas due).

### 5.2 Changement de plan

Le verrou `echeancier.plan_verrouille` (versements et avoirs imputés sur la scolarité) ne regarde
pas l'argent de l'État : le placement de l'État se recalcule sur les nouvelles échéances. Un élève
dont la scolarité est toute à l'État peut donc changer de plan ; cela ne change rien pour lui.

### 5.3 Changement de classe

L'ajustement de la facture (hausse, baisse, avoir) se fait comme aujourd'hui ; le partage se
recalcule sur les nouvelles lignes. Si la nouvelle classe n'est plus du lycée ni du technique,
l'aperçu propose de décocher la case ; si `montant_etat` donne un autre montant pour la nouvelle
classe, il propose de l'appliquer. Rien ne se fait sans le geste de la direction.

### 5.4 Abandon, transfert, exclusion

Les échéances à venir sans paiement s'annulent comme aujourd'hui (une échéance « payée » est celle
dont le parent ou l'État a payé une part) ; la part de l'État qu'elles portaient disparaît avec
elles ; ce que l'État avait payé au-delà devient « reçu en plus » (§7.5), à répartir ou à laisser.

### 5.5 Frais ajouté après coup (reprise §3.7)

Une ligne de scolarité ajoutée à la facture entre dans `N` ; une autre (cantine oubliée, tenue)
est due par le parent et absorbe d'abord son trop-payé.

---

## 6. Écrans

Entrée **Finance → Prise en charge par l'État**, montrée si l'année a une classe de lycée ou du
technique, ou si une prise en charge existe ; trois onglets.

### 6.1 « Élèves »

- Année (courante par défaut), classe (toutes les classes du lycée et du technique), recherche par
  nom ou matricule (`Tableau`).
- Une ligne par inscription de ces classes : case **Pris en charge**, classe, élève, matricule,
  référence, **attendu de l'État**, **reçu de l'État**, **reste dû par l'État** (dont échu),
  **reste du parent**, **à rembourser au parent**.
- Cocher une case ouvre l'aperçu (montant proposé, ce que le parent ne doit plus, trop-payé et à
  rembourser) ; Entrée confirme. Cocher une sélection (« Cocher les 25 élèves affichés ») : un
  aperçu groupé, une transaction.
- Décocher demande le motif et montre ce que le parent devra.
- Montant attendu modifiable dans la ligne ; « Appliquer un montant à la sélection » ; « Ne plus
  attendre » (§3.6) pour une sélection.
- En tête : nombre d'élèves pris en charge, total attendu, total reçu, total dû par l'État, total à
  rembourser aux parents.
- Export Excel et CSV (`Tableau`, R-41). Année close : cases grisées, montant et abandon
  modifiables.

### 6.2 « Versements de l'État »

Liste des versements de l'année : date, référence, libellé, compte, montant, réparti, **non
réparti**, nombre d'élèves, état. Bouton « Nouveau versement de l'État » (§7.2). Une ligne s'ouvre
sur sa répartition : modifier (§7.3), compléter, annuler.

### 6.3 « Ce que doit l'État »

L'état du §9.2 à l'écran : filtres année, classe, série ; regroupé par classe ; boutons
« Imprimer » et « Excel ».

### 6.4 Ailleurs

- **Fiche de l'élève, section « Frais »** (`FraisEleve.tsx`) : pour un élève coché, une ligne
  « Pris en charge par l'État » (référence, montant attendu) avec, pour le titulaire de la
  permission, « Modifier », « Décocher », « Ne plus attendre » et l'historique des versions ; dans
  le tableau des échéances, deux colonnes **Part de l'État** et **Reste de l'État** ; le reste
  affiché en grand reste celui du parent. Un bloc « Ce que doit l'État » : attendu, reçu (avec
  chaque versement de l'État : date, référence, montant), reste, reçu en plus. S'il y a un
  trop-payé : « Payé par le parent sur la part de l'État : 48 000 F », « Reporté sur ses autres
  frais », « À rembourser au parent » et « Rembourser ».
- **Grille « Pour les rapports »** (`PourLesRapports.tsx`) : la même case, qui appelle le même
  service ; pour qui n'a pas la permission (§8), la case se lit sans se modifier.
- **Assistant d'inscription, étape « Frais »** : pour une classe de lycée ou du technique, la case
  « Pris en charge par l'État » (non cochée), montrée au seul titulaire de la permission ; cochée,
  la facture naît partagée et le premier versement proposé ne porte que la part du parent (pas
  d'inscription ; cantine s'il l'a cochée).
- **Écran Caisse** : la ligne du §4.3.
- **Relevé de compte** (`releve_compte`, remis au parent) : les échéances avec la part du parent
  seulement, et une mention « Scolarité prise en charge par l'État » sans montant de l'État ni
  retard de l'État ; s'il y a lieu, le trop-payé reporté, remboursé et à rembourser.

Aucun texte affiché ne cite une question, une spécification ou une version ; aucun nom réel dans
les exemples, les essais ni les captures.

---

## 7. Le versement de l'État

### 7.1 Modèle

`versement_etat` (table nouvelle, ADR) : année scolaire concernée, date de réception, montant
(entier > 0), référence (numéro de l'ordre de virement ou du chèque), libellé, payeur (« État »
par défaut, modifiable), `recette_id` (la recette de trésorerie qui porte l'argent), `avant_gesco`
(0 ou 1 ; 1 ⇔ pas de recette), statut (`valide`, `annule`), motif d'annulation, colonnes
habituelles.

`versement_etat_part` (table nouvelle) : versement, inscription, montant (> 0), origine (`ecran`,
`import`), statut (`active`, `annulee`), colonnes habituelles. La somme des parts actives d'un
versement est au plus son montant (vérifié par le service, dans la transaction).

Rien ne se supprime (déclencheurs). La catégorie de recette livrée **« Versements de l'État »**
(code stable `versement_etat`) s'ajoute comme les autres catégories livrées, à l'ouverture d'une
base qui ne l'a pas ; l'école la renomme, ne la supprime pas.

### 7.2 Enregistrer

« Nouveau versement de l'État » (titulaire de `treasury:manage`) :
1. **L'argent** : date de réception (date du relevé ou de l'avis de crédit ; passée possible),
   montant, mode (**virement** par défaut ; chèque ; espèces), compte (le compte du mode, choisi par
   l'école, à défaut la première banque active — `SPEC_PHASE_8.md` §5), référence (exigée hors
   espèces), libellé (« Scolarité des élèves orientés — 1er versement »), année concernée (l'année
   courante par défaut ; une année précédente pour un arriéré).
2. **La répartition** : la liste des élèves pris en charge de l'année concernée, avec attendu,
   déjà reçu, reste dû par l'État ; cases à cocher ; montant prérempli = reste dû par l'État ;
   « Même montant pour chaque élève coché : … » ; en pied : montant reçu, réparti, **non réparti**.
   Ou l'import d'un fichier (§7.4).
3. **Enregistrer**, une transaction : `versement_etat`, la recette (§7.6), les parts, le
   recalcul de chaque facture touchée, l'audit, `VersementEtatEnregistre`.

Règles :
- **Virement ou chèque** : la recette va au compte de banque, **sans session de caisse**
  (`session_id` vide) : la caisse du jour ne bouge pas.
- **Espèces** : la recette entre dans la session ouverte de qui l'enregistre, comme toute recette en
  espèces (`entree_recette`) ; sans session ouverte, la règle des recettes s'applique.
- **Réparti plus que reçu** : refusé (`versement_etat.reparti_superieur`), avec le montant possible.
- **Réparti moins que reçu** : accepté ; le reste est « non réparti », visible sur le versement et
  dans l'état (§9), à répartir plus tard (§7.3).
- **Part plus grande que ce que l'État doit pour l'élève** : acceptée, avec l'avertissement « Dont
  10 000 F de plus que l'attendu : corriger l'attendu ? » et un bouton qui porte l'attendu à la
  somme reçue ; sinon la différence est « reçu en plus » pour l'élève (§7.5).
- **Élève non coché** dans la liste : il ne s'y trouve pas ; l'import (§7.4) le signale ; la case se
  coche d'abord, ou la ligne reste non répartie.
- Une répartition **sans aucun élève** est permise : tout est non réparti (l'école sait que l'argent
  est arrivé, pas encore pour qui).

### 7.3 Modifier, compléter, annuler

- **Modifier la répartition** : les parts actives du versement passent `annulee`, les nouvelles
  naissent, dans une transaction ; la recette ne bouge pas ; recalcul des factures touchées ;
  audit avant et après ; `VersementEtatReparti`. Compléter = la même chose avec des parts en plus.
- **Année close** : enregistrer et répartir un versement de l'État sur une année close est
  **permis** : l'argent arrive aujourd'hui (la recette est datée de sa réception), et seul le côté
  de l'État des factures de l'année close change. C'est l'exception voulue : l'État paie souvent
  l'année d'avant.
- **Annuler** (motif obligatoire) : le versement passe `annule`, ses parts `annulee`, sa recette est
  annulée par le geste des recettes (une recette en espèces repasse par la session comme
  aujourd'hui) ; recalcul ; audit ; `VersementEtatAnnule`. Annuler la recette depuis la trésorerie
  annule le versement de l'État de la même façon.

### 7.4 Répartition par fichier

Même façon de faire que l'import des reprises (`SPEC_REPRISE_AVANT_GESCO.md` §5) :
- **modèle** prérempli : les élèves pris en charge de l'année (matricule, nom, prénoms, classe,
  série, référence, attendu, reste dû par l'État) et une colonne « Montant versé » vide ;
- **lecture** par `import_eleves::lire_fichier` (xlsx, xls, ods, csv) ; l'élève se retrouve par le
  matricule, à défaut par la référence d'orientation, à défaut par nom, prénoms et classe
  (avertissement) ; montants « 120000 », « 120 000 », « 120 000 F », « 120.000 » ;
- **aperçu** ligne par ligne : élève introuvable, plusieurs élèves possibles, élève présent deux
  fois (erreurs) ; élève non coché (erreur : « Cet élève n'est pas coché pris en charge ») ; montant
  supérieur au reste dû par l'État (avertissement) ; case vide (ignorée) ; totaux : réparti, non
  réparti, et refus si le total dépasse le montant reçu ;
- **écriture** des lignes valides et en avertissement en une transaction, avec la sauvegarde
  automatique d'avant (R-48), un audit et `ImportRepartitionEtatApplique` ; export des lignes
  refusées.

### 7.5 Reçu en plus, non réparti

Deux sommes sont à l'école sans être dues par personne :
- **non réparti** d'un versement : reçu, pas encore attribué ;
- **reçu en plus** d'un élève : ce que l'État a payé pour lui au-delà de sa part (attendu trop bas,
  case décochée, départ).

Décision (T-168, tranchée) : elles restent visibles, à part, sur le versement et dans l'état de ce que
doit l'État (« Reçu de l'État à répartir : 120 000 F »), jusqu'à ce que l'école les répartisse
(corriger l'attendu, répartir sur un autre élève) ou les laisse. **Elles ne sont jamais imputées au
parent** et ne changent pas la trésorerie : l'argent y est déjà, une fois, par la recette. Si
l'école doit rendre de l'argent à l'État, c'est une dépense ordinaire.

### 7.6 La recette

`versement_etat_enregistrer` crée la recette par le même code que `recette_saisir` (date, catégorie
« Versements de l'État », payeur, libellé, montant, compte, mode, référence, session si espèces)
et la lie (`versement_etat.recette_id`). Ainsi, sans aucun changement de ces écrans : le livre du
compte, le rapprochement (coche contre le relevé), le solde, le journal de trésorerie du mois et le
budget comptent l'argent de l'État **une fois**, au jour de sa réception. La liste des recettes
montre la ligne avec « Répartition » qui ouvre le versement de l'État.

### 7.7 Versement de l'État reçu avant GESCO

Croisement avec la reprise : un versement reçu avant l'installation est déjà dans le solde
d'ouverture du compte. Case « Reçu avant GESCO (déjà dans le solde d'ouverture) » dans le
formulaire : **aucune recette**, aucun compte, aucune session ; la date affichée est l'arrêt avant
GESCO de l'année (`SPEC_REPRISE_AVANT_GESCO.md` §3.2) ; la répartition se fait comme pour un autre
versement. Réservé au titulaire de `invoices:discount` en plus de `treasury:manage` (comme la
reprise : un règlement sans argent reçu dans GESCO).

---

## 8. Droits

Décision (T-167, tranchée) : la case change ce que doit un parent sans argent reçu, comme une
remise accordée ou une reprise. **`invoices:discount`** (Administrateur, Directeur, Comptable) pour
cocher, décocher, modifier, abandonner. Pas de permission nouvelle. Le Secrétaire voit la case sans
la changer, y compris dans la grille « Pour les rapports » (GESCO n'étant installé nulle part,
aucune habitude n'est cassée).

| Commande | Permission | Effet |
|---|---|---|
| `prises_en_charge_lister` | `invoices:issue` | §6.1 : lignes, totaux |
| `prise_en_charge_apercu` | `invoices:discount` | Ce que changerait un geste, pour un élève ou une sélection |
| `prise_en_charge_regler` | `invoices:discount` | Cocher, modifier, décocher, pour un élève ou une sélection (§3.7) |
| `prise_en_charge_abandonner` | `invoices:discount` | §3.6 |
| `prise_en_charge_historique` | `students:read` | Versions d'une inscription |
| `montants_etat_lire` | `students:read` | §3.2 |
| `montants_etat_regler` | `fees:manage` | §3.2 |
| `versements_etat_lister` | `treasury:read` | §6.2 |
| `versement_etat_enregistrer` | `treasury:manage` (+ `invoices:discount` si avant GESCO) | §7.2, §7.7 |
| `versement_etat_repartir` | `treasury:manage` | §7.3 |
| `versement_etat_annuler` | `treasury:manage` | §7.3 |
| `versement_etat_modele`, `versement_etat_analyser_import`, `versement_etat_importer` | `treasury:manage` | §7.4 |
| `du_etat_lister` | `invoices:issue` | §9.2 à l'écran |
| `du_etat_produire` | `invoices:issue` | PDF et Excel du §9.2 |
| `complement_eleve_regler` | inchangée (`students:update`) pour l'année de recrutement ; le champ de la case exige `invoices:discount` et passe par `prise_en_charge_regler` | §6.4 |
| `depense_saisir`, `depense_payer`, `depense_annuler` | inchangées | Le remboursement accepte le trop-payé du §3.5 dans « à rembourser » |

Déclarées à la frontière et pour les postes reliés (phase 11) comme les autres ;
`docs/matrice-permissions.md` gagne ces lignes.

---

## 9. Suivi de ce que doit l'État

### 9.1 Chiffres

Par inscription, recalculés (R-20) : attendu (`montant_attendu`), abandonné, **dû** (`E + K`),
**reçu** (somme des parts actives, `S`), **reste dû** (somme des `reste_etat` + complément non
reçu), dont **échu** (échéances dont la date est passée ou du jour ; le complément compte comme
échu avec la dernière tranche), reçu en plus. Par classe et pour l'école : les sommes, plus le
non réparti des versements de l'année.

### 9.2 L'état imprimable

Document **« État des sommes dues par l'État »** (Typst, R-45, mise en page des états de la
finance), à joindre à une réclamation :
- en-tête de l'école (nom, adresse, autorisation d'ouverture si la fiche de l'établissement la
  porte), année scolaire, « arrêté au 18 novembre 2026 » ;
- par classe : élève, matricule, sexe, série, référence d'orientation, attendu, reçu, reste dû,
  dont échu ; sous-total par classe ;
- total général : élèves, attendu, reçu, reste dû ;
- liste des versements de l'État reçus pour l'année (date, référence, montant) et non réparti ;
- lieu, date, « Le chef d'établissement », place de la signature et du cachet ; QR de
  vérification comme les autres états.

Options à l'impression : une classe ou toutes ; « seulement les élèves dont l'État doit encore » ;
une année close. Export Excel des mêmes colonnes.

### 9.3 Tableau de bord

Pour le titulaire de `invoices:issue`, si l'année a une prise en charge : une tuile « À recevoir de
l'État : 2 400 000 F (dont échu 960 000 F) ». « Reste échu » (parents) et « Encaissé » (caisse)
ne changent pas de sens.

### 9.4 Fin d'année

- Le report des impayés vers l'année suivante ne lit que le reste du parent (§4.4) : ce que doit
  l'État **n'est jamais reporté sur la facture du parent**.
- Ce que doit l'État reste attaché à l'année close : l'état se produit pour une année close, les
  versements de l'État s'y répartissent (§7.3), l'abandon s'y fait (§3.7).
- À la réinscription (T-169), l'inscription préparée de l'année suivante d'un élève coché qui reste
  au lycée ou au technique reçoit la case cochée, proposée, avec le montant attendu recalculé
  (§3.3) ; la direction la décoche si l'État ne l'a pas reconduit.

---

## 10. Rapports de recettes et trésorerie

| Rapport | Ce qui change |
|---|---|
| Journal de trésorerie du mois | L'argent de l'État dans la catégorie « Versements de l'État », au mois de sa réception, une fois (par la recette) ; rien pour la part de l'État non encore payée ; rien pour un versement reçu avant GESCO |
| Livre et solde des comptes, rapprochement | La recette du virement au compte de banque ; rien d'autre |
| État de caisse, sessions | Rien, sauf un versement de l'État en espèces (entrée de la session, comme toute recette en espèces) ; un remboursement payé en espèces est une sortie, comme toute dépense |
| Journal des encaissements (reçus des parents) | Rien : l'argent de l'État n'est pas un versement de parent ; les versements des parents gardent leurs imputations et leurs totaux |
| Budget de l'année | Recette « Versements de l'État » : prévu au choix de l'école, réalisé = recettes reçues ; sous le tableau des recettes, une ligne **« À recevoir de l'État »** (reste dû par l'État de l'année) **hors du total réalisé** ; dépenses « Remboursements aux parents » comme dans la reprise |
| Impayés | Parents seulement (§4.4) |
| Ce que doit l'État | §9 |
| Tableau de bord | Tuile du §9.3 |

**Jamais compté deux fois**, par construction :
1. l'argent de l'État entre en trésorerie par sa seule recette ; la répartition n'écrit aucun
   argent ;
2. la part de l'État n'est jamais dans le reste du parent, et le reste de l'État n'est jamais dans
   les impayés ;
3. le trop-payé du parent est soit reporté, soit à rembourser, soit remboursé (§3.4, contrôle) ;
4. un versement reçu avant GESCO n'a pas de recette : il est dans le solde d'ouverture ;
5. « à recevoir de l'État » est hors du réalisé : il y entre seulement quand l'argent arrive, et
   sort alors de « à recevoir ».

---

## 11. Croisement avec la reprise avant GESCO

`SPEC_REPRISE_AVANT_GESCO.md` v1.2 est codée juste avant ; cette spécification s'appuie sur elle et
la prolonge sans la contredire :

| Point | Reprise (codée avant) | Ici |
|---|---|---|
| Migration | 0065 | 0066 (ou la suivante libre) |
| Placement de la reprise | Sur les restes sans reprise, plus ancienne d'abord | Sur la part du parent seulement, après le report du trop-payé (§3.4, §4.2) : la reprise est l'argent du parent |
| À rembourser | Excédent de la reprise | Excédent de la reprise **plus** trop-payé non reporté ; une seule ligne, un seul bouton « Rembourser » |
| Remboursé | Réduit la reprise utilisable | S'impute d'abord sur la reprise, puis sur le trop-payé (§3.4) |
| `reprise.deja_remboursee` | Refus d'une reprise plus petite que le remboursé | Le minimum devient le remboursé diminué du trop-payé disponible ; sans prise en charge, inchangé |
| Écran « Payé avant GESCO » | Colonne « dû de l'année » | Dû **du parent** ; pour un élève coché, une mention « dont scolarité prise en charge par l'État » ; l'import des reprises compare au dû du parent |
| Argent reçu avant GESCO | Parents : reprise, sans caisse | État : versement « reçu avant GESCO », sans recette (§7.7) ; même date d'arrêt |
| Parcours de l'école d'essai (§9 de la reprise) | Étapes 1 à 8 | Une étape 4 bis : « cocher les élèves pris en charge » (liste de la classe), avant ou après la reprise, dans n'importe quel ordre ; puis, si l'État a déjà payé, un versement « reçu avant GESCO » |
| Essais | 1 à 12 | L'essai « caisse intacte » de la reprise s'étend aux gestes d'ici (§12, essai 1) |

Rien de la reprise n'est à réécrire : le seul changement est dans `recalculer_dans` (placement sur
la part du parent) et dans le calcul de « à rembourser », tous deux faits à l'étape 2 d'ici.

---

## 12. Essais

Chaque étape écrit ses essais **sans les passer** ; tous les essais de la 2.0.0 se passent une
seule fois, après le lot 5 (précision du porteur du 17 novembre 2026). Rust :

1. **Caisse intacte** (propriété) : pour une école avec sessions, versements, annulations, comptes
   avec soldes d'ouverture : cocher, décocher, modifier, abandonner, répartir et modifier une
   répartition laissent identiques les totaux des sessions, l'état de caisse, le journal des
   encaissements, le prochain numéro de reçu et les tables `versement`, `imputation_versement`,
   `mouvement_caisse`. Un versement de l'État par virement ne change que le livre et le solde de
   son compte, le journal de trésorerie du mois (catégorie « Versements de l'État ») et le budget ;
   en espèces, il ajoute une entrée à la session ; reçu avant GESCO, il ne change rien de tout cela.
2. **Partage** (exemple) : inscription 15 000 F au 1er octobre, scolarité 120 000 F en trois
   tranches (48 000 F au 15 octobre, 36 000 F au 15 janvier, 36 000 F au 15 avril), cantine
   30 000 F au 15 octobre. Le parent paie 15 000 F (inscription) et 48 000 F (1re tranche) en
   octobre. Case cochée le 10 novembre, attendu 120 000 F → part de l'État 120 000 F, inscription
   renoncée (15 000 F), trop-payé 63 000 F, reporté 30 000 F sur la cantine, à rembourser
   33 000 F, reste du parent 0, l'élève absent des impayés, aucune relance. Versement de l'État de
   120 000 F → reste de l'État 0, facture soldée. Même élève coché à l'inscription → la facture
   naît avec la cantine seule à la charge du parent : le premier versement proposé est 30 000 F.
3. **Écart** : attendu 100 000 F → renoncé 20 000 F sur la tranche d'avril, à la charge de
   l'école ; le parent ne doit rien de plus. Abandon de 36 000 F ensuite → part de l'État
   64 000 F, renoncé 56 000 F ; le parent ne doit toujours rien de plus.
3 bis. **Complément** : scolarité de l'école 100 000 F, attendu 126 000 F → 100 000 F sur les
   échéances, complément 26 000 F hors facture, l'État doit 126 000 F, total de la facture
   inchangé ; versement de l'État de 126 000 F → l'État ne doit plus rien, rien de « reçu en
   plus », facture soldée ; versement de 110 000 F → facture soldée côté État, complément restant
   16 000 F dans ce que doit l'État ; abandon de 16 000 F → l'État ne doit plus rien.
4. **Décocher** (suite de l'exemple 2, après le remboursement de 33 000 F) : sans versement de
   l'État → l'État ne doit rien, le parent doit 135 000 F (inscription 15 000 F, scolarité
   120 000 F et cantine 30 000 F, moins 63 000 F payés, plus 33 000 F rendus ; 33 000 F sur la
   tranche d'octobre) ; après un versement de l'État de 60 000 F pour l'élève → part de l'État
   60 000 F (octobre entier, 12 000 F de janvier), reste de l'État 0, le parent doit 75 000 F
   (15 000 F de cantine, 24 000 F en janvier, 36 000 F en avril). Annuler le versement de
   48 000 F du parent ensuite → refusé (`versement.deja_rembourse`).
4 bis. **École sans cantine** : même élève sans ligne de cantine → trop-payé 63 000 F, rien à
   reporter, à rembourser 63 000 F, reste du parent 0.
5. **Propriété du partage** : pour toute suite de versements, remises, ajustements, frais ajoutés,
   reprises, remboursements, cases cochées et décochées, abandons, versements et répartitions de
   l'État : `net = part_etat + renonce + part_parent` ; contrôle du §3.4 ; aucun reste négatif ;
   somme des `paye_etat` + `paye_complement` + reçu en plus = `S` ; sans prise en charge, chiffres identiques à ceux
   d'aujourd'hui.
6. **Versement groupé** : un virement de 2 400 000 F pour 20 élèves → 20 parts, une recette, aucune
   session touchée ; répartition de 2 500 000 F refusée (`versement_etat.reparti_superieur`) ;
   1 800 000 F répartis → 600 000 F non répartis visibles ; part de 130 000 F pour un élève qui en
   doit 120 000 F → 10 000 F reçus en plus ; annulation → recette annulée, restes de l'État revenus.
7. **Année close** : répartir un versement sur l'année close et abandonner → permis ; cocher ou
   décocher → `prise_en_charge.annee_close` ; le report des impayés ne porte jamais la part de
   l'État.
8. **Lecteurs** : impayés, relances (file d'attente des SMS), consultation, guichet, documents
   remis, départ (solde exigé), abonnés de la cantine, encaissement (proposition, refus au-delà du
   reste du parent), reçu ; et l'essai qui parcourt les requêtes SQL de `gesco-db` : toute requête
   qui additionne `imputation_versement` pour un payé passe par `etat_echeances`, lit la reprise et
   la prise en charge, ou est un journal d'argent reçu.
9. **Droits** : Secrétaire et Caissier refusés sur chaque commande d'écriture de la prise en charge ;
   Directeur et Comptable acceptés ; versement de l'État refusé sans `treasury:manage` ; « reçu
   avant GESCO » refusé sans `invoices:discount`.
10. **Trace** : chaque geste laisse la version ancienne `remplacee` et un audit avant et après ;
    suppression refusée par la base sur les quatre tables.
11. **Import de la répartition** : un fichier avec chaque cas du §7.4 → aperçu attendu ; écriture
    des seules lignes valides et en avertissement ; sauvegarde prise avant.
12. **État et rapports** : totaux de l'état = somme des restes de l'État ; budget : « À recevoir de
    l'État » hors du total réalisé, recette « Versements de l'État » dans le réalisé ; PDF produit
    sans erreur pour une classe, toutes, une année close.

TypeScript (vitest) : aperçu du cochage (ce que le parent ne doit plus, trop-payé, à rembourser),
pied de la répartition (réparti, non réparti, refus au-delà du reçu), « même montant pour chaque
élève ». Playwright, dans l'essai global de fin de codage : cocher une classe, enregistrer un
virement réparti sur la liste, imprimer l'état ; captures de l'onglet « Élèves », du formulaire de
versement et de la section « Frais » d'un élève coché.

---

## 13. Ordre de réalisation

Huit étapes courtes, commit et push sur `developpement` après chacune ; fmt, clippy, tsc et ESLint
à chaque étape ; essais écrits avec le code, passés seulement à la fin du codage, après le lot 5.
Toutes les questions sont tranchées : rien n'attend le porteur avant l'étape 1.

1. **Tables et noyau** : migration 0066 montante et descendante (`prise_en_charge_etat`,
   `montant_etat`, `versement_etat`, `versement_etat_part`, index, déclencheurs, `CHECK`), ADR,
   catégorie livrée « Versements de l'État » ; dépôt
   `gesco-db/src/depots/prise_en_charge.rs` ; calcul pur du §3.4 dans
   `gesco-core/src/finance/prise_en_charge.rs` (partage, renoncé, côté État, trop-payé,
   remboursé, report) ; essais 5 (propriété, sur le calcul pur) et 10 écrits.
2. **Calcul de la facture** : `etat_echeances`, `EcheanceAffichee`, `FactureAffichee`,
   `recalculer_dans` (statut, reprise sur la part du parent, à rembourser), remises (§5.1), plan
   (§5.2), abandon et départ (§5.4), frais ajouté, abonnés de la cantine, refus du remboursement
   au-delà de « à rembourser » ; essais 2, 3, 4 et 8 écrits.
3. **Service de la prise en charge** : cocher, modifier, abandonner, décocher, pour un élève ou une
   sélection ; aperçu ; montant proposé (§3.3) ; synchronisation de la case ;
   `complement_eleve_regler` qui y passe ; validation et rattrapage des factures ; année close ;
   événements, audit, fraîcheur ; essais 7 (partie case) et 9 écrits.
4. **Versement de l'État** : enregistrer (recette liée ou reçu avant GESCO), répartir, compléter,
   annuler ; annulation de la recette depuis la trésorerie ; année close ; essais 1 et 6 écrits.
5. **Commandes et écrans** : frontière, types TypeScript, entrée « Prise en charge par l'État »
   (onglets « Élèves » et « Versements de l'État »), section « Frais » de la fiche, grille « Pour
   les rapports », assistant d'inscription, écran Caisse, réglages (§3.2), textes `fr-ML.ts`,
   matrice des permissions ; essais vitest écrits.
6. **Répartition par fichier** : modèle prérempli, analyse, écriture, rapport et export des
   refus ; essai 11 écrit.
7. **Ce que doit l'État et rapports** : onglet « Ce que doit l'État », document Typst, export
   Excel, tuile du tableau de bord, ligne « À recevoir de l'État » du budget, relevé de compte ;
   essai 12 écrit.
8. **Fin d'année et manuel** : case proposée à la réinscription (T-169), écran « Payé avant GESCO »
   (§11), `docs/manuel-2.0-prise-en-charge-etat.md`, étape 4 bis du parcours de l'école d'essai
   dans le manuel de la reprise, essais à la main, notes de version 2.0.0.

**Après le lot 5** (hors de cette spécification) : tous les essais de la 2.0.0 en une fois,
corrections, puis l'installeur.

---

## 14. Questions au porteur du projet

Numérotées à partir de T-164 (T-163 est la dernière employée, vérifié dans `docs/orchestration.md`
et les spécifications le 18 novembre 2026). **Toutes tranchées** le 18 novembre 2026 : T-171 par
le porteur ; T-164 par une logique souple après une recherche sur Internet, comme le porteur l'a
demandé ; T-165 à T-170 par la solution la plus simple qui ne fausse aucun chiffre, le porteur
ayant laissé le choix (« pour la gestion de l'État, débrouille pour ne pas fausser »).

### T-164 — Combien l'État paie-t-il pour un élève ? (tranchée)

Le porteur : « regarde sur Internet combien l'État paie pour un élève, ou une logique flexible ».

Recherche du 18 novembre 2026 : **aucun barème officiel par élève n'est publié en ligne**. Ce que
disent les sources trouvées :
- rapport de l'OCLEI sur les subventions de l'État aux écoles privées du secondaire (2017-2021) :
  l'État paie pour ses élèves orientés des frais scolaires, une demi-bourse (« dépenses liées au
  matériel didactique ») et des pensions alimentaires ; 196 milliards F en quatre ans ; ses
  tableaux par établissement donnent, tout compris, de 145 000 F à 181 000 F environ par élève et
  par an ; jusqu'en 2017 les décisions d'attribution « ne précisaient que le nombre d'élèves »,
  les états de paiement étant établis par l'établissement et vérifiés par l'académie, puis une base
  de données unique du ministère sert de référence ;
- presse malienne (2024) : 96 000 F de frais scolaires plus 30 000 F de demi-bourse, soit
  126 000 F par élève orienté, sans texte cité ni année ; des subventions versées « à tour de
  rôle », certaines pas encore reçues en mai.

Décision : **logique souple, sans montant livré**. Un montant par niveau et série que l'école
règle pour l'année (vide = la scolarité de l'école), corrigeable pour un élève ou une sélection
d'après l'état de paiement validé par l'académie ; s'il dépasse la scolarité, l'excédent est un
**complément attendu de l'État** suivi hors facture (§3.3, §3.4) ; s'il est plus bas, l'écart est
renoncé par l'école. Un montant livré serait faux pour une partie des écoles et des filières.
Étapes 1 et 3.

Sources : [rapport de l'OCLEI](https://oclei.ml/wp-content/uploads/2023/11/OCLEI-Rapport-Subventions-Ecoles.pdf),
[Mali Émergence, juillet 2024](https://maliemergenceinfo.com/2024/07/26/ecoles-privees-le-capharnaum/),
[Bamada.net](https://bamada.net/rapport-de-loclei-sur-les-subventions-accordees-aux-ecoles-privees-au-mali-1077-etablissements-secondaires-prives-fonctionnent-sur-la-base-de-faux-documents),
[Benbere](https://benbere.org/terre-dopportunites/orientation-scolaire-embarras-choix-apres-def/).

### T-171 — Les frais d'inscription d'un élève pris en charge (tranchée)

Le porteur : « pas question de frais d'inscription pour l'État ». **Personne ne les doit** :
ni le parent, ni l'État. La ligne reste sur la facture (rien n'est réécrit), son dû est renoncé, à
la charge de l'école ; elle n'entre pas dans ce que doit l'État. §3.2. Étape 2.

### T-165 — L'État paie moins que les frais de l'école : qui paie l'écart ? (tranchée)

**L'école** y renonce : personne ne doit l'écart (exemple : scolarité 120 000 F, État 100 000 F,
20 000 F renoncés). Pas de réglage. C'est la décision « L'État, pas le parent » prise à la lettre,
et la plus simple : un seul calcul, aucune somme demandée au parent pour la scolarité. §3.4.

### T-166 — Ce que le parent a déjà payé sur la part de l'État (tranchée)

**Reporté sur ses autres frais** (cantine, tenue…), le surplus **à rembourser** par la dépense
« Remboursements aux parents », comme le trop-payé avant GESCO (« remboursement », réponse du
porteur à T-162). Le parent garde son reçu, rien n'est réécrit, rien n'est remboursé deux fois ;
le laisser à l'école (« l'État doit d'autant moins ») ferait payer l'école deux fois dès que l'État
paie son montant entier. §3.5.

### T-167 — Qui coche la case (tranchée)

Le titulaire de **`invoices:discount`** : Administrateur, Directeur, Comptable, comme les remises
et la reprise ; le Secrétaire voit la case sans la changer. Une seule case, une seule vérité pour
les rapports et pour l'argent ; aucune permission nouvelle. §8.

### T-168 — L'État paie plus que ce qui était attendu (tranchée)

L'excédent reste **visible à part** (« reçu de l'État à répartir » sur le versement, « reçu en
plus » sur l'élève) jusqu'à ce que l'école le répartisse ou le laisse ; jamais imputé au parent,
jamais recompté : l'argent est déjà en trésorerie une fois, par la recette. §7.5.

### T-169 — L'année suivante, la case suit-elle l'élève ? (tranchée)

**Proposée cochée** à la réinscription d'un élève qui reste au lycée ou au technique, montant
recalculé ; la direction la décoche si l'État ne l'a pas reconduit. La case se voit dans
l'inscription préparée, avant toute facture : rien n'est faux si elle est décochée ensuite. §9.4.

### T-170 — L'État est-il « en retard », et depuis quand ? (tranchée)

La part de l'État suit **les tranches de la facture**, placée de la plus ancienne à la plus
récente : l'état montre l'échu et l'à-venir sans aucun réglage. §3.4, §9.

---

## 15. Journal des révisions

| Version | Date | Changement |
|---|---|---|
| 1.0 | 18 novembre 2026 | Proposée d'après la décision du porteur du jour (« L'État, pas le parent ») ; vérification du code : case des rapports sans effet financier, `etat_echeances` seul calcul du reste, recettes hors versements sans session ; partage calculé de chaque échéance entre l'État et le parent, trop-payé reporté puis à rembourser, versement de l'État par une recette et une répartition, reçu avant GESCO, état imprimable, croisement avec la reprise ; quatre tables (ADR) ; T-164 à T-170 ; huit étapes |
| 1.1 | 18 novembre 2026 | Précisions du porteur du jour (« pas d'inscription et la cantine est le parent s'il l'école possède une et pour la gestion de l'État débrouille pour ne pas fausser ») et rappel que GESCO n'est installé dans aucune école : inscription non due pour un élève coché (T-171, ouverte : personne ou l'État), cantine due par le parent si la facture en porte une, scolarité seule à l'État, plus de réglage des natures ni de l'écart (colonnes `types_frais` et `ecart_a_charge` retirées) ; T-165 à T-170 tranchées sur leurs recommandations ; exemples et essais refaits (essai 4 bis : école sans cantine) |
| 1.2 | 18 novembre 2026 | Prête à coder. T-171 répondue par le porteur (« pas question de frais d'inscription pour l'État ») : inscription due par personne. T-164 tranchée après recherche sur Internet (pas de barème public ; rapport de l'OCLEI : frais scolaires, demi-bourse, pensions, de 145 000 F à 181 000 F environ par élève selon les établissements ; presse : 96 000 F + 30 000 F) : montant par niveau et série réglé par l'école, vide = scolarité de l'école, corrigeable ; un montant supérieur à la scolarité devient un complément attendu de l'État, suivi hors facture (§3.3, §3.4, §9.1) ; essai 3 bis |
