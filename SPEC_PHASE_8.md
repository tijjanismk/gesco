# SPEC_PHASE_8.md — Dépenses, trésorerie, emploi du temps, cantine

**Projet** : ERP de gestion scolaire — Tauri 2 + Rust + React/TypeScript
**Phase** : 8 — dépenses et fournisseurs, comptes de trésorerie, budget, emploi du temps, cantine activable
**Statut** : v1.2 — spécification engagée le 16 octobre 2026 après les réponses du porteur du projet à T-61 à T-64 (toutes les recommandations retenues), avec T-38 (journée de l'école) et T-47 (cantine)
**Documents parents** : `PROJECT_BLUEPRINT.md` v0.23 (§8.8, §22.2, Q-06), `SPEC_PHASE_3.md` (caisse, versements), `SPEC_PHASE_7.md` (paiements du personnel, `SalairePaye`), `docs/questions-terrain.md` (T-38, T-47, T-58, T-61 à T-64)
**Date** : 16 octobre 2026

---

## 0. Objet et critère de fin

### 0.1 Ce que produit cette phase

Le livrable du blueprint (§22.2) : **la comptabilité de trésorerie**. L'école sait ce qui entre et ce qui sort, par catégorie et par mois, sur chacun de ses comptes, et le compare à son budget ; elle tient son emploi du temps et, si elle l'active, sa cantine.

À la fin de la phase :
- chaque dépense se saisit avec sa catégorie, son compte, sa pièce ; au-dessus du seuil de l'école, elle attend l'accord de la direction ;
- un fournisseur a ses factures, payées en une ou plusieurs fois, et ce que l'école lui doit ;
- la caisse, chaque compte en banque et chaque compte mobile money ont leur solde ; les transferts entre eux se notent ; les lignes se cochent contre le relevé ;
- les salaires payés et les avances deviennent des dépenses d'eux-mêmes ;
- le journal de trésorerie du mois et le budget prévu et réalisé se lisent et s'exportent en Excel ;
- l'emploi du temps de chaque classe se place à la main dans la journée de l'école, les conflits sont signalés, il s'imprime par classe et par enseignant ;
- une école qui a une cantine l'active : abonnés, liste du jour, repas, achats de vivres.

### 0.2 Décisions déjà prises qui s'appliquent

- **Approbation** (T-61) : au-dessus d'un seuil réglé par l'école, une dépense attend l'accord du directeur ou du fondateur ; en dessous, le comptable ou le caissier paie ; seuil zéro : jamais d'accord.
- **Comptes** (T-62) : caisse, comptes en banque, comptes mobile money, chacun son solde ; transferts ; rapprochement en cochant le relevé.
- **Comptabilité** (T-63) : trésorerie, pas SYSCOHADA ; budget avec avertissement, jamais de blocage ; export Excel pour le comptable.
- **Emploi du temps** (T-64) : placé à la main, conflits signalés sans refus, impression par classe et par enseignant ; pas de génération automatique.
- **Journée de l'école** (T-38) : horaires de l'école réglables, récréations, inter-classe ; horaire de canicule du seul 1er cycle sur une plage de dates, sans après-midi.
- **Cantine** (T-47) : module que l'école active ; abonnement facturé comme un frais (déjà possible), abonnés du jour, repas pris si l'école les compte, vivres en dépenses, personnel de cuisine au registre.
- **Salaires** (T-58, Q-06) : chaque `SalairePaye` devient une dépense « Salaires » par un événement, jamais par un déclencheur.
- R-14 : rien ne se supprime ; une dépense saisie par erreur s'annule avec un motif.

### 0.3 Critère de fin, vérifiable

1. Une dépense en dessous du seuil se paie aussitôt ; au-dessus, elle attend l'accord, puis se paie ; refusée, elle ne sort pas ; chaque état est tracé.
2. Une dépense en espèces de la caisse est une sortie de la session ouverte de qui paie ; une dépense par banque ou mobile money porte sa référence et sort de son compte.
3. Une facture de fournisseur se paie en plusieurs fois ; ce qui reste dû à chaque fournisseur se lit.
4. Le solde de chaque compte est le report, plus les entrées (versements, transferts reçus), moins les sorties (dépenses, salaires, transferts émis) ; un transfert de la caisse à la banque sort de l'une et entre dans l'autre.
5. Chaque salaire payé et chaque avance versée devient une dépense de la catégorie « Salaires », une seule fois, et s'annule avec son paiement.
6. Le journal de trésorerie d'un mois, par catégorie de recette et de dépense, et le budget prévu et réalisé s'exportent en Excel ; un dépassement de budget avertit.
7. Une séance de l'emploi du temps se place dans la journée de sa classe ; un enseignant à deux endroits à la fois, ou une salle prise deux fois, est signalé ; l'emploi du temps s'imprime par classe et par enseignant.
8. La cantine activée montre la liste des abonnés du jour d'après leurs factures ; les repas se pointent si l'école les compte ; les achats de vivres sont des dépenses « Cantine ».
9. Toute écriture est auditée avec son événement ; les tests des phases précédentes passent sans changement.

## 1. Périmètre

**Inclus** : catégories de dépenses et de recettes ; dépenses, accord, paiement, annulation, pièce jointe ; fournisseurs et leurs factures ; comptes de trésorerie, transferts, rapprochement ; dépenses nées des salaires et des avances ; journal de trésorerie ; budget annuel ; journée de l'école et horaire de canicule ; emploi du temps ; cantine activable.

**Exclu** :
- Comptabilité générale SYSCOHADA (journaux, grand livre, balance) : attendra une école qui la demande.
- Rapprochement automatique avec un fichier de relevé bancaire : on coche à la main.
- Génération automatique de l'emploi du temps.
- Bons de commande et stocks de vivres.

## 2. Catégories

Catégories de dépenses livrées, à code stable, que l'école renomme, complète ou désactive : Salaires, Loyer, Électricité et eau, Fournitures, Entretien et réparations, Cantine (vivres), Transport, Impôts et taxes, Frais bancaires, Divers. Catégories de recettes : les types de frais de la phase 3 (inscription, scolarité, cantine, transport, tenue, examen, autre), plus « Autres recettes » (dons, location de salle).

## 3. Dépenses

**Dépense** — `date`, `categorie`, `beneficiaire` (fournisseur ou texte libre), `montant`, `libelle`, `compte` (caisse, banque, mobile money), `mode`, `reference` (exigée hors espèces), `piece` (photo ou PDF, facultative), `statut`. États : **à approuver → approuvée → payée**, **à approuver → refusée** (motif), **payée → annulée** (motif) ; en dessous du seuil, elle naît approuvée. Le seuil est le réglage `depense.seuil_accord` (0 : jamais d'accord). Payer en espèces de la caisse : sortie de la session ouverte de qui paie, comme un salaire ; annuler la remet par une entrée.

## 4. Fournisseurs

**Fournisseur** — nom, téléphone, ce qu'il fournit. **Facture fournisseur** — numéro, date, montant, échéance, payée en une ou plusieurs dépenses ; reste dû par fournisseur.

## 5. Comptes et trésorerie

**Compte de trésorerie** — la caisse (une, faite des sessions de la phase 3), chaque compte en banque, chaque compte mobile money (opérateur, numéro), avec son solde d'ouverture et sa date. **Transfert** — d'un compte à un autre (dépôt de la caisse à la banque, retrait), un montant, une référence. **Rapprochement** — les mouvements d'un compte se cochent contre le relevé ; le solde rapproché se lit à côté du solde du livre.

Les versements des parents hors espèces (phase 3) entrent dans le compte de leur mode ; les paiements du personnel hors caisse (phase 7) sortent du compte de leur mode. Le compte d'un mode est celui que l'école choisit ; à défaut, le premier compte actif de sa nature (une banque pour le virement et le chèque, un compte mobile money pour Orange Money, Wave et Moov). Un transfert qui touche la caisse passe par la session ouverte de qui le fait (dépôt : une sortie ; retrait : une entrée) ; un transfert erroné se corrige par le transfert inverse. Le livre d'un compte réunit ses lignes depuis sa date d'ouverture : versements, mouvements de caisse, dépenses payées hors du tiroir, transferts ; chaque ligne se coche contre le relevé.

## 6. Salaires et avances

À chaque `SalairePaye` et chaque `AvanceAccordee`, une dépense « Salaires » naît, payée, liée au paiement ; un `PaiementPersonnelAnnule` l'annule. Le rattachement se fait dans la transaction même du paiement, jamais par un déclencheur, une seule fois par paiement (un index unique le garantit) ; il se rattrape à l'ouverture d'une base mise à niveau et quand un compte s'ajoute, pour les paiements faits avant. Un paiement dont le mode n'a pas encore de compte attend, signalé. Une dépense née d'un paiement ne s'annule que par l'annulation du paiement.

## 7. Journal et budget

**Journal de trésorerie** d'un mois : par catégorie, les recettes et les dépenses, et le solde de chaque compte au début et à la fin ; export Excel. Les recettes d'un versement se rangent par type de frais d'après ses imputations aux échéances ; la part qu'aucune échéance n'a reçue va aux autres recettes. Les dépenses comptent au jour de leur paiement. La saisie d'une recette hors des versements (don, location de salle) attendra une école qui la demande. **Budget** de l'année scolaire : un montant prévu par catégorie de recette et de dépense ; le réalisé à la date, l'écart, un avertissement au-delà du prévu, jamais de blocage.

## 8. Journée de l'école et emploi du temps

**Journée** — par cycle : début, fin, récréations, inter-classe, après-midi ou non par jour de la semaine ; **horaire de canicule** du 1er cycle sur une plage de dates, sans après-midi (T-38).

Journée proposée, que l'école règle : du lundi au vendredi, de 8 h à 12 h et de 15 h à 17 h, sans après-midi le mercredi, récréation de 10 h à 10 h 15 ; le jardin, le matin seul, récréation d'une demi-heure.

**Séance** — classe, jour, heure de début et de fin dans la journée de sa classe, matière, enseignant, salle facultative. Conflits signalés : un enseignant à deux séances en même temps, une salle prise deux fois, plus d'heures que le programme ne prévoit ; jamais refusés. Impression par classe et par enseignant. Les heures prévues des vacataires peuvent se préremplir d'après leurs séances (T-41).

## 9. Cantine activable

Réglage `cantine.active`. Abonnés : les élèves dont la facture de l'année porte des frais de type `cantine`, avec ce qui reste échu de leur cantine ; liste du jour, imprimable ; pointage des repas si l'école les compte (`cantine.repas_comptes`) ; achats de vivres en dépenses « Cantine » ; bilan du mois : recettes de cantine, dépenses de vivres, repas.

## 10. Droits

`expenses:create` (saisir), `expenses:approve` (accorder ou refuser), `expenses:pay` (payer), `treasury:read` (lire les comptes et le journal), `treasury:manage` (comptes, transferts, rapprochement, budget), `timetable:manage` (journée, emploi du temps), `canteen:manage` (cantine). Rôles livrés en version 9 : le Directeur approuve et lit tout ; le Comptable saisit, paie, gère la trésorerie ; le Caissier saisit et paie les petites dépenses en espèces ; le Directeur des études tient l'emploi du temps.

## 11. Ordre de réalisation

1. Catégories, comptes de trésorerie, réglages
2. Dépenses : saisie, accord au seuil, paiement, annulation, pièce
3. Fournisseurs et leurs factures
4. Transferts, soldes, rapprochement ; dépenses nées des salaires et des avances
5. Journal de trésorerie, budget, export Excel
6. Journée de l'école et emploi du temps, impressions
7. Cantine activable
8. Droits et rôles en version 9, tableau de bord, navigation, captures, école d'essai, validation

## 12. Journal des révisions

| Version | Date | Modification |
|---|---|---|
| v1.0 | 2026-10-16 | Création après les réponses du porteur du projet du 16 octobre 2026 (T-61 à T-64) : accord au-dessus d'un seuil, caisse, banque et mobile money, trésorerie et budget, emploi du temps à la main ; journée de l'école (T-38) et cantine activable (T-47) |
| v1.1 | 2026-10-16 | Étapes 3 à 5 : compte de chaque mode, transfert par la session de caisse, transfert inverse pour corriger ; salaires rattachés dans la transaction du paiement ; recettes du journal par imputation |
| v1.2 | 2026-10-16 | Étapes 6 à 8 : journée proposée ; une séance hors de la journée de sa classe est refusée, les conflits seulement signalés ; abonnés de la cantine d'après la facture de l'année, avec le reste échu ; rôles en version 9 (Directeur : dépenses, trésorerie, emploi du temps, cantine ; Comptable : trésorerie ; Caissier : saisie et paiement des dépenses ; Directeur des études : emploi du temps ; Secrétaire : cantine ; Consultation seule : lecture de la trésorerie) ; bloc « Trésorerie » et trois avertissements au tableau de bord |
