# SPEC_REPRISE_AVANT_GESCO.md — Installer GESCO dans une école en cours d'année

**Projet** : ERP de gestion scolaire — Tauri 2 + Rust + React/TypeScript
**Objet** : « Reprise avant GESCO » — ce que les parents ont payé avant l'installation réduit le
reste à payer dès le premier jour, sans versement, sans reçu, sans caisse ; et ce qu'il faut pour
saisir les absences et les notes des mois passés ; version 2.0.0
**Statut** : v1.2 — **acceptée** le 17 novembre 2026 par le porteur du projet (« t162
remboursement et le reste oui ») : T-161 et T-163 sur leurs recommandations, T-162 tranchée
autrement (l'excédent est à rembourser au parent, §3.6 bis) ; v1.1 : essais renvoyés à la fin du
codage (après le lot 5) ; v1.0 proposée le même jour, d'après la décision du porteur
(« ok » à la version simple, `docs/orchestration.md`, commit `0dd1623`)
**Documents parents** : `SPEC_PHASE_3.md` (§4 facture et échéancier, §4.2 arrivée en cours d'année,
§6 sessions de caisse, §7 encaissement et imputation, §9 impayés et journal),
`SPEC_PHASE_4.md` (§6 absences, précisions de l'étape 8 : Q-27 périodes non suivies, Q-28
moyennes reprises), `SPEC_PHASE_8.md` (§5 comptes et soldes d'ouverture, §7 journal et budget),
`SPEC_PHASE_11_B.md` et principe du porteur « la flexibilité sans incohérence » (14 novembre
2026), migration `0053_changement_de_classe` (ajustements et avoirs : règlement sans encaissement)
**ADR proposé avec cette spécification** : `docs/adr/2026-11-17-reprise-avant-gesco.md`
**Ordre** : fin du lot 4 de la phase 12 → **code de cette spécification** → lot 3 (quiz) → lot 5
(devoirs) → tous les essais, une seule fois → installeur, pour l'école d'essai comme pour les
autres (précision du porteur du 17 novembre 2026 : « on fait les test a la fin du codage » ; plus
d'installeur intermédiaire).
**Date** : 17 novembre 2026

---

## 0. Objet et critère de fin

### 0.1 Le cas à servir

Le porteur a une école d'essai réelle, au deuxième mois de l'année scolaire. Elle a des élèves
inscrits depuis la rentrée, des parents qui ont déjà payé une partie de la scolarité (au cahier,
sur des reçus papier), des absences et des notes prises depuis octobre. GESCO doit y entrer sans
que le premier jour soit faux :

- le reste à payer de chaque élève est juste dès que la direction a saisi ce qui était déjà payé ;
- la caisse de GESCO ne compte que l'argent qu'elle a reçu : l'argent d'avant est déjà dans le
  tiroir, la banque ou le compte mobile money, donc dans les **soldes d'ouverture** des comptes de
  trésorerie (`SPEC_PHASE_8.md` §5). Le compter une seconde fois fausserait tout ;
- les absences et les notes des semaines passées se saisissent comme celles du jour.

### 0.2 Ce que l'application possède déjà (vérifié dans le code le 17 novembre 2026)

| Besoin | Existe | Où |
|---|---|---|
| Élèves en masse | Import Excel ligne par ligne, brouillons d'inscription | `import_eleves.rs` |
| Validation en masse | Validation d'une classe entière, plan par défaut | `ServiceInscriptions::valider_classe` |
| Moyennes des périodes non suivies | Moyenne reprise d'un relevé (Q-28) | `moyenne_reprise_saisir` |
| Personnel | Import Excel | `import_personnel.rs` |
| Argent déjà dans les comptes | Solde d'ouverture et sa date, par compte | `SPEC_PHASE_8.md` §5 |
| Règlement sans encaissement | Avoir né d'un changement de classe, imputé aux échéances | `avoir`, `avoir_imputation` |
| Évaluation datée dans le passé | Oui, si la date est dans la période et la période non verrouillée | `verifier_evaluation` |
| Absences d'un jour passé | Oui, si le jour est dans une période non verrouillée | `verifier_jour` |
| Import d'une grille de notes | Oui (Q-39) | `ServiceEvaluations::importer` |

Ce qui manque :

1. **Aucun moyen de dire « déjà payé »** : `DemandeEncaissement` n'a pas de date ; tout versement
   prend l'heure du jour (`encaisse_le: contexte.maintenant`) et entre dans la session de caisse
   ouverte. Saisir le passé en versements gonflerait la caisse du jour, le journal des
   encaissements, le livre des comptes, et compterait deux fois l'argent des soldes d'ouverture.
2. **L'import des élèves date toutes les inscriptions du jour de l'import**
   (`import_eleves.rs`, `date_inscription: aujourd_hui`). Au deuxième mois de l'année, c'est faux
   partout : la facture applique la règle d'arrivée en cours d'année (Q-26, par défaut seules les
   tranches non échues), le relevé d'absences d'un jour passé ne montre pas l'élève (il n'y est
   « qu'à partir de sa date d'inscription »), la grille d'une évaluation datée d'octobre non plus
   (« ne montre que les élèves inscrits à sa date »), et une période finie avant l'import devient
   « non suivie » (Q-27). C'est le plus petit ajout qui débloque absences et notes (§8).

### 0.3 Critère de fin, vérifiable

La spécification est réalisée quand ces dix assertions sont vraies, chacune démontrée par un essai
automatisé (§11) :

1. Une reprise d'un montant M sur une inscription réduit son reste à payer de M exactement,
   imputée de la plus ancienne échéance à la plus récente.
2. Une reprise ne crée ni versement, ni reçu, ni numéro de reçu, ni mouvement de caisse, ni ligne
   dans le livre d'un compte : sessions, état de caisse, journal des encaissements (ses totaux),
   journal de trésorerie et soldes des comptes sont identiques avant et après une reprise, sa
   modification et son annulation.
3. Le montant saisi est toujours accepté : ce qui dépasse ce que GESCO facture devient « à
   rembourser » au parent, visible dans sa situation ; le remboursement est une dépense « Remboursements
   aux parents » qui sort de la caisse ou d'un compte le jour où elle est payée ; un frais ajouté
   ensuite à la facture réduit d'autant ce qui reste à rembourser ; rien n'est remboursé deux fois ;
   aucun reste ne devient négatif (§3.6 bis).
4. La reprise se modifie et s'annule tant que l'année n'est pas close ; l'ancienne valeur reste,
   annulée, avec son auteur, sa date et son motif ; rien ne se supprime (R-14).
5. Seul le titulaire de la permission retenue (§6) enregistre, modifie ou annule une reprise ; un
   caissier ne le peut pas.
6. Le plan de paiement se change encore après une reprise : la reprise se réimpute sur le nouvel
   échéancier dans la même transaction, et le reste total ne bouge pas.
7. La situation de l'élève, son relevé de compte, la liste des impayés, les relances, la liste
   des abonnés de la cantine et le report des impayés de fin d'année lisent le reste après reprise.
8. La reprise se voit à part : « Payé avant GESCO » dans la situation de l'élève et son relevé ;
   une ligne « Payé avant GESCO » dans le journal des encaissements (hors de ses totaux) et dans le
   budget de l'année ; jamais de répartition par mois.
9. L'import Excel des reprises vérifie chaque ligne, montre les erreurs avant d'écrire, puis écrit
   tout en une transaction après une sauvegarde automatique (R-48).
10. L'import des élèves propose une date d'arrivée, la rentrée par défaut : les élèves importés au
    deuxième mois ont leurs tranches entières, apparaissent au relevé d'absences d'un jour passé
    et dans la grille d'une évaluation passée.

---

## 1. Périmètre

**Inclus** : montant « payé avant GESCO » par inscription, à l'écran et par Excel (modèle fourni) ;
imputation sur l'échéancier ; modification et annulation tracées ; prise en compte partout où se
lit le reste à payer ; lignes « Payé avant GESCO » dans la situation, le relevé, le journal des
encaissements et le budget ; excédent à rembourser et son remboursement par une dépense (§3.6 bis) ;
ajout d'un frais facultatif oublié à une facture validée (§3.7) ; date d'arrivée à l'import des
élèves ; parcours d'installation de l'école d'essai (§9).

**Exclu** :
- Le détail des paiements d'avant (date, mode, payeur, reçu papier) : un seul montant par
  inscription, sur décision du porteur. Une note libre facultative garde la référence du cahier.
- Toute répartition par mois ou par jour de l'argent d'avant.
- Une reprise des dépenses ou des salaires d'avant : les soldes d'ouverture des comptes suffisent.
- Un relevé d'absences « en totaux » pour les mois passés : la saisie jour par jour suffit (§8.2).
- La correction de la date d'une inscription **déjà validée** : l'école d'essai importe ses élèves
  après la mise à jour ; un brouillon se corrige déjà (`ServiceInscriptions::modifier`).

---

## 2. Principes

- **Pas d'argent, des chiffres justes** : une reprise n'est pas un encaissement. Elle dit « cette
  part de la facture était déjà réglée quand GESCO est arrivé ». Elle règle des échéances comme un
  avoir, sans toucher à la caisse ni aux comptes.
- **Pas de double compte** : l'argent d'avant est dans les soldes d'ouverture (`SPEC_PHASE_8.md`
  §5) ; la reprise n'entre dans aucun solde, aucun livre, aucun journal de trésorerie.
- **La flexibilité sans incohérence** : rien n'est bloqué par l'ordre des gestes (reprise avant ou
  après le choix du plan, avant ou après les premiers encaissements) ; les totaux restent justes
  à chaque geste. Le montant saisi n'est jamais refusé ; seuls sont refusés les gestes qui
  rendraient un chiffre faux (rembourser plus que l'excédent, baisser une reprise sous ce qui a déjà
  été remboursé).
- **L'argent ne sort que par la caisse ou un compte** : l'excédent d'une reprise n'est qu'une dette
  envers le parent ; il ne quitte l'école que par une dépense payée, au jour de son paiement.
- **Rien ne se réécrit** : une modification annule l'ancienne reprise et ses imputations et en
  crée une nouvelle, dans une transaction ; l'audit garde avant et après.

---

## 3. La reprise

### 3.1 Modèle

`reprise_avant_gesco` — une inscription, un montant strictement positif, une note facultative
(120 caractères au plus : « cahier p. 12 », « reçus 0451 à 0453 »), une origine (`ecran` ou
`import`), un statut (`active`, `annulee`), le motif d'annulation, le lien vers la reprise qui la
remplace, et les colonnes habituelles (créée le, par, poste). **Une seule reprise active par
inscription**, garantie en base par un index unique partiel.

`reprise_imputation` — la part d'une reprise portée par une échéance : reprise, échéance, montant
strictement positif, statut (`active`, `annulee`). La somme des parts actives est au plus le
montant de la reprise active diminué de ce qui a été remboursé ; la différence est l'excédent
« à rembourser » (§3.6 bis).

`depense.inscription_id` (colonne nouvelle, facultative) : la dépense qui rembourse l'excédent
d'une inscription, rangée dans la catégorie de dépense livrée « Remboursements aux parents » (code
stable `remboursement_parent`).

