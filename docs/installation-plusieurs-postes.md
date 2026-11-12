# GESCO — installer sur plusieurs ordinateurs

Pour la version 2.0.0. Ce guide complète `guide-installation.md`, qui installe GESCO sur un
ordinateur. Il va d'une école équipée d'un poste à une école où plusieurs personnes travaillent
en même temps, chacune sur son ordinateur. Comptez une heure pour le premier poste relié, un
quart d'heure pour chacun des suivants. Rien ne passe par Internet.

---

## 1. Comment cela marche

- **Un seul ordinateur garde les données de l'école : le poste principal.** C'est lui qui a la
  base, les sauvegardes, la licence et la phrase de secours.
- Les autres ordinateurs sont des **postes reliés**. Ils n'ont aucune donnée : chaque geste part
  au poste principal par le réseau de l'école et la réponse revient à l'écran.
- **Si le poste principal est éteint, ou si GESCO y est fermé, les postes reliés ne travaillent
  pas.** Ils le disent dans leur bandeau, gardent ce qui est saisi à l'écran et reprennent seuls
  quand le poste principal revient.
- Une école qui n'a qu'un ordinateur ne change rien : le réseau reste éteint tant qu'elle ne
  l'allume pas.

## 2. Ce qu'il faut

- **Le poste principal** : de préférence un **ordinateur portable**. Sa batterie tient pendant
  une coupure de courant, et il se range sous clé le soir. Il reste allumé, GESCO ouvert, tant
  que les autres travaillent.
- **Un petit routeur Wi-Fi** (ou la box de l'école), branché sur une prise protégée si possible.
  Tous les ordinateurs se branchent sur ce même réseau, en Wi-Fi ou par câble. Le routeur n'a
  pas besoin d'Internet.
- **Windows 10 ou 11, en 64 bits**, sur chaque ordinateur, et le **même installeur** partout :
  `GESCO_2.0.0_x64-setup.exe`. Deux versions différentes ne travaillent pas ensemble.
- **Une licence qui couvre le nombre de postes.** Sans licence, ou avec une licence qui n'en dit
  rien, un seul poste relié peut travailler, le temps de l'essai.
- **Le mot de passe d'un administrateur de Windows** sur le poste principal, une seule fois,
  pour ouvrir le pare-feu (§4).

## 3. Préparer le poste principal

1. Installez GESCO et créez l'école comme le décrit `guide-installation.md` (§3 à §7), ou gardez
   l'ordinateur où l'école travaille déjà : il devient le poste principal, avec ses données.
2. **Donnez une adresse fixe au poste principal.** Les postes reliés retiennent son adresse ; si
   le routeur lui en donne une autre un matin, ils ne le trouvent plus. Dans la page de réglage
   du routeur, réservez l'adresse de cet ordinateur (« réservation d'adresse », « bail
   statique », « DHCP statique » selon les marques). À défaut, voir le §9 « L'adresse du poste
   principal a changé ».
3. **Déclarez le réseau de l'école comme réseau privé dans Windows.** Paramètres de Windows →
   Réseau et Internet → cliquez sur le réseau de l'école (Wi-Fi ou Ethernet) → « Type de profil
   réseau » → **Réseau privé**. Windows range d'office un nouveau réseau parmi les réseaux
   publics, où son pare-feu refuse les autres ordinateurs. Ne faites ce réglage que pour le
   réseau de l'école, jamais pour celui d'un lieu public.

## 4. Allumer le réseau de l'école

Sur le poste principal, avec un compte de la direction (Directeur ou Administrateur) :

1. **Paramètres → Postes de l'école**. L'écran dit : « Le réseau est éteint : GESCO ne sert que
   cet ordinateur. »
2. Cliquez sur **« Activer le réseau »**.
3. Windows affiche sa fenêtre **« Voulez-vous autoriser cette application à apporter des
   modifications à votre appareil ? »**, au nom de Windows PowerShell. Répondez **« Oui »**. Si
   le compte Windows n'est pas administrateur, Windows demande d'abord le mot de passe d'un
   administrateur. Cette fenêtre peut mettre quelques secondes à paraître, et peut rester
   derrière GESCO : regardez la barre des tâches si rien ne se passe.
