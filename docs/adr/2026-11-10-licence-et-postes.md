# Licence et postes : deux formules, portées par les modules de la licence

**Date** : 10 novembre 2026
**Statut** : proposé le 10 novembre 2026, accepté le 10 novembre 2026 par le porteur du projet, qui retient **un poste relié à l'essai** quand la licence ne dit rien — décision 5 de `docs/plan-phase-11.md` (partie « postes »), avant l'étape 2 de `SPEC_PHASE_11_A.md`
**Précise** : blueprint §26.4 (modules optionnels) et §26.5 (mécanique de licence)

## Contexte

Le porteur du projet a répondu le 10 novembre 2026 à la question du prix : **« les deux
options »**. Une école pourra acheter soit une licence où les postes sont compris, soit des
postes à l'unité. Les prix sont l'affaire du porteur ; cet ADR dit seulement comment la licence
porte le nombre de postes et ce que GESCO en fait.

- Une licence porte déjà une liste libre de **modules** (`modules`), que l'outil de l'éditeur
  remplit par `--modules a,b`. Aucune version de GESCO ne la lit encore.
- Le contenu signé d'une licence **refuse tout champ inconnu** : une licence émise avec un champ
  nouveau serait illisible pour une 1.2.x.
- Cent licences libres (GESCO-0001 à 0100) sont déjà émises, sans module.
- Règle tenue depuis la phase 0.5 (`ALERTES.md`, blueprint §26.5) : **la licence ne bloque rien**,
  elle avertit ; une échéance ou un changement de machine ne coupe jamais l'accès aux données ; le
  contournement est un risque assumé.
- Les postes reliés sont comptés par le poste principal, qui garde leur liste dans la base.

## Options

| | 1. Mention dans les modules de la licence | 2. Champ nouveau, format de licence 2 | 3. Rien dans la licence |
|---|---|---|---|
| Apprentissage | Faible : le champ et l'outil existent | Moyen : deux formats à émettre et à lire | Nul |
| Exploitation | Réémettre une licence pour ajouter des postes | Idem, et choisir le bon format selon la version de l'école | Suivi des ventes à la main |
| Maturité | Mécanique de licence en service depuis la 0.11 | Idem, à étendre | — |
| Pour un agent de code | Bon : lire une chaîne, un essai par cas | Moyen : compatibilité entre formats à essayer | — |
| Plus gros risque | Une faute de frappe dans la mention à l'émission | Une licence neuve refusée par un poste resté en 1.2 | La formule par poste ne se vend pas : rien ne la fait respecter |

## Décision

Le nombre de postes est une mention parmi les modules de la licence, sans changer son format :
`postes:illimites` pour la formule « postes compris », `postes:N` pour la formule par poste ; le
poste principal refuse de relier un poste au-delà de ce nombre, et ne coupe jamais un poste déjà
relié.

Aucune dépendance nouvelle, donc aucune version à retenir.

Ce que la décision fixe :

- **Deux formules** : `postes:illimites`, ou `postes:N`, où N est le nombre de postes reliés
  achetés, en plus du poste principal.
- **Sans mention** (aucune licence, licence libre, licence déjà émise) : **un poste relié**, pour
  que l'école puisse essayer à deux postes avant d'acheter. *Point à confirmer par le porteur :
  un, ou aucun.*
- **Ce qui est refusé** : seulement le geste « Relier un poste » quand le nombre est atteint, avec
  un message qui dit combien de postes la licence couvre. Les postes en service comptent ; un
  poste révoqué rend sa place.
- **Ce qui n'est jamais coupé** : un poste déjà relié, même si la licence est échue, à réémettre
  ou retirée ; le poste principal ; les données. La règle « la licence ne bloque rien » tient
  pour tout ce qui existe.
- **Ajouter des postes** : l'éditeur réémet la licence avec un nombre plus grand ; l'école
  l'installe par-dessus, comme aujourd'hui.
- **Outil de l'éditeur** : `--modules postes:5` marche déjà ; il vérifiera la forme de la mention
  et refusera une faute de frappe.
- L'écran « Postes de l'école » montre le nombre de postes couverts et le nombre en service.

## Justification

Les deux formules demandées tiennent dans un champ qui existe et que personne ne lit encore :
rien à changer au format, et une licence émise pour la 2.0 reste lisible par un poste en 1.2.
Sans rien dans la licence, la vente par poste reposerait sur la seule parole. Le refus est placé
au seul endroit où il ne peut faire perdre ni donnée ni travail : l'ajout d'un poste. Tout ce qui
marche continue de marcher, ce que le blueprint exige pour protéger l'école plutôt que l'éditeur.
Un poste d'essai sans mention laisse voir le réseau avant de payer.

## Conséquences

Positives : aucune migration de licence ; les cent licences libres restent valables ; l'éditeur
vend l'une ou l'autre formule avec le même outil ; une école qui ne paie plus garde ses postes.

Négatives : c'est la première fois que la licence refuse un geste ; chaque poste ajouté demande
une licence réémise, donc un aller-retour avec l'éditeur ; une mention est du texte libre, plus
fragile qu'un champ typé ; qui contourne la licence contourne aussi ce nombre (risque assumé).

## Contre-argument conservé

La règle du projet est que la licence avertit et ne bloque rien : un bandeau « trois postes
reliés, licence pour deux » suffirait, et garderait la promesse entière. Il n'a pas gagné parce
qu'un avertissement ne fait pas payer un poste, et que le refus proposé ne touche à rien de ce
que l'école possède déjà.

## Conditions de reconsidération

- Les réémissions de licence pour un poste de plus deviennent une charge pour l'éditeur ou un
  frein pour les écoles : passer à des paliers (5, 10, illimité).
- Le porteur ne vend en pratique qu'une des deux formules : retirer l'autre.
- L'abonnement de l'édition Connectée (lot C) demande de toute façon un nouveau format de
  licence : y déplacer le nombre de postes dans un champ typé.