Aucune des deux tables ne se supprime (déclencheurs, R-14). La finance référence l'inscription,
jamais l'élève (§8.3 du blueprint) : une fusion de fiches emporte les reprises avec les
inscriptions.

Modèle de données et options écartées : ADR `docs/adr/2026-11-17-reprise-avant-gesco.md`.

### 3.2 Date : « avant GESCO », sans saisie

La reprise ne porte pas de date saisie. Sa date est celle de l'**arrêt avant GESCO de l'année**,
calculée et jamais stockée (R-20) :
- la veille du jour où la première session de caisse de l'année a été ouverte dans GESCO (session
  dont l'ouverture tombe entre le début et la fin de l'année de l'inscription) ;
- tant qu'aucune caisse n'a été ouverte dans l'année : la veille d'aujourd'hui, montrée « à ce
  jour ».

Ainsi la reprise est toujours datée avant la première caisse, même saisie des semaines plus tard
pour un élève oublié : l'incohérence est impossible par construction, sans rien refuser. Toutes
les reprises d'une année partagent cette date, donc aucune répartition par mois n'est possible.

### 3.3 Règles

- L'inscription est validée et a une facture vivante (non annulée) ; sinon
  `reprise.sans_facture` (« Validez d'abord l'inscription de cet élève : sa facture n'existe pas
  encore »).
