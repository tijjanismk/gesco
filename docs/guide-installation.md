# GESCO — installer sur un ordinateur

Pour la version 1.2.7, du 10 novembre 2026. Ce guide va de l'ordinateur vide à GESCO prêt à
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
`GESCO_1.2.7_x64-setup.exe` (environ 225 Mo). Prenez aussi le manuel
`GESCO-manuel-utilisation.pdf`. Copiez-les sur une clé USB pour les porter à l'école.

**Vérifier le fichier (conseillé).** Pour être sûr que le fichier n'a pas été abîmé ni
remplacé, ouvrez PowerShell dans le dossier du fichier et tapez :

`Get-FileHash .\GESCO_1.2.7_x64-setup.exe`

Le résultat doit être exactement l'empreinte SHA-256 indiquée sur la page de la version.
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
   (identifiant et mot de passe d'au moins 6 caractères). Notez-les.
2. Cochez les **cycles** que l'école enseigne : jardin d'enfants, 1er cycle, 2e cycle, lycée,
   enseignement technique. Un cycle oublié s'ouvre plus tard dans Paramètres → Niveaux et
   séries.
3. GESCO affiche une **phrase de secours de douze mots**. **Recopiez-la sur papier**, vérifiez-la,
   et rangez-la hors de l'école (chez le promoteur, par exemple). Sans elle, une sauvegarde ne se
   restaure pas sur un autre ordinateur. Personne, pas même l'éditeur, ne peut la retrouver.
4. Imprimez le certificat affiché sous les douze mots, puis **cochez la case « J'ai imprimé le
   certificat et je l'ai rangé hors du poste »** : le bouton « Continuer » reste grisé tant
   qu'elle n'est pas cochée. Le message « certificat non confirmé » du bandeau disparaît alors.
   Celui de la licence disparaît quand la licence est installée (§5), celui de la sauvegarde
   quand une clé USB est ajoutée comme destination (§6).
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

### Une copie hors de l'école (version 2.0, facultatif)

La clé ne protège pas l'école si l'ordinateur et la clé disparaissent ensemble (vol, incendie,
inondation). GESCO peut aussi déposer une copie chiffrée de la sauvegarde dans un dossier que
Google Drive, OneDrive ou Dropbox envoie sur un compte en ligne. Les comptes gratuits suffisent :
une copie pèse quelques mégaoctets, et GESCO en garde 16 au plus.

1. **Le compte.** Créez un compte **au nom de l'école**, pas à celui d'un employé qui peut
   partir ; le promoteur en garde l'accès. Google (le plus courant, celui des téléphones
   Android), Microsoft ou Dropbox : GESCO marche de la même façon avec les trois, et avec tout
   autre logiciel qui synchronise un dossier.
2. **Le logiciel du fournisseur**, sur le poste principal, dans la session Windows où GESCO est
   utilisé :
   - *Google Drive* : installez « Google Drive pour ordinateur » depuis le site de Google, puis
     connectez-vous ; un lecteur (souvent `G:`) apparaît, avec le dossier « Mon Drive ».
   - *OneDrive* : il est déjà dans Windows 10 et 11 ; ouvrez-le et connectez-vous avec le compte
     de l'école.
   - *Dropbox* : installez l'application depuis le site de Dropbox, puis connectez-vous.
3. **Dans GESCO**, Outils → Sauvegarde, section « Copie hors de l'école » : cliquez sur
   **Choisir** à côté du dossier proposé (par exemple « Google Drive — G:\Mon Drive\GESCO »). Si
   rien n'est proposé, **Choisir un autre dossier…** et désignez le dossier du fournisseur.
   Choisissez le rythme : à la demande, chaque semaine ou chaque mois.
4. **Envoyez une copie** : « Envoyer une copie maintenant ».
5. **Vérifiez sur le site du fournisseur**, depuis un téléphone ou un autre ordinateur, que le
   dossier `GESCO` contient un fichier `gesco-….gsv` daté du jour. GESCO affiche aussi l'état de
   la dernière copie ; selon le fournisseur, il reste « Déposée dans le dossier », sans que ce
   soit une erreur.

