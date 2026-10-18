# SPEC_PHASE_5.md — SMS aux familles, remises, guichet

**Projet** : ERP de gestion scolaire — Tauri 2 + Rust + React/TypeScript
**Phase** : 5 — moteur de notification, canal SMS par clé modem GSM, relances d'impayés, avis de bulletin, absences du jour, remise des documents et convocations, écran guichet, consultation par SMS entrant
**Statut** : v1.4 — engagée le 17 octobre 2026 après les réponses du porteur du projet à T-74 à T-77 (toutes les recommandations retenues)
**Documents parents** : `PROJECT_BLUEPRINT.md` (§3.10, §8.5bis, §19.5, §22.2, §25.3 à §25.8), `SPEC_PHASE_2.md` (responsables, téléphones), `SPEC_PHASE_3.md` (échéances, impayés), `SPEC_PHASE_4.md` (absences, bulletins publiés), `SPEC_VERSION_1_1.md` (discipline), `docs/questions-terrain.md` (T-74 à T-77)
**Date** : 17 octobre 2026

---

## 0. Objet et critère de fin

### 0.1 Ce que produit cette phase

Le livrable du blueprint (§22.2) : **la relance automatisée du recouvrement, le guichet et la consultation des parents sans serveur**. C'est la première phase de la version 2, développée sur la branche `developpement`.

Le SMS ne transporte pas le contenu, il transporte **l'avis** (§8.5bis) : « le bulletin est disponible, passez le retirer et le signer ». L'avis amène le parent à l'école, où il règle aussi la scolarité. Le canal de notification est d'abord un levier de recouvrement, ensuite un canal d'information.

À la fin de la phase :
- une clé modem GSM branchée sur le poste envoie les SMS de l'école, avec sa puce prépayée, sans internet ;
- trois messages partent d'eux-mêmes : bulletin à retirer, relance d'impayé, absence du jour ; chacun s'éteint ou se règle dans les réglages de l'école ;
- les convocations et les messages de sanction sont préparés par GESCO, relus, et envoyés seulement par quelqu'un qui le décide ;
- chaque numéro sait s'il est joignable, chaque responsable peut refuser un type de message sans perdre les autres ;
- le crédit de la puce est surveillé : à court de crédit, la file s'arrête et la direction est avertie, au lieu d'échouer en silence ;
- un bulletin publié se remet, se signe, ou reste « non retiré » ; une convocation sait par quel canal elle est partie et si elle a été reçue ;
- au guichet, une seule recherche montre tout ce qui concerne la famille et permet de tout régler sans changer d'écran ;
- en fin de phase, un parent interroge GESCO par SMS avec son code confidentiel : « SOLDE code », « ABS code », « NOTE code ».

### 0.2 Décisions déjà prises qui s'appliquent

- **Matériel** (T-74) : une **clé modem GSM USB** qui expose un port série en commandes AT, avec une puce prépayée de l'école. Les clés qui se présentent comme une carte réseau (mode routeur) ne conviennent pas (C-1) ; les modèles essayés sont notés dans `docs/modems-valides.md`. La passerelle HTTP d'un opérateur reste une deuxième implémentation possible derrière la même interface, non faite dans cette phase.
- **Messages automatiques** (T-75) : bulletin à retirer, relance d'impayé, absence du jour. **Les convocations et les sanctions ne partent jamais seules** : GESCO prépare le message, une personne le relit et décide de l'envoyer (§25.6).
- **Langue** (T-76) : **français sans accents**. Le texte est translittéré en alphabet GSM-7 avant l'envoi (« eleve », « etait ») : 160 caractères par SMS au lieu de 70 (C-3). L'écran montre toujours le nombre de SMS que coûtera un message. Le fait et sa formulation restent séparés (§25.8) : une autre langue s'ajoutera par un autre catalogue, sans toucher aux règles.
- **Consultation par SMS entrant** (T-77) : oui, **en fin de phase 5**, une fois l'envoi stabilisé, avec un code confidentiel propre à chaque responsable (§25.4, §25.5).
- **Canal abstrait, file persistante** (§25.3) : événement → règle → message (fait et destinataire) → file → canal. Une seule implémentation réelle, le modem ; un canal simulé pour les tests, les captures et l'école d'essai.
- **Données exposées par SMS** (§8.5bis) : solde et prochaine échéance, absences, document à retirer, moyenne générale (activée par défaut), rang (désactivé par défaut) ; jamais d'appréciation, de sanction ni de décision du conseil de classe.
- R-14 : rien ne se supprime ; un message en attente s'annule avec un motif, un message parti reste au journal.

