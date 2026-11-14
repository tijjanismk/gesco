# SPEC_PHASE_9.md — Rapports et statistiques de l'école

**Projet** : ERP de gestion scolaire — Tauri 2 + Rust + React/TypeScript
**Phase** : 9 — statistiques de l'école, rapports de rentrée et de fermeture, données de l'enquête annuelle ; version 2.0.0
**Statut** : v1.1 — recommandations de T-98 à T-108 validées par le porteur du projet le 12 novembre 2026, avec une réserve sur T-103 (§13) ; étape 1 codée le 14 novembre 2026
**Documents parents** : `ETAT_AVANCEMENT.md` (section « Phase 9 », décisions du 11 novembre 2026), `docs/orchestration.md` (décisions du 12 novembre 2026 : ne pas bloquer, ne pas le mentionner), `SPEC_PHASE_2.md` (élèves, inscriptions), `SPEC_PHASE_4.md` (notes, bulletins), `SPEC_PHASE_6.md` (personnel), `SPEC_PHASE_10.md` (clôture de l'année, décisions de fin d'année), `SPEC_PHASE_11_A.md` (plusieurs postes), `docs/questions-terrain.md`
**Date** : 12 novembre 2026

**Formulaires étudiés** (remis par le porteur le 11 novembre 2026, gardés hors du dépôt parce
qu'ils portent des noms réels) : le rapport succinct de rentrée de la CPS (fondamental) ; un
rapport de rentrée et un rapport de fermeture d'un CAP, déjà remplis par une école ; la fiche
scolaire de l'élève ; les questionnaires annuels CPS 2023-2024 du secondaire général, du
technique, du professionnel et des instituts de formation des maîtres, avec leurs notices ; le
programme et le guide pédagogiques des CDPE ; la démarche de création et d'ouverture d'un CDPE.
Trois pièces scannées (dossier d'école, arrêté d'ouverture, décision de création) n'ont pas pu
être lues par la machine : elles ne servent ici qu'à savoir qu'un acte de création et un acte
d'ouverture existent, avec une référence et une date.

---

## 0. Objet et critère de fin

### 0.1 Ce que produit cette phase

Chaque année, la direction recopie à la main, dans plusieurs documents, des chiffres que
l'application connaît déjà : combien d'élèves par classe, de filles et de garçons, de
redoublants, de maîtres par statut. Elle y ajoute des chiffres que l'application ne connaît pas
encore : salles, latrines, tables-bancs, manuels, résultats aux examens.

À la fin de la phase :
- un écran **Statistiques** donne les chiffres de l'école par thème, arrêtés à la date que l'on
  choisit, et les exporte en Excel ;
- GESCO **imprime le rapport succinct de rentrée, le rapport de rentrée et le rapport de
  fermeture** : les tableaux sont remplis, les parties rédigées sont saisies par la direction et
  reprises d'une année sur l'autre ;
- GESCO imprime la **fiche scolaire** d'un élève ;
- pour l'**enquête annuelle de la CPS**, GESCO ne reproduit pas le questionnaire : il fournit,
  dans l'ordre des rubriques, les chiffres qui servent à le remplir ;
- l'école tient un **inventaire simple** (locaux, mobilier, matériel, manuels, existant et
  besoins) et une **fiche de l'établissement** qu'elle peut compléter de ses propres rubriques ;
- aucun de ces documents n'est jamais refusé parce qu'une donnée manque.

### 0.2 Décisions déjà prises qui s'appliquent

- **Version et ordre** (11 novembre 2026) : la phase 9 entre dans la 2.0, après le lot A de la
  phase 11 ; spécification d'abord.
- **Enquête CPS** (11 novembre 2026) : elle change d'une année à l'autre ; GESCO n'en reproduit
  pas la mise en page, il fournit les données. Si le formulaire change, l'école s'arrange avec
  ces données.
- **Rapports de rentrée et de fermeture** (11 novembre 2026) : documents imprimés par GESCO,
  tableaux remplis, parties rédigées saisies par la direction.
- **Proposé le 11 novembre 2026, confirmé ici** : écran de statistiques par thème (élèves,
  personnel, locaux et équipements, examens et fin d'année, établissement), exportable en Excel,
  chiffres arrêtés à une date choisie ; puis un tableau libre dont l'école choisit lignes et
  colonnes ; des rubriques que l'école ajoute à sa fiche (T-107).
- **Ne pas bloquer, ne pas le mentionner** (12 novembre 2026) : un dossier d'enseignant absent ou
  incomplet, des frais impayés, une paie que l'école ne tient pas dans l'application ne bloquent
  rien et ne sont signalés nulle part. Cette phase l'applique à tout ce qu'elle produit (§10).
- **Interface sans sources** : aucun texte affiché ou imprimé ne cite un numéro de question, de
  spécification ou une date de décision.
- **Questions de terrain plutôt que collecte de documents** : ce qui manque se demande au porteur
  (§13), la phase ne l'attend pas.
- R-21 : toute commande passe par `Application::executer`. R-14 : rien ne se supprime ; une ligne
  d'inventaire ou une rubrique se désactive.
- Lot A : les écrans de cette phase marchent depuis un poste relié ; les fichiers Excel et PDF
  voyagent par leur contenu ; deux personnes qui rédigent le même rapport ne s'écrasent pas
  (refus si le texte a changé depuis sa lecture).

### 0.3 Critère de fin, vérifiable

1. Sur l'école d'essai de trois ans, l'écran Statistiques donne, pour une date choisie, les
   effectifs par classe, niveau, cycle, sexe, âge et année de recrutement, les redoublants, les
   entrées et les départs ; chaque total est égal au décompte fait directement sur les
   inscriptions par un essai indépendant.
2. Changer la date d'arrêt change les chiffres : un élève parti le 15 janvier compte au
   10 janvier, ne compte plus au 20.
3. Les trois rapports et la fiche scolaire s'impriment sur une école qui vient d'être créée,
   sans élève, sans personnel, sans inventaire : tableaux vides, aucun refus, aucun message
   d'erreur.
4. Les mêmes documents s'impriment sur une école où manquent des dates de naissance, des sexes,
   des statuts, des diplômes, des dossiers de personnel, toute la paie, et où des frais sont
   impayés : les cases correspondantes sont vides, les totaux restent justes, rien dans le
   document ne signale ces manques.
5. Chaque tableau imprimé des rapports de rentrée et de fermeture a les lignes et les colonnes
   du modèle remis (§3 à §5), et ses totaux de lignes et de colonnes se recoupent.
6. Une partie rédigée saisie l'an dernier est proposée cette année ; la direction la garde, la
   modifie ou la vide.
7. Un rapport arrêté se réimprime à l'identique six mois plus tard, même si des élèves ont été
   inscrits depuis.
8. L'export « Données pour l'enquête » contient, pour le secondaire, une feuille par partie du
   questionnaire (§6), dans son ordre, avec la mention de ce qui reste à remplir à la main.
9. Une rubrique ajoutée par l'école à sa fiche se saisit, se retrouve dans l'export et dans le
   tableau libre.
10. Le handicap d'un élève n'est lu que par un compte qui en a le droit ; il n'apparaît dans
    aucune liste nominative imprimée.
11. Depuis un poste relié : consulter les statistiques, exporter en Excel, rédiger et imprimer un
    rapport.
12. Une école en 1.2 qui installe la 2.0 ne voit rien changer dans ses écrans existants ; les
    essais des phases précédentes passent sans changement.

## 1. Périmètre

**Inclus** : écran Statistiques par thème, date d'arrêt, export Excel ; fiche de
l'établissement et rubriques (livrées et ajoutées par l'école) ; compléments sur l'élève, la
classe et le personnel ; inventaire de l'école ; résultats aux examens ; rapport succinct de
rentrée, rapport de rentrée, rapport de fermeture, fiche scolaire ; données pour l'enquête CPS
du secondaire ; tableau libre ; droits ; manuel, captures, école d'essai.

