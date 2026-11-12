# SPEC_PHASE_11_B.md — Évaluation par compétences et livret du jardin d'enfants

**Projet** : ERP de gestion scolaire — Tauri 2 + Rust + React/TypeScript
**Phase** : 11, lot B — domaines et compétences du jardin, observations, livret imprimé, publié et remis ; dans la version 2.0.0 s'il est prêt à temps
**Statut** : v1.0 — proposée le 12 novembre 2026, à soumettre au porteur du projet avec les questions T-109 à T-115 (§12) ; rien n'est codé
**Documents parents** : `docs/plan-phase-11.md` (§2.3, T-84 répondue le 10 novembre 2026, décision 10 du §4), `PROJECT_BLUEPRINT.md` (ADR-013, §8.4 B, révision du 7 octobre 2026), `SPEC_PHASE_4.md` (bulletins, appréciations), `SPEC_PHASE_5.md` (remise des bulletins, avis par SMS, consultation), `SPEC_PHASE_11_A.md` §7 (saisie des notes par les maîtres), `ETAT_AVANCEMENT.md` (décisions du porteur du 12 novembre 2026 : ne jamais bloquer, ne pas faire remarquer ce qui manque)
**Date** : 12 novembre 2026

---

## 0. Objet et critère de fin

### 0.1 Ce que produit ce lot

Une école qui a un jardin d'enfants remet à chaque famille, à la fin de chaque période, un
**livret** : pour chaque domaine (langage, motricité, vivre ensemble, découverte du monde, arts),
les compétences de la section de l'enfant, chacune « acquis », « en cours » ou « non acquis »,
et une appréciation générale. Aujourd'hui le jardin s'inscrit, paie, a ses décisions de fin
d'année, mais n'a aucun bulletin.

À la fin du lot :
- GESCO livre une liste de domaines et de compétences par section (petite, moyenne, grande),
  que l'école modifie : elle retire, ajoute, renomme, réordonne ;
- la maîtresse ou la direction saisit les observations d'une classe entière au clavier, en
  quelques minutes par domaine ;
- le livret se prépare, se valide et se publie comme un bulletin ; il s'imprime en A4, lisible en
  noir et blanc et en photocopie ;
- sa publication le met « à remettre » et envoie l'avis par SMS, comme pour un bulletin ;
- rien ne bloque : un enfant sans observation, sans photo, sans téléphone de responsable, ou
  dont les frais ne sont pas réglés a son livret comme les autres, et rien ne signale ce qui
  manque.

### 0.2 Décisions déjà prises qui s'appliquent

- **Contenu du livret** (T-84) : par domaine, chaque compétence sur trois niveaux, chaque
  trimestre par défaut (réglable) ; une liste livrée que l'école modifie ; une appréciation
  générale. Pour le jardin seulement : le 1er cycle garde ses notes.
- **Mode d'évaluation** (ADR-013) : porté par le niveau ; les trois niveaux du jardin (PS, MS,
  GS) sont livrés en « compétences » depuis la phase 2.
- **Ne jamais bloquer, ne pas faire remarquer ce qui manque** (porteur du projet, 12 novembre
  2026, pour le dossier des enseignants, les frais et la paie) : appliqué ici aux observations,
  aux appréciations, à la photo, aux responsables et aux frais (§6).
- **Saisie par les maîtres** (T-82, `SPEC_PHASE_11_A.md` §7) : éteinte à la livraison ; allumée
  par le réglage `notes.saisie_maitres`, elle vaut aussi pour les observations, dans les seules
  classes du maître.
- **Version** (`docs/plan-phase-11.md` §5) : le lot entre dans la 2.0.0 s'il est validé avant sa
  sortie ; sinon il attend la version suivante sans rien retarder.
- R-14 : rien ne se supprime ; une compétence se retire, une observation se vide, un livret
  publié se corrige par une nouvelle version. R-21 : la garde reste dans
  `Application::executer`.
- Interface : aucun texte affiché ne cite de question de terrain, de spécification ou de date de
  réponse.

### 0.3 Critère de fin, vérifiable

Les critères d'acceptation du §9 sont tous tenus, prouvés par un essai automatique ou une
capture, et le porteur du projet a validé le lot sur l'école d'essai.

---

## 1. Périmètre

