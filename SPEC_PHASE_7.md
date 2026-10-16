# SPEC_PHASE_7.md — Paie

**Projet** : ERP de gestion scolaire — Tauri 2 + Rust + React/TypeScript
**Phase** : 7 — paie mensuelle du personnel : salaires des engagements, heures visées des vacataires, primes, retenues paramétrées, avances, bulletins de paie, paiements en plusieurs fois
**Statut** : v1.1 — spécification engagée le 15 octobre 2026 après les réponses du porteur du projet à T-51 à T-60 et à Q-73 à Q-81 (§13) ; prorata au choix de l'école (Q-74), taux du Mali proposés à vérifier (Q-79)
**Documents parents** : `PROJECT_BLUEPRINT.md` v0.23 (§8.7, §9.2, §10.6, §11, Q-06), `ARCHITECTURE_RULES.md` v0.3 (R-11, R-14, R-15, R-17), `SPEC_PHASE_3.md` v1.0 (caisse), `SPEC_PHASE_6.md` v1.0 (personnel, engagements, heures), `docs/questions-terrain.md` (T-51 à T-60)
**Date** : 15 octobre 2026

---

## 0. Objet et critère de fin

### 0.1 Ce que produit cette phase

Le livrable du blueprint (§22.2) : **le bulletin de paie**. La paie s'appuie sur ce que la phase 6 enregistre : le salaire de base des engagements mensuels, le taux de l'heure des vacations, les heures visées de chaque mois, les absences du personnel, le moyen de paiement de chacun.

À la fin de la phase :
- chaque mois, la paie de l'école se calcule d'un geste : salaires des permanents, heures visées des vacataires, primes du mois, retenues que l'école pratique, remboursement des avances ;
- la direction revoit la paie, retient ou non les absences non justifiées, puis la valide : elle ne change plus ;
- chacun reçoit son bulletin de paie ; l'état de paie du mois se signe « pour acquit » ;
- un salaire se paie en une ou plusieurs fois, en espèces ou par mobile money ou banque, sans que rien ne bloque ; le reste dû se suit d'un mois sur l'autre ;
- une école tenue par son seul directeur fait toute la paie elle-même.

### 0.2 Décisions déjà prises qui s'appliquent

- **Rythme** (T-51) : une paie par mois, préparée en fin de mois, payée jusqu'au 5 du mois suivant ; les vacataires dans la même paie, sur leurs heures visées.
- **Mois payés** (T-52) : neuf mois dans certaines écoles, douze dans d'autres ; réglage de l'école, repris par chaque engagement mensuel (Q-73).
- **Retenues** (T-53) : pas dans toutes les écoles ; aucune règle fiscale ou sociale écrite dans le code (§8.7) : chaque retenue est un élément que l'école active et règle, avec sa date d'effet.
- **Primes** (T-54) : rares ; surtout saisies dans la paie du mois où on les donne.
- **Avances** (T-55) : retenues chaque mois jusqu'au solde.
- **Absences** (T-56) : jamais retenues d'office ; la direction décide, ligne par ligne.
- **Documents** (T-57) : bulletin de paie de chacun, état de paie à signer, état des retenues pour les déclarations.
- **Argent** (T-58, Q-06) : un salaire en espèces sort de la caisse, tracé, et deviendra une dépense en phase 8 par un événement explicite ; **rien ne bloque un paiement**.
- **Paiement en plusieurs fois** (T-59) : oui ; le reste dû se suit.
- **Qui fait quoi** (T-60) : calcul, validation et paiement séparés par des droits ; le Directeur peut tout faire, pour l'école tenue par lui seul.
- Au 2e cycle et au lycée, les enseignants sont surtout des vacataires payés à l'heure (remarque du porteur) : la paie des vacataires est le cas courant, pas l'exception.
- §10.6 du blueprint : une paie validée n'est plus recalculable ; une erreur se corrige par une régularisation sur une paie suivante, jamais par modification rétroactive. R-14 : rien ne se supprime ; un paiement saisi par erreur s'annule avec un motif.
- Montants en francs entiers (`Montant`), chaque ligne tronquée au franc (R-11).

