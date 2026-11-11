# Plan de la phase 11 — proposition à discuter

**Projet** : GESCO — Tauri 2 + Rust + React/TypeScript
**Phase** : 11 — plusieurs postes sur le réseau de l'école, évaluation par compétences, service en ligne, portail des parents
**Statut** : proposition du 10 novembre 2026, **acceptée le même jour par le porteur du projet** : toutes les recommandations de T-80 à T-86 sont retenues, T-82 comprise après précision (les maîtres pourront saisir leurs notes, possibilité bloquée à la livraison) ; ordre des lots, version et branche acceptés. Aucune ligne de code, aucune spécification encore ; les réponses restent à recopier dans `docs/questions-terrain.md`
**Documents parents** : `PROJECT_BLUEPRINT.md` (ADR-001, 002, 003, 009, 013, 014, 017 ; §15, §17.4, §22.2, §22.3 M5 et M6, §24.1 à §24.3, §25.4 à §25.6, §26), `ARCHITECTURE.md`, `ALERTES.md`, `docs/adr/2026-10-04-frontiere-commande-unique.md`, `docs/adr/2026-10-10-chiffrement-de-la-base.md`, `docs/optimisations-2026-10-31.md`, `docs/versions.md`, `docs/questions-terrain.md` (T-23, T-77)

Décisions du porteur du projet du 10 novembre 2026 : la phase 9 est laissée de côté ; la phase 11 se
planifie et sortira comme une nouvelle version ; la 1.2.7 reste une version de corrections à part.
Jusqu'ici la phase 11 « attendait une école qui la demande » (M6) : ce plan la découpe pour qu'on
puisse l'engager lot par lot, chacun validé comme une phase.

Ce document dit ce qui existe, propose un ordre, pose sept questions (T-80 à T-86, §3, répondues
le 10 novembre 2026) et liste les décisions à écrire en ADR (§4), qu'il ne tranche pas. La
spécification du lot A s'écrit maintenant, après les ADR 1 à 4.

---

## 1. État des lieux

Relevé le 10 novembre 2026 sur la branche `1.2.x` (1.2.7 en cours, non commitée), en lecture
seule, sans compilation. « Vérifié » : relu dans le code. « À vérifier » : supposé, à confirmer en
écrivant la spécification.

### 1.1 Ce que le code a déjà posé

