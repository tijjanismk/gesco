# SPEC_PHASE_10.md — Clôture de l'année, durcissement, installeur, licences

**Projet** : ERP de gestion scolaire — Tauri 2 + Rust + React/TypeScript
**Phase** : 10 — clôture d'une année et démarrage de la suivante (UC-06), restauration éprouvée (UC-07), durcissement, installeur, licences
**Statut** : v1.0 — spécification engagée le 16 octobre 2026 après les réponses du porteur du projet à T-61 à T-69
**Documents parents** : `PROJECT_BLUEPRINT.md` v0.23 (§10.8, §22.2, ADR-012, Q-30, §26.5), `ARCHITECTURE_RULES.md` v0.3, `SPEC_PHASE_2.md` (inscriptions), `SPEC_PHASE_3.md` (facturation, caisse), `SPEC_PHASE_4.md` (bulletins, moyenne annuelle), `docs/questions-terrain.md` (T-65 à T-69)
**Date** : 16 octobre 2026

---

## 0. Objet et critère de fin

### 0.1 Ce que produit cette phase

Le livrable du blueprint (§22.2) : **UC-06 et UC-07 — clôture d'année et restauration**, et un GESCO qui s'installe et se remet à une école. Le porteur du projet installe GESCO le 16 octobre 2026 sur un poste d'essai, pour une école qui a un jardin et un 1er cycle : la phase commence par ce qu'il lui faut ce jour-là (licences, installeur), la clôture d'année suit.

À la fin de la phase :
- l'école reçoit un installeur qui s'installe sans droits d'administrateur et sans connexion, et une licence numérotée qui s'attache au poste ;
- en fin d'année, GESCO propose la décision de chaque élève d'après la moyenne de passage que l'école a reprise du CAP ou de l'académie ; la direction la valide ou la corrige ;
- l'année se clôture d'un geste, après une sauvegarde : elle ne se modifie plus, sauf réouverture tracée ;
- chaque élève qui revient est préparé dans sa classe de l'année suivante ; la secrétaire valide sa réinscription quand le parent vient, sans rien ressaisir ;
- ce qu'un élève doit encore le suit dans la nouvelle année ; un parent peut payer d'avance les frais de l'année suivante ;
- une école de taille réelle tient dans GESCO sans lenteur, et sa sauvegarde se restaure sur un poste neuf.

### 0.2 Décisions déjà prises qui s'appliquent

- **Licences** (T-69) : cent licences libres numérotées, GESCO-0001 à GESCO-0100, chacune attachée au poste qui l'installe ; sans licence, tout marche, un bandeau le rappelle (§26.5) ; aucune situation de licence ne bloque l'école ni ses données.
- **Décisions** (T-65, Q-30) : le CAP ou l'académie fixe la règle de passage, l'école la reporte en réglage ; GESCO propose, la direction valide ; rien ne passe sans validation ; « non statué » tant qu'elle n'a pas tranché.
- **Réinscription** (T-66) : préparée par GESCO, validée au guichet élève par élève ou par classe entière ; la facture de la nouvelle année naît à la validation.
- **Impayés** (T-67) : l'arriéré suit l'élève dans la nouvelle année, visible au guichet, sans bloquer la réinscription ; un versement règle l'arriéré d'abord.
- **Avances** (T-68) : dès que l'année suivante est préparée, un versement se garde en avance au nom de l'élève et s'applique à sa première facture.
- ADR-012 : une année clôturée est en lecture seule, sauf réouverture par `academic_year:reopen`, tracée. R-14 : rien ne se supprime.
- T-36 : un impayé ne bloque jamais un document ; il ne bloque pas non plus une réinscription.

### 0.3 Critère de fin, vérifiable