### 0.3 Critère de fin, vérifiable

La phase est terminée quand ces dix assertions sont vraies, chacune démontrée par un test automatisé ou par un scénario Playwright :

1. La paie d'un mois rassemble chaque membre qui a un engagement payé ce mois-là : salaire de base au trentième des jours couverts, heures visées au taux de l'engagement ; un engagement de l'année scolaire ne paie pas les mois de vacances que l'école a réglés.
2. Une ligne d'heures visée n'est payée qu'une fois : visée après la validation de la paie de son mois, elle entre en rappel dans la paie suivante.
3. Une retenue inactive ne s'applique pas ; activée, elle s'applique aux natures d'engagement choisies, à son taux ou à son barème par tranches, avec son plafond, à partir de sa date d'effet ; la part patronale ne touche jamais le net.
4. Une avance se retient mois après mois jusqu'à son solde, sans jamais rendre un net négatif.
5. Les absences non justifiées du mois sont proposées au calcul et ne sont retenues que confirmées, au trentième du salaire par jour.
6. Une paie validée ne se recalcule plus ; une régularisation dans une paie suivante corrige une erreur.
7. Un bulletin validé se paie en une ou plusieurs fois ; un paiement en espèces de la caisse est une sortie de la session ouverte ; une caisse fermée ou un tiroir insuffisant avertit sans bloquer ; le reste dû se lit par membre et au tableau de bord.
8. Le bulletin de paie, l'état de paie à signer et l'état des retenues se produisent en PDF par le moteur documentaire.
9. Le Directeur calcule, valide et paie ; un compte sans droit de paie ne lit aucun bulletin, sauf les siens dans « Mon compte ».
10. Toute écriture de la paie est auditée, montants compris, avec l'événement correspondant ; les tests des phases précédentes passent sans changement.

## 1. Périmètre

**Inclus** : éléments de paie de l'école (primes, retenues salariales, cotisations patronales) ; mois payés par engagement ; paie du mois : calcul, revue, absences proposées, primes du mois, régularisations, validation ; avances et leur remboursement ; paiements en plusieurs fois, en espèces (de la caisse ou hors caisse), par mobile money ou par banque, annulation motivée ; bulletin de paie, état de paie à signer, état des retenues ; « Mes bulletins de paie » dans « Mon compte » ; droits et rôles en version 8 ; paie au tableau de bord.

