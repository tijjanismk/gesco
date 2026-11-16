# Identité de l'élève dans le portail : matricule et code remis par l'école, ou lecture sans code

**Date** : 16 novembre 2026
**Statut** : proposé le 16 novembre 2026
**Précise** : décision 3 de `docs/plan-phase-12.md` §5 ; T-91 (suivi allumé, réglable, jamais
bloquant) ; exigence E-13 ; reprend la mécanique de l'ADR « identité des parents » (15 novembre
2026)
**Spécification** : `SPEC_PHASE_12_LOT_4.md`

## Contexte

Le suivi des élèves par leurs maîtres est allumé à la livraison et réglable par la direction
(T-91) ; il demande de savoir quel élève a lu quoi. Aujourd'hui l'élève n'a pas d'identité dans
GESCO : un matricule, pas de compte ni de code.

- Les élèves ont de 12 à 20 ans au 2e cycle et au lycée ; beaucoup n'ont pas de téléphone à eux et
  empruntent celui d'un camarade, d'un frère ou d'un parent ; en salle informatique, un même
  ordinateur sert à plusieurs élèves dans la journée.
- Ils oublient : un code perdu ne doit jamais empêcher de lire (« ne jamais bloquer »).
- La phase 5 a un code d'accès pour les responsables : six caractères d'un alphabet de 32 signes
  sans ambiguïté, empreinte Argon2id, un seul code actif, billet imprimé, essais comptés.
- Le poste principal est souvent un vieil ordinateur : soixante élèves qui entrent ensemble au
  début d'un cours ne doivent pas le figer.
- Ce que le portail montre est peu sensible (des leçons) ; ce que l'élève y laisse (avancement,
  réponses, copies au lot 5) concerne un mineur.

## Options

| | 1. Matricule et code de l'école, plus une entrée sans code | 2. Matricule et code seulement | 3. Sans nom : on choisit sa classe |
|---|---|---|---|
| Apprentissage | Faible : mécanique des parents | Faible | Très faible |
| Exploitation | Billets à imprimer et remettre | Idem ; élèves bloqués par un code oublié | Rien |
| Maturité | En service pour les parents | Idem | — |
| Pour un agent de code | Bon | Bon | Très bon |
| Plus gros risque | Un élève qui lit sans code échappe au suivi | Un élève sans code ne lit pas | Aucun suivi : T-91 n'est pas tenu |

## Décision

L'élève entre avec son matricule et un code de six caractères remis par l'école sur un billet ;
s'il n'a pas son code, ou si la direction a éteint le suivi, il peut toujours lire sans code, en
choisissant son niveau, sans que rien ne soit gardé.

| Brique | Version | Sortie | Suite prévue |
|---|---|---|---|
| argon2 | 0.6.0 (déjà dans le dépôt) | 27 août 2026 | Dernière stable sur crates.io le 16 novembre 2026 |
| rand, sha2 | déjà dans le dépôt | — | Suivent le poste |

Vérifié le 16 novembre 2026 sur crates.io. Aucune dépendance nouvelle.

Ce que la décision fixe :

- **Code** : six caractères, même alphabet que celui des responsables ; tiré au hasard ; l'école
  n'en garde que l'empreinte Argon2id, avec des paramètres plus légers que ceux des responsables
  (le code n'ouvre que le portail local, et la base est chiffrée) pour qu'une entrée coûte une
  vingtaine de millisecondes ; quatre vérifications à la fois au plus. Un seul code actif par
  élève ; « Remettre un code » remplace l'ancien.
- **Billet** : par classe, une feuille A4 de billets à découper (prénom, nom, classe, matricule,
  code, adresse du portail et son code QR). Le code n'est lisible qu'au moment où le billet est
  produit ; réimprimer, c'est remettre de nouveaux codes.
- **Valable tant que l'élève est inscrit** dans une classe de l'année en cours ; il le reste d'une
  année à l'autre ; un élève sorti, radié ou non réinscrit est refusé avec le message des codes
  faux.
- **Essais comptés** : cinq codes faux dans la journée pour un matricule bloquent ce matricule
  jusqu'au lendemain ; même réponse pour un matricule inconnu et un code faux. La porte « Lire
  sans code » reste ouverte.
- **Session** : jeton de 256 bits dans un témoin `HttpOnly`, gardé en mémoire par le poste
  principal (un redémarrage fait sortir tout le monde) ; fin au bouton « Sortir » et après une
  durée sans activité (T-135). Aucun mot de passe à choisir, aucun compte.
- **Ce que le portail montre de l'élève** : son prénom et sa classe, pour qu'il sache qu'il ne
  s'est pas trompé ; rien d'autre de sa fiche.
- **Lire sans code** : choix du niveau (et de la série au lycée) parmi ceux de l'école ; l'élève
  voit les contenus visibles du niveau, sauf les ajouts réservés à une classe ; ni avancement, ni
  tentative, ni devoir ne sont gardés.
- **Suivi éteint par la direction** : l'entrée par code reste possible (devoirs du lot 5), mais
  rien de l'avancement ni des quiz n'est écrit.
- **Journal** : billets produits, code remis, matricule bloqué vont au journal d'audit ; ni le
  code ni le jeton n'y paraissent.

## Justification

Le code remis en main propre est une identité déjà comprise par les écoles (codes des parents) :
aucun compte, aucun mot de passe oublié à réinitialiser. La porte sans code tient la règle « ne
jamais bloquer » : un élève qui a oublié son billet lit quand même, et seul son suivi manque ce
jour-là. Des paramètres légers d'Argon2id suffisent pour un code qui n'ouvre que des leçons, sur
une base déjà chiffrée, et laissent entrer une classe entière en quelques secondes.

## Conséquences

Positives : rien de nouveau à apprendre pour le secrétariat ; T-91 tenu sans bloquer ; un élève
parti perd l'accès sans geste de l'école.

Négatives : des billets à imprimer chaque rentrée pour les nouveaux ; un code qui circule entre
camarades fausse le suivi d'un élève ; le suivi n'est jamais complet, puisque l'on peut lire sans
code ; une table de codes de plus.

## Contre-argument conservé

Sans porte « sans code », le suivi serait complet, ce que T-91 semble demander (« on va obliger
le suivi des maîtres »). Elle a gagné parce que le porteur a confirmé le 14 novembre 2026 que le
suivi ne bloque jamais ni l'élève ni le maître : un élève refusé devant une leçon serait ce
blocage.

## Conditions de reconsidération

- Les maîtres constatent que la plupart des élèves lisent sans code : rendre le code plus simple
  (quatre chiffres, carte plastifiée) ou retirer la porte sans code, avec l'accord du porteur.
- Des codes sont essayés en masse depuis le Wi-Fi (journal) : durcir les essais comptés.
- Le portail en ligne arrive : l'identité de l'élève devra y suivre les règles de l'ADR des
  parents (jeton d'appareil, HTTPS).