**Exclu** :
- La mise en page des questionnaires CPS, et tout dépôt en ligne auprès de l'administration
  (SIGE ou autre) : GESCO fournit des chiffres, l'école remplit.
- La gestion de stock : pas d'entrées ni de sorties de matériel, pas de prêt de manuels suivi
  élève par élève. L'inventaire est un relevé de quantités.
- Les rapports mensuels ou trimestriels à l'administration, tant que le porteur n'en a pas
  montré un (T-98).
- Le questionnaire des instituts de formation des maîtres : GESCO ne vise pas ces
  établissements.
- Le livret du jardin (lot B de la phase 11) et les contenus pédagogiques des CDPE (phase 12) :
  le programme et le guide des CDPE remis le 11 novembre 2026 servent à ces deux chantiers, pas
  à celui-ci. Les effectifs du jardin, eux, sont dans les statistiques.
- Les tableaux de bord financiers : ils existent déjà (phases 3 et 8). La phase 9 ne produit
  aucun chiffre d'impayés.

**Non fourni à ce jour, place réservée** : le questionnaire annuel CPS du fondamental et du
préscolaire, la fiche SIGE, une édition plus récente des questionnaires du secondaire. Le §7
dit ce qui est prévu sans eux.

## 2. Ce que l'application possède déjà

Relevé fait sur le schéma au 12 novembre 2026 (migrations 0001 à 0050).

| Donnée | Où | Sert à |
|---|---|---|
| Nom, statut (public, privé, communautaire, medersa), académie, CAP, adresse, téléphones de l'établissement | `etablissement` | en-têtes, identification |
| Année scolaire, périodes | `annee_scolaire`, `periode_academique` | tout |
| Niveaux, cycle (jardin, premier cycle, second cycle, lycée, technique), diplôme terminal, séries, classes de l'année | `niveau`, `serie`, `classe` | lignes et colonnes des tableaux ; une classe est un groupe pédagogique |
| Sexe, date de naissance (parfois estimée), lieu de naissance, nationalité | `personne` | répartitions par sexe et par âge |
| Matricules, acte de naissance, école d'origine | `eleve` | fiche scolaire |
| Inscription : classe, date, type (nouvelle, réinscription, transfert entrant), statut, redoublant, décision de fin d'année (admis, redouble, exclu, rattrapage), date et motif de départ, établissement de destination | `inscription` | effectifs à une date, redoublants, passages, départs, historique de scolarité |
| Exclusions temporaires et définitives | `exclusion` | lignes « exclus » |
| Responsables de l'élève, lien de parenté | `responsable`, `eleve_responsable` | fiche scolaire |
| Personnel : catégorie, fonction, date d'arrivée, date de départ, fonctionnaire ou non, numéro de fonctionnaire, situation de famille, diplômes | `membre_personnel`, `categorie_personnel` | listes et effectifs du personnel |
| Engagements : nature (durée indéterminée, durée déterminée, vacation, stage, bénévole), dates | `engagement` | permanent, contractuel, vacataire, stagiaire |
| Enseignants et leurs matières par classe | `enseignant`, `affectation_enseignement` | qui tient une classe, dans quel cycle, quelle discipline |
| Emploi du temps | `seance` | heures hebdomadaires par discipline |
| Moyennes annuelles, bulletins | `bulletin` | taux de réussite interne |
| Tarifs par niveau | `grille_tarifaire`, `ligne_tarifaire` | « frais de scolarité par élève » de l'enquête |
| Cantine | `repas_cantine`, réglage | « cantine fonctionnelle » |
| Absences du personnel, congés | `absence_personnel`, `conge` | non utilisés par les formulaires étudiés |

Ce que l'application ne possède pas, et que les formulaires demandent : voir §8.

Dans les sections 3 à 6, chaque rubrique porte une lettre :
**A** — l'application la remplit avec ce qu'elle a déjà ;
**N** — elle la remplira avec une donnée nouvelle de cette phase (§8) ;
**M** — elle reste remplie à la main, sur le papier ou dans une zone de texte.

## 3. Rapport succinct de rentrée (CPS, fondamental)

Une page, à remettre au CAP dans les quinze jours qui suivent la rentrée. Chiffres arrêtés à la
date choisie par la direction, proposée quinze jours après le premier jour de l'année.

