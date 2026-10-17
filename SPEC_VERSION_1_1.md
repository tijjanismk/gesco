# SPEC_VERSION_1_1.md — Ce qui manquait aux phases construites

**Projet** : ERP de gestion scolaire — Tauri 2 + Rust + React/TypeScript
**Version** : 1.1 — complète la version 1.0 avec les étapes restées non faites des phases 2, 4, 6, 8 et 10
**Statut** : v1.0 — engagée le 17 octobre 2026, après les réponses du porteur du projet à T-70 à T-73 (toutes les recommandations retenues)
**Documents parents** : `PROJECT_BLUEPRINT.md` (§12.4, §22.2), `SPEC_PHASE_2.md`, `SPEC_PHASE_4.md`, `SPEC_PHASE_6.md`, `SPEC_PHASE_8.md`, `SPEC_PHASE_10.md`, `docs/questions-terrain.md` (T-37, T-41, T-70 à T-73)
**Date** : 17 octobre 2026

---

## 0. Objet

La version 1.0 est sortie le 16 octobre 2026. Le porteur du projet a décidé le 17 octobre 2026 : les phases 5 (SMS), 9 (rapports) et 12 (e-learning) iront dans la **version 2** ; la phase 11 attend une école qui la demande ; d'abord, **compléter les étapes restées non faites** des phases construites. C'est la version 1.1, développée sur la branche `developpement` ; la branche `1.0.x` ne reçoit que des corrections.

Restent hors de la 1.1, par décision : l'essai sur machine vierge (abandonné le 10 octobre 2026), l'évaluation par compétences du jardin (Q-03), la comptabilité générale SYSCOHADA, la signature de l'installeur (un certificat à acheter).

## 1. Étapes

1. **Autres recettes** (SPEC_PHASE_8 §2) : une recette hors des versements des parents — don, location de salle — avec sa catégorie de recette, son payeur, son montant, son compte et son mode ; en espèces, une entrée dans la session de caisse ouverte ; annulation motivée. Elle entre dans le livre de son compte, le journal du mois et le budget.
2. **Photo à la webcam** (SPEC_PHASE_2 §4) : la photo de l'élève se prend avec la caméra du poste, au lieu d'un fichier ; mêmes contrôles (PNG ou JPEG, 100 Ko au plus).
3. **Discipline** (T-70) : dossier de discipline de l'élève — avertissement, blâme, retenue, convocation des parents —, daté, motivé, avec qui l'a décidé ; une sanction saisie par erreur s'annule avec un motif. L'avertissement et le blâme du conseil de classe, rattachés à une période, paraissent sur le bulletin de la période. L'exclusion temporaire et la radiation restent celles de la phase 4.
4. **Heures des vacataires d'après l'emploi du temps** (T-41) : les heures prévues d'un mois se calculent d'après les séances du vacataire et les jours de classe du mois ; l'école peut toujours les corriger.
5. **Archive du journal d'audit** (§12.4, T-73) : à la clôture de l'année, le journal de l'année s'écrit dans un fichier signé, rangé avec la sauvegarde de clôture et copié sur ses destinations ; il reste dans la base ; GESCO vérifie la signature d'une archive choisie.
6. **Absences par séance** (T-71) : réglage de l'école, éteint par défaut. Activé, l'absence se relève à chaque séance de l'emploi du temps ; absent à toutes les séances d'une demi-journée, l'élève compte une demi-journée, qui se justifie comme aujourd'hui.
7. **Import des relevés** (T-72) : un relevé Excel ou CSV d'un compte ; l'école indique une fois les colonnes de la date, du montant (ou du débit et du crédit) et de la référence, gardées pour ce compte ; GESCO coche les lignes du livre retrouvées par la référence, puis par le montant et la date (trois jours d'écart au plus), une ligne pour une ligne, et montre ce qui reste.
8. **Import du personnel** (SPEC_PHASE_6 §1) : une liste du personnel en Excel, sur un modèle fourni, vérifiée ligne par ligne avant l'enregistrement, comme l'import des élèves.
9. **Validation** : tests, captures, école d'essai, version 1.1.0, installeur, étiquette `v1.1.0`.

## 2. Critère de fin

Chaque étape a son test de bout en bout par la frontière et sa capture ; toute écriture est auditée avec son événement ; rien ne se supprime (R-14) ; les tests de la version 1.0 passent sans changement.

## 3. Journal des révisions

| Version | Date | Modification |
|---|---|---|
| v1.0 | 2026-10-17 | Création après les réponses du porteur du projet à T-70 à T-73 ; phases 5, 9 et 12 renvoyées à la version 2 |
