# Questions de terrain

Depuis le 7 octobre 2026, ces questions remplacent la collecte des documents réels de trois écoles (blueprint §27). Le porteur du projet y répond ; une réponse datée fait foi au même titre qu'une décision du §24 du blueprint. Chaque question porte une recommandation : répondre « d'accord » suffit pour la retenir.

## Phase 2 — élèves, responsables, classes, inscriptions

Déjà tranché, pour mémoire : matricule national obligatoire et unique au lycée, vide au fondamental (Q-18) ; date de naissance toujours exigée (Q-19) ; aucune unicité sur nom, prénoms et date de naissance (§3.5) ; responsable légal et payeur sont deux qualités distinctes (§3.6) ; arrivée en cours d'année et transferts (Q-26 à Q-36) ; le jardin d'enfants s'inscrit dès la phase 2, son évaluation par compétences reste reportée (Q-03 révisée le 7 octobre 2026).

### T-01 — Forme du matricule interne

Le système attribue à chaque élève un matricule interne qui fait autorité (§3.5). Sous quelle forme les écoles l'attendent-elles ?

*Recommandation* : année d'entrée et numéro d'ordre sans trou, `2026-00042`, attribué par le service de numérotation. Lisible au téléphone, trié par ancienneté, sans information personnelle.

*Réponse* (porteur du projet, 7 octobre 2026) : recommandation retenue, `2026-00042`. *Révisée le même jour* : l'école choisit entre un matricule qu'elle saisit et un matricule généré selon ses règles (préfixe, année, longueur, remise à zéro) ; le matricule national du DEF, lui, se recopie toujours et devient exigé dès la 9e.

### T-02 — Champs de la fiche d'inscription

Quels champs une secrétaire remplit-elle réellement pour un nouvel élève, et lesquels sont obligatoires ?

*Recommandation* — obligatoires : nom, prénoms, sexe, date de naissance (avec la case « date estimée »), lieu de naissance, classe, au moins un responsable avec un téléphone. Facultatifs : nationalité (Malienne par défaut), numéro et date de l'acte de naissance ou du jugement supplétif, école d'origine, photo, adresse.

*Réponse* (porteur du projet, 7 octobre 2026) : recommandation retenue. Au jardin d'enfants, la fiche porte deux à trois responsables : les personnes qui viennent chercher l'enfant.

### T-03 — Responsables et payeurs

Combien de responsables une fiche porte-t-elle en général, et que faut-il savoir d'eux ?

*Recommandation* : de un à trois. Pour chacun : nom, prénoms, lien (père, mère, tuteur, autre), un ou deux téléphones, profession facultative ; cases « responsable légal », « payeur », « reçoit les SMS ». Un même responsable se retrouve d'une fratrie à l'autre sans être ressaisi (§25.5).

*Réponse* (porteur du projet, 7 octobre 2026) : recommandation retenue. S'y ajoute la case « vient chercher l'enfant » (jardin, voir T-02).

### T-04 — Noms des niveaux et des classes

Comment les écoles nomment-elles leurs niveaux et leurs classes sur les documents ?

*Recommandation* — niveaux : 1re année à 6e année (1er cycle), 7e à 9e année (2e cycle, DEF), 10e, 11e et 12e, avec les séries du §8.2. Classes : niveau et lettre, « 7e A », « 11e S B ». Le libellé reste modifiable par l'école.

*Réponse* (porteur du projet, 7 octobre 2026) : au lycée, la classe se nomme par le niveau suivi du code de la série, de deux à trois caractères : « 10CG », « 12TSE ». Le reste de la recommandation est retenu, libellé modifiable par l'école.

### T-05 — Effectif maximal d'une classe

Faut-il bloquer l'inscription quand une classe est pleine, ou seulement avertir ?

*Recommandation* : avertir sans bloquer, avec un effectif maximal par classe saisi par l'école (§3.4 : les classes de 90 élèves existent). La validation affiche l'effectif atteint.

*Réponse* (porteur du projet, 7 octobre 2026) : pas d'effectif maximal. Aucun champ, aucun avertissement ; l'écran affiche seulement l'effectif de la classe.

### T-06 — Fichier Excel des élèves existants

À quoi ressemble le fichier Excel d'une école qui veut importer ses élèves (§14.1, premier obstacle commercial) ? Une feuille par classe, ou une seule feuille ? Quelles colonnes, dans quel ordre ? Les noms et prénoms sont-ils dans la même colonne ?

*Recommandation* : accepter les deux dispositions (une feuille par classe, ou une colonne « classe »), reconnaître les colonnes par leur titre avec une correspondance à confirmer à l'écran, et séparer nom et prénoms quand ils partagent une colonne en proposant un découpage à valider. Un exemple anonymisé, même de cinq lignes, réglerait la question.

*Réponse* (porteur du projet, 7 octobre 2026) : recommandation retenue, à confirmer sur un premier fichier réel quand il se présentera.

### T-07 — Variantes d'orthographe des noms

Quelles variantes la recherche doit-elle rapprocher ? Par exemple Mohamed, Mahamadou et Mahamane ; Traoré et Traore ; Keïta et Keita ; Coulibaly et Kouloubaly.

*Recommandation* : ignorer accents et majuscules, rapprocher les orthographes proches (distance d'édition) et les prénoms d'une table d'équivalences que l'école peut compléter. Une recherche ne fusionne jamais rien d'elle-même : elle propose les doublons probables.

*Réponse* (porteur du projet, 7 octobre 2026) : recommandation retenue.

### T-08 — Fiche d'inscription imprimée

Que porte la fiche d'inscription remise ou signée par le parent (UC-01) ?

*Recommandation* : en-tête de l'école, année, classe, identité de l'élève et matricule, responsables, frais de l'année (phase 3), date, signatures du parent et du secrétariat, et un QR de vérification (§25.9).

*Réponse* (porteur du projet, 7 octobre 2026) : recommandation retenue, sauf le bloc des responsables : chaque école choisit s'il figure sur la fiche imprimée (paramètre d'établissement). Précision donnée le même jour.

### T-09 — Réinscription des anciens élèves

Les élèves de l'année précédente refont-ils une fiche complète ou une simple confirmation ?

*Recommandation* : une confirmation qui reprend la fiche existante, avec la classe proposée d'après la décision de fin d'année ; seuls les changements se ressaisissent. La réinscription en masse arrive avec la clôture d'année (UC-06, phase 10).

