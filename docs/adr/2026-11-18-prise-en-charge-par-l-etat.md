# Prise en charge par l'État : un partage calculé de chaque échéance et des versements de l'État portés par une recette

**Date** : 18 novembre 2026
**Statut** : proposé le 18 novembre 2026
**Précise** : décision du porteur du 18 novembre 2026 (« L'État, pas le parent »,
`docs/orchestration.md`) ; `SPEC_PHASE_9.md` §8.2 (migration 0056) ; `SPEC_PHASE_3.md` §4, §7 ;
`SPEC_PHASE_8.md` §5, §7 ; migration `0038_recettes` ; ADR « reprise avant GESCO » du
17 novembre 2026 ; R-14, R-20
**Spécification** : `SPEC_PRISE_EN_CHARGE_ETAT.md`

## Contexte

- Une case `inscription.prise_en_charge_etat` existe (lycée et technique), lue seulement par les
  rapports officiels. Le porteur veut que la scolarité d'un élève coché soit due par l'État, pas
  par le parent.
- Le reste à payer se calcule en un seul endroit, `facturation::etat_echeances`, lu par une dizaine
  de services (encaissement, impayés, relances SMS, consultation, guichet, documents remis,
  départs) ; une seule requête SQL calcule un payé (abonnés de la cantine).
- Une facture et ses échéances ne se réécrivent jamais ; les imputations des versements sont
  imprimées sur les reçus.
- L'État paie tard, par un virement groupé, parfois l'année d'après ; l'argent arrive en banque,
  pas à la caisse. La table `recette_tresorerie` porte déjà une recette hors des versements, dans un
  compte, sans session hors espèces, et la compte au livre, au rapprochement, au journal et au
  budget.
- Aucune dépendance n'est ajoutée : rien à vérifier sur endoflife.date ni crates.io.

## Options

| | 1. Partage calculé + prise en charge versionnée + versement de l'État lié à une recette | 2. Parts de l'État stockées par échéance et imputations de l'État stockées | 3. L'État comme payeur ordinaire : versements « État » à la caisse, remise pour le parent |
|---|---|---|---|
| Apprentissage | Moyen : un calcul de plus dans `etat_echeances` | Moyen : calqué sur les remises | Faible en apparence |
| Exploitation | Quatre tables ; le partage se recalcule à chaque lecture | Six tables ; chaque geste réécrit des parts | Aucune table, mais une remise fausse le sens des remises et l'État passe par la caisse |
| Maturité | Même forme que la reprise (version active unique, annuler et recréer) | Même forme que les remises | Contraire à la décision : le parent « doit » puis reçoit une remise |
| Pour un agent de code | Bon : un seul calcul, pur, testable par propriétés | Moyen : parts à tenir à jour partout | Mauvais : un virement groupé devient des dizaines de reçus et de passages en caisse |
| Plus gros risque | Une requête SQL future qui ignore le partage | Parts stockées désaccordées du calcul | Caisse du jour gonflée par un virement ; trésorerie comptée deux fois |

## Décision

Option 1, migration 0066 (ou la suivante libre) :

- `prise_en_charge_etat` : une ligne par version de la prise en charge d'une inscription
  (`en_cours`, `terminee`, `remplacee`), avec les natures de frais prises en charge, le montant
  attendu, l'abandonné, qui porte l'écart, la référence d'orientation, le motif ; une seule version
  courante par inscription (index unique partiel). La colonne de la case reste, écrite dans la
  même transaction, pour les rapports.
- `montant_etat` : montant attendu de l'État par année, niveau et série (vide = les frais de
  l'école) ; table de réglage, créée seulement si la réponse à T-164 est b).
- `versement_etat` : un versement de l'État (date, montant, référence, année concernée), lié à la
  `recette_tresorerie` qui porte l'argent, ou marqué « reçu avant GESCO » sans recette.
- `versement_etat_part` : ce qu'un versement attribue à une inscription (active ou annulée).

Le partage de chaque échéance (part de l'État, renoncé, part du parent), le placement de l'argent
de l'État et le report du trop-payé du parent **ne sont pas stockés** : ils se calculent dans
`etat_echeances` depuis ces tables (R-20). Aucune suppression (déclencheurs). Catégorie de recette
livrée « Versements de l'État ».

| Élément | Choix | Fin de support | Relais |
|---|---|---|---|
| Dépendances | Aucune nouvelle | — | — |

## Justification

1. « La facture n'est jamais réécrite » est tenu par construction : rien n'écrit dans `facture`,
   `ligne_facture`, `echeance` ni `imputation_versement`.
2. Garder `EcheanceAffichee.reste` comme reste du parent rend justes, sans les toucher, tous les
   lecteurs qui ne doivent parler qu'au parent.
3. Un partage calculé ne peut pas se désaccorder des remises, ajustements, plans et départs : il
   les relit à chaque fois. Stocker des parts (option 2) obligerait chaque geste à les réécrire.
4. Versionner la prise en charge (annuler et recréer) garde l'historique et un audit simple, comme
   la reprise.
5. Porter l'argent par une recette existante le compte une fois, au bon compte, sans session hors
   espèces, sans aucun changement des livres, du rapprochement, du journal et du budget.
6. Séparer l'argent (recette) de sa répartition (parts) permet de répartir plus tard, de corriger
   une liste, de répartir sur une année close, sans toucher à la trésorerie.
7. Un versement « reçu avant GESCO » sans recette respecte les soldes d'ouverture, comme la reprise.

## Conséquences

Positives : caisse et journal des encaissements inchangés par construction ; relances, impayés,
SMS et documents du parent justes sans y toucher ; un état de ce que doit l'État qui est la somme
exacte des restes des factures ; sans élève coché, chiffres identiques à ceux d'avant.

Négatives : `etat_echeances` fait plus de calcul (quelques élèves par facture, sans effet visible) ;
la liste des abonnés de la cantine doit passer par `etat_echeances` pour les inscriptions
concernées ; toute requête SQL future qui calcule un payé doit tenir compte du partage (l'essai qui
parcourt les requêtes le vérifie) ; la reprise se place désormais sur la part du parent seulement.

## Contre-argument conservé

« Stocker les parts de l'État, comme les parts de remise, rendrait chaque chiffre lisible en SQL et
l'historique du partage visible ligne par ligne. » Vrai ; mais chaque geste de la finance devrait
alors réécrire les parts, et un oubli laisserait deux vérités. L'historique utile (qui a coché,
quel montant attendu, quel abandon) est dans les versions de la prise en charge et l'audit.

## Conditions de reconsidération

- Un second tiers payeur apparaît (entreprise, ONG, collectivité) : généraliser
  `prise_en_charge_etat` en prise en charge par un tiers, avec une table des tiers.
- Un rapport doit lire le partage en SQL pour des milliers de factures et `etat_echeances` devient
  trop lent : stocker le partage comme une donnée dérivée, recalculée par `recalculer_dans`.
- L'État paie par élève avec un avis nominatif électronique : importer cet avis directement dans la
  répartition.