**Dans le lot**
- Domaines et compétences par section du jardin : liste livrée, modifiable par l'école.
- Périodes du jardin : trois trimestres par défaut, deux au choix de l'école.
- Observations : une par enfant, par compétence et par période ; commentaire facultatif par
  domaine ; appréciation générale.
- Livret : préparation, validation, publication, correction, impression (un enfant ou toute la
  classe), remise, avis par SMS, bulletins non retirés.
- Droits, matrice des permissions, école d'essai avec un jardin évalué, manuel, captures.
- Mise à niveau d'une base qui a déjà des enfants au jardin.

**Hors du lot**
- Les compétences au 1er cycle et au-delà (T-84) : le mode « compétences » reste réservé au
  jardin ; l'écran des niveaux ne permet pas de basculer un autre niveau.
- Une appréciation proposée d'office d'après les observations (celle des notes, migration 0049,
  ne s'applique pas au jardin).
- Un livret dont l'école dessine elle-même la mise en page ; l'en-tête, le logo et la qualité du
  signataire suivent les réglages existants des bulletins.
- Le livret en ligne (lot D) et l'envoi du PDF par SMS ou WhatsApp.
- Les ressources d'apprentissage du préscolaire (phase 12).

---

## 2. Ce que l'application possède déjà

Relevé dans le code au 12 novembre 2026 (migrations 0001 à 0050), en lecture seule.

| Élément | Où | Ce que le lot en fait |
|---|---|---|
| Niveaux du jardin | `gesco-core/src/scolarite/academique.rs` : `Cycle::Jardin`, niveaux livrés PS, MS, GS, `ModeEvaluation::Competences` posé d'office au jardin | Sert tel quel à choisir l'écran (notes ou observations) et la forme du document (bulletin ou livret) |
| Périodes | Table `periode_academique` (par niveau et par année) ; `gesco-core/src/pedagogie/periode.rs` : `calendrier_par_defaut` rend « rien » au jardin, `verifier_decoupage` refuse tout découpage au jardin ; `gesco-services/src/pedagogie.rs::poser_annee` saute le jardin | Le jardin reçoit ses périodes : trois trimestres par défaut, deux au choix (§3.3) |
| Matières et programme | `import_programme.rs` refuse un programme au jardin (« jardin sans notes ») | Inchangé : le jardin n'a ni matière ni coefficient ; les compétences ne sont pas des matières |
| Bulletin | Table `bulletin` (migration 0019) : par inscription et période, versions, statuts `calcule`, `valide`, `publie`, `remplace`, contenu figé en JSON, `moyenne` et `rang` facultatifs, déclencheurs « ne se supprime pas » et « publié, ne change plus » ; `ServiceBulletins` (`calculer`, `apprecier`, `valider`, `publier`, `corriger`, `a_imprimer`, `reglages_impression`) | Le livret **est** un bulletin : même table, même cycle de vie, contenu d'une autre forme (§3.4) |
| Appréciation | Table `appreciation` (migration 0019, colonne `automatique` en 0049) : par inscription, période et `matiere_cle` (« generale » pour la direction), 300 caractères | Porte l'appréciation générale du livret (`generale`) et le commentaire facultatif par domaine (`domaine:<id>`) |
| Remise | Table `remise_document` (migration 0046), `remises_documents::a_la_publication`, liste des non retirés | Un livret publié naît « à remettre », se remet, se signe, devient « non retiré », sans rien de nouveau |
| Avis par SMS | `messages_auto::avis_bulletin` (réglage `sms.auto_bulletin`, nature `Bulletin`, une fois par élève et par période), `rappel_bulletin` | Part à la publication du livret ; le texte dit « livret » (§6, RB-12) |
| Consultation par SMS | `consultation.rs` (solde, absences, moyenne) | Pour un enfant du jardin, la réponse ne parle pas de moyenne et ne dit pas qu'il n'en a pas |
| Décisions de fin d'année | `fin_annee.rs` : décisions du jardin sans barème ni moyenne de passage | Le livret de la dernière période porte la décision quand elle est posée |
| Portée des maîtres | `PorteeNotes` (`gesco-core/src/pedagogie`), réglage `notes.saisie_maitres` (lot A, étape 5) | Étendue aux observations, par classe (§7) |
| Droits | `GRADES_CREATE`, `GRADES_VALIDATE`, `REPORTS_GENERATE`, `REPORTS_PUBLISH`, `ACADEMIC_YEAR_MANAGE` | Réutilisés : aucun droit nouveau (§7) |

Ce qui n'existe pas : les entités `DomaineApprentissage`, `Competence` et `Observation` du §8.4 B
du blueprint, reportées le 7 octobre 2026 ; tout écran derrière elles.

---

## 3. Données nouvelles

Une migration à l'étape 1, numérotée à la suite de la dernière au moment de l'étape (0050 le
12 novembre 2026 ; la phase 9 peut en prendre avant), avec son retour arrière. Chaque table porte
`etablissement_id`, `cree_le`, `cree_par`, `mis_a_jour_le`, `mis_a_jour_par`,
`identifiant_poste`, comme les autres. La forme exacte est fixée par l'ADR de la décision 10
(§10, étape 1).

### 3.1 Domaines et compétences

**`domaine_competence`** — un domaine du jardin.
- `id`, `libelle` (60 caractères au plus), `ordre`, `livre` (vrai pour les cinq livrés),
  `retire_le` (vide tant qu'il sert).
- Commun aux trois sections ; un domaine retiré disparaît des écrans et des livrets à venir.

**`competence`** — une compétence d'une section.
- `id`, `domaine_id`, `niveau_id` (PS, MS ou GS de l'école), `libelle` (120 caractères au
  plus), `ordre`, `livree` (vrai pour celles de la liste livrée), `retiree_le`.
- Pas d'année scolaire : la liste vaut d'une année à l'autre. Un livret fige au moment où il se
  prépare le libellé de chaque compétence ; renommer ou retirer une compétence ne change donc
  aucun livret déjà préparé.
- Une compétence ne se supprime pas (R-14) ; retirée, elle garde ses observations passées.
- La liste livrée est rédigée à neuf d'après le programme des jardins d'enfants (annexe A) ; rien
  n'y est recopié d'un document.

### 3.2 Observations

**`observation`** — clé (`inscription_id`, `competence_id`, `periode_id`).
- `niveau` : `acquis`, `en_cours`, `non_acquis`, ou vide (observation vidée) ;
- `saisie_le`, `saisie_par`, `identifiant_poste`.
- Pas de ligne = pas d'observation. Une ligne ne se supprime pas (déclencheur, comme `note`) :
  vider remet `niveau` à vide, l'audit garde l'ancienne valeur.
- Le commentaire par domaine et l'appréciation générale vont dans la table `appreciation`
  existante, `matiere_cle` = `domaine:<id>` ou `generale`.

### 3.3 Périodes du jardin

- `calendrier_par_defaut` donne au jardin le même calendrier que le 2e cycle : trois trimestres
  (octobre à décembre, janvier à mars, avril à juin), ou deux périodes si l'école en choisit
  deux (T-112).
- `verifier_decoupage` accepte au jardin deux ou trois trimestres ; l'écran des périodes, qui
  existe, le propose comme ailleurs.
- `poser_annee` pose désormais les périodes du jardin à l'ouverture d'une année.
- Mise à niveau : pour l'année ouverte, les périodes du jardin sont posées si elles manquent,
  sans toucher à celles des autres cycles ni aux années closes.

### 3.4 Le livret est un bulletin

- Une ligne de `bulletin` par enfant et par période, comme au fondamental ; `moyenne` et `rang`
  restent vides ; `regle_version` vaut 0.
- Le `contenu` JSON porte `"forme": "livret"` et tout ce qui s'imprime, figé à la préparation :
  identité, classe, section, période, domaines dans leur ordre, pour chacun les compétences
  observées avec leur niveau pour chaque période déjà passée (T-111), le commentaire du domaine,
  l'appréciation générale, les absences si l'école les relève (RB-9), la décision de fin d'année
  au dernier livret si elle est posée.
- Rien de nouveau dans les statuts, les versions, la remise, l'avis ni les non retirés : c'est
  ce qui rend le lot petit.
- À vérifier à l'étape 3 : que rien de ce qui lit `bulletin` (moyenne annuelle, fin d'année,
  consultation par SMS, tableau de bord, rapports de la phase 9) ne trébuche sur une moyenne vide
  ou ne compte un livret comme un bulletin à moyenne.

### 3.5 Réglages

| Clé | Valeur livrée | Rôle |
|---|---|---|
| `jardin.echelle` | « Acquis », « En cours », « Non acquis » | Libellés des trois niveaux, que l'école peut reformuler (T-110) ; trois niveaux toujours |
| `jardin.commentaire_domaine` | allumé | Montre la ligne de commentaire par domaine à la saisie et à l'impression |
| `sms.auto_bulletin` | existant | Vaut pour le livret comme pour le bulletin |

### 3.6 Événements et audit

Chaque écriture passe par l'unité de travail : audit et événement dans la même transaction.
Événements nouveaux : `CompetenceModifiee` (création, modification, retrait d'un domaine ou
d'une compétence) et `ObservationSaisie` (par lot : une saisie de grille donne un événement par
enfant et par domaine, pas un par case). La publication reste `BulletinPublie`.

---

## 4. Écrans

Aucun écran nouveau dans la barre latérale : les trois écrans existants (paramètres, saisie,
bulletins) changent de contenu quand la classe ou le niveau est du jardin.

### 4.1 « Compétences du jardin » (Paramètres)

- Onglet par section (PS, MS, GS), seules celles que l'école a ouvertes.
- Domaines dans leur ordre, compétences dessous ; ajouter, renommer, monter, descendre, retirer ;
  « Afficher les retirées » pour en remettre une.
- « Recopier depuis une autre section » : prend les compétences d'une section pour partir d'elles.
- Les libellés de l'échelle (§3.5) et le commentaire par domaine se règlent ici.
- Pas de bouton « remettre la liste livrée » qui effacerait le travail de l'école : les
  compétences livrées retirées se remettent une à une.

### 4.2 Saisie des observations

L'écran des notes, ouvert sur une classe du jardin, devient « Observations ».
- Choix de la période et du domaine ; grille : une ligne par enfant (nom, photo en vignette si
  elle existe), une colonne par compétence du domaine.
- Clavier : `A`, `E`, `N` (initiales des libellés réglés, ou `1`, `2`, `3`) posent le niveau et
  passent à la case suivante ; `Suppr` vide ; flèches et `Entrée` comme dans la grille des notes.
- « Toute la colonne » : pose un niveau à tous les enfants d'une compétence, la maîtresse corrige
  ensuite les exceptions. C'est ce qui rend la saisie rapide pour une classe de quarante.
- Sous la grille, pour l'enfant sélectionné : commentaire du domaine (si allumé) et appréciation
  générale, 300 caractères chacun.
- Enregistrement à la sortie de la case, comme les notes ; refus si la fiche a changé depuis sa
  lecture sur un autre poste (règle du lot A, §6.1), avec la valeur de l'autre en clair.
- Les cases vides sont simplement vides : pas de compteur de manques, pas de couleur d'alerte,
  pas de message à la sortie.
- Une période verrouillée se lit sans se modifier, comme pour les notes.

### 4.3 Livrets de la classe

L'écran des bulletins, ouvert sur une classe du jardin, montre des livrets.
- « Préparer les livrets » (le calcul des bulletins), « Valider », « Publier », « Corriger »,
  « Imprimer la classe », « Imprimer » pour un enfant : mêmes boutons, mêmes états.
- Aperçu du livret d'un enfant avant validation.
- Aucun bouton ne dépend d'une condition sur les observations (RB-1).

### 4.4 Ailleurs

- Fiche de l'enfant, onglet scolarité : ses livrets publiés, comme les bulletins d'un élève.
- Remise des documents et bulletins non retirés : les livrets y figurent, sous le mot « livret ».
- Tableau de bord : rien de nouveau, aucune alerte de livrets à faire.

---

## 5. Impression du livret

- A4 portrait, par `gesco-docs`, comme le bulletin ; en-tête de l'école, logo et qualité du
  signataire tirés des réglages d'impression existants.
- Haut de page : nom et prénoms de l'enfant, date de naissance, classe et section, année
  scolaire, période ; photo si elle existe, sinon rien à sa place (pas de cadre vide).
- Corps : un bloc par domaine. Chaque compétence observée sur une ligne, avec une colonne par
  période de l'année déjà passée (T-111) ; dans chaque colonne, le niveau (T-110) écrit en toutes
  lettres ou marqué d'une croix sous son en-tête, plus un pictogramme noir sur blanc (rond plein,
  demi-rond, rond vide) pour les parents qui lisent peu. Jamais la couleur seule.
