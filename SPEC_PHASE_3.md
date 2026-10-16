# SPEC_PHASE_3.md — Frais, factures, caisse, impayés

**Projet** : ERP de gestion scolaire — Tauri 2 + Rust + React/TypeScript
**Phase** : 3 — tarification, facturation, échéanciers, remises, sessions de caisse, encaissements, reçus, impayés
**Statut** : v1.3 (révisée le 10 octobre 2026) — spécification exécutable, engagée le 9 octobre 2026 après les réponses du porteur du projet à T-11 à T-21 et Q-48 à Q-53
**Documents parents** : `PROJECT_BLUEPRINT.md` v0.22 (§2.3 UC-01, UC-02, UC-05 ; §3.1, §3.6, §3.7, §3.11 ; §8.8, §8.9 ; §9.1 ; §10.1, §10.2, §10.3, §10.9, §10.10 ; §11.5 ; §21.2 ; §24.7 Q-26 ; §25.9), `ARCHITECTURE_RULES.md` v0.3 (R-11, R-14, R-17, R-20, R-33, R-35, R-45, R-46, R-48), `SPEC_PHASE_2.md` v1.5 (§0.3), `docs/questions-terrain.md` (T-11 à T-21)
**Date** : 9 octobre 2026

---

## 0. Objet et critère de fin

### 0.1 Ce que produit cette phase

Le livrable du blueprint (§22.2) : **UC-02, encaisser un versement**, et **UC-05, sortir la liste des impayés**. La phase complète aussi **UC-01** de sa partie financière : frais, échéancier, premier versement et reçu.

À la fin de la phase :
- la direction saisit les frais de l'année par niveau, et les plans de paiement que les parents peuvent choisir ;
- toute inscription validée porte sa facture et son échéancier, nés dans la même transaction (R-17) ;
- le caissier ouvre sa caisse, encaisse en moins de 60 secondes avec un reçu numéroté sans trou, et ferme sa caisse quand il remet l'argent, en déclarant ce qu'il a compté ;
- la direction accorde les remises, avec un motif, et voit la liste des impayés par classe et par ancienneté, avec le téléphone des payeurs.

### 0.2 Condition de livraison inchangée

La phase 2 a été validée le 9 octobre 2026 ; le chiffrement de la base reste hors du projet pour le moment, sur décision du porteur du projet du même jour. La condition de `SPEC_PHASE_2.md` §0.2 demeure : **aucune école ne reçoit GESCO avec des données d'élèves tant que la base n'est pas chiffrée et que l'essai sur machine vierge n'a pas réussi.**

Le blueprint recommande de chercher une école pilote à la fin de cette phase (§22.4). Les deux s'opposent : un pilote réel suppose le chiffrement. Le porteur du projet tranchera au moment de chercher le pilote, pas avant.

### 0.3 Factures des inscriptions validées en phase 2

En phase 2, la validation ne créait pas de facture (`SPEC_PHASE_2.md` §0.3). En phase 3, la facture rejoint la transaction de validation. Les inscriptions déjà validées n'ont pas de facture, et la grille de frais de l'année n'existe pas encore au moment où la base est mise à niveau : une génération automatique à l'ouverture de la base n'aurait rien à facturer.

Décision (Q-50) : une commande de rattrapage, `factures_rattraper`, réservée à la direction (`fees:manage`). Elle liste les inscriptions validées de l'année sans facture, montre ce qui sera facturé, puis, après une sauvegarde (R-48), crée toutes les factures en une transaction. Tant qu'il reste des inscriptions validées sans facture, un avertissement le signale à la direction.

### 0.4 Critère de fin, vérifiable

La phase est terminée quand ces douze assertions sont vraies, chacune démontrée par un test automatisé ou par le scénario de bout en bout (§13) :