| Couture | Où | État |
|---|---|---|
| Point de passage unique | `gesco_services::frontiere::Application::executer(nom, jeton, entree)` ; `gesco-app` ne fait que le lui remettre (`main.rs`, 144 lignes) | Vérifié. Environ 317 commandes au contrat `contrat!`, toutes gardées au même endroit. Une route réseau n'a qu'à appeler cette même fonction : c'est ce que l'ADR du 4 octobre avait prévu |
| Accès du frontend | `frontend/src/api/` : interface `Transport`, `transportTauri` actif, `transportHttp` déclaré (`POST /api/commandes/{nom}`, jeton en en-tête) | Vérifié. Hors de `api/`, aucun écran ne touche à Tauri. Le choix du transport est écrit en dur dans `index.ts` |
| Sessions | `session.rs` : registre en mémoire, un jeton opaque par session, verrouillage, PIN, durées réglées par l'école | Vérifié. Le registre accepte déjà plusieurs sessions à la fois. La session ne dit pas de quel poste elle vient |
| Caisse | `session_caisse` : une session ouverte par caissier (index unique, migration 0012) | Vérifié. Deux caissiers sur deux postes, chacun sa caisse : rien à changer au modèle |
| Écritures concurrentes | SQLite en WAL, transactions `BEGIN IMMEDIATE`, attente de 10 s sur le verrou (`gesco-db/src/base.rs`, `unite.rs`) | Vérifié. Les écritures se suivent une à une sans se corrompre |
| Base chiffrée | SQLite3 Multiple Ciphers ; clé de données sous enveloppe machine (trousseau Windows) et enveloppe de secours (douze mots) | Vérifié. En réseau local, seul le poste principal détient la base et la clé : un poste relié volé ne contient aucune donnée |
| Licence | `gesco-core/src/core/licence.rs` : signature Ed25519, `Edition` (Essentielle, Connectée), `modules`, empreinte machine, `echeance`, grâce de 90 jours, `modules_connectes_actifs` | Vérifié. L'abonnement de l'édition Connectée est déjà modélisé ; aucune fonction ne le lit encore |
| Journal des événements | Table `outbox`, écrite dans la transaction de chaque mutation, avec `identifiant_poste` et `consomme_le` ; dépôt `non_consommes` et `marquer_consommes` | Vérifié. Personne ne la consomme (seuls des tests la lisent) et rien ne la purge : elle grossit depuis l'installation de chaque école |
| Identifiants et colonnes | ULID partout ; `mis_a_jour_le` et `identifiant_poste` sur chaque table | Vérifié sur le socle |
| Identité du responsable | `code_acces` (empreinte Argon2id, un code actif par responsable), consentement par type de message, joignabilité des numéros (phase 5, T-77) | Vérifié. Le périmètre de la consultation par SMS (solde, absences, moyenne ; rang au choix de l'école) est un précédent pour le portail |
| Mode d'évaluation | `ModeEvaluation { Notes, Competences }` sur le niveau ; le jardin est livré en « compétences » ; le jardin s'inscrit depuis la phase 2 | Vérifié. Aucune table ni écran derrière : une école qui a un jardin n'a aujourd'hui aucun bulletin pour lui |
| Plusieurs établissements | `etablissement_id` sur chaque table, filtre unique dans `gesco-db` | Posé, non exposé (ADR-006) |
| Portée des enseignants | `PorteeNotes` et `PorteeAbsences` dans `gesco-core/src/pedagogie`, appliquées par les services des notes et des absences ; rôle Enseignant livré ; compte lié à un enseignant | Vérifié (corrigé le 10 novembre 2026 : la première version de ce plan disait la restriction absente). La saisie limitée à ses classes existe depuis la phase 4 ; la lecture des fiches, elle, n'est pas limitée (la garde utilise `PorteeTotale`) |

### 1.2 Ce qui manque

**Pour plusieurs postes**

- Aucun serveur réseau : axum a quitté l'espace de travail le 31 octobre avec les deux crates vides.
- Le rôle d'un poste (seul, principal, relié) n'existe nulle part ; `Application` porte un seul
  nom de poste, celui de l'ordinateur : en l'état, tout ce qu'écrirait un poste relié serait signé
  du poste principal dans le journal d'audit et dans `outbox`.
- Rien pour relier un poste : ni appairage, ni chiffrement du canal, ni liste des postes, ni
  révocation, ni journal des connexions (§17.4 du blueprint les demande).
- Les commandes qui reçoivent un chemin de fichier (archive, restauration, destinations de
  sauvegarde) supposent que l'écran et le noyau sont sur la même machine. Vérifié pour ces
  trois-là ; les imports et les exports sont à relire un par un.
- Deux personnes sur la même fiche : à vérifier, mais rien n'indique qu'une modification faite
  entre-temps soit détectée ; la dernière écriture l'emporterait sans rien dire.
- L'écran ne sait pas que le noyau peut être injoignable (câble débranché, poste principal éteint).
- Des lenteurs sans gêne sur un poste deviennent gênantes à plusieurs : la vérification du mot de
  passe pendant que le verrou d'écriture est tenu (P-19), les envois par la clé modem qui bloquent
  (P-21), la sauvegarde à la fermeture (P-2), les permissions relues à chaque commande (C-8).
- Livraison : pare-feu de Windows, adresse du poste principal qui change, versions différentes
  entre postes. Rien n'a jamais été essayé sur deux machines.

**Pour l'évaluation par compétences** : les entités du §8.4 B (domaine, compétence, observation),
la saisie, le modèle de livret, le droit de les régler.

**Pour le service en ligne** : tout. Pas de serveur, pas de consommation du journal, pas de compte
d'école en ligne, pas d'hébergeur, pas de tarif, pas d'analyse de conformité (le blueprint §24.3
en fait un préalable validé par un juriste avant la mise en service). À vérifier aussi : les
événements du journal n'ont pas été écrits pour reconstruire une copie des données ; certains
portent tout (un versement et ses imputations), d'autres seulement des identifiants. Il faudra les
relire un par un avant de décider ce qui monte en ligne et sous quelle forme (§4, décision 7).

**Pour le portail** : tout, sauf l'identité du responsable et le précédent du SMS.

### 1.3 Ce qu'il faut reprendre de `gesco-server` et `gesco-sync`

Presque rien, et c'est une bonne nouvelle. Les deux crates retirées le 31 octobre (commit
`467b04e`) étaient vides : `fn main() {}` pour l'une, une ligne de commentaire pour l'autre.
L'historique garde seulement :

- leurs noms et leurs rôles : `gesco-server`, binaire du serveur **en ligne**, « réplique en
  lecture, jamais source de vérité » ; `gesco-sync`, consommation du journal des événements ;
- leurs `Cargo.toml` (`gesco-server` dépendait de `gesco-services`, d'axum et de tokio ;
  `gesco-sync` de `gesco-core` seul) et leurs lignes dans `members`, `default-members` et les
  dépendances de l'espace de travail.

À retenir pour la suite :

1. **Le réseau local n'a besoin d'aucune des deux.** ADR-014 et la correction du 4 octobre
   (`ALERTES.md`) le disent : le serveur du réseau local vit dans `gesco-app`, ses gestionnaires
   dans `gesco-services`. Pour le premier lot, seul axum revient, comme dépendance de `gesco-app`
   (version à vérifier le jour même, ADR de dépendance).
2. `gesco-sync` et `gesco-server` ne reviennent qu'au lot du service en ligne, et **hors de
   `default-members`** : le disque du porteur ne doit pas les compiler à chaque vérification.
3. La dépendance de `gesco-server` à `gesco-services` entier (donc à Typst) est à revoir à ce
   moment-là : un serveur qui montre des données aux parents a besoin de `gesco-core`, pas du
   moteur de documents.

---