- Une compétence jamais observée dans l'année ne s'imprime pas ; un domaine sans aucune
  compétence observée ne s'imprime pas ; aucune mention « non observé » ou « non évalué ».
- Commentaire du domaine sous le bloc, s'il existe.
- Bas de page : appréciation générale, absences de la période si l'école les relève, décision
  de fin d'année au dernier livret si elle est posée, puis trois emplacements de signature : la
  maîtresse ou le maître, la direction, les parents.
- Une page par enfant ; une seconde page seulement si la liste de l'école est très longue, et
  alors le bas de page tient sur la dernière.
- Impression de la classe : un seul PDF, les enfants dans l'ordre de la liste de classe.
- Vérifié comme le bulletin : PDF rendu en image, regardé, gardé en capture ; lisible en niveaux
  de gris.

---

## 6. Règles

- **RB-1 — Rien ne bloque.** Préparer, valider, publier et imprimer un livret ne demandent
  aucune observation, aucune appréciation, aucune photo, aucun responsable joignable ; un livret
  se prépare même pour un enfant qui n'a rien.
- **RB-2 — Rien ne fait remarquer ce qui manque.** Pas de compteur, de pourcentage de saisie, de
  liste d'enfants « incomplets », d'avertissement à la validation, de mention imprimée, d'alerte
  au tableau de bord. Les cases vides se voient comme vides, c'est tout.