1. Tous les montants sont des entiers de francs CFA, du noyau aux types TypeScript et aux exports ; aucune division n'arrondit en silence ; le montant en toutes lettres du reçu est exact de 0 à 999 999 999 francs.
2. La grille de frais d'une année porte, par niveau et au lycée par série, des lignes de frais obligatoires et facultatives, et de un à quatre plans de paiement ; le parent choisit son plan à l'inscription et en change tant que rien n'est payé sur la scolarité ; une grille utilisée par une facture ne se modifie plus, elle se remplace pour les inscriptions suivantes.
3. La validation d'une inscription crée sa facture et son échéancier dans la même transaction ; sans grille couvrant son niveau, la validation est refusée avec un message clair.
4. Un élève arrivé en cours d'année est facturé selon le paramètre de l'école (Q-26) : frais d'inscription entiers et seules tranches non échues, par défaut.
5. Les inscriptions validées en phase 2 reçoivent leur facture par la commande de rattrapage, après sauvegarde, une seule fois.
6. Une remise proposée par le secrétariat n'a d'effet qu'accordée par la direction, avec un type et un motif ; accordée, elle réduit l'échéancier ; annulée, elle se contre-passe ; jamais un reste dû ne devient négatif.
7. Aucun encaissement hors d'une session de caisse ouverte ; une session s'ouvre avec son fonds initial et se clôture avec le total compté ; un écart non nul exige un motif ; une session clôturée ne se rouvre pas.
8. Un versement s'impute sur les échéances, la plus ancienne d'abord, ou selon une répartition manuelle ; la somme des imputations égale le versement ; un versement supérieur au reste dû est refusé.
9. Chaque versement produit un reçu numéroté sans trou (R-46), avec le montant en chiffres et en lettres, le mode, la référence, le payeur, le reste dû et un QR code de vérification ; rejouer le même encaissement ne double ni la recette ni le reçu.
10. Un versement s'annule avec un motif, par contre-passation : le reçu reste, marqué annulé ; l'annulation après la clôture de sa session passe par une régularisation tracée dans la session du jour.
11. La liste des impayés sort par classe et par tranche d'ancienneté, avec le reste dû et le téléphone des payeurs ; elle s'exporte en Excel et en CSV.
12. Les invariants du blueprint §21.2 et de R-35 tiennent sous tests de propriétés ; UC-02 tient en moins de 60 secondes au clavier dans le scénario Playwright, et UC-01 enchaîne inscription, frais, premier versement et reçu sans quitter l'écran.

---

## 1. Périmètre

**Inclus** : grilles de frais et calendrier des tranches ; facture et échéancier à la validation ; arrivée en cours d'année (Q-26) ; rattrapage des inscriptions de la phase 2 ; remises ; sessions de caisse ; encaissement avec imputation et reçu imprimé ; encaissement pour plusieurs enfants d'une même famille ; annulation de versement ; situation financière de l'élève ; impayés ; journal des encaissements ; état de clôture de caisse.

**Exclu** :
- Relances SMS des impayés : phase 5, avec le canal SMS.
- Transfert sortant, avec Q-33 (impayé et certificat) et Q-34 (échéances d'un élève transféré) : phase 4, avec le certificat de transfert. L'échéancier est construit pour que Q-34 s'applique sans migration.
- Dépenses, fournisseurs, budgets, mouvements de caisse autres que les régularisations : phase 8.
- Rapprochement des relevés Orange Money, Wave et Moov par import (§3.11) : phase 8, avec la comptabilité de trésorerie. Le mode et la référence de transaction se saisissent dès maintenant.
- Report des impayés sur l'année suivante, avances sur l'année suivante : clôture d'année, phase 10.
- Remboursement (`payments:refund`) hors annulation : aucun cas en phase 3, puisqu'un trop-perçu est refusé (T-18).
- Blocage d'un élève pour impayé (bulletin, examen) : aucun en phase 3 ; la question reviendra en phase 4 avec les bulletins.

---

## 2. Montants

`gesco-core/src/commun/montant.rs` existe depuis la phase 0.5 : `Montant(i64)`, opérations vérifiées, division avec quotient et reste, répartition, arrondi commercial. La phase 3 y ajoute :
- **montant en toutes lettres**, en français : « cent vingt-cinq mille francs CFA ». Règles d'accord de « cent » et « vingt », « mille » invariable, « million » variable. C'est la mention qui fait foi sur un reçu malien ;
- **pourcentage** d'un montant en arithmétique entière, quotient et reste explicites, arrondi au multiple de l'école ;
- **paramètre `montant.multiple_arrondi`**, posé dès la phase 0.5 : multiple de 5 par défaut ; 1, 5, 10, 25, 50 ou 100, bornes vérifiées par le noyau. Il ne s'applique qu'aux montants calculés (prorata, remise en pourcentage), jamais à un montant saisi.

Côté TypeScript, un montant voyage en `number` entier. Le test de génération des types refuse tout champ de montant qui ne serait pas un entier (R-11).

---

## 3. Grille de frais

### 3.1 Modèle

`grille_tarifaire` (§8.8) : une par année scolaire et par établissement ; libellé ; état (brouillon, en vigueur, remplacée).

`ligne_tarifaire` : niveau, série facultative, type de frais (inscription, scolarité, cantine, transport, tenue, examen, autre), libellé, montant, obligatoire ou facultatif.

`plan_paiement` (T-12, réponse du 9 octobre 2026 : **au choix de chaque parent**) : la grille porte de un à quatre plans, l'un d'eux marqué par défaut. Chaque plan est une liste ordonnée de tranches, avec un libellé (« 1re tranche »), une date d'échéance et une part de la scolarité en pourcentage entier ; la somme des parts vaut 100 ; le reste d'une division non entière va à la dernière tranche. Plans livrés avec une grille neuve, que l'école modifie ou supprime : « Comptant » (100 % au 15 octobre), « Trois tranches » (15 octobre, 15 janvier, 15 avril : 40, 30, 30 %, plan par défaut) et « Mensuel » (neuf échéances d'octobre à juin).

