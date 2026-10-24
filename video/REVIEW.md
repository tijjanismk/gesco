# Revue de la vidéo

## Choix faits

- Le scénario, les textes et les durées sont ceux de la commande ; seuls ont été ajoutés un
  sur-titre par scène, la mention « Version 1.2.5 » à l'ouverture et le chemin du menu de
  GESCO en tête de chaque écran redessiné (« Scolarité › Élèves »…).
- Fond bleu nuit uni (`--marine` de l'application) : le premier essai, en dégradé avec des halos
  lumineux, faisait « image générée » et a été abandonné.
- Fondus : la scène qui part s'efface en 0,5 s, la suivante apparaît en 0,5 s sur le même fond.
  Un fondu enchaîné superposait les deux titres au même endroit.
- Le chronomètre de la caisse s'arrête à 0:48 sous le badge « Moins d'une minute » : c'est la
  seule mesure de temps montrée.
- Le tableau de bord ne montre que ce que le manuel lui donne (effectifs, encaissements,
  impayés, une action à faire) ; la répartition des impayés par ancienneté, qui vit dans
  Finance › Impayés, n'y figure pas.
- La 9:16 reprend les scènes 1, 3, 4, 7 et 8 ; son bulletin a cinq matières (218 points sur
  15 coefficients, soit 14,53 aussi).

## Mesuré

- `check` sans erreur sur les deux compositions : lint, exécution, mise en page, contraste
  WCAG AA (aucun écart), assertions de mouvement (logo et nom visibles dès la première image,
  logo de fin avant le numéro, cartes dans le cadre, rien de figé plus de 3 s).
- Images contrôlées à chaque scène, autour de chaque fondu et à la largeur d'un téléphone.
- Rendus (`--quality high`, 30 images/s, H.264) : 16:9 de 60,0 s (1 800 images, 4,5 Mo) et
  9:16 de 30,0 s (900 images, 2,7 Mo), mesurés avec `ffprobe` ; images décodées des deux MP4
  regardées (polices, logo et QR code présents).

## Avertissements gardés

- `film_silent` : la vidéo est muette à la demande (la compétence ne fournit pas de musique
  libre de droits sans compte HeyGen).
- `nested_structure_needs_subcomposition`, `composition_file_too_large`,
  `timeline_track_too_dense` : le linter préfère une sous-composition par scène ; la commande
  demande un seul fichier racine et une seule timeline par composition.
- `content_overlap` en info pendant les entrées : chevauchements d'un instant pendant qu'un
  élément glisse.

## Le plus faible, et ce qui viendrait ensuite

- En 9:16, le bas de l'image (sous 1 330 px) reste vide pour laisser la place aux boutons des
  applications ; sur un statut WhatsApp, l'image paraît haute.
- Le portrait de l'élève est un dessin simple ; une vraie photo (avec accord) serait plus
  chaleureuse.
