# Identité des parents en ligne : le code d'accès relie un téléphone, une fois

**Date** : 15 novembre 2026
**Statut** : proposé le 15 novembre 2026
**Précise** : décision 8 de `docs/plan-phase-11.md` (§4) ; blueprint §17.1, §17.5, §25.4 ;
`SPEC_PHASE_5.md` §9 (code d'accès et consultation par SMS)
**Spécification** : `SPEC_LOTS_C_D.md`

## Contexte

Un parent doit entrer dans le portail sans compte à créer ni mot de passe à retenir, et personne
d'autre ne doit voir les données de son enfant.

Ce qui existe (vérifié dans le code le 15 novembre 2026) :

- Le **code d'accès** du responsable (phase 5) : six caractères tirés d'un alphabet de 32 signes
  sans ambiguïté (`K7P X3M`), soit environ un milliard de codes ; **remis en main propre au
  guichet** sur un billet imprimé ; un seul code actif par responsable ; l'école le révoque ou en
  remet un autre ; la base n'en garde qu'une empreinte Argon2id.
- La consultation par SMS l'utilise déjà avec le numéro du responsable : cinq codes faux dans la
  journée bloquent le numéro jusqu'au lendemain ; une réponse par jour au plus pour un code faux.
- Les parents ont des téléphones Android pour la plupart, souvent partagés dans la famille, et
  changent de numéro moins souvent que de téléphone.
- Aucun envoi de SMS depuis Internet : l'école envoie par sa clé modem ; le serveur n'a aucune
  passerelle SMS, et en ajouter une coûte par message.
- Sur Internet, un code de six caractères n'est plus protégé par le coût d'un SMS : un robot peut
  essayer vite et beaucoup. Et si le serveur était volé, un milliard de codes se parcourent en peu
  de temps sur des machines puissantes, même avec Argon2id.

## Options

| | 1. Numéro et code à chaque entrée | 2. Numéro et code pour relier le téléphone, puis jeton d'appareil | 3. Second contrôle : SMS envoyé par le serveur, ou mot de passe choisi |
|---|---|---|---|
| Apprentissage | Faible | Faible à moyen | Moyen |
| Exploitation | Rien de plus | Appareils à compter et à révoquer | Passerelle SMS payante, ou mots de passe oubliés à réinitialiser |
| Maturité | Mécanique du SMS déjà en service | Jeton de session classique | Classique, mais coûteux ici |
| Pour un agent de code | Très bon | Bon | Moyen |
| Plus gros risque | Le code tapé souvent, vu par l'entourage, essayé souvent | Un téléphone volé et non verrouillé | Coût par message ; parent bloqué par un mot de passe oublié, renvoyé au guichet |

Écartée sans tableau : **un compte avec adresse électronique** (beaucoup de parents n'en ont pas
ou ne la lisent pas).

## Décision

Le parent relie son téléphone une fois, avec son numéro et son code d'accès ; le téléphone reçoit
un jeton d'appareil qui vaut ensuite pour lui seul ; le code ne se retape qu'au changement de
téléphone ou de code.

| Brique | Version | Sortie | Suite prévue |
|---|---|---|---|
| argon2 (vérifier l'empreinte venue de l'école) | 0.6.0, déjà dans le dépôt | 27 août 2026 | Suit le poste |
| hmac (numéro rangé sans être lisible) | 0.13.0, déjà dans `Cargo.lock` | — | Devient une dépendance directe du serveur |
| sha2, rand, chacha20poly1305 | déjà dans le dépôt | — | Suivent le poste |

Vérifié le 15 novembre 2026 : argon2 sur crates.io ; hmac dans `Cargo.lock`. Aucune dépendance
nouvelle.

Ce que la décision fixe :

- **Ce qui monte de l'école**, par responsable qui a un code actif : son identifiant, ses
  numéros (forme internationale), l'empreinte Argon2id de son code telle que l'école la garde,
  la liste de ses enfants. Le code lui-même ne quitte jamais l'école ; il n'y est d'ailleurs
  pas gardé en clair.
- **Ce que le serveur garde** : le numéro sous forme d'empreinte à clé (HMAC-SHA-256 avec une clé
  du serveur), qui permet de le retrouver sans le rendre lisible ; l'empreinte du code chiffrée
  par une clé du serveur. Ces deux clés vivent hors de la base et hors de la copie de nuit
  ordinaire ; le porteur en garde un double hors du serveur. Un voleur de la base seule ne peut
  ni lire les numéros, ni essayer les codes hors ligne.
