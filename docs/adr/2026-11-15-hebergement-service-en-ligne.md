# Hébergement du service en ligne : une machine louée, un binaire, une base par école

**Date** : 15 novembre 2026
**Statut** : proposé le 15 novembre 2026
**Précise** : décision 6 de `docs/plan-phase-11.md` (§4) ; blueprint ADR-001, ADR-003 (révisé ici
pour le serveur), §3.12, §24.3, §26.3
**Spécification** : `SPEC_LOTS_C_D.md`

## Contexte

Le porteur a décidé le 14 novembre 2026 que tout « en ligne » entre dans la version 2.0 : la
sauvegarde hors site sur un serveur de l'éditeur (C1), l'envoi des données destinées aux parents
(C2), le serveur (C3) et le portail des parents (D). Il faut donc un serveur, quelque part, et
quelqu'un pour le tenir.

Contraintes du terrain et du projet :

- **Une seule personne exploite** : le porteur. Pas d'équipe de garde, pas d'astreinte la nuit.
  Ce qui tombe en panne doit pouvoir attendre quelques heures sans dommage.
- **L'école ne dépend jamais du serveur** (ADR-001). Un serveur arrêté ne bloque ni une
  inscription, ni un encaissement, ni une impression. Seuls les parents voient des données moins
  fraîches, datées à l'écran.
- **Données de mineurs.** La loi malienne sur les données personnelles et l'autorité de contrôle
  (APDP) s'appliquent ; le blueprint (§24.3) fait de l'avis d'un juriste un préalable à la mise
  en service. Le pays d'hébergement en dépend : transfert hors du Mali permis ou non, déclaration
  ou autorisation, contrat entre l'école et l'éditeur. **Cet avis n'existe pas encore.**
- **Parents sur téléphone**, en 3G ou 4G, forfait compté. Quelques dixièmes de seconde de plus
  par requête ne gênent pas ; des mégaoctets de trop, si.
- **Coût par école faible** : l'abonnement doit couvrir un coût réel et se défendre (§26.1).
- **Le disque du porteur est juste** : le serveur ne doit pas se compiler à chaque vérification.
  Le poste du porteur est sous Windows ; le serveur, lui, tournera sous Linux. La CI Linux du
  dépôt construit déjà tout sauf `gesco-app`.