Les frais d'inscription, d'examen et de tenue sont dus en une fois, à la date de l'inscription. La cantine et le transport suivent le plan choisi pour la scolarité ou une date unique, au choix de la ligne.

À l'écran, un frais se saisit une fois pour plusieurs niveaux, un cycle entier se cochant d'un clic (remarque du porteur du projet, 9 octobre 2026 : « frais de scolarité groupables, idem tenue scolaire ») ; la grille garde une ligne par niveau, que le calcul de la facture lit sans changement.

### 3.1 bis Choix du plan

Le plan se choisit à l'inscription, dans la fiche ou dans l'assistant ; à défaut, le plan par défaut de la grille s'applique. Il se change ensuite **tant qu'aucun versement n'est imputé sur la scolarité** (`echeancier.plan_verrouille` sinon) : les échéances de scolarité sont alors annulées par contre-passation et recréées selon le nouveau plan, remises comprises, dans une seule transaction. Commande `echeancier_changer_plan` (`enrollments:create`), audit, événement `PlanPaiementChange`.

### 3.2 Règles

- Une grille en brouillon se modifie librement. Mise en vigueur, elle sert aux factures ; une grille en vigueur qui a déjà servi ne se modifie plus. Pour changer un tarif en cours d'année, la direction crée une nouvelle version qui remplace la précédente **pour les inscriptions validées ensuite** ; les factures existantes ne bougent pas.
- Une grille en vigueur couvre chaque niveau actif qui a des classes dans l'année. Un niveau sans ligne de scolarité est signalé à la mise en vigueur, sans la bloquer : une école peut ne pas faire payer le jardin.
- Les montants d'une ligne sont positifs ou nuls ; zéro est permis (exemple : enfants du personnel par grille dédiée, plutôt que par remise).
- Une grille se recopie d'une année à l'autre, tarifs et plans compris, dates décalées d'un an.
- Un plan qui a servi à une facture ne se supprime pas d'une grille en vigueur ; il se remplace avec la grille.

### 3.3 Permission

Le catalogue n'a pas de permission pour les frais. Décision (Q-48) : **`fees:manage`**, accordée à l'Administrateur, au Directeur et au Comptable. La composition des rôles livrés passe en version 3, avec sa montée des bases installées (`SPEC_PHASE_1.md` §5.6). La version 3 donne aussi au Directeur `invoices:discount` (il accorde les remises, §5) et `cashbox:reconcile` (il vise la caisse, §6). Elle ne lui donne pas `payments:cancel` : le Directeur valide les notes, et aucun rôle livré ne cumule notes et argent (§11.5 du blueprint). L'annulation d'un versement (§8) revient donc à l'Administrateur et au Comptable ; une école sans comptable l'accorde à qui elle veut par un rôle personnalisé.

### 3.4 Commandes

| Commande | Permission | Effet |
|---|---|---|
| `grilles_lister` | `students:read` | Grilles de l'année, avec leurs lignes et leurs plans |
| `grille_creer` | `fees:manage` | Brouillon vide, ou copie d'une grille existante |
| `grille_modifier` | `fees:manage` | Brouillon : lignes, plans de paiement, libellé |
| `grille_mettre_en_vigueur` | `fees:manage` | Vérifications du §3.2 ; remplace la grille en vigueur ; audit |

---

## 4. Facture et échéancier

### 4.1 Création à la validation