## 2. Découpage proposé

Quatre lots, chacun livrable et validé seul, comme une phase. Les deux premiers appartiennent à
l'édition Essentielle (aucun coût récurrent, rien d'extérieur à attendre) ; les deux derniers
forment l'édition Connectée et dépendent de décisions hors du code.

### 2.1 Ordre proposé et pourquoi

**A. Plusieurs postes → B. Compétences (jardin) → C. Service en ligne → D. Portail des parents.**

Commencer par les postes en réseau local, pour cinq raisons :

1. **Rien d'extérieur ne le bloque.** Ni hébergeur, ni tarif d'abonnement, ni juriste : il peut
   s'engager dès les réponses à T-80 à T-83.
2. **Il lève la plus grosse limite du produit vendu aujourd'hui.** Un seul clavier pour la
   secrétaire, le caissier et la direction : c'est le risque de compte partagé (RA-13) et la
   raison pour laquelle la direction saisit toutes les notes (T-23).
3. **Les trois autres lots s'appuient dessus.** Le serveur embarqué, le canal chiffré et
   l'appairage serviront au service en ligne ; et l'e-learning de la phase 12 a pour mode par
   défaut « le poste principal sert les cours sur le Wi-Fi de l'école » (§16.3), qui est ce même
   serveur.
4. **Il se vérifie entièrement chez nous** : deux instances sur le poste de développement pour
   les tests, deux vraies machines pour l'essai du porteur. Aucun tiers.
5. **La couture a été payée pour lui** depuis la phase 0.5 (frontière unique, `api/`). Tant
   qu'elle n'a pas servi, on ne sait pas si elle tient : mieux vaut le savoir avant d'ajouter
   encore des commandes.

Le lot B est petit et ne dépend de rien : il peut passer **avant** A si une école avec un jardin
attend son livret (T-80), ou se faire entre deux étapes de A. Les lots C et D viennent en dernier
parce qu'ils visent les seules écoles qui ont Internet et un budget récurrent (§26.3), que le SMS
et WhatsApp couvrent déjà l'essentiel du besoin des parents (§3.9), et que leurs préalables
(§4, décisions 5 à 8) prennent du temps hors du code : autant les lancer pendant A et B.

**Par quoi commencer : l'étape A1.** C'est la preuve la moins chère : sur une seule machine, sans
réseau ni sécurité encore, faire répondre le noyau par une route et rejouer les essais existants
par ce chemin. Si quelque chose dans les 317 commandes suppose « même machine », on le voit là.

### 2.2 Lot A — Plusieurs postes sur le réseau de l'école

Un poste principal détient la base ; les autres s'y relient (ADR-014). Un seul installeur, le
rôle se choisit au premier lancement.

| Étape | Contenu | Risques | Vérifiable par |
|---|---|---|---|
| A1 — Le noyau répond par une route | Route unique dans `gesco-app` vers `executer`, éteinte à la livraison, limitée à la machine elle-même à cette étape ; le nom du poste appelant accompagne chaque appel et signe l'audit et les événements ; rôle du poste dans un réglage du poste, hors de la base | Retour d'axum (disque, durée de compilation) ; état global qui suppose un seul utilisateur ; commandes à chemin de fichier | Les essais de services rejoués par la route donnent les mêmes résultats et les mêmes refus ; un essai parcourt tout le contrat par les deux chemins ; l'audit porte le nom du poste appelant |
| A2 — Relier un poste en sûreté | Canal chiffré, certificat créé par le poste principal, empreinte retenue par le poste relié ; code d'appairage affiché sur le poste principal ; liste des postes, révocation ; journal des connexions ; écoute sur le réseau privé seulement ; essais de connexion comptés par poste | Le webview refuse un certificat auto-signé (voir §4, décision 2) ; une clé de plus à garder et à sauvegarder ; pare-feu | Un poste non appairé ou révoqué est refusé ; une écoute du réseau ne montre rien de lisible ; le journal nomme chaque poste |
| A3 — Le poste relié | Premier lancement : « rejoindre le poste principal » (adresse ou recherche sur le réseau) ; refus clair si les versions diffèrent ; bandeau « poste principal injoignable » et reprise sans rien perdre en silence ; fichiers échangés par leur contenu ; ce qui reste au poste principal : clés, douze mots, licence, sauvegarde, restauration, clé modem, mise à jour | Adresse du poste principal qui change ; saisie en cours perdue à la coupure ; écrans qui supposent le sélecteur de fichier du système | Playwright sur deux instances (une principale, une reliée) : inscrire, encaisser, imprimer depuis le poste relié ; couper puis rétablir le poste principal |
| A4 — Travailler à plusieurs en même temps | Deux personnes sur la même fiche : le dire et ne rien écraser en silence ; listes qui se relisent ; opérations qui demandent d'être seul (clôture d'année, restauration, import, mise à niveau) : prévenir et écarter les autres postes ; lenteurs P-19, P-21, P-2, C-8 corrigées | Modification perdue ; attente sur le verrou d'écriture ; vieux PC en poste principal | Essai de charge : dix postes simulés (T-81) sur la base de l'école d'essai, temps de réponse mesurés ; essai de deux modifications croisées |
| A5 — Les maîtres et leurs classes (T-82) | Bloqué à la livraison ; l'école l'ouvre par un réglage. La saisie limitée à ses classes existe déjà (phase 4) : l'étape ajoute le réglage et limite aussi la lecture des fiches aux élèves de ses classes | Touche à la sécurité ; une portée oubliée sur une commande montre toute l'école | Essai : réglage éteint, un maître n'entre nulle part ; allumé, il ne lit ni n'écrit hors de ses classes, commande par commande ; matrice des permissions régénérée |
| A6 — Livraison | Installeur, règle de pare-feu, guide d'installation à plusieurs postes, manuel, notes de version, captures ; licence selon la décision 5 ; **essai du porteur sur deux vraies machines** | Tout ce qui casse sur la vraie machine du client ; jamais essayé hors du poste de développement | Liste d'essais à la main sur deux machines, tenue par le porteur ; mise à jour d'une école en 1.2 vers la nouvelle version sans rien changer pour un poste seul |

