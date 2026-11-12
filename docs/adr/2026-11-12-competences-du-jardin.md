# Les compétences du jardin : trois tables, une échelle fixe, le livret porté par le bulletin

**Date** : 12 novembre 2026
**Statut** : proposé le 12 novembre 2026 avec l'étape 1 du lot B, d'après `SPEC_PHASE_11_B.md` v1.1 dont le porteur du projet a validé les recommandations (T-109 à T-115) le 12 novembre 2026 ; à accepter par le porteur du projet — décision 10 de `docs/plan-phase-11.md`, étape B1

## Contexte

Le jardin d'enfants s'inscrit et paie depuis la phase 2, mais n'a aucun bulletin. Le blueprint
(§8.4 B) prévoyait trois entités, `DomaineApprentissage`, `Competence` et `Observation`,
reportées le 7 octobre 2026. Le plan de la phase 11 laissait trois questions ouvertes : les
entités telles quelles ou simplifiées ; une échelle fixe ou réglable ; une liste par année
scolaire (ADR-012) ou non.

- Vérifié dans le code (migrations 0001 à 0050) : les niveaux PS, MS, GS portent déjà
  `ModeEvaluation::Competences` ; le jardin n'a ni période, ni matière, ni programme.
- La table `bulletin` (migration 0019) a déjà tout ce que le livret demande : une ligne par
  élève et par période, des versions, la validation, la publication, la remise et l'avis par
  SMS ; `moyenne` et `rang` y sont facultatifs, le contenu est un JSON figé.
- La table `appreciation` porte déjà une appréciation par élève, période et clé de matière.
- T-109 à T-115 (validées le 12 novembre 2026) : une vingtaine de compétences livrées par
  section ; trois niveaux fixes aux libellés réglables ; un livret qui suit l'année ; trois
  trimestres par défaut, deux au choix ; le livret dans toute licence.

## Options

| | 1. Tables du blueprint et table de livret à part | 2. Trois tables, le livret dans `bulletin` | 3. Compétences rangées comme des matières |
|---|---|---|---|
| Apprentissage | Moyen : un cycle de vie à refaire | Faible : celui du bulletin | Faible au début |
| Exploitation | Deux circuits de remise et d'avis à tenir | Un seul circuit | Un seul circuit |
| Maturité | — | Le bulletin sert depuis la phase 4 | Le calcul des moyennes trébucherait partout |
| Pour un agent de code | Beaucoup de code dupliqué | Bon : des tables petites, un contenu de plus | Mauvais : des exceptions dans chaque calcul |
| Plus gros risque | Deux circuits qui divergent | Un lecteur de `bulletin` qui compte un livret comme un bulletin à moyenne | Une compétence comptée dans une moyenne |

## Décision

Option 2. Aucune dépendance nouvelle, donc aucune version à retenir.

- **Trois tables** (migration 0051) : `domaine_competence` (commun aux trois sections, un code
  stable pour les domaines livrés), `competence` (par section : `niveau_id`), `observation`
  (clé : inscription, compétence, période ; niveau vide pour une observation vidée). Aucune
  ne se supprime (déclencheurs) ; un domaine ou une compétence se retire et se remet.
- **Pas d'année scolaire** sur la liste : elle vaut d'une année à l'autre ; le livret fige les
  libellés au moment où il se prépare, donc un renommage ne change aucun livret passé.
- **Échelle fixe à trois niveaux** (`acquis`, `en_cours`, `non_acquis`), contrainte dans la
  table ; seuls les libellés se règlent, un réglage chacun (`jardin.echelle_acquis`,
  `jardin.echelle_en_cours`, `jardin.echelle_non_acquis`).
- **Le livret est un bulletin** : une ligne de `bulletin` par enfant et par période, contenu
  `"forme": "livret"`, sans moyenne ni rang (étape 3). L'appréciation générale et le
  commentaire par domaine vont dans `appreciation`.
- **Liste livrée posée par l'application**, pas par la migration : aux sections ouvertes, à
  l'installation, à l'ouverture d'une base et quand l'école ouvre son jardin ; jamais une
  seconde fois pour une section qui a déjà des compétences, même retirées.

## Justification

Le livret vit exactement comme un bulletin : préparé, validé, publié, corrigé, remis, annoncé
par SMS. Le porter par `bulletin` donne tout cela sans une ligne de plus et garde un seul
circuit de remise et de bulletins non retirés. Les compétences ne sont pas des matières : les
ranger parmi elles mettrait une exception dans chaque calcul de moyenne. Une liste sans année
évite à l'école de tout recopier chaque rentrée ; le livret figé protège le passé. Une échelle
fixe garde le livret lisible d'une école à l'autre et en photocopie ; les mots, eux, restent
ceux de l'école. Le code stable d'un domaine livré permet de poser la liste d'une section
ouverte plus tard même si l'école a renommé « Arts ».

## Conséquences

Positives : trois petites tables ; remise, avis, non retirés et droits inchangés ; aucun droit
nouveau ; une école sans jardin ne voit rien.

Négatives : tout ce qui lit `bulletin` doit être vérifié à l'étape 3 (moyenne annuelle, fin
d'année, consultation par SMS, tableau de bord) ; les périodes du jardin existent désormais et
d'autres écrans pourraient les montrer ; une compétence ne change pas de section, il faut la
recopier.

## Contre-argument conservé

Une table de livret à part serait plus claire à lire : un livret n'a ni moyenne ni rang, et
chaque lecteur de `bulletin` devra se garder de la forme « livret ». Il n'a pas gagné parce
que dupliquer validation, publication, versions, remise et avis coûte plus, et diverge plus
sûrement, que vérifier une fois les lecteurs de `bulletin`.

## Conditions de reconsidération

- Si plus de deux lecteurs de `bulletin` doivent distinguer la forme « livret » à l'étape 3.
- Si le porteur du projet demande des compétences au 1er cycle (hors du lot, T-84).
- Si des écoles demandent une échelle à quatre niveaux ou plus.