1. Une licence libre s'installe sur n'importe quel poste et s'y attache ; restaurée sur un autre poste, la base la signale à réinstaller sans rien bloquer ; l'outil de l'éditeur émet un lot numéroté sans jamais réémettre un numéro.
2. La décision proposée suit la moyenne annuelle et la moyenne de passage du cycle ; au jardin, l'enfant passe à la section suivante ; la direction valide ou corrige, élève par élève ou classe entière ; la dernière année d'un cycle propose la sortie du cycle.
3. L'année ne se clôture pas tant qu'une décision reste non statuée ; la clôture fait une sauvegarde, achève les inscriptions et met l'année en lecture seule ; toute écriture sur une année clôturée est refusée, sauf après réouverture tracée.
4. À la clôture, chaque élève admis ou redoublant est préparé dans la classe de son niveau suivant (ou du même niveau) de l'année préparée ; un exclu ne l'est pas ; la validation au guichet crée la facture sans ressaisie.
5. Le reste dû de l'année close devient un arriéré dans la nouvelle année, réglé avant les nouveaux frais ; il paraît au guichet et sur la fiche.
6. Un versement pour l'année préparée, avant toute facture, se garde en avance et s'impute sur la première facture.
7. Une école de 1 500 élèves en 40 classes se calcule (factures, bulletins d'une période) dans des temps bornés, mesurés par un test.
8. L'installeur s'installe pour l'utilisateur courant, WebView2 compris, et une version plus récente s'installe par-dessus sans perte de données.
9. Toute écriture est auditée avec son événement ; les tests des phases précédentes passent sans changement.

## 1. Périmètre

**Inclus** : licences libres et lot de l'éditeur ; moyenne de passage par cycle ; décisions de fin d'année ; clôture, lecture seule, réouverture ; réinscriptions préparées et validées ; arriérés ; avances sur l'année suivante ; test de charge ; installeur en version 0.10.0.

**Exclu** :
- Remontées administratives (CPS, statistiques de l'académie) : phase 9.
- Archivage d'une année sur un support externe au-delà de la sauvegarde ordinaire : la sauvegarde de clôture en tient lieu.
- Mode réseau entre plusieurs postes d'une école (Q-13) : phase 11, conditionnelle.
- Signature de code de l'installeur (certificat Authenticode) : à acheter par l'éditeur ; sans elle, Windows avertit au premier lancement de l'installeur.

## 2. Licences libres

Le fichier de licence porte l'empreinte `LIBRE` à la place de celle d'un poste. À l'installation, le poste note sa propre empreinte avec la licence : la licence lui est attachée. Une base restaurée sur un autre poste voit sa licence « à réinstaller » : il suffit d'y réinstaller le fichier. Le journal d'audit garde le numéro, l'édition et l'empreinte du poste à chaque installation.

L'outil de l'éditeur : `gesco-licence lot --cle <clé privée> --nombre 100 --dossier <dossier> [--premier 1] [--prefixe GESCO]` émet `GESCO-0001.lic` … et `registre.csv` (numéro, fichier, édition, date d'émission, école, date de remise), où l'éditeur note l'école qui reçoit chaque numéro. Rien n'est écrit si un seul des fichiers existe déjà.

## 3. Décisions de fin d'année

**Moyenne de passage** — un réglage par cycle noté (`passage.premier_cycle`, `passage.second_cycle`, `passage.lycee`, `passage.technique`), en centièmes de point, livré à la moitié du barème : 5 sur 10 au 1er cycle, 10 sur 20 ailleurs. L'école y reporte la consigne du CAP ou de l'académie ; il reste dans le barème du cycle. Le jardin n'en a pas.

**Proposition** — d'après la moyenne annuelle que porte le bulletin de la dernière période (T-33) : admis à la moyenne de passage, redouble en dessous ; au jardin, l'enfant passe à la section suivante ; sans moyenne annuelle (dernier bulletin pas encore calculé), rien n'est proposé. La proposition n'est jamais écrite seule.

**Décision** — posée par la direction (`academic_year:close`), élève par élève (admis, redouble, exclu, rattrapage, ou non statué pour la retirer) ou en validant d'un geste les propositions d'une classe pour les élèves encore sans décision. Elle se pose sur l'année ouverte, pour un élève inscrit jusqu'à la fin de l'année ; chaque décision est auditée et émet `DecisionFinAnneePosee`. Un rattrapage laisse la décision en attente : son résultat se pose ensuite.

**Écran « Fin d'année »** (groupe Scolarité) : les moyennes de passage des cycles enseignés, les classes avec leurs admis, redoublants, exclus et décisions à poser ; la classe choisie montre chaque élève, sa moyenne annuelle, la proposition et la décision.

## 4. Clôture de l'année

Refusée tant qu'un élève inscrit jusqu'à la fin de l'année n'a pas de décision définitive (non statué ou rattrapage). Elle commence par une sauvegarde (`avant_cloture_annee`), puis, dans une seule transaction : chaque inscription validée s'achève avec sa décision, chaque brouillon jamais validé s'abandonne (« Année clôturée sans validation de l'inscription. »), l'année passe `cloturee`. Événement `AnneeScolaireCloturee`.

**Lecture seule** (ADR-012) — sur une année clôturée ou archivée, GESCO refuse (`annee.cloturee`) : la création, la saisie, la validation et le déverrouillage des évaluations, l'import de notes, le relevé et la justification des absences, le calcul, la validation, la publication et la correction des bulletins, les appréciations, les moyennes reprises, les décisions. Les classes, les frais et le programme l'étaient déjà. Les paiements de ce qui reste dû restent possibles (§6).

**Réouverture** — `academic_year:reopen` (Administrateur), avec un motif inscrit au journal, tant qu'aucune autre année n'est ouverte : les inscriptions achevées redeviennent validées, leurs décisions gardées. Événement `AnneeScolaireRouverte`.

## 5. Réinscriptions préparées

L'année suivante se crée en préparation dans « Année scolaire », comme avant. Depuis « Fin d'année » :
- **Reprendre les classes** : chaque classe de l'année qui précède, absente de l'année suivante par son libellé, y est créée avec son niveau et sa série ;
- **Préparer les réinscriptions** : chaque élève admis ou redoublant de l'année qui précède reçoit une inscription en brouillon dans l'année suivante, au niveau suivant s'il est admis, au même s'il redouble, dans la classe de même rang parmi celles du niveau (la 7e B revient en 8e B), sinon la dernière. Il est marqué redoublant s'il redouble, l'inscription date de la rentrée. Un élève qui a déjà une inscription dans l'année suivante est laissé tel quel ; l'admis du dernier niveau (12e année, CAP 2, BT 4) quitte l'école ; un niveau sans classe est signalé, la préparation se relance après l'avoir créée.

La secrétaire valide ensuite au guichet, élève par élève ou classe entière, dans « Inscriptions », l'année suivante choisie : la validation crée la facture sur la grille de frais de l'année suivante, comme toute validation. Une inscription se crée et se valide désormais dans l'année suivante en préparation (T-66), datée au plus tôt de sa rentrée.

## 6. Arriérés et avances

**Arriérés** (T-67) — la facture de l'année close reste ce qu'elle est : rien n'est recopié. Ce qu'elle doit encore suit l'élève :
- dans la liste des impayés, après ceux de l'année en cours, chaque arriéré porte son année (« 7e A · 2026-2027 ») ;
- au guichet, la caisse propose d'abord la plus ancienne inscription qui doit encore de l'argent, avec la mention « Arriéré de l'année 2026-2027 : il se règle avant les frais de l'année » ; quand plusieurs années doivent, la caissière peut en choisir une autre, sans blocage (T-36) ;
- sur la fiche de l'élève, la situation financière de chaque année.

**Avances** (T-68) — un parent paie d'avance en faisant valider la réinscription avant la rentrée : la facture de l'année suivante existe alors, le versement s'y impute comme tout versement, sur la caisse du jour. Aucune somme ne flotte sans facture.

## 7. Durcissement

- Test de charge : une école de 1 500 élèves en 40 classes ; la facturation de toutes les inscriptions et le calcul des bulletins d'une période se mesurent et restent sous un plafond écrit dans le test.
- Restauration sur poste neuf : le test de bout en bout de la phase 0.5 (UC-07) reste dans la suite.
- Clôture : une sauvegarde la précède toujours ; l'année close se rouvre seulement avec un motif.

## 8. Installeur

Installeur NSIS de Tauri, pour l'utilisateur courant, en français, WebView2 hors ligne inclus ; GESCO s'installe dans `%LOCALAPPDATA%\GESCO`, ses données restent dans `%APPDATA%\ml.gesco.poste` et survivent à une désinstallation comme à une mise à jour. Version 0.10.0. Construit par `pnpm installeur` depuis PowerShell.

## 9. Ordre de réalisation

1. Licences libres ; lot de cent licences ; installeur de la version d'essai
2. Moyenne de passage et décisions de fin d'année
3. Clôture de l'année, lecture seule, réouverture
4. Réinscriptions préparées et validées au guichet
5. Arriérés et avances sur l'année suivante
6. Durcissement : test de charge, restauration sur poste neuf
7. Navigation, captures, école d'essai, installeur final, validation

## 10. Journal des révisions

| Version | Date | Modification |
|---|---|---|
| v1.0 | 2026-10-16 | Création après les réponses du porteur du projet du 16 octobre 2026 (T-61 à T-69) : licences libres numérotées, décisions validées par l'école sur la règle du CAP ou de l'académie, réinscription préparée, arriérés, avances |
