# Les tables de la bibliothèque : le programme comme colonne, un contenu et ses versions, l'état de l'école à part

**Date** : 15 novembre 2026
**Statut** : proposé le 15 novembre 2026 — décision 6 de `docs/plan-phase-12.md` §5, lot 1 de la phase 12 (`SPEC_PHASE_12_LOT_1.md`)

## Contexte

Le blueprint (§16.1) prévoyait d'un coup : cours, chapitre, leçon, média adressé par son
contenu, quiz, question, réponse, devoir en ligne, soumission, progression, programme officiel
et objectifs, avec un champ `langue` partout. Le lot 1 de la phase 12 n'a besoin que d'une
partie : ranger la bibliothèque livrée sous le programme malien, la lire, la retirer, y ajouter,
améliorer une leçon livrée sans la toucher, faire valider avant que les élèves voient, revenir à
la version livrée, imprimer.

Ce que les décisions imposent au modèle :
- **Amélioration des cours** (porteur, 14 novembre 2026) : la leçon livrée reste intacte ;
  l'école en fait sa version ; chaque modification est tracée ; retour possible à la version
  livrée ; une mise à jour de la bibliothèque ne touche jamais la version de l'école.
- **T-89** (14 novembre 2026) : l'enseignant de la matière modifie, la direction valide avant
  que les élèves voient ; la direction peut donner nommément à un maître le droit de publier.
- **Rien sans chapitre** (E-1) ; chaque contenu dit ce qu'il est (E-7) ; relu par l'école, et
  cela se voit (E-5).
- Règles du projet : rien ne se supprime (R-14), toute commande passe par
  `Application::executer` (R-21), journal d'audit chaîné (R-27), deux personnes qui écrivent la
  même chose ne s'écrasent pas (lot A : refus si le texte a changé depuis sa lecture).
- Les migrations s'arrêtent à 0060 le 15 novembre 2026 ; la spécification de la sauvegarde
  hors de l'école, écrite en même temps, peut prendre la suivante.

## Options

| | 1. Toutes les entités du blueprint dès le lot 1 | 2. Tables du lot 1 seulement : programme, contenu, versions, état de l'école | 3. Une table de documents JSON |
|---|---|---|---|
| Apprentissage | Élevé : quiz, devoirs et progression à penser avant d'en avoir besoin | Faible | Faible au début |
| Exploitation | Des tables vides pendant trois lots | Les lots 3 à 5 ajoutent leurs tables par migration | Requêtes et contrôles difficiles |
| Maturité | — | Même démarche que le jardin (trois tables, le reste plus tard) | Aucune contrainte vérifiée par la base |
| Pour un agent de code | Mauvais : beaucoup de code mort | Bon | Mauvais : tout se vérifie à la main |
| Plus gros risque | Des tables figées avant que le portail et les quiz soient décidés | Une table du lot 1 à reprendre au lot 3 | Des incohérences silencieuses |

## Décision