| Rubrique | | Source ou donnée nouvelle |
|---|---|---|
| Année scolaire | A | année en cours |
| École, CAP, académie | A | `etablissement` |
| Village ou quartier, commune, cercle | N | fiche de l'établissement |
| Langue, si pédagogie convergente | N | fiche de l'établissement |
| Cycles (1er cycle, 2e cycle, cycle complet) | A | déduit des niveaux qui ont une classe cette année |
| Statut administratif (publique, privée catholique, médersa, privée laïque, communautaire) | A + N | le statut existe ; « confessionnelle ou laïque » est une rubrique nouvelle |
| Type d'enseignement (classique, pédagogie convergente, franco-arabe, curriculum) | N | fiche de l'établissement |
| Mode de recrutement (annuel, biennal, autre) | N | fiche de l'établissement |
| Tableau 2 — par année d'études, 1re à 9e : nombre de groupes pédagogiques | A | nombre de classes du niveau |
| Tableau 2 — élèves par sexe, total | A | inscriptions valides à la date |
| Tableau 2 — redoublants par sexe, total | A | `inscription.redoublant` |
| Tableau 3 — maîtres permanents et contractuels, hommes et femmes, par cycle | A + N | cycle déduit des affectations ; permanent ou contractuel déduit du statut du personnel (§8.3) |
| Tableau 3 — salles de classe en dur, semi-dur, autre, par cycle | N | inventaire des locaux |

Rien ne reste à la main, sauf la signature.

## 4. Rapport de rentrée (CAP)

Deux parties : une partie administrative rédigée, des tableaux.

### 4.1 En-tête et partie administrative

| Rubrique | | Source ou donnée nouvelle |
|---|---|---|
| Région, académie, CAP, commune, nom de l'école, année | A + N | `etablissement` et fiche |
| Introduction : situation de l'école (cercle, commune, effectifs du personnel et des élèves, domaine scolaire) | M, aidée | texte de la direction ; GESCO propose à côté les chiffres du jour (enseignants, élèves, superficie) qu'elle recopie si elle veut |
| Préparation de la rentrée : conseils des maîtres tenus, rencontres avec les partenaires | M | texte |
| Déroulement de la rentrée : visites reçues, suggestions et recommandations | M | texte |
| Organisation pédagogique : emploi du temps, leçons modèles, formation continue des maîtres | M | texte |
| Rapports avec les partenaires : comité de gestion, syndicats, associations ; partenaires techniques et financiers et leurs domaines | M | texte |
| Conclusion | M | texte |
| « Fait à … le … », qualité du signataire | N | réglages du rapport (lieu, date, « Le Directeur », « La Directrice », « Le Proviseur ») |

Chaque texte est repris de l'année précédente quand il existe (§9.3).

### 4.2 Tableaux