### 0.3 Critère de fin, vérifiable

1. Avec le canal simulé, un bulletin publié met en file un avis pour chaque responsable qui reçoit les notifications et n'a pas refusé ce type de message ; le texte est en GSM-7, d'un seul SMS.
2. Une échéance échue depuis le délai réglé met en file une relance pour le payeur ; la relance se répète à l'intervalle réglé, jamais au-delà du nombre réglé, et cesse dès que l'échéance est soldée.
3. Une absence non justifiée relevée dans la journée met en file un avis du jour ; corrigée avant le délai de grâce, l'avis s'annule sans partir.
4. Une convocation ou une sanction prépare un message qui attend une décision ; rien ne part sans le geste d'une personne qui en a le droit.
5. La file survit à la fermeture de GESCO et reprend à l'ouverture ; hors de la fenêtre d'envoi ou au-delà du débit réglé, les messages attendent ; un message trop vieux pour avoir un sens expire sans partir.
6. Avec la clé modem, un SMS part, son accusé de remise revient et marque le numéro joignable ; un numéro qui échoue plusieurs fois passe injoignable et ne consomme plus de crédit.
7. Le crédit lu par USSD sous le seuil, ou une suite d'échecs de crédit, arrête la file et avertit la direction au tableau de bord ; la file repart quand le crédit revient.
8. Un bulletin publié est « à remettre » ; au guichet, il se remet à un responsable, à l'élève ou à une personne autorisée, se signe ; non retiré à J+7, il paraît dans la liste croisée avec les impayés.
9. Au guichet, la recherche d'un élève montre la situation financière de toute la fratrie, les documents à remettre, les convocations en cours, les absences récentes non justifiées ; encaisser, remettre, signer, justifier se font depuis cet écran.
10. Un SMS « SOLDE code » venu d'un numéro du responsable reçoit le reste à payer et la prochaine échéance ; un code faux, un numéro inconnu ou un quota dépassé ne vident pas le crédit ; chaque consultation est au journal d'audit.
11. Toute écriture est auditée avec son événement ; les tests des phases précédentes passent sans changement.

## 1. Périmètre

**Inclus** : joignabilité et consentement des numéros ; moteur de notification et file persistante ; translittération GSM-7 et compte des SMS ; canal modem GSM (envoi, accusés de remise, réception, crédit par USSD) et canal simulé ; trois messages automatiques ; messages préparés à décider ; remise des documents ; convocations ; écran guichet ; code d'accès des responsables ; consultation par SMS entrant.

**Exclu** :
- Passerelle HTTP d'un opérateur (Orange, Moov) : l'interface est prête, l'implémentation attendra une école qui en a besoin (gros volumes, puce suspendue).
- Deuxième clé modem sur un autre opérateur (C-6) : après mesure de la répartition réelle des numéros sur une école pilote.
- SMS en bambara et messages vocaux (§25.8) : la couture est là, pas le catalogue.
- Liens WhatsApp (`wa.me`) : une convocation peut noter ce canal à la main ; aucune génération de lien dans cette phase.
- Campagnes libres (un texte quelconque à toute une classe) : seuls les types de messages définis ici.
- Timeline de l'élève (§25.7).

## 2. Numéros, consentement, joignabilité

Les numéros sont déjà enregistrés sous forme canonique `+223…` (phase 2) ; la case « reçoit les notifications » du lien élève-responsable (phase 2) reste le premier filtre.

**Joignabilité** (§25.6) — chaque numéro porte un état : non vérifié, joignable, injoignable, invalide. Un accusé de remise le rend joignable ; trois échecs de remise d'affilée le rendent injoignable ; un refus du réseau sur le format le rend invalide. Un numéro injoignable ne reçoit plus rien jusqu'à ce qu'on le corrige ou qu'on le remette à l'essai depuis la fiche du responsable.

**Consentement** — par responsable et par type de message (bulletin, impayé, absence, convocation), coché par défaut. Un parent qui ne veut plus de relances est retiré des relances, immédiatement, sans perdre l'avis de bulletin. Chaque changement est audité.

**Destinataires** — l'avis de bulletin et l'avis d'absence vont à chaque responsable qui reçoit les notifications ; la relance d'impayé va au payeur ; une convocation, au responsable choisi par qui la prépare. Un message par numéro joignable : un responsable à deux numéros reçoit sur le premier joignable de sa liste.

## 3. Moteur de notification et file

**Message** — type, fait (les données, jamais la phrase), destinataire (responsable et numéro), élève, texte formulé au moment de la mise en file, nombre de SMS, état, tentatives, date d'expiration. États : **à décider → en attente → envoyé → remis**, **en attente → échoué** (après les essais), **à décider ou en attente → annulé** (motif), **en attente → expiré**.

