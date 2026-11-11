# Sécurité du réseau local : certificat de l'école retenu à l'appairage

**Date** : 10 novembre 2026
**Statut** : proposé le 10 novembre 2026, accepté le 10 novembre 2026 par le porteur du projet — décision 3 de `docs/plan-phase-11.md`, étape A2
**Précise** : blueprint §17.4 (mode serveur local) ; s'appuie sur l'ADR « poste relié »

## Contexte

Dès que le poste principal écoute le réseau, ce qui circule entre les postes peut être lu ou
imité : mots de passe, codes PIN, données d'élèves mineurs, encaissements.

- Le Wi-Fi d'une école est souvent partagé (personnel, élèves, visiteurs, voisins qui ont le mot
  de passe). On ne peut pas le tenir pour sûr.
- Pas d'informaticien dans l'école, et pas d'Internet garanti : aucune autorité publique ne peut
  signer le certificat, rien ne doit expirer en silence un lundi de rentrée.
- Un poste relié peut être volé ou quitter l'école : il faut pouvoir le couper.
- Aujourd'hui une session ne dit pas de quel poste elle vient, et le nom du poste dans l'audit
  est le nom de l'ordinateur.
- Le poste garde déjà un secret dans le trousseau de Windows (`keyring-core`, enveloppe machine).
- Le 7 octobre 2026, une dépendance qui exigeait des outils de compilation absents du poste
  (OpenSSL, Perl) a dû être retirée : le choix de la brique de chiffrement doit l'éviter.

## Options

| | 1. Canal chiffré, certificat de l'école, empreinte retenue | 2. Canal en clair, jeton d'appairage seul | 3. Certificats des deux côtés, émis par le poste principal |
|---|---|---|---|
| Apprentissage | Moyen | Faible | Élevé |
| Exploitation | Un code à saisir une fois par poste | Idem | Certificats à émettre, renouveler, surveiller |
| Maturité | rustls, rcgen : répandus | — | Idem option 1, plus de code autour |
| Pour un agent de code | Bon : essais en Rust, deux instances | Bon | Moyen : beaucoup de cas d'expiration |
| Plus gros risque | Un faux poste principal au moment de l'appairage | Tout appareil du Wi-Fi lit les mots de passe et les données | Une école bloquée par un certificat expiré |

Écartées sans tableau : un réseau privé virtuel entre les postes (un logiciel de plus à installer
et à administrer) ; un chiffrement écrit pour l'occasion (on n'écrit pas sa propre cryptographie).

## Décision

Le poste principal fabrique son certificat ; chaque poste relié en retient l'empreinte lors d'un
appairage par code, reçoit un secret qui lui est propre, et tout passe ensuite par un canal
chiffré que le poste relié vérifie contre cette empreinte.

| Brique | Version | Sortie | Suite prévue |
|---|---|---|---|
| rustls | 0.23.46 | 14 octobre 2026 | Série 0.23 ouverte en 2024 ; pas de date de fin publiée |
| tokio-rustls | 0.26.6 | 28 septembre 2026 | Suit rustls 0.23 |
| rcgen | 0.14.10 | 28 août 2026 | Demande Rust 1.88, le projet est en 1.99 |
| ring (brique de chiffrement de rustls) | 0.17.14 | 11 mars 2025 | **À surveiller** : aucune sortie depuis vingt mois |

