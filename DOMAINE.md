# Domaine — règles métier

Le code n'implémente encore aucune règle métier au-delà des types de `commun/`
(voir `modules/gesco-core.md`). Les règles ci-dessous sont celles du blueprint et
des règles opposables : elles sont la cible, pas l'existant.

Les références renvoient à des numéros de section (§), de règle (R-xx), de risque (RA-xx) ou de question (Q-xx), jamais à des numéros de ligne : ils bougent à chaque révision. Blueprint v0.17, règles v0.3.

## Valeurs

- [CONFIRMÉ] Montants en entiers de francs CFA — R-11
- [CONFIRMÉ] Notes et moyennes en entiers de centièmes — R-11bis
- [CONFIRMÉ] Identifiants ULID générés côté client — R-10
- [CONFIRMÉ] Toute table métier porte `etablissement_id` — R-12
- [CONFIRMÉ] Suppression physique interdite sur les entités financières et académiques validées (versement, facture, remise, dépense, session de caisse, bulletin de paie, note validée, bulletin publié, inscription) ; suppression logique ailleurs — R-14, ADR-010
- [CONFIRMÉ] Données historiques partitionnées par année scolaire — R-16
- [CONFIRMÉ] Code écrit dans le vocabulaire du domaine, en français ; permissions en anglais — R-54 (amendée le 4 octobre 2026)
- [CONFIRMÉ] Risques du blueprint en `RA-xx`, règles en `R-xx` — `PROJECT_BLUEPRINT.md` §23

## Référentiel académique (gestion)

- [CONFIRMÉ] `Niveau` porte `mode_evaluation` (notes / compétences) — `PROJECT_BLUEPRINT.md` §8.2, ADR-013
- [CONFIRMÉ] Séries du lycée : 10e commune ; 11e `11S`, `11SES`, `11LL` ; 12e `TSE`, `TSExp`, `TSEco`, `TSS`, `TLL` (TAL abandonnée) — `PROJECT_BLUEPRINT.md` §8.2
- [CONFIRMÉ] `Matiere.groupe` = littéraire / scientifique / autre — `PROJECT_BLUEPRINT.md` §8.2
- [CONFIRMÉ] Horaires et coefficients dans `ProgrammeMatiere (annee, niveau, serie?, matiere)` — `PROJECT_BLUEPRINT.md` §8.2
- [CONFIRMÉ] 1re et 2e année : chaque domaine du curriculum 2011 (LC, SMT, SH, Arts, DP) est une `Matiere` ; 50 séances de 30 min par semaine (Q-23, Q-24) — `PROJECT_BLUEPRINT.md` §24.5
- [CONFIRMÉ] Horaires du 1er cycle : grille du curriculum 2011 par défaut de la 1re à la 6e, **indicative, jamais bloquante** — aucun contrôle ne refuse un emploi du temps qui s'en écarte ; souplesse limitée aux horaires, les autres règles restent appliquées (Q-37, remplace Q-25) — `PROJECT_BLUEPRINT.md` §16.5 (point 5), §24.5
- [CONFIRMÉ] Pas de coefficient au 1er cycle (Q-16) : un coefficient vide y est « sans objet » — `PROJECT_BLUEPRINT.md` §16.5 (point 6), §24.5

## Inscriptions

- [CONFIRMÉ] Arrivée en cours d'année et transferts : Q-26 à Q-36 tranchées (facturation et classement paramétrables, périodes non suivies exclues de la moyenne, moyennes reprises sur relevé officiel, certificat de transfert numéroté, échéances non échues contre-passées) — `PROJECT_BLUEPRINT.md` §24.7
- [CONFIRMÉ] `Inscription.type` inclut `transfert entrant`, `statut` inclut `transférée` ; transition `VALIDÉE → TRANSFÉRÉE` réservée à la direction — `PROJECT_BLUEPRINT.md` §8.3, §10.1

## Évaluation