| Tableau | | Source ou donnée nouvelle |
|---|---|---|
| Maîtres du premier cycle par statut (fonctionnaire de l'État, fonctionnaire des collectivités, contractuel des collectivités, contractuel local, élève-maître), hommes, femmes, total | N | statut du personnel (§8.3), déduit par défaut de ce qui existe ; cycle déduit des affectations |
| Maîtres du second cycle, mêmes statuts, par spécialité (LHG, MPC, SNPC, LDM, EPS, autres) | N | spécialité du maître (§8.3) |
| Élèves par année d'études et par sexe : passants, redoublants, exclus, total, total garçons et filles, totaux par cycle | A | inscriptions à la date ; passant = non redoublant ; exclu = exclusion définitive prononcée depuis la rentrée |
| Élèves par année d'études, sexe et année de recrutement, par cycle | N | année de recrutement de l'élève (§8.2), déduite quand c'est possible |
| Élèves par année d'études, sexe et année de naissance, par cycle | A | `personne.date_naissance` ; une date estimée compte comme une date |
| Nombre de classes par cours, fonctionnement (simple, double division, double vacation), pédagogie appliquée (classique, pédagogie convergente, curriculum) | A + N | nombre de classes existant ; fonctionnement et pédagogie sont nouveaux sur la classe (§8.2) |
| Salles de classe par nature de construction (dur, semi-dur, banco, autres), total, besoins ; toilettes, existant et besoins ; points d'eau, existant et besoins | N | inventaire des locaux |
| Mobilier scolaire : nature, existant, besoins, observations, total | N | inventaire du mobilier |
| Matériel didactique : nature, existant, besoins, observations, total | N | inventaire du matériel |
| Manuels disponibles : désignation, quantité par année d'études, totaux par cycle | N | inventaire des manuels |

## 5. Rapport de fermeture (CAP)

Le modèle remis est un fichier Word d'ancien format ; une partie des tableaux de la fin se lit
mal. Les tableaux marqués « à vérifier » seront comparés au modèle ouvert dans Word à l'étape 7,
avant d'être dessinés.

### 5.1 En-tête et partie administrative

| Rubrique | | Source ou donnée nouvelle |
|---|---|---|
| Région, académie, cercle, CAP, commune, école | A + N | `etablissement` et fiche |
| Statut de l'école (publique, privée, communautaire) | A | `etablissement.statut` |
| Type de programme (classique, curriculum, medersa) | N | fiche de l'établissement |
| Type de l'école (mobile, double division, double vacation, classe unique ; rurale, urbaine) | N | fiche de l'établissement ; double division et double vacation proposées d'après les classes |
| Introduction | M | texte |
| A — Conditions de déroulement de l'année, difficultés, solutions préconisées | M | texte |
| B — État des lieux : personnel, effectifs, infrastructure, mobilier, matériel didactique, manuels | M, aidée | texte ; GESCO propose à côté une phrase toute faite tirée des chiffres (« L'école compte … élèves au premier cycle, dont … filles et … garçons : … en 1re année, … ») que la direction insère et retouche |
| C — Besoins : personnel, infrastructure, mobilier, matériel, manuels | M, aidée | texte ; les besoins de l'inventaire sont proposés à côté |
| D — Organisation des examens : professionnels, scolaires | M | texte |
| E — Rapports avec les partenaires | M | texte |
| F et G — Résultats scolaires, pourcentage par cycle | A, puis M | taux d'admis parmi les décisions de fin d'année, proposé ; la phrase reste à la direction |
| Conclusion ; « Fait à … le … » ; signataire | M + N | texte et réglages du rapport |

### 5.2 Partie statistique

| Tableau | | Source ou donnée nouvelle |
|---|---|---|
| Personnel du premier cycle, nominatif : prénoms et nom, numéro matricule, situation de famille, statut, diplôme, date d'arrivée au poste | A + N | tout existe sauf le statut selon la nomenclature (§8.3) ; une case non renseignée reste vide |
| Personnel du second cycle, mêmes colonnes | A + N | idem |
| Résultats des examens — passage en 7e année : inscrits, présentés, admis, pourcentage de réussite, garçons, filles, total | N | résultats aux examens (§8.5) |
| Résultats des examens — DEF, mêmes colonnes (à vérifier) | N | idem |
| Par année d'études et par sexe : effectifs de fermeture, passages, redoublements, exclusions | A | inscriptions à la date de fermeture et décisions de fin d'année ; « non statué » et « rattrapage » ne comptent dans aucune des trois lignes |
| Effectifs probables à la rentrée, par année d'études et par sexe | A, modifiable | proposé : passants de l'année d'en dessous plus redoublants de l'année ; la direction corrige, surtout la 1re année |
| Infrastructures par cycle : salles de classe, direction, blocs de latrines, magasin, points d'eau potable, coins de lecture, bibliothèque, existant et besoins | N | inventaire des locaux |
| Mobilier, matériel, manuels à la fermeture, existant et besoins (à vérifier) | N | inventaire |
| Enseignants généralistes et spécialistes (LHG, MPC, LDM, SNPC, EPS), hommes et femmes, par statut (à vérifier) | N | statut et spécialité du personnel |

### 5.3 Fiche scolaire de l'élève

Une page par élève, demandée par le CAP ; GESCO l'imprime depuis la fiche de l'élève ou pour
toute une classe.

| Rubrique | | Source ou donnée nouvelle |
|---|---|---|
| École, commune, cercle, région | A + N | `etablissement` et fiche |
| Nom, prénom, date et lieu de naissance | A | `personne` |
| Pièce d'état civil, numéro, autorité qui l'a délivrée | A + M | nature, numéro et date existent ; l'autorité se remplit à la main |
| Date de recrutement, numéro matricule | N + A | année de recrutement ; matricule national, sinon matricule de l'école |
| Père, mère, professions, domicile | A + M | responsables de lien « père » et « mère » ; profession si elle est connue, sinon à la main |
| Tuteur : nom, lien de parenté, adresse | A | responsable principal quand il n'est ni le père ni la mère |
| Scolarité au premier puis au second cycle : période, école, cercle, par année d'études ; classes doublées | A + M | années passées dans l'école : inscriptions ; avant : école d'origine connue, le reste à la main |
| DEF : date, résultat | N | résultats aux examens |

## 6. Enquête annuelle de la CPS — secondaire général, technique, professionnel

Les trois questionnaires (2023-2024) ont le même plan en treize parties ; ils diffèrent par les
séries ou filières en colonnes. GESCO n'imprime pas le questionnaire. L'export « Données pour
l'enquête » est un classeur Excel : une feuille par partie, dans l'ordre du questionnaire, avec
pour chaque rubrique sa valeur, ou vide, ou « à remplir à la main ».

| Partie | | Ce que donne GESCO |
|---|---|---|
| I. Identification et localisation : nom, sigle, statut, confessionnel ou laïc, option, franco-arabe, éducation spécialisée, dates de création et d'ouverture, référence de l'acte d'ouverture, date de la demande, années de construction et d'équipement, région, académie, cercle, commune urbaine ou rurale, quartier, rue, porte, coordonnées, boîte postale, téléphone, courriel, distances au chef-lieu, nom et téléphone du proviseur | A + N | nom, statut, académie, téléphones existent ; tout le reste est une rubrique de la fiche de l'établissement, saisie une fois et gardée d'année en année ; le code de l'établissement se saisit quand l'administration l'a donné |
| II. Informations générales (46 questions : foncier, eau, lavage des mains, clôture, électricité, latrines et cabines, hygiène, ordures, bibliothèque, internet, laboratoires, cantine, terrain de sport, comité de gestion, association de parents, projet d'établissement, formations reçues, grèves, incidents) | N + M | rubriques livrées de la fiche, par oui ou non, choix ou nombre, reprises de l'année précédente ; « cantine fonctionnelle » proposée d'après le réglage de la cantine ; le nombre d'élèves handicapés par type vient des élèves (§8.2) ; les nombres de cas d'incidents, de grossesses et d'attaques ne sont pas tenus par GESCO : à la main |
| III. Nombre et état des locaux : par type de local (salle de classe utilisée ou non, bureau, salle des professeurs, salle de langue, salle informatique, bibliothèque, laboratoire, CDI, salle de conférence, logement, infirmerie, toilettes, magasin) — nombre, dont neufs, dont en location, murs bons ou mauvais, toits bons ou mauvais | N | inventaire des locaux |
| IV. Mobilier et équipements en bon état, des salles de classe et de la salle des professeurs ; tables-bancs à 1, 2, 3, 4 places | N | inventaire du mobilier, colonne « en bon état » |
| V. Données financières : frais de scolarité annuels par élève, année précédente et année en cours | A | montant annuel des grilles tarifaires, par niveau, pour les deux années ; l'école retient celui qu'elle déclare |
| VI.1 Personnel par statut (fonctionnaire de l'État, des collectivités, contractuel de l'État, des collectivités, contractuel privé, vacataire, autre), par sexe ; enseignants par diplôme le plus élevé ; « dont tenant classe » | N + A | statut et diplôme le plus élevé (§8.3) ; « tenant classe » = a une affectation cette année |
| VI.2 Enseignants par discipline ; heures hebdomadaires assurées par discipline | A | matières des affectations ; heures de l'emploi du temps quand l'école le tient, sinon vide |
| VII. Nouveaux inscrits en 10e par âge et par sexe | A | inscrits en 10e non redoublants ; âge = année civile de la rentrée moins année de naissance |
| VIII. Élèves par année d'études, série, âge et sexe ; nombre de groupes pédagogiques ; dont redoublants ; élèves pris en charge par l'État | A + N | tout existe sauf la prise en charge par l'État (§8.2) ; colonnes = séries de l'école |
| IX. Élèves handicapés par type, degré, série et sexe | N | handicap de l'élève (§8.2) |
| X. Obstacles physiques et matériels d'apprentissage (route, rampes, salles accessibles, matériel adapté) | N + M | rubriques livrées de la fiche ; les quantités de matériel adapté sont des lignes d'inventaire |
| XI. Ressources humaines et services pour le handicap (enseignants handicapés, formés, spécialistes, itinérants, consultants) | M | à la main : GESCO ne note pas le handicap d'un membre du personnel |
| XII. Résultats scolaires : élèves de terminale de l'année précédente, dont admis, dont handicapés | A + N | inscriptions de l'année précédente ; admis = résultat à l'examen (§8.5) |
| XIII. Observations, signatures | M | à la main |

Technique et professionnel : mêmes feuilles ; les colonnes de la partie VIII sont les séries ou
filières que l'école a créées ; la feuille des enseignants par année d'études du professionnel
se tire des affectations.

## 7. Enquête annuelle du fondamental et du préscolaire, fiche SIGE — place réservée

Ces formulaires n'ont pas été fournis. Ce que la phase fait sans eux :
- les tableaux du rapport succinct et du rapport de rentrée couvrent ce qu'une enquête du
  fondamental demande d'ordinaire sur les élèves, les maîtres et les locaux ; ils sont dans
  l'export Excel de l'écran Statistiques, thème par thème ;
- les rubriques de la fiche de l'établissement et l'inventaire ne dépendent pas du cycle : ce
  qui est saisi pour le lycée vaut pour le fondamental ;
- le jardin a ses effectifs par section, sexe et âge dans le thème Élèves.

Quand le questionnaire du fondamental arrivera : une session le lit, ajoute au catalogue les
rubriques qui manquent, et compose la feuille « Données pour l'enquête » du fondamental comme
celle du secondaire. Cette session est prévue dans l'étape 8 ; si le formulaire n'est pas là,
l'étape se termine sans elle et la version sort quand même.

## 8. Données nouvelles

Les numéros de migration se prennent à la suite de la dernière au moment de chaque étape (0050
au 12 novembre 2026 ; l'étape 5 du lot A et le lot B peuvent en ajouter d'ici là). Chaque
migration a son retour arrière. Aucune colonne nouvelle n'est obligatoire.

### 8.1 Fiche de l'établissement et rubriques

- `rubrique_fiche` : le catalogue. Code, thème, libellé, aide, type de réponse (oui ou non,
  nombre, texte court, date, choix dans une liste, plusieurs choix), liste des choix, valeur
  annuelle ou permanente, livrée ou ajoutée par l'école, active, ordre. Les rubriques livrées
  sont celles des §3 à §6 : localisation (région, cercle, commune, type de commune, quartier ou
  village, rue, porte, coordonnées, boîte postale, courriel, distances), identité (sigle, code
  donné par l'administration, confessionnel ou laïc, type d'enseignement, langue, mode de
  recrutement, milieu rural ou urbain, superficie), actes (décision de création et arrêté
  d'ouverture : référence et date ; date de la demande ; années de construction et
  d'équipement), eau et hygiène, énergie, équipements collectifs, vie de l'école, accessibilité.
- `valeur_fiche` : la réponse à une rubrique, pour une année quand la rubrique est annuelle.
  Une rubrique sans réponse n'a pas de ligne.
- Une rubrique livrée ne se supprime ni ne se renomme ; l'école peut la désactiver. Une rubrique
  ajoutée par l'école se modifie et se désactive.
- À l'ouverture d'une année, les réponses annuelles de l'année précédente sont proposées,
  jamais recopiées sans que la direction les voie.

### 8.2 Élèves et classes

- **Année de recrutement** de l'élève (année d'entrée en 1re année du fondamental), facultative.
  Proposée d'office quand la première inscription connue dans l'école est une inscription
  nouvelle en 1re année ; saisie sinon ; importable par le fichier d'élèves (T-105).
- **Handicap** : table à part `difficulte_eleve` — élève, type (vue, ouïe, motricité globale,
  motricité fine, intellect, communication, comportement et socialisation, multiple, autre),
  degré (quelques difficultés, beaucoup de difficultés, n'y parvient pas), note libre.
  Facultatif. Lu et écrit avec un droit propre (§11) ; jamais dans une liste nominative, un
  bulletin, un SMS ou un export d'élèves (T-102).
- **Prise en charge par l'État**, sur l'inscription de l'année, pour les niveaux du lycée et du
  technique : oui ou non, non par défaut.
- **Fonctionnement** de la classe (simple, double division, double vacation) et **pédagogie
  appliquée** (classique, pédagogie convergente, curriculum), sur la classe ; valeur par défaut
  prise dans la fiche de l'établissement, « simple » sinon.
- **Profession** du père, de la mère ou du tuteur : champ de texte sur le responsable, s'il
  n'existe pas déjà (à vérifier à l'étape 3 avant d'ajouter).

### 8.3 Personnel

- **Statut pour les rapports**, sur le membre du personnel : fonctionnaire de l'État,
  fonctionnaire des collectivités, contractuel de l'État, contractuel des collectivités,
  contractuel de l'école (privé, « local », « fonds local »), vacataire ou prestataire,
  élève-maître ou stagiaire, autre. **Jamais vide à l'affichage** : tant que personne ne l'a
  saisi, il se déduit — fonctionnaire coché → fonctionnaire de l'État ; engagement en cours de
  nature vacation → vacataire ; stage → élève-maître ; sinon contractuel de l'école. La valeur
  saisie l'emporte. Chaque rapport regroupe cette liste selon ses propres colonnes : le rapport
  succinct n'en a que deux (permanent, contractuel), le rapport de rentrée cinq, l'enquête sept
  (T-100).
- **Spécialité** de l'enseignant du second cycle (LHG, MPC, SNPC, LDM, EPS, autre), facultative.
  Proposée d'après les matières qu'il enseigne quand elles tombent toutes dans une même
  spécialité ; vide → « autres ».
- **Diplôme le plus élevé**, choisi dans la liste de l'enquête (doctorat, master, DEA, diplôme
  de l'ENSUP, ingénieur, maîtrise, licence, DEUG ou BTS, BT, baccalauréat, CAP, diplôme d'IFM,
  autre), facultatif. Les diplômes déjà saisis en texte libre restent tels quels ; ils servent à
  proposer une valeur, pas à l'imposer.
- **Cycle** et **« tient une classe »** ne se saisissent pas : ils se lisent dans les
  affectations de l'année. Un maître affecté dans deux cycles compte dans chacun des deux
  tableaux et une seule fois dans le total de l'école.

Rien de cela ne touche à la paie, et rien n'exige une pièce au dossier.

### 8.4 Inventaire de l'école

- `article_inventaire` : le catalogue. Famille (local, mobilier, matériel didactique, manuel,
  équipement adapté), désignation, livré ou ajouté par l'école, actif, ordre. Livrés : les
  locaux, le mobilier et le matériel nommés dans les §3 à §6. Les manuels se créent par l'école
  (titre, matière).
- `releve_inventaire` : pour une année et un article — cycle (facultatif), niveau (pour les
  manuels), quantité existante, dont en bon état, besoins, observation. Pour les locaux
  s'ajoutent : nature de construction (dur, semi-dur, banco, autre), dont neufs de l'année, dont
  en location, murs bons et mauvais, toits bons et mauvais ; pour les latrines : cabines des
  filles, des garçons, des enseignants.
- Un relevé par année, repris de l'année précédente à l'ouverture, corrigeable toute l'année ;
  le rapport de fermeture lit l'état du jour où il est arrêté (T-103).
- L'inventaire est tenu par la personne que l'école en charge, quel que soit son rôle : un droit
  à part (§11), que l'administrateur donne à qui il veut (réserve du porteur sur T-103).
- Toutes les quantités sont facultatives ; une quantité absente est une case vide, pas un zéro.

### 8.5 Examens

- `resultat_examen` : pour une inscription d'un niveau à examen (passage en 7e, DEF,
  baccalauréat, CAP, BT) — inscrit à l'examen, présenté, admis, mention, date. Saisie en grille,
  classe par classe, après la proclamation ; tous les champs sont facultatifs.
- Pour l'école qui ne veut saisir que des totaux : `total_examen` — année, examen, sexe,
  inscrits, présentés, admis. Quand les deux existent pour un même examen, le détail par élève
  l'emporte (T-101).
- Le niveau porte déjà son diplôme terminal ; l'examen de passage en 7e se rattache à la 6e
  année par un réglage, éteint pour l'école qui n'en organise pas.

### 8.6 Rapports

- `rapport_ecole` : année, type (succinct de rentrée, rentrée, fermeture), état (en
  préparation, arrêté), date d'arrêt des chiffres, lieu, date et qualité du signataire. Un seul
  rapport de chaque type par année et par périmètre (T-99).
- `texte_rapport` : rapport, section, texte. Version de ligne pour le refus des modifications
  croisées.
- À l'arrêt, les chiffres de tous les tableaux sont figés avec le rapport (document enregistré
  dans `document`, comme les bulletins), pour qu'une réimpression donne le même papier.
- **Réglages** (réglages d'école) : date d'arrêt proposée pour le rapport succinct (quinze
  jours après la rentrée) ; examen de passage en 7e organisé ou non ; qualité du signataire.

## 9. Écrans et impressions

### 9.1 Statistiques

Entrée « Statistiques » dans le menu, pour qui a le droit de les consulter. En haut : l'année,
la **date d'arrêt** (aujourd'hui par défaut), le périmètre (toute l'école, un cycle). Cinq
onglets :

- **Élèves** : effectifs par classe, niveau, cycle et sexe ; redoublants ; par année de
  naissance ; par âge ; par année de recrutement ; nouveaux inscrits ; entrées et départs depuis
  la rentrée avec leur motif ; élèves pris en charge par l'État ; pour qui en a le droit,
  élèves handicapés par type et degré, sans nom.
- **Personnel** : par statut, sexe et cycle ; enseignants par spécialité, par discipline, par
  diplôme ; heures hebdomadaires par discipline ; liste nominative du rapport de fermeture.
- **Locaux et équipements** : l'inventaire de l'année, existant, bon état, besoins.
- **Examens et fin d'année** : décisions de fin d'année par classe et sexe ; résultats aux
  examens ; taux de réussite ; effectifs probables à la rentrée.
- **Établissement** : la fiche et ses rubriques, par thème, avec la saisie sur place pour qui
  en a le droit.

Chaque tableau a son bouton « Exporter en Excel » ; un bouton exporte tout l'onglet, un autre
compose le classeur « Données pour l'enquête ». Un chiffre se clique : la liste des élèves ou
des personnes qui le composent s'ouvre (sauf pour le handicap). Sous un tableau où des fiches
n'entrent dans aucune colonne, une ligne discrète le dit en termes neutres (« 4 élèves sans
date de naissance ») et mène à ces fiches ; cette ligne n'existe qu'à l'écran.

### 9.2 Inventaire, examens, compléments

- **Inventaire** : un écran par famille, une ligne par article, saisie directe des quantités,
  bouton « Reprendre l'an dernier ».
- **Résultats aux examens** : une grille par classe d'examen, trois cases à cocher et une
  mention par élève ; ou la saisie des totaux.
- **Élève, classe, personnel** : les champs nouveaux prennent place dans les fiches existantes,
  dans un bloc « Pour les rapports », replié par défaut, sans astérisque. Une saisie en grille
  par classe permet de remplir l'année de recrutement de tous les élèves d'un coup.

### 9.3 Rapports

Écran « Rapports » : la liste des rapports de l'année, leur état. Ouvrir un rapport montre, de
haut en bas, ses sections dans l'ordre du modèle :
- une section rédigée est une zone de texte, avec à côté « Reprendre le texte de l'an dernier »
  et, quand le §4 ou le §5 dit « aidée », les chiffres ou la phrase proposés à insérer ;
- un tableau est montré rempli, en lecture ; un lien mène à l'écran où se corrige la donnée ;
- en bas : « Aperçu », « Imprimer », « Exporter les tableaux en Excel », « Arrêter le rapport ».

Arrêter fige les chiffres et les textes ; « Rouvrir » est possible, journalisé, et laisse
l'ancien document dans l'historique.

### 9.4 Impressions

Par le moteur de documents existant (`gesco-docs`), format A4 :
- rapport succinct de rentrée : une page, portrait ;
- rapport de rentrée et rapport de fermeture : partie rédigée en portrait, tableaux larges en
  paysage ; en-tête administratif (région, académie, CAP, commune), nom et logo de l'école ;
- fiche scolaire : une page par élève, par élève ou par classe ;
- chaque tableau des statistiques : impression simple, titre, date d'arrêt.

Les tableaux reprennent les intitulés et l'ordre des modèles, pour que le CAP reconnaisse son
formulaire ; la mise en page n'en est pas la copie au millimètre. L'export Excel passe par le
tableur existant.

## 10. Règles

1. **Un rapport sort toujours.** Aucune impression, aucun export, aucun arrêt de rapport n'est
   refusé pour une donnée absente. Une donnée absente donne une case vide. Zéro ne s'écrit que
   lorsque l'application sait qu'il n'y en a pas (aucun redoublant dans une classe qui a des
   élèves).