- **RB-3 — Les frais ne touchent pas au livret.** Des frais impayés n'empêchent ni sa
  préparation, ni son impression, ni sa remise ; quand le suivi des frais est éteint, aucune
  colonne d'impayés ne s'affiche à côté des livrets non retirés.
- **RB-4 — Trois niveaux, toujours.** Acquis, en cours, non acquis ; seuls les libellés se
  règlent. Une observation vide n'est pas un quatrième niveau.
- **RB-5 — Une observation par enfant, compétence et période.** La ressaisir la remplace ;
  l'audit garde l'ancienne.
- **RB-6 — Le livret fige ce qu'il imprime.** Préparé, il ne change plus quand la liste ou les
  observations changent ; « Préparer » de nouveau refait les livrets non publiés ; un livret
  publié se corrige par une nouvelle version, avec un motif, comme un bulletin.
- **RB-7 — Seules les compétences de la section.** Un enfant de MS est observé sur les
  compétences de MS. Changé de section en cours d'année, il garde ses observations passées,
  imprimées sous la section où elles ont été faites.
- **RB-8 — Arrivé en cours d'année.** Son livret ne montre que les périodes où il était inscrit ;
  aucune colonne vide pour les autres.
- **RB-9 — Absences.** Imprimées en demi-journées si au moins un relevé d'absences existe pour la
  classe dans la période ; sinon la ligne n'apparaît pas (une école qui ne relève pas les
  absences au jardin ne verra pas « 0 absence »).