**Exclu** :
- Dépenses, fournisseurs, budgets : phase 8. La paie émet l'événement `SalairePaye` que la phase 8 transformera en dépense (Q-06).
- Déclarations officielles aux organismes (formulaires de l'INPS, de la direction des impôts) : l'état des retenues donne les chiffres, la déclaration reste faite hors du logiciel.
- Virements bancaires générés (fichier de banque), paiement mobile money automatique : la référence du paiement se saisit (§3.11 du blueprint).
- Heures supplémentaires des permanents au pointage : sans pointage (T-43), une heure supplémentaire se saisit comme une prime du mois.
- Paie de plusieurs établissements à la fois, multi-devises.

---

## 2. Ce que la paie d'un mois rassemble

**Membres payés** — tout membre du personnel qui, pendant le mois, a un engagement en cours ou terminé dans le mois, sauf un engagement bénévole ; un membre parti dans le mois est payé jusqu'à son dernier jour.

**Salaire de base** (engagement à durée indéterminée, à durée déterminée, stage) — le montant mensuel de l'engagement ; un mois entièrement couvert le paie entier. Un mois couvert en partie se paie **selon le réglage de l'école** (`paie.prorata`, Q-74) :
- *au trentième* (défaut) : trente trentièmes moins un par jour du mois non couvert, jamais moins de zéro ;
- *aux jours du calendrier* : jours couverts ÷ jours du mois ;
- *aux jours ouvrables* : jours ouvrables couverts ÷ jours ouvrables du mois, du lundi au vendredi, le samedi compté si l'école le compte pour les congés (`conge.samedi_compte`).

Deux engagements successifs du même mois (nouveau salaire à une date, Q-65) se paient chacun sur ses jours. La retenue d'une journée d'absence suit la même règle (un trentième, un jour du calendrier ou un jour ouvrable du salaire de base).

**Mois payés** (T-52, Q-73) — chaque engagement mensuel est « payé douze mois » ou « payé sur l'année scolaire » ; le second ne paie pas les mois que l'école déclare mois de vacances (réglage `paie.mois_vacances`, juillet, août et septembre par défaut : neuf mois payés). Le choix par défaut d'un nouvel engagement est un réglage de l'école (`paie.douze_mois`, vrai par défaut) ; il se change sur chaque engagement, sans changer les paies validées.

**Heures des vacataires** — chaque ligne d'heures **visée** du mois, au taux de son engagement (montant de la phase 6, tronqué au franc) ; une ligne du mois encore en saisie n'est pas payée, et le calcul le signale. Une ligne visée après la validation de la paie de son mois entre dans la paie suivante, en rappel, avec son mois (Q-75) ; une ligne payée porte le bulletin qui l'a payée et ne se déverrouille plus.

**Primes et indemnités** — les éléments de gain attribués au membre et en vigueur ce mois (§3), plus ceux saisis dans la paie du mois.

## 3. Éléments de paie

**ElementPaie** — `code` (stable), `libelle`, `nature` (gain, retenue salariale, cotisation patronale), `calcul`, `natures_concernees` (natures d'engagement auxquelles il s'applique : toutes par défaut), `fonctionnaires` (s'applique aussi aux fonctionnaires, vrai par défaut), `ordre`, `actif`. Chaque version de son calcul porte une **date d'effet** ; une paie applique la version en vigueur le dernier jour de son mois ; une version passée ne change plus.

**Calcul** :
- *Montant fixe* : un montant par mois (une prime de transport, une cotisation forfaitaire) ;
- *Taux* : un pourcentage d'une assiette, avec un plafond d'assiette facultatif (une cotisation sociale) ;
- *Barème par tranches* : des tranches de l'assiette, chacune à son taux, cumulées (un impôt progressif) ; les bornes sont **mensuelles ou annuelles** : un barème annuel s'applique à l'assiette du mois multipliée par douze, et l'impôt trouvé se divise par douze ; il peut porter deux réductions, chacune facultative : une **réduction en points** (un pourcentage de l'assiette retranché de l'impôt, jamais au-dessous de zéro) et une **réduction pour charges de famille** (un pourcentage de l'impôt : tant pour un membre marié, tant par enfant à charge, jusqu'à un plafond) ;
- *Saisi* : le montant se saisit dans la paie du mois (une prime d'examen, une heure supplémentaire, une régularisation).

**Assiette** d'un taux ou d'un barème : le brut, diminué des retenues salariales que l'élément désigne (l'impôt sur le salaire après la seule cotisation de retraite, par exemple).

**Situation de famille** — la fiche du membre (phase 6) reçoit sa situation (célibataire, marié, veuf, divorcé) et son nombre d'enfants à charge, facultatifs ; sans eux, aucune réduction pour charges de famille.

**Livrés** (T-53, Q-79) : *Prime* (gain saisi), *Heures supplémentaires* (gain saisi), *Régularisation* (gain ou retenue saisis, motif obligatoire), *Retenue diverse* (retenue saisie) — actifs. Les retenues et cotisations du Mali sont livrées **inactives, avec les taux proposés ci-dessous, à vérifier par l'école ou son comptable avant de les activer** ; rien n'est écrit dans le code, tout se corrige par une nouvelle version datée :

| Élément | Nature | Calcul proposé | Source |
|---|---|---|---|
| Retraite INPS (part salariale) | retenue | 3,6 % du brut, sans plafond | CLEISS, taux au 1er janvier 2025 |
| Retraite INPS (part patronale) | cotisation patronale | 5,4 % du brut (vieillesse 3,4 %, invalidité et décès 2 %) | idem |
| Prestations familiales (INPS) | cotisation patronale | 8 % du brut | idem |
| Accidents du travail (INPS) | cotisation patronale | 1 % du brut (de 1 à 4 % selon le risque) | idem |
| AMO (part salariale) | retenue | 3,06 % du brut | idem |
| AMO (part patronale) | cotisation patronale | 3,5 % du brut | idem |
| Contribution à l'ANPE | cotisation patronale | 1 % du brut | idem |
| Impôt sur les traitements et salaires (ITS) | retenue | barème annuel sur le brut diminué de la retraite INPS : 0 % jusqu'à 330 000 F, 5 % jusqu'à 578 400 F, 12 % jusqu'à 1 176 400 F, 18 % jusqu'à 1 789 733 F, 26 % jusqu'à 2 384 195 F, 31 % jusqu'à 3 494 130 F, 37 % au-delà ; réduction de 2 points de l'assiette ; charges de famille : 10 % marié, 2,5 % par enfant, plafond 35 % | Code général des impôts, art. 10 (loi 2016-010) ; réduction de 2 points (loi 2008-002) |

Les cotisations s'appliquent par défaut aux engagements payés (hors bénévoles), sauf aux fonctionnaires, déjà couverts par l'État ; l'ITS s'applique à tous. L'école renomme, ajoute et retire ses éléments ; un élément retiré reste sur les bulletins qui le portent.

**Attribution** — un gain ou une retenue à montant fixe peut s'attribuer à un membre, avec son montant propre, une date de début et une fin facultative (une indemnité de logement, une retenue convenue) ; il entre dans chaque paie de ses mois. Les primes étant rares (T-54), l'usage courant reste la saisie dans la paie du mois.

## 4. Avances

**Avance** — `membre`, `montant`, `le`, `mensualites` (nombre de mois de remboursement), `premier_mois`, `motif`, `paiement` (comme un salaire, §7 : mode, caisse ou hors caisse, référence), `annulee`. Chaque paie à partir du premier mois retient la mensualité (montant ÷ mensualités, tronqué ; la dernière solde l'avance), jamais plus que le reste dû ni que le net disponible (Q-76) : le surplus se reporte. Le bulletin de paie montre le reste dû de chaque avance. Une avance non encore remboursée s'annule avec un motif ; une avance remboursée en partie se solde par une régularisation.

## 5. Paie du mois

**PaieMois** — `mois`, `statut`, `calculee_le`, `calculee_par`, `validee_le`, `validee_par`, `note`. Une par mois et par établissement. États : **calculée → validée** (table exhaustive) ; le paiement se suit par bulletin.

- **Calculer** (`payroll:calculate`) : pose ou refait les bulletins de tous les membres payés du mois, d'après les engagements, les heures visées, les éléments et les avances ; garde ce que la direction a saisi (primes du mois, absences confirmées, régularisations). Le calcul signale : heures du mois encore en saisie, membres sans moyen de paiement, retenue active sans taux.
- **Revoir** : pour chaque membre, les lignes du bulletin ; la direction ajoute une prime ou une régularisation (motif obligatoire), ou une retenue saisie ; elle voit les **absences non justifiées du mois** (journées, demi-journées) et les **confirme ou non** une à une : une absence confirmée devient une retenue au trentième du salaire de base par jour, la moitié pour une demi-journée (T-56). Rien n'est retenu sans confirmation. Un retard ne se retient pas.
- **Valider** (`payroll:validate`) : la paie se fige, chaque bulletin reçoit son numéro (`PAI-AAAA-NNNNNN`, le préfixe livré du compteur des bulletins de paie), les lignes d'heures payées sont marquées, les avances avancent. Une paie validée ne se recalcule plus et ne se dévalide pas (§10.6) ; une erreur se corrige par une régularisation dans une paie suivante.
- **Un membre arrivé après la validation** (engagement saisi en retard) entre dans la paie suivante, en rappel du mois manqué.
- Pas de paie d'un mois à venir ; les mois se suivent : on ne valide pas un mois tant que le précédent, s'il a été calculé, n'est pas validé.

## 6. Calcul d'un bulletin

Dans l'ordre, chaque ligne tronquée au franc :
1. **Gains** : salaire de base au trentième, heures visées du mois et rappels, primes attribuées, gains saisis ; leur somme est le **brut**.
2. **Retenues salariales** actives et applicables, dans leur ordre, chacune sur son assiette ; puis les retenues d'absence confirmées et les retenues saisies.
3. **Avances** : la mensualité de chaque avance, sans dépasser ce qui reste.
4. **Net à payer** = brut − retenues − avances ; jamais négatif : si les retenues l'exigeaient, la retenue d'avance se réduit d'abord (et se reporte), puis le calcul s'arrête sur un avertissement à corriger (Q-76).
5. **Cotisations patronales** : sur leur assiette, pour l'état des retenues ; elles ne touchent pas le net.
6. **Cumuls** de l'année civile, de janvier au mois de la paie : brut, retenues, net (Q-78).

Le moteur est dans `gesco-core` (`paie/`), sans aucune règle d'un pays : il applique des éléments ; des cas de référence chiffrés, calculés à la main, le vérifient (§15).

## 7. Paiement

**PaiementSalaire** — `bulletin`, `montant`, `le`, `mode` (espèces, Orange Money, Wave, Moov Money, chèque, virement), `reference` (exigée hors espèces, comme les versements), `source` (de la caisse, hors caisse), `session` (la session de caisse, pour la caisse), `par`, `annule`, `motif_annulation`.

- Un bulletin **validé** se paie en une ou plusieurs fois (T-59), jamais au-delà de son net ; il est *à payer*, *payé en partie* ou *payé*. Le mode proposé est celui de la fiche du membre.
- **En espèces de la caisse** : une sortie dans la session de caisse ouverte de qui paie, comptée dans son total théorique ; si le tiroir n'a pas assez, GESCO **avertit sans bloquer** (T-58). **Hors caisse** : la direction paie sans passer par la caisse (l'école sans caissier, l'argent d'une autre source) ; le paiement est noté, sans mouvement de caisse. Sans session ouverte, seul « hors caisse » est possible pour les espèces, avec un avertissement : jamais un blocage (Q-77).
- **Tout payer d'un geste** : pour l'école tenue par son directeur, « payer le reste du mois » paie chaque bulletin non soldé de son reste, selon le mode de chacun, après une confirmation qui annonce le total et le nombre de personnes.
- **Annulation** : un paiement saisi par erreur s'annule avec un motif ; s'il était sorti de la caisse, une entrée de régularisation le remet dans la session ouverte de qui annule. Le bulletin redevient à payer d'autant.
- **Reste dû** : par membre, tous mois confondus ; les salaires encore dus du mois et des mois précédents paraissent au tableau de bord.
- Chaque paiement émet `SalairePaye` (montant, mode, source, membre, mois) : la phase 8 en fera une dépense, par cet événement, jamais par un déclencheur de base (R-15, Q-06).

## 8. Documents

- **Bulletin de paie** (modèle `bulletin_paie`, A4 par défaut, format réglable) : école, membre (matricule, fonction, catégorie, numéro INPS et de fonctionnaire s'ils existent), mois, numéro ; gains (base et jours payés, heures et taux, primes), retenues (assiette, taux, montant), avances (mensualité, reste dû), net à payer, paiements reçus et reste à payer ; cumuls de l'année. Il s'imprime validé ; un bulletin calculé s'imprime marqué « projet ».
- **État de paie du mois** (modèle `etat_paie`, A4 paysage) : une ligne par membre, brut, retenues, avances, net, déjà payé, reste ; une colonne « signature pour acquit » ; totaux.
- **État des retenues** (modèle `etat_retenues`) : par élément de retenue ou de cotisation, une ligne par membre concerné : assiette, taux, part salariale, part patronale ; totaux par organisme.

## 9. Droits

| Droit | Ce qu'il permet |
|---|---|
| `payroll:calculate` | régler les éléments et les avances, calculer et revoir la paie, saisir primes, régularisations et absences retenues |
| `payroll:validate` | valider la paie du mois |
| `payroll:pay` | payer, annuler un paiement, payer le reste du mois d'un geste |

Lire la paie et ses documents demande l'un de ces droits ; ils montrent des montants, comme `contracts:read`. Rôles livrés en **version 8** (T-60, Q-80) : le Directeur reçoit `payroll:calculate` et `payroll:pay` (il avait `payroll:validate`) ; le Comptable reçoit `payroll:calculate` et `payroll:pay` ; le Gestionnaire RH garde `payroll:calculate` et `payroll:pay`. Le caissier ne paie pas les salaires : la paie montre les nets de chacun. Un compte lié à un membre lit **ses propres bulletins de paie validés** dans « Mon compte », sans aucun droit.

## 10. Tableau de bord

Au bloc « Personnel », avec un droit de paie : la paie du mois écoulé (calculée, validée ou à faire), le nombre de bulletins payés sur le total, le net restant à payer du mois et des mois précédents ; un avertissement si la paie du mois écoulé n'est pas validée le 5.

## 11. Modèle de données

Une migration par étape, dans l'ordre du §16 : `element_paie` et ses versions (calcul, date d'effet), `attribution_element` ; colonne `paye_douze_mois` sur `engagement` ; colonnes `situation_famille` et `enfants_a_charge` sur `membre_personnel` ; `avance` ; `paie_mois`, `bulletin_paie`, `ligne_bulletin_paie` (élément, libellé, base, taux, montant, sens, mois d'origine d'un rappel), `absence_retenue` ; colonne `bulletin_paie_id` sur `heures_vacation` ; `paiement_salaire` et colonne `paiement_salaire_id` sur `mouvement_caisse`. Rien ne se supprime (R-14) : bulletins, lignes et paiements ne se modifient plus après la validation, déclencheurs de non-suppression comme ailleurs ; un élément retiré reste.

## 12. Événements

`ElementPaieModifie`, `AvanceAccordee`, `AvanceAnnulee`, `PaieCalculee`, `PaieValidee`, `SalairePaye`, `PaiementSalaireAnnule`.

## 13. Questions de conception

| Id | Question | Recommandation | Étape |
|---|---|---|---|
| **Q-73** | Mois payés d'un engagement mensuel ? | **Douze mois, ou ceux de l'année scolaire** : choix par engagement, défaut réglé par l'école (douze) ; mois de vacances réglés par l'école (juillet à septembre : neuf mois payés) | 1 |
| **Q-74** | Mois incomplet (arrivée, départ, nouveau salaire) ? | **Au choix de l'école** (réponse : « flexible ») : au trentième par défaut, aux jours du calendrier ou aux jours ouvrables ; la retenue d'une absence suit la même règle | 1 |
| **Q-75** | Heures visées après la paie de leur mois ? | **Rappel dans la paie suivante**, avec leur mois ; une ligne n'est payée qu'une fois | 3 |
| **Q-76** | Retenues plus fortes que le brut ? | Net jamais négatif : l'avance se réduit et se reporte d'abord ; sinon un avertissement bloque la validation de ce bulletin jusqu'à correction | 2 |
| **Q-77** | Espèces sans caisse ouverte, tiroir insuffisant ? | **Jamais bloqué** : de la caisse si une session est ouverte (avertissement si le tiroir ne suffit pas), sinon hors caisse, noté et signalé | 5 |
| **Q-78** | Cumuls du bulletin ? | **Année civile**, de janvier au mois de la paie | 6 |
| **Q-79** | Taux de l'INPS, de l'AMO, de l'ITS ? | **Livrés inactifs, avec les taux du Mali proposés** (§3), à vérifier par l'école avant de les activer ; rien d'écrit dans le code | 1 |
| **Q-80** | Rôles en version 8 ? | Directeur et Comptable reçoivent `payroll:calculate` et `payroll:pay` ; le Caissier, aucun droit de paie | 7 |
| **Q-81** | Corriger une paie validée ? | **Jamais la dévalider** : régularisation (gain ou retenue, motif obligatoire) dans une paie suivante ; un paiement erroné s'annule avec un motif | 3 |

## 14. Essais de bout en bout et captures

- la direction règle douze mois payés, active la cotisation INPS (part salariale et patronale) avec un taux ;
- une avance de 30 000 F à la secrétaire, en trois mois ;
- la paie d'octobre : la secrétaire et la cuisinière au mois, le vacataire sur ses heures visées, une prime d'examen saisie, une absence non justifiée proposée puis confirmée ;
- validation ; état de paie à signer ; bulletins de paie en PDF ;
- paiement en espèces de la caisse, un salaire payé en deux fois, le reste dû au tableau de bord ;
- la secrétaire lit son bulletin de paie dans « Mon compte » ;
- les heures de novembre visées après la paie de novembre, payées en rappel en décembre.

## 15. Tests de la phase

| Cible | Nature |
|---|---|
| Moteur | Trentième, mois payés, heures et rappels, taux et plafond, barème par tranches, assiette après retenues, avance plafonnée et reportée, net jamais négatif, cumuls : cas de référence calculés à la main |
| Transitions | Paie du mois, avance, paiement : tables exhaustives |
| Unicité | Une ligne d'heures payée une fois ; une paie par mois ; un mois validé après le précédent |
| Paiement | Plusieurs fois, jamais au-delà du net, caisse ou hors caisse sans blocage, annulation et entrée de régularisation |
| Droits | Directeur seul de bout en bout ; aucun bulletin sans droit de paie, sauf les siens ; mise à niveau des rôles en version 8 |
| Audit | Avant et après sur chaque écriture, montants compris |
| Bout en bout | Scénarios du §14, captures |

## 16. Ordre de réalisation

1. Éléments de paie et leurs versions, attributions, éléments du Mali proposés ; mois payés des engagements ; prorata ; situation de famille ; écrans de réglage
2. Moteur de calcul d'un bulletin dans `gesco-core`, cas de référence
3. Paie du mois : calcul, revue, primes et régularisations, absences confirmées, validation, rappels d'heures
4. Avances
5. Paiements : plusieurs fois, caisse ou hors caisse, annulation, reste dû, payer le reste du mois
6. Documents : bulletin de paie, état de paie à signer, état des retenues ; « Mes bulletins de paie »
7. Droits et rôles en version 8 ; tableau de bord
8. Navigation, captures, école d'essai, validation

## 17. Journal des révisions

| Version | Date | Modification |
|---|---|---|
| v1.0 | 2026-10-15 | Création après les réponses du porteur du projet du 15 octobre 2026 : paie en fin de mois jusqu'au 5 (T-51), neuf ou douze mois (T-52), retenues facultatives (T-53), primes rares (T-54), avances (T-55), absences confirmées par la direction (T-56), trois documents (T-57), sortie de caisse sans blocage (T-58), paiement en plusieurs fois (T-59), le directeur fait tout (T-60) |
| v1.1 | 2026-10-15 | Réponses du porteur aux choix : Q-73, Q-75, Q-80 retenues ; prorata au choix de l'école (Q-74, « flexible ») ; taux du Mali proposés, inactifs, à vérifier (Q-79) : barème annuel, réductions en points et pour charges de famille, assiette diminuée des retenues désignées, situation de famille sur la fiche ; spécification engagée |
