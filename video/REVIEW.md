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
- Voix off ajoutée après coup (la commande disait « pas de voix ») : une phrase par scène, qui
  dit ce que montre l'écran, sans rien ajouter au manuel. Sans compte HeyGen dans ce conteneur,
  c'est la seule voix française locale de Kokoro (« Siwis », féminine, accent de France), à
  vitesse 0,92 en 16:9 et 0,95 en 9:16 ; la phrase de fin, à 1,0, laisse tomber « GESCO » pour
  tenir dans ses 5 secondes. [HYPOTHÈSE] « GESCO » se dit « jèsko », comme le « ges » de
  « gestion ».
- Le numéro est écrit en toutes lettres dans le texte de la voix, pour être lu par paires.

## Mesuré

- `check` sans erreur sur les deux compositions : lint, exécution, mise en page, contraste
  WCAG AA (aucun écart), assertions de mouvement (logo et nom visibles dès la première image,
  logo de fin avant le numéro, cartes dans le cadre, rien de figé plus de 3 s).
- Images contrôlées à chaque scène, autour de chaque fondu et à la largeur d'un téléphone.
- Voix off : phonèmes envoyés à Kokoro relus pour chaque phrase (liaisons, « d'œil », numéro) ;
  chaque phrase entre -16,3 et -17,9 LUFS, crête sous -1,8 dBFS ; chacune finit avant le fondu
  de sortie de sa scène. Elle a été mesurée, pas écoutée : la transcription automatique n'a pas
  pu tourner (modèles Whisper inaccessibles depuis ce conteneur).
- Rendus (`--quality high`, 30 images/s, H.264 et AAC) : 16:9 de 60,0 s (1 800 images, 5,5 Mo)
  et 9:16 de 30,0 s (900 images, 3,1 Mo), mesurés avec `ffprobe` ; images décodées des deux MP4
  regardées (polices, logo et QR code présents). Son des MP4 : -14,0 et -14,1 LUFS, crête
  -1,8 et -1,9 dBFS ; les plages de voix détectées commencent aux instants prévus et finissent
  avant chaque fondu.

## Avertissements gardés

- `nested_media_start_basis_ambiguous` (9:16) : le linter prend `compositions/vertical.html`
  pour une sous-composition ; ses temps de voix sont locaux à la composition, ce qui est juste
  quand on la rend seule.
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
- La voix de synthèse s'entend comme telle, avec un accent de France ; une voix enregistrée
  au Mali, découpée phrase par phrase dans `assets/voix/`, serait plus proche du public.
- Pas de musique : la compétence n'en fournit pas de libre de droits sans compte HeyGen.