À la validation d'une inscription (`SPEC_PHASE_2.md` §5), dans la même transaction (R-17) :
1. le noyau demande à la scolarité le **contexte tarifaire** de l'inscription (§9.1 du blueprint) : niveau, série, date d'inscription, nature (nouvelle, réinscription) ; la finance ne lit jamais les tables de la scolarité ;
2. il prend la grille en vigueur et les lignes obligatoires du niveau, plus les lignes facultatives cochées à l'inscription (cantine, transport…) ;
3. il crée la facture, émise et numérotée (`FAC-2026-000123`), avec une ligne par frais ;
4. il crée les échéances : les frais dus en une fois à la date d'inscription, la scolarité selon le plan choisi par le parent (§3.1 bis) ;
5. il applique la règle d'arrivée en cours d'année (§4.2) et les remises déjà accordées (§5).

Sans grille en vigueur couvrant le niveau, la validation est refusée (`facture.grille_absente`, Q-49). Une inscription validée sans facture est une perte de recette silencieuse (§10.1 du blueprint).

La facture naît émise : l'état brouillon du §10.2 du blueprint ne sert à rien quand la facture se calcule entièrement et naît en même temps que la validation (Q-51). Le blueprint est amendé.

### 4.2 Arrivée en cours d'année (Q-26)

Paramètre d'établissement `facturation.arrivee_en_cours` :
- `echeances_restantes` (**défaut**) : frais d'inscription entiers ; scolarité sur les seules tranches du plan choisi dont l'échéance n'est pas passée à la date d'inscription ;
- `prorata_mensuel` : scolarité au prorata des mois restants de l'année, arrondie selon `montant.multiple_arrondi` ;
- `plein_tarif` : tout est dû.

Toute autre réduction passe par une remise tracée.

### 4.3 États

```
ÉMISE ──première imputation──► PARTIELLEMENT RÉGLÉE ──reste nul──► SOLDÉE
  └──annuler (aucun versement imputé)──► ANNULÉE
```

Les états réglée et soldée se déduisent des imputations ; ils sont recalculables (R-20). Annuler une facture exige `invoices:cancel`, un motif, et aucun versement imputé ; c'est le cas d'une inscription abandonnée. L'abandon d'une inscription validée ne supprime rien : sa facture s'annule si rien n'est payé ; sinon les échéances non échues s'annulent par contre-passation et le payé reste acquis, comme le prévoit Q-34 pour un transfert.

### 4.4 Situation financière

Sur la fiche élève, une section « Frais » montre, pour chaque inscription : la facture, ses échéances avec leur dû, leur payé et leur reste, les remises, les versements avec leur reçu, et le reste dû total. Elle s'imprimera en relevé, modèle `releve_compte` avec QR, à l'étape 6 avec le reçu, dont il partage la mise en page.

### 4.5 Commandes

| Commande | Permission | Effet |
|---|---|---|
| `situation_financiere` | `students:read` | §4.4 |
| `echeancier_changer_plan` | `enrollments:create` | §3.1 bis |
| `releve_compte_produire` | `students:read` | PDF du relevé (étape 6) |
| `factures_rattraper_apercu` | `fees:manage` | Inscriptions validées sans facture et ce qui sera facturé |
| `factures_rattraper` | `fees:manage` | Sauvegarde, puis une transaction ; audit ; une `FactureEmise` par facture |

Une facture ne s'annule pas seule : une inscription validée sans facture serait une perte de recette (R-17). Elle s'annule avec l'abandon de son inscription, dans la même transaction (§4.3) ; la commande `facture_annuler` du projet est retirée.

---

## 5. Remises

### 5.1 Modèle et transitions (§10.10 du blueprint)

`reduction` : inscription, type (bourse, demi-bourse, fratrie, enfant du personnel, arrangement), en taux ou en montant, ligne de frais visée (la scolarité par défaut), motif obligatoire, proposée par, accordée ou refusée par, date.

```
PROPOSÉE ──accorder──► ACCORDÉE ──annuler──► ANNULÉE
    └────refuser────► REFUSÉE
```

Le secrétariat propose (`enrollments:create`), la direction accorde ou refuse (`invoices:discount`). Un utilisateur qui détient les deux propose et accorde d'un seul geste, comme l'inscription et la validation en phase 2.

### 5.2 Effet sur l'échéancier