- **RB-10 — Jardin seulement.** Les commandes d'observations refusent une classe dont le niveau
  est en « notes » ; les commandes de notes refusent une classe du jardin (déjà le cas par le
  programme, à confirmer par un essai).
- **RB-11 — Retirer ne détruit rien.** Une compétence ou un domaine retiré disparaît des
  écrans et des livrets à préparer, garde ses observations et revient tel quel si on le remet.
- **RB-12 — Avis par SMS.** À la publication, une fois par enfant et par période, si
  `sms.auto_bulletin` est allumé et qu'un responsable est joignable et a consenti ; le texte dit
  « livret » là où celui du fondamental dit « bulletin ». Pas d'avis, pas de mention.
- **RB-13 — Consultation par SMS.** Pour un enfant du jardin, la réponse donne ce qu'elle donne
  déjà hors moyenne et ne dit rien de l'absence de moyenne.
- **RB-14 — Année close.** Observations et livrets d'une année close se lisent et s'impriment,
  ne se modifient plus.
- **RB-15 — Interface sans sources.** Aucun texte affiché, imprimé ou envoyé ne cite une
  question de terrain, une spécification ou une date de réponse.

---

## 7. Droits

Aucun droit nouveau : la matrice des permissions est régénérée, sans ligne de plus.