*Réponse* (porteur du projet, 7 octobre 2026) : certaines écoles font une réinscription, d'autres non. Paramètre d'établissement : avec réinscription, la confirmation de la recommandation ; sans réinscription, l'élève passe dans la classe proposée par la décision de fin d'année sans démarche.

### T-10 — Photo de l'élève

La photo est-elle utile dès l'inscription (carte scolaire, reconnaissance au guichet), ou peut-elle attendre ?

*Recommandation* : facultative, prise par la webcam ou importée d'un fichier, réduite à moins de 100 Ko. Elle sert à la fiche et, plus tard, à la carte scolaire.

*Réponse* (porteur du projet, 7 octobre 2026) : facultative, sauf au jardin d'enfants où elle est obligatoire. Le reste de la recommandation est retenu.

## Phase 3 — frais, factures, caisse, impayés

Déjà tranché, pour mémoire : montants en francs CFA entiers (R-11) ; payeur distinct du responsable légal (§3.6) ; aucun encaissement hors d'une session de caisse ouverte (§8.8) ; remise proposée par le secrétariat et accordée par la direction (§10.10) ; mobile money saisi à la main avec sa référence (§3.11) ; facturation d'un élève arrivé en cours d'année (Q-26). Posées le 9 octobre 2026, avec `SPEC_PHASE_3.md` v0.1.

### T-11 — Frais pratiqués

Quels frais les écoles demandent-elles, et de quoi dépend leur montant ?

*Recommandation* : frais d'inscription et frais de scolarité, fixés par niveau (et par série au lycée) ; frais facultatifs cochés à l'inscription : cantine, transport, tenue, examen. Une école qui ne fait pas payer un niveau lui donne un tarif à zéro.

*Réponse* (porteur du projet, 9 octobre 2026) : recommandation retenue.

### T-12 — Tranches de paiement

En combien de fois la scolarité se paie-t-elle, et à quelles dates ?

*Recommandation* : un calendrier par grille, modifiable par l'école. Livré par défaut : trois tranches, le 15 octobre, le 15 janvier et le 15 avril, à 40, 30 et 30 %. Une école peut passer au mensuel (neuf échéances d'octobre à juin) ou à une seule échéance. Les frais d'inscription se paient à l'inscription.

*Réponse* (porteur du projet, 9 octobre 2026) : **au choix de chaque parent.** L'école définit dans sa grille de deux à quatre plans de paiement (comptant, trois tranches, mensuel…) ; le parent en choisit un à l'inscription ; le plan se change tant que rien n'est payé sur la scolarité.

### T-13 — Premier versement à l'inscription

L'école exige-t-elle un premier paiement pour qu'une inscription soit validée ?

*Recommandation* : non, aucun blocage : l'inscription se valide, la facture naît, le paiement suit à la caisse. Les frais d'inscription sont simplement dus dès ce jour et apparaissent tout de suite dans les impayés s'ils ne sont pas réglés.

*Réponse* (porteur du projet, 9 octobre 2026) : recommandation retenue : aucun blocage.

### T-14 — Qui encaisse

Dans une petite école, la secrétaire encaisse-t-elle aussi ?

*Recommandation* : oui, c'est permis : un même compte porte les rôles Secrétaire et Caissier. Seuls les notes et l'argent ne se cumulent jamais (§11.5). Dans ce cas, l'assistant d'inscription enchaîne jusqu'au reçu.

*Réponse* (porteur du projet, 9 octobre 2026) : recommandation retenue : un même compte peut être Secrétaire et Caissier.

### T-15 — Paiement pour plusieurs enfants

Quand un parent paie pour trois enfants d'un coup, l'école fait-elle un reçu ou trois ?

*Recommandation* : un reçu par enfant, imprimés à la suite dans la même opération. Chaque dossier garde ses pièces, et un enfant qui change d'école emporte un historique propre.

*Réponse* (porteur du projet, 9 octobre 2026) : recommandation retenue : un reçu par enfant.

### T-16 — Format du reçu et imprimante

Sur quoi l'école imprime-t-elle ses reçus aujourd'hui : carnet à souches, imprimante A4, petite imprimante à ticket ?

*Recommandation* : demi-page A5, deux exemplaires sur une feuille A4 (un pour le parent, un en souche), avec le montant en chiffres et en toutes lettres et un QR code de vérification. Le format ticket de 80 mm viendra si des écoles en ont.

*Réponse* (porteur du projet, 9 octobre 2026) : recommandation retenue : demi-page A5, deux exemplaires sur une feuille A4.

### T-17 — Remises courantes

Quelles réductions les écoles accordent-elles, et sont-elles automatiques ?

*Recommandation* : bourse, demi-bourse, fratrie, enfant du personnel, arrangement avec la famille. Aucune n'est automatique : la secrétaire propose, la direction accorde avec un motif. La réduction fratrie n'est pas calculée d'office, parce que les règles varient d'une école à l'autre (troisième enfant, enfants au même cycle…).

*Réponse* (porteur du projet, 9 octobre 2026) : recommandation retenue : aucune remise automatique.

### T-18 — Versement supérieur au reste dû

Un parent veut payer plus que ce qu'il doit (avance sur l'année suivante, arrondi). Que fait l'école ?

*Recommandation* : refusé en phase 3. Les avances sur l'année suivante arrivent avec la clôture d'année (phase 10).

*Réponse* (porteur du projet, 9 octobre 2026) : recommandation retenue : refusé en phase 3.

### T-19 — Paiements mobiles

Les parents paient-ils par Orange Money, Wave ou Moov, et comment l'école le vérifie-t-elle ?

*Recommandation* : le caissier choisit le mode et saisit la référence de la transaction, obligatoire pour tout paiement qui n'est pas en espèces. Le rapprochement automatique avec les relevés arrivera en phase 8. Ces montants sont totalisés à part à la clôture de caisse : ils ne sont pas dans le tiroir.

*Réponse* (porteur du projet, 9 octobre 2026) : recommandation retenue : mode et référence saisis.

### T-20 — Impayés

Comment l'école suit-elle les impayés, et bloque-t-elle quelque chose (bulletin, examen, accès en classe) ?

*Recommandation* : liste par classe, avec l'ancienneté (0 à 30 jours, 31 à 60, 61 à 90, plus de 90) et le téléphone des payeurs, exportable en Excel. Aucun blocage automatique en phase 3 ; la question du bulletin reviendra en phase 4.

*Réponse* (porteur du projet, 9 octobre 2026) : recommandation retenue : aucun blocage.

### T-21 — Fonctionnement de la caisse