2. **Rien n'est rendu obligatoire.** Les champs nouveaux sont facultatifs, à la saisie comme à
   l'import ; aucune fiche existante ne devient incomplète.
3. **Ne pas le mentionner.** Aucun document imprimé ou exporté ne porte de mention du genre
   « non renseigné », « dossier incomplet », « à compléter ». À l'écran, la seule indication est
   la ligne neutre du §9.1. Il n'existe ni case « refus », ni compteur de dossiers manquants,
   ni rappel.
4. **Le personnel se compte sur sa fiche**, jamais sur son dossier ni sur la paie. Une personne
   dont la fiche existe, qui n'est pas partie à la date d'arrêt, compte ; qu'elle ait remis des
   pièces ou non, qu'elle ait un bulletin de paie ou non.
5. **Les frais n'entrent pas dans les effectifs.** Un élève inscrit compte, qu'il ait payé ou
   non. Aucun rapport de cette phase ne contient un impayé.
6. **Un élève compte à une date** si son inscription est validée, datée de ce jour ou d'avant,
   et qu'il n'est pas parti à cette date. Les brouillons ne comptent pas. Pour une année close,
   les inscriptions achevées comptent jusqu'à la clôture.
7. **Totaux.** Le total d'une ligne compte toutes les fiches, y compris celles qui n'entrent
   dans aucune colonne (sexe ou date de naissance inconnus) : un total peut donc dépasser la
   somme des colonnes, et c'est le total qui est juste.