- [CONFIRMÉ] Neuf périodes mensuelles au 1er cycle (rangs 1 à 6), trois au-delà, dans la même année ; `PeriodeAcademique.type` ∈ mensuelle / trimestre / semestre (Q-22) — `PROJECT_BLUEPRINT.md` §8.1, §24.6
- [CONFIRMÉ] Barème /10 aux rangs 1 à 6, /20 aux rangs 7 à 12 (Q-15) ; une seule formule de moyenne, la différence entre cycles est portée par le paramétrage (Q-17) — `PROJECT_BLUEPRINT.md` §8.4bis
- [CONFIRMÉ] L'absent n'est pas un zéro : valeur nulle et drapeau, exclu du numérateur et du dénominateur sauf paramètre (RC-3, Q-08) — `PROJECT_BLUEPRINT.md` §8.4, §8.4bis
- [CONFIRMÉ] Composition mensuelle du 1er cycle notée sur 10 (Q-21) — `PROJECT_BLUEPRINT.md` §8.4bis, Q-21
- [CONFIRMÉ] Moyenne de matière tronquée avant report, `tronquer_avant_report` vrai par défaut (Q-20) — `PROJECT_BLUEPRINT.md` §8.4bis (RC-7), Q-20

## Module e-learning

- [CONFIRMÉ] Modèle `Cours`, `Chapitre`, `Lecon`, `Media`, `Quiz`, `DevoirEnLigne`, `Soumission`, `Progression` — `PROJECT_BLUEPRINT.md` §16.1
- [CONFIRMÉ] `ProgrammeOfficiel` (avec `code` et `fait_foi`) et `ObjectifProgramme` — `PROJECT_BLUEPRINT.md` §16.1
- [CONFIRMÉ] Ces entités servent uniquement l'e-learning ; aucun écran de gestion ne les référence — `PROJECT_BLUEPRINT.md` §16.1
- [CONFIRMÉ] Champ `langue` sur `Cours`, `Lecon`, `Media`, `Question` ; `fr` par défaut, `bm` sans migration — `PROJECT_BLUEPRINT.md` §16.1
- [CONFIRMÉ] Une note d'e-learning n'entre jamais seule dans le bulletin ; un enseignant la reporte — `PROJECT_BLUEPRINT.md` §16.2
- [CONFIRMÉ] Élèves en PWA, pas d'APK — `PROJECT_BLUEPRINT.md` §16.5 (point 1), ADR-017

## Programmes officiels (sources des données)

- [CONFIRMÉ] Version la plus récente fait foi ; 1re et 2e année livrées d'abord — `PROJECT_BLUEPRINT.md` §16.5
- [CONFIRMÉ] Curriculum niveau 1 (2011) : 26 h 30 par semaine, LC 12 h, SMT 7 h, DP 3 h, SH 1 h 30, Arts 1 h 30 ; 14 UA mensuelles — `PROJECT_BLUEPRINT.md` §16.5 (point 3)
- [CONFIRMÉ] Bambara : français d'abord, TTS et STT traités à part, conversion Unicode des anciennes polices — `PROJECT_BLUEPRINT.md` §16.5 (point 7)
- [CONFIRMÉ] Offre hébergée retenue (Q-11) ; avis d'un juriste APDP avant mise en service — `PROJECT_BLUEPRINT.md` §24.3
- Données : `outils/donnees-initiales/referentiel/`. Analyse détaillée : guide d'intégration https://claude.ai/code/artifact/1afffe27-54d8-4988-9bcc-889c5326eda2

## Finance

- [CONFIRMÉ] Responsable légal et payeur sont deux qualités indépendantes ; un versement s'impute partiellement sur plusieurs échéances (`ImputationVersement`) — `PROJECT_BLUEPRINT.md` §3.6, §8.3, §8.8
- [CONFIRMÉ] Aucun encaissement hors d'une session de caisse OUVERTE ; transitions OUVERTE → CLÔTURÉE → RAPPROCHÉE, écart non nul motivé — `PROJECT_BLUEPRINT.md` §8.8, §10.9
- [CONFIRMÉ] Remise : PROPOSÉE → ACCORDÉE / REFUSÉE, ACCORDÉE → ANNULÉE par contre-passation ; motif obligatoire — `PROJECT_BLUEPRINT.md` §10.10

## Sécurité et sauvegarde

