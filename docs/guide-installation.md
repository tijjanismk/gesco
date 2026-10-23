# GESCO — installer sur un ordinateur

Pour la version 1.2.2, du 23 octobre 2026. Ce guide va de l'ordinateur vide à GESCO prêt à
servir. Comptez une demi-heure, sans internet une fois l'installeur copié.

---

## 1. Ce qu'il faut

- Un ordinateur sous **Windows 10 ou Windows 11, en 64 bits**. Pour le savoir : Paramètres →
  Système → Informations système ; la ligne « Type du système » doit dire « 64 bits ».
- **4 Go de mémoire** au moins, et **2 Go libres** sur le disque.
- **Une clé USB** pour les sauvegardes, réservée à GESCO.
- Le fichier de licence remis par l'éditeur (par exemple `GESCO-0001.lic`), si l'école en a
  un. GESCO fonctionne aussi sans, le temps de l'essai.
- Pour les SMS aux familles, plus tard : une clé modem GSM USB et une puce prépayée de l'école.

## 2. Récupérer l'installeur

Sur un ordinateur qui a internet, ouvrez la page de la version :
**https://github.com/tijjanismk/gesco/releases/latest**, puis cliquez sur
`GESCO_1.2.2_x64-setup.exe` (environ 225 Mo). Prenez aussi le manuel
`GESCO-manuel-utilisation.pdf`. Copiez-les sur une clé USB pour les porter à l'école.

**Vérifier le fichier (conseillé).** Pour être sûr que le fichier n'a pas été abîmé ni
remplacé, ouvrez PowerShell dans le dossier du fichier et tapez :

`Get-FileHash .\GESCO_1.2.2_x64-setup.exe`

Le résultat doit être exactement :
`F72A884063AD9D9F9A9F573AF121521C31EF439DF41915C8237224E087F9C06B`.
S'il diffère, ne l'installez pas : téléchargez-le de nouveau.

## 3. Installer

1. Copiez l'installeur sur le bureau de l'ordinateur de l'école, puis double-cliquez dessus.
2. Windows peut afficher **« Windows a protégé votre ordinateur »** : l'installeur n'est pas
   encore signé par un certificat d'éditeur. Cliquez sur **« Informations complémentaires »**,
   puis sur **« Exécuter quand même »**.
3. Suivez l'assistant, en français. Il ne demande pas les droits d'administrateur : GESCO
   s'installe pour l'utilisateur Windows qui est connecté. Tout ce qu'il faut est dans
   l'installeur, même le composant d'affichage de Windows (WebView2) : pas besoin d'internet.
4. À la fin, GESCO a un raccourci dans le menu Démarrer. Épinglez-le à la barre des tâches.

Installez GESCO sous le compte Windows de la personne qui s'en servira tous les jours : chaque
compte Windows a ses propres données GESCO.

## 4. Premier démarrage