- Ce que le projet sait déjà faire : axum, tokio, rustls (réseau local, lot A), SQLite3 Multiple
  Ciphers (copie dans `vendor/`), Ed25519 (licence), Argon2id (codes d'accès).

## Options

| | 1. Machine virtuelle louée, binaire unique, SQLite par école | 2. Plateforme gérée : conteneur et PostgreSQL gérés | 3. Hébergeur au Mali, même installation que l'option 1 |
|---|---|---|---|
| Apprentissage | Faible à moyen : Linux, un service systemd, un relais HTTPS | Moyen : la plateforme, PostgreSQL, un second dialecte SQL | Comme l'option 1, plus la relation avec un hébergeur local |
| Exploitation | Mises à jour du système automatiques ; une copie de nuit chez un second fournisseur | Moins de système à tenir ; coût mensuel plus élevé ; dépendance à la plateforme | Comme l'option 1 ; coupures de courant et de liaison à mesurer |
| Maturité | Debian, Caddy, SQLite : très répandus | Bonne, mais propre à chaque plateforme | Variable selon l'hébergeur |
| Pour un agent de code | Très bon : tout s'essaie sur le poste, même code que l'école (SQLite) | Moyen : PostgreSQL à installer pour les essais, requêtes à écrire deux fois | Très bon (même code que l'option 1) |
| Plus gros risque | Le porteur oublie une alerte ; disque plein | Coût qui grimpe avec le nombre d'écoles ; lieu des données moins maîtrisé | Disponibilité et sauvegardes de l'hébergeur moins sûres |

Écartées sans tableau :

- **Un ordinateur chez le porteur** : coupures de courant, adresse qui change, aucune protection
  physique des données de mineurs.
- **Un service par fonction** (stockage d'objets pour les archives, base gérée pour les vues,
  hébergement statique pour le portail) : trois factures, trois tableaux de bord, trois pannes
  possibles pour une seule personne.

## Décision

Une machine virtuelle Linux louée, qui fait tourner un seul binaire `gesco-server` derrière le
relais HTTPS Caddy, avec un fichier SQLite chiffré par école ; une copie chiffrée de nuit part
chez un second fournisseur. Le pays et l'hébergeur se choisissent sur l'avis du juriste : si le
Mali est exigé, la même installation va chez un hébergeur malien (option 3).

| Brique | Version | Sortie | Suite prévue |
|---|---|---|---|
| Debian | 13 (13.7) | série ouverte le 9 août 2025 | Support ordinaire jusqu'au 9 août 2028, support long jusqu'au 30 juin 2030 |
| Repli si l'hébergeur ne propose pas Debian : Ubuntu | 26.04 LTS (26.04.1) | 23 avril 2026 | Fin le 29 mai 2031 |
| Caddy (relais HTTPS, certificats automatiques) | série 2.11 ; 2.11.7 aujourd'hui | 3 octobre 2026 | 2.11.8 et 2.11.9 (4 et 5 novembre 2026) sont trop récentes pour être fixées aujourd'hui : prendre la dernière corrective de la série 2.11 à la mise en service ; pas de date de fin publiée |
| axum | 0.8.9 (déjà dans le dépôt) | 14 avril 2026 | Série 0.8 ; pas de date de fin publiée |
| tokio | 1.53.2 (déjà dans le dépôt ; 1.54.1 existe) | 3 octobre 2026 | Suivre les correctives de la série 1.x |
| sqlx et SQLite3 Multiple Ciphers | ceux du poste (sqlx 0.9.0, copie de `vendor/`) | — | Suivent le poste |

Vérifié le 15 novembre 2026 : Debian et Ubuntu sur endoflife.date ; Caddy sur les sorties GitHub
du projet ; axum, tokio et sqlx sur crates.io. Non vérifié : les offres et les prix des
hébergeurs (au porteur de les comparer) ; que la copie de SQLite3 Multiple Ciphers se compile
sous Linux par la CI (elle compile déjà `gesco-db` sous Linux, donc probable).

Ce que la décision fixe :

- **Un seul binaire**, `gesco-server`, crate du dépôt **hors de `default-members`**, construit
  par la CI Linux et non sur le poste du porteur. Il dépend de `gesco-core`, jamais de
  `gesco-services` (pas de Typst sur le serveur).
- **Caddy** obtient et renouvelle seul le certificat HTTPS du nom de domaine ; il ne fait que
  relayer vers `gesco-server`, qui n'écoute que sur la machine elle-même.
- **Une base par école** (`ecoles/<licence>.sqlite`), plus une base de registre (comptes
  d'écoles, journal d'administration). Une erreur de requête ne peut pas mêler deux écoles ;
  effacer une école, c'est effacer un fichier. Les archives de sauvegarde (C1) sont des fichiers
  à part, déjà chiffrés par l'école.
- **Chiffrement au repos** des bases du serveur par SQLite3 Multiple Ciphers, clé dans un
  fichier lisible du seul compte de service ; la même clé est gardée hors du serveur par le
  porteur, pour la restauration.
- **Copie de nuit** de tout le serveur (bases et archives) chez un second fournisseur, chiffrée,
  avec une restauration d'essai chaque trimestre.
- **ADR-003 révisé pour le serveur** : PostgreSQL n'est plus prévu. Le serveur ne garde que des
  vues déjà calculées par l'école (ADR « données qui montent ») et n'a aucune requête
  croisée à faire entre écoles ; SQLite suffit et garde un seul dialecte SQL dans le dépôt.
- **Aucune administration par le web** : le porteur administre par une commande sur la machine
  (`gesco-server admin …`), à travers SSH par clé. Rien d'administratif n'est exposé à Internet.

## Justification

L'école ne dépend pas du serveur : le besoin est une bonne disponibilité, pas une disponibilité
parfaite, et une seule machine bien sauvegardée y répond. Une seule personne exploite : un seul
binaire, un seul relais qui gère seul ses certificats, des mises à jour du système automatiques,
pas de base de données à administrer. Le code reste celui du poste (SQLite, sqlx, rustls) : rien
de nouveau à apprendre ni à essayer deux fois. Une base par école protège d'une fuite d'une école
vers une autre, l'erreur la plus grave pour des données de mineurs. Le coût est celui d'une petite
machine et d'un peu de stockage, que l'abonnement couvre. La plateforme gérée enlève un peu
d'exploitation du système mais ajoute un dialecte SQL, un coût qui grimpe et un lieu de données
moins clair. Le choix du pays reste ouvert : il appartient au juriste, et l'installation est la
même partout.

## Conséquences

Positives : un seul livrable de plus, construit par la CI ; une installation qu'on refait en une
heure sur une machine neuve à partir d'un script et de la dernière copie ; des écoles isolées
les unes des autres ; aucun dialecte SQL de plus.

Négatives : le système de la machine est à la charge du porteur (mises à jour, pare-feu, accès
SSH, disque) ; une machine unique tombe parfois (quelques heures par an) ; la clé de chiffrement
des bases et la clé SSH sont deux secrets de plus à garder hors du serveur ; un second fournisseur
pour la copie de nuit ; ADR-003 change pour le serveur, à reporter dans le blueprint.

## Contre-argument conservé

Une plateforme gérée enlève les mises à jour du système, le pare-feu et les disques, c'est-à-dire
ce qu'une personne seule oublie le plus facilement ; et PostgreSQL permettrait un jour des
statistiques entre écoles. Il n'a pas gagné parce que le serveur ne fait que ranger et montrer des
vues, que les statistiques entre écoles ne sont pas demandées, et que le coût d'une plateforme
grimpe avec chaque école alors que celui d'une machine reste plat jusqu'à plusieurs centaines
d'écoles (à mesurer).

## Conditions de reconsidération

- Le juriste exige un hébergement au Mali et aucun hébergeur malien ne propose de machine
  virtuelle fiable : réexaminer une plateforme gérée dans la sous-région.
- Plus de quelques centaines d'écoles, ou une machine qui sature à l'essai de charge : séparer
  les archives sur un stockage d'objets.
- Le porteur n'arrive pas à tenir le système (une alerte manquée qui fait perdre des données, ou
  une mise à jour de sécurité en retard de plus d'un mois) : passer à une plateforme gérée.
- Une organisation à plusieurs écoles demande des chiffres consolidés en ligne : rouvrir la
  question de PostgreSQL.