**Règles** — une par type automatique, déclenchée par un événement de la transaction (`BulletinPublie`, échéance échue constatée, `AbsenceConstatee`) ; la mise en file se fait dans la même transaction que l'événement, jamais par un déclencheur. Un même fait ne met en file qu'un message par destinataire (un index unique le garantit).

**Formulation** — un catalogue de modèles en français, avec des variables (`{eleve}`, `{classe}`, `{periode}`, `{montant}`, `{date}`, `{ecole}`), que l'école peut reformuler dans les réglages ; l'écran de réglage montre le texte d'exemple, son nombre de caractères et de SMS. Le texte est translittéré en GSM-7 : accents retirés, caractères hors alphabet remplacés ; un message qui dépasserait trois SMS est refusé à l'enregistrement du modèle. Le nom de l'école est le réglage `sms.signature` (le sigle, par défaut).

Modèles proposés :
- bulletin : « {ecole} : le bulletin de {periode} de {eleve} ({classe}) est disponible. Passez le retirer et le signer a l'ecole. »
- impayé : « {ecole} : il reste {montant} a payer pour {eleve} ({classe}), echu depuis le {date}. Merci de passer a l'ecole. »
- absence : « {ecole} : {eleve} ({classe}) etait absent(e) le {date}, {demi_journee}, sans justification. »
- convocation : « {ecole} : vous etes convoque(e) le {date} a {heure} au sujet de {eleve}. Motif : {motif}. »

**File** — persistante dans la base, traitée en tâche de fond pendant que GESCO est ouvert sur le poste où la clé est branchée ; l'utilisateur n'attend jamais. Réglages : débit (`sms.debit_par_minute`, 6 par défaut), heures d'envoi (`sms.fenetre_envoi`, « 7-20 » : de 7 h à 20 h ; rien la nuit), plafond mensuel (`sms.plafond_mensuel`, 0 : sans plafond). Réessai d'un échec réseau trois fois, espacé. Expiration : avis d'absence à la fin du jour, relance après 7 jours, avis de bulletin après 15 jours.

**Écran « Messages »** (groupe Communication) : à décider, en attente, envoyés, échoués, avec filtres ; état de la clé, crédit lu, consommation du mois ; un message à décider se relit, se corrige, s'envoie ou s'annule ; un message échoué se renvoie.

## 4. Canal modem GSM

**Interface `Canal`** — envoyer un message, lire les accusés, lire les messages reçus, lire le crédit, état. Deux implémentations : le modem et le canal simulé (qui garde tout dans un fichier et peut simuler un échec, un crédit épuisé, un SMS entrant).

