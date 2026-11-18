# GESCO 2.0 — manuel d'utilisation : commencer GESCO en cours d'année

Chapitre du manuel de la version 2.0, à placer dans la partie « Finance » quand le manuel de la
2.0 sera assemblé. Il concerne la direction et la comptabilité : ce sont elles qui saisissent ce
que les parents avaient déjà payé. Le secrétariat importe les élèves ; la caisse encaisse ensuite
comme d'habitude.

Une école qui commence GESCO au deuxième mois de l'année a déjà des élèves, des parents qui ont
payé une partie de la scolarité au cahier ou sur des reçus papier, des absences et des notes
depuis la rentrée. GESCO doit y entrer **sans que le premier jour soit faux** :

- le reste à payer de chaque élève est juste dès que la direction a saisi ce qui était déjà payé ;
- la caisse de GESCO ne compte que l'argent qu'elle reçoit. L'argent d'avant est déjà dans le
  tiroir, à la banque ou sur le compte mobile money : il entre dans GESCO par les **soldes
  d'ouverture** des comptes, jamais une seconde fois ;
- les absences et les notes des semaines passées se saisissent comme celles du jour.

---

## 1. Le parcours, dans l'ordre

1. **L'année.** Paramètres → Années : l'année scolaire, active, avec **sa vraie date de
   rentrée** ; les niveaux, les classes, les périodes.
2. **Les frais.** Finance → Frais de l'année : la grille et les plans de paiement, mise en
   vigueur.
3. **Les élèves.** Scolarité → Élèves → Importer : au moment de l'import, le champ « **Dans
   l'école depuis le** » est prérempli avec la rentrée. Gardez-le pour des élèves qui sont là
   depuis la rentrée : leurs factures auront toutes leurs tranches, et ils apparaîtront dans les
   relevés d'absences et les grilles de notes des jours passés. Un élève vraiment arrivé plus tard
   se corrige dans son brouillon avant la validation.
4. **La validation.** Validez les inscriptions par classe (plan par défaut). Pour les parents qui
   ont choisi un autre plan, changez le plan sur la fiche de l'élève ; accordez les remises et les
   bourses.
5. **Payé avant GESCO.** Finance → Payé avant GESCO : saisissez à l'écran ce que chaque parent
   avait payé, ou remplissez le modèle Excel d'après le cahier et importez-le (section 2 et 3).
   Un parent qui a payé plus que sa facture : ajoutez d'abord le frais oublié s'il y en a un
   (section 5), sinon remboursez-le quand la caisse est ouverte (section 4).
6. **Les comptes.** Finance → Trésorerie : le solde d'ouverture de chaque compte (banque, mobile
   money) au jour où l'école commence GESCO. La caisse ouvre sa première session avec l'argent
   vraiment dans le tiroir.
7. **La première caisse.** Dès que la première session de caisse de l'année est ouverte, la date
   d'arrêt se fixe d'elle-même à la veille (section 2).
8. **Absences et notes.** Les absences des jours passés se relèvent au relevé du jour, en
   choisissant la date ; les évaluations d'octobre se créent avec leur date et se notent au
   clavier ou par l'import d'une grille ; pour un élève arrivé d'une autre école après la fin d'une
   période, saisissez sa moyenne reprise du relevé officiel.

Les étapes 4 et 5 se font dans n'importe quel ordre : changer un plan après avoir saisi le payé
avant GESCO garde le même reste à payer.

## 2. L'écran « Payé avant GESCO » (Finance → Payé avant GESCO)

- En haut : l'**année** (l'année courante par défaut), la **classe** (toutes par défaut) et la
  **recherche** par nom ou matricule.
- **« Arrêté avant GESCO : avant le 7 octobre »** : ce qui est saisi ici est ce qui était payé
  avant le jour de la première caisse de l'année. Tant qu'aucune caisse n'est ouverte, l'écran dit
  « à ce jour ». Cette date n'est jamais saisie : un élève oublié et saisi trois semaines plus tard
  reste « avant GESCO ».
