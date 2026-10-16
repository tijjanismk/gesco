# SPEC_PHASE_4.md — Évaluations, notes, absences, bulletins

**Projet** : ERP de gestion scolaire — Tauri 2 + Rust + React/TypeScript
**Phase** : 4 — référentiel pédagogique, enseignants et affectations, évaluations et saisie des notes, moteur de calcul des moyennes, absences, bulletins, transfert sortant
**Statut** : v1.0 — spécification exécutable, engagée le 10 octobre 2026 après les réponses du porteur du projet à T-26 à T-37 ; Q-54 à Q-61 retenues (§16)
**Documents parents** : `PROJECT_BLUEPRINT.md` v0.23 (§2.3 UC-03, UC-04 ; §8.2, §8.4, §8.4bis, §8.5, §8.5bis ; §10.4 ; §11.3, §11.5 ; §12.3 ; §13 ; §14.1 ; §24.5, §24.6, §24.7 Q-27 à Q-34 ; Q-39), `ARCHITECTURE_RULES.md` v0.3 (R-11, R-14, R-17, R-20, R-28, R-35, R-42), `SPEC_PHASE_3.md` v1.3, `docs/questions-terrain.md` (T-22 à T-37)
**Date** : 10 octobre 2026

---

## 0. Objet et critère de fin

### 0.1 Ce que produit cette phase

Le livrable du blueprint (§22.2) : **UC-03, saisir les notes d'une composition** (une classe de 90 élèves, une matière, au clavier, en moins de 10 minutes) et **UC-04, produire les bulletins d'une classe** (calcul, rangs, appréciations, validation, PDF en lot, impression, sans intervention technique). C'est le jalon commercial du blueprint (§22.4) : à la fin de cette phase, une école peut gérer une année scolaire courante, inscriptions, argent et bulletins.

À la fin de la phase :
- l'école a ses matières, leurs coefficients par niveau et par série, ses périodes (neuf mois au 1er cycle, trois trimestres ensuite) et ses règles d'évaluation ;
- chaque classe a ses enseignants, matière par matière ; au 1er cycle, le maître reçoit toutes les matières de sa classe d'un geste ;
- la direction crée les évaluations et saisit les notes au clavier, l'absent restant distinct du zéro ;
- les moyennes sont calculées comme l'enseignant les calcule au crayon, au centième près, tronquées et jamais arrondies ;
- les bulletins de toute une classe sortent en PDF, validés puis publiés, et un bulletin publié ne change plus ;
- les absences sont relevées et comptées au bulletin ;
- un élève qui part reçoit son certificat de transfert et le relevé de ses moyennes.

### 0.2 Décisions déjà prises qui s'appliquent

Arrêtées dans le blueprint, elles ne sont pas rediscutées ici :
- **Formules** (§8.4bis) : moyenne de matière = Σ notes × pondérations ÷ (Σ barèmes × pondérations ÷ barème du niveau) ; moyenne générale = moyenne pondérée par les coefficients ; moyenne annuelle = moyenne pondérée des périodes. Une seule formule par étage, le 1er cycle ayant tous ses coefficients à 1.
- **Troncature à deux décimales, jamais d'arrondi** (RC-1), en arithmétique entière, sans aucun flottant (RC-6) ; la moyenne de matière est tronquée avant d'alimenter la moyenne générale (Q-20, `tronquer_avant_report` vrai par défaut).
- **La note est stockée telle qu'écrite**, avec le barème de son évaluation (RC-2). **L'absent n'est pas un zéro** (RC-3). La version des règles est conservée (RC-4), tout bulletin se recalcule à l'identique (RC-5).
- **Barèmes** : /10 au 1er cycle, composition mensuelle seule, sans devoir (Q-21) ; /20 avec coefficients aux 2e cycle et lycée, devoirs et composition.
- **Bulletin mensuel au 1er cycle**, neuf périodes (Q-22).
- **La direction saisit les notes** à partir du relevé de l'enseignant (T-23).
- Plusieurs enseignants peuvent partager une matière d'une classe (T-22) ; un vacataire est un enseignant comme les autres (T-24) ; au 1er cycle, un maître par classe, une matière pouvant être confiée à un autre (T-25).
- Arrivée en cours d'année : périodes antérieures « non suivies » (Q-27), moyennes reprises d'un relevé officiel (Q-28), classement dès la période d'arrivée par défaut (Q-29).
- Transfert sortant : certificat numéroté et relevé des moyennes (Q-32) ; un impayé avertit sans bloquer (Q-33) ; les échéances non échues s'annulent par contre-passation (Q-34).
- **Pas d'évaluation par compétences au jardin** (Q-03 révisée) : les enfants du jardin sont inscrits, sans bulletin en phase 4.