| Action | Droit | Comme |
|---|---|---|
| Régler domaines, compétences, échelle | `ACADEMIC_YEAR_MANAGE` | le programme des matières |
| Saisir observations, commentaires, appréciation générale | `GRADES_CREATE`, avec la portée des maîtres | les notes et les appréciations ; Enseignant seulement si `notes.saisie_maitres` est allumé |
| Préparer et imprimer les livrets | `REPORTS_GENERATE` | le calcul et l'impression des bulletins |
| Valider, corriger | `GRADES_VALIDATE` | la validation des bulletins |
| Publier | `REPORTS_PUBLISH` | la publication des bulletins |
| Remettre | droits existants de la remise | inchangés |

Les rôles livrés gardent donc pour le livret exactement ce qu'ils ont pour le bulletin.

**Portée des maîtres** : un maître observe les enfants des classes du jardin où il est affecté,
sans notion de matière ; il écrit le commentaire de domaine et l'appréciation générale de ces
enfants. À vérifier à l'étape 2 : comment un maître est affecté à une classe du jardin
aujourd'hui (le jardin n'a pas de matière) ; si rien ne le permet, l'affectation de titulaire
de classe s'étend au jardin.

---

## 8. Mise à niveau des bases installées

- La migration crée les tables et pose la liste livrée pour les seules sections que l'école a
  ouvertes ; une école sans jardin ne voit rien changer.
- Les périodes du jardin de l'année ouverte sont posées (§3.3). Une école qui a déjà remis ses
  livrets à la main cette année peut saisir les trimestres passés ou commencer au trimestre en
  cours : aucune des deux voies n'est imposée ni signalée.
- Essai : une copie de l'école d'essai en 2.0.0, avec des enfants au jardin, passe la mise à
  niveau ; ses élèves, factures, bulletins et décisions ne changent pas ; ses classes du jardin
  ont leurs périodes et la liste livrée.

---

## 9. Critères d'acceptation

1. Une école neuve avec un jardin trouve la liste livrée pour chaque section ouverte, et trois
   trimestres au jardin pour l'année ouverte.
2. L'école ajoute, renomme, réordonne, retire puis remet une compétence ; les livrets déjà
   préparés ne changent pas ; les observations d'une compétence retirée restent en base.
3. Le choix de deux périodes au jardin est accepté ; un autre découpage est refusé avec un message
   clair.
4. La saisie d'une classe de quarante enfants sur un domaine de quatre compétences, au clavier,
   avec « Toute la colonne » puis dix corrections, tient en moins de trois minutes (Playwright,
   chronométré).
5. Une observation ressaisie remplace l'ancienne ; vidée, elle n'apparaît plus au livret ;
   l'audit garde les deux valeurs.
6. Un enfant sans aucune observation ni appréciation a un livret préparé, validé, publié et
   imprimé, sans message ni mention de manque.
7. Le livret imprimé ne montre ni compétence jamais observée, ni domaine vide, ni cadre de photo
   vide, ni ligne d'absences quand l'école ne les relève pas ; il est lisible en niveaux de gris
   (PDF rendu en image et regardé).
8. Le livret du 2e trimestre montre, pour chaque compétence, les niveaux du 1er et du 2e ; celui
   d'un enfant arrivé au 2e trimestre ne montre que le 2e.
9. La publication crée la remise « à remettre » et, si le réglage est allumé et un responsable
   joignable, un avis par SMS qui dit « livret » ; un livret corrigé et republié ne renvoie pas
   d'avis.
10. Des frais impayés, ou le suivi des frais éteint, ne changent rien au livret ni à sa remise.
11. Réglage `notes.saisie_maitres` éteint : un compte de maître n'observe rien ; allumé, il
    observe les enfants de ses classes du jardin et aucun autre, commande par commande.
12. Les commandes d'observations refusent une classe du fondamental ; celles des notes refusent
    une classe du jardin.
13. Par le réseau (poste relié), chaque commande nouvelle donne le même résultat et le même refus
    qu'en local ; deux postes qui modifient la même observation ne s'écrasent pas en silence.
14. Une base de l'école d'essai en 2.0.0 passe la mise à niveau sans rien perdre ; une école sans
    jardin ne voit ni écran ni menu nouveau.
