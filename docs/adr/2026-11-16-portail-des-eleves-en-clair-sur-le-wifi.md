# Portail des élèves : pages en clair sur le Wi-Fi de l'école, sans installation ni hors ligne

**Date** : 16 novembre 2026
**Statut** : proposé le 16 novembre 2026, accepté le 16 novembre 2026 par le porteur du projet
**Révise** : blueprint §16.5, point 1 (« application installable sur le Wi-Fi de l'école ») ;
répond à la condition laissée ouverte par l'ADR « poste relié » et l'ADR « sécurité du réseau
local » (10 novembre 2026) ; décision 2 de `docs/plan-phase-12.md` §5
**Spécification** : `SPEC_PHASE_12_LOT_4.md`

## Contexte

Le canal des postes reliés est chiffré par un certificat que l'école fabrique : un poste relié en
retient l'empreinte à l'appairage. Un navigateur de téléphone ne sait pas retenir une empreinte.

- **Ce que font les navigateurs** (règle connue, à mesurer sur un vrai téléphone à l'étape 4.1) :
  devant un certificat qu'aucune autorité n'a signé, ils affichent une page d'alerte à chaque
  appareil, souvent à chaque visite ; une page servie en clair sur une adresse du réseau local
  s'ouvre sans alerte, mais n'est pas un « contexte sécurisé » : pas de service worker, donc pas
  d'installation véritable ni de fonctionnement hors ligne. L'ajout d'un raccourci sur l'écran
  d'accueil reste possible.
- **Aucune autorité ne signe** un certificat pour une adresse privée ; un certificat reconnu
  demande un nom de domaine public, une demande par Internet tous les trois mois au plus, et un
  service de l'éditeur pour les renouveler. Beaucoup d'écoles n'ont pas Internet au poste
  principal.
- **Ce qui circule** sur le portail : des leçons que toute école reçoit, l'avancement d'un élève,
  ses réponses aux quiz, ses devoirs (lot 5), et son code d'entrée. Aucune donnée de gestion
  (ADR « portail séparé de la gestion »).
- **T-87** : l'élève lit aussi à la maison. Le portail sur le Wi-Fi ne s'atteint qu'à l'école ;
  la maison est couverte par le PDF à emporter (lot 1) et, plus tard, par le portail en ligne,
  qui aura un vrai certificat.
- Le lot 5 demande une photo prise par le téléphone : le champ de fichier du navigateur avec
  l'appareil photo marche aussi en clair.

## Options

| | 1. En clair sur le Wi-Fi, sans installation | 2. Certificat de l'école, alerte acceptée sur chaque téléphone | 3. Certificat reconnu pour un nom de l'éditeur, renouvelé par Internet |
|---|---|---|---|
| Apprentissage | Faible | Faible | Élevé : DNS, ACME, service de renouvellement |
| Exploitation | Rien | Expliquer l'alerte à chaque élève, chaque année | Internet tous les 90 jours au plus, un service et un domaine payés par l'éditeur |
| Maturité | Simple | Simple, mais déroutant | Bonne ailleurs ; inconnue dans une école sans Internet |
| Pour un agent de code | Très bon | Bon | Moyen : peu essayable hors ligne |
| Plus gros risque | Un appareil du Wi-Fi lit le code d'un élève qui entre | Les élèves apprennent à cliquer « continuer » sur une alerte de sécurité | Le portail cesse de marcher le jour où le certificat expire sans Internet |

## Décision

Sur le Wi-Fi de l'école, le portail des élèves est servi en clair (HTTP), sans service worker ni
installation ; il se garde en raccourci sur l'écran d'accueil ; ce qui doit tenir sans réseau
passe par le PDF.

Aucune brique nouvelle. Non vérifié le 16 novembre 2026 : le comportement réel d'un téléphone
Android récent et d'un ancien devant une page en clair sur une adresse privée (raccourci, appareil
photo, son) ; il se mesure à l'étape 4.1, avec l'essai de dix minutes prévu au §6 du plan.

Ce que la décision fixe, pour que le clair ne coûte rien de grave :

- **Rien de la gestion ne passe sur ce port** (ADR « portail séparé de la gestion »).
- **Le code d'un élève ne sert qu'au portail** : il ne donne ni nom, ni note, ni responsable ;
  volé, il permet de lire des leçons et de marquer un faux avancement. La direction en remet un
  autre d'un clic (ADR « identité de l'élève »).
- **Le jeton de session** n'est jamais dans l'adresse ; il vit dans un témoin `HttpOnly`,
  `SameSite=Strict`, limité au portail.
- **Le guide d'installation recommande** un Wi-Fi des élèves séparé de celui du personnel quand
  le routeur le permet ; le portail marche aussi sans.
- **Le portail en ligne**, plus tard, sera en HTTPS avec un vrai certificat, comme celui des
  parents ; cet ADR ne vaut que pour le Wi-Fi de l'école.

## Justification

Le clair n'expose que ce qui a peu de valeur, puisque la gestion n'est pas sur ce port. Une alerte
de sécurité montrée à des centaines d'élèves leur apprendrait le pire réflexe, pour protéger des
leçons publiques. Un certificat reconnu ferait dépendre le portail local d'Internet, ce qui
contredit le premier mode de diffusion du blueprint. L'installation et le hors ligne ne sont
perdus que sur le Wi-Fi : le PDF couvre la maison dès aujourd'hui.

## Conséquences

Positives : rien à configurer, rien qui expire ; marche sur tout navigateur ; essais simples.

Négatives : pas d'application installée ni de lecture hors ligne par le portail local ; un
appareil malveillant sur le même Wi-Fi peut lire un code d'élève ou une réponse de quiz ; le
blueprint §16.5 (point 1) est révisé, à reporter.

## Contre-argument conservé

Un élève qui a installé le portail retrouverait ses leçons à la maison sans rien demander : c'est
l'usage que T-87 décrit. Il n'a pas gagné parce que cela demande soit une alerte de sécurité
apprise par cœur, soit Internet au poste principal, et que le PDF et le futur portail en ligne
donnent la même chose sans ces deux prix.

## Conditions de reconsidération

- L'essai sur de vrais téléphones montre qu'une page en clair sur une adresse privée est bloquée
  ou marquée dangereuse par les navigateurs courants.
- Le service en ligne (lot C) existe et l'école a Internet : un nom par école avec un certificat
  reconnu devient possible sans service de plus.
- Le portail doit garder autre chose que des leçons (données de santé, notes) : le clair n'est
  plus admissible.