### 0.3 Critère de fin, vérifiable

La phase est terminée quand ces douze assertions sont vraies, chacune démontrée par un test automatisé ou par le scénario de bout en bout (§13) :

1. Aucune note ni moyenne ne passe par un flottant, du noyau aux types TypeScript, aux exports et aux PDF ; une note se saisit au centième (`13,5` ou `13,50`) et se stocke en centièmes avec son barème.
2. Les moyennes de matière, générale et annuelle reproduisent le calcul au crayon sur les cas de référence du §8 (tables de cas écrites avec le porteur du projet), tronquées à deux décimales.
3. Un élève absent à une évaluation n'a pas de note ; il est exclu du calcul, ou compté zéro si l'absence n'est pas justifiée et que l'école l'a choisi ; un élève dispensé n'est jamais compté.
4. Le rang suit la règle de l'école pour les ex aequo ; un élève sans moyenne n'est pas classé ; l'effectif classé est imprimé.
5. Une évaluation ne propose que les types autorisés au cycle de sa classe ; une note hors barème est refusée à la saisie.
6. Une évaluation validée ne se modifie plus que par un déverrouillage motivé (`grades:unlock`), tracé ; toute saisie et toute modification de note est auditée avec la valeur avant et après.
7. La portée tient (§11.3) : un compte enseignant ne saisit que dans ses couples classe et matière ; la direction saisit partout.
8. UC-03 : 90 notes d'une composition se saisissent au clavier seul, Entrée passant à l'élève suivant, en moins de 10 minutes dans le scénario Playwright.
9. UC-04 : les bulletins d'une classe se calculent, se valident, se publient et sortent en un PDF prêt à imprimer, avec les absences de la période ; un bulletin publié est immuable, une correction en crée une nouvelle version, l'ancienne restant consultable.
10. Recalculer une période à partir des notes redonne exactement les résultats enregistrés (RC-5), sous test de propriétés.
11. Les moyennes et le rang d'un élève arrivé en cours d'année suivent Q-27 à Q-29 ; une moyenne reprise d'un relevé porte sa source.
12. Un transfert sortant produit le certificat numéroté et le relevé des moyennes, avertit d'un impayé et annule les échéances non échues.

---

## 1. Périmètre

**Inclus** : matières et programme par niveau et série (coefficient, barème, volume horaire indicatif) ; périodes de l'année par niveau ; règles d'évaluation ; enseignants et affectations ; portée par classe et matière ; évaluations et notes ; import d'une grille de notes Excel (si T-35 le confirme) ; moteur de calcul ; appréciations ; absences et retards ; résultats de période ; bulletins mensuels et trimestriels ; moyenne annuelle ; moyennes reprises ; certificat de transfert et relevé des moyennes.

**Exclu** :
- Notification des bulletins par SMS, remise des documents et convocations (§8.5bis) : phase 5.
- Sanctions disciplinaires : une phase ultérieure, à fixer avec le porteur du projet (T-34).
- Décisions de passage, conseil de fin d'année, réinscriptions en masse, clôture d'année : phase 10 (UC-06, Q-30).
- Emploi du temps et absences par séance : phase 8 (§8.6). Les absences de phase 4 se relèvent par journée ou demi-journée.
- Dossier du personnel, contrats, paie : phases 6 et 7. L'enseignant de la phase 4 est un nom et, s'il le faut, un compte ; le dossier RH le complétera sans migration de ses affectations (Q-55).
- Évaluation par compétences du jardin (Q-03).
- Portail parent, consultation par SMS entrant : phases 5 et 11.

---

## 2. Référentiel pédagogique

### 2.1 Matières

`matiere` : code, libellé, libellé court (colonne du bulletin), groupe (littéraire, scientifique, autre), ordre d'affichage. Le référentiel est livré (Q-54) et l'école le complète :
- **1er cycle** : les cinq domaines du curriculum 2011 (Q-23) — Langue et communication, Sciences, mathématiques et technologie, Sciences humaines, Arts, Développement de la personne — ou les disciplines de 1994 au choix de l'école (§16.5) ;
- **2e cycle et lycée** : une liste par défaut à confirmer (T-27), que l'école modifie.

Une matière ne se supprime pas si une note la référence ; elle se désactive.