À savoir :
- **Le forfait Internet.** Chaque copie part par Internet ; GESCO dit avant combien de
  mégaoctets. OneDrive suspend l'envoi sur une connexion « limitée » ; un partage de connexion du
  téléphone l'est souvent. La copie part alors au retour d'une connexion ordinaire.
- **La corbeille du fournisseur** garde quelques jours les copies que GESCO retire.
- **Plusieurs comptes Windows sur le poste principal** : la copie n'est déposée que lorsque GESCO
  tourne sous le compte où le logiciel du fournisseur est connecté.
- **Ce que le fournisseur voit** : le nom du fichier, sa taille, le nom de l'école et la version
  de GESCO. Le reste est chiffré ; sans les douze mots, personne ne le lit.
- **Ne rangez jamais le certificat des douze mots**, ni sa photo, sur ce compte.
- **Restaurer** depuis le compte : téléchargez la dernière copie `.gsv` sur le site du
  fournisseur, puis suivez le §10 en désignant ce fichier.

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

### Le portail des élèves (version 2.0, facultatif)

Les élèves lisent les cours de la bibliothèque sur leur téléphone ou sur un ordinateur de la salle
informatique, par le Wi-Fi de l'école, sans Internet. Le poste principal les sert. À préparer une
fois, avec la personne qui s'occupe du routeur :

1. **Le réseau de l'école d'abord** : Paramètres → « Postes de l'école » → allumer le réseau
   (`installation-plusieurs-postes.md`, §4). Windows demande alors d'ouvrir son pare-feu :
   répondez « Oui ». La règle « GESCO postes relies » ouvre le port 47800 des postes reliés et
   les ports 80 et 47801 du portail, sur les réseaux privés seulement. Le portail ne s'ouvre
   jamais sur un poste relié : seulement sur le poste principal.
2. **Une adresse qui ne change pas** : dans le routeur, réservez l'adresse du poste principal
   (« réservation DHCP » ou « adresse IP fixe », par l'adresse physique de sa carte Wi-Fi ou
   réseau). Sans cela, le routeur peut lui donner une autre adresse un matin, et l'affiche de la
   classe ne mène plus nulle part ; GESCO le dit alors sous l'adresse, et l'affiche se réimprime.
3. **Le Wi-Fi des élèves** : si le routeur le permet, donnez aux élèves un réseau Wi-Fi à part,
   avec son propre mot de passe, **sur le même réseau** que le poste principal. Attention au
   « réseau invité » de beaucoup de routeurs : il isole souvent les appareils du reste du
   réseau, et le portail ne répond pas. Dans ce cas, un second nom de Wi-Fi sur le réseau
   principal, ou un point d'accès branché au routeur, fait l'affaire.
4. **Allumer le portail** : Paramètres → section « Portail des élèves » → « Les élèves lisent les
   cours sur leurs appareils ». L'adresse à donner s'affiche en grand, par exemple
   `http://192.168.1.20`. GESCO prend le port 80 s'il est libre ; si un autre programme le tient
   (un serveur web, certains logiciels de visioconférence), il prend le 47801 et l'adresse le
   dit (`http://192.168.1.20:47801`) : rien ne bloque. Si la ligne « Le pare-feu de Windows ne
   laisse pas encore entrer les appareils des élèves » paraît, cliquez sur « Ouvrir le pare-feu
   de Windows » et répondez « Oui ».
5. **L'affiche et les billets** : « Imprimer l'affiche de classe » sous l'adresse (l'adresse en
   grand et son code QR) ; puis, classe par classe, Scolarité → Classes → la classe →
   « Billets des élèves sans code » : dix billets par page, avec le matricule et le code de
   chaque élève. Le code n'est écrit nulle part ailleurs : gardez les billets comme des
   bulletins jusqu'à leur remise.

**Combien d'élèves à la fois.** Un routeur ordinaire d'école tient une trentaine d'appareils qui
lisent en même temps ; au-delà, prévoyez un second point d'accès. GESCO sert lui-même une
classe entière sans ralentir le secrétariat : quand trop d'élèves demandent au même instant,
le téléphone affiche « Beaucoup d'élèves sont connectés. Réessaie dans un instant. » et réessaie
seul toutes les cinq secondes. Les vidéos sont les plus lourdes : la direction peut les garder
pour la classe (réglage « Les vidéos se regardent aussi sur les appareils des élèves »).