Accordée, une remise réduit les échéances non soldées de la ligne visée, **en commençant par la dernière** (Q-52) : le parent qui a déjà payé la première tranche voit ses dernières tranches diminuer ou disparaître. Une remise ne crée jamais de reste dû négatif ; une remise supérieure au reste est refusée (`remise.superieure_au_reste`). Annulée, elle se contre-passe sur les mêmes échéances ; si des versements ont déjà été imputés au-delà du nouveau dû, l'annulation est refusée.

Une remise accordée avant la validation s'applique à la création de la facture.

### 5.3 Commandes

| Commande | Permission | Effet |
|---|---|---|
| `remise_proposer` | `enrollments:create` | Proposée ; audit ; `RemiseProposee` |
| `remise_accorder` | `invoices:discount` | §5.2 dans la transaction ; audit ; `RemiseAccordee` |
| `remise_refuser` | `invoices:discount` | Motif ; audit ; `RemiseRefusee` |
| `remise_annuler` | `invoices:discount` | Contre-passation ; audit ; `RemiseAnnulee` |
| `remises_en_attente` | `invoices:discount` | Remises proposées, pour la direction |

---

## 6. Sessions de caisse

### 6.1 Règles (§10.9 du blueprint)

```
OUVERTE ──clôturer (total compté)──► CLÔTURÉE ──rapprocher──► RAPPROCHÉE
```

- Une seule session ouverte par caissier. Le poste est unique (Q-13) : il n'y a qu'une caisse physique, mais deux caissiers qui se relaient ont chacun leur session.
- **Session libre** (T-21, réponse du 9 octobre 2026) : une session dure autant que le caissier le décide, un jour ou plusieurs ; il la clôture quand il remet l'argent. L'écran Caisse affiche depuis quand elle est ouverte ; l'état de clôture couvre toute sa durée, jour par jour.
- L'ouverture exige le fonds de caisse initial (`cashbox:open`).
- La clôture exige le total compté en espèces (`cashbox:close`). Le noyau calcule le total théorique : fonds initial, plus les encaissements en espèces, moins les sorties. Un écart non nul exige un motif. Les encaissements par Orange Money, Wave, Moov, chèque ou virement sont totalisés à part : ils n'entrent pas dans le tiroir.
- Une session clôturée ne se rouvre pas ; une correction passe par une régularisation datée du jour (§8).
- Le rapprochement (`cashbox:reconcile`) est le visa du comptable ou de la direction.

### 6.2 État de clôture

À la clôture, le modèle `etat_caisse` imprime : période de la session, fonds initial, encaissements par mode et par jour, sorties, total théorique, total compté, écart et son motif, liste des reçus de la session, signatures du caissier et du contrôleur, QR.

### 6.3 Commandes

| Commande | Permission | Effet |
|---|---|---|
| `caisse_session_courante` | connectée | La session ouverte de l'utilisateur, avec ses totaux |
| `caisse_ouvrir` | `cashbox:open` | Fonds initial ; audit ; `SessionCaisseOuverte` |
| `caisse_cloturer` | `cashbox:close` | Total compté, motif si écart ; audit ; `SessionCaisseCloturee` |
| `caisse_rapprocher` | `cashbox:reconcile` | Visa ; audit ; `SessionCaisseRapprochee` |
| `caisse_sessions_lister` | `cashbox:reconcile` | Historique, filtres par caissier et par date |
| `etat_caisse_produire` | connectée | PDF du §6.2 : le caissier imprime le sien, le titulaire de `cashbox:reconcile` tous |

---

## 7. Encaissement

### 7.1 Parcours UC-02, en moins de 60 secondes

Écran Caisse, tout au clavier :
1. recherche de l'élève, ou d'un payeur par son téléphone (recherche tolérante de la phase 2) ;
2. la situation s'affiche : reste dû, échéances en retard en tête ;
3. montant, mode (espèces par défaut), référence de transaction exigée pour un paiement mobile, un chèque ou un virement ;
4. payeur : un responsable payeur proposé, ou un nom libre (§3.6) ;
5. imputation proposée, la plus ancienne échéance d'abord, modifiable ligne par ligne ;
6. Entrée : le versement est enregistré, le reçu s'affiche et s'imprime.

### 7.2 Règles