**Modem** — dans le crate `gesco-sms`, sur un port série (la dépendance de port série s'ajoute à la liste blanche). Le port est choisi dans les réglages parmi ceux du poste, avec un bouton « Essayer » qui interroge la clé (fabricant, modèle, réseau, force du signal) et envoie un SMS d'essai au numéro saisi. Mode PDU : contrôle de l'encodage GSM-7, messages en plusieurs parties, accusés de remise ; réception des SMS entrants signalée par la clé et lue aussitôt. Une clé débranchée met la file en pause, signalée, sans rien perdre.

**Crédit** (C-5) — lu par le code USSD de l'opérateur, réglé par l'école (`sms.ussd_solde`), toutes les heures pendant l'envoi et à chaque ouverture ; le montant est lu dans la réponse, la réponse brute est montrée si elle ne se lit pas. Sous le seuil (`sms.seuil_credit`), ou après cinq échecs d'envoi d'affilée attribués au crédit : **la file s'arrête**, un avertissement paraît au tableau de bord de la direction, et un SMS d'alerte part au numéro de la direction s'il reste de quoi l'envoyer. Elle repart d'elle-même quand le crédit lu repasse le seuil, ou à la main.

**Blocage par l'opérateur** (C-4) — le débit et la fenêtre étalent les envois ; un avis de bulletin pour toute l'école se répartit sur plusieurs heures. Une puce qui ne peut plus émettre se signale comme un crédit épuisé.

## 5. Messages automatiques

Chacun s'active dans les réglages (`sms.auto_bulletin`, `sms.auto_impaye`, `sms.auto_absence`), tous trois actifs par défaut une fois la clé réglée ; rien ne part tant qu'aucun canal n'est réglé.

**Bulletin à retirer** — à la publication d'un bulletin. Pas de rappel automatique : les bulletins non retirés à J+7 forment une liste (§6) d'où l'on peut renvoyer l'avis.

**Relance d'impayé** — chaque jour, à l'ouverture de GESCO : une échéance non soldée depuis `sms.impaye_delai_jours` (7 par défaut) met en file une relance au payeur, pour le total échu de l'élève (une relance par élève, pas par échéance) ; une nouvelle relance après `sms.impaye_intervalle_jours` (15), au plus `sms.impaye_nombre_max` (3) par échéance ; aucune relance pour un élève parti ou radié, ni pour un total sous `sms.impaye_montant_min` (0). Un versement qui solde avant l'envoi annule la relance en attente.

**Absence du jour** — à l'enregistrement d'une absence non justifiée (demi-journée, ou séance si l'école relève par séance) ; un avis par élève et par jour, même pour deux demi-journées. Il attend `sms.absence_delai_grace_min` (30) avant de partir : une absence corrigée ou justifiée dans ce délai l'annule.

**Messages à décider** — la convocation (§7), et, si l'école le veut (`sms.sanction_preparer`, éteint par défaut), un avis de sanction préparé à la saisie d'une sanction. Ils naissent « à décider » et ne partent qu'après relecture et envoi par une personne qui a le droit `notifications:send_manual`.

## 6. Remise des documents

**Remise** (§8.5bis) — pour chaque bulletin publié, une remise naît « à remettre ». États : **à remettre → remis → signé**, **à remettre → non retiré** (à la clôture de la période suivante). Remis à : un responsable de l'élève, l'élève, ou une personne autorisée à le retirer (phase 2) ; nom du signataire, date ; l'agent est celui de la session. Un bulletin publié n'est pas un bulletin remis : les deux états se suivent à part.

**Liste « Bulletins non retirés »** — à J+7 après publication, croisée avec les impayés de la même famille, comme le veut le blueprint : c'est souvent le même parent qui évite le même déplacement. Elle s'imprime et l'avis peut se renvoyer depuis elle.

Les autres documents à retirer (certificat de scolarité, de transfert) s'ajoutent à la liste quand ils sont demandés au guichet.

## 7. Convocations

**Convocation** (§8.5bis) — élève, responsable convoqué, motif, date d'émission, date et heure du rendez-vous, émetteur, canal (confiée à l'élève, en main propre, SMS, WhatsApp noté à la main), accusé (confirmé, transmis non confirmé, sans réponse), suite (honorée, non honorée, reportée). Une convocation confiée à l'élève est **transmise, non confirmée** : c'est ce qui permet de décider de la doubler par SMS. Choisir le canal SMS prépare un message « à décider ». Une sanction « convocation des parents » (version 1.1) propose de créer la convocation. Impression d'un billet de convocation (modèle `convocation`).

## 8. Écran guichet

**Guichet** (§19.5, groupe Scolarité, premier de la liste pour le Secrétaire et le Caissier) — une recherche unique sur le nom ou le matricule de l'élève ; l'écran montre la famille :
- situation financière de chaque enfant du même responsable (fratrie) : solde, échéances, prochaine date, derniers versements ;
- documents à remettre et bulletins à signer ;
- convocations en cours, dont celles transmises non confirmées ;
- absences récentes non justifiées ;
- messages envoyés à la famille ces trente derniers jours.

Actions sans changer d'écran : encaisser (le formulaire de versement de la phase 3), imprimer le reçu, remettre un document, enregistrer une signature, justifier une absence, marquer une convocation honorée, remettre le code d'accès.

## 9. Code d'accès et consultation par SMS entrant

Étape faite en dernier, une fois l'envoi essayé avec une vraie clé (T-77).

**Code d'accès** (§25.5) — une identité du responsable : un code de six caractères (lettres et chiffres sans ambiguïté : ni O, 0, I, 1), un état (actif, révoqué), la date de dernière utilisation. Le code se montre et s'imprime une seule fois, sur un billet remis en main propre au guichet ; la base n'en garde qu'une empreinte. Il se régénère (l'ancien est révoqué) ; il se révoque. Il servira au portail parent le jour où il existera.

**Consultation** — un SMS entrant de la forme « MOT CODE », sans souci des majuscules ni des espaces :
- `SOLDE code` : reste à payer de chaque enfant du responsable, prochaine échéance ;
- `ABS code` : absences de la période en cours, dont non justifiées ;
- `NOTE code` : moyenne générale de la dernière période publiée ; le rang seulement si l'école l'active (`sms.consultation_rang`, éteint par défaut) ;
- `AIDE` : la liste des mots.

**Contrôles** — le code et le numéro appelant doivent appartenir au même responsable. Un numéro inconnu ne reçoit aucune réponse. Un code faux reçoit une réponse « code incorrect » la première fois du jour, puis plus rien ; cinq codes faux dans la journée bloquent le numéro jusqu'au lendemain. Quota par numéro (`sms.consultation_quota_jour`, 5) ; au-delà, plus de réponse jusqu'au lendemain. Chaque consultation, servie ou refusée, est au journal d'audit : numéro, mot, élève, réponse. Activation par l'école (`sms.consultation_active`, éteinte par défaut).

## 10. Droits

Droits déjà déclarés, utilisés pour la première fois : `notifications:configure` (clé, réglages, modèles), `notifications:send` (voir la file, relancer un échec, renvoyer un avis), `notifications:send_manual` (décider et envoyer un message préparé), `documents:deliver` (remise et signature), `convocations:create`, `convocations:update`, `convocations:close`. Nouveau : `guardians:access_code` (remettre, régénérer, révoquer un code d'accès). Rôles livrés en version 10 : le Directeur a tout ; le Secrétaire remet les documents, tient les convocations, remet les codes et voit la file ; le Caissier remet les documents ; le Surveillant général tient les convocations et décide des messages préparés ; la Consultation seule lit la file.

## 11. Ordre de réalisation

1. Numéros et consentement : joignabilité, consentement par type, réglages SMS, fiche du responsable
2. Moteur de notification : messages, file persistante, translittération GSM-7, modèles, canal simulé, écran « Messages »
3. Canal modem GSM : port série, essai de la clé, envoi PDU, accusés de remise, crédit par USSD, arrêt et alerte
4. Messages automatiques : bulletin à retirer, relance d'impayé, absence du jour ; messages à décider
5. Remise des documents et liste des bulletins non retirés ; convocations
6. Écran guichet
7. Code d'accès et consultation par SMS entrant
8. Droits et rôles en version 10, tableau de bord (file, crédit, bulletins non retirés), navigation, captures, école d'essai, validation

**Décision du porteur du projet (17 octobre 2026)** : toute la phase se fait avec le canal simulé ; ce qui exige le matériel est simulé ou différé. L'étape 3 écrit le pilotage de la clé et le vérifie contre un modem simulé qui répond aux commandes AT ; l'essai sur une vraie clé, le crédit lu par USSD sur un vrai réseau et l'envoi réel de l'étape 7 attendent qu'une clé soit achetée, et sont notés comme différés.

## 12. Ce qu'il faut du porteur du projet

- Plus tard, une **clé modem GSM USB** et une **puce prépayée** pour l'essai réel, différé par décision du 17 octobre 2026. Avant d'acheter en quantité, n'en essayer qu'une (C-1).
- Le **code USSD du solde** de l'opérateur de la puce, s'il le connaît.
- Le numéro de la phase dans la version : proposé, une préversion 1.2.0 si une école veut les SMS avant la fin de la version 2.

## 13. Journal des révisions

| Version | Date | Modification |
|---|---|---|
| v1.0 | 2026-10-17 | Création après les réponses du porteur du projet du 17 octobre 2026 (T-74 à T-77) : clé modem GSM, trois messages automatiques (bulletin, impayé, absence du jour), convocations et sanctions toujours décidées par une personne, français sans accents, consultation par SMS entrant en fin de phase |
| v1.1 | 2026-10-17 | Étape 1 : noms des réglages à un seul point, comme les autres (`sms.auto_bulletin`…) ; heures d'envoi en un réglage `sms.fenetre_envoi` ; un numéro pas encore vérifié n'affiche pas d'état sur la fiche |
| v1.2 | 2026-10-17 | Décision du porteur du projet : tout en canal simulé, le matériel différé (essai réel de la clé, crédit par USSD, consultation entrante sur un vrai réseau) |
| v1.3 | 2026-10-18 | Étape 2 : canal choisi par le réglage `sms.canal` (`aucun` par défaut, `simule`) ; modèles dans les réglages `sms.modele_*`, reformulés depuis l'écran « Messages » ; avis de sanction préparé (nature `sanction`, consentement des convocations) ; SMS d'essai dans la file ; le passage de la file n'écrit pas au journal d'audit, chaque message garde son envoi, sa remise et son erreur |
| v1.4 | 2026-10-18 | Étape 3 : clé modem vérifiée contre un modem simulé qui répond aux commandes AT ; port `simule` dans les compilations de développement ; alerte envoyée directement par la clé, hors de la file ; seuil de crédit par défaut 500 F ; points à confirmer sur une vraie clé dans `ALERTES.md` |
