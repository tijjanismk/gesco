# Reprise avant GESCO : deux tables à part qui règlent des échéances sans encaissement

**Date** : 17 novembre 2026
**Statut** : proposé le 17 novembre 2026
**Précise** : décision du porteur du 17 novembre 2026 (`docs/orchestration.md`, commit `0dd1623`) ;
`SPEC_PHASE_3.md` §4, §6, §7 ; `SPEC_PHASE_8.md` §5 (soldes d'ouverture) ; migration
`0053_changement_de_classe` (avoirs) ; R-14, R-20
**Spécification** : `SPEC_REPRISE_AVANT_GESCO.md`

## Contexte

- Une école installe GESCO au deuxième mois de l'année. Des parents ont déjà payé une partie des
  frais, hors de GESCO. Le reste à payer doit être juste dès le premier jour.
- Le payé d'une échéance se calcule en un seul endroit, `facturation::etat_echeances` : versements
  imputés (`imputation_versement`) plus avoirs utilisés (`avoir_imputation`). Une seule requête
  SQL refait ce calcul (abonnés de la cantine).
- Un versement est daté de l'instant de sa saisie et entre dans la session de caisse ouverte ;
  il alimente l'état de caisse, le journal des encaissements, le livre des comptes et le journal
  de trésorerie. L'argent d'avant GESCO est déjà dans les soldes d'ouverture des comptes.
- Le porteur veut un seul montant par inscription, modifiable et annulable avec trace, réservé à
  la direction, visible à part, sans aucun effet sur la caisse.
- Aucune dépendance n'est ajoutée : rien à vérifier sur endoflife.date ni crates.io.

## Options

| | 1. Tables `reprise_avant_gesco` et `reprise_imputation` | 2. Un avoir d'origine « reprise » dans `avoir` | 3. Un versement « historique » hors session |
|---|---|---|---|
| Apprentissage | Faible : calqué sur `avoir` et `avoir_imputation` | Faible en apparence | Moyen : exceptions partout où un versement se lit |
| Exploitation | Une colonne de plus dans `etat_echeances`, une dans la requête de la cantine | `avoir.ajustement_id` est `NOT NULL` : reconstruction de la table ; un avoir ne s'annule pas | Chaque rapport de caisse, livre et journal doit l'exclure |
| Maturité | Même forme que les avoirs, essayés depuis la 1.2 | Idem | Contraire à §6 et §7 de la phase 3 (pas d'encaissement hors session) |
| Pour un agent de code | Bon : un seul calcul à étendre | Moyen : sens de l'avoir (trop-perçu) brouillé | Mauvais : un oubli d'exclusion compte l'argent deux fois |
| Plus gros risque | Une requête future qui oublie la reprise | Un avoir « reprise » utilisé comme trop-perçu | Un rapport de caisse faux, sans erreur visible |

## Décision

Deux tables nouvelles, migration 0065 : `reprise_avant_gesco` (une inscription, un montant, une
note, une origine, un statut actif ou annulé, le motif, le lien vers la reprise qui la remplace ;
une seule active par inscription par index unique partiel) et `reprise_imputation` (la part d'une
reprise portée par une échéance, active ou annulée). Aucune suppression (déclencheurs). La date
de la reprise n'est pas stockée : c'est la veille de la première session de caisse de l'année,
calculée.

| Élément | Choix | Fin de support | Relais |
|---|---|---|---|
| Dépendances | Aucune nouvelle | — | — |

## Justification

1. Le reste à payer se lit en un seul endroit : y ajouter une source de payé est le geste le plus
   petit et le plus sûr.
2. Une reprise n'est pas de l'argent reçu : elle ne doit toucher ni `versement`, ni la caisse, ni
   les comptes. Des tables à part le garantissent par construction, sans exclusions à oublier.
3. La forme est celle des avoirs (règlement sans encaissement), connue du code et des essais.
4. Un avoir dit « trop-perçu » ; le détourner mêlerait deux sens et demanderait de reconstruire
   `avoir` pour lever `NOT NULL`.
5. Modifier = annuler et recréer garde l'historique (R-14) et un audit simple.
6. L'index unique partiel tient « une seule reprise par inscription » en base, pas seulement dans
   le code.
7. Une date calculée ne peut jamais tomber après la première caisse : la cohérence voulue par le
   porteur sans aucun refus.

## Conséquences

Positives : caisse, livres, journaux et soldes inchangés par construction ; tous les écrans et
documents qui lisent `etat_echeances` deviennent justes sans être touchés ; aucune donnée existante
à transformer.

Négatives : deux tables de plus ; toute requête future qui calcule un payé en SQL doit lire
`reprise_imputation` (un essai parcourt les requêtes pour le vérifier) ; le changement de plan doit
annuler et réimputer la reprise dans sa transaction.

## Contre-argument conservé

« Un avoir fait déjà exactement cela ; une troisième source de payé est une source de plus à
oublier. » Vrai pour le calcul ; mais l'avoir ne s'annule pas, porte un ajustement obligatoire et
signifie un trop-perçu. L'essai qui parcourt les requêtes répond au risque d'oubli.

## Conditions de reconsidération

- Une école demande le détail des paiements d'avant (dates, modes, reçus papier) : il faudrait
  alors des lignes datées, peut-être des versements « hors caisse » tracés comme tels.
- Une troisième sorte de règlement sans encaissement apparaît : regrouper avoirs et reprises dans
  une seule table de règlements, avec leur nature.