- **Relier un téléphone** : le parent tape son numéro et son code. Même réponse pour un numéro
  inconnu et pour un code faux (on ne révèle pas qu'un numéro existe). Un même numéro peut
  être connu de plusieurs écoles : chaque code ne s'essaie que contre les responsables de ce
  numéro.
- **Essais comptés**, comme pour le SMS : cinq codes faux dans la journée bloquent le numéro
  jusqu'au lendemain ; en plus, vingt essais par heure au plus depuis une même adresse Internet ;
  au-delà d'un seuil global d'échecs dans l'heure, le porteur reçoit une alerte. Le serveur ne
  vérifie que deux empreintes Argon2id à la fois (chacune demande beaucoup de mémoire), pour
  qu'on ne puisse pas l'étouffer.
- **Jeton d'appareil** : 256 bits tirés au hasard, remis dans un témoin (cookie) réservé au
  serveur, illisible par le code de la page, envoyé seulement en HTTPS et seulement au portail ;
  le serveur n'en garde que l'empreinte. Il vaut **180 jours, prolongés à chaque visite**. Cinq
  téléphones au plus par responsable ; le sixième remplace le plus ancien.
- **Fin d'un jeton** : le parent appuie sur « Se déconnecter » (le téléphone efface aussi ce qu'il
  gardait) ; l'école révoque le code ou en remet un autre (tous les téléphones reliés avec l'ancien
  sont coupés au prochain envoi) ; le responsable n'a plus d'enfant inscrit ; 180 jours sans
  visite.
- **Ce que l'école voit** : sur la fiche du responsable, le nombre de téléphones reliés et la date
  de la dernière consultation, qui descendent avec le prochain envoi. Pour couper un téléphone
  perdu, l'école remet un nouveau code : c'est le geste qu'elle connaît déjà.
- **Ni le numéro, ni le code, ni le jeton n'apparaissent dans les journaux du serveur.**

## Justification

Le code d'accès est déjà une identité vérifiée : il a été remis en main propre au guichet, à une
personne que l'école connaît. Le réutiliser évite tout compte, tout mot de passe, et garde une
seule règle pour le SMS et le portail. Le taper une seule fois par téléphone réduit les occasions
de le voir ou de l'essayer ; le jeton qui suit est long, aléatoire, et ne sert qu'au portail. Les
essais comptés rendent la devinette en ligne sans espoir (cinq essais par jour contre un milliard
de codes), et les clés du serveur protègent d'une devinette hors ligne après un vol de la base.
Le second contrôle par SMS coûterait à chaque entrée et ajouterait une passerelle à exploiter ;
le mot de passe choisi serait oublié et renverrait le parent au guichet, où il retrouve de toute
façon un code.

## Conséquences

Positives : aucun compte à créer ; une règle unique pour le SMS et le portail ; un téléphone
perdu se coupe par un geste que l'école connaît ; aucune donnée lisible ne permet de retrouver un
parent dans la base du serveur.

Négatives : un téléphone volé et non verrouillé montre les données de l'enfant jusqu'au nouveau
code ; deux clés de plus à garder pour le porteur ; le code d'accès devient plus précieux (il ouvre
aussi le portail), ce que le billet doit dire ; un parent qui change de numéro doit le faire
changer à l'école avant de relier un nouveau téléphone.

## Contre-argument conservé

Un code de six caractères n'est pas un secret fort pour Internet ; un second contrôle par SMS,
comme le font les banques, couperait court à tout vol de code. Il n'a pas gagné parce que le code
ne sert qu'une fois par téléphone, qu'il est protégé par des essais comptés et des clés du serveur,
et qu'un SMS à chaque entrée coûterait à l'éditeur pour un risque déjà réduit.

## Conditions de reconsidération

- Une tentative d'intrusion réussie, ou un seuil d'alerte franchi plusieurs fois par mois.
- L'APDP ou le juriste exige un second contrôle pour des données de mineurs.
- Le portail ouvre un jour une écriture qui engage (paiement, justification d'absence officielle) :
  un second contrôle devient nécessaire pour cette action.
- Les écoles réclament de couper un seul téléphone sans changer le code.
