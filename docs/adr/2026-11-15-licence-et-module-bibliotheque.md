# La bibliothèque dans toute licence, et un module de l'école allumé à la livraison

**Date** : 15 novembre 2026
**Statut** : proposé le 15 novembre 2026 — décision 13 de `docs/plan-phase-12.md` §5, lot 1 de la phase 12 (`SPEC_PHASE_12_LOT_1.md`)

## Contexte

- **T-92** (porteur, 11 novembre 2026) : l'e-learning est compris dans la licence, sans module
  vendu à part ni paquet payant.
- L'e-learning est « un pilier de l'application » (10 novembre 2026).
- **T-91** (14 novembre 2026) : le suivi des élèves par leurs maîtres est allumé à la livraison
  et réglable par chaque direction ; il ne bloque jamais. Il ne sert qu'à partir du portail
  (lot 4), mais son réglage doit exister dès que le module existe.
- **E-12** : module éteint ou paquet absent, GESCO se comporte comme avant ; les essais des
  phases précédentes passent sans changement.
- Ce qui existe : sept modules de l'école (`ModuleEcole`, `gesco-core/src/core/module.rs`), chacun
  allumé ou éteint par un réglage ; éteint, il quitte la barre latérale et le tableau de bord, ses
  données restent. La licence porte une liste `modules`, utilisée depuis le lot A pour les postes.
- Les écoles en 1.2 qui passent à la 2.0 trouvent déjà des nouveautés allumées (statistiques,
  livret du jardin) sans rien régler.

## Options

| | 1. Dans toute licence, module de l'école allumé | 2. Dans toute licence, module éteint à la livraison | 3. Module de la licence, vendu à part |
|---|---|---|---|
| Apprentissage | Nul | Nul | Moyen : un module de licence de plus |
| Exploitation | Rien à faire pour l'école | L'école doit savoir que la bibliothèque existe pour l'allumer | Une licence à refaire pour chaque école qui l'achète |
| Maturité | Même mécanisme que les sept modules | Idem | Mécanisme du lot A |
| Pour un agent de code | Bon | Bon | Moyen |
| Plus gros risque | Une école qui n'en veut pas voit une entrée de plus | Un pilier que personne ne voit | Contraire à T-92 |

## Décision

Option 1.

- **Licence** : aucune vérification de licence pour la bibliothèque ; elle est dans toute licence,
  y compris les cent licences libres. Le paquet livré n'est lié à aucune licence.
- **Module de l'école** : un huitième module, `Bibliotheque`, réglage `module.bibliotheque`,
  **allumé** pour une école neuve et pour une école qui passe de la 1.2 à la 2.0. La direction
  l'éteint dans l'écran des modules ; éteint, la bibliothèque quitte la barre latérale, le tableau
  de bord et la recherche, ses données et ses versions restent, et elle revient telle quelle.
- **Paquet absent ou refusé** : le module allumé montre la bibliothèque de l'école seule (ses
  ajouts) ; le reste de GESCO ne change pas.
- **Réglage du suivi** (T-91) : `bibliotheque.suivi_eleves`, booléen, **allumé** à la
  livraison, posé dès ce lot dans les réglages de la bibliothèque ; il ne commande rien avant le
  lot 4 et l'écran le dit en une ligne neutre (« Le suivi servira quand les élèves liront les
  cours sur leurs appareils »). Il ne bloque jamais ni l'élève ni le maître.
- **Pas de réglage pour publier sans validation** : la direction valide, et donne le droit de
  publier nommément à un maître (T-89) ; ce droit remplace un réglage général.

Aucune dépendance nouvelle.

## Justification

1. T-92 tranche la licence : rien à vérifier, rien à vendre.
2. Un pilier éteint à la livraison n'est vu par personne ; les nouveautés de la 2.0 arrivent
   allumées, la bibliothèque suit la même règle.
3. Le module de l'école garde la liberté de l'école qui n'en veut pas, avec le mécanisme connu
   depuis la version 1.1.
4. E-12 se tient par le module : éteint, aucune commande de la bibliothèque n'est proposée, et les
   essais des phases précédentes ne la voient pas.
5. Poser le réglage du suivi dès ce lot évite une migration de réglages au lot 4 et le montre à la
   direction tôt.

## Conséquences

Positives : rien à configurer ; aucune licence à refaire ; une école qui ne veut pas de la
bibliothèque l'éteint d'un geste.

Négatives : une école qui passe de la 1.2 à la 2.0 voit une entrée nouvelle dans sa barre
latérale ; un réglage (le suivi) existe avant de servir ; l'éditeur ne peut pas vendre la
bibliothèque à part sans un nouvel ADR.

## Contre-argument conservé

Éteindre le module à la livraison respecterait mieux l'école qui ne veut rien de nouveau, et
l'exigence E-12 serait tenue sans y penser. Il n'a pas gagné parce que le porteur fait de
l'e-learning un pilier, et qu'un pilier caché derrière un réglage que personne ne cherche ne sert
à rien.

## Conditions de reconsidération

- Si le porteur décide de vendre la bibliothèque, ou des paquets, à part.
- Si des écoles se plaignent de la bibliothèque allumée sans l'avoir demandée.
- Si le lot 4 montre que le suivi doit être réglé autrement (par classe, par maître).