*Précision du 13 octobre 2026* : au 1er cycle, le référentiel livré ajoute les matières de l'emploi du temps des écoles (langage, lecture, élocution, vocabulaire, grammaire, conjugaison, initiation à l'orthographe, dictée, rédaction, écriture, récitation, numération, mesure, géométrie, problèmes pratiques, sciences d'observation, histoire, géographie, exercices sensoriels, économie familiale, technologie, dessin, chant), avec les mathématiques, l'éducation civique et morale et l'EPS ; le programme livré de chaque année les reprend dans l'ordre de son emploi du temps (1re-2e : 12 matières ; 3e : 19 ; 4e à 6e : 22). Les cinq domaines restent au référentiel, hors du programme livré. Aucun texte affiché ne cite d'où viennent ces choix.

### 2.2 Programme de chaque niveau

`programme_matiere` `(annee, niveau, serie?, matiere)` → coefficient, barème du niveau, volume horaire hebdomadaire indicatif (Q-37), obligatoire. Le coefficient dépend de la série au lycée. Au 1er cycle, aucun coefficient n'est saisi (Q-16) : le calcul prend 1. Aux 2e cycle et lycée, une matière sans coefficient bloque le calcul de la moyenne générale avec un message qui la nomme ; elle ne bloque pas la saisie des notes.

Le programme d'une année se reprend de l'année précédente, comme la grille de frais.

### 2.3 Périodes

La table `periode_academique` existe depuis la phase 0.5 ; elle est rattachée à l'année **et au niveau** (§24.6). Livré par défaut, modifiable par l'école :
- 1er cycle : neuf périodes mensuelles, d'octobre à juin (T-26) ;
- 2e cycle et lycée : trois trimestres, la norme (T-26, réponse du 10 octobre 2026) ; une école qui n'en fait que deux pour un niveau passe à **deux trimestres** (octobre-janvier, février-juin), **jamais à des semestres** (réponse du 11 octobre 2026). La moyenne annuelle se calcule sur les deux ou trois trimestres, sans autre réglage.

Chaque période a ses bornes, sa pondération dans la moyenne annuelle (1 par défaut, T-33) et un verrou. Une période verrouillée refuse toute note et toute absence ; elle se déverrouille par la direction, avec motif.

### 2.4 Règles d'évaluation

`regle_evaluation` par année et cycle (§8.2), versionnée (RC-4) :
- types d'évaluation autorisés et leurs barèmes : au 1er cycle la composition mensuelle seule, sur 10 ; ensuite des devoirs, autant que l'enseignant en donne, et la composition, sur 20 par défaut (T-28) ;
- calcul de la moyenne de matière (T-28) : **note de classe = moyenne des devoirs ramenée au barème du niveau, puis (note de classe + 2 × composition) ÷ 3**, par défaut ; sans devoir, la composition seule ; sans composition, la note de classe seule. Réglage de l'école : la somme des barèmes du blueprint (§8.4bis), identique avec un seul devoir et une composition sur 40 ;
- traitement de l'absent non justifié : exclu (défaut) ou zéro, au choix de l'école (T-29) ;
- `tronquer_avant_report` : vrai (Q-20) — la moyenne de matière entre dans la moyenne générale telle qu'imprimée ;
- **arrondi des moyennes au centième, au choix de chaque école** (réponse du 11 octobre 2026) : couper (défaut, RC-1), au plus proche, ou par excès ; le blueprint, qui imposait la troncature, est assoupli ;
- ex aequo au rang (T-30) : « 3e ex, puis 5e », seule règle ;
- échelle des mentions (T-31), au choix de l'école : livrée sur 20, Excellent à 18, Très bien à 16, Bien à 14, Assez bien à 12, Passable à 10, Médiocre à 8, Insuffisant en dessous (réponses du 11 octobre 2026). **Excellent, Très bien, Bien, Assez bien et Passable sont fixes** ; sous Passable, chaque école choisit ses mentions et leurs seuils (Médiocre, Insuffisant, Nul…) ; ramenée sur 10 au 1er cycle ;
- moyenne annuelle (T-33) : calculée et enregistrée pour toutes les classes, obligatoire dans les classes d'examen (9e, 12e) ; imprimée au bulletin dans les classes d'examen par défaut, partout en réglage ;
- classement des arrivées en cours d'année (Q-29).

---

## 3. Enseignants et affectations

### 3.1 Enseignant

En attendant le dossier du personnel (phase 6), un enseignant est une `personne` avec un statut (titulaire, vacataire) et, s'il saisit lui-même, un compte utilisateur lié (Q-55). La phase 6 le rattachera à `employe` sans toucher aux affectations.

### 3.2 Affectation