- [CONFIRMÉ] Clé de données aléatoire sous deux enveloppes (machine, secours) ; phrase de secours de douze mots imprimée ; la sauvegarde embarque l'enveloppe de secours et jamais l'enveloppe machine — `PROJECT_BLUEPRINT.md` §17.3, §18 ; `SPEC_PHASE_0_5.md` §6.3 ; R-57. **Sous réserve de Q-38**
- [CONFIRMÉ] Mode LAN embarqué dans `gesco-app` ; `gesco-server` réservé au cloud — `PROJECT_BLUEPRINT.md` §6, ADR-014

## Interface (frontend, pas encore créé)

- [CONFIRMÉ] Composants shadcn/ui sur Radix UI ; tableaux de bord Tremor en composants copiés dans le dépôt (le paquet npm exige React 18) ; Tailwind 4 au thème remplacé par les jetons ; React 19 ; toutes les dépendances à leur dernière version stable au démarrage, puis figées par le verrouillage — `PROJECT_BLUEPRINT.md` §5.2
- [CONFIRMÉ] Bleu (`primaire`) couleur principale ; mode sombre qui suit le système, avec choix manuel ; jetons clair/sombre `fond`, `surface`, `marine`, `encre`, `muet`, `primaire`, `rouge`, `vert`, `ocre` alimentent les variables shadcn et les couleurs Tremor ; contraste > 6:1 dans les deux thèmes — `PROJECT_BLUEPRINT.md` §19.6
- [CONFIRMÉ] Règle des encres conservée dans les grilles : saisi en bleu, calculé en noir, à corriger en rouge souligné — `PROJECT_BLUEPRINT.md` §19.6
- [CONFIRMÉ] Polices embarquées : Fira Sans (interface), Andika (portail élève et e-learning), Charis SIL (bulletins) ; critère éliminatoire, afficher ɛ ɔ ɲ ŋ — `PROJECT_BLUEPRINT.md` §19.6
- [CONFIRMÉ] 9 interdits visuels, dont palette Tailwind par défaut et valeurs arbitraires bloquées par l'outillage — `PROJECT_BLUEPRINT.md` §19.6
- [CONFIRMÉ] E-learning au style LMS WordPress (Tutor LMS, LearnDash) : catalogue, page de cours en accordéon, lecteur de leçon, quiz, tableau de bord élève ; sélecteur `fr` / `bm` dès qu'une version bambara existe — `PROJECT_BLUEPRINT.md` §19.7

## Qualité du code et versions

- [CONFIRMÉ] Dernières versions stables au démarrage, puis figées par les verrous : Rust 1.99.0 en édition 2024, Tauri 2.12.1, sqlx 0.9.0, axum 0.8.9, tokio 1.53.2, typst 0.15.1, ulid 3.0.0… ; seule exception, `tauri-specta` 2.0.0-rc.25 épinglé exactement — `PROJECT_BLUEPRINT.md` §5.4
- [CONFIRMÉ] Aucun signe de « vibe coding » : 12 règles (pas de commentaire paraphrase, de code mort, de `TODO`/`todo!()`, de nom générique, de `unwrap()` hors tests, de `any`, de fichier de gabarit, de README décoratif…) contrôlées par clippy, ESLint, `cargo machete`, `knip` et une recherche en CI — `PROJECT_BLUEPRINT.md` §5.5 ; R-55 et R-56 d'`ARCHITECTURE_RULES.md` (ex-« R-47/R-48 » du prompt, numéros qui désignaient déjà l'import et la sauvegarde)

## Points ouverts (blueprint §24.8)

- [OUVERT] **Q-38** — clé de secours indépendante de la machine : recommandation ci-dessus retenue par défaut ; **bloque l'étape 11 de la phase 0.5**
- [OUVERT] **Q-39** — qui saisit physiquement les notes sur un poste unique (Q-13) : secrétariat depuis le relevé papier signé, enseignant quand il le peut, import de grille ; auteur de saisie et correcteur tracés séparément — phase 4
- [OUVERT] **Q-40** — durée des phases et date du jalon commercial ; la phase 0.5 sert d'étalon — avant la phase 2
