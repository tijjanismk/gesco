# Données qui montent en ligne : des vues calculées par l'école, élève par élève

**Date** : 15 novembre 2026
**Statut** : proposé le 15 novembre 2026
**Précise** : décisions 7 et 9 de `docs/plan-phase-11.md` (§4) ; blueprint ADR-001, ADR-002,
ADR-009, §15.2, §24.3
**Spécification** : `SPEC_LOTS_C_D.md`

## Contexte

Le portail des parents (lot D) montre des données que seule l'école possède. Il faut décider ce
qui part du poste principal vers le serveur, et sous quelle forme.

Faits relevés dans le code le 15 novembre 2026 :

- Le journal `outbox` est écrit dans la transaction de chaque mutation (ADR-009), mais personne ne
  le lit, rien ne le purge, et ses événements n'ont pas été écrits pour reconstruire une copie :
  certains portent tout (un versement et ses imputations), d'autres seulement des identifiants.
- Un bulletin publié est figé : la table `bulletin` garde son `contenu`, sa version et son statut
  (`publie`, puis `remplace` quand une correction le remplace).
- La consultation par SMS (phase 5) calcule déjà, pour un responsable, le reste à payer, les
  absences et la moyenne du dernier bulletin publié, avec le rang si l'école le veut. C'est le même
  périmètre, en plus court, que celui que le porteur a retenu pour le portail (T-85 : ce que donne
  le SMS, plus les bulletins publiés et les reçus, en lecture seule).
- Le principe du porteur : « la flexibilité sans incohérence ». Un chiffre vu par un parent doit
  être celui que l'école voit sur son écran, au moment de l'envoi.
- La connexion de l'école est lente, coupée, payée au mégaoctet. Le blueprint (§15.2) ne fait
  monter aucun média par Internet.
- Le serveur ne doit pas contenir le moteur de calcul (`gesco-services`, Typst) : il dépend de
  `gesco-core` seul (ADR « hébergement »).

## Options

| | 1. Vues calculées par l'école, par élève | 2. Événements rejoués sur une copie du schéma | 3. Lignes de tables copiées, calcul au serveur |
|---|---|---|---|
| Apprentissage | Faible : les calculs existent, on les range dans un document | Élevé : chaque type d'événement à compléter, ordre, rejeu, schéma serveur | Moyen : choisir les tables, les tenir à jour, recalculer |
| Exploitation | Le serveur range et montre ; rien à recalculer | Rejeu à surveiller ; une erreur de rejeu fausse tout en silence | Le serveur recalcule : deux endroits où une règle peut diverger |
| Maturité | Mécanique simple (document, empreinte, remplacement) | Demande de reprendre les événements un par un | Demande un schéma serveur qui suit chaque migration |
| Pour un agent de code | Très bon : essais « même chiffre qu'à l'écran » directs | Mauvais : beaucoup de cas, essais longs | Moyen |
| Plus gros risque | Un oubli dans la vue (une donnée qui manque) | Un événement mal rejoué : un solde faux montré à un parent | Une règle de calcul différente au serveur |

