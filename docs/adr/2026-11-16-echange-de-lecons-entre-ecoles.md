# Échange de leçons entre écoles : un fichier `.gles` non signé, reçu en brouillons à valider

**Date** : 16 novembre 2026
**Statut** : proposé le 16 novembre 2026
**Précise** : étape 2.3 de `docs/plan-phase-12.md` §3.4 ; ADR « paquet de contenus », « repère
commun des contenus », « retirer un contenu » et « stockage des contenus » (15 novembre 2026)
**Spécification** : `SPEC_PHASE_12_LOT_2.md`

## Contexte

- Le lot 1 laisse chaque école écrire ses chapitres, ses leçons, ses sujets, ses documents et ses
  versions des leçons livrées ; il a laissé de côté l'envoi de ce travail à une autre école
  (place réservée). La 2.0 garde tout son périmètre : l'échange y entre.
- Les écoles d'un même promoteur, ou d'un même CAP, se passent déjà des fiches sur clé ou par
  WhatsApp.
- Chaque école a ses matières : un contenu se range par les codes nationaux de niveau, de série
  et de discipline et par la correspondance des disciplines de l'école (ADR « repère commun »).
- Les leçons livrées ont une clé stable dans le paquet ; deux écoles qui ont le même paquet ont
  les mêmes clés.
- Le paquet de l'éditeur est signé par une clé que seul l'éditeur détient ; une école ne peut pas
  signer.
- Ce qui arrive d'ailleurs ne doit jamais devenir visible des élèves sans un geste de l'école qui
  reçoit (T-89 : la direction valide avant que les élèves voient).

## Options

| | 1. Fichier `.gles` sur le modèle du paquet, sans signature, reçu en brouillons | 2. Réutiliser la sauvegarde `.gsv` | 3. Envoyer chaque leçon en PDF |
|---|---|---|---|
| Apprentissage | Faible : lecture du `.gbib` déjà écrite | Faible | Nul |
| Exploitation | Un fichier à passer par clé ou WhatsApp | Livre toute la base de l'école (élèves, caisse) | Rien |
| Maturité | Code du lot 1 | Code existant | — |
| Pour un agent de code | Bon | Mauvais : extraire une partie d'une base chiffrée | — |
| Plus gros risque | Un fichier fabriqué pour nuire (texte, SVG) | Fuite de toutes les données d'une école | L'école qui reçoit ne peut rien modifier |

## Décision

Une école exporte des leçons dans un fichier `.gles` (manifeste JSON et fichiers, compressés,
avec l'empreinte de chaque fichier, sans signature) ; l'école qui l'importe reçoit chaque leçon en
brouillon, rangée par les codes nationaux, à valider par sa direction avant que les élèves la
voient.

Aucune brique nouvelle : `flate2` 1.1.10 et la lecture du paquet du lot 1, déjà dans le dépôt.
Vérifié le 16 novembre 2026 : `flate2` dans `Cargo.lock`.

Ce que la décision fixe :

- **Ce qui part** : au choix de qui exporte, un chapitre de l'école, une matière d'un niveau ou
  une sélection de leçons ; pour chacune, sa **version visible** (jamais un brouillon), ses
  images, ses documents et ses sons ; pas les vidéos (trop lourdes : le bouton « Copier les
  vidéos vers une clé » existe), pas l'historique, pas les noms des auteurs, pas le nom de
  l'école. Une version de l'école d'une leçon livrée part avec la clé de la leçon livrée.
- **Qui** : exporter demande `library:edit` dans la portée de la personne ; importer demande
  `library:manage`.
- **À l'import** : le fichier est refusé entier si une empreinte ne correspond pas ; chaque texte
  passe par le filtre du noyau (aucun HTML, aucun lien extérieur), chaque SVG par le nettoyage du
  lot 1, chaque fichier par les bornes du lot 1. Puis, leçon par leçon :
  - une version d'une leçon livrée que l'école a aussi devient un **brouillon** de cette leçon,
    au nom de la personne qui importe, qui suit le circuit du lot 1 ; si l'école a déjà un
    brouillon de cette leçon, la version reçue s'ajoute à l'historique comme « version reçue »,
    à comparer, sans rien écraser ;
  - une leçon de l'école devient une **leçon de l'école en brouillon**, sous un chapitre du même
    titre au même niveau et dans la discipline correspondante, créé au besoin ;
  - une discipline sans correspondance se range sous son libellé de référence (règle du lot 1).
- **Écran d'import** : la liste de ce qui arrive et où cela se rangera, avant de confirmer ; un
  résumé après (« 12 leçons reçues en brouillon »).
- **Le même fichier importé deux fois** ne double rien : une leçon dont l'empreinte du texte est
  déjà dans l'historique est ignorée, et le résumé le dit.
- **Journal** : export et import vont au journal d'audit, avec le nombre de leçons.

## Justification

Le format du paquet existe et se lit déjà : le réutiliser sans signature coûte peu. La signature
ne protège ici de rien, puisque n'importe quelle école peut fabriquer un fichier ; ce qui protège,
c'est le filtre du texte, le nettoyage des SVG et l'arrivée en brouillon. La sauvegarde emporterait
les données des élèves ; le PDF ne se modifie pas.

## Conséquences

Positives : les écoles d'un même groupe se passent leur travail sans rien perdre de leurs
retraits ni de leurs versions ; rien n'est visible des élèves sans la direction qui reçoit.

Négatives : des brouillons à valider en nombre après un gros import ; aucune garantie sur l'origine
d'un fichier ; une école peut recevoir des leçons de mauvaise qualité (elle les retire).

## Contre-argument conservé

Recevoir les leçons déjà publiées ferait gagner du temps à un groupe d'écoles qui se fait
confiance. Il n'a pas gagné parce que la direction de chaque école valide ce que voient ses
élèves (T-89) ; une direction qui veut aller vite publie la liste reçue d'un geste par leçon.

## Conditions de reconsidération

- Un groupe d'écoles demande une publication directe : un réglage « publier les leçons reçues de
  cette école » avec une clé propre au groupe.
- Le service en ligne (lot C) existe : l'échange peut passer par lui.