1. Saisissez le **nom de l'école**, ses coordonnées, puis le **compte administrateur**
   (identifiant et mot de passe d'au moins 8 caractères). Notez-les.
2. Cochez les **cycles** que l'école enseigne : jardin d'enfants, 1er cycle, 2e cycle, lycée,
   enseignement technique. Un cycle oublié s'ouvre plus tard dans Paramètres → Niveaux et
   séries.
3. GESCO affiche une **phrase de secours de douze mots**. **Recopiez-la sur papier**, vérifiez-la,
   et rangez-la hors de l'école (chez le promoteur, par exemple). Sans elle, une sauvegarde ne se
   restaure pas sur un autre ordinateur. Personne, pas même l'éditeur, ne peut la retrouver.
4. Imprimez le certificat affiché sous les douze mots, puis **cochez la case « J'ai imprimé le
   certificat et je l'ai rangé hors du poste »** : le bouton « Continuer » reste grisé tant
   qu'elle n'est pas cochée.
5. Créez tout de suite un **second compte administrateur** (Paramètres → Utilisateurs), pour
   le directeur par exemple : seul un administrateur peut réinitialiser le mot de passe d'un
   autre administrateur.

## 5. Installer la licence

**Outils → Licence**, puis choisissez le fichier `.lic` remis par l'éditeur. GESCO affiche
« Licence valide pour ce poste ». La licence s'attache à cet ordinateur. Sans licence, GESCO
fonctionne de la même façon ; un bandeau rappelle seulement qu'aucune licence n'est installée.

## 6. Mettre en place la sauvegarde

**Outils → Sauvegarde** : branchez la clé USB, ajoutez-la comme destination, puis cliquez sur
« Sauvegarder maintenant ». Ensuite, GESCO sauvegarde seul, à intervalles réguliers et à la
fermeture. Laissez la clé branchée, ou rebranchez-la chaque soir ; ne la prêtez pas.

## 7. Préparer l'école

Dans cet ordre (le manuel d'utilisation détaille chaque écran) :

1. **Paramètres → Établissement** : la fiche de l'école, son logo, et en tête des réglages les
   **modules de l'école**. Allumez ceux dont l'école a besoin : messages aux familles par SMS,
   cantine… Les autres restent éteints et n'encombrent pas le menu.
2. **Paramètres → Année scolaire** : créez l'année 2026-2027 et ouvrez-la.
3. **Paramètres → Classes**, **Frais de l'année**, **Matières et notation** (le programme se
   corrige aussi par un fichier Excel, en bas de Paramètres → Établissement).
4. **Paramètres → Utilisateurs** : un compte par personne (secrétaire, caissier,
   enseignants…), chacun avec son rôle.
5. **Finance → Trésorerie** : les comptes de l'école (banque, Orange Money…) avec leur solde de
   départ.

## 8. Mettre à jour une école déjà équipée

Téléchargez la nouvelle version et installez-la **par-dessus** l'ancienne, de la même façon. Les
données de l'école sont gardées : au démarrage suivant, GESCO fait d'abord une sauvegarde, puis
met sa base à niveau. Ne désinstallez pas l'ancienne version avant.

## 9. Désinstaller

Paramètres de Windows → Applications → GESCO → Désinstaller. Les données de l'école ne sont pas
effacées : elles restent dans le dossier `%APPDATA%\ml.gesco.poste`. Avant de changer
d'ordinateur, faites une dernière sauvegarde sur la clé USB.

## 10. Changer d'ordinateur, ou après une panne

1. Installez GESCO sur le nouvel ordinateur (§3).
2. Au premier écran, choisissez de **restaurer une sauvegarde**, branchez la clé USB et
   choisissez la dernière sauvegarde.
3. Saisissez la **phrase de secours** de douze mots.
4. Réinstallez la licence : elle s'attache au nouvel ordinateur (demandez-la à l'éditeur si
   besoin).

## 11. En cas de souci

- **Je clique et rien ne se passe** : regardez s'il y a une case à cocher au-dessus du bouton
  (écran des douze mots), ou un message en rouge sous le formulaire. La création de
  l'établissement prend quelques secondes ; au-delà de vingt, GESCO l'écrit sous le bouton.
  Laissez-le ouvert, puis envoyez le journal technique si rien ne change.
- **Le journal technique** : chaque erreur est notée, avec l'heure, dans le fichier
  `journal-technique.txt` du dossier `%APPDATA%\ml.gesco.poste` (tapez ce chemin dans la barre
  d'adresse de l'Explorateur). Le lancement et chaque étape de la création de l'établissement
  y sont notés aussi. C'est ce fichier qu'on envoie à l'assistance ; il ne contient ni les
  données de l'école ni les mots de passe.

- **« Windows a protégé votre ordinateur »** : voir §3, étape 2.
- **GESCO demande la phrase de secours au démarrage** : la clé des données n'est plus lisible
  sur ce compte Windows (Windows réinstallé, autre compte). Saisissez la phrase recopiée au
  premier démarrage.
- **Un mot de passe oublié** : l'administrateur le réinitialise dans Paramètres → Utilisateurs.
- **L'administrateur a oublié le sien** : seul un autre administrateur peut le réinitialiser.
  D'où le second compte administrateur conseillé au §4.
- **L'écran reste blanc au démarrage** : redémarrez l'ordinateur, puis relancez GESCO. Si cela
  continue, réinstallez la même version par-dessus : les données sont gardées.