- L'année de l'inscription est en préparation ou active ; close ou archivée,
  `reprise.annee_close`. La reprise d'une année close se lit, ne se change plus.
- **Aucun montant maximal** (réponse du porteur à T-162) : un montant plus grand que ce que
  l'inscription doit est accepté ; l'excédent est à rembourser (§3.6 bis).
- **Imputation** : la plus ancienne échéance due d'abord, par date puis par rang, toutes natures
  de frais confondues, chaque part limitée au reste de son échéance, sur les restes calculés
  **sans** la reprise ; on impute au plus le montant **utilisable** de la reprise (son montant moins
  ce qui a été remboursé). C'est `imputer_plus_ancienne` du noyau, borné à la somme des restes au
  lieu de refuser. Les versements déjà faits gardent leurs imputations ; la reprise remplit ce qui
  reste.
- **Baisse d'une reprise sous ce qui a déjà été remboursé** : refusée (`reprise.deja_remboursee`),
  le message donne le minimum (« 15 000 F ont déjà été remboursés au parent : 15 000 F au moins »).
  Pour aller plus bas, on annule d'abord la dépense de remboursement.
- **Montant zéro** à l'écran ou dans l'import : annule la reprise active s'il y en a une, sinon
  rien.
- **Baisse d'une reprise après un changement de classe** : si un ajustement a fait naître un avoir
  sur l'argent de cette inscription, la reprise ne descend pas sous le montant qui garde cet avoir
  couvert (payé total au moins égal au dû net plus les avoirs nés) ; `reprise.avoir_a_couvrir`,
  avec le minimum. Cas rare.

### 3.4 Gestes

| Geste | Effet, dans une seule transaction (durabilité critique) |
|---|---|
| Enregistrer (aucune active) | Reprise active, imputations, facture recalculée (`recalculer_dans`) ; audit ; `RepriseAvantGescoEnregistree` |
| Modifier (une active) | L'ancienne passe `annulee` (motif « remplacée »), ses imputations aussi ; la nouvelle naît, liée ; imputations ; facture recalculée ; audit avant et après ; `RepriseAvantGescoModifiee` |
| Annuler | Motif obligatoire ; reprise et imputations `annulee` ; facture recalculée ; audit ; `RepriseAvantGescoAnnulee` |

Chaque commande d'écriture porte la date de lecture de l'écran et passe par
`crate::fraicheur::verifier` sur la ligne de la reprise et la facture, comme les autres
modifications simultanées.

### 3.5 Effet sur le reste à payer

