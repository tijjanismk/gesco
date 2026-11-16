# Copies rendues en photo : réduites sur le téléphone, nettoyées, rangées dans la base chiffrée

**Date** : 16 novembre 2026
**Statut** : proposé le 16 novembre 2026, accepté le 16 novembre 2026 par le porteur du projet
**Précise** : ADR « stockage des contenus » (15 novembre 2026) ; étape 5.2 de
`docs/plan-phase-12.md` §3.7 ; exigence E-13 ; règle R-14 (rien ne se supprime)
**Spécification** : `SPEC_PHASE_12_LOT_5.md`

## Contexte

- Au lot 5, l'élève rend un devoir par le portail : un texte tapé, ou des photos de sa copie.
  Rédiger au téléphone se fera peu ; photographier une copie papier est le geste réaliste.
- Une photo de téléphone pèse de 2 à 8 Mo et porte souvent des métadonnées (lieu GPS, modèle de
  l'appareil). Ce sont des copies de mineurs.
- Ordre de grandeur : 60 élèves, 3 photos, 10 devoirs par classe et par an, 10 classes : 18 000
  photos par an. À 5 Mo, 90 Go ; à 300 Ko, 5,4 Go.
- La sauvegarde `.gsv` emporte la base chiffrée ; elle part sur une clé et, depuis la 2.0, vers un
  dossier synchronisé hors de l'école (forfait compté).
- R-14 : aucune ligne ne se supprime. Le lot 1 range images et documents de l'école dans la base,
  et les vidéos dans un dossier à part, hors sauvegarde.

## Options

| | 1. Réduites sur le téléphone, nettoyées par le noyau, dans la base chiffrée | 2. Telles quelles, dans un dossier à part, hors sauvegarde | 3. Dans la base, telles quelles |
|---|---|---|---|
| Apprentissage | Moyen : réduction dans la page, lecture des segments JPEG | Faible | Faible |
| Exploitation | Base et sauvegardes plus grosses, mesurées | Copies perdues avec le disque | Base de dizaines de gigaoctets |
| Maturité | Canvas des navigateurs ; JPEG bien connu | Code des vidéos du lot 1 | — |
| Pour un agent de code | Bon | Bon | Bon |
| Plus gros risque | Une photo trop réduite pour se lire | Une note sans la copie qui la justifie | Sauvegarde impossible à envoyer hors de l'école |

## Décision

La page du portail réduit chaque photo à 1 600 pixels au plus sur le grand côté, en JPEG de
qualité moyenne (environ 300 Ko), avant l'envoi ; le noyau vérifie le fichier, retire toute
métadonnée et le range dans la base chiffrée, par son empreinte ; une fois l'année close, la
direction peut libérer la place des photos de cette année, les lignes et les notes restant.

Aucune brique nouvelle : la réduction se fait par le canvas du navigateur ; le noyau lit les
segments du JPEG lui-même et ne garde que l'image (pas de décodage). `image` 0.25.10 est dans
`Cargo.lock` par Tauri, mais n'est pas retenue en dépendance du noyau (poids de compilation).
Vérifié le 16 novembre 2026 sur crates.io et dans `Cargo.lock`.

Ce que la décision fixe :

- **Par rendu** : cinq photos au plus (T-144), 2 Mo au plus chacune après réduction ; un fichier
  qui n'est pas un JPEG ou un PNG (reconnu par ses premiers octets) est refusé avec un message
  simple. Un PNG est rangé tel quel après vérification.
- **Nettoyage** : du JPEG, le noyau ne garde que les segments nécessaires à l'image ; les
  segments d'application (EXIF, XMP, vignettes) et les commentaires sont retirés. Un essai vérifie
  qu'une photo avec position GPS ressort sans elle.
- **Rangement** : table des fichiers rendus, dans la base chiffrée, une fois par empreinte ; donc
  dans la sauvegarde.
- **Ce que voit qui** : l'élève voit ses propres photos ; le maître celles de ses classes et de ses
  matières ; la direction toutes ; aucune autre personne, aucune route du portail pour la photo
  d'un autre élève.
- **Libérer la place** (T-144) : après la clôture d'une année, un bouton de la direction vide le
  contenu des photos des devoirs de cette année ; la ligne du rendu, son empreinte, la note et le
  commentaire restent ; la photo est remplacée à l'écran par « Photo retirée pour libérer de la
  place ». C'est le seul écart à R-14 du lot : il vise le contenu d'un fichier, pas une ligne ;
  il s'écrit au journal d'audit.
- **Mesure** : l'écran des sauvegardes montre déjà la taille de la sauvegarde ; l'écran des
  devoirs montre la place prise par les photos de l'année.

## Justification

Réduite, une copie reste lisible (1 600 pixels suffisent pour une page manuscrite) et pèse quinze
fois moins ; c'est ce qui rend la sauvegarde hors de l'école encore possible. La copie justifie
une note qui peut entrer au bulletin : elle doit être sauvegardée, ce qui écarte le dossier à part.
Retirer les métadonnées protège l'adresse de la famille. Libérer la place après la clôture laisse
les copies tant qu'une réclamation est possible.

## Conséquences

Positives : copies lisibles et sauvegardées ; aucune position ni modèle d'appareil gardé ; place
maîtrisée.

Négatives : la sauvegarde grossit de quelques gigaoctets par an dans une grande école qui utilise
beaucoup les devoirs ; la réduction dépend du navigateur du téléphone (un très vieux navigateur
envoie la photo entière, refusée au-delà de 2 Mo avec un message) ; un écart à R-14, borné.

## Contre-argument conservé

Ranger les copies hors de la base, comme les vidéos, garderait la sauvegarde légère. Il n'a pas
gagné parce qu'une note reportée au bulletin doit pouvoir se justifier après la perte d'un disque.

## Conditions de reconsidération

- La sauvegarde d'une école dépasse ce qu'un dossier synchronisé gratuit peut recevoir à cause des
  photos : proposer un rythme de libération plus court, ou les ranger à part.
- Une copie réduite se révèle illisible pour certaines écritures (essai du porteur) : monter à
  2 000 pixels.