Vérifié le 10 novembre 2026 sur crates.io. Non vérifié : que tout cela compile sur le poste du
porteur sans outil supplémentaire (rien n'a été compilé) ; que `tokio-rustls` se branche sur le
serveur d'axum 0.8 sans crate d'appoint — `axum-server` 0.9.1, qui le ferait, est sortie le jour
même et n'est pas retenue.

Ce que la décision fixe :

- **Certificat** fabriqué par le poste principal à l'activation du réseau, de longue durée : la
  confiance vient de l'empreinte retenue, pas d'une date. Il vit avec sa clé dans la base
  chiffrée, donc dans les sauvegardes : un poste principal remplacé et restauré garde ses postes
  reliés. « Renouveler la clé du réseau » le refait et oblige à relier chaque poste de nouveau.
- **Appairage** : sur le poste principal, une personne qui a le droit de gérer les postes
  (droit nouveau) demande « Relier un poste » ; un code court s'affiche, valable quelques minutes
  et une seule fois, annulé après quelques essais faux. Le poste relié affiche une empreinte
  courte du poste principal, que l'on compare à l'œil avec celle de l'écran du poste principal
  avant de valider.
- **Secret du poste** : tiré au hasard à l'appairage, gardé dans le trousseau de Windows du poste
  relié ; le poste principal n'en garde qu'une empreinte, avec le nom donné au poste, qui l'a
  relié, quand, et sa dernière connexion.
- **Chaque appel** porte le secret du poste et le jeton de la session. Une session ne vaut que
  depuis le poste qui l'a ouverte.
- **Révocation** : un poste révoqué est refusé à l'appel suivant, ses sessions sont fermées.
- **Journal** : poste relié, poste révoqué, appairage raté et connexion refusée vont au journal
  d'audit ; le nom du poste dans l'audit et les événements est celui donné à l'appairage.
- **Écoute** : seulement si le poste est principal et que la direction l'a activée ; seulement
  sur les adresses du réseau privé ; essais de connexion comptés par poste.
- **Brique de chiffrement** : `ring`, qui se compile avec le seul compilateur C déjà présent,
  plutôt que celle que rustls propose par défaut (`aws-lc-rs`), qui demande d'autres outils sous
  Windows.

Durées, longueur du code et port se fixent dans la spécification ; la règle du pare-feu de
Windows se traite à l'étape de livraison.

## Justification

Les données sont celles de mineurs et le Wi-Fi d'une école n'est pas sûr : le clair est exclu.
Entre les deux façons de chiffrer, celle-ci ne demande à l'école qu'un code à recopier une fois
par poste, et rien ne s'y périme tout seul. Elle réutilise ce que le poste sait déjà faire :
garder un secret dans le trousseau, tenir un journal d'audit. Le secret propre à chaque poste
donne la révocation et un nom de poste qu'on ne peut pas usurper. Les certificats des deux côtés
ajouteraient un calendrier d'expirations à une école sans informaticien, pour un gain que le
secret de poste donne déjà.

## Conséquences

Positives : rien de lisible sur le réseau ; un poste volé se coupe en un clic et ne contient
aucune donnée ; l'audit dit de quel poste vient chaque écriture ; une restauration ne casse pas
les liaisons.

Négatives : une pile de chiffrement entre dans le dépôt (rustls, ring, rcgen), à compiler et à
tenir à jour ; quiconque a une sauvegarde et les douze mots a aussi la clé du réseau (il a déjà
toutes les données) ; la comparaison d'empreinte à l'œil repose sur la personne qui relie le
poste ; une table et un droit nouveaux.

## Contre-argument conservé

Dans une petite école où trois ordinateurs sont reliés par un câble, tout cela est lourd : un
simple jeton sur un réseau fermé suffirait. Il n'a pas gagné parce qu'on ne sait pas d'avance
quelle école a un réseau fermé (la question du réseau dans T-83 est restée sans réponse), et
qu'un mot de passe de directeur lu une fois sur le Wi-Fi donne tout.

## Conditions de reconsidération

- `ring` reste sans sortie ou reçoit une alerte de sécurité non corrigée : passer à l'autre
  brique de rustls, en vérifiant d'abord qu'elle se compile sur le poste.
- L'appairage s'avère trop difficile pour les écoles (essai du porteur sur deux machines).
- Des téléphones ou des navigateurs doivent se relier (voir l'ADR « poste relié ») : un
  certificat reconnu par les appareils devient nécessaire.