`facturation::etat_echeances` est le seul calcul du reste ; tous les écrans et documents le
lisent (situation, encaissement, impayés, relances SMS, guichet, consultation du solde,
documents remis, fin d'année, ajustements, remises). Il gagne une colonne :

- `EcheanceAffichee.reprise` : la part de reprise active portée par l'échéance ;
- `paye` = versements imputés + avoirs utilisés + **reprise** ; `reste` = net − payé, inchangé dans
  sa forme ;
- `FactureAffichee.reprise` : le total imputé, pour l'affichage à part ;
  `FactureAffichee.a_rembourser` : l'excédent qui reste à rendre au parent (§3.6 bis) ;
  `FactureAffichee.rembourse` : ce qui lui a déjà été rendu.

**La réimputation a un seul point d'entrée** : `facturation::recalculer_dans`, que tous les gestes
qui changent des échéances appellent déjà (changement de plan, remise, ajustement, abandon,
départ, rétablissement, encaissement, frais ajouté). Il recalcule les parts de la reprise (plus
ancienne d'abord, au plus le montant utilisable) et ne les réécrit que si elles changent : les
parts qui ne tiennent plus passent `annulee`, les nouvelles naissent. Ainsi un dû qui monte
absorbe d'abord l'excédent, un dû qui baisse le libère.

Un seul autre endroit calcule un payé en SQL : la liste des abonnés de la cantine
(`gesco-db/src/depots/cantine.rs`), qui additionne `imputation_versement` et `avoir_imputation` ;
elle ajoute `reprise_imputation` active de la même façon. L'essai du §11 cherche toute requête qui
lit `imputation_versement` pour un payé sans lire `reprise_imputation`.

Le statut de la facture (émise, partiellement réglée, soldée) se déduit du payé : une reprise qui
couvre tout solde la facture, sans aucun reçu.

### 3.6 Effet sur les autres gestes de la finance

- **Changement de plan** (`echeancier_changer_plan`) : le verrou `echeancier.plan_verrouille` ne
  regarde que les versements et les avoirs, **pas la reprise**. Dans la même transaction : les
  imputations de la reprise sont annulées, les échéances recréées selon le nouveau plan, puis la
  reprise se réimpute (même montant, plus ancienne d'abord). L'école d'essai peut donc saisir la
  reprise avant ou après avoir rectifié le plan choisi par chaque parent.
- **Remise accordée** après une reprise : règle inchangée (dernière échéance d'abord, jamais plus
  que le reste) ; une remise plus grande que le reste après reprise est refusée comme aujourd'hui.
- **Changement de classe** : la reprise compte comme payé ; une baisse au-delà du non-payé devient
  un avoir, comme pour un versement.
- **Abandon, transfert, exclusion** : les échéances sans paiement à venir s'annulent ; une
  échéance portée par la reprise est payée, donc reste.
- **Encaissement** : la situation affichée au caissier est déjà nette de la reprise ; le reçu porte
  le reste après versement, juste. Le reçu ne mentionne pas la reprise.
- **Annulation d'un versement** : inchangée ; la reprise n'est pas touchée.

### 3.6 bis Excédent à rembourser (réponse du porteur à T-162)

Vérifié dans le code et les spécifications le 17 novembre 2026 : GESCO **n'a pas de geste de
remboursement d'un parent**. La permission `payments:refund` existe (Administrateur, Comptable)
mais aucune commande ne l'utilise ; la phase 3 l'avait laissée sans cas, puisqu'un trop-perçu y est
refusé (T-18) ; `SPEC_PHASE_11_B.md` ne traite pas de remboursement ; l'avoir (migration 0053)
règle des échéances mais ne sort jamais d'argent. Ce qui sort de l'argent existe : la **dépense**
de la phase 8, payée en espèces par la session ouverte de qui paie (`sortie_depense`) ou par un
compte de banque ou de mobile money, avec son seuil d'accord, son annulation motivée, sa place au
journal de trésorerie et au budget. Le plus petit ajout est donc de rembourser par une dépense.

Chiffres d'une inscription, recalculés à chaque fois (R-20) :
- **remboursé** = somme des dépenses **payées** et non annulées liées à l'inscription
  (`depense.inscription_id`) ;
- **utilisable** = montant de la reprise active − remboursé (jamais négatif, §3.3) ;
- **imputé** = ce que l'utilisable couvre des restes, plus ancienne échéance d'abord ;
- **à rembourser** = utilisable − imputé.

Gestes :
- **Situation de l'élève** : sous « Payé avant GESCO », une ligne « À rembourser au parent :
  15 000 F », et la liste des remboursements faits (date, montant, mode). Une dépense de
  remboursement en attente d'accord s'y voit aussi (« en attente d'accord »).
- **« Rembourser »** (titulaire de `expenses:create`, comme toute dépense) ouvre le formulaire de
  dépense prérempli : catégorie « Remboursements aux parents », bénéficiaire = le responsable
  payeur (au choix), montant = l'excédent (modifiable vers le bas), libellé « Trop-payé avant GESCO
  — <élève> », inscription liée. Ensuite, **les règles ordinaires des dépenses** : seuil d'accord,
  paiement en espèces par la session ouverte de qui paie ou par un compte, référence hors espèces,
  pièce jointe, annulation avec motif.
- **C'est la dépense, au jour de son paiement, qui sort de la caisse ou du compte** ; la reprise
  n'y entre toujours pas.
- Une dépense liée qui dépasse l'excédent à rembourser au moment de sa saisie ou de son paiement
  est refusée (`remboursement.superieur_a_l_excedent`), avec le montant possible : rien n'est
  remboursé deux fois.
- Payer ou annuler une dépense liée appelle `recalculer_dans` sur la facture de l'inscription :
  l'utilisable change, la reprise se réimpute.
- **Frais ajouté après coup** (§3.7) : le dû monte, la réimputation absorbe d'abord l'excédent, et
  « à rembourser » baisse d'autant ; si le parent a déjà été remboursé, le frais ajouté devient un
  reste à payer ordinaire.
- L'excédent non remboursé à la clôture de l'année reste visible dans la situation de l'année
  close ; la dépense de remboursement peut encore se faire (elle est datée de son jour).

Exemple : facture 185 000 F, reprise 200 000 F → facture soldée, à rembourser 15 000 F, caisse
inchangée. Remboursement de 10 000 F en espèces le 20 novembre → la session ouverte de qui paie
perd 10 000 F, le journal de trésorerie de novembre porte 10 000 F en « Remboursements aux
parents », à rembourser 5 000 F. La cantine oubliée (30 000 F) est ajoutée → la reprise couvre
5 000 F de la cantine, à rembourser 0, reste à payer 25 000 F.

### 3.7 Ajouter un frais facultatif oublié

Vérifié dans le code le 17 novembre 2026 : les frais facultatifs (cantine, transport…) ne se
cochent qu'à la validation (`ChoixFrais`) ; une facture validée ne gagne une ligne que par la hausse
d'un changement de classe. Pour que « l'école coche ensuite le frais oublié », le plus petit ajout :

- section « Frais » de la fiche : « Ajouter un frais », qui propose les lignes facultatives de la
  grille de la facture, pour le niveau de la facture, qui n'y sont pas encore ;
- la ligne s'ajoute à la facture (`ligne_facture` nouvelle, rien ne se réécrit) avec ses
  échéances calculées comme à la validation pour cette seule ligne (même plan, même date
  d'inscription, même règle d'arrivée en cours d'année) ; puis `recalculer_dans` (la reprise
  absorbe d'abord son excédent) ; audit ; événement `FraisAjoute` ;
- permission `enrollments:create`, comme le changement de plan ; refusé seulement si la facture
  est annulée ou l'année close.
- Retirer un frais ajouté par erreur : l'annulation de ses échéances non payées, par
  contre-passation, comme à l'abandon ; une échéance déjà payée reste.

---

## 4. Écran « Payé avant GESCO »

Dans **Finance**, une entrée « Payé avant GESCO », montrée au seul titulaire de la permission (§6).

- Choix de l'année (l'année courante par défaut) et de la classe (toutes par défaut).
- Une ligne par inscription validée : classe, élève, matricule, **dû de l'année** (net des remises
  et des baisses), **payé avant GESCO** (case modifiable), **payé dans GESCO** (versements et
  avoirs), **reste**, **à rembourser**. Recherche par nom ou matricule, comme les autres listes
  (`Tableau`).
- La case se saisit au clavier : Entrée enregistre la ligne et passe à la suivante ; le reste et
  l'excédent se mettent à jour. Un montant plus grand que le dû est enregistré ; la colonne « à
  rembourser » le montre.
- Vider la case ou taper 0 sur une ligne qui a une reprise ouvre la demande du motif d'annulation
  (motif proposé « Saisie erronée », modifiable).
- En tête : « Arrêté avant GESCO : avant le 18 novembre 2026 » (jour de la première caisse), ou
  « à ce jour » s'il n'y a pas encore de caisse ; total payé avant GESCO de la sélection ; nombre
  d'élèves concernés ; total à rembourser et nombre de parents à rembourser.
- Boutons « Télécharger le modèle Excel » et « Importer » (§5). Export Excel et CSV de la liste par
  le composant `Tableau` (R-41).
- Année close : la liste se lit, les cases sont grisées, avec « Année close : lecture seule ».
- Une inscription en brouillon n'apparaît pas ; une phrase sous le titre le rappelle s'il en reste
  dans la sélection : « 12 élèves sont encore en brouillon : validez-les pour saisir ce qu'ils ont
  payé. »

**Fiche de l'élève, section « Frais »** (`FraisEleve.tsx`) : sous les échéances, une ligne
« Payé avant GESCO : 75 000 F » (avec la note si elle existe), avant la liste des versements ; dans
chaque échéance, le payé inclut la reprise. Le titulaire de la permission y trouve « Modifier »
et « Annuler » ; les autres voient la ligne seulement. L'historique (anciennes valeurs annulées,
auteur, date, motif) se déplie sous la ligne. S'il y a un excédent : « À rembourser au parent »,
les remboursements faits, et le bouton « Rembourser » (§3.6 bis). Le bouton « Ajouter un frais »
(§3.7) est à côté de « Changer de plan ».

**Relevé de compte** (`releve_compte`) : une ligne « Payé avant GESCO » en tête des paiements,
sans date de reçu, avec la mention « arrêté avant le … » ; s'il y a lieu, « Remboursé au parent »
(date, montant) et « Reste à rembourser au parent ».

Aucun texte affiché ne cite une question, une spécification ou une version ; aucun nom réel dans
les exemples, les essais ni les captures.

---

## 5. Import Excel des reprises

Même façon de faire que l'import des élèves : fichier choisi, aperçu ligne par ligne, écriture des
lignes valides après une sauvegarde automatique (R-48), rapport.

### 5.1 Modèle fourni

« Télécharger le modèle » produit un classeur Excel (`gesco-docs`, `tableur.rs`) **prérempli** avec
les inscriptions validées de la sélection (année, classe) : Matricule, Nom, Prénoms, Classe, Dû de
l'année (pour lecture), **Payé avant GESCO** (vide, ou la reprise actuelle), Note. Une feuille
« Mode d'emploi » de cinq lignes. L'école ne remplit que la colonne « Payé avant GESCO ».

### 5.2 Lecture

Lecture par `import_eleves::lire_fichier` (xlsx, xls, ods, csv ; mêmes limites de taille et de
lignes). L'élève se retrouve **par le matricule** ; sans matricule, par nom, prénoms et classe
(recherche tolérante de la phase 2), avec un avertissement. Le montant accepte « 75000 »,
« 75 000 », « 75 000 F », « 75.000 » ; pas de décimales.

### 5.3 Vérification ligne par ligne

| Cas | Sort de la ligne | Message |
|---|---|---|
| Élève introuvable | Erreur | « Élève introuvable : vérifiez le matricule » |
| Plusieurs élèves possibles (sans matricule) | Erreur | « Plusieurs élèves portent ce nom dans cette classe : indiquez le matricule » |
| Inscription en brouillon ou sans facture | Erreur | « Inscription pas encore validée » |
| Montant illisible ou négatif | Erreur | « Montant illisible » |
| Montant supérieur à ce que l'élève doit | Avertissement | « Dont 15 000 F à rembourser au parent » |
| Élève deux fois dans le fichier | Erreur sur les deux lignes | « Élève présent deux fois » |
| Case vide | Ignorée | — |
| Montant égal à la reprise actuelle | Ignorée | « Inchangé » |
| Une reprise existe, montant différent | Avertissement | « Remplace 50 000 F par 75 000 F » |
| Montant inférieur à ce qui a déjà été remboursé | Erreur | « 15 000 F déjà remboursés : 15 000 F au moins » |
| Montant 0 et une reprise existe | Avertissement | « Annule la reprise de 50 000 F » |
| Retrouvé sans matricule | Avertissement | « Retrouvé par le nom : à vérifier » |

L'aperçu compte les lignes valides, en avertissement, en erreur et ignorées, et le total qui sera
enregistré. « Importer » écrit les lignes valides et en avertissement **en une transaction**, chaque
reprise auditée et son événement émis, plus un audit et un événement `ImportReprisesApplique`
pour le fichier. Une ligne en erreur n'empêche pas les autres ; le rapport la redonne, avec un
export des lignes refusées à corriger et réimporter.

---

## 6. Droits

La reprise change ce qu'un parent doit sans qu'un sou passe par la caisse : c'est un geste de
direction, du même ordre qu'une remise accordée. Décision (T-161, recommandation retenue) : **`invoices:discount`**,
déjà détenue par l'Administrateur, le Directeur et le Comptable, et par eux seuls parmi les rôles
livrés. Pas de permission nouvelle, donc pas de nouvelle version des rôles livrés ni de montée des
bases installées. Le Caissier et le Secrétaire n'y ont pas accès. La lecture (ligne dans la
situation) suit `students:read`, comme la situation elle-même.

Commandes, déclarées comme les autres à la frontière et pour les postes reliés (phase 11) :

| Commande | Permission | Effet |
|---|---|---|
| `reprises_lister` | `invoices:discount` | §4 : lignes de l'année et de la classe, date d'arrêt, totaux |
| `reprise_enregistrer` | `invoices:discount` | Enregistre ou remplace (§3.4) ; montant 0 : annule |
| `reprise_annuler` | `invoices:discount` | Motif obligatoire (§3.4) |
| `reprise_historique` | `students:read` | Reprises d'une inscription, actives et annulées |
| `reprises_modele` | `invoices:discount` | Classeur prérempli (§5.1) |
| `reprises_analyser_import` | `invoices:discount` | Aperçu (§5.3) |
| `reprises_importer` | `invoices:discount` | Sauvegarde automatique, puis écriture (§5.3) |
| `frais_ajouter_apercu` | `enrollments:create` | Frais facultatifs ajoutables et échéances qu'ils créeraient (§3.7) |
| `frais_ajouter` | `enrollments:create` | Ligne et échéances, réimputation de la reprise (§3.7) |
| `depense_saisir`, `depense_payer`, `depense_annuler` | inchangées (`expenses:*`) | Acceptent l'inscription liée d'un remboursement ; refus au-delà de l'excédent ; recalcul de la facture (§3.6 bis) |

Le remboursement n'utilise pas `payments:refund` : c'est une dépense, avec les droits, le seuil
d'accord et la trace des dépenses. `payments:refund` reste sans commande.

`docs/matrice-permissions.md` gagne ces lignes.

---

## 7. Rapports

| Rapport | Ce qui change |
|---|---|
| Situation de l'élève, relevé de compte | Ligne « Payé avant GESCO » (§4) ; payé et reste nets de la reprise ; « À rembourser au parent » et remboursements faits |
| Impayés (UC-05) | Reste échu et ancienneté après reprise ; aucune ligne à part (ce n'est pas une recette) |
| Journal des encaissements (§9.2 de la phase 3) | Si la période choisie contient la date d'arrêt (§3.2) : une ligne « Payé avant GESCO — hors caisse » avec le nombre d'élèves et le total, **sous les totaux et hors d'eux** ; totaux par jour, par mode, par caissier inchangés. Même chose dans l'export Excel et CSV |
| Budget de l'année (`SPEC_PHASE_8.md` §7) | Dans les recettes, une ligne « Payé avant GESCO » : réalisé = total des reprises actives de l'année, sans prévu, comptée dans le total des recettes réalisées ; les catégories (scolarité, inscription…) ne la reçoivent pas. Un champ à part du `Budget`, pas une catégorie de trésorerie |
| Journal de trésorerie du mois, livres des comptes, soldes | **Rien pour la reprise** : l'argent d'avant est dans les soldes d'ouverture ; l'ajouter le compterait deux fois. Un remboursement payé y entre comme toute dépense, au mois et au compte de son paiement, catégorie « Remboursements aux parents » |
| État de caisse, sessions | **Rien pour la reprise** ; un remboursement payé en espèces est une sortie de la session de qui paie, comme toute dépense |
| Budget de l'année, dépenses | Catégorie « Remboursements aux parents » : réalisé = remboursements payés ; prévu au choix de l'école |
| Tableau de bord | « Encaissé aujourd'hui » et « ce mois » inchangés (caisse) ; « reste échu » après reprise |

---

## 8. Absences et notes des mois passés

### 8.1 Ce que GESCO permet déjà

- **Absences** : un jour passé se relève au relevé du jour s'il est dans une période du niveau qui
  n'est pas verrouillée (`verifier_jour` : refuse seulement un jour à venir, hors période ou dans
  une période verrouillée). Le relevé par séance (s'il est allumé) demande l'emploi du temps de ce
  jour ; le relevé du jour reste toujours disponible.
- **Notes de détail** : une évaluation se crée avec une date passée si elle tombe dans sa période
  et que la période n'est pas verrouillée (`verifier_evaluation`) ; ses notes se saisissent au
  clavier ou s'importent d'un fichier (Q-39). Une période verrouillée se déverrouille par la
  direction dans le réglage des périodes.
- **Moyennes d'une période finie avant l'arrivée de l'élève** : moyenne reprise d'un relevé (Q-28),
  jamais de notes de détail.

**Mais** relevé et grille ne montrent un élève qu'à partir de sa date d'inscription, et l'import
date tout du jour de l'import (§0.2). Sans correction, l'école d'essai ne peut saisir ni les
absences ni les notes d'octobre de ses élèves importés, et ses factures perdent la première
tranche.

### 8.2 Le plus petit ajout : la date d'arrivée à l'import

- L'écran d'import des élèves gagne un champ **« Dans l'école depuis le »**, prérempli avec le
  **premier jour de l'année scolaire** (la rentrée), modifiable, borné au début et à la fin de
  l'année (`inscription.date_hors_annee` sinon). Aide sous le champ : « Pour des élèves qui sont là
  depuis la rentrée, gardez cette date. »
- `Reglages` de l'import porte cette date ; `appliquer` l'écrit dans `date_inscription` de chaque
  brouillon, à la place du jour de l'import. Le défaut est le même qu'à l'inscription à l'écran
  (« une inscription préparée avant la rentrée compte depuis la rentrée »).
- Un élève réellement arrivé plus tard se corrige dans son brouillon avant validation, comme
  aujourd'hui.
- Rien d'autre ne change : la règle d'arrivée en cours d'année (Q-26), les périodes non suivies
  (Q-27), le relevé et la grille lisent `date_inscription`, qui est désormais juste.

Pas de relevé d'absences « en totaux » pour les mois passés : la saisie jour par jour existe, et
un total sans jours contredirait le relevé et les justifications (T-163, recommandation retenue).
Si l'école d'essai trouve la saisie de huit semaines trop longue, le porteur le dira après l'essai.

---

## 9. Parcours de l'école d'essai (deuxième mois de l'année)

Écrit tel quel dans le manuel (`docs/manuel-2.0-reprise.md`) et dans les essais à la main
(`docs/essais-a-la-main-2.0.md`) :

1. Année scolaire active avec sa vraie date de rentrée, niveaux, classes, périodes.
2. Grille de frais et plans de paiement, mise en vigueur.
3. Import des élèves, « Dans l'école depuis le » = la rentrée.
4. Validation par classe (plan par défaut) ; puis, pour les parents qui ont choisi un autre plan,
   changement de plan ; remises et bourses accordées.
5. **Payé avant GESCO** : modèle Excel prérempli, colonne remplie d'après le cahier, import ; ou
   saisie à l'écran. Un parent qui a payé plus que sa facture : ajouter d'abord le frais oublié
   s'il y en a un (§3.7), sinon le rembourser par « Rembourser » quand la caisse est ouverte.
6. Comptes de trésorerie : solde d'ouverture de chaque compte à la date de départ (banque,
   mobile money) ; la caisse ouvre sa première session avec l'argent réellement dans le tiroir.
7. Première caisse ouverte : la date d'arrêt se fixe d'elle-même à la veille.
8. Absences des jours passés au relevé du jour ; évaluations d'octobre datées et notées (clavier
   ou import de grille) ; moyennes reprises pour les élèves arrivés d'une autre école après la fin
   d'une période.

