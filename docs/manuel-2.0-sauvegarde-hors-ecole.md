# GESCO 2.0 — manuel d'utilisation : la sauvegarde hors de l'école

Chapitre du manuel de la version 2.0, à placer après « Sauvegarde et restauration » quand le
manuel de la 2.0 sera assemblé. Il concerne l'administrateur et la direction, sur le poste
principal : un poste relié n'a pas l'écran des sauvegardes.

La clé USB protège l'école contre une panne de l'ordinateur. Elle ne la protège pas si
l'ordinateur et la clé disparaissent ensemble : vol, incendie, inondation. La copie hors de
l'école met une copie de la sauvegarde sur un compte en ligne qui appartient à l'école (Google
Drive, OneDrive ou Dropbox), sans abonnement chez l'éditeur. **Elle s'ajoute à la clé USB, elle
ne la remplace pas.** Une école qui ne s'en sert pas ne voit rien changer.

Avant de commencer, le logiciel du fournisseur doit être installé sur le poste principal et
connecté au compte de l'école. Le guide d'installation l'explique (« Une copie hors de
l'école »).

---

## 1. Ce qui part, et ce qui ne part jamais

- **Seule l'archive de sauvegarde part** : un fichier `.gsv`, déjà chiffré par la clé de l'école.
  Sans les douze mots, personne ne peut la lire, pas même le fournisseur.
- **Ne partent jamais** : la base en clair, la clé de l'école, les douze mots. GESCO n'écrit
  rien d'autre dans le dossier.
- **Le fournisseur voit seulement** le nom du fichier (une date), sa taille, le nom de l'école,
  la date et la version de GESCO. Aucune donnée d'élève, de personnel ou d'argent.
- **Ne rangez jamais le certificat des douze mots**, ni sa photo, dans ce dossier ni sur ce
  compte.
- Les vidéos ajoutées à la bibliothèque ne sont pas dans la sauvegarde : elles ne partent pas non
  plus.

## 2. La section « Copie hors de l'école » (Outils → Sauvegarde)

Elle se trouve entre l'historique et les destinations.

- **Dossier.** GESCO montre les dossiers Google Drive, OneDrive et Dropbox qu'il trouve sur
  l'ordinateur, par exemple « OneDrive — C:\Users\…\OneDrive\GESCO ». **Choisir** retient ce
  dossier ; GESCO y crée le sous-dossier `GESCO` à la première copie. **Choisir un autre
  dossier…** ouvre la fenêtre de Windows : tout dossier convient, avec n'importe quel
  fournisseur. **Retirer** arrête les copies vers ce dossier ; les copies déjà déposées y restent.
  Rien n'est choisi à l'installation.
- **Rythme.**
  - *À la demande* (réglage livré) : rien ne part sans un geste de l'école.
  - *Chaque semaine* : une copie part quand la dernière a sept jours.
  - *Chaque mois* : une copie part le même jour du mois suivant (le 31 mars donne le 30 avril).

  Sous les choix, GESCO dit combien pèse une copie et, pour la semaine et le mois, combien cela
  fait à peu près par mois. Une copie ne part jamais chaque jour, ni à la fermeture de GESCO :
  elle part pendant que GESCO est ouvert sur le poste principal. Si GESCO reste fermé une
  semaine, la copie part à l'ouverture suivante.
- **Envoyer une copie maintenant.** Le bouton n'apparaît que lorsqu'un dossier est choisi.
  Au-dessus, GESCO dit combien de mégaoctets partiront par Internet. La copie part tout de suite
  dans chaque dossier choisi, et dans les autres destinations comme une sauvegarde ordinaire.
  Elle repousse la copie suivante du rythme.
- **Dernière copie.** Pour chaque dossier : la date, la taille et l'état (§3) ; au rythme semaine
  ou mois, la date de la prochaine copie.

Les autres sauvegardes (« Sauvegarder maintenant », celles faites à intervalles réguliers, à la
fermeture, avant un import ou une facturation, à l'ouverture ou à la clôture d'une année) ne
partent pas hors de l'école : elles vont à l'ordinateur et à la clé, comme avant.

## 3. Les états d'une copie