Option 2. Une migration, la suivante libre, crée dix tables et un index de recherche (plus la
table de correspondance de l'ADR « repère commun »). Aucune
ne porte d'année scolaire : un contenu dure d'une année à l'autre. Aucune ligne ne se supprime
(déclencheurs, comme pour les compétences du jardin).

| Table | Ce qu'elle porte |
|---|---|
| `bibliotheque_paquet` | Un paquet installé : code, version, empreinte du fichier, signature vérifiée, installé le et par qui, actif ou remplacé |
| `chapitre_programme` | La colonne vertébrale : clé stable, origine (`livre` ou `ecole`), code du niveau, code de la matière, partie (« Algèbre », « Examens »…), titre, ordre, « programme à confirmer » (livré seulement). Les séries d'un chapitre de lycée sont dans `chapitre_serie` ; aucune série = toutes |
| `chapitre_serie` | Chapitre, code de série |
| `contenu` | Un contenu : clé stable, origine, nature (`lecon`, `sujet`, `corrige`, `document`), chapitre, titre, ordre, langue (`fr`), contenu lié (le corrigé d'un sujet), et pour un sujet l'examen et la session ; pour un contenu livré, le paquet, son empreinte et sa liste de relecture |
| `version_contenu` | Une version écrite par l'école : contenu, numéro, texte Markdown, état (`brouillon`, `soumise`, `renvoyee`, `publiee`, `remplacee`), version de départ (la livrée, avec son empreinte, ou une version de l'école), auteur, dates de soumission et de décision, qui a décidé, commentaire de renvoi, et une marque « retour à la version livrée » (texte vide) |
| `relecture_contenu` | « Relu » : contenu, empreinte du texte relu (livré ou version de l'école), qui, quand |
| `retrait_bibliotheque` | Un retrait et sa remise (ADR « retirer un contenu ») |
| `fichier_bibliotheque` | Image ou document joint par l'école : empreinte, type, taille, nom d'origine, contenu chiffré dans la base (ADR « stockage des contenus ») |
| `video_bibliotheque` | Vidéo de l'école : empreinte, nom, taille, ajoutée le et par qui ; le fichier est hors de la base |
| `droit_publication` | Le droit de publier donné nommément à un maître : utilisateur, donné par, le, retiré par, le |
| `recherche_bibliotheque` | Index plein texte FTS5 (compilé dans SQLite3 Multiple Ciphers, vérifié dans `vendor/libsqlite3-sys/build.rs`) : titre et texte de la version visible de chaque contenu |

Règles portées par le modèle :
- **La version visible** d'un contenu est sa dernière version `publiee` ; sans version publiée,
  ou si la dernière est un « retour à la version livrée », c'est le texte livré. Une seule
  version publiée à la fois : publier passe la précédente à `remplacee`.
- **Un brouillon à la fois par contenu**. Il se réécrit sur place tant qu'il est brouillon, avec
  un compteur de révision : un enregistrement fait sur une révision dépassée est refusé (« Cette
  leçon a été modifiée depuis que vous l'avez ouverte »). Chaque enregistrement, chaque
  soumission, chaque décision s'écrit au journal d'audit. Une version soumise ne change plus.
- **Le livré se rafraîchit, l'école ne bouge pas** : à l'installation d'un paquet, les lignes
  d'origine `livre` de `chapitre_programme` et `contenu` sont mises à jour par leur clé ; les
  lignes d'origine `ecole`, les versions, les relectures et les retraits ne sont jamais écrits
  par une mise à jour. Un contenu livré qui disparaît d'un paquet n'est pas effacé : il est
  marqué « plus livré » et reste lisible tant que l'école en a une version.
- **Repères** : le niveau, la série et la matière d'un contenu livré sont des codes nationaux,
  pas des identifiants de l'école (ADR « repère commun »).

## Justification

1. Le lot 1 n'a pas besoin des quiz, des devoirs ni de la progression ; les écrire maintenant
   figerait des choix que le portail (lot 4) et les quiz (lot 3) n'ont pas encore faits.
2. La colonne vertébrale est le programme : tout contenu a un chapitre, ce qui tient E-1 par
   construction.
3. Séparer le contenu (ce qui est livré, réinstallable) de ses versions (ce que l'école écrit)
   tient la décision du 14 novembre 2026 : le livré n'est jamais modifié, l'école garde tout.
4. Un brouillon unique avec compteur de révision reprend la règle du lot A contre l'écrasement,
   sans fusion de textes.
5. Une table par fait (relecture, retrait, droit) garde chaque geste daté et attribué, donc
   tracé, sans réécrire de ligne.

## Conséquences

Positives : des tables petites ; le livré se met à jour sans toucher au travail de l'école ;
le journal d'audit raconte l'histoire de chaque leçon ; la recherche plein texte est locale et
rapide.

Négatives : les lots 3 à 5 ajouteront leurs tables (quiz, questions, progression, devoirs) ;
l'index plein texte se refait à chaque publication et à chaque paquet installé ; un maître qui
veut proposer une variante pendant qu'un collègue a un brouillon ouvert doit attendre ou
reprendre ce brouillon ; le texte d'un brouillon qui change plusieurs fois n'est gardé, avant sa
soumission, que dans le journal (avant et après).

## Contre-argument conservé

Tout poser dès maintenant éviterait des migrations aux lots suivants et donnerait au portail un
modèle déjà prêt. Il n'a pas gagné parce que les migrations sont la façon normale d'avancer dans
GESCO depuis la phase 1, alors qu'une table pensée trop tôt se reprend plus cher qu'une table
ajoutée au bon moment.

## Conditions de reconsidération

- Si le lot 3 ou le lot 4 doit changer une colonne de `contenu` ou de `version_contenu`.
- Si des écoles demandent plusieurs brouillons concurrents d'une même leçon.
- Si l'index plein texte ralentit une publication de plus d'une seconde sur un vieux poste.
