# Le paquet de contenus : un fichier signé par l'éditeur, fabriqué par un outil qui refuse ce qui n'est pas en règle

**Date** : 15 novembre 2026
**Statut** : accepté le 15 novembre 2026 (étape 2 du lot 1 : format `.gbib` lu dans le noyau, outil `outils/bibliotheque`, premier paquet fabriqué) ; proposé le 15 novembre 2026 — décision 8 de `docs/plan-phase-12.md` §5, lot 1 de la phase 12 (`SPEC_PHASE_12_LOT_1.md`)

## Contexte

La bibliothèque est rédigée pour GESCO dans `gesco-bibliotheque\contenus`, hors du dépôt. Elle
doit entrer dans chaque école, sans Internet, et se mettre à jour plus tard.

Ce que les fichiers de la bibliothèque contiennent, et qui ne doit pas s'afficher tel quel :
- En tête de chaque leçon : une ligne de classement (« 9e année · Mathématiques · Algèbre ·
  Chapitre I · Leçon 1 »), le point du programme, l'état (« rédigé pour GESCO le…, recalculé par
  un programme, pas encore relu par un enseignant »), parfois « Programme : à confirmer ».
- En tête de chaque énoncé officiel : une ligne « Source : épreuve officielle…, retapée pour
  GESCO d'après la photo du sujet… » et un lien vers le fichier du corrigé.
- Dans certaines leçons : des lignes « Image à chercher : … » (71 leçons) et une section
  « Figures qui manquent », notes de travail des rédacteurs.
- À côté : programmes de vérification `.verif.py` et `.fig.py`, listes de faits à cocher
  `.faits.md` (94), fichiers d'état `ETAT.md`.