- Aucun encaissement sans session ouverte de l'utilisateur (`caisse.session_fermee`).
- Montant strictement positif ; un montant supérieur au reste dû de l'inscription est refusé (`versement.superieur_au_reste`, T-18).
- La somme des imputations égale le montant ; aucune imputation ne dépasse le reste de son échéance.
- Numéro de reçu attribué dans la transaction par le service de numérotation (`REC-2026-000147`, R-46).
- **Idempotence** (§21.2, R-35) : l'écran tire une clé unique par encaissement ; une commande rejouée avec la même clé rend le même versement et le même reçu, sans rien écrire. C'est la garde contre le double clic et contre la coupure de courant au mauvais moment (§3.1).
- Les écritures financières sont en durabilité critique (`synchronous = FULL`, §3.1).

### 7.3 Plusieurs enfants d'une même famille

Un père paie souvent pour plusieurs enfants d'un coup. Recommandation (T-15) : l'écran Caisse propose, depuis un payeur, la liste de ses enfants avec leur reste dû ; le caissier répartit le montant. Le noyau crée **un versement et un reçu par enfant**, dans une seule transaction, imprimés à la suite. Chaque dossier d'élève garde ainsi ses propres pièces.

### 7.4 Reçu

Modèle `recu` (Typst, QR, R-45) : en-tête et logo de l'école, numéro, date et heure, élève, matricule et classe, payeur, montant en chiffres et en lettres, mode et référence, détail de l'imputation par échéance, reste dû après versement, caissier, mention « annulé » le cas échéant. Format recommandé (T-16) : **demi-page A5, deux exemplaires sur une feuille A4**, l'un pour le parent, l'autre en souche. Un format ticket de 80 mm pourra suivre.

La réimpression d'un reçu porte la mention « duplicata » (`documents:reprint`).

### 7.5 Commandes

| Commande | Permission | Effet |
|---|---|---|
| `encaissement_preparer` | `payments:create` | Situation et imputation proposée pour un montant |
| `encaisser` | `payments:create` | §7.2 ; audit ; `VersementEncaisse` ; reçu |
| `encaisser_famille` | `payments:create` | §7.3 ; un versement et un reçu par enfant |
| `recu_produire` | `students:read` | Reçu, ou duplicata |

---

## 8. Annulation de versement

- Motif obligatoire, permission `payments:cancel` (Administrateur et Comptable, §3.3).
- **Session encore ouverte** : le versement est annulé, ses imputations contre-passées, le total théorique de la session diminue.
- **Session clôturée** (Q-53) : l'annulation crée une sortie de régularisation dans la session ouverte de la personne qui annule, datée du jour et liée au versement ; elle exige donc que cette personne ait une session ouverte. La session d'origine ne bouge pas. Le tableau de bord de la direction signale toute annulation de ce type.
- Le reçu reste, marqué « annulé », avec le motif et l'auteur. Aucune suppression (R-14).
- Événement `VersementAnnule`.
- Précisions de réalisation (v1.2) :
  - la sortie porte le mode du versement ; seules les sorties en espèces diminuent le tiroir, et elles sont refusées si le total théorique de la session ouverte ne les couvre pas (`versement.especes_insuffisantes`) ;
  - la session d'origine garde le versement dans ses totaux, puisqu'il était dans le tiroir à sa clôture ;
  - le signalement à la direction est un avertissement (`avertissement.regularisations_a_viser`), montré à qui détient `cashbox:reconcile`, tant que la session où passe la sortie n'est pas visée ;
  - l'état de caisse passe en version 2 et liste les sorties de régularisation.

| Commande | Permission | Effet |
|---|---|---|
| `versement_annuler` | `payments:cancel` | Ci-dessus ; audit avant et après |

---

## 9. Impayés et journaux

### 9.1 Impayés (UC-05)

Écran « Impayés » : par classe ou pour l'école, les élèves dont une échéance est passée et non soldée. Colonnes : classe, élève, matricule, reste dû échu, reste dû total, échéance la plus ancienne, ancienneté, payeurs et leurs téléphones. Tranches d'ancienneté : 0 à 30 jours, 31 à 60, 61 à 90, plus de 90 (T-20). Totaux par classe. Export Excel et CSV par le composant `Tableau` (R-41).

### 9.2 Journal des encaissements

Par période, par caissier et par mode : reçus, montants, annulations. C'est l'export que demande le comptable externe (§2.2). Totaux par jour et par mode.

### 9.2 bis Précisions de réalisation (v1.3)

