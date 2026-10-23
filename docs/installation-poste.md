# Installer GESCO sur le poste d'une école

Pour la version 1.2.2, du 23 octobre 2026.

## Ce qu'il faut emporter

Sur une clé USB :
- l'installeur `GESCO_1.2.2_x64-setup.exe` (environ 225 Mo) ; il contient tout, même le
  composant d'affichage de Windows (WebView2), et s'installe sans connexion internet ;
- un fichier de licence, par exemple `GESCO-0001.lic`, pris dans
  `Documents\gesco-licences\lot-2026-10-16`. Notez dans `registre.csv` le numéro remis, l'école
  et la date.

Il faut Windows 10 ou 11, en 64 bits.

## Installer

1. Copiez l'installeur sur le bureau du poste, puis double-cliquez dessus.
2. Windows peut afficher « Windows a protégé votre ordinateur », parce que l'installeur n'est pas
   encore signé par un certificat d'éditeur. Cliquez sur « Informations complémentaires », puis
   sur « Exécuter quand même ».
3. Suivez l'assistant, en français. Il n'a pas besoin des droits d'administrateur. GESCO
   s'installe pour l'utilisateur Windows en cours ; un raccourci est créé dans le menu Démarrer.

## Premier démarrage

1. Saisissez le nom de l'école, ses coordonnées, puis le compte administrateur : identifiant et
   mot de passe.
2. Cochez les cycles que l'école enseigne : pour une école qui a un jardin et un 1er cycle,
   « Jardin d'enfants » et « 1er cycle ». Le jardin a ses trois sections : petite, moyenne et
   grande. Un cycle oublié s'ouvre plus tard dans « Niveaux et séries ».
3. **Recopiez la phrase de secours de douze mots** sur papier et rangez-la hors de l'école.
   Sans elle, une sauvegarde ne se restaure pas sur un autre ordinateur.
4. Créez l'année scolaire 2026-2027 dans « Année scolaire », ouvrez-la, puis les classes, les
   frais et les comptes des utilisateurs.

## Installer la licence

Outils → Licence → choisir le fichier `GESCO-0001.lic`. GESCO affiche « Licence valide pour ce
poste ». La licence s'attache à cet ordinateur, et le journal garde son numéro.

Sans licence, GESCO fonctionne de la même façon ; un bandeau rappelle seulement qu'aucune licence
n'est installée.

## Sauvegarde

Dans « Sauvegarde », ajoutez une clé USB comme destination, puis « Sauvegarder maintenant ».
GESCO sauvegarde aussi seul, à intervalles réguliers et à la fermeture.

## Mettre à jour

Une version plus récente s'installe par-dessus l'ancienne, de la même façon. Les données de
l'école restent dans `%APPDATA%\ml.gesco.poste` : la mise à jour et même une désinstallation
ne les touchent pas. GESCO met sa base à niveau au démarrage suivant, après une sauvegarde.

Une école qui avait la version 0.11.0 installe la 1.0.0 par-dessus : ses données sont gardées, sa
base reçoit la trésorerie, l'emploi du temps et la cantine, et ses rôles livrés les droits qui
vont avec. Les salaires et les avances déjà payés deviennent des dépenses « Salaires ».

Une école en 1.0.0, 1.1.0, 1.2.0 ou 1.2.1 installe la 1.2.2 par-dessus de la même façon ; ses rôles
reçoivent les nouveaux droits. Ce qui est nouveau est décrit dans `notes-de-version-1.2.0.md`
`notes-de-version-1.2.1.md` et `notes-de-version-1.2.2.md`.

## Après l'installation : la trésorerie

Dans « Trésorerie », ajoutez les comptes de l'école (banque, Orange Money…) avec leur solde au
jour de leur ouverture, et réglez le seuil au-dessus duquel une dépense attend l'accord de la
direction. Une école qui a une cantine l'active au même endroit.