**Ce que voit un téléphone sans Internet.** Le Wi-Fi de l'école n'a pas forcément Internet :
Android affiche alors « Connecté, sans Internet » et peut proposer de changer de réseau ; il faut
choisir de **rester connecté** à ce Wi-Fi. Si les données mobiles sont allumées, certains
téléphones envoient tout par elles : l'adresse de l'école ne répond pas ; coupez les données
mobiles le temps de lire. Rien de la page ne vient d'Internet : la police, les formules, les
images, les sons et les vidéos viennent du poste principal. L'adresse commence par `http://` et
non `https://` : c'est voulu sur le Wi-Fi de l'école ; le navigateur peut écrire « Non
sécurisé », la lecture se fait quand même. L'élève peut ajouter la page à l'écran d'accueil de
son téléphone (menu du navigateur → « Ajouter à l'écran d'accueil »).

**Désinstaller.** La règle « GESCO postes relies » reste dans le pare-feu, sans effet une fois
GESCO retiré (`installation-plusieurs-postes.md`, §8, pour l'enlever).

## 8. Mettre à jour une école déjà équipée

Téléchargez la nouvelle version et installez-la **par-dessus** l'ancienne, de la même façon. Les
données de l'école sont gardées : au démarrage suivant, GESCO fait d'abord une sauvegarde, puis
met sa base à niveau. Ne désinstallez pas l'ancienne version avant.

La version 2.0 apporte la bibliothèque de cours : l'installeur la dépose, GESCO la copie dans le
dossier des données au démarrage et l'installe à l'ouverture de l'école. Une version plus
récente de la bibliothèque, livrée par une mise à jour de GESCO, s'installe de la même façon ;
une mise à jour reçue à part s'installe depuis Paramètres → « Bibliothèque installée ».

## 9. Désinstaller

Paramètres de Windows → Applications → GESCO → Désinstaller. Les données de l'école ne sont pas
effacées : elles restent dans le dossier `%APPDATA%\ml.gesco.poste`, avec la bibliothèque de
cours (`bibliotheque\`) et les vidéos de l'école (`videos\`). Avant de changer d'ordinateur,
faites une dernière sauvegarde sur la clé USB, et copiez les vidéos sur une clé (Paramètres →
« Vidéos de l'école ») : elles ne sont pas dans la sauvegarde.

La fenêtre de désinstallation porte une case « Supprimer les données de l'application ».
**Jusqu'à la version 1.2.7, ne la cochez jamais** : elle efface l'école. À partir de la
version 2.0, elle est sans effet sur un ordinateur qui garde les données d'une école, et
l'installeur le dit.

Pour plusieurs ordinateurs dans la même école (version 2.0), voir
`installation-plusieurs-postes.md`.

## 10. Changer d'ordinateur, ou après une panne

1. Installez GESCO sur le nouvel ordinateur (§3).
2. Au premier écran, choisissez de **restaurer une sauvegarde**, branchez la clé USB et
   choisissez la dernière sauvegarde.
3. Saisissez la **phrase de secours** de douze mots.
4. Réinstallez la licence : elle s'attache au nouvel ordinateur (demandez-la à l'éditeur si
   besoin).
5. Reprenez les vidéos de l'école depuis la clé : Paramètres → « Vidéos de l'école » →
   « Reprendre des vidéos depuis une clé ».

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
- **L'administrateur a oublié le sien** : un autre administrateur le réinitialise (d'où le second
  compte conseillé au §4). Sinon, sur l'écran de connexion, cliquez sur « Mot de passe
  administrateur oublié ? » et saisissez les douze mots du certificat : un nouveau mot de passe
  se choisit aussitôt. Gardez donc le certificat en lieu sûr.
- **L'écran reste blanc au démarrage** : redémarrez l'ordinateur, puis relancez GESCO. Si cela
  continue, réinstallez la même version par-dessus : les données sont gardées.