`affectation_enseignement` `(annee, classe, matiere, enseignant)` (§8.2). Plusieurs enseignants par couple classe et matière sont permis (T-22). Au 1er cycle, « affecter le maître à la classe » crée toutes les affectations de la classe d'un geste ; une matière se réaffecte ensuite (T-25). Le professeur principal est une propriété de la classe.

### 3.3 Portée

La garde combine permission, établissement et ressource (§11.3). `grades:create` et `grades:update` portent, pour un compte lié à un enseignant, sur ses seuls couples classe et matière ; pour la direction, sur tout l'établissement. La vérification est dans la frontière Rust, jamais dans l'écran.

### 3.4 Rôles

Composition des rôles livrés en version 4 (Q-56) : le Directeur reçoit `grades:create` et `grades:update` (T-23) ; il n'a toujours aucune permission `payments:*` (§11.5). Le Secrétaire reste sans `grades:*` sauf réponse contraire à T-35.

---

## 4. Évaluations et notes

### 4.1 Évaluation

`evaluation` `(periode, classe, matiere)` : type (parmi ceux du cycle), libellé (« Devoir 1 », « Composition de mars »), date, barème, pondération (1 par défaut), enseignant correcteur, saisi par, statut. Machine du §10.4 : brouillon → saisie → validée → publiée.

### 4.2 Note

`note` `(evaluation, inscription)` → valeur en centièmes, nullable ; `absent`, `justifie`, `dispense`. Une valeur et un drapeau d'absence s'excluent. Aucune note pour un élève dont l'inscription n'est pas validée dans la classe à la date de l'évaluation.

### 4.3 Saisie au clavier (UC-03, R-42)

Une grille, un élève par ligne, dans l'ordre de la liste de classe. Au clavier seul :
- un nombre (`13`, `13,5`, `13.5`) ; Entrée passe à l'élève suivant ;
- `a` pour absent, `aj` pour absent justifié, `d` pour dispensé (Q-57) ;
- une valeur hors barème est refusée sur place, sans quitter la ligne ;
- la saisie s'enregistre au fil de l'eau : une coupure de courant ne perd que la note en cours.

Le relevé papier de l'enseignant peut être joint à l'évaluation (pièce jointe, Q-39).

### 4.4 Validation et déverrouillage

La direction valide une évaluation complète (chaque élève a une note, une absence ou une dispense). Validée, elle ne se modifie plus ; `grades:unlock` la rouvre avec motif. Toute écriture de note est auditée, valeur avant et après (§12.3).

### 4.5 Import d'une grille (Q-39, canal 3)