La caisse s'ouvre-t-elle et se ferme-t-elle chaque jour ? Qui garde l'argent le soir ?

*Recommandation* : une session par jour et par caissier, ouverte le matin avec le fonds de caisse, fermée le soir avec le total compté ; un écart exige une explication. Une caisse oubliée ouverte la veille se ferme avant d'en ouvrir une nouvelle. La direction ou le comptable vise ensuite la session.

*Réponse* (porteur du projet, 9 octobre 2026) : **session libre.** Une session de caisse peut durer plusieurs jours ; le caissier la ferme quand il le décide (fin de semaine, remise de l'argent à la direction). Le reste est retenu : fonds à l'ouverture, total compté et écart motivé à la clôture, visa ensuite.

## Phase 4 — notes et bulletins (réponses anticipées)

Posées le 9 octobre 2026, à la suite d'une remarque du porteur du projet : « un maître peut enseigner plusieurs matières dans plusieurs classes ou filières ». Le modèle du blueprint le permet déjà : une affectation d'enseignement par année, classe, matière et enseignant (§8.4), la série étant portée par la classe. Réponses reçues le même jour, à reprendre dans `SPEC_PHASE_4.md`.

### T-22 — Physique et chimie

Une même matière peut-elle être partagée entre deux enseignants dans une classe ?

*Réponse* (porteur du projet, 9 octobre 2026) : la physique-chimie s'enseigne ensemble, par un seul professeur, **mais la possibilité de la partager n'est pas exclue**. Conséquence : plusieurs enseignants peuvent être affectés à la même matière d'une même classe.

### T-23 — Qui saisit les notes

Sur un poste unique, qui tape les notes ? (Q-39)

*Réponse* (porteur du projet, 9 octobre 2026) : **la direction saisit les notes**, à partir du relevé de l'enseignant. Conséquence : le rôle Directeur recevra `grades:create` en phase 4. Il ne détient aucune permission `payments:*` (composition 3), la séparation des notes et de l'argent (§11.5) tient. Les autres canaux de Q-39 (l'enseignant saisit lui-même, import d'une grille Excel) restent à confirmer dans la spécification de la phase 4.

### T-24 — Vacataires

Un vacataire peut-il enseigner dans plusieurs écoles ?

*Réponse* (porteur du projet, 9 octobre 2026) : oui. Chaque école ne voit que ses propres affectations ; un vacataire est un enseignant comme les autres dans chaque établissement où il intervient.

### T-25 — Maître du 1er cycle

Au 1er cycle, une classe a-t-elle un seul maître, et peut-on confier une matière à un autre (anglais, éducation physique) ?

*Réponse* (porteur du projet, 9 octobre 2026) : **généralement un seul maître par classe** ; confier une matière à un autre maître doit rester possible. Conséquence : affecter un maître à sa classe lui donne toutes les matières d'un geste, et une matière se réaffecte ensuite à un autre.

### Phase 4 — questions du 10 octobre 2026

Déjà tranché, pour mémoire : formules de calcul, troncature sans arrondi, note stockée telle qu'écrite, absent distinct du zéro (§8.4bis) ; barème /10 et composition mensuelle seule au 1er cycle (Q-21) ; bulletin mensuel au 1er cycle (Q-22) ; pas de coefficient au 1er cycle (Q-16) ; la direction saisit les notes (T-23). Posées le 10 octobre 2026, avec `SPEC_PHASE_4.md` v0.1.

### T-26 — Périodes de l'année

Quelles sont les périodes de notes, et à quelles dates ?

*Recommandation* : au 1er cycle, neuf mois d'octobre à juin, chacun avec sa composition ; aux 2e cycle et lycée, trois trimestres (octobre-décembre, janvier-mars, avril-juin). L'école corrige les dates chaque année.

*Réponse* (porteur du projet, 10 octobre 2026) : au 1er cycle, **une composition chaque mois**. Aux 2e cycle et lycée, **trois trimestres dans la plupart des cas** ; certaines écoles n'en font que deux, de la 7e à la 9e et au lycée, mais la norme est trois. Conséquence : trois trimestres livrés par défaut, l'école pouvant passer à deux périodes pour un niveau.

Précision du 11 octobre 2026 : la moyenne annuelle se calcule sur deux ou trois trimestres ; **on ne parle pas de semestres à ces niveaux**. Deux périodes s'appellent donc deux trimestres.

### T-27 — Matières et coefficients du 2e cycle et du lycée

Les coefficients des matières de la 7e à la 12e, par série, l'école les a-t-elle par écrit ?

*Recommandation* : GESCO livre une liste de matières par défaut (français, anglais, mathématiques, physique-chimie, sciences de la vie et de la terre, histoire-géographie, éducation civique et morale, philosophie au lycée, éducation physique…) ; l'école saisit les coefficients une fois par niveau et par série, puis les reprend d'une année sur l'autre. Aucun coefficient n'est écrit en dur.

*Réponse* (porteur du projet, 10 octobre 2026) : **c'est à l'école de choisir** ses matières et ses coefficients ; GESCO livre une liste par défaut, sans aucun coefficient écrit en dur.

### T-28 — Moyenne d'une matière avec plusieurs devoirs

Aux 2e cycle et lycée, avec deux devoirs sur 20 et une composition, comment l'enseignant calcule-t-il la moyenne de la matière ?

*Recommandation* : la pratique la plus répandue : **note de classe = moyenne des devoirs, puis (note de classe + 2 × composition sur 20) ÷ 3**. L'autre formule, celle du blueprint, additionne tous les barèmes : avec un seul devoir les deux donnent le même résultat, avec deux devoirs elles divergent (la composition compte pour la moitié au lieu des deux tiers).

*Réponse* (porteur du projet, 10 octobre 2026) : les notes sont apportées par le maître de la classe ; **s'il y a eu des devoirs, c'est à lui de choisir** ce qui forme la note de classe, et il apporte la note de composition. Conséquence : autant de devoirs que l'enseignant veut, leur moyenne fait la note de classe ; la formule recommandée, (note de classe + 2 × composition) ÷ 3, est retenue par défaut, la somme des barèmes du blueprint restant un réglage de l'école.

Précision du 11 octobre 2026 : **chaque école choisit son type d'arrondi** des moyennes. Livré : couper au centième (défaut), au plus proche, ou par excès.

### T-29 — Absent à un devoir ou une composition

Un élève absent sans justification : sa note est-elle exclue ou comptée zéro ?

*Recommandation* : **exclue** par défaut (absent justifié ou non), chaque école pouvant choisir le zéro pour l'absence non justifiée.

*Réponse* (porteur du projet, 10 octobre 2026) : **on a le choix** : soit on ne prend en compte que les autres notes de devoir, soit on met zéro ; le choix doit exister. Conséquence : réglage de l'école, l'exclusion par défaut.

### T-30 — Ex aequo au rang

Deux élèves ont la même moyenne : comment s'écrit le rang ?

*Recommandation* : **même rang, le suivant saute** : deux élèves 3e ex aequo, le suivant est 5e. Le bulletin écrit « 3e ex ».

*Réponse* (porteur du projet, 10 octobre 2026) : recommandation retenue, précisée le même jour : **s'il y a deux élèves troisièmes, le suivant est forcément cinquième**. Une seule règle, « 3e ex, puis 5e ».

### T-31 — Mentions et appréciations

Quelles mentions figurent au bulletin, à partir de quelle moyenne ?

*Recommandation* : seuils modifiables par l'école, livrés par défaut sur 20 : tableau d'honneur à 12, encouragements à 14, félicitations à 16 ; rien sous 12. Appréciations types au choix (« Très bien », « Bien », « Assez bien », « Passable », « Insuffisant », « Doit redoubler d'efforts ») et texte libre.

*Réponse* (porteur du projet, 10 octobre 2026) : **c'est à l'école de décider** ; généralement, 18 et 19, c'est « Excellent », puis « Très bien », « Bien », « Assez bien », « Passable » et la suite. Conséquence : échelle livrée sur 20, Excellent à 18, Très bien à 16, Bien à 14, Assez bien à 12, Passable à 10, ramenée sur 10 au 1er cycle ; seuils et libellés modifiables. Pas de tableau d'honneur par défaut.

Précision du 11 octobre 2026 : **après Passable vient Médiocre**, puis d'autres mentions dont le porteur ne se souvient plus ; « Insuffisant » n'est pas sûr. Proposition livrée, à confirmer : Médiocre à 8, Faible à 6, Très faible à 4, Nul en dessous.

Précision suivante, le même jour : pour les plus faibles, **on écrit « Insuffisant »**, pas « Nul ». Échelle livrée : Médiocre à 8, Insuffisant en dessous ; l'école ajoute d'autres paliers si elle en a. Puis : **Excellent, Très bien, Bien, Assez bien et Passable restent à leur place** ; à partir de Médiocre, l'école choisit (Insuffisant, Nul…).

### T-32 — Contenu du bulletin

Que doit porter le bulletin ?

*Recommandation* : en-tête de l'école, année, période, élève, matricule, classe, effectif ; par matière la note ou la moyenne, le coefficient (2e cycle et lycée), la moyenne pondérée, l'appréciation et le nom de l'enseignant ; la moyenne générale, le rang, la mention, les absences de la période ; l'appréciation de la direction ; les signatures du directeur et du parent ; un QR de vérification.

*Réponse* (porteur du projet, 10 octobre 2026) : recommandation retenue, avec par matière la note de classe (moyenne des devoirs), la composition, la moyenne de la période, le coefficient et la moyenne pondérée ; la somme des moyennes pondérées divisée par la somme des coefficients donne la moyenne générale.

Précision du 11 octobre 2026 : le bulletin porte aussi **les rangs, la moyenne du premier et la moyenne du dernier**, et **le nombre total d'élèves qui ont composé**.

### T-33 — Moyenne annuelle

Comment se calcule la moyenne de l'année ?

*Recommandation* : **moyenne des trois trimestres à poids égal** aux 2e cycle et lycée ; **moyenne des neuf mois** au 1er cycle. Une école qui fait compter le troisième trimestre double le règle dans ses périodes.

*Réponse* (porteur du projet, 10 octobre 2026) : la moyenne annuelle **ne figure généralement pas au bulletin, sauf dans les classes d'examen**, où elle doit être transmise ; elle se calcule alors comme la somme des moyennes des trimestres divisée par le nombre de trimestres effectivement suivis. Précision du même jour : **on en a besoin, l'application la consigne dans tous les cas**, et dans les classes d'examen (9e, 12e) elle doit obligatoirement l'être. Conséquence : GESCO la calcule et l'enregistre toujours, pour toutes les classes ; le bulletin l'imprime par défaut dans les classes d'examen, un réglage permettant de l'imprimer partout.

### T-34 — Sanctions

Les sanctions (avertissement, blâme, exclusion temporaire) doivent-elles entrer dès la phase 4 ?

*Recommandation* : **non**, elles entrent avec la discipline dans une phase ultérieure ; la phase 4 garde les absences et les retards.

*Réponse* (porteur du projet, 10 octobre 2026) : recommandation retenue : les sanctions attendent une phase ultérieure.

### T-35 — Autres façons de saisir les notes

Outre la direction (T-23), qui peut saisir les notes ?

*Recommandation* : l'enseignant avec son propre compte, limité à ses classes et matières, quand l'école le veut ; et une grille Excel que l'enseignant remplit et que la direction importe. Le secrétariat ne saisit pas les notes.

*Réponse* : pas de réponse distincte le 10 octobre 2026 ; recommandation retenue comme annoncé (l'enseignant avec son compte si l'école le veut, grille Excel importée par la direction, pas de saisie par le secrétariat).

### T-36 — Impayé et bulletin

Une école retient-elle le bulletin d'un élève qui n'a pas payé ?

*Recommandation* : **le bulletin se calcule et s'imprime toujours** ; c'est sa remise au parent, suivie en phase 5, qui se fait au guichet où l'impayé se règle. Aucun blocage automatique.

*Réponse* (porteur du projet, 10 octobre 2026) : **les bulletins s'impriment toujours, on ne peut pas les bloquer.**

### T-37 — Relevé des absences

Comment les absences sont-elles relevées aujourd'hui ?

*Recommandation* : par demi-journée (matin, après-midi), saisies par classe chaque jour par le surveillant ou le secrétariat, justifiées ensuite sur présentation d'un mot ou d'un certificat ; les retards notés à part. Le bulletin porte les demi-journées d'absence de la période, dont justifiées.

*Réponse* (porteur du projet, 10 octobre 2026) : recommandation retenue.

### T-38 — Journée de classe et emploi du temps du 1er cycle

Le porteur du projet a transmis le 12 octobre 2026 un emploi du temps réel du 1er cycle (`EMPLOI du temps (1er Cycle).pdf` : 1re et 2e années, 3e, 4e, 5e et 6e années), à reprendre pour l'emploi du temps (phase 8, §8.6). Ce qu'il montre :

- du lundi au vendredi ; le matin de 8 h à 12 h, en séances de 10, 15, 20 ou 30 minutes, avec une récréation de 9 h 50 à 10 h ;
- l'**inter-classe** de 12 h à 15 h : les enfants rentrent à la maison ;
- l'après-midi de 15 h à 17 h 10, trois jours sur cinq selon la classe, avec une récréation de 16 h à 16 h 10 ;
- le même découpage de la matinée d'une année à l'autre ; les matières changent (langage, lecture, mathématiques en 1re et 2e années ; lecture, vocabulaire, grammaire, numération, mesure, géométrie… ensuite).

*Réponse* (porteur du projet, 12 octobre 2026), dictée :

- **les horaires de l'inter-classe sont au choix de chaque école**, et modifiables ;
- **la récréation du soir n'est pas faite partout** : elle se règle par école ;
- **en période de canicule, certaines écoles finissent à 13 h ou 14 h pour le 1er cycle** : il faut pouvoir passer à un horaire réduit pour une période.
- **pas de classe le soir pendant la canicule** (précision du même jour) : la journée s'arrête à la sortie de 13 h ou 14 h, sans reprise après l'inter-classe.
- **l'horaire de canicule ne vaut que pour le 1er cycle** (précision du même jour) : le 2e cycle et le lycée gardent leur journée.
- **les matières du 1er cycle sont celles de cet emploi du temps** (réponse du 13 octobre 2026) : le programme livré de chaque année du 1er cycle les reprend, dans leur ordre ; les cinq domaines restent disponibles pour l'école qui les préfère.

*Conséquences* : la phase 8 décrit la journée de chaque école (ouverture, récréations, inter-classe, fin), avec un horaire de canicule pour le seul 1er cycle, applicable sur une plage de dates, sans séance l'après-midi ; les séances de l'emploi du temps se placent dans ces bornes. Le relevé des absences par demi-journée de la phase 4 n'en dépend pas : une demi-journée sans classe reste vide, sans être comptée.

## Phase 6 — personnel

Posées le 14 octobre 2026, après la validation de la phase 4. Ordre des phases décidé le même jour par le porteur du projet : 6, 7, 8 et 10, puis 5 et 9. Remarque du porteur à l'origine de T-40 : « pour le contrat parfois c'est difficile parce qu'on recrute beaucoup les fonctionnaires d'État en 2e cycle et lycée ». Déjà en place depuis la phase 4 : l'enseignant (titulaire ou vacataire, un compte facultatif), ses affectations aux matières des classes, et un vacataire peut enseigner dans plusieurs écoles (T-24).

### T-39 — Qui travaille dans l'école

Quelles catégories de personnel une école tient-elle dans son registre ?

*Recommandation* : tout le monde, pas seulement les enseignants : enseignants, direction et administration (directeur, censeur, secrétaire, comptable, caissier, surveillant), personnel d'appui (gardien, agent d'entretien, chauffeur, cuisinière). Chaque personne a une fiche ; un enseignant de la phase 4 devient une personne du personnel sans rien ressaisir.

*Réponse* (porteur du projet, 14 octobre 2026) : **on ne peut pas être complet, on laisse de la souplesse.** Conséquence : aucune liste fermée ; GESCO propose des catégories et des fonctions que l'école renomme, complète ou retire, et une fonction peut s'écrire librement.

### T-40 — Fonctionnaires de l'État qui enseignent à l'école

Qu'est-ce qui lie à l'école un fonctionnaire qui y donne des heures en 2e cycle ou au lycée ?

*Recommandation* : pas un contrat de travail, mais un **engagement de vacation pour l'année** : les matières et classes (ses affectations), le taux de l'heure, le nombre d'heures prévu par semaine, la date de début ; on note qu'il est fonctionnaire et son service d'origine, sans rien exiger. L'école peut imprimer une lettre d'engagement si elle le veut ; rien ne l'y oblige. L'engagement se renouvelle d'un clic à la rentrée suivante.

*Réponse* (porteur du projet, 14 octobre 2026) : recommandation retenue, **sans le service d'origine**, qu'on laisse de côté. On garde l'indication « fonctionnaire de l'État » et son numéro, facultatif.

### T-41 — Paiement des heures de vacation

Comment se comptent les heures payées à un vacataire ?

*Recommandation* : chaque mois, un **état des heures** par vacataire : heures prévues d'après ses affectations, heures faites saisies par la direction (cours manqués, heures de rattrapage), visé par la direction. La phase 7 paie ces heures au taux de l'engagement ; la phase 8 pourra préremplir l'état depuis l'emploi du temps.

*Réponse* (porteur du projet, 14 octobre 2026) : recommandation retenue.

### T-42 — Personnel permanent

Le personnel permanent (titulaires de l'école, administration, appui) a-t-il un contrat écrit ?

*Recommandation* : on enregistre l'**engagement** de chacun, écrit ou non : nature (contrat à durée indéterminée, à durée déterminée, stage, bénévole), dates, salaire de base, fonction. Le contrat imprimé est possible, jamais exigé. Un engagement à durée déterminée qui arrive à son terme est signalé deux mois avant.

*Réponse* (porteur du projet, 14 octobre 2026) : recommandation retenue.

### T-43 — Présence du personnel

Comment l'école suit-elle la présence de son personnel ?

*Recommandation* : pas de pointage quotidien de chacun ; on note les **absences et retards** du personnel (avec motif, justifiés ou non), comme pour les élèves. Pour les vacataires, l'état des heures (T-41) suffit.

*Réponse* (porteur du projet, 14 octobre 2026) : recommandation retenue.

### T-44 — Congés et permissions

Quels congés et permissions l'école accorde-t-elle ?

*Recommandation* : congé annuel (souvent pris pendant les vacances scolaires), permission d'absence de quelques jours, congé de maladie, congé de maternité. Chacun est demandé, accordé ou refusé par la direction, avec ses dates ; le décompte des jours acquis est un réglage de l'école, désactivé par défaut.

*Réponse* (porteur du projet, 14 octobre 2026) : recommandation retenue.

### T-45 — Dossier de chaque personne

Que contient le dossier d'un membre du personnel ?

*Recommandation* : identité, contacts, personne à prévenir, diplômes et qualifications, pièces jointes (pièce d'identité, diplômes, engagement signé), numéro de sécurité sociale (INPS) et numéro de fonctionnaire s'il en a, coordonnées pour le paiement (espèces, mobile money, compte bancaire).

*Réponse* (porteur du projet, 14 octobre 2026) : recommandation retenue.

### T-46 — Qui voit et gère le personnel

Qui, à l'école, gère le personnel et voit les salaires ?

*Recommandation* : la direction (fondateur, directeur) gère tout ; le comptable voit les salaires pour la paie ; le secrétariat tient les fiches sans voir les montants ; les enseignants ne voient que leur propre fiche. Chaque modification est tracée.

*Réponse* (porteur du projet, 14 octobre 2026) : recommandation retenue.

### T-47 — Cantine scolaire

La cantine sera un module que chaque école active ou non (réponse du 14 octobre 2026). Que doit-elle couvrir, et à quel moment la faire ?

*Recommandation* : avec la phase 8 (dépenses), puisque la cantine, ce sont surtout des frais d'un côté et des achats de vivres de l'autre : l'élève abonné à la cantine pour le mois ou le trimestre (frais facturés comme les autres, déjà possibles), la liste des abonnés du jour, les repas pris si l'école les compte, les achats de vivres en dépenses, et le personnel de cuisine dans le registre de la phase 6.

*Réponse* (porteur du projet, 14 octobre 2026) : recommandation retenue.

### T-48 — Lycées techniques et professionnels

Question du porteur du 14 octobre 2026 : que faudrait-il si un lycée technique ou un centre de formation professionnelle voulait le logiciel ?

*Réponse de conception* : inscriptions, frais, caisse, impayés, absences et personnel conviennent tels quels. Il manque un ordre d'enseignement technique et professionnel avec ses niveaux et ses filières (aujourd'hui les cycles sont fixés : jardin, 1er cycle, 2e cycle, lycée), les notes de pratique et d'atelier, le stage en entreprise et son évaluation, et les bulletins et relevés propres à ses diplômes. *À préciser avec une école de ce type* : diplômes préparés (CAP, BT, bac technique), durée des formations, filières, périodes (trimestres ou semestres), place de la pratique et du stage dans la moyenne.

*Réponse* (porteur du projet, 14 octobre 2026) : **on inclut le CAP et le BT, avec des semestres ou des trimestres au choix de l'école.** Conséquence : la phase 6 ajoute l'enseignement technique et professionnel (`SPEC_PHASE_6.md`, partie B) ; durées, filières et place de la pratique deviennent des réglages de l'école, avec des défauts à confirmer sur les captures.

### T-49 — Versements entre deux tranches

Remarque du porteur du projet du 15 octobre 2026 : les parents alternent entre le paiement d'une tranche et de petites sommes (« je donne 5 000 aujourd'hui »).

*Réponse* (porteur du projet, 15 octobre 2026) : **ce que fait déjà GESCO suffit** : tout montant s'encaisse n'importe quel jour, s'impute sur l'échéance la plus ancienne, et le reçu porte le reste dû. Ni plan de paiement libre, ni changement de plan après des versements.

### T-50 — Matières sans note sur le bulletin

Remarque du porteur du projet du 15 octobre 2026 : « on ne doit pas polluer le bulletin si une matière n'est pas enseignée aux différents cycles ».

*Réponse appliquée* : une matière qu'aucun élève de la classe n'a eue sur la période (pas enseignée à ce niveau, ou pas encore évaluée) ne s'imprime plus ; un élève seul sans note dans une matière que les autres ont eue la garde, « non noté » (Q-59). À confirmer sur les captures.

## Phase 7 — paie

Posées le 15 octobre 2026, après la validation de la phase 6. Déjà en place : le registre du personnel, le moyen de paiement de chacun (espèces, mobile money, banque), les engagements avec leur salaire de base ou leur taux de l'heure, l'état mensuel des heures des vacataires visé par la direction, les absences et congés du personnel, et les droits `payroll:calculate`, `payroll:validate`, `payroll:pay` (Gestionnaire RH, Directeur). Règle du blueprint (§8.7) : aucune règle fiscale ou sociale écrite dans le code ; toute retenue est un élément de paie que l'école règle, avec sa date d'effet.

### T-51 — Quand l'école paie

Le personnel est-il payé une fois par mois ? À quel moment (fin du mois, début du mois suivant) ? Les vacataires en même temps que les permanents ?

*Recommandation* : une paie par mois, arrêtée quand le mois est fini ; les vacataires sur les heures visées du mois, dans la même paie ; chaque membre payé selon le moyen de sa fiche.

*Réponse* (porteur du projet, 15 octobre 2026) : **en fin de mois, jusqu'au 5 du mois suivant.** La paie d'un mois se prépare quand il finit et se paie jusqu'au 5 ; les vacataires y entrent sur leurs heures visées.

### T-52 — Mois payés dans l'année

Les permanents sont-ils payés douze mois sur douze ? Les enseignants engagés pour l'année scolaire sont-ils payés pendant les grandes vacances ? Les vacataires seulement pour les heures faites ?

*Recommandation* : chaque mois couvert par un engagement mensuel se paie (douze mois pour une durée indéterminée) ; un engagement limité à l'année scolaire ne paie pas les vacances ; un vacataire n'est payé que de ses heures visées.

*Réponse* (porteur du projet, 15 octobre 2026) : **parfois neuf mois, douze dans les écoles sérieuses.** Les mois payés varient d'une école à l'autre, et d'un engagement à l'autre : réglage de l'école, repris par chaque engagement mensuel.

### T-53 — Retenues obligatoires

Quelles retenues l'école fait-elle sur les salaires : cotisation INPS du salarié, AMO, impôt sur les traitements et salaires (ITS) ? Sur qui : permanents, vacataires, fonctionnaires vacataires ? L'école déclare-t-elle aussi sa part patronale ? Souhaitez-vous que GESCO propose les taux en vigueur au Mali, à vérifier par l'école ?

*Recommandation* : chaque retenue est un élément que l'école règle (nom, taux ou barème par tranches, plafond, à qui elle s'applique, date d'effet) ; la part patronale se calcule pour l'état de déclaration sans jamais être retenue au salarié ; GESCO livre ces éléments vides, et l'école ou son comptable y met les taux.

*Réponse* (porteur du projet, 15 octobre 2026) : **pas dans toutes les écoles.** Les retenues sont facultatives : livrées inactives et sans taux, l'école active celles qu'elle pratique.

### T-54 — Primes, indemnités et heures supplémentaires

Qu'est-ce qui s'ajoute au salaire de base : prime de transport, de logement, d'ancienneté, de responsabilité (censeur, surveillant général), prime d'examen, treizième mois, heures supplémentaires d'un permanent ?

*Recommandation* : un catalogue de primes et d'indemnités propre à l'école ; chacune attribuée à un membre avec son montant et sa date d'effet, versée chaque mois ou seulement le mois où on la saisit ; les heures supplémentaires d'un permanent au taux fixé dans son engagement.

*Réponse* (porteur du projet, 15 octobre 2026) : **rarement.** Une prime se saisit surtout dans la paie du mois où on la donne ; l'attribution d'une prime chaque mois reste possible, sans plus.

### T-55 — Avances sur salaire et prêts

L'école donne-t-elle des avances sur salaire, ou des prêts remboursés sur plusieurs mois ?

*Recommandation* : une avance se note avec son montant et le nombre de mois de remboursement ; GESCO la retient chaque mois jusqu'à ce qu'elle soit soldée, et le bulletin de paie montre ce qui reste dû.

*Réponse* (porteur du projet, 15 octobre 2026) : **oui.** Recommandation retenue.

### T-56 — Absences et salaire

Une absence non justifiée d'un permanent est-elle retirée de son salaire ? Et les retards ?

*Recommandation* : jamais d'office : au calcul de la paie, la direction voit les absences non justifiées du mois de chacun et décide d'une retenue (au trentième du salaire par jour, par défaut), qu'elle confirme ligne par ligne.

*Réponse* (porteur du projet, 15 octobre 2026) : recommandation retenue.

### T-57 — Bulletin de paie et émargement

Remettez-vous un bulletin de paie à chaque membre du personnel ? Faites-vous signer un état « pour acquit » au moment du paiement ?

*Recommandation* : un bulletin de paie imprimable pour chacun (gains, retenues, net à payer, cumuls de l'année) ; un état de paie du mois avec une ligne par personne et une colonne de signature ; un état des retenues par organisme pour les déclarations.

*Réponse* (porteur du projet, 15 octobre 2026) : recommandation retenue.

### T-58 — L'argent des salaires

L'argent des salaires sort-il de la caisse de l'école, celle des frais de scolarité ? Le salaire payé doit-il compter dans les dépenses de l'école ?

*Recommandation* : oui (Q-06) : un salaire payé en espèces est une sortie de la caisse ouverte, tracée ; un paiement par mobile money ou par banque note sa référence ; chaque salaire payé devient une dépense quand la phase 8 les tiendra, par un événement explicite.

*Réponse* (porteur du projet, 15 octobre 2026) : **oui, sans blocage.** Le salaire payé sort de la caisse et compte dans les dépenses, mais rien n'empêche de payer : ni une caisse fermée, ni un tiroir qui ne suffit pas.

### T-59 — Salaires payés en plusieurs fois

Arrive-t-il que l'école paie une partie du salaire, puis le reste plus tard (fin de mois difficile, frais de scolarité pas encore rentrés) ?

*Recommandation* : le net d'un bulletin validé peut se payer en plusieurs fois ; le reste à payer de chacun se suit d'un mois sur l'autre, et le tableau de bord montre les salaires encore dus.

*Réponse* (porteur du projet, 15 octobre 2026) : **oui.** Recommandation retenue.

### T-60 — Qui calcule, qui valide, qui paie

Qui prépare la paie à l'école (comptable, gestionnaire du personnel, direction) ? Qui la valide ? Qui remet l'argent ?

*Recommandation* : le comptable ou le gestionnaire du personnel calcule (`payroll:calculate`), la direction valide (`payroll:validate`), la caisse paie (`payroll:pay`) ; une paie validée ne se recalcule plus, une erreur se corrige le mois suivant par une régularisation (§10.6).

*Réponse* (porteur du projet, 15 octobre 2026) : **le directeur aussi** ; parfois une école n'est tenue que par son directeur et une secrétaire qui n'a pas d'ordinateur. Le Directeur calcule, valide et paie ; la paie d'un mois doit se faire en quelques gestes.

*Remarque du porteur* (15 octobre 2026) : au 2e cycle et au lycée, les enseignants sont payés à l'heure dans la plupart des cas. Les vacataires sont donc le cas courant de la paie.

## Phase 8 — dépenses, trésorerie, emploi du temps

Posées le 16 octobre 2026, avec celles de la phase 10, quand le porteur du projet a demandé de mener ensemble la fin des phases 7, 8 et 10. Déjà décidés : la journée de chaque école et l'horaire de canicule du 1er cycle (T-38), la cantine activable par école (T-47), le salaire payé qui devient une dépense (T-58).

### T-61 — Approbation des dépenses

Une dépense doit-elle être approuvée avant d'être payée ?

*Recommandation* : au-dessus d'un montant réglé par chaque école, la dépense attend l'accord du directeur ou du fondateur ; en dessous, le comptable ou le caissier paie directement ; un seuil à zéro supprime l'accord.

*Réponse* (porteur du projet, 16 octobre 2026) : recommandation retenue.

### T-62 — Comptes de l'école

Où l'école garde-t-elle son argent ?

*Recommandation* : la caisse, chaque compte en banque et chaque compte de mobile money, chacun avec son solde ; un dépôt de la caisse à la banque se note comme un transfert ; les lignes se cochent contre le relevé de la banque ou de l'opérateur.

*Réponse* (porteur du projet, 16 octobre 2026) : recommandation retenue.

### T-63 — Comptabilité

Quelle comptabilité pour la phase 8 ?

*Recommandation* : une comptabilité de trésorerie : recettes et dépenses par catégorie et par mois, solde de chaque compte, budget prévu et réalisé (avertissement, jamais de blocage), export Excel pour le comptable. Le plan SYSCOHADA attendra une école qui le demande.

*Réponse* (porteur du projet, 16 octobre 2026) : recommandation retenue.

### T-64 — Emploi du temps

Comment l'emploi du temps se fait-il ?

*Recommandation* : la direction place les séances dans la grille de chaque classe, dans les horaires de l'école ; GESCO signale un enseignant à deux endroits à la fois sans refuser, et imprime l'emploi du temps par classe et par enseignant. Pas de génération automatique.

*Réponse* (porteur du projet, 16 octobre 2026) : recommandation retenue.

## Phase 10 — clôture de l'année, installeur, licences

Posées le 16 octobre 2026. Le porteur du projet installe GESCO le jour même sur un poste d'essai, pour une école qui a un jardin (petite, moyenne et grande section) et un 1er cycle.

### T-65 — Décisions de fin d'année

Comment se posent les décisions de fin d'année (passe, redouble, exclu) ?

*Recommandation* : GESCO propose « passe » à partir de la moyenne de passage réglée par l'école et « redouble » en dessous ; la direction confirme ou change chaque élève après le conseil ; au jardin, chaque enfant passe à la section suivante.

*Réponse* (porteur du projet, 16 octobre 2026), dictée : **c'est le CAP ou l'académie qui décide à la fin de ce qui doit passer, mais la décision nous revient d'une façon : on la valide dans l'application.** Conséquence : la moyenne de passage est un réglage de l'école, qui y reporte la consigne du CAP ou de l'académie ; GESCO en tire une proposition, la direction valide ou corrige chaque élève, et rien ne passe sans sa validation (Q-30).

### T-66 — Réinscription

Comment les élèves se réinscrivent-ils pour l'année suivante ?

*Recommandation* : GESCO prépare chaque élève dans la classe qui suit sa décision ; la secrétaire valide quand le parent vient, sans rien ressaisir, ou une classe entière d'un clic ; la facture de la nouvelle année se crée à la validation.

*Réponse* (porteur du projet, 16 octobre 2026) : recommandation retenue.

### T-67 — Impayés en fin d'année

Que devient ce qu'un élève doit encore à la fin de l'année ?

*Recommandation* : un arriéré qui suit l'élève dans la nouvelle année, visible au guichet et sur sa fiche, sans bloquer la réinscription ; un versement règle l'arriéré d'abord.

*Réponse* (porteur du projet, 16 octobre 2026) : recommandation retenue.

### T-68 — Paiements d'avance

Un parent peut-il payer avant la rentrée les frais de l'année suivante ?

*Recommandation* : oui, dès que l'année suivante est préparée : le versement est gardé en avance au nom de l'élève et s'applique à sa première facture de l'année suivante.

*Réponse* (porteur du projet, 16 octobre 2026) : recommandation retenue.

### T-69 — Licences

Comment les licences s'installent-elles, et que fait GESCO sans licence ?

*Réponse* (porteur du projet, 16 octobre 2026) : **cent licences libres et numérotées**, GESCO-0001 à GESCO-0100 : chacune s'installe sur n'importe quel poste, qui s'y attache ; rien n'empêche hors ligne d'installer deux fois le même fichier, mais chaque numéro est tracé dans le journal du poste. **Sans licence, tout marche**, un bandeau le rappelle (§26.5). La paie, inachevée, se décide à la fin de la phase 10 pour la version remise à l'école d'essai.

### T-70 — Sanctions

L'exclusion temporaire et la radiation existent depuis la phase 4. Que faut-il ajouter pour la discipline ?

*Recommandation* : un dossier de discipline par élève : avertissement, blâme, retenue et convocation des parents, chacun daté, motivé, avec qui l'a décidé ; l'avertissement et le blâme du conseil de classe paraissent sur le bulletin de la période.

*Réponse* (porteur du projet, 17 octobre 2026) : recommandation retenue.

### T-71 — Absences par séance

L'école relève-t-elle les absences à chaque séance ?

*Recommandation* : la demi-journée reste la règle (T-37) ; une école qui le veut, surtout un lycée, active le relevé par séance d'après l'emploi du temps. Absent à toutes les séances d'une demi-journée, l'élève compte une demi-journée sur le bulletin.

*Réponse* (porteur du projet, 17 octobre 2026) : recommandation retenue.

### T-72 — Relevés des comptes

Comment importer les relevés d'Orange Money, Wave, Moov ou de la banque ?

*Recommandation* : tout fichier Excel ou CSV ; l'école indique une fois, pour chaque compte, les colonnes de la date, du montant et de la référence ; GESCO coche les lignes du livre qu'il retrouve par la référence, puis par le montant et la date, et montre les autres.

*Réponse* (porteur du projet, 17 octobre 2026) : recommandation retenue.

### T-73 — Archive du journal d'audit

À la clôture de l'année, où archiver le journal ?

*Recommandation* : dans un fichier signé, rangé avec la sauvegarde de clôture et copié sur ses destinations ; le journal reste aussi dans la base, et GESCO vérifie la signature d'une archive.

*Réponse* (porteur du projet, 17 octobre 2026) : recommandation retenue.

## Phase 5 — SMS aux familles, remises, guichet

Posées le 17 octobre 2026, à l'ouverture de la version 2, après la validation de la phase 8 et de la version 1.1. Déjà décidés au blueprint : le canal abstrait et sa file persistante (§25.3), la translittération et les contraintes du modem (§25.3bis), les données exposées par SMS (§8.5bis), le consentement par type de message et les sanctions jamais envoyées par automate (§25.6).

### T-74 — Matériel d'envoi

Pour envoyer les SMS, quel matériel l'école branchera-t-elle sur le poste ?

*Recommandation* : une clé modem GSM USB qui expose un port série en commandes AT, avec une puce prépayée de l'école : coût marginal, sans internet. N'en essayer qu'une avant d'en acheter plusieurs ; la passerelle d'un opérateur reste un repli possible plus tard.

*Réponse* (porteur du projet, 17 octobre 2026) : recommandation retenue — clé modem GSM.

### T-75 — Messages envoyés d'eux-mêmes

Quels messages GESCO envoie-t-il de lui-même ?

*Recommandation* : bulletin à retirer, relance d'impayé, absence du jour. Les convocations et les sanctions ne partent jamais seules : GESCO prépare le message, quelqu'un le relit et décide de l'envoyer.

*Réponse* (porteur du projet, 17 octobre 2026) : recommandation retenue — bulletin à retirer, relance d'impayé, absence du jour.

### T-76 — Langue des SMS

En quelle langue partent les SMS ?

*Recommandation* : français sans accents (alphabet GSM-7) : 160 caractères par SMS au lieu de 70, donc moitié moins de crédit. Une autre langue, le bambara par exemple, pourra s'ajouter par un autre catalogue.

*Réponse* (porteur du projet, 17 octobre 2026) : recommandation retenue.

### T-77 — Consultation par SMS

Les parents pourront-ils interroger GESCO par SMS avec un code confidentiel (« SOLDE code », « ABS code », « NOTE code ») ?

*Recommandation* : oui, en fin de phase 5, une fois l'envoi essayé avec une vraie clé ; code propre à chaque responsable, croisé avec son numéro, quota par jour, chaque consultation au journal.

*Réponse* (porteur du projet, 17 octobre 2026) : recommandation retenue.
