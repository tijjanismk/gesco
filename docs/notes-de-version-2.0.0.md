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
aux postes reliés, sur les réseaux privés seulement ; Windows demande son accord. L'écran dit si
le pare-feu est resté fermé, ou si Windows range le réseau parmi les réseaux publics.

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

## Mettre à jour

Installez la 2.0.0 par-dessus la version en place : les données sont gardées. Dans une école à
plusieurs ordinateurs, mettez à jour le poste principal d'abord, puis chaque poste relié : deux
versions différentes ne travaillent pas ensemble.
