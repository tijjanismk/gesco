# GESCO 2.0.0 — notes de version (en préparation)

La version 2.0.0 n'est pas encore sortie. Ces notes décrivent ce qui est prêt ; elles seront
complétées avant la publication.

## Ce qui change

**Plusieurs ordinateurs dans la même école.** Un ordinateur, le poste principal, garde les
données ; les autres s'y relient par le réseau de l'école, sans Internet, et chacun y travaille
avec son propre compte. Le poste principal relie un ordinateur par un code affiché à l'écran, le
révoque, renouvelle la clé du réseau. Tout se règle dans Paramètres → « Postes de l'école ». Une
école à un seul ordinateur ne voit aucune différence. L'installation est décrite pas à pas dans
`installation-plusieurs-postes.md`.

**Pare-feu de Windows.** Quand la direction allume le réseau, GESCO ouvre le pare-feu de Windows
aux postes reliés et au portail des élèves, sur les réseaux privés seulement ; Windows demande son
accord. L'écran dit si
le pare-feu est resté fermé, ou si Windows range le réseau parmi les réseaux publics.

**Les cours sur les téléphones des élèves.** La direction allume « Les élèves lisent les cours
sur leurs appareils » : le poste principal sert les leçons de la bibliothèque sur le Wi-Fi de
l'école, sans Internet. GESCO affiche l'adresse à donner et imprime une affiche de classe avec son
code QR ; le secrétariat imprime, classe par classe, les billets des élèves (matricule et code).
L'élève entre avec son code, ou lit sans code, et trouve les leçons de sa classe avec formules,
figures, sons et vidéos ; il marque une leçon finie et l'enregistre en PDF pour la maison. Le
maître voit qui a ouvert et fini quelles leçons dans ses classes (Bibliothèque → « Suivi des
élèves ») ; la direction règle ou éteint ce suivi. Un nouveau réglage, « Les maîtres ont leur
compte », ouvre aux maîtres la bibliothèque et le suivi sans la saisie des notes. Quand toute une
classe lit, la gestion ne ralentit pas : au-delà de ce que le poste sert à la fois, le téléphone
demande d'attendre un instant et réessaie seul. Le module E-learning éteint ferme le portail.
Mise en place : guide d'installation, « Le portail des élèves » ; usage : manuel de la
bibliothèque, §9.

**Deux personnes sur la même fiche.** La seconde à enregistrer est prévenue, avec le nom de la
personne et l'heure ; rien n'est écrasé.

**Saisie des notes par les maîtres.** Un réglage de l'école, éteint à l'installation. Éteint, un
compte qui n'a que le rôle Enseignant n'ouvre pas de session. Allumé, le maître ne voit que les
élèves de ses classes, sans leur situation financière.

**Désinstallation.** La case « Supprimer les données de l'application » n'efface plus l'école :
sur un ordinateur qui garde les données d'une école, elle est ignorée et l'installeur le dit.

**Le jardin d'enfants a son livret.** Paramètres → « Compétences du jardin » : cinq domaines et
vingt compétences par section, que l'école modifie, et les libellés des trois niveaux. Dans
Notes, une classe du jardin s'observe au clavier (A, E, N ou 1, 2, 3, « Toute la colonne »),
avec un commentaire par domaine et une appréciation. Dans Bulletins, les livrets se préparent,
se valident, se publient et s'impriment : un livret par enfant et par trimestre, une colonne par
trimestre déjà passé, des ronds noirs lisibles en photocopie. Remise, signature et avis par SMS
comme pour un bulletin, au mot « livret ». Une maîtresse est affectée à sa classe du jardin par
« Lui confier la classe ». Le jardin reçoit ses trimestres à l'ouverture de la 2.0.0. Le
chapitre du manuel est `manuel-2.0-jardin.md`.

**La bibliothèque de cours.** GESCO arrive avec 398 leçons de la 7e, de la 8e, de la 9e et de
terminale (TSE et TSExp), et 41 sujets du DEF et du BAC avec leurs corrigés, rangés par niveau,
matière et chapitre, cherchés en quelques mots, lus en classe sur tout l'écran, imprimés en PDF
avec ou sans les corrigés. La direction retire et remet ce qu'elle veut. L'école écrit ses propres
versions, ajoute ses chapitres, ses leçons, ses sujets, ses images, ses documents et ses vidéos ;
un maître soumet, la direction publie, ou donne à un maître le droit de publier lui-même. Les
vidéos ne sont pas dans la sauvegarde : elles se copient sur une clé. Une mise à jour de la
bibliothèque s'installe depuis un fichier, sans Internet. Une école qui passe de la 1.2 à la 2.0
trouve la bibliothèque allumée. Le chapitre du manuel est `manuel-2.0-bibliotheque.md`.

**Des sons, des fiches Word et des leçons d'une autre école.** Une leçon reçoit des sons (MP3,
M4A, OGG ou WAV, 10 Mo au plus) : une consigne lue, une prononciation, un chant ; ils se lisent à
la demande, en grand en mode classe, sur tous les postes de l'école, et la sauvegarde les garde.
Une fiche Word (`.docx`) devient un brouillon avec ses titres, ses listes, ses tableaux et ses
images ; une formule de Word est signalée à l'auteur, à retaper. Un chapitre ou une leçon que
l'école ajoute peut être réservé à certaines classes (« 9e A seulement »). Deux écoles
s'échangent les leçons qu'elles ont écrites par un fichier passé sur une clé ou par WhatsApp :
elles arrivent en brouillons, rangées sous les matières de l'école, et ne sont visibles des élèves
qu'une fois publiées ; le même fichier importé deux fois n'ajoute rien, un fichier abîmé est
refusé entier.

**Une copie hors de l'école.** Outils → Sauvegarde, section « Copie hors de l'école » : GESCO
propose les dossiers Google Drive, OneDrive et Dropbox qu'il trouve sur le poste principal, ou
tout autre dossier, et y dépose une copie de la sauvegarde, déjà chiffrée, que le logiciel du
fournisseur envoie sur le compte de l'école. À la demande, chaque semaine ou chaque mois, jamais
chaque jour ; « Envoyer une copie maintenant » dit avant combien de mégaoctets partiront. Seize
copies au plus restent sur le compte. Ni la base en clair, ni la clé, ni les douze mots ne
partent. GESCO vérifie, sans rien bloquer, que la dernière copie est arrivée, et dit quand
Windows le sait si elle est envoyée ; une ligne du bandeau prévient si elle n'est pas arrivée.
Les destinations ont désormais une sorte (dossier, support amovible, dossier synchronisé), et
l'historique la montre ; un réglage de la 1.2 garde son comportement. Le chapitre du manuel est
`manuel-2.0-sauvegarde-hors-ecole.md`, la mise en place est dans le guide d'installation.

## Mettre à jour

Installez la 2.0.0 par-dessus la version en place : les données sont gardées. Dans une école à
plusieurs ordinateurs, mettez à jour le poste principal d'abord, puis chaque poste relié : deux
versions différentes ne travaillent pas ensemble.