GESCO vérifie la dernière copie de chaque dossier quelques minutes après le dépôt, une heure
après, au démarrage, puis une fois par jour. Il ne relit jamais son contenu, et ne fait donc
jamais retélécharger une copie que le fournisseur a libérée du disque.

| État | Ce qu'il veut dire | Que faire |
|---|---|---|
| Envoyée | Windows dit que le fichier est sur le compte | Rien |
| En attente d'envoi | Windows dit que le fichier n'est pas encore envoyé | Laisser l'ordinateur allumé et connecté à Internet |
| Déposée dans le dossier | La copie est intacte dans le dossier ; Windows ne dit rien de l'envoi | Vérifier une fois sur le site du fournisseur (§5) |
| Envoi en échec | Le logiciel du fournisseur n'a pas pu l'envoyer | Ouvrir le logiciel du fournisseur : compte déconnecté, espace plein |
| N'a pas pu être déposée | Le dossier manquait, était plein ou refusé | Vérifier que le logiciel du fournisseur est installé et connecté, puis envoyer une copie |
| N'est plus dans le dossier | Le fichier a été effacé ou modifié depuis le dépôt | Envoyer une copie |

« Envoyée » n'apparaît que si Windows le dit. Selon le fournisseur, l'état peut rester
« Déposée dans le dossier » : ce n'est pas une erreur.

Quand la dernière copie n'est pas arrivée (« N'a pas pu être déposée », « N'est plus dans le
dossier », « Envoi en échec », ou « En attente d'envoi » depuis plus de sept jours), une ligne le
dit dans le bandeau ; un clic ouvre l'écran des sauvegardes. Elle disparaît dès qu'une copie
suivante est déposée. Rien de cela ne bloque : la sauvegarde sur l'ordinateur et sur la clé se
fait toujours.

L'historique montre aussi, pour chaque ligne, la **sorte** de la destination et, pour un dossier
synchronisé, son **envoi**.

## 4. Combien de copies restent sur le compte

GESCO garde dans le dossier une copie par semaine pour les 4 dernières semaines et une copie par
mois pour les 12 derniers mois : 16 copies au plus. Il ne supprime que les copies qu'il a
lui-même déposées pour cette école ; un autre fichier, ou la copie d'une autre école, n'est
jamais touché. La copie la plus récente n'est jamais supprimée. Le fournisseur garde en général
les fichiers supprimés quelques jours dans sa corbeille.

## 5. Vérifier sur le site du fournisseur

Une fois après le premier envoi, puis de temps en temps :

1. Sur un téléphone ou un autre ordinateur, ouvrez le site du fournisseur avec le compte de
   l'école (drive.google.com, onedrive.live.com ou dropbox.com).
2. Ouvrez le dossier `GESCO` : la dernière copie y est, avec la date du jour dans son nom
   (`gesco-AAAAMMJJ-…`) et une taille proche de celle que GESCO affiche.

## 6. Restaurer depuis le fichier téléchargé

Après un vol ou un incendie, sur un ordinateur neuf ou non :

1. Sur le site du fournisseur, téléchargez la dernière copie `.gsv` du dossier `GESCO`.
2. Installez GESCO. Au premier écran, choisissez **restaurer une sauvegarde** et désignez le
   fichier téléchargé.
3. Saisissez les **douze mots** du certificat.
4. Réinstallez la licence, puis, si l'école en avait, reprenez les vidéos depuis leur clé.

Sur un ordinateur où GESCO tourne déjà, l'administrateur restaure en bas de l'écran des
sauvegardes, partie « Restaurer une sauvegarde ».

## 7. Plusieurs ordinateurs, plusieurs comptes Windows

- La copie hors de l'école se règle et part depuis le **poste principal** seulement.
- Le dossier appartient au compte Windows où le logiciel du fournisseur est connecté. Si le poste
  principal sert à plusieurs comptes Windows, la copie n'est déposée que lorsque GESCO tourne
  sous ce compte ; sous un autre, elle « n'a pas pu être déposée ».
- Un dossier synchronisé ne remplace pas la clé USB : un virus qui chiffre l'ordinateur peut
  atteindre ce dossier. L'avertissement « Aucune destination de sauvegarde déconnectable n'est
  configurée. » reste donc tant qu'aucune clé n'est réglée comme support amovible.