Les règles qui s'appliquent : aucun texte affiché ne cite de source (règle de l'interface) ;
rien n'est recopié, sauf les énoncés officiels du DEF et du BAC (décisions des 11 et 14 novembre
2026) ; T-93 et T-94 sont sans objet (aucun manuel d'éditeur, aucun quiz au gabarit) ; chaque
document dit ce qu'il est (E-7) ; un paquet se contrôle tout seul (E-14).

Ce qui existe : la licence est signée par l'éditeur en Ed25519 (`ed25519-dalek` 3.0.0) avec
l'outil `outils/licence` ; l'archive de sauvegarde compresse par `flate2` 1.1.10 et découpe en
blocs ; `sha2` 0.11.0 sert aux empreintes.

## Options

| | 1. Un fichier `.gbib` : manifeste signé, fichiers compressés, outil qui contrôle | 2. Une archive ZIP signée à part | 3. Les fichiers Markdown copiés tels quels avec l'installeur |
|---|---|---|---|
| Apprentissage | Faible : reprend l'archive et la licence | Faible | Très faible |
| Exploitation | Un fichier à poser ou remplacer ; contrôles à la fabrication | Deux fichiers (archive et signature) à garder ensemble | Aucun contrôle ; notes de travail et sources affichées |
| Maturité | Briques déjà dans le dépôt | Crate `zip` à ajouter | — |
| Pour un agent de code | Bon | Bon | Très bon, mais aucune garde |
| Plus gros risque | Un outil de fabrication de plus à tenir | Une signature séparée de son archive | Une « Source : … » ou un chemin `C:\` affiché à l'école |

## Décision

Option 1.

- **Le fichier** : un en-tête (marque et version du format), un manifeste JSON, puis les
  fichiers compressés par `flate2`. Le manifeste porte le code et la version du paquet
  (`2026.11.1`), sa date, la version minimale de GESCO, les chapitres (clé, niveau, séries,
  matière, partie, titre, ordre, « programme à confirmer ») et, pour chaque contenu, sa clé
  stable, sa nature, son chapitre, son titre, son ordre, son contenu lié, l'examen et la
  session d'un sujet, l'empreinte SHA-256 et la taille de chaque fichier, ses notes pour le
  personnel. La signature Ed25519 du manifeste suit ; comme le manifeste contient l'empreinte
  de chaque fichier, elle les couvre tous.
- **La clé** : une paire de clés de l'éditeur propre aux paquets, distincte de celle des
  licences ; GESCO embarque la clé publique. Un paquet dont la signature ne se vérifie pas est
  refusé entier, avec le message « Ce paquet de la bibliothèque est abîmé ou ne vient pas de
  GESCO ».
- **Un seul paquet aujourd'hui**, `bibliotheque-mali`, pour toutes les classes : il pèse
  quelques mégaoctets, il est livré dans l'installeur et se remplace depuis une clé. Le format
  accepte plusieurs paquets (un code chacun) si la bibliothèque grossit.
- **L'outil de fabrication**, `outils/bibliotheque` (membre du dépôt, hors de la suite par
  défaut, comme `outils/remplir`), lit `gesco-bibliotheque\contenus` et :
  - tire du haut de chaque fichier ses métadonnées (titre, classement, nature, examen, session,
    « à confirmer »), puis **retire du texte affiché** les lignes d'état, de source, de lien
    vers un fichier, les mentions de `.verif.py`, les lignes « Image à chercher » et la section
    « Figures qui manquent » ; ces deux dernières deviennent des notes visibles du seul
    personnel qui écrit (« Image à ajouter ici : … ») ;
  - joint à chaque leçon sa liste de faits à cocher, montrée au seul personnel comme liste de
    relecture ;
  - n'emporte ni `.verif.py`, ni `.fig.py`, ni `ETAT.md`, ni rien de `_fusion-2026-11-13` ;
  - **refuse de fabriquer** le paquet si un contenu n'a pas de chapitre (E-1) ou pas de nature
    (E-7), si une image appelée manque, si une formule emploie une commande hors du sous-ensemble
    convertible (ADR « format des leçons »), ou si le texte affiché contient encore une marque
    interdite (« Source : », « .verif.py », « SPEC », « T-1 », « Q-0 », un chemin de disque,
    une adresse web) ;
  - écrit le tableau de couverture du paquet (par classe et matière : chapitres, leçons, sujets,
    corrigés, relues ou non), à côté du paquet, pour l'éditeur (E-14).
- **Crédit des auteurs** : sans objet. Les contenus sont rédigés pour GESCO ; les énoncés
  officiels disent ce qu'ils sont, « DEF 2024 — Mathématiques — Énoncé », ce qui est leur
  nature et non une source. Aucun nom de site, de livre ni d'auteur n'est affiché.

Aucune dépendance nouvelle : `flate2`, `sha2`, `ed25519-dalek`, `serde_json` sont déjà dans le
dépôt.

## Justification

1. Un seul fichier, signé, qui se pose ou se remplace : rien à recoller, rien à oublier.
2. La signature du manifeste couvre chaque fichier par son empreinte : un octet changé et le
   paquet est refusé.
3. L'outil de fabrication est l'endroit unique où les notes de travail, les sources et les
   chemins sont retirés : la règle « l'interface ne cite pas de source » se tient avant
   d'arriver dans l'école, et un essai la vérifie.
4. Les refus de l'outil font des exigences E-1, E-7 et E-14 des contrôles, pas des souhaits.
5. Des clés séparées limitent les dégâts : une clé de paquet perdue ne permet pas de fabriquer
   des licences, et l'inverse.
6. Les briques existent déjà dans le dépôt.

## Conséquences

Positives : un paquet d'une école est exactement celui de l'éditeur ; ses défauts se voient à la
fabrication, pas chez le client ; la mise à jour est un fichier ; le tableau de couverture dit à
l'éditeur où la bibliothèque est mince.

Négatives : un outil de plus à tenir ; une seconde clé privée à garder hors du dépôt, comme celle
des licences ; chaque forme d'en-tête des fichiers de la bibliothèque doit être reconnue par
l'outil, qui refusera la première forme nouvelle (c'est voulu) ; les rédacteurs ne verront leurs
erreurs qu'au passage de l'outil.

## Contre-argument conservé

Copier les fichiers Markdown tels quels serait le plus simple et laisserait l'école lire les
fichiers hors de GESCO. Il n'a pas gagné parce que ces fichiers portent des sources, des états
et des notes de travail que l'interface ne doit pas montrer, et qu'aucun contrôle ne garantirait
qu'un paquet incomplet ou altéré n'arrive pas dans une école.

## Conditions de reconsidération

- Si un paquet dépasse 100 Mo : le découper par cycle.
- Si l'éditeur veut publier des mises à jour par Internet (lot C de la phase 11) : le format
  reste, le transport change.
- Si une école doit recevoir des contenus d'un autre éditeur : il faudra une liste de clés
  reconnues.

## Mise en œuvre

- 15 novembre 2026, étape 2 : les images sont rangées dans le paquet par dossier de contenu (`images/<dossier>/<nom>`), deux leçons appelant deux figures différentes du même nom. Le tableau de couverture ne compte pas les leçons « relues » : sur décision du porteur du 15 novembre 2026, aucun texte ne parle de relecture. La mention « Source : » n'est refusée qu'en tête de ligne (« le Niger prend sa source : » est du texte de leçon).