Si T-35 le confirme : GESCO produit, par évaluation, un classeur Excel pré-rempli des élèves ; l'enseignant le remplit ; l'import contrôle chaque ligne (élève reconnu, valeur dans le barème, codes d'absence) et montre les écarts avant d'écrire, comme l'import des élèves de la phase 2.

---

## 5. Moteur de calcul

Pur, dans `gesco-core`, sans E/S (R-03), en entiers (RC-6) :
- moyenne de matière d'une période (§8.4bis niveau 1, ou la variante de T-28) ;
- moyenne générale (niveau 2), coefficients du programme ;
- moyenne annuelle (niveau 3) : somme des moyennes des périodes suivies divisée par leur nombre (T-33), les pondérations des périodes restant possibles ; périodes non suivies exclues (Q-27), moyennes reprises comprises (Q-28) ;
- rang et effectif classé, ex aequo selon T-30 ;
- mention selon les seuils de l'école (T-31).

Toute valeur affichée ou imprimée est tronquée à deux décimales. Les tests de référence (§14) sont écrits avec des cas calculés à la main, et un test de propriétés vérifie que recalculer redonne le même résultat (RC-5).

---

## 6. Absences et retards

`absence` `(inscription, date)` : demi-journée (matin, après-midi) ou journée, motif, justifiée, justificatif ; `retard` avec une durée. Saisie par classe et par jour, au clavier : la liste de classe, une case par demi-journée. Justification après coup (`attendance:justify`). Totaux par période au bulletin : demi-journées d'absence, dont justifiées, retards (T-32).

*Précisions de réalisation (étape 6, 11 octobre 2026)* : une seule table `absence`, une ligne par élève et par demi-journée, de nature absence ou retard (un élève n'est pas à la fois absent et en retard sur la même demi-journée) ; la journée entière se note sur ses deux cases. Codes du relevé : vide (présent), `a` (absent), `r` ou `r15` (retard, de 1 à 240 minutes). Un jour à venir, hors des périodes du niveau ou dans une période verrouillée ne se relève pas ; un élève n'est sur le relevé qu'à partir de sa date d'inscription. Le relevé fait d'une classe pour un jour est noté (`releve_absences`), pour distinguer une journée sans absent d'une journée non relevée. Une absence effacée n'est jamais supprimée (R-14). Une absence justifiée ne se change que par qui justifie. Portée : qui justifie relève partout ; un compte d'enseignant, dans les classes de ses affectations. Rôles en version 6 : le Directeur reçoit `attendance:create` et `attendance:justify`.

---

## 7. Bulletins

### 7.1 Résultat de période

`resultat_periode` `(inscription, periode, matiere?)` → moyenne, rang, effectif, mention, version des règles, calculé le. Jamais saisi, toujours recalculable (§8.4). Le calcul d'une classe se relance tant que le bulletin n'est pas publié.

### 7.2 Appréciations

Par matière et par élève (enseignant ou direction), et une appréciation générale (direction ou professeur principal). Une liste d'appréciations types se choisit au clavier ; le texte reste libre (T-31).

### 7.3 Bulletin

`bulletin` `(inscription, periode)`, machine brouillon → calculé → validé → publié (§10.4). Publié, il est immuable ; une correction produit une nouvelle version, l'ancienne marquée remplacée et consultable. La publication compose le PDF (moteur documentaire, R-44) avec QR de vérification.

Deux modèles :
- **1er cycle, mensuel** : matières, note sur 10, total, moyenne sur 10, rang, absences, appréciation, signatures (T-32) ;
- **2e cycle et lycée, trimestriel** : matière, note de classe (moyenne des devoirs), composition, moyenne de la période sur 20, coefficient, moyenne × coefficient, appréciation, enseignant ; total des moyennes pondérées, somme des coefficients, moyenne générale, rang, effectif, mention, absences (T-32) ; dans les classes d'examen, la moyenne annuelle (T-33).
- **Sur les deux modèles** (réponse du 11 octobre 2026) : le rang de l'élève, **la moyenne du premier et la moyenne du dernier de la classe**, et **le nombre d'élèves qui ont composé**.

Génération en lot d'une classe : un seul PDF à imprimer, barre de progression, reprise après coupure. Cible mesurée en mode optimisé : 400 bulletins en moins de 2 minutes (§24.6).

*Précisions de réalisation (étape 7, 12 octobre 2026)* : le résultat de période n'a pas de table à lui : il est figé dans le contenu du bulletin (JSON), avec en colonnes la moyenne et le rang que la moyenne annuelle relira. Un bulletin existe dès son premier calcul (pas d'état brouillon enregistré) ; états : calculé → validé → publié → remplacé, et retour de validé à calculé quand une note change. Le calcul ne prend que les inscriptions validées de la classe ; une matière sans évaluation n'est pas notée et sort de la moyenne générale (Q-59) ; **une matière qu'aucun élève de la classe n'a eue sur la période ne s'imprime pas** (réponse du 15 octobre 2026 : pas de lignes de tirets pour une matière non enseignée à ce niveau ou pas encore évaluée), alors qu'un élève seul sans note dans une matière des autres la garde, « non noté ». La validation exige que toutes les évaluations de la période soient validées et que le calcul soit à jour. Les appréciations s'écrivent après le calcul : par matière (l'enseignant dans sa portée, ou la direction), générale (qui valide les notes). Une correction crée une version recalculée avec les notes du jour, à valider puis publier ; l'ancienne version publiée reste en vigueur jusque-là, puis est marquée remplacée. Le bulletin porte aussi la moyenne de la classe. **Pas de rang dans une matière** (réponse du 12 octobre 2026) : seule la moyenne générale classe les élèves. Un seul modèle documentaire, `bulletin`, porte les deux mises en page ; le lot d'une classe est un seul PDF, chaque bulletin avec son QR (`GESCO:DOC:<identifiant du bulletin>`) ; un bulletin non publié s'imprime marqué « Projet ». La barre de progression et la reprise du lot (§7.3) attendent l'impression de toutes les classes d'un coup.

*Colonnes imprimées au choix de l'école* (réponse du 12 octobre 2026) : quatorze réglages ; chaque colonne du tableau des matières (note de classe, composition, coefficient, moyenne × coefficient, moyenne de la classe dans la matière, appréciation de l'enseignant ; le nom de l'enseignant ne s'imprime pas sous la matière, réponse du même jour) et chaque ligne du bilan (moyenne générale, élèves qui ont composé, rang, mention, moyenne du premier et du dernier, moyenne de la classe, absences, appréciation de la direction) s'affiche ou non, par un réglage oui/non de l'écran « Établissement » (thème « Bulletin imprimé »). Tout s'affiche par défaut, sauf la moyenne de la classe dans chaque matière. La matière et sa moyenne s'impriment toujours. Le réglage ne touche qu'à l'impression : les notes, les moyennes et le contenu figé des bulletins ne changent pas ; il s'applique à chaque impression, réimpression d'un bulletin publié comprise. Modèle `bulletin` en version 2.

### 7.4 Impayés et bulletin

T-36 : le bulletin se calcule, se publie et s'imprime **toujours**, quel que soit le solde ; aucun réglage ne le bloque. Sa **remise** au parent, suivie en phase 5 (§8.5bis), est le lieu où l'impayé se règle.

---

## 8. Cas de référence du calcul

Écrits avec le porteur du projet avant le code de l'étape 2, chacun avec son résultat calculé à la main :
- 1er cycle : une composition /10 par mois ; moyenne générale de cinq domaines sans coefficient ;
- 2e cycle : deux devoirs /20 et une composition /20, (note de classe + 2 × composition) ÷ 3 (T-28), et la même matière selon la somme des barèmes ;
- lycée : coefficients d'une série ;
- un absent non justifié à un devoir, exclu puis compté zéro (T-29) ;
- un dispensé ;
- deux élèves ex aequo ;
- une moyenne de matière à troncature délicate (13,359 → 13,35) ;
- un élève arrivé au deuxième trimestre, avec une moyenne reprise.

---

*Précisions de réalisation (étape 8, 13 octobre 2026)* : une période est **non suivie** par l'élève inscrit après sa fin : il n'y a ni note (la grille d'une évaluation ne montre que les élèves inscrits à sa date) ni bulletin (Q-27). Un élève **arrive pendant** une période s'il s'inscrit après son début, hors de la première période de l'année, où une inscription tardive n'est pas une arrivée ; son bulletin porte « Arrivée dans l'école le … » ; il est classé si la règle de l'école le veut (Q-29, oui par défaut), sinon il a sa moyenne sans rang et ne déplace pas celui des autres. Pour une période non suivie, la direction (`grades:validate`) saisit la moyenne générale du relevé officiel de l'ancienne école, avec l'école d'origine et la référence du relevé (`moyenne_reprise`, Q-28), jamais les notes de détail. La **moyenne annuelle** se calcule au bulletin de la dernière période de l'année, pour toutes les classes : moyennes des périodes telles qu'imprimées, pondérées, les moyennes reprises comprises, les périodes non suivies exclues ; le rang annuel classe tous les élèves qui en ont une. Une période suivie dont le bulletin n'est pas encore calculé suspend la moyenne annuelle de l'élève. Elle s'imprime dans les classes d'examen (DEF, BAC) ou partout selon la règle (T-33) ; elle est toujours enregistrée.

## 9. Transfert sortant

Commande `inscription_transferer` (`enrollments:transfer`) : date de départ, établissement de destination, motif. Dans une transaction : inscription transférée, échéances non échues annulées par contre-passation (Q-34), avertissement si un impayé reste (Q-33, un paramètre permet d'exiger un solde nul, la direction passant outre avec motif). Documents : certificat de transfert numéroté (`CTR-2026-000001`) et relevé des moyennes des périodes suivies, sans rang pour la période en cours, avec le matricule national au lycée (Q-32).

*Précisions de réalisation (étape 9, 13 octobre 2026)* : le transfert annule les échéances à venir sans paiement (date après le départ) et laisse dues les échues ; l'impayé échu avertit (`transfert.impaye`) ; le paramètre `transfert.solde_nul_exige` (faux par défaut) le rend bloquant, sauf motif de la direction pour passer outre (Q-33). Le certificat garde son numéro (`certificat_transfert`), réimprimable ; le relevé des moyennes reprend, par période suivie, la moyenne de chaque matière et la moyenne générale du bulletin publié en vigueur, sans rang pour la période du départ, et les moyennes reprises avec leur source. **Exclusions** (réponse du 13 octobre 2026, les deux formes, dès cette étape) : l'**exclusion définitive** radie l'inscription et annule ses échéances à venir sans paiement, gardées dans l'exclusion ; la direction peut la **révoquer** par une décision motivée : l'élève revient dans sa classe (transition radiée → validée) et ses échéances sont rétablies en échéances dues nouvelles, les anciennes restant annulées (R-14). L'**exclusion temporaire** (30 jours au plus) laisse l'élève inscrit jusqu'à sa date de retour ; elle se lève avant son terme avec motif ; pendant l'exclusion, le relevé des absences le montre exclu et refuse de le compter absent. Permissions : `enrollments:transfer` (transfert, certificat), `enrollments:expel` (exclusions), `reports:generate` (relevé). Les sanctions (avertissement, blâme, conseil de discipline) restent pour la phase de la discipline (T-34).

---

## 10. Commandes (aperçu)

| Commande | Permission |
|---|---|
| `matieres_lister`, `matiere_creer`, `matiere_modifier` | lecture connectée ; `academic_year:manage` |
| `programme_lire`, `programme_enregistrer`, `programme_reprendre` | lecture connectée ; `academic_year:manage` |
| `periodes_lister`, `periode_modifier`, `periode_verrouiller`, `periode_deverrouiller` | lecture connectée ; `academic_year:manage` ; `grades:unlock` |
| `regles_evaluation_lire`, `regles_evaluation_enregistrer` | lecture connectée ; `academic_year:manage` |
| `enseignants_lister`, `enseignant_creer`, `enseignant_modifier` | lecture connectée ; `academic_year:manage` |
| `affectations_lister`, `affectation_creer`, `affectation_retirer`, `maitre_affecter` | lecture connectée ; `academic_year:manage` |
| `evaluations_lister`, `evaluation_creer`, `evaluation_modifier` | `grades:create` (portée) |
| `notes_lire`, `note_saisir` | `grades:create` / `grades:update` (portée) |
| `evaluation_valider`, `evaluation_deverrouiller` | `grades:validate` ; `grades:unlock` |
| `grille_notes_exporter`, `grille_notes_importer` | `import:grades` |
| `portee_absences_lire`, `absences_releve_lire`, `absences_releve_enregistrer`, `absences_periode_lire`, `absences_justifier` | connectée ; `attendance:create` (portée) ; `attendance:justify` |
| `bulletins_lire`, `resultats_calculer`, `appreciation_saisir` | `grades:create` ; `reports:generate` ; `grades:create` (portée) |
| `bulletins_valider`, `bulletins_publier`, `bulletin_corriger` | `grades:validate` ; `reports:publish` |
| `bulletins_produire`, `bulletin_produire` | `reports:generate` |
| `moyennes_reprises_lire`, `moyenne_reprise_saisir` | `grades:validate` |
| `inscription_transferer`, `certificat_transfert_produire`, `releve_moyennes_produire` | `enrollments:transfer` ; `reports:generate` |
| `inscription_exclure`, `exclusion_lever`, `exclusions_lister` | `enrollments:expel` ; lecture `students:read` |

---

## 11. Modèle de données

Migrations à partir de 0015 : `matiere`, `programme_matiere`, `regle_evaluation` ; `enseignant`, `affectation_enseignement` ; `evaluation`, `note` ; `absence`, `releve_absences` ; `appreciation`, `resultat_periode`, `bulletin` ; `moyenne_reprise` ; colonnes de transfert sur `inscription` (déjà présentes depuis la phase 2). Index du §8.9 : `note (evaluation_id)`, `note (inscription_id)`, `absence (inscription_id, date)`. Jamais de DELETE sur les notes validées, les résultats et les bulletins (R-14) ; audit renforcé sur les notes validées et les bulletins publiés (§12.3).

---

## 12. Événements

`EvaluationCreee`, `NotesSaisies`, `EvaluationValidee`, `EvaluationDeverrouillee`, `AbsenceRelevee`, `AbsenceJustifiee`, `ResultatsCalcules`, `BulletinValide`, `BulletinPublie` (servira à la notification de la phase 5), `BulletinCorrige`, `InscriptionTransferee`.

---

## 13. Essais de bout en bout et captures

- la direction saisit les matières et les coefficients d'un niveau, reprend ceux de l'année précédente ;
- le maître du 1er cycle est affecté à sa classe d'un geste, l'anglais confié à un autre ;
- **UC-03 chronométré** : une composition de 90 élèves saisie au clavier, avec absents et dispensés ;
- refus d'une note hors barème, d'un type d'évaluation étranger au cycle ;
- validation d'une évaluation, puis déverrouillage motivé ;
- relevé des absences d'une journée, puis justification ;
- **UC-04** : calcul, validation, publication et PDF des bulletins d'une classe du 1er cycle (mensuel) et d'une classe du 2e cycle (trimestriel) ;
- correction d'un bulletin publié : nouvelle version, l'ancienne consultable ;
- transfert sortant : certificat et relevé.

---

## 14. Tests de la phase

| Cible | Nature |
|---|---|
| Calcul | Cas de référence du §8, au centième près ; troncature ; aucune valeur intermédiaire tronquée hors `tronquer_avant_report` |
| Propriétés (R-35, RC-5) | Recalcul identique ; moyenne bornée par le barème ; un absent exclu ne change pas le dénominateur des autres ; le rang est une permutation cohérente des moyennes |
| Saisie | Barème, codes d'absence, statut de l'évaluation, période verrouillée |
| Portée | Un compte enseignant refusé hors de ses affectations, la direction partout |
| Transitions | Évaluation, bulletin : tables exhaustives |
| Audit | Toute note écrite après validation porte avant et après |
| Performance | 400 bulletins en moins de 2 minutes, en mode optimisé |
| Bout en bout | Scénarios du §13, captures |

---

## 15. Ordre de réalisation

1. Référentiel pédagogique : matières, programmes et coefficients, périodes, règles d'évaluation ; écrans
2. Enseignants et affectations, maître du 1er cycle ; portée par classe et matière ; rôles en version 4
3. Moteur de calcul et cas de référence ; tests de propriétés
4. Évaluations et saisie des notes ; validation et déverrouillage ; UC-03 chronométré
5. Import d'une grille de notes Excel (si T-35)
6. Absences et retards
7. Résultats de période, appréciations, bulletins : calcul, validation, publication, PDF en lot, versions
8. Moyenne annuelle, arrivées en cours d'année et moyennes reprises
9. Transfert sortant : certificat et relevé des moyennes
10. UC-04 complet, navigation, captures, validation

---

## 16. Questions à trancher

### 16.1 Questions de terrain

Posées dans `docs/questions-terrain.md`, T-26 à T-37 ; réponses du porteur du projet le 10 octobre 2026, reportées dans ce document. Écarts à la recommandation : trois trimestres avec la possibilité de deux (T-26) ; matières, coefficients et mentions au choix de l'école, échelle Excellent à Passable (T-27, T-31) ; moyenne annuelle imprimée dans les seules classes d'examen (T-33). T-30 et T-33 précisés le même jour : « 3e ex, puis 5e » seule règle ; moyenne annuelle enregistrée pour toutes les classes.

### 16.2 Questions de conception

Recommandations retenues le 10 octobre 2026, sans objection du porteur du projet.

| Id | Question | Recommandation | Étape |
|---|---|---|---|
| **Q-54** | Livrer un référentiel de matières ? | **Oui** : les cinq domaines de 2011 au 1er cycle, une liste par défaut aux 2e cycle et lycée ; l'école modifie, ajoute, désactive. Coefficients du 2e cycle et du lycée à saisir par l'école, aucun n'étant écrit en dur | 1 |
| **Q-55** | Qui est l'enseignant avant le dossier du personnel ? | **Une personne avec un statut et, s'il saisit, un compte lié** ; la phase 6 le rattache à l'employé sans toucher aux affectations | 2 |
| **Q-56** | Composition des rôles en version 4 ? | **Directeur : `grades:create` et `grades:update`** (T-23) ; rien d'autre ne change sans réponse à T-35 | 2 |
| **Q-57** | Codes de saisie des absences dans la grille ? | **`a` absent, `aj` absent justifié, `d` dispensé**, en minuscules ou majuscules | 4 |
| **Q-58** | Qui valide et publie les bulletins ? | **La direction, en deux gestes distincts** : valider (le conseil de classe s'est tenu), puis publier (le PDF est figé) ; un réglage permet de publier dès la validation | 7 |
| **Q-59** | Une moyenne de matière sans aucune note (toutes les évaluations manquées) ? | **« Non noté »** : la matière sort du calcul de la moyenne générale, coefficient compris, et le bulletin l'indique | 3 |
| **Q-60** | Un élève qui change de classe en cours de période ? | **Ses notes suivent l'élève** : chaque note reste attachée à l'évaluation de sa classe d'alors ; le bulletin se calcule dans la classe du jour de clôture de la période, avec toutes ses notes de la période | 3 |
| **Q-61** | Les notes d'une évaluation se voient-elles avant validation sur le bulletin ? | **Non** : seules les évaluations validées entrent dans le calcul des bulletins ; la grille montre les autres comme « en cours » | 7 |

---

## 17. Journal des révisions

| Version | Date | Modification |
|---|---|---|
| v1.0 | 2026-10-10 | Réponses du porteur du projet : périodes (T-26), note de classe et composition (T-28), absent au choix (T-29), mentions et coefficients au choix de l'école (T-27, T-31), moyenne annuelle des classes d'examen (T-33), bulletin jamais bloqué (T-36) ; spécification engagée |
| v0.1 | 2026-10-10 | Création, soumise au porteur du projet avec T-26 à T-37 et Q-54 à Q-61 |
