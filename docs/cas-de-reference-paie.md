# Cas de référence du calcul de la paie

Cas écrits pour la phase 7 (`SPEC_PHASE_7.md` §2 et §6), chacun calculé à la main. Le moteur
(`crates/gesco-core/src/paie/calcul.rs`) doit retrouver exactement ces résultats ; un test
automatique le vérifie pour chacun. Règles appliquées :

- chaque ligne est **tronquée au franc** : 4 157,4 F s'écrit 4 157 F ;
- **ordre du calcul** : gains (salaire de base, heures, primes) → brut → retenues salariales dans l'ordre des éléments → absences confirmées → avances → net ; les cotisations patronales se calculent à part et ne touchent pas le net ;
- l'**assiette** d'un taux ou d'un barème est le brut, moins les retenues que l'élément désigne (l'ITS retranche la seule retraite INPS) ;
- un **barème annuel** s'applique au brut du mois multiplié par 12 ; l'impôt trouvé, réduit, se divise par 12 ;
- **taux proposés pour le Mali**, à vérifier par l'école ou son comptable : retraite INPS 3,6 % (salarié) et 5,4 % (école), AMO 3,06 % et 3,5 %, prestations familiales 8 % ; ITS : barème annuel de l'article 10 du Code général des impôts, réduction de deux points de l'assiette, puis 10 % pour un marié et 2,5 % par enfant à charge, au plus 35 % ;
- **mois incomplet**, au choix de l'école : au trentième (chaque jour vaut un trentième, le dernier jour du mois complète à trente), aux jours du calendrier, ou aux jours ouvrables.

À vérifier par le porteur du projet et, pour les retenues, par un comptable ; toute différence avec
la pratique d'une école se corrige ici d'abord.

## Cas 1 — La secrétaire en octobre

Engagement à durée indéterminée depuis octobre 2025, salaire de base 90 000 F ; prime de transport
de 10 000 F ; mariée, deux enfants à charge ; retraite INPS, AMO et ITS actifs ; avance de 30 000 F
remboursée en trois mois.

| Ligne | Calcul | Montant |
|---|---|---|
| Salaire de base | 90 000 × 30/30 | 90 000 F |
| Prime de transport | fixe | 10 000 F |
| **Brut** | | **100 000 F** |
| Retraite INPS | 3,6 % de 100 000 | 3 600 F |
| AMO | 3,06 % de 100 000 | 3 060 F |
| ITS | voir ci-dessous | 4 157 F |
| Avance | mensualité 30 000 ÷ 3, reste 20 000 F | 10 000 F |
| **Net à payer** | 100 000 − 3 600 − 3 060 − 4 157 − 10 000 | **79 183 F** |
| Retraite INPS (école) | 5,4 % de 100 000 | 5 400 F |
| Prestations familiales (école) | 8 % de 100 000 | 8 000 F |

ITS : assiette 100 000 − 3 600 = 96 400 F, soit 1 156 800 F l'an. Barème : rien jusqu'à 330 000 F ;
5 % de 330 000 à 578 400 F, soit 12 420 F ; 12 % de 578 400 à 1 156 800 F, soit 69 408 F ; impôt
81 828 F. Moins deux points de l'assiette annuelle (23 136 F) : 58 692 F. Moins 15 % pour la
famille (10 % marié, 2 × 2,5 % pour les enfants), soit 8 803 F : 49 889 F l'an, **4 157 F par mois**.

## Cas 2 — Un vacataire fonctionnaire

22 h 30 faites en octobre, à 2 500 F l'heure ; fonctionnaire de l'État, sans situation de famille
indiquée.

| Ligne | Calcul | Montant |
|---|---|---|
| Heures de vacation | 22,5 × 2 500 | 56 250 F |
| Retraite INPS, AMO | pas pour un fonctionnaire | — |
| ITS | 675 000 F l'an : 12 420 + 96 600 × 12 % = 24 012 F, moins 13 500 F = 10 512 F, ÷ 12 | 876 F |
| **Net à payer** | | **55 374 F** |

Des heures d'un mois précédent, visées après sa paie, s'ajoutent en rappel avec leur mois.

## Cas 3 — Arrivée le vendredi 16 octobre 2026

Salaire de base 90 000 F.

| Prorata | Jours payés | Montant |
|---|---|---|
| Au trentième | 15/30 (du 16 au 30 ; le 31 ne compte pas) | 45 000 F |
| Aux jours du calendrier | 16/31 | 46 451 F |
| Aux jours ouvrables | 11/22 (octobre compte 22 jours du lundi au vendredi) | 45 000 F |

## Cas 4 — Nouveau salaire au 16 octobre

90 000 F jusqu'au 15 octobre, 100 000 F à partir du 16, au trentième : 15/30 de 90 000 = 45 000 F,
plus 15/30 de 100 000 = 50 000 F ; **95 000 F**, le mois entier payé une fois.

## Cas 5 — Arrivée le 15 février 2027

Au trentième : du 15 au 27, 13 jours, et le dernier jour du mois en vaut trois (28 jours + 2 = 30) ;
16/30 de 90 000 = **48 000 F**, comme « trente moins les quatorze jours d'absence ».

## Cas 6 — Mois de vacances

Un engagement « hors des mois de vacances » ne paie pas août : aucun bulletin. Le même engagement
« douze mois » paie 90 000 F.

## Cas 7 — Absences confirmées par la direction

Salaire de base 90 000 F, une journée et une demi-journée non justifiées, confirmées.

| Prorata | Journée | Demi-journée | Net |
|---|---|---|---|
| Au trentième | 90 000 ÷ 30 = 3 000 F | 1 500 F | 85 500 F |
| Aux jours ouvrables | 90 000 ÷ 22 = 4 090 F | 2 045 F | 83 865 F |

## Cas 8 — Une avance plus forte que ce qui reste à payer

Indemnité de stage de 5 000 F, avance de 30 000 F en mensualités de 10 000 F : 5 000 F seulement
sont retenus, le net est nul, 25 000 F restent dus ; GESCO signale l'avance reportée.

## Cas 9 — Plafond et régularisation

Une mutuelle à 10 % plafonnée à 50 000 F d'assiette, sur un salaire de 100 000 F : 5 000 F. Une
régularisation de −5 000 F (trop-perçu du mois précédent) réduit le brut à 95 000 F ; net 90 000 F.
Une retenue saisie plus forte que le brut rend le net négatif : GESCO l'avertit et la paie ne se
valide pas tant qu'il n'est pas corrigé.