Écartées sans tableau : **envoyer la base**, même chiffrée (le serveur n'en a pas la clé, et ce
serait faire sortir toute l'école pour montrer trois chiffres) ; **un serveur qui interroge
l'école à la demande** (l'école n'est pas joignable d'Internet et ne doit pas l'être).

## Décision

Le poste principal calcule, pour chaque élève concerné, une **vue** — un document qui contient
exactement ce que le parent verra — avec les mêmes fonctions que ses propres écrans ; il n'envoie
que les vues qui ont changé ; le serveur les range et les montre, sans rien recalculer.

| Brique | Version | Sortie | Suite prévue |
|---|---|---|---|
| Aucune dépendance nouvelle côté école | — | — | serde_json, sha2, flate2 sont déjà dans le dépôt |
| Client HTTPS de l'école vers le serveur | hyper 1.11.1 et rustls 0.23.46, déjà dans le dépôt | 28 août et 14 octobre 2026 | Suivent le réseau local |
| Certificats reconnus d'Internet : webpki-roots | série 1.0 ; 1.0.11 aujourd'hui | 23 octobre 2026 | 1.0.12 (10 novembre 2026) trop récente pour être fixée aujourd'hui ; prendre la dernière de la série à l'étape qui l'ajoute, et la tenir à jour à chaque version de GESCO (elle porte la liste des autorités) |

Vérifié le 15 novembre 2026 sur crates.io. Non retenue : `rustls-platform-verifier` 0.7.1
(24 septembre 2026), qui lit les autorités de Windows : un poste d'école jamais mis à jour peut ne
pas connaître l'autorité du serveur. Non vérifié : la taille réelle d'une vue sur l'école d'essai
(estimation : quelques kilo-octets compressés par élève, à mesurer à l'étape C2).

Ce que la décision fixe :

- **Une vue par élève**, au format versionné, dont les types vivent dans `gesco-core`
  (`en_ligne`), partagés par l'école et le serveur (ADR-002). Elle contient, pour l'année en
  cours : prénoms et nom, classe ; reste à payer et échéances à venir ; versements et leurs
  reçus (numéro, date, montant, mode, ce qu'ils ont réglé) ; absences ; bulletins et livrets
  publiés de l'année, dernière version seulement, tels qu'ils ont été figés à la publication,
  dans les limites que fixe la spécification ; la date et l'heure du calcul. Rien d'autre : ni
  date de naissance, ni adresse, ni photo, ni sanction, ni dossier médical, ni note non publiée.
- **Une vue par responsable** : ses numéros, l'empreinte de son code d'accès et la liste de ses
  enfants (voir l'ADR « identité des parents »).
- **Minimisation par construction** : seuls montent les élèves dont au moins un responsable a un
  code d'accès actif, dans une école qui a allumé le portail. Pas de code, rien ne monte.
- **Même calcul qu'à l'écran** : les vues sont calculées dans `gesco-services` par les fonctions
  qui servent la fiche de l'élève, la caisse et les bulletins. Un essai compare, élève par élève,
  la vue et l'écran.
- **Ce qui a changé** : le poste garde l'empreinte (SHA-256 du document) de la dernière vue
  acceptée par le serveur, dans une table locale nouvelle. À chaque envoi il recalcule les vues,
  compare les empreintes, n'envoie que les différentes. Le journal `outbox` ne sert plus que de
  signal (« quelque chose a changé depuis le dernier envoi ») ; ses lignes sont marquées
  consommées après un envoi réussi, puis purgées après 90 jours. Pour une école qui n'allume
  jamais le service, le journal est purgé de ce qui a plus d'un an (la trace des actions vit
  dans `journal_audit`, qui ne bouge pas).
- **Premier envoi** : toutes les vues de l'instant, calculées une fois ; l'historique du journal
  n'est pas rejoué.
- **Envoi** : lots compressés (gzip) de taille bornée, chacun idempotent (le serveur remplace la
  vue entière) ; un lot interrompu se renvoie tel quel ; compteur et volume affichés.
- **Ce qui disparaît** : chaque envoi complet porte la liste des élèves et responsables présents ;
  le serveur efface ce qui n'y est plus (élève parti, code révoqué, année close).
- **Maître unique** (§15.2, décision 9) : l'école écrit, le serveur lit. Les seules données qui
  descendent sont celles que le serveur crée et qui ne se modifient jamais : messages des parents
  (étape D4) et compteurs de consultation. Aucun conflit possible par construction. Avec
  plusieurs postes (lot A), seul le poste principal envoie.
- **`gesco-sync` ne revient pas** : le calcul est un module de `gesco-services`, le transport un
  client de `gesco-reseau`, qui a déjà hyper et rustls. Une crate de moins à compiler.
- **Pas de PDF** dans la 2.0, sous réserve de la réponse du porteur (spécification) : le portail
  montre les chiffres des reçus et des bulletins sous forme de pages lisibles sur un téléphone.

## Justification

Le parent doit voir le chiffre que voit l'école : le calculer une seule fois, au même endroit,
par le même code, est la seule façon de le garantir sans essais de concordance sans fin.
L'école connaît ses règles (tarifs, imputations, bulletins figés) ; le serveur n'a pas à les
connaître, ce qui garde `gesco-services` et Typst hors du serveur. Comparer des empreintes de vues
est simple, rejouable et sûr : un envoi raté se refait sans risque de double. Rejouer le journal
aurait demandé de reprendre chaque événement écrit depuis la phase 0.5, avec un risque de solde
faux montré à un parent. La minimisation par le code d'accès suit une règle déjà comprise par les
écoles : le parent qui a son code peut consulter, les autres non. Le volume reste petit : quelques
kilo-octets par élève, et seulement ce qui change.

## Conséquences

Positives : aucune règle métier sur le serveur ; un chiffre en ligne toujours égal à celui de
l'écran au moment de l'envoi ; un journal `outbox` enfin purgé ; aucun conflit possible ; aucune
crate nouvelle.

Négatives : les données en ligne ont l'âge du dernier envoi (l'écran du parent doit le dire) ;
recalculer toutes les vues d'une grande école prend un peu de temps au poste principal (à mesurer,
hors des heures de classe si besoin) ; chaque changement de forme de la vue demande une version de
format que le serveur et le portail comprennent ; le sens premier d'ADR-009 (le journal comme
source de la synchronisation) est abandonné.

## Contre-argument conservé

Le journal des événements a été écrit précisément pour cela, depuis la phase 0.5, et l'abandonner
gaspille cet investissement ; un rejeu d'événements permettrait un jour un fonctionnement à
plusieurs maîtres. Il n'a pas gagné parce que les événements ne suffisent pas à reconstruire les
données, que le fonctionnement à plusieurs maîtres est écarté (T-83), et que le risque d'un solde
faux montré à un parent pèse plus que l'élégance du rejeu.

## Conditions de reconsidération

- Une vue moyenne dépasse 20 Ko compressés, ou le premier envoi d'une école de 1 000 élèves
  dépasse 10 Mo : revoir le découpage (par rubrique plutôt que par élève).
- Le calcul de toutes les vues prend plus d'une minute sur le poste principal de l'école d'essai :
  partir du journal pour savoir quels élèves recalculer.
- L'e-learning en ligne (phase 12, mode cloud) ou une organisation à plusieurs écoles demande des
  données qui ne sont pas une vue par élève.
- Le porteur veut les PDF des bulletins et des reçus en ligne : mesurer leur poids et ajouter une
  rubrique de fichiers à la vue.