Ce qui ne change pas : une école à un seul poste ne voit aucune différence ; le serveur reste
éteint tant qu'elle ne le demande pas.

Étape A5 (T-82, réponse du 10 novembre 2026) : à la livraison rien ne change, la direction saisit
toutes les notes et aucun accès n'est ouvert aux maîtres ; l'école qui le veut ouvre la saisie
par les maîtres, chacun dans ses classes et ses matières, sur un poste de l'école. Le téléphone
personnel des maîtres reste hors de la première version.

### 2.3 Lot B — Évaluation par compétences (jardin d'enfants)

| Étape | Contenu | Risques | Vérifiable par |
|---|---|---|---|
| B1 — Domaines et compétences | Domaines et compétences par niveau, livrés et modifiables par l'école ; migration ; droit de les régler | Le contenu livré dépend du terrain (T-84) | Essais de services ; capture de l'écran de réglage |
| B2 — Observations | Saisie par classe et par période : une case par élève et par compétence, au clavier ; commentaire | Saisie trop lente pour une classe entière | Essai chronométré par Playwright ; captures |
| B3 — Livret du jardin | Modèle imprimé, appréciation générale, publication, remise aux parents et avis par SMS comme pour un bulletin | Chaque école veut son livret | PDF vérifié en image comme le bulletin ; capture |
| B4 — Droits, école d'essai, validation | Rôles livrés, école d'essai avec un jardin évalué, manuel | Bases installées qui ont déjà des élèves au jardin | Essai de mise à niveau ; validation du porteur |

Hors du lot, sauf réponse contraire à T-84 : les compétences au 1er cycle.

### 2.4 Lot C — Service en ligne (édition Connectée, première partie)

Préalables hors du code, à lancer pendant A et B : hébergeur, tarif, analyse de conformité par un
juriste (§4, décisions 5 à 7).

| Étape | Contenu | Risques | Vérifiable par |
|---|---|---|---|
| C1 — Sauvegarde hors site | L'archive de sauvegarde, déjà chiffrée par la clé de l'école, part en ligne : à la demande ou hors des heures de classe, volume affiché, reprise après coupure | Forfait de l'école consommé ; connexion lente | Restauration sur une autre machine depuis l'archive en ligne ; l'hébergeur ne peut rien lire |
| C2 — Ce qui monte | `gesco-sync` revient : envoi des seules données destinées aux parents, par lots compressés, avec reprise et compteur ; licence Connectée lue (échéance, grâce) ; purge du journal consommé | Forme des données (décision 7) ; journal accumulé depuis l'installation à écouler une première fois | Essais contre un serveur d'essai : coupure en plein envoi, reprise, rien en double, rien de perdu ; volume mesuré sur l'école d'essai |
| C3 — Le serveur | `gesco-server` revient : réception, comptes d'écoles, stockage, sa propre sauvegarde, sa surveillance | Un service à exploiter jour et nuit par une seule personne ; données de mineurs en ligne | Mise en service d'essai ; procédure de panne écrite et jouée une fois |

C1 vient en premier parce qu'elle est la moins risquée (l'hébergeur ne voit qu'une archive
chiffrée) et la plus utile : aujourd'hui la machine unique est le seul exemplaire vivant des
données (RA-12).

### 2.5 Lot D — Portail des parents

Application web installable sur l'écran d'accueil (ADR-017), jamais une application de magasin.

| Étape | Contenu | Risques | Vérifiable par |
|---|---|---|---|
| D1 — Entrer | Le responsable entre avec ce qu'il a déjà (numéro et code d'accès), selon la décision 8 ; essais comptés | Un code de six caractères exposé à Internet | Essais d'intrusion simples : codes faux, numéro d'un autre |
| D2 — Voir | Ses enfants ; pour chacun, ce que T-85 aura retenu | Montrer trop (sanctions, appréciations) ; données en retard sur l'école, à dater à l'écran | Captures sur téléphone ; lisibilité et contrastes ; un parent ne voit que ses enfants |
| D3 — Hors ligne et avis | Dernier état gardé sur le téléphone ; avis quand un bulletin est publié | Peu de parents l'installeront (§3.9) | Essai sans réseau ; mesure d'usage chez l'école pilote |
| D4 — Écrire à l'école *(si demandé)* | Message du parent vers l'école, seule donnée qui descende | Modération, charge pour la secrétaire | Essai de bout en bout ; aucun conflit possible par construction |