Les étapes 4 et 5 se font dans n'importe quel ordre (§3.6).

---

## 10. Données nouvelles et événements

Migration **0065** (la suivante libre après le lot 4 de la phase 12, qui prend 0064 ; le lot 3
prendra la suivante), montante et descendante :
- `reprise_avant_gesco` ; index unique partiel `(inscription_id) WHERE statut = 'active'` ; index
  sur `inscription_id` ;
- `reprise_imputation` ; index `(echeance_id) WHERE statut = 'active'` et `(reprise_id)` ;
- déclencheurs refusant toute suppression sur les deux tables ;
- `CHECK` : montants entiers strictement positifs, statut dans sa liste, motif non vide pour une
  reprise annulée, note de 120 caractères au plus ;
- `depense.inscription_id TEXT REFERENCES inscription(id)` (`ALTER TABLE … ADD COLUMN`, sans
  reconstruction) et un index partiel `(inscription_id) WHERE inscription_id IS NOT NULL`.

La catégorie de dépense livrée « Remboursements aux parents » (code `remboursement_parent`)
s'ajoute comme les autres catégories livrées, à l'ouverture d'une base qui ne l'a pas ; l'école
peut la renommer, pas la supprimer. Aucune autre donnée existante ne se transforme.

Événements nouveaux (`TypeEvenement`) : `RepriseAvantGescoEnregistree`,
`RepriseAvantGescoModifiee`, `RepriseAvantGescoAnnulee`, `ImportReprisesApplique`, `FraisAjoute`.
Aucun abonné nouveau : ni SMS, ni trésorerie (c'est voulu). Le remboursement garde les événements
des dépenses.

---

## 11. Essais

Chaque étape écrit ses essais **sans les passer** ; tous les essais de la 2.0.0 (ceux-ci compris)
se passent une seule fois, à la fin du codage, après le lot 5 de la phase 12 (précision du porteur
du 17 novembre 2026). Rust :

1. **Caisse intacte** (essai de propriété) : pour une école avec sessions ouvertes et clôturées,
   versements, annulations, sorties, comptes avec soldes d'ouverture, on capture : totaux
   théoriques de chaque session, état de caisse, journal des encaissements (totaux par jour, mode,
   caissier), journal de trésorerie du mois, solde de chaque compte, livre de chaque compte,
   prochain numéro de reçu, nombre de lignes de `versement`, `imputation_versement`,
   `mouvement_caisse`. Puis reprise, modification, annulation, import : tout est identique.
2. **Reste juste** : dû 150 000 F en trois tranches (inscription 25 000 F) ; reprise 60 000 F →
   inscription soldée, 1re tranche partiellement réglée, reste 90 000 F ; versement 10 000 F →
   reste 80 000 F ; reprise portée à 100 000 F → reste 40 000 F ; annulée → reste 140 000 F.
   Essai de propriété : pour toute suite de versements, remises, frais ajoutés, reprises et
   remboursements, reste = net − payé, aucun reste négatif, imputations actives de la reprise +
   à rembourser + remboursé = montant de la reprise.
3. **Excédent et remboursement** (exemple du §3.6 bis) : reprise 200 000 F sur une facture de
   185 000 F → acceptée, facture soldée, à rembourser 15 000 F, caisse et comptes identiques ;
   remboursement de 10 000 F en espèces → la session de qui paie perd exactement 10 000 F, le
   journal de trésorerie du mois porte 10 000 F en « Remboursements aux parents », à rembourser
   5 000 F ; un remboursement de 6 000 F est refusé (`remboursement.superieur_a_l_excedent`) ;
   cantine de 30 000 F ajoutée → à rembourser 0, reste 25 000 F ; reprise baissée à 5 000 F →
   refusée (`reprise.deja_remboursee`) ; dépense de remboursement annulée → la reprise se
   réimpute, reste 15 000 F, et la session reçoit l'entrée d'annulation comme pour toute dépense.
   Totaux de caisse, journaux et soldes comparés avant et après chaque geste : seule la dépense
   payée (et son annulation) les change.
3 bis. **Frais ajouté** sans reprise : ligne et échéances comme à la validation (même plan, même
   date d'inscription, même règle d'arrivée) ; facture annulée ou année close → refus.
4. **Plan** : reprise puis changement de plan → accepté, reprise réimputée, reste total inchangé ;
   versement sur la scolarité puis changement de plan → toujours refusé.
5. **Changement de classe** vers des frais plus bas après une reprise qui couvre tout → avoir ;
   baisse de la reprise sous le minimum → `reprise.avoir_a_couvrir`.
6. **Droits** : Caissier refusé sur chaque commande d'écriture ; Directeur et Comptable acceptés.
7. **Année close** : `reprise.annee_close` ; la lecture marche.
8. **Trace** : modification et annulation laissent l'ancienne ligne annulée, auditée avant et
   après ; suppression refusée par la base.
9. **Lecteurs du reste** : impayés (ancienneté), cantine (abonnés et reste échu de cantine),
   report de fin d'année, situation et relevé ; et un essai qui parcourt les sources SQL de
   `gesco-db` : toute requête qui additionne `imputation_versement` pour un payé lit aussi
   `reprise_imputation` (ou passe par `etat_echeances`).
10. **Rapports** : journal des encaissements dont la période contient la date d'arrêt → ligne hors
    totaux ; période qui ne la contient pas → pas de ligne ; budget → ligne comptée dans le total
    des recettes réalisées, catégories inchangées ; date d'arrêt = veille de la première caisse de
    l'année, « à ce jour » sans caisse.
11. **Import** : un fichier avec chaque cas du §5.3 → aperçu attendu ; écriture des seules lignes
    valides et en avertissement ; sauvegarde prise avant ; rejouer le même fichier → tout
    « Inchangé ».
12. **Date d'arrivée** : import au 17 novembre avec la rentrée au 1er octobre → `date_inscription`
    = 1er octobre ; facture avec la tranche du 15 octobre ; relevé du 20 octobre et grille d'une
    évaluation du 25 octobre montrent l'élève ; date hors de l'année refusée.

TypeScript (vitest) : saisie de la case (montants « 75 000 », « 75.000 », 0, vide), calcul du
reste et de l'excédent affichés, préremplissage du formulaire de remboursement. Playwright : dans l'essai global de fin de codage, après le lot 5 (pas d'essai
Playwright par étape), le parcours du §9 étapes 3 à 7, avec captures de l'écran « Payé avant GESCO », de la
section « Frais » de la fiche et de l'aperçu d'import.

---

## 12. Ordre de réalisation

Huit étapes courtes, commit et push sur `developpement` après chacune ; fmt, clippy, tsc et ESLint
à chaque étape ; les essais s'écrivent avec le code et ne se passent qu'à la fin du codage, après
le lot 5.

1. **Date d'arrivée à l'import des élèves** (§8.2) : champ, réglage, écriture, textes ; essai 12
   écrit. Indépendante du reste.
2. **Tables** : migration 0065 montante et descendante, ADR, dépôt `gesco-db/src/depots/reprises.rs`
   (insérer, annuler, active de l'inscription, historique, imputations de la facture, insérer et
   annuler les imputations), colonne `depense.inscription_id`, catégorie livrée, types du noyau
   `gesco-core/src/finance/reprise.rs` (reprise, utilisable, imputé, à rembourser, minimum) ;
   essai 8 (suppression refusée).
3. **Calcul et service** : `etat_echeances` et `FactureAffichee` (§3.5), réimputation dans
   `recalculer_dans`, requête de la cantine, `ServiceReprises` (enregistrer, modifier, annuler,
   lister, historique, date d'arrêt), changement de plan (§3.6), événements, audit, fraîcheur ;
   essais 2 et 4 à 9 écrits.
4. **Remboursement et frais oublié** : dépense liée à une inscription (refus au-delà de l'excédent,
   recalcul au paiement et à l'annulation), « Ajouter un frais » (§3.7) ; essais 3 et 3 bis
   écrits.
5. **Commandes et écrans** : frontière, types TypeScript, écran « Payé avant GESCO », ligne,
   excédent, « Rembourser », « Ajouter un frais » et historique dans « Frais » de la fiche, relevé
   de compte, textes `fr-ML.ts`, matrice des permissions.
6. **Import Excel** : modèle prérempli, analyse, écriture, rapport et export des refus ; essai 11
   écrit.
7. **Rapports** : journal des encaissements (ligne hors totaux, exports), budget ; essais 1 et 10
   écrits.
8. **Manuel** : `docs/manuel-2.0-reprise.md`, essais à la main, notes de version 2.0.0 ; les
   essais vitest et l'essai de propriété de la caisse sont écrits, pas passés.

**Après le lot 5** (hors de cette spécification) : tous les essais de la 2.0.0 en une fois (Rust
complet, vitest, knip, Playwright et captures du §11), corrections, puis l'installeur.

---

## 13. Questions au porteur du projet

Numérotées à partir de T-161 ; toutes répondues par le porteur le 17 novembre 2026.

### T-161 — Qui saisit « Payé avant GESCO »

- a) le titulaire de `invoices:discount` : Administrateur, Directeur, Comptable (comme les
  remises) ;
- b) une permission nouvelle, accordée au seul Directeur et à l'Administrateur ;
- c) le titulaire de `fees:manage` (frais de l'année : les mêmes trois rôles).

*Recommandation* : a). Le geste a la nature d'une remise accordée (le dû baisse sans argent en
caisse), les trois rôles qui l'ont sont ceux qui connaissent le cahier de l'école, et aucune
nouvelle version des rôles n'est à monter. Une école qui ne veut pas que son comptable le fasse
lui retire la permission dans un rôle personnalisé. Étape 3.

