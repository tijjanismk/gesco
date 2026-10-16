# Cas de référence du calcul des moyennes

Cas écrits pour la phase 4 (`SPEC_PHASE_4.md` §8), chacun calculé à la main, comme l'enseignant au
crayon. Le moteur (`crates/gesco-core/src/pedagogie/calcul.rs`) doit retrouver exactement ces
résultats ; un test automatique le vérifie pour chacun. Règles appliquées :

- les moyennes sont **tronquées** à deux décimales par défaut : 11,8333… s'écrit 11,83 et 13,3583… s'écrit 13,35. Chaque école peut choisir d'arrondir au plus proche ou par excès (cas 9) ;
- 2e cycle et lycée : **note de classe = moyenne des devoirs**, puis **(note de classe + 2 × composition) ÷ 3** ;
- la moyenne de chaque matière est tronquée avant d'entrer dans la moyenne générale ;
- un absent n'a pas de note : il est exclu, ou compté zéro si l'absence n'est pas justifiée et que l'école l'a choisi.

À vérifier par le porteur du projet ; toute différence avec la pratique d'une école se corrige ici
d'abord.

## Cas 1 — 1er cycle, cinq domaines, sans coefficient

Compositions du mois, sur 10.

| Domaine | Note |
|---|---|
| Langue et communication | 7,5 |
| Sciences, maths et technologie | 6 |
| Sciences humaines | 8 |
| Arts | 9 |
| Développement de la personne | 7 |

Moyenne : (7,5 + 6 + 8 + 9 + 7) ÷ 5 = 37,5 ÷ 5 = **7,50 sur 10**, mention « Bien ».

## Cas 2 — 2e cycle, deux devoirs et une composition

Mathématiques : devoirs 12 et 15 sur 20, composition 11 sur 20.

- Note de classe : (12 + 15) ÷ 2 = **13,50**
- Moyenne : (13,50 + 2 × 11) ÷ 3 = 35,50 ÷ 3 = 11,8333… → **11,83**

Pour comparaison, l'autre formule (somme des barèmes, composition sur 40, soit 22 sur 40) donne
(12 + 15 + 22) ÷ 4 = **12,25**. Les deux formules ne coïncident qu'avec un seul devoir.

## Cas 3 — moyenne générale avec coefficients

| Matière | Moyenne | Coefficient | Moyenne × coefficient |
|---|---|---|---|
| Mathématiques | 11,83 | 4 | 47,32 |
| Français | 14,00 | 4 | 56,00 |
| Anglais | 9,50 | 2 | 19,00 |
| Physique-chimie | 10,25 | 3 | 30,75 |
| **Total** | | **13** | **153,07** |

Moyenne générale : 153,07 ÷ 13 = 11,7746… → **11,77**.

## Cas 4 — élève absent à un devoir sans justification

Devoir 1 : 12 ; devoir 2 : absent ; composition : 11.

- Absence exclue (par défaut) : note de classe 12 → (12 + 22) ÷ 3 = 11,333… → **11,33**
- Absence comptée zéro : note de classe (12 + 0) ÷ 2 = 6 → (6 + 22) ÷ 3 = 9,333… → **9,33**
- Absence justifiée : toujours exclue → **11,33**

## Cas 5 — élève dispensé

Dispensé d'EPS : la matière n'est pas notée et sort de la moyenne générale, coefficient compris.
Français 14 (coefficient 4), anglais 10 (coefficient 2) : (56 + 20) ÷ 6 = 12,666… → **12,66**.

## Cas 6 — ex aequo

| Élève | Moyenne | Rang |
|---|---|---|
| A | 14,25 | 1er |
| B | 12,50 | 2e ex |
| C | 12,50 | 2e ex |
| D | 11,00 | **4e** |
| E | pas de moyenne | non classé |

Classés : 4 ; moyenne du premier : 14,25 ; moyenne du dernier : 11,00.

## Cas 7 — troncature, jamais d'arrondi

Devoirs 13,25 et 13,50 : note de classe 13,375 ; composition 13,35.
(13,375 + 26,70) ÷ 3 = 40,075 ÷ 3 = 13,3583… → **13,35**. Un arrondi aurait donné 13,36.

## Cas 8 — moyenne annuelle

- Trois trimestres : (11,77 + 12,40 + 13,10) ÷ 3 = 37,27 ÷ 3 = 12,4233… → **12,42**
- Élève arrivé au 2e trimestre : (12,40 + 13,10) ÷ 2 = **12,75**
- École à deux trimestres : (10,00 + 11,01) ÷ 2 = **10,50**

## Cas 9 — chaque école choisit son arrondi

| Valeur exacte | Couper (défaut) | Au plus proche | Par excès |
|---|---|---|---|
| 13,3583… (cas 7) | 13,35 | 13,36 | 13,36 |
| 11,8333… (cas 2) | 11,83 | 11,83 | 11,84 |
| 13,355 | 13,35 | 13,36 | 13,36 |
| 12,50 | 12,50 | 12,50 | 12,50 |
