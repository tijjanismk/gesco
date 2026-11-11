# Poste relié : GESCO installé, appels relayés par le binaire

**Date** : 10 novembre 2026
**Statut** : proposé le 10 novembre 2026, accepté le 10 novembre 2026 par le porteur du projet — décision 2 de `docs/plan-phase-11.md`, étapes A2 et A3
**Précise** : ADR-014 du blueprint (« un seul binaire, un basculement par configuration »)

## Contexte

Il faut dire ce qu'est un poste relié et par où ses appels partent vers le poste principal.

- Réponse du porteur à T-82 (10 novembre 2026) : les maîtres pourront saisir sur **un poste de
  l'école** ; leur téléphone n'est pas demandé. Rien n'exige donc un navigateur.
- Le frontend a un transport réseau déclaré depuis la phase 0.5 (`transportHttp`, par `fetch`),
  jamais branché.
- Vérifié dans `tauri.conf.json` : la politique de sécurité de l'écran n'autorise que
  `connect-src ipc: http://ipc.localhost`. Un `fetch` vers un autre ordinateur est refusé tant
  qu'on n'ouvre pas cette politique à des adresses qu'on ne connaît pas d'avance.
- Le canal sera chiffré avec un certificat que l'école fabrique elle-même (ADR « sécurité du
  réseau local ») : aucune autorité publique ne signe un certificat pour une adresse de réseau
  local sans Internet. WebView2 refuse un tel certificat ou alerte ; le faire accepter demande de
  l'installer dans Windows sur chaque poste, avec les droits d'administrateur.
- Les écrans se servent des dialogues du système (ouvrir, enregistrer) et de l'impression de
  l'aperçu.
- `reqwest` 0.13.5 est déjà dans `Cargo.lock`, apporté par Tauri, sans chiffrement activé.

## Options

| | 1. GESCO installé, le binaire relaie | 2. GESCO installé, l'écran appelle (`fetch`) | 3. Navigateur, rien à installer |
|---|---|---|---|
| Apprentissage | Faible : l'écran ne change pas | Faible côté écran | Moyen : servir l'écran, remplacer les dialogues |
| Exploitation | Installer et mettre à jour chaque poste | Idem, plus un certificat à installer dans Windows par poste | Rien sur les postes ; une alerte de certificat par navigateur |
| Maturité | reqwest et rustls, très répandus | `fetch` | Navigateurs |
| Pour un agent de code | Bon : un seul chemin côté écran, essais en Rust | Moyen : deux transports à garder justes | Moyen : écrans à adapter hors Tauri |
| Plus gros risque | Installeur de 225 Mo sur des postes qui n'en utilisent qu'une partie | Politique de sécurité de l'écran affaiblie ; ou canal en clair | Surface d'attaque ouverte à tout appareil du Wi-Fi |

## Décision

Un poste relié est le même GESCO, installé par le même installeur, dont le binaire relaie chaque
commande de l'écran au poste principal par un canal chiffré qu'il vérifie lui-même ; l'écran
garde le transport Tauri.

| Brique | Version | Sortie | Suite prévue |
|---|---|---|---|
| reqwest | 0.13.5 (déjà dans le verrou) | 8 septembre 2026 | Activer le chiffrement par rustls ; pas de date de fin publiée |
| rustls | 0.23.46 | 14 octobre 2026 | Série 0.23 ouverte en 2024 ; détail dans l'ADR « sécurité du réseau local » |

Vérifié le 10 novembre 2026 sur crates.io et dans `Cargo.lock`.

Ce que la décision fixe :

- **Rôle du poste** : seul (comme aujourd'hui), principal ou relié ; choisi au premier lancement,
  gardé dans un réglage du poste, hors de la base. Passer de seul à principal ne touche pas aux
  données.
- **Un poste relié ne garde aucune donnée de l'école** : seulement l'adresse du poste principal,
  l'empreinte de son certificat et son propre secret d'appairage, dans le trousseau de Windows.
- **L'écran ne change pas de transport.** `transportHttp` reste déclaré et inutilisé ; il
  servirait à l'option 3 si elle était un jour demandée.
- **Versions égales** : un poste relié d'une autre version que le poste principal est refusé
  avec un message clair. On met à jour le poste principal, puis chaque poste relié.
- **Fichiers** : ils voyagent par leur contenu, jamais par un chemin. Ce qui dépend du disque du
  poste principal y reste : clés et douze mots, licence, sauvegarde, restauration, clé modem,
  mise à jour, tâches périodiques.
- **Trouver le poste principal** : par son adresse ou son nom d'ordinateur, saisi une fois. La
  recherche automatique sur le réseau se décidera à l'étape A3 : la crate envisagée (`mdns-sd`
  0.21.7) est sortie le 2 novembre 2026, trop récente pour être retenue aujourd'hui.

## Justification

Le certificat fabriqué par l'école est le fait qui tranche : seul un client écrit en Rust peut
retenir son empreinte sans rien installer dans Windows ni affaiblir la politique de l'écran.
L'écran n'a alors plus rien à savoir du réseau : il continue d'appeler `executer`. Le porteur ne
demande pas de téléphone, donc le navigateur n'apporte rien aujourd'hui et ouvrirait le noyau à
tout appareil du Wi-Fi. Un seul installeur et un seul binaire, comme ADR-014 le voulait.

## Conséquences

Positives : aucune réécriture d'écran ; la politique de sécurité de l'écran reste fermée ; les
dialogues du système et l'impression marchent comme avant ; les essais du relais se font en Rust.

Négatives : chaque poste s'installe et se met à jour (225 Mo, moteur de documents compris, qui ne
sert pas sur un poste relié) ; pas de téléphone ni de tablette ; une coupure du réseau se
traduit dans le binaire puis à l'écran, deux endroits à tenir justes ; le transport réseau
déclaré dans le frontend reste du code qui ne sert pas.

## Contre-argument conservé

Le navigateur éviterait d'installer et de mettre à jour dix postes, et ouvrirait la saisie sur
téléphone, qui est l'usage le plus naturel pour un maître. Il n'a pas gagné parce que personne ne
le demande aujourd'hui, que le certificat y redevient une alerte à chaque appareil, et que
l'option 1 ne le ferme pas : le serveur et le contrat seraient les mêmes.

## Conditions de reconsidération

- Une école demande la saisie depuis les téléphones des maîtres (T-82, option c).
- L'installation ou la mise à jour de plusieurs postes s'avère un frein réel chez une école.
- L'e-learning de la phase 12 sert de toute façon des pages aux téléphones sur le Wi-Fi : la
  question du certificat s'y posera, et sa réponse pourra servir ici.