*Réponse* (porteur du projet, 17 novembre 2026) : recommandation retenue.

### T-162 — Un parent qui a payé plus que ce que GESCO lui facture

Exemple : le cahier dit 200 000 F, la facture de GESCO 185 000 F (une cantine payée mais pas cochée
à l'inscription).
- a) refuser le montant, en donnant le maximum ; l'école coche d'abord le frais oublié, puis
  saisit ;
- b) accepter, et garder l'excédent comme un avoir de l'élève.

*Recommandation* : a). C'est la règle des versements (T-18) ; l'écart révèle presque toujours un
frais manquant dans la facture, qu'il vaut mieux corriger que cacher dans un avoir. Le message dit
le maximum, l'école corrige et ressaisit. Étape 3.

*Réponse* (porteur du projet, 17 novembre 2026) : « remboursement ». Ni refus ni avoir conservé :
le montant saisi est accepté ; l'excédent sur ce que GESCO facture est « à rembourser » au parent,
visible dans sa situation ; le remboursement est une dépense « Remboursements aux parents » qui
sort de la caisse ou d'un compte au jour de son paiement (le geste de remboursement d'un parent
n'existant pas, la dépense est le plus petit ajout) ; un frais ajouté ensuite réduit d'autant
l'excédent, rien n'est remboursé deux fois. Voir §3.6 bis et §3.7 (ajout d'un frais oublié, qui
n'existait pas non plus). Étapes 3 et 4.

### T-163 — Les absences des huit premières semaines

- a) jour par jour au relevé du jour, comme aujourd'hui (rien à ajouter) ;
- b) un relevé « en totaux » par élève pour les semaines d'avant GESCO (nombre de demi-journées,
  dont justifiées, retards), compté au bulletin.