4. GESCO pose alors une règle dans le pare-feu de Windows, nommée **« GESCO postes relies »** :
   elle laisse entrer les autres ordinateurs vers GESCO seulement, sur le port 47800, et
   seulement sur les réseaux privés. L'écran dit : « Le pare-feu de Windows laisse entrer les
   postes reliés, sur les réseaux privés seulement. »
5. L'écran montre maintenant ce qu'il faut pour relier un poste : le **nom de ce poste**,
   l'**adresse à donner aux postes** (par exemple 192.168.1.20) et l'**empreinte de ce poste**
   (quatre groupes de quatre caractères).

**Si vous avez répondu « Non »**, ou fermé la fenêtre de Windows : le réseau est allumé, mais
l'écran affiche « Le pare-feu de Windows n'est pas ouvert à GESCO » avec le bouton **« Ouvrir le
pare-feu de Windows »**. Cliquez dessus et répondez « Oui ».

**Si Windows affiche sa propre fenêtre « Le Pare-feu Windows Defender a bloqué certaines
fonctionnalités de cette application »** : cochez « Réseaux privés » seulement, puis
« Autoriser l'accès ».

**Si l'écran dit que Windows range le réseau parmi les réseaux publics** : faites le réglage du
§3, étape 3, puis cliquez sur « Vérifier de nouveau ».

## 5. Relier un ordinateur

Sur l'ordinateur à relier :

1. Installez GESCO avec le même installeur (`guide-installation.md`, §3). **Ne créez pas
   d'école.**
2. Au premier écran, choisissez **« Rejoindre le poste principal de l'école »**.
3. Saisissez l'**adresse du poste principal**, telle que l'affiche son écran « Postes de
   l'école », puis cliquez sur **« Chercher le poste principal »**.
4. GESCO affiche l'empreinte du poste trouvé. **Comparez-la** à celle de l'écran du poste
   principal : elles doivent être identiques. Si elles diffèrent, arrêtez : ce n'est pas le
   poste principal de l'école.

Sur le poste principal :

5. Dans « Postes de l'école », sous « Relier un poste », cliquez sur **« Afficher un code »**.
   Le code (huit caractères) vaut dix minutes, pour un seul poste.

De retour sur l'ordinateur à relier :

6. Donnez un **nom au poste** (« Caisse », « Secrétariat »…) : c'est lui qui signera les
   écritures dans le journal. Saisissez le code, puis cliquez sur **« L'empreinte est la même :
   relier ce poste »**.
7. L'écran de connexion de l'école paraît, sous un bandeau qui porte le nom du poste. Chacun s'y
   connecte avec son propre compte GESCO, créé sur le poste principal (Paramètres →
   Utilisateurs).

Le poste relié paraît aussitôt dans la liste « Postes reliés » du poste principal.

## 6. Au quotidien

- **Le matin** : allumez d'abord le poste principal et ouvrez GESCO ; les autres ensuite.
- **Le soir** : fermez GESCO sur les postes reliés, puis sur le poste principal. Si quelqu'un a
  encore une session ouverte, le poste principal le dit avant de se fermer.
- **Les sauvegardes** se font sur le poste principal seulement, comme avant : la clé USB y reste
  branchée.
- **Certaines opérations demandent d'être seul** (restaurer une sauvegarde, clôturer l'année,
  certains imports) : le poste principal montre les postes connectés avant de les lancer, et les
  autres attendent.
- **Deux personnes sur la même fiche** : la seconde à enregistrer est prévenue que la fiche a
  été modifiée entre-temps, par qui et à quelle heure. Rien n'est écrasé ; elle relit la fiche et
  refait sa correction.
- **Les maîtres ne se connectent pas** tant que la direction n'a pas allumé « Les maîtres
  saisissent leurs notes » dans Paramètres.

## 7. Mettre à jour

**Le poste principal d'abord, puis chaque poste relié**, avec le même installeur, par-dessus
l'ancienne version. Tant qu'un poste relié n'a pas la même version que le poste principal, il
est refusé, avec un message qui donne la version attendue. Les postes restent reliés après la
mise à jour : rien n'est à refaire.