15. Aucun texte affiché, imprimé ou envoyé ne cite une question de terrain ou une spécification.
16. Matrice des permissions régénérée, identique hors des commandes nouvelles ; captures de chaque
    écran montrées au porteur du projet.

---

## 10. Ordre de réalisation

Une étape, une session neuve. Chaque étape se termine comme au lot A : essais verts, captures
montrées, accord du porteur. Une seule compilation complète par étape, `target` élagué avant.

1. **Données et réglage.** ADR de la décision 10 (tables, échelle fixe à trois niveaux, livret
   porté par `bulletin`, liste sans année scolaire) ; migration ; liste livrée (annexe A, relue
   d'après le programme des jardins d'enfants) ; périodes du jardin (§3.3) et leur mise à
   niveau ; services et commandes de réglage ; écran « Compétences du jardin » ; essais de
   services et capture.
2. **Observations.** Services et commandes de saisie, portée des maîtres, refus des niveaux en
   notes, contrôle de la même fiche ; écran de saisie au clavier, « Toute la colonne »,
   commentaire et appréciation ; essai chronométré par Playwright (critère 4) ; captures.
3. **Livret.** Préparation du contenu figé, validation, publication, correction ; remise et
   avis par SMS au mot « livret » ; consultation par SMS ; vérification de tout ce qui lit
   `bulletin` (§3.4) ; modèle d'impression dans `gesco-docs`, PDF vérifié en image ; écran des
   livrets ; captures.
4. **Livraison du lot.** Matrice des permissions, essais commande par commande par le réseau,
   école d'essai avec un jardin évalué sur deux trimestres, manuel (chapitre du jardin), notes de
   version, essai de mise à niveau d'une base 2.0.0, validation par le porteur.

---

## 11. Ce qu'il faut du porteur du projet

- Les réponses à T-109 à T-115 (§12), avant l'étape 1 ; T-110 et T-111 décident du modèle
  imprimé de l'étape 3.