- Sous la date : le total payé avant GESCO de la sélection et le nombre d'élèves ; s'il y a lieu,
  le total à rembourser et le nombre de parents ; et le nombre d'élèves encore en brouillon, à
  valider avant de pouvoir saisir ce qu'ils ont payé.
- **Une ligne par élève validé** : classe, élève, matricule, dû de l'année (net des remises),
  **payé avant GESCO** (la case), payé dans GESCO, reste, à rembourser.
- **La case se saisit au clavier** : tapez le montant (« 75000 », « 75 000 », « 75 000 F » ou
  « 75.000 »), puis **Entrée** : la ligne s'enregistre, le reste se met à jour, le curseur passe à
  l'élève suivant. Un montant plus grand que le dû est accepté : la colonne « à rembourser » montre
  l'excédent.
- **Vider la case ou taper 0** sur un élève qui avait un montant : GESCO demande le motif de
  l'annulation (« Saisie erronée » est proposé). L'ancienne valeur reste dans l'historique.
- **Un refus avec un minimum** : si une partie de l'excédent a déjà été rendue au parent, le
  montant ne descend pas sous ce qui lui a été rendu ; l'écran dit le minimum. Pour aller plus
  bas, annulez d'abord la dépense de remboursement.
- **Exporter en Excel ou en CSV** : comme les autres listes.
- **Année close** : la liste se lit, les cases sont grisées.

Ce n'est **pas un encaissement** : ni reçu, ni numéro de reçu, ni caisse, ni compte. Le reste à
payer est le seul chiffre qui change. Le reçu d'un versement fait ensuite porte le reste juste.

## 3. Importer depuis Excel

1. Dans l'écran « Payé avant GESCO », choisissez l'année et, si vous voulez, une classe, puis
   « Télécharger le modèle Excel ou importer » → « **Télécharger le modèle Excel** ».
2. Le classeur contient les élèves validés de la sélection : matricule, nom, prénoms, classe, dû
   de l'année (pour lecture), **Payé avant GESCO** (vide, ou le montant déjà saisi) et une note
   facultative (« cahier p. 12 »). Une feuille « Mode d'emploi » rappelle les consignes.
3. Remplissez **seulement la colonne « Payé avant GESCO »** d'après le cahier. Une case vide ne
   change rien ; 0 annule ce qui avait été saisi.
4. « **Choisir le fichier rempli** » : GESCO montre chaque ligne avant d'écrire.
   - **Valide** ou **À vérifier** : sera enregistrée. « À vérifier » dit pourquoi : « Remplace
     50 000 F par 75 000 F », « Dont 15 000 F à rembourser au parent », « Retrouvé par le nom : à
     vérifier », « Annule la reprise de 50 000 F ».
   - **Erreur** : laissée de côté. « Élève introuvable », « Plusieurs élèves portent ce nom dans
     cette classe : indiquez le matricule », « Inscription pas encore validée », « Montant
     illisible », « Élève présent deux fois », « 15 000 F déjà remboursés : 15 000 F au moins ».
   - **Ignorée** : case vide, ou « Inchangé ».
5. « **Importer** » : GESCO fait d'abord une sauvegarde, puis enregistre toutes les lignes
   valides et à vérifier d'un coup. Pendant l'import, les autres postes attendent.
6. Le bilan dit combien de montants sont enregistrés, annulés, ignorés et refusés. Les lignes
   refusées s'exportent en Excel : corrigez-les et importez-les de nouveau ; les lignes déjà
   enregistrées reviendront « Inchangé ».

## 4. Sur la fiche de l'élève (section « Frais »)

- Sous les échéances : « **Payé avant GESCO : 75 000 F** (arrêté avant le 7 octobre) », avec la
  note. Dans le tableau des échéances, la colonne « Dont avant GESCO » montre la part portée par
  chaque échéance, la plus ancienne d'abord.