*Recommandation* : a) pour l'essai. Le relevé du jour accepte les jours passés une fois la date
d'arrivée juste (§8.2) ; des totaux sans jours ne se justifieraient pas et contrediraient le
relevé. Si l'école d'essai trouve la saisie trop longue, b) se spécifie après l'essai.

*Réponse* (porteur du projet, 17 novembre 2026) : recommandation retenue.

---

## 14. Journal des révisions

| Version | Date | Changement |
|---|---|---|
| 1.0 | 17 novembre 2026 | Proposée d'après la décision du porteur du jour (version simple) ; vérification du code : absences et notes passées possibles, import des élèves daté du jour (ajout de la date d'arrivée) ; T-161 à T-163 |
| 1.1 | 17 novembre 2026 | Précision du porteur (« on fait les test a la fin du codage ») : les étapes écrivent leurs essais sans les passer ; tous les essais après le lot 5 ; plus d'installeur intermédiaire pour l'école d'essai |
| 1.2 | 17 novembre 2026 | Acceptée (« t162 remboursement et le reste oui ») : T-161 et T-163 sur recommandations ; T-162 : excédent à rembourser au parent, remboursé par une dépense « Remboursements aux parents » liée à l'inscription (aucun geste de remboursement n'existait) ; ajout d'un frais facultatif oublié à une facture validée ; réimputation de la reprise dans `recalculer_dans` ; huit étapes |