- Les impayés portent sur l'année en cours ; une échéance du jour compte comme échue, avec 0 jour de retard. L'ancienneté part de la plus ancienne échéance passée et non soldée.
- La liste se filtre par classe et par tranche à l'écran ; ligne cliquée, la fiche de l'élève s'ouvre pour la relance.
- Le journal range les versements par date d'encaissement, annulés compris et marqués ; ses totaux ne comptent que les versements valides, les annulés sont totalisés à part. Le filtre de caissier propose ceux de la période.

### 9.3 Commandes

| Commande | Permission | Effet |
|---|---|---|
| `impayes_lister` | `invoices:issue` | §9.1 |
| `encaissements_lister` | `cashbox:reconcile` | §9.2 |

---

## 10. UC-01 complet

L'assistant d'inscription de la phase 2 gagne une quatrième étape, « Frais » :
- le plan de paiement choisi par le parent, celui de l'école par défaut ;
- les frais facultatifs à cocher (cantine, transport…) ;
- une remise à proposer, accordée d'office si l'utilisateur détient `invoices:discount` ;
- après la validation, la facture et l'échéancier ;
- si l'utilisateur détient `payments:create` et a une session ouverte, le premier versement et son reçu sans quitter l'écran ; sinon, l'assistant indique que le parent passe à la caisse.

Dans une petite école, la même personne inscrit et encaisse (T-14) : un compte peut porter les rôles Secrétaire et Caissier. La règle de §11.5 du blueprint ne sépare que les notes et l'argent.

---

## 11. Modèle de données