- La direction et la comptabilité y trouvent **Modifier** et **Annuler** ; les autres voient la
  ligne seulement. **Historique** déplie les anciennes valeurs : montant, saisie, annulation,
  motif.
- **À rembourser au parent** : si le parent a payé plus que sa facture, la ligne dit combien, et
  la liste des remboursements faits suit (date, montant, mode, ou « en attente d'accord »).
- **Rembourser** ouvre une dépense déjà remplie : catégorie « Remboursements aux parents »,
  bénéficiaire le parent payeur, montant l'excédent (vous pouvez le baisser), libellé
  « Trop-payé avant GESCO — » suivi du nom de l'élève. Ensuite, **les règles ordinaires des
  dépenses** : au-dessus du seuil, l'accord de la direction ; payée en espèces par la caisse
  ouverte de qui paie, ou par un compte. C'est cette dépense, le jour où elle est payée, qui sort
  l'argent de la caisse ou du compte. Rien n'est remboursé deux fois : un remboursement plus grand
  que ce qui reste à rendre est refusé.
- **Le relevé de compte** imprimé porte la ligne « Payé avant GESCO » en tête des paiements, avec
  sa date d'arrêt, puis les remboursements et ce qui reste à rendre au parent.

## 5. Ajouter un frais oublié

Les frais facultatifs (cantine, transport, tenue…) se cochent à la validation. Un frais oublié
s'ajoute après : sur la fiche, section « Frais », bouton « **Ajouter un frais** », à côté du
changement de plan.

- GESCO propose les frais facultatifs de la grille, pour le niveau de l'élève, qui ne sont pas
  encore sur sa facture, avec les échéances qu'ils créeraient (mêmes tranches qu'à la validation).
- Si le parent avait payé plus que sa facture, le frais ajouté prend d'abord sur cet excédent :
  « à rembourser » baisse d'autant. Si le parent a déjà été remboursé, le frais devient un reste à
  payer ordinaire.
- Un frais ajouté par erreur se **retire** du même panneau : ses échéances que rien n'a encore
  payées sont annulées ; une échéance déjà payée reste.

## 6. Dans les rapports

- **Journal des encaissements** : si la période choisie contient la date d'arrêt, une ligne
  « Payé avant GESCO — hors caisse » donne le nombre d'élèves et le total. Elle est **sous les
  totaux et hors d'eux** : les totaux par jour, par mode et par caissier ne comptent que la
  caisse. L'export Excel et CSV porte la même ligne, marquée « Hors des totaux ».
- **Budget de l'année** : dans les recettes, une ligne « Payé avant GESCO (hors caisse) », sans
  prévu, comptée dans le total des recettes réalisées ; les catégories (scolarité, inscription…)
  ne la reçoivent pas.
- **Impayés, relances, cantine, fin d'année, tableau de bord** : ils lisent le reste après le
  payé avant GESCO.
- **Journal de trésorerie, livres des comptes, état de caisse** : rien pour le payé avant GESCO.
  Un remboursement payé y entre comme toute dépense, au mois et au compte de son paiement.

## 7. Qui fait quoi

| Geste | Qui |
|---|---|
| Saisir, modifier, annuler, importer le payé avant GESCO | Administrateur, Directeur, Comptable (comme les remises) |
| Lire la ligne « Payé avant GESCO » sur la fiche | Qui lit la situation financière de l'élève |
| Rembourser un parent | Qui saisit les dépenses ; l'accord et le paiement suivent les règles des dépenses |
| Ajouter ou retirer un frais | Qui change le plan de paiement |

Le caissier et le secrétariat ne saisissent pas le payé avant GESCO. Une école qui ne veut pas que
sa comptable le fasse lui retire la permission « Accorder une remise » dans un rôle personnalisé.