8. **Âge** : année civile de la rentrée moins année de naissance, comme le demande la notice de
   l'enquête. Les tableaux « nés en » prennent l'année de naissance.
9. **Chiffres calculés, non retouchés.** Un tableau calculé ne se corrige pas dans le rapport :
   on corrige la donnée. Seuls les effectifs probables à la rentrée et les textes se saisissent
   dans le rapport.
10. **Un rapport arrêté ne bouge plus** ; rouvrir crée une nouvelle version.
11. **Le handicap reste confidentiel** : chiffres sans nom, droit propre, pas de liste.
12. Les statistiques ne font que lire ; elles ne demandent pas d'être seul sur le réseau et ne
    retiennent aucune écriture.

## 11. Droits

Cinq permissions nouvelles, nommées à l'étape 1 selon `docs/matrice-permissions.md` ; chacune
entre au catalogue avec l'étape qui s'en sert :
- `statistics:read` : consulter les statistiques et les exporter — direction, secrétariat
  (étape 1) ;
- `school_profile:manage` : tenir la fiche de l'établissement et les résultats aux examens —
  direction, secrétariat (étapes 2 et 5) ;
- `inventory:manage` : tenir l'inventaire — direction, secrétariat à la livraison ; un droit à
  part, pour que l'école le donne à la personne qu'elle charge de l'inventaire, quel que soit son
  rôle (réserve du porteur sur T-103 ; étape 4) ;