### 2.6 Ce que ce plan laisse dehors

Le transfert de données entre deux écoles d'une même organisation (Q-36), WhatsApp Business,
l'e-learning en ligne (phase 12), et tout fonctionnement où chaque poste continuerait seul puis
se réconcilierait : c'est un autre projet, bien plus lourd (voir T-83).

---

## 3. Questions de terrain avant la spécification

À la suite de T-79, dernière question du dépôt. Réponses courtes attendues ; aucune ne demande de
document. T-80 à T-83 suffisent pour écrire la spécification du lot A ; T-84 pour le lot B ; T-85
et T-86 pour les lots C et D.

### T-80 — Ce qu'une école attend en premier

Parmi les écoles que vous connaissez, qu'est-ce qui est demandé d'abord ?

- a) plusieurs ordinateurs en même temps ;
- b) le livret du jardin d'enfants ;
- c) la consultation en ligne par les parents ;
- d) rien de précis encore.

*Recommandation* : a), puis b). Si une école à jardin attend déjà son livret, b) passe devant :
il est court et ne dépend de rien.

*Réponse* (porteur du projet, 10 novembre 2026) : recommandation retenue.

### T-81 — Combien de postes, et qui s'y assoit

Dans une école qui veut plusieurs postes, combien, et pour qui ?

- a) deux ou trois : secrétariat, caisse, direction ;
- b) jusqu'à cinq ou six, avec le surveillant, le comptable, le directeur des études ;
- c) davantage, avec une salle des professeurs.

*Recommandation* : construire et mesurer pour dix postes en même temps, ce qui couvre a) et b) ;
au-delà, on mesure avant de promettre.

*Réponse* (porteur du projet, 10 novembre 2026) : recommandation retenue.

### T-82 — Les enseignants et leurs notes

Aujourd'hui la direction saisit toutes les notes (T-23). Avec plusieurs postes, les enseignants
saisiront-ils eux-mêmes ?

- a) non, rien ne change ;
- b) oui, sur un poste partagé de l'école, chacun ne voyant que ses classes et ses matières ;
- c) oui, et aussi depuis leur propre téléphone ou ordinateur sur le Wi-Fi de l'école.

*Recommandation* : b), comme une étape à part, éteinte à la livraison : l'école l'allume si
elle le veut. c) change la sécurité du réseau local (§4, décision 2) : pas dans la première
version.

*Réponse* (porteur du projet, 10 novembre 2026) : recommandation retenue, après précision. Une
première réponse (« la direction saisit ») a été complétée le même jour : « on peut donner aussi
la possibilité au maître de saisir, mais par défaut bloqué ». Donc b), étape A5, bloquée à la
livraison ; c) (téléphone personnel) n'est pas demandé.

### T-83 — Le réseau de l'école, et le poste principal éteint

Deux choses liées. Comment les ordinateurs d'une école sont-ils reliés aujourd'hui : une box ou
un routeur Wi-Fi déjà là, des câbles, rien du tout ? Et quand le poste principal est éteint ou en
panne (coupure de courant), les autres postes ne peuvent plus travailler : est-ce acceptable ?

- a) oui : les autres attendent son retour, l'écran le dit clairement ;
- b) non : chaque poste doit pouvoir continuer seul et tout se recolle ensuite.

*Recommandation* : a). Le guide conseillera un portable (sa batterie tient pendant les coupures)
comme poste principal, relié par câble à un petit routeur ; GESCO ne dépendra jamais d'Internet
pour cela. b) est un autre produit : des mois de travail, et des reçus numérotés en double à
réconcilier.

*Réponse* (porteur du projet, 10 novembre 2026) : recommandation retenue — quand le poste
principal est éteint, les autres attendent. La façon dont les ordinateurs sont reliés
aujourd'hui n'a pas été précisée : GESCO acceptera tout réseau local privé, le guide conseillera
le routeur et le câble.

### T-84 — Le jardin d'enfants : ce qu'on évalue et ce qu'on remet

Que remettent les jardins aux parents, et quand ?

- a) un livret par domaine (langage, motricité, vie avec les autres, découverte du monde, arts)
  avec pour chaque compétence « acquis », « en cours », « non acquis », chaque trimestre ;
- b) seulement une appréciation rédigée par période ;
- c) rien d'écrit.

*Recommandation* : a), avec une liste livrée que l'école modifie, une appréciation générale, et
le trimestre par défaut (réglable). Pour le jardin seulement : le 1er cycle garde ses notes.

*Réponse* (porteur du projet, 10 novembre 2026) : recommandation retenue.

### T-85 — Ce qu'un parent voit en ligne

Si un portail existe, que doit y trouver un parent ?

