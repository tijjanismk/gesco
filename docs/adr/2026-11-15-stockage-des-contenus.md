# Le stockage des contenus : le livré dans son paquet signé, l'école dans la base, les vidéos dans un dossier à part

**Date** : 15 novembre 2026
**Statut** : proposé le 15 novembre 2026 — décision 5 de `docs/plan-phase-12.md` §5, lot 1 de la phase 12 (`SPEC_PHASE_12_LOT_1.md`)

## Contexte

Trois sortes de contenus, qui ne vivent pas de la même façon :

1. **La bibliothèque livrée** : 398 leçons, 41 sujets officiels et leurs corrigés, 285 figures
   SVG, 18 Mo de texte, quelques mégaoctets une fois compressés. Elle est la même dans toutes
   les écoles, se réinstalle, et change par mise à jour. Les gigaoctets de documents rassemblés
   ne sont plus livrés (décision 7 du plan : rien de recopié).
2. **Ce que l'école écrit** : ses versions des leçons livrées, ses leçons nouvelles, ses
   chapitres, les images et les documents qu'elle y joint (par exemple la photo d'un sujet que
   la bibliothèque n'a pas pu lire, décision du porteur du 14 novembre 2026). C'est son travail :
   il ne doit jamais se perdre et doit voyager avec sa sauvegarde.
3. **Les vidéos de l'école** (décision du porteur du 12 novembre 2026) : ajoutées par l'école,
   lues sans Internet et depuis les postes reliés, **jamais dans la sauvegarde** ; un bouton les
   copie vers une clé et les reprend pour changer d'ordinateur ; une vidéo absente ne bloque
   rien ; MP4 seulement, avertissement sans refus au-delà d'une taille conseillée.

Ce qui existe :
- La sauvegarde `.gsv` contient l'instantané de la base chiffrée et les modèles sur mesure,
  rien d'autre (ADR du 4 octobre 2026).
- Les pièces des dépenses et du dossier du personnel sont rangées **dans la base chiffrée**
  (colonne `contenu BLOB`, empreinte SHA-256), donc dans la sauvegarde sans rien de plus.
- Le réseau local échange des fichiers par leur contenu, jusqu'à 20 Mo par fichier
  (`TAILLE_MAX_FICHIER`) et 32 Mo par appel.
- Les postes d'école ont un disque compté (24 Go libres sur le poste du porteur, 18 Go le
  14 novembre 2026).

## Options

| | 1. Trois places : paquet signé, base chiffrée, dossier des vidéos | 2. Tout dans la base chiffrée | 3. Tout dans un dossier adressé par empreinte, hors de la base |
|---|---|---|---|
| Apprentissage | Faible : chaque place reprend un mécanisme existant | Très faible | Moyen : un dossier nouveau à chiffrer, à sauvegarder, à servir |
| Exploitation | Sauvegarde inchangée ; vidéos à copier à part, comme décidé | La sauvegarde grossit de la bibliothèque, et des vidéos | La sauvegarde doit apprendre à prendre un dossier |
| Maturité | Pièces jointes et archive éprouvées depuis la phase 0.5 | Éprouvé pour de petits fichiers seulement | À écrire |
| Pour un agent de code | Bon : trois règles simples | Très bon | Moyen |
| Plus gros risque | Une école qui change d'ordinateur sans copier ses vidéos | Une base de plusieurs gigaoctets avec des vidéos : sauvegarde lente, poste saturé | Un dossier oublié par la sauvegarde : travail perdu |

## Décision

Option 1.

- **Livré** : le paquet signé reste tel quel, en un fichier, dans le dossier de données de
  GESCO (`bibliotheque\`), hors de la base et hors de la sauvegarde ; il n'est jamais modifié.
  La base ne garde que l'index du paquet (une ligne par contenu : clé, chapitre, titre,
  nature, empreinte) et l'état de l'école (retraits, versions, relectures). Le paquet est livré
  dans l'installeur et se remplace depuis une clé ; après la restauration d'une sauvegarde sur
  un autre ordinateur, l'index se refait depuis le paquet installé.
- **École** : textes dans des tables de la base ; images (SVG, PNG, JPEG) et documents (PDF)
  joints dans la base chiffrée, comme les pièces des dépenses, adressés par leur empreinte et
  gardés une fois. Bornes : 5 Mo par image, 20 Mo par document (la limite du réseau local). Ils
  sont donc dans la sauvegarde, chiffrés, sans changer l'archive.
- **Vidéos** : fichiers MP4 dans `videos\` du dossier de données du poste principal, nommés par
  leur empreinte SHA-256, ni chiffrés ni sauvegardés ; la base garde leur nom, leur taille,
  leur empreinte et qui les a ajoutées. Taille conseillée 200 Mo, avertissement sans refus
  au-delà. Un poste relié les lit par morceaux sur une route du réseau local réservée aux
  postes appairés. Copier vers une clé et reprendre depuis une clé : un bouton chacun, annoncé
  à l'ajout de la première vidéo. Une vidéo absente du dossier laisse la leçon s'ouvrir, avec
  à sa place la ligne « Vidéo non disponible sur cet ordinateur » pour le personnel seulement.

Aucune dépendance nouvelle.

## Justification

1. Ce qui se réinstalle n'a rien à faire dans la sauvegarde ; ce que l'école a écrit doit y
   être sans qu'elle y pense. La règle suit cette ligne.
2. Ranger le travail de l'école dans la base reprend un mécanisme éprouvé : chiffrement,
   sauvegarde, restauration, échange par le réseau local, tout existe déjà.
3. Le paquet signé n'est jamais touché : sa signature reste vérifiable à chaque ouverture, et
   une mise à jour se fait en remplaçant un fichier.
4. Les vidéos hors de la sauvegarde sont une décision du porteur : une sauvegarde de quelques
   mégaoctets reste rapide et tient sur une clé ou dans un dossier synchronisé.
5. Les bornes de 5 et 20 Mo gardent la base et la sauvegarde légères, et collent à la limite
   déjà en place sur le réseau local.

## Conséquences

Positives : la sauvegarde ne change pas de format ; une école qui écrit beaucoup de leçons voit
sa sauvegarde grossir de quelques mégaoctets, pas de gigaoctets ; le paquet se contrôle par sa
signature ; une vidéo perdue ne casse aucune leçon.

Négatives : une école qui change d'ordinateur sans copier ses vidéos les perd (le guide et le
bouton le disent à la première vidéo) ; les vidéos ne sont pas chiffrées sur le disque, et
peuvent montrer des élèves (le guide conseille de filmer le tableau et les mains, pas les
visages) ; un document de plus de 20 Mo est refusé avec un message clair ; la lecture d'une
vidéo depuis un poste relié demande une route nouvelle dans `gesco-reseau`.

## Contre-argument conservé

Tout dans la base serait plus simple : une seule règle, et la sauvegarde contiendrait tout,
vidéos comprises. Il n'a pas gagné parce qu'une seule vidéo d'une demi-heure pèse plus que toute
la base d'une école, que la sauvegarde deviendrait trop lourde pour une clé ou une connexion, et
que le porteur a décidé que les vidéos restent dehors.

## Conditions de reconsidération

- Si des écoles joignent régulièrement des documents de plus de 20 Mo (manuels numérisés).
- Si la base d'une école dépasse 500 Mo à cause des images et documents de la bibliothèque.
- Si le porteur décide que les vidéos doivent voyager avec la sauvegarde hors de l'école.
- Si une école demande que les vidéos soient chiffrées sur le disque.