- `school_reports:manage` : rédiger, arrêter et rouvrir les rapports — direction (étape 6) ;
- `students:disability` : lire et saisir le handicap d'un élève — direction seule à la
  livraison (étape 3).

Les maîtres n'ont aucune de ces permissions. Les champs nouveaux des fiches de l'élève et du
personnel suivent les droits existants de ces fiches.

## 12. Ordre de réalisation

Une étape, une session. Chaque étape se termine comme les précédentes : essais verts, captures
montrées, accord du porteur ; une seule compilation complète par étape, `target` élagué avant.

1. **Statistiques avec ce qui existe.** Calcul à une date, onglets Élèves et Personnel sur les
   données actuelles, détail d'un chiffre, export Excel, droit de consultation ; essai des
   totaux contre un décompte indépendant ; essai « école vide ».
2. **Fiche de l'établissement et rubriques.** Catalogue livré, réponses, reprise d'une année
   sur l'autre, rubriques ajoutées par l'école, onglet Établissement, migration.
3. **Compléments élèves, classes, personnel.** Année de recrutement, handicap et son droit,
   prise en charge par l'État, fonctionnement et pédagogie de la classe, statut déduit ou
   saisi, spécialité, diplôme le plus élevé ; saisie en grille ; colonnes facultatives de
   l'import ; tableaux correspondants des statistiques.
4. **Inventaire.** Catalogue, relevé de l'année, reprise, onglet Locaux et équipements.
5. **Examens et fin d'année.** Résultats par élève et totaux, onglet Examens et fin d'année,
   effectifs probables.
6. **Rapports de rentrée.** Rapport succinct et rapport de rentrée : textes, reprise, tableaux,
   aperçu, impression, arrêt.
7. **Rapport de fermeture et fiche scolaire.** Vérification des tableaux du modèle dans Word,
   puis mêmes mécanismes qu'à l'étape 6.