Migrations, dans l'ordre des étapes :
- `0009_tarifs` : `grille_tarifaire`, `ligne_tarifaire`, `plan_paiement`, `tranche` ;
- `0010_factures` : `facture`, `ligne_facture`, `echeance` ;
- `0011_remises` : `reduction`, `reduction_echeance` (la part de remise portée par chaque échéance) ;
- `0012_caisse` : `session_caisse` (une seule session ouverte par caissier, garantie en base) ;
- `0013_versements` : `versement` (numéro de reçu et clé d'idempotence uniques), `imputation_versement` ;
- `0014_mouvements` : `mouvement_caisse` (sorties de régularisation, étape 7).

Montants en `INTEGER`, `CHECK (montant >= 0)` sauf sur les lignes de contre-passation, qui portent leur nature. Index du §8.9 du blueprint : `versement (inscription_id, date)`, `echeance (statut, date_echeance)`. Suppression physique refusée par déclencheur sur les tables financières (R-14), comme pour les inscriptions.

La finance référence `inscription_id`, jamais `eleve_id` (§8.3 du blueprint).

---

## 12. Événements

`GrilleActivee`, `FactureEmise`, `PlanPaiementChange`, `FactureAnnulee`, `RemiseProposee`, `RemiseAccordee`, `RemiseRefusee`, `RemiseAnnulee`, `SessionCaisseOuverte`, `SessionCaisseCloturee`, `SessionCaisseRapprochee`, `VersementEncaisse`, `VersementAnnule`. Noms au participe passé, version 1. `VersementEncaisse` et `RemiseAccordee` serviront au SMS de confirmation de la phase 5.

---

## 13. Essais de bout en bout et captures

Scénarios Playwright à ajouter :
- la direction saisit la grille de l'année et ses plans de paiement, puis la met en vigueur ;
- un parent change de plan avant tout paiement, puis le changement est refusé après le premier versement ;
- rattrapage des factures des inscriptions de la phase 2 ;
- **UC-02 chronométré** : la caissière ouvre sa caisse, encaisse un versement en espèces et un versement Orange Money avec référence, imprime les reçus, en moins de 60 secondes chacun ;
- encaissement pour trois enfants d'un même père ;
- remise proposée par la secrétaire, accordée par la direction ;
- annulation d'un versement par la direction ;
- clôture de caisse avec un écart motivé, état de caisse imprimé ;
- liste des impayés exportée en Excel ;
- **UC-01 complet** : inscription, frais, premier versement, reçu.

---

## 14. Tests de la phase

| Cible | Nature |
|---|---|
| Montant en lettres | Tables de référence : 0, 1, 21, 71, 80, 81, 100, 200, 1 000, 80 000, 1 000 000, 2 000 000, 999 999 999 |
| Arithmétique | Pourcentages, prorata, arrondi au multiple ; division avec reste, jamais d'arrondi implicite |
| Échéancier | Plans, reste à la dernière tranche, changement de plan avant et après paiement, arrivée en cours d'année selon les trois réglages |
| Imputation | Plus ancienne d'abord, répartition manuelle, refus des dépassements |
| Remises | Sur les dernières échéances, refus d'un reste négatif, contre-passation |
| Propriétés (R-35) | Somme des imputations = versement ; aucun montant négatif hors contre-passation ; total théorique = fonds + entrées − sorties ; rejouer un encaissement ne double pas la recette ; somme des imputations d'un élève = somme de ses versements non annulés |
| Transitions | Facture, remise, session de caisse, versement : tables exhaustives |
| Garde | Chaque nouvelle commande refusée sans sa permission |
| Audit | Chaque mutation écrit audit et `outbox` dans la même transaction |
| Bout en bout | Scénarios du §13, captures |

---

## 15. Ordre de réalisation

0. Correction du trousseau partagé (Q-48 bis, `ALERTES.md` du 9 octobre 2026)
1. Montants : toutes lettres, pourcentages, arrondi ; tests de propriétés
2. Grille de frais et plans de paiement ; permission `fees:manage`, rôles en version 3 ; écran Frais
3. Facture et échéancier à la validation ; choix et changement de plan ; arrivée en cours d'année ; situation financière ; rattrapage des inscriptions de la phase 2
4. Remises
5. Sessions de caisse et état de clôture
6. Encaissement, imputation, reçu ; idempotence ; famille ; UC-02 chronométré
7. Annulation de versement et régularisation
8. Impayés et journal des encaissements
9. UC-01 complet, navigation, captures, validation

---

## 16. Questions à trancher

Toutes tranchées le 9 octobre 2026 par le porteur du projet. Questions de terrain : T-11 à T-21 (`docs/questions-terrain.md`), recommandation retenue sauf T-12 (plans de paiement au choix du parent, §3.1) et T-21 (session de caisse libre, §6.1). Questions de conception : **toutes les recommandations sont retenues.**

| Id | Question | Décision | Étape |
|---|---|---|---|
| **Q-48** | Quelle permission pour saisir les frais ? | **`fees:manage`**, nouvelle, pour l'Administrateur, le Directeur et le Comptable ; rôles livrés en version 3 | L'étape 2 |
| **Q-48 bis** | Corriger le trousseau partagé avant la phase 3 ? | **Oui, en étape 0** : une entrée de trousseau par clé de données, en relisant l'ancien nom pour les postes installés ; sinon deux espaces de données sur un même compte Windows s'écrasent | L'étape 0 |
| **Q-49** | Valider une inscription sans grille de frais ? | **Non** : refus avec un message clair ; une grille à zéro reste possible pour une école qui ne facture pas un niveau | L'étape 3 |
| **Q-50** | Comment facturer les inscriptions validées en phase 2 ? | **Commande de rattrapage de la direction**, avec aperçu et sauvegarde préalable ; avertissement tant qu'il en reste | L'étape 3 |
| **Q-51** | La facture passe-t-elle par un brouillon ? | **Non : elle naît émise à la validation**, entièrement calculée ; le §10.2 du blueprint est amendé | L'étape 3 |
| **Q-52** | Sur quelles échéances une remise s'impute-t-elle ? | **Sur les dernières non soldées de la ligne visée, en remontant** | L'étape 4 |
| **Q-53** | Annuler un versement d'une session déjà clôturée ? | **Oui, par la direction, avec une sortie de régularisation dans la session du jour**, signalée au tableau de bord ; la session d'origine ne bouge pas | L'étape 7 |

---

## 17. Journal des révisions

| Version | Date | Modification |
|---|---|---|
| v1.3 | 2026-10-10 | Impayés et journal (§9.2 bis) : échéance du jour échue, filtres à l'écran, annulés hors totaux |
| v1.2 | 2026-10-10 | Annulation (§8) : sortie au mode du versement, espèces suffisantes exigées, avertissement jusqu'au visa, état de caisse v2 |
| v1.1 | 2026-10-09 | Frais saisis pour plusieurs niveaux à la fois (§3.1) ; facture née émise sans commande d'annulation propre (§4.5) ; migrations renumérotées (§11) |
| v1.0 | 2026-10-09 | Réponses du porteur du projet : plans de paiement au choix du parent (T-12, §3.1, §3.1 bis), session de caisse libre (T-21, §6.1) ; Q-48 à Q-53 retenues ; spécification engagée |
| v0.1 | 2026-10-09 | Création, soumise au porteur du projet avec T-11 à T-21 et Q-48 à Q-53 |
