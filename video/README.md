# Vidéo de présentation de GESCO

Vidéo pour les directeurs et promoteurs d'écoles privées du Mali, faite avec
[HyperFrames](https://hyperframes.heygen.com) (HTML et GSAP rendus en MP4). Elle présente
GESCO 1.2.5 d'après `docs/manuel-utilisateur-1.2.md` et les notes de version 1.2.0 à 1.2.5.
Aucune capture de l'application : chaque écran est redessiné, en simplifié.

| Format | Fichier | Durée | Pour |
|---|---|---|---|
| 16:9, 1920 × 1080 | `index.html` | 60 s | présentation, site, réunion |
| 9:16, 1080 × 1920 | `compositions/vertical.html` | 30 s | statuts WhatsApp, Facebook, TikTok |

Les MP4 rendus sont dans `renders/` (`GESCO-16x9.mp4`, `GESCO-9x16.mp4`, avec la voix off) ;
après une modification, les refaire avec les commandes ci-dessous et les commiter de nouveau.

## Les scènes

| # | 16:9 | 9:16 | Scène | Écran redessiné |
|---|---|---|---|---|
| 1 | 0–6 s | 0–5 s | GESCO, « La gestion de votre école, sur un seul ordinateur, sans internet. » | logo, version 1.2.5 |
| 2 | 6–14 s | — | Inscrire un élève | Scolarité › Élèves : photo, nom, matricule, classe, responsable retrouvé par son téléphone |
| 3 | 14–23 s | 5–12,5 s | Encaisser en moins d'une minute | Finance › Caisse : montant, mode, reste à payer, reçu numéroté avec QR code, chronomètre |
| 4 | 23–33 s | 12,5–21 s | Des notes au bulletin | Scolarité › Notes puis › Bulletins : coefficients, totaux, moyenne, rang, mention |
| 5 | 33–40 s | — | La conduite suivie | Scolarité › Note de conduite : 20 moins les retraits du registre |
| 6 | 40–48 s | — | La paie du mois | Personnel › Paie : trois salaires, une prime, retenue INPS 3,6 %, net à payer |
| 7 | 48–55 s | 21–26 s | Toute l'école d'un coup d'œil | Général › Tableau de bord ; « Données chiffrées », « Sauvegarde chaque jour », « Sans internet » |
| 8 | 55–60 s | 26–30 s | Fin | logo, GESCO, « Pour la gestion de votre école », 72 08 22 28 |

Données fictives : noms maliens courants, classes 7e A et 10e S, 1er trimestre 2026-2027.
Les calculs tiennent : 276 points sur 19 coefficients donnent 14,53 (mention « Bien », 14 et
plus) ; la note de conduite applique les retraits livrés (avertissement 2, retenue 1, demi-journée
d'absence non justifiée 0,5, retard 0,25) ; l'INPS salarié est à 3,6 % du brut
(`docs/cas-de-reference-paie.md`). Le numéro du reçu suit le format de GESCO (`REC-2026-000147`) ;
son QR code est réel et contient `GESCO:DOC:DEMO-REC-2026-000147`.

Volontairement absents : les SMS aux familles, toute matière de langue nationale ou religieuse,
toute adresse, tout nom d'école réel, tout prix, tout chiffre de performance autre que
« encaisser en moins d'une minute ». Pas de musique.

## Voix off

Une phrase par scène, voix de synthèse française « Siwis » (Kokoro, générée hors ligne),
ramenée à -16 LUFS. Le texte est dans `outils/voix-off.json`, les MP3 dans `assets/voix/`,
placés par les `<audio id="voix-…">` en fin de chaque composition.

| Scène | 16:9 | 9:16 |
|---|---|---|
| 1 | GESCO, la gestion de votre école, sur un seul ordinateur, sans internet. | GESCO : votre école sur un seul ordinateur, sans internet. |
| 2 | Inscrivez vos élèves, du jardin d'enfants au lycée. Un parent déjà connu se retrouve par son téléphone. | — |
| 3 | À la caisse, encaissez en moins d'une minute : le reçu numéroté s'imprime, et le reste à payer est aussitôt à jour. | Encaissez en moins d'une minute, avec un reçu numéroté, et le reste à payer à jour. |
| 4 | Saisissez les notes : GESCO calcule les moyennes et les rangs, avec les coefficients, puis imprime les bulletins. | Des notes au bulletin : moyennes et rangs calculés, bulletins imprimés. |
| 5 | Le registre suit les sanctions et les absences, et la note de conduite se calcule. | — |
| 6 | Chaque mois, la paie : salaires, primes, retenues, et les bulletins de paie. | — |
| 7 | Toute l'école d'un coup d'œil. Vos données sont chiffrées, et sauvegardées chaque jour. | Toute l'école d'un coup d'œil, sans internet. |
| 8 | Pour la gestion de votre école : soixante-douze, zéro huit, vingt-deux, vingt-huit. | Soixante-douze, zéro huit, vingt-deux, vingt-huit. |

Pour changer le texte, la vitesse ou la voix : modifier `outils/voix-off.json`, puis

```
pip install kokoro-onnx soundfile     # une fois
node outils/voix-off.mjs              # ou : node outils/voix-off.mjs 16x9-s3
```

La commande affiche la durée de chaque phrase : reporter-la dans le `data-duration` de son
`<audio>` et vérifier qu'elle finit avant le fondu de sortie de sa scène. Pour une vraie voix,
remplacer simplement les MP3 de `assets/voix/` (même nom, même découpage).

## Installer

Node.js 22 ou plus et FFmpeg, puis :

```
cd video
npm install
npx hyperframes browser ensure
```

Rien n'est chargé depuis Internet au rendu : GSAP (`assets/gsap.min.js`), les polices Fira Sans
400, 600 et 700 (`assets/fonts/`, licence OFL), le logo (`assets/logo.png`, copie de
`crates/gesco-app/icons/icon.png`) et la voix off (`assets/voix/`) sont dans `assets/`. Après une mise à jour de `gsap` ou de
`@fontsource/fira-sans`, recopier les fichiers depuis `node_modules`.

## Vérifier

```
npx hyperframes check                       # 16:9
sh outils/verifier-vertical.sh check        # 9:16
npx hyperframes snapshot --at 3,18,52       # images de contrôle 16:9
sh outils/verifier-vertical.sh snapshot --at 3,9,24
```

`check` et `snapshot` ne lisent que l'`index.html` d'un dossier : `outils/verifier-vertical.sh`
copie la composition verticale dans `.verif-vertical/` (chemins `../assets/` ramenés à
`assets/`) et y lance la commande. Les assertions de mouvement sont dans `index.motion.json` et
`compositions/vertical.motion.json`.

## Rendre

```
npx hyperframes render --quality high --output renders/GESCO-16x9.mp4
npx hyperframes render --composition compositions/vertical.html --quality high --output renders/GESCO-9x16.mp4
```

## Modifier

- Une seule timeline GSAP en pause par composition, enregistrée sur `window.__timelines`
  (`gesco` et `vertical`) ; rendu déterministe (ni `Math.random`, ni `Date.now`).
- Seuls `x`, `y`, `scale` et `opacity` sont animés. Les compteurs et les saisies changent le
  texte d'après le temps de la timeline (`piloter`), jamais d'après l'horloge.
- Texte de lecture d'au moins 28 px (32 px en 9:16) ; en 9:16, tout le texte reste entre 168
  et 1 330 px de haut, sous la barre des applications et au-dessus de leurs boutons.
- Couleurs de `frontend/src/styles.css` : fond `--marine` uni, cartes `--fond` et `--surface`,
  encre `--encre`, accent `--primaire`, bleu clair `#8fb0ff` pour les sur-titres.
- Icônes : [Lucide](https://lucide.dev) (ISC), tracés recopiés dans le HTML.