8. **Enquête, tableau libre, livraison.** Classeur « Données pour l'enquête » du secondaire ;
   feuille du fondamental si le questionnaire est arrivé (§7) ; tableau libre (lignes et
   colonnes au choix parmi classe, niveau, cycle, sexe, âge, statut, rubrique de la fiche) ;
   école d'essai et outil de démonstration complétés ; manuel ; essais à la main ; validation.

Les étapes 2, 4 et 5 ne dépendent pas les unes des autres. Le tableau libre est la seule pièce
qui peut glisser à une version suivante sans rien retirer aux rapports.

## 13. Questions de terrain au porteur du projet

À la suite de T-97. Chacune a une recommandation ; la phase commence sans attendre les
réponses, sur les recommandations, sauf T-99 qu'il faut avant l'étape 6.

### T-98 — Les documents que l'école remet vraiment

Quels documents une école remet-elle chaque année, à qui et quand : rapport succinct (quinze
jours après la rentrée), rapport de rentrée, rapport de fermeture, enquête de la CPS, fiche
scolaire ? Y en a-t-il d'autres, par mois ou par trimestre ?

*Recommandation* : ces cinq-là dans la 2.0. Un rapport mensuel ou trimestriel se fera plus
tard, sur un exemplaire réel, avec les mêmes tableaux.

### T-99 — Un rapport pour toute l'école, ou un par cycle

Une école qui a un fondamental et un lycée remet-elle un seul rapport, ou un au CAP pour le
fondamental et un à l'académie pour le lycée ? Le jardin a-t-il son propre rapport ?

*Recommandation* : un rapport pour le fondamental (premier et second cycles ensemble, comme
dans le modèle), un autre pour le lycée quand l'école en a un, rien de séparé pour le jardin
dont les effectifs figurent en tête du rapport du fondamental.

### T-100 — Les statuts du personnel

Les formulaires ne s'accordent pas : le rapport de rentrée a cinq statuts, le rapport de
fermeture en cite trois, une école y écrit « prestataire », l'enquête en a sept. Dans une école
privée, comment classez-vous un maître payé par l'école à l'année, un vacataire, un
fonctionnaire qui donne des heures ?

*Recommandation* : une seule liste de huit statuts dans l'application (§8.3), regroupée d'elle-
même selon chaque formulaire ; par défaut, un maître de l'école est « contractuel de l'école »
et compte dans « contractuel local » ; un fonctionnaire qui donne des heures compte comme
fonctionnaire de l'État.

### T-101 — Les résultats aux examens

Après le DEF ou le baccalauréat, l'école veut-elle noter le résultat de chaque élève (admis ou
non, mention), ou seulement les totaux par sexe ?

*Recommandation* : élève par élève, en grille, parce que la fiche scolaire demande le résultat
du DEF et que l'école s'en sert pour sa publicité ; les totaux restent possibles pour qui ne
veut pas.

### T-102 — Le handicap des élèves

L'enquête demande le nombre d'élèves handicapés par type et par degré. L'école veut-elle le
noter sur la fiche de l'élève, ou seulement saisir des totaux une fois l'an ?

*Recommandation* : sur la fiche, facultatif, visible de la direction seule, jamais imprimé avec
un nom. L'école qui ne veut pas le noter laisse vide et remplit l'enquête à la main.

### T-103 — L'inventaire

Qui compte les tables-bancs et les manuels, et quand : une fois à la rentrée, ou à la rentrée
et à la fermeture ? L'école veut-elle suivre les manuels prêtés aux élèves ?

*Recommandation* : un relevé par an, repris de l'année d'avant et corrigé quand il change, tenu
par le secrétariat ; pas de suivi des prêts dans la 2.0.

*Réponse du porteur (12 novembre 2026)* : validée, avec une réserve : l'inventaire n'est pas
forcément tenu par le secrétaire, mais par toute personne que l'école en charge — un droit à part,
pas lié au rôle (§8.4, §11).

### T-104 — Sous quelle forme le CAP reçoit les rapports

Papier signé, PDF, ou fichier Word que le CAP modifie ?

*Recommandation* : PDF imprimé et signé, plus l'Excel des tableaux. Un export Word ne se fait
que si le CAP l'exige.

### T-105 — L'année de recrutement

Est-ce bien l'année où l'élève est entré en 1re année, dans n'importe quelle école ? Pour un
élève arrivé par transfert, l'école la connaît-elle ?

*Recommandation* : oui ; déduite quand l'élève est entré en 1re année dans l'école, saisie
sinon, laissée vide quand on ne sait pas.

### T-106 — Les formulaires qui manquent

Pouvez-vous obtenir le questionnaire annuel de la CPS pour le fondamental et le préscolaire, la
fiche SIGE, et l'édition la plus récente des questionnaires du secondaire ?

*Recommandation* : on avance sans eux ; leur place est prévue (§7) et une session suffit pour
les intégrer quand ils arrivent.

### T-107 — Tableau libre et rubriques de l'école

Confirmez-vous les deux : l'école ajoute ses propres rubriques à sa fiche, et compose un
tableau en choisissant lignes et colonnes ?

*Recommandation* : les rubriques de l'école dès l'étape 2, elles ne coûtent presque rien ; le
tableau libre à l'étape 8, et il passe à la version suivante s'il devait retarder la 2.0.

### T-108 — En-tête, signataire et licence

Qui signe (directeur, directrice, proviseur, promoteur) ? Faut-il un cachet ? Les rapports
font-ils partie de toute licence, ou d'un module vendu à part ?

*Recommandation* : qualité du signataire en réglage, emplacement vide pour le cachet ; les
rapports et les statistiques dans toute licence, sans module.

## 14. Journal des révisions

- v1.0, 12 novembre 2026 : première rédaction, d'après les formulaires remis le 11 novembre
  2026 et les décisions des 11 et 12 novembre 2026.
- v1.1, 14 novembre 2026 : réponses du porteur à T-98 à T-108 reportées, réserve sur T-103
  (inventaire tenu par la personne que l'école désigne, droit à part) ; cinq permissions nommées
  (§11).