## 8. Retirer un poste, changer de poste principal

- **Un ordinateur relié est perdu, volé ou cédé** : sur le poste principal, « Postes de
  l'école » → « Révoquer » sur sa ligne. S'il a pu tomber en de mauvaises mains, cliquez aussi
  sur « Renouveler la clé du réseau » : tous les postes devront alors être reliés de nouveau.
- **Un poste relié doit redevenir un ordinateur sans GESCO, ou rejoindre un autre poste
  principal** : sur son écran de connexion, « Quitter le poste principal ». Révoquez-le aussi
  sur le poste principal, pour libérer sa place.
- **Changer de poste principal** (panne, ordinateur neuf) : installez GESCO sur le nouvel
  ordinateur et restaurez la dernière sauvegarde (`guide-installation.md`, §10). Le réseau y
  revient allumé, avec les mêmes postes reliés. Ouvrez « Postes de l'école » : le pare-feu de ce
  nouvel ordinateur n'est pas encore ouvert, cliquez sur « Ouvrir le pare-feu de Windows ».
  Donnez au nouvel ordinateur la même adresse que l'ancien (§3, étape 2) ; sinon, voir le §9.
- **Désinstaller GESCO du poste principal** : les données de l'école restent dans
  `%APPDATA%\ml.gesco.poste`, même si la case « Supprimer les données de l'application » est
  cochée ; l'installeur le dit. La règle « GESCO postes relies » reste dans le pare-feu, sans
  effet une fois GESCO retiré. Pour l'enlever : Pare-feu Windows Defender → Paramètres avancés →
  Règles de trafic entrant → « GESCO postes relies » → Supprimer.
- **Éteindre le réseau** (« Éteindre le réseau ») : les postes reliés restent enregistrés et
  retravaillent dès qu'il est rallumé. La règle du pare-feu reste en place.

## 9. En cas de souci

- **Le poste relié dit « poste principal injoignable »**. Dans l'ordre : le poste principal
  est-il allumé, GESCO ouvert, le réseau allumé dans « Postes de l'école » ? Les deux ordinateurs
  sont-ils sur le même réseau (le même routeur) ? L'écran « Postes de l'école » dit-il que le
  pare-feu est ouvert, et ne signale-t-il pas un réseau public (§3, étape 3) ? L'adresse du poste
  principal est-elle toujours celle que le poste relié a retenue (ci-dessous) ?
- **L'adresse du poste principal a changé** : l'écran « Postes de l'école » montre la nouvelle.
  Sur chaque poste relié : « Quitter le poste principal », puis le relier de nouveau (§5) avec
  la nouvelle adresse ; sur le poste principal, révoquez l'ancienne ligne du poste. Pour que
  cela n'arrive plus, réservez l'adresse dans le routeur (§3, étape 2).
- **« Version différente »** : installez sur ce poste le même installeur que sur le poste
  principal (§7).
- **« La licence ne couvre pas un poste de plus »** : révoquez un poste qui ne sert plus, ou
  demandez à l'éditeur une licence pour davantage de postes.
- **L'empreinte a changé** sur un poste relié : la clé du réseau a été renouvelée, ou ce n'est
  pas le bon poste principal. Vérifiez l'empreinte sur l'écran du poste principal, puis reliez
  le poste de nouveau.
- **Le réseau est allumé mais aucun poste ne peut se relier, sans message** : un autre programme
  occupe peut-être le port 47800 sur le poste principal. Le journal technique
  (`%APPDATA%\ml.gesco.poste\journal-technique.txt`) le dit ; envoyez-le à l'assistance.
- **Un antivirus avec son propre pare-feu** (autre que celui de Windows) peut refuser les postes
  reliés malgré la règle de Windows : autorisez-y GESCO sur le port 47800.
- **Le journal technique** existe sur chaque poste, au même endroit. Celui d'un poste relié note
  chaque coupure et chaque reprise de la liaison ; celui du poste principal note l'ouverture du
  pare-feu et les appareils non reconnus.