- Si possible, un livret de jardin réellement remis par une école (nom de l'enfant masqué),
  pour comparer la mise en page ; la spécification ne l'attend pas.

---

## 12. Questions de terrain au porteur du projet

À la suite de T-108. Réponses courtes attendues ; aucune ne demande de document.

### T-109 — La liste livrée des compétences

GESCO livre une liste par section, rédigée à neuf d'après le programme des jardins d'enfants
(annexe A : cinq domaines, quatre compétences par domaine et par section). Est-ce la bonne
quantité ? Les jardins en suivent-ils plus, ou moins ?

*Recommandation* : livrer une vingtaine de compétences par section, que l'école retire, ajoute
et renomme ; une liste courte se remplit vite et tient sur une page, une école qui veut le
détail l'ajoute.

### T-110 — L'échelle et sa forme sur le livret

Les jardins écrivent-ils « acquis, en cours, non acquis », ou autre chose (« en voie
d'acquisition », lettres, couleurs, visages) ? Les parents lisent-ils tous le français ?

*Recommandation* : trois niveaux fixes, libellés modifiables par l'école ; sur le livret, une
croix sous l'en-tête du niveau et un pictogramme noir sur blanc (rond plein, demi-rond, rond
vide), jamais la couleur seule, pour que le livret reste lisible en photocopie et par un parent
qui lit peu.

### T-111 — Un livret par période, ou un livret qui suit l'année

Le livret du 2e trimestre montre-t-il seulement le 2e trimestre, ou aussi le 1er à côté, pour
voir les progrès ?

*Recommandation* : qui suit l'année : une colonne par trimestre déjà passé. Le parent voit
l'enfant avancer, et le dernier livret résume l'année sur la même page.

### T-112 — Le rythme et le calendrier du jardin

Le jardin remet-il son livret aux mêmes dates que le fondamental ? Certaines écoles n'en
remettent-elles que deux par an ?

*Recommandation* : trois trimestres par défaut, aux mêmes dates que le 2e cycle ; l'école peut
passer à deux périodes ; pas de livret mensuel.

### T-113 — Qui observe, qui écrit l'appréciation

Au jardin, qui remplit le livret aujourd'hui : la maîtresse, la direction, les deux ?

*Recommandation* : même règle que les notes : la direction saisit par défaut ; si l'école ouvre
la saisie aux maîtres, la maîtresse observe et écrit l'appréciation de ses classes ; la direction
seule valide et publie.

### T-114 — Ce que le livret porte en plus des compétences

Photo de l'enfant, absences, décision de passage en fin d'année, commentaire par domaine,
signature des parents : lesquels voulez-vous ?

*Recommandation* : tous, chacun seulement quand il existe : photo si elle est dans la fiche,
absences si l'école les relève, décision au dernier livret si elle est posée, commentaire par
domaine facultatif (réglage allumé), trois emplacements de signature (maîtresse, direction,
parents).

### T-115 — Licence

Le livret du jardin fait-il partie de toute licence, ou d'un module vendu à part ?

*Recommandation* : dans toute licence, sans module : une école à jardin qui achète GESCO
s'attend à remettre un livret comme elle remet un bulletin.

---

## 13. Journal des révisions

| Version | Date | Modification |
|---|---|---|
| v1.0 | 2026-11-12 | Création d'après T-84 (réponse du 10 novembre 2026), le §2.3 du plan de la phase 11 et les décisions du porteur du projet du 12 novembre 2026 (ne jamais bloquer, ne pas faire remarquer ce qui manque) ; questions T-109 à T-115 |

---

## Annexe A — Liste livrée proposée

Rédigée à neuf, à relire à l'étape 1 d'après le programme des jardins d'enfants ; l'école la
modifie librement. Quatre compétences par domaine et par section.

**Langage**

| Petite section | Moyenne section | Grande section |
|---|---|---|
| Comprend une consigne simple qu'on lui dit | Comprend une consigne donnée au groupe | Raconte dans l'ordre une histoire entendue |
| Se fait comprendre par des mots ou de courtes phrases | Raconte en quelques phrases ce qu'il a fait | Entend et sépare les syllabes d'un mot |
| Dit une comptine apprise en classe | Pose des questions et répond à celles qu'on lui pose | Reconnaît des lettres et écrit son prénom |
| Nomme les objets de la classe et les parties du corps | Reconnaît son prénom écrit | Copie un mot court en capitales |

**Motricité**

| Petite section | Moyenne section | Grande section |
|---|---|---|
| Marche, court et s'arrête au signal | Saute à pieds joints | Saute à cloche-pied |
| Lance et attrape un gros ballon | Se déplace en équilibre sur une ligne | Joue à un jeu collectif en respectant la règle |
| Tient un crayon pour tracer librement | Découpe en suivant un trait droit | Découpe en suivant une courbe |
| Enfile de grosses perles | Trace des lignes, des ronds et des ponts | Tient bien son crayon et forme les lettres |

**Vivre ensemble**

| Petite section | Moyenne section | Grande section |
|---|---|---|
| Accepte la séparation d'avec sa famille le matin | Attend son tour | Aide un camarade |
| Joue à côté des autres enfants | Partage le matériel | Dit son désaccord par la parole |
| Range le matériel avec de l'aide | Salue et remercie | Prend une responsabilité dans la classe |
| Se lave les mains et va aux toilettes avec de l'aide | Respecte une règle de la classe | Termine une tâche commencée |

**Découverte du monde**

| Petite section | Moyenne section | Grande section |
|---|---|---|
| Reconnaît les couleurs principales | Dénombre une petite collection jusqu'à cinq | Dénombre jusqu'à dix et associe le chiffre |
| Trie des objets selon leur taille | Reconnaît le rond, le carré et le triangle | Compare deux quantités |
| Récite les nombres jusqu'à trois | Situe un objet : dessus, dessous, devant, derrière | Range des objets du plus petit au plus grand |
| Reconnaît les moments de la journée de classe | Nomme des animaux et des plantes de son milieu | Connaît les gestes d'hygiène : mains lavées, eau propre |

**Arts**

| Petite section | Moyenne section | Grande section |
|---|---|---|
| Chante avec le groupe | Colorie en restant dans la forme | Dessine un bonhomme complet |
| Peint et colorie librement | Danse en suivant le rythme | Reproduit un rythme frappé |
| Modèle avec de la pâte ou de l'argile | Dessine un bonhomme | Fabrique un objet en suivant des étapes |
| Frappe un rythme simple | Mime une action ou un animal | Dit un conte, une devinette ou une comptine devant la classe |