- a) comme par SMS aujourd'hui : reste à payer et prochaines échéances, absences, moyenne du
  dernier bulletin publié ;
- b) a), plus les bulletins publiés et les reçus à relire ;
- c) b), plus les sanctions, les appréciations, les convocations, l'emploi du temps, et la
  possibilité d'écrire à l'école.

*Recommandation* : b), en lecture seule. Les sanctions et les appréciations se disent en face
(§25.4) ; écrire à l'école viendra seulement si une école le demande.

*Réponse* (porteur du projet, 10 novembre 2026) : recommandation retenue.

### T-86 — Internet, téléphones et abonnement

Trois faits qui décident si les lots C et D valent d'être construits maintenant. Les écoles visées
ont-elles Internet à l'école (en permanence, de temps en temps par une clé 4G, jamais) ? Leurs
parents ont-ils un téléphone qui va sur Internet, avec du forfait ? Une école paierait-elle chaque
année pour le service en ligne ?

- a) oui aux trois pour au moins une école que vous connaissez : on prépare C et D pendant A et B ;
- b) Internet de temps en temps seulement : on fait la sauvegarde hors site (C1), le portail attend ;
- c) non : C et D attendent, la phase 11 s'arrête après A et B, puis vient la phase 12.

*Recommandation* : ne pas construire le portail sans une école qui l'attend et accepte
l'abonnement. La sauvegarde hors site, elle, se défend même avec une connexion occasionnelle.

*Réponse* (porteur du projet, 10 novembre 2026) : recommandation retenue. Les lots C et D
attendent donc une école qui les demande et accepte l'abonnement ; aucune n'a été nommée.

---

## 4. Décisions à écrire en ADR

Aucune n'est tranchée ici. Chacune s'écrit avant l'étape qui en dépend, avec ses options
comparées et ses versions vérifiées le jour même.

Les décisions 1 à 4 sont acceptées par le porteur le 10 novembre 2026 :
`docs/adr/2026-11-10-serveur-reseau-local.md`, `2026-11-10-poste-relie.md`,
`2026-11-10-securite-reseau-local.md` et `2026-11-10-modification-simultanee.md`. La
spécification du lot A, `SPEC_PHASE_11_A.md`, est proposée puis acceptée le même jour.

Décision 5, pour sa partie « postes » : le porteur répond le 10 novembre 2026 « les deux
options » — une formule où les postes sont compris dans la licence, et une formule où ils se
vendent par poste. Les prix restent à fixer par le porteur. La façon dont la licence le porte
est dans `docs/adr/2026-11-10-licence-et-postes.md`, accepté le même jour avec un poste relié à
l'essai quand la licence ne dit rien ; la spécification du lot A passe en v1.2. La partie « abonnement de l'édition
Connectée » reste ouverte jusqu'au lot C.

| N° | Décision | Options à comparer | Avant |
|---|---|---|---|
| 1 | Serveur embarqué : retour d'axum dans `gesco-app` | Version et fonctionnalités d'axum ; serveur compilé toujours ou derrière une option de compilation (disque) | A1 |
| 2 | Ce qu'est un poste relié, et où vit son client réseau | GESCO installé seulement, ou aussi un navigateur ou un téléphone (non demandé : T-82 s'en tient à un poste de l'école) ; appels faits par l'écran (`transportHttp`, déjà déclaré) ou relayés par le binaire du poste relié, qui sait retenir l'empreinte d'un certificat auto-signé — ce que le webview ne sait pas faire sans alerte ; dépendance d'un client HTTP en Rust | A2 |
| 3 | Sécurité du réseau local | Certificat auto-signé et empreinte retenue à l'appairage, ou autre ; forme du code d'appairage ; révocation ; où vit la clé du certificat et si elle entre dans la sauvegarde | A2 |
| 4 | Deux personnes sur la même fiche | La dernière écriture gagne ; refus si la fiche a changé depuis sa lecture (`mis_a_jour_le`) ; verrou de fiche. Peut différer selon l'écran (notes, fiche élève, grille de tarifs) | A4 |
| 5 | Tarification et licence | Réseau local compris dans la licence Essentielle (§26.2 le range là) ou vendu par poste comme un module ; prix et durée de l'abonnement Connectée ; ce que la licence doit porter de plus. Attention : `Licence` refuse tout champ inconnu, donc une licence émise avec un champ nouveau serait rejetée par une 1.2.x | A6, C2 |
| 6 | Hébergement | Pays et hébergeur ; machine louée ou service géré ; qui l'exploite et qui répond en cas de panne ; nom de domaine ; coût par école ; conformité à la loi malienne sur les données personnelles, validée par un juriste (§24.3) | C1 |
| 7 | Ce qui monte en ligne, et sous quelle forme | Copie d'une partie du schéma sur le serveur, qui recalcule (ADR-002, ADR-003 : PostgreSQL) ; ou vues déjà calculées par le poste de l'école, que le serveur ne fait que ranger et montrer ; base du serveur ; les bulletins en PDF comptent-ils comme des médias interdits de synchronisation (§15.2) | C2 |
| 8 | Périmètre exposé aux parents et identité en ligne | Ce que T-85 aura retenu, écrit une fois pour le SMS et le portail ; le code d'accès de six caractères suffit-il sur Internet, ou faut-il un second contrôle (SMS de confirmation, mot de passe choisi) ; durée de session sur le téléphone | D1 |
| 9 | Conflits de synchronisation | Maître unique (l'école écrit, le service en ligne lit, §15.2), où seuls les messages des parents descendent et ne se modifient jamais ; ce que devient cette règle avec plusieurs postes, puis avec plusieurs écoles d'une organisation (Q-36) | C2 |
| 10 | Tables de l'évaluation par compétences | Entités du §8.4 B telles quelles ou simplifiées ; échelle fixe à trois niveaux ou réglable ; par année scolaire (ADR-012) | B1 |

---

## 5. Version et branche

**Version proposée : la ligne 2.x.** `2.0.0` pour le premier lot validé (plusieurs postes, avec le
livret du jardin s'il est prêt), puis `2.1.0` et `2.2.0` pour les lots suivants. Raisons :

- l'installation change de nature (poste principal, postes reliés, pare-feu, guide à part) : ce
  n'est plus « la même chose en mieux », et `docs/versions.md` appelle déjà `developpement`
  « la suite de la version 2 » ;
- la licence peut changer de forme (décision 5) ;
- la `1.2.x` reste la ligne de corrections des écoles à un poste, tant que la 2.0 n'a pas fait ses
  preuves dans une école.

L'autre choix, `1.3.0`, garderait la suite 1.0, 1.1, 1.2 où chaque phase a donné un numéro
mineur ; il se défend si le porteur veut réserver « version 2 » à l'édition Connectée.

**Branche proposée : `developpement`**, comme le prévoit `docs/versions.md`, sans branche
nouvelle. Deux conditions avant d'y écrire :

1. la 1.2.7 est validée, commitée et étiquetée sur `1.2.x` (elle ne l'est pas aujourd'hui) ;
2. `developpement` reçoit `1.2.x` : au 10 novembre elle a 14 commits de retard (elle en est à la
   1.2.5 du 24 octobre) et aucun d'avance, la mise à jour se fait donc sans conflit.

**Révision du 11 novembre 2026 (porteur du projet)** : un seul installeur pour le réseau local,
l'e-learning et la sauvegarde hors de l'école. La version 2.0.0 ne sort donc plus au premier lot
validé : elle sort quand le lot A (plusieurs postes), la sauvegarde hors de l'école (§7) et la
phase 12 (e-learning) sont tous validés ; le livret du jardin (lot B) y entre s'il est prêt. Les
suites `2.1.0` et `2.2.0` prévues ci-dessus n'ont plus lieu d'être pour ces lots. Chaque étape
reste validée une à une sur `developpement`, sans installeur publié entre-temps. Conséquence à
garder en tête : les écoles restent en 1.2.x jusque-là, et attendent donc plusieurs postes aussi
longtemps que l'e-learning.

**Seconde révision du 11 novembre 2026 (porteur du projet)** : la phase 9 (rapports de rentrée et
de fermeture imprimés, statistiques de l'école pour l'enquête CPS) entre aussi dans la version
2.0, après le lot A. Les décisions sont dans `ETAT_AVANCEMENT.md`, section « Phase 9 ».

À la sortie de la 2.0.0 : étiquette `v2.0.0`, branche `2.0.x` pour ses corrections, `main` avance.
Les lots C et D amèneront un second livrable, le serveur, qui aura ses propres versions : à
décider avec l'hébergement.

---

## 6. Suite proposée

1. ~~Le porteur répond à T-80 à T-86.~~ Fait le 10 novembre 2026 ; version 2.0.0 et branche
   `developpement` acceptées avec le reste. Les réponses sont à recopier dans
   `docs/questions-terrain.md` quand la 1.2.7 sera commitée.
2. ADR 1 à 4 proposés, puis spécification du lot A, soumise avant toute ligne de code.
3. Étape A1, une fois la 1.2.7 étiquetée et `developpement` mise à jour (§5).
4. Lot B (livret du jardin) après le lot A, ou avant si une école à jardin l'attend.
5. Lots C et D : en attente d'une école qui les demande et accepte l'abonnement (T-86) ; rien à
   lancer hors du code d'ici là.

---

## 7. Demande du 11 novembre 2026 : mettre les sauvegardes à l'abri sur le cloud

Demande du porteur du projet, le 11 novembre 2026 : « une dernière phase : possibilité d'héberger
en ligne et/ou sauvegarder sur le cloud comme Google Drive ou tout autre », « pour qu'en local,
en plus de la sauvegarde sur clé, il ne perde pas les données ». Le but est donc de ne pas perdre
les données d'une école si l'ordinateur et la clé USB disparaissent ensemble (vol, incendie).

### 7.1 Ce que GESCO sait déjà faire

Vérifié dans `gesco-services/src/sauvegarde.rs` : une destination de sauvegarde est un simple
dossier ; chaque sauvegarde y est copiée, avec la même rotation ; une destination absente
n'empêche pas les autres. L'archive `.gsv` est chiffrée par la clé de l'école et se restaure sur
un autre ordinateur avec les douze mots.

Conséquence : **un dossier synchronisé par Google Drive, OneDrive ou Dropbox peut déjà servir de
destination**, sans une ligne de code, dès la 1.2.x. Le logiciel du fournisseur envoie l'archive
quand il y a Internet ; le fournisseur ne peut pas la lire. Non essayé sur un vrai compte.

### 7.2 Ce qui manque pour en faire une vraie fonction

- GESCO ne sait pas qu'une destination est « en ligne » : il ne la propose pas, ne l'explique
  pas, et n'avertit pas l'école qui n'a aucune copie hors de ses murs.
- Il ne sait pas si l'envoi a vraiment eu lieu : seul le logiciel du fournisseur le sait.
- **Le forfait** : chaque sauvegarde est une archive entière. Une sauvegarde par jour envoyée en
  ligne peut coûter cher en données (blueprint §3.2). Il faut un rythme à part pour cette
  destination (une fois par semaine, ou à la demande), et afficher la taille de ce qui part.
- Un dossier synchronisé ne protège pas d'un rançongiciel comme le fait une clé débranchée : il
  s'ajoute à la clé, il ne la remplace pas.
- Le guide d'installation n'en dit rien.

### 7.3 Lot proposé — « Sauvegarde hors de l'école »

Il remplace l'étape C1 du §2.4, sans serveur de l'éditeur ni abonnement.

| Étape | Contenu | Risques | Vérifiable par |
|---|---|---|---|
| S1 — Destination « en ligne » | Troisième sorte de destination à côté du dossier du poste et du support déconnectable ; GESCO propose les dossiers Google Drive, OneDrive et Dropbox qu'il trouve sur le poste ; rythme choisi par l'école : à la demande (rien ne part seul, réglage livré), chaque semaine ou chaque mois ; taille affichée ; avertissement au tableau de bord quand aucune copie n'existe hors de l'école | Forfait consommé ; logiciel du fournisseur non installé ou déconnecté | Essais de services ; captures ; une archive déposée dans un dossier synchronisé puis restaurée sur un autre poste avec les douze mots |
| S2 — Guide et essai réel | Guide : installer le logiciel du fournisseur, choisir le dossier, vérifier que l'envoi se fait ; essai du porteur sur un vrai compte | L'école croit être protégée alors que rien ne part | Liste d'essais à la main sur un vrai compte ; restauration depuis le fichier téléchargé |

**Rythme** (porteur du projet, 11 novembre 2026) : « pas de sauvegarde automatique, ou il choisit
hebdomadaire ou mensuel ». La copie en ligne ne part donc jamais chaque jour : l'école choisit
entre à la demande, chaque semaine et chaque mois. Livré sur « à la demande » (interprétation à
confirmer : le réglage de départ est celui qui ne consomme rien sans un geste). Les sauvegardes
du poste et de la clé gardent leur rythme actuel.

Non retenu dans ce lot, à décider en ADR si le dossier synchronisé ne suffit pas : GESCO qui se
connecte lui-même au compte Google de l'école (pas de logiciel à installer, mais un agrément à
obtenir de Google par l'éditeur, un code par fournisseur, des jetons à garder). L'hébergement en
ligne de GESCO lui-même reste le lot C (§2.4), en attente d'une école (T-86).

### 7.4 Questions de terrain

À la suite de T-94, dernière question de `docs/plan-phase-12.md` au 11 novembre 2026.

#### T-95 — Ce que « en ligne » doit apporter

- a) seulement mettre les sauvegardes à l'abri hors de l'école ;
- b) aussi travailler ou consulter à distance (le directeur chez lui, deux sites d'une même école).

*Recommandation* : a) maintenant. b) est l'hébergement en ligne du lot C : un serveur à louer et
à surveiller, donc un abonnement ; il attend une école qui le demande.

#### T-96 — Par quel moyen

- a) un dossier synchronisé par le logiciel du fournisseur (Google Drive, OneDrive, Dropbox), sur
  le compte de l'école ;
- b) GESCO se connecte lui-même au compte Google de l'école ;
- c) un serveur de l'éditeur, par abonnement.

*Recommandation* : a). Rien à payer à l'éditeur, l'école garde la main sur son compte, cela
marche avec n'importe quel fournisseur, et l'essentiel existe déjà.

#### T-97 — Quand

- a) en dernière phase, après l'e-learning ;
- b) dans la version 2.0, comme une petite étape à part.

*Recommandation* : b). C'est court, cela ne dépend ni du réseau local ni de l'e-learning, et
c'est ce qui protège une école d'une perte totale. En attendant, une école peut déjà ajouter à
la main un dossier Google Drive comme destination de sauvegarde.

*Réponse* (porteur du projet, 11 novembre 2026) : « dans le même installeur que réseau,
e-learning ». La sauvegarde hors de l'école sort donc dans la version 2.0, avec le réseau local
et l'e-learning, en un seul installeur (voir §5, révision du 11 novembre 2026).
