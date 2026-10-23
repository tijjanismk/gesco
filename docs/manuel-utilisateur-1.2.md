# GESCO — manuel d'utilisation

Pour la version 1.2.2, du 23 octobre 2026. Il vaut aussi pour les versions 1.0.0 et 1.1.0 : ce
qui n'existe qu'à partir de la 1.1 ou de la 1.2 est signalé « (1.1) » ou « (1.2) ».

GESCO gère une école sur un seul poste Windows, sans internet : les élèves et leurs familles,
les notes et les bulletins, l'argent qui entre et qui sort, le personnel et sa paie, et, si
l'école le veut, les SMS aux familles. Ce manuel
suit l'ordre du travail d'une école : mettre l'année en place, tenir la scolarité, encaisser et
dépenser, payer le personnel, protéger les données.

Chaque écran est désigné par son nom dans le menu de gauche, sous son groupe : **Scolarité →
Élèves** veut dire « l'entrée Élèves du groupe Scolarité ». Chaque utilisateur ne voit que les
écrans que son rôle lui ouvre, et seulement ceux des modules que l'école a allumés (§2).

---

## 1. Installer et ouvrir GESCO

L'installation est décrite pas à pas dans le guide `GESCO-guide-installation.pdf`, remis avec
l'installeur. En bref :

1. Lancez `GESCO_1.2.2_x64-setup.exe` sur un poste Windows 10 ou 11 (64 bits). Si Windows affiche
   « Windows a protégé votre ordinateur », cliquez sur « Informations complémentaires », puis sur
   « Exécuter quand même ».
2. Au premier démarrage, saisissez le nom de l'école, ses coordonnées et le compte
   administrateur, puis cochez les cycles que l'école enseigne.
3. **Recopiez sur papier la phrase de secours de douze mots** et rangez-la hors de l'école. Sans
   elle, une sauvegarde ne se restaure pas sur un autre ordinateur.

Une version plus récente s'installe par-dessus l'ancienne : les données de l'école sont gardées,
et GESCO fait une sauvegarde avant de mettre sa base à niveau.

### Se connecter, verrouiller, changer d'utilisateur

- Chacun se connecte avec son identifiant et son mot de passe. Après trois mots de passe faux,
  le compte se bloque un moment.
- Sans activité pendant quinze minutes, la session se verrouille ; on la rouvre avec son mot de
  passe ou son code PIN. Après vingt-quatre heures sans activité, elle se ferme. Ces deux durées
  se règlent dans Établissement, section « Sécurité ». Un mot de passe compte au moins six
  caractères ; `000000` est accepté, sauf si l'école refuse les mots de passe les plus courants. Le code PIN se choisit dans **Outils → Mon compte**.
- Le bouton « Verrouiller » en haut de l'écran ferme la session sans la perdre : un collègue peut
  se connecter sur le même poste, puis chacun retrouve la sienne.
- **Rechercher (Ctrl+K)** ouvre la recherche : tapez quelques lettres du nom d'un élève, même mal
  orthographié, puis Entrée pour ouvrir sa fiche.

### Les bandeaux en haut de l'écran

Ils signalent ce qui demande une action : certificat de la clé de secours pas encore confirmé,
aucune destination de sauvegarde amovible, aucune licence installée, paie à valider, dépenses à
accorder… Ils disparaissent quand la chose est faite.

---

## 2. Mettre l'année en place

À faire une fois, dans cet ordre, par l'administrateur ou la direction. Ces écrans sont dans le
groupe **Paramètres**, replié par défaut : cliquez sur son titre pour le déplier.

1. **Paramètres → Établissement** : la fiche de l'école (nom, statut, codes de l'académie et du
   CAP, adresse, téléphones, mentions légales imprimées au pied des documents), le logo, et les
   réglages de l'école, rangés par thème : élèves et inscriptions, sécurité, sauvegardes, frais
   et caisse, personnel, documents (format A4 ou A5 de chaque document), bulletin imprimé.
   **Modules de l'école (1.2)** : en tête des réglages, une case par module — messages aux
   familles par SMS, cantine, personnel et paie, dépenses et trésorerie, emploi du temps,
   discipline. Un module éteint disparaît du menu et du tableau de bord pour tout le monde ;
   ses données restent et reviennent quand on le rallume. Les SMS et la cantine sont éteints à
   l'installation ; les autres sont allumés. Élèves, inscriptions, notes, bulletins et caisse
   restent toujours.
2. **Paramètres → Niveaux et séries** : les cycles enseignés (jardin, 1er et 2e cycles, lycée,
   enseignement technique et professionnel) ; un cycle oublié au premier démarrage s'ouvre ici.
3. **Paramètres → Année scolaire** : créez l'année (2026-2027), ses périodes (trimestres,
   semestres ou mois selon le cycle), puis ouvrez-la.
4. **Paramètres → Classes** : les classes de l'année, par niveau et, au lycée, par série.
5. **Paramètres → Frais de l'année** : la grille des frais (inscription, scolarité, cantine,
   transport, tenue, examen…) et les plans de paiement en tranches. Les échéances d'une grille
   neuve tombent le 5 du mois, ou le jour choisi dans **Établissement** (« Jour du mois des
   échéances ») ; sur une grille en brouillon, « Mettre toutes les échéances à ce jour » les
   déplace toutes d'un clic.
6. **Paramètres → Matières et notation** : les matières, les programmes de chaque niveau avec
   leurs coefficients, et les règles de notation. GESCO pose déjà les matières et le programme
   de chaque niveau ; l'école corrige. **Import du programme (1.2)** : en bas de **Paramètres →
   Établissement**, la section repliée « Importer le programme des matières (Excel) » fournit un
   modèle qui contient le programme actuel (niveau, série, matière, coefficient, heures par
   semaine). Corrigez-le dans Excel, puis choisissez le fichier : GESCO vérifie chaque ligne,
   récrit le programme des seuls niveaux du fichier et crée les matières qui manquent. Un niveau
   dont une ligne est fausse est laissé tel quel. Une sauvegarde est faite avant l'import.
   **Import des classes et des matières (1.2.1)** : juste au-dessus, la section repliée
   « Importer les classes et les matières (Excel) » crée en une fois les classes de l'année et
   les matières qui manquent, sans toucher au programme. Son modèle a deux feuilles,
   « Classes » (niveau, série au lycée, nom de la classe) et « Matières » (matière, code, nom
   court, groupe), déjà remplies de ce que l'école a ; chacune est facultative. Ce qui existe
   déjà reste tel quel ; un niveau d'un cycle fermé ou une série manquante sont signalés.
7. **Paramètres → Enseignants** : qui enseigne quoi dans quelle classe ; un enseignant ne saisit
   que les notes de ses classes et de ses matières.
8. **Paramètres → Utilisateurs** : un compte par personne, avec son ou ses rôles (voir §8).

Ensuite, dans le groupe **Finance → Trésorerie** : les comptes de l'école (banque, Orange Money,
Wave, Moov) avec leur solde au jour de leur ouverture, le seuil au-dessus duquel une dépense
attend l'accord de la direction, et la cantine si l'école en a une.

---

## 3. Scolarité

### Guichet (Scolarité → Guichet) (1.2)

Le parent qui se présente au comptoir : cherchez un élève par son nom ou son matricule, et toute
la famille s'affiche sur un seul écran. Les responsables et leurs téléphones ; l'élève et ses
frères et sœurs inscrits, avec ce que chacun doit d'échu et sur l'année ; les bulletins à
remettre ou à faire signer ; les convocations en cours ; les absences des trente derniers jours
non justifiées ; les SMS envoyés à la famille. Tout se règle sans changer d'écran : encaisser,
remettre un bulletin, justifier une absence, noter que le parent convoqué est venu, remettre un
code d'accès (§4bis).

### Élèves (Scolarité → Élèves)

- **Créer un élève** : « Nouvel élève », son identité, son extrait d'acte de naissance. GESCO
  donne le matricule de l'école (`2026/00001`), sauf si l'école saisit les siens (réglages dans
  Établissement, avec le préfixe, la longueur et le séparateur : barre oblique, tiret, point ou
  rien). S'il trouve un élève qui ressemble, il le signale avant d'enregistrer.
- **La fiche de l'élève** porte son identité, sa photo, ses responsables, ses inscriptions et ses
  frais. La photo se choisit dans un fichier ou **se prend à la webcam du poste (1.1)** ; la
  première fois, Windows demande l'autorisation d'utiliser la caméra.
- **Responsables** : père, mère, tuteur… avec, pour chaque lien, ce qu'il est (responsable légal,
  payeur, reçoit les SMS, vient chercher l'enfant). Un responsable déjà connu, parent d'un frère
  ou d'une sœur, se retrouve par son téléphone ou son nom et se lie sans être ressaisi. Les
  téléphones s'écrivent comme on veut (`76 12 34 56`, `0022376123456`) : GESCO les range sous une
  seule forme.
- **Doublons** : « Fusionner avec une fiche en double… » réunit deux fiches de la même personne.
  L'ancien matricule mène ensuite à la fiche gardée.
- **Importer des élèves** : « Importer des élèves depuis un fichier… » lit une liste Excel ou
  CSV, montre ce qu'il comprend de chaque ligne, puis crée les fiches.
- Toute liste s'exporte en Excel ou en CSV par les boutons au-dessus d'elle.

### Inscriptions (Scolarité → Inscriptions)

- L'assistant d'inscription mène de la fiche de l'élève à sa classe, ses frais et son premier
  versement. Une inscription peut rester en brouillon tant qu'une pièce manque.
- La **fiche d'inscription** s'imprime, avec un code QR qui permet de la vérifier.
- À la validation, GESCO établit la facture de l'année et son échéancier ; un élève arrivé en
  cours d'année paie selon la règle choisie dans Établissement (par défaut, la tranche du mois
  d'arrivée compte, même si son jour est passé), ou selon une autre règle choisie pour lui à la
  validation (« Élève arrivé en cours d'année »).
- **Un parent payé un autre jour** : dans les frais de la fiche de l'élève, « Jour d'échéance
  pour cet élève », puis « Décaler ses échéances » ; ses échéances de scolarité pas encore
  payées passent à ce jour de leur mois.
- **Réinscriptions** : à la fin de l'année, GESCO prépare l'inscription de chaque élève dans sa
  classe suivante ; elle se valide au guichet quand le parent vient.
- **Départs** : transfert vers une autre école (certificat de transfert imprimé), abandon,
  exclusion temporaire ou définitive.

### Notes (Scolarité → Notes)

- Créez une évaluation (devoir, composition) pour une classe et une matière, puis saisissez les
  notes au clavier, ligne après ligne. Une grille de notes Excel s'importe aussi.
- Les notes sont validées puis verrouillées ; une note verrouillée se rouvre avec un motif.

### Absences (Scolarité → Absences)

- Le relevé se fait par classe et par demi-journée : absent, en retard, absence justifiée.
- **Par séance (1.1)** : si l'école coche le réglage « Absences : relever à chaque séance de
  l'emploi du temps », le relevé montre les séances du jour. Un élève absent à une séance du
  matin compte la demi-journée : les totaux et les bulletins ne changent pas.

### Bulletins (Scolarité → Bulletins)

- GESCO calcule moyennes et rangs d'après les notes, les coefficients et les règles de chaque
  cycle ; la direction relit, saisit les appréciations, valide puis publie.
- Le bulletin s'imprime au format réglé (A5 par défaut), avec les colonnes que l'école a
  choisies (Établissement → Bulletin imprimé).
- **Avertissement et blâme du conseil de classe (1.1)** : notés dans Discipline et rattachés à la
  période, ils s'impriment sur le bulletin.

### Remise des bulletins (Scolarité → Remise des bulletins) (1.2)

Un bulletin publié est « à remettre ». Choisissez la classe et la période, puis la ligne de
l'élève : il se remet à un responsable, à l'élève, ou à une personne nommée (un oncle, un voisin),
avec ou sans signature ; un bulletin remis se fait signer plus tard. Quand la période suivante
est publiée, les bulletins jamais retirés deviennent « non retirés » ; leur liste, en bas de
l'écran, montre aussi ce que la famille doit. Avec les SMS allumés, l'avis « bulletin à
retirer » se renvoie d'un clic.

### Convocations (Scolarité → Convocations) (1.2)

Convoquer un responsable au sujet d'un élève : le motif, le jour, l'heure et le canal (billet
confié à l'élève, en main propre, SMS, WhatsApp). Le billet s'imprime, avec un coupon à
rapporter signé. On note ensuite ce que l'école sait de la réception (confirmée ou non), un
report, puis la suite : honorée ou non honorée. Par SMS, le message est préparé et attend qu'une
personne le relise et l'envoie (Communication → Messages).

### Discipline (Scolarité → Discipline) (1.1)

Le registre de l'année et le dossier de chaque élève : avertissement, blâme, retenue,
convocation des parents, chacun daté, motivé, avec qui l'a décidé. Une sanction saisie par erreur
s'annule avec un motif ; elle reste visible, barrée.

### Emploi du temps (Scolarité → Emploi du temps)

- Réglez d'abord la journée de chaque cycle (heures de classe, récréations, après-midi libres),
  et l'horaire de canicule du 1er cycle s'il y a lieu.
- Placez les séances à la main : classe, jour, heures, matière, enseignant, salle. GESCO signale
  un enseignant à deux endroits, une salle prise deux fois ou un programme dépassé, sans
  refuser.
- L'emploi du temps s'imprime par classe et par enseignant.

### Fin d'année (Scolarité → Fin d'année)

1. GESCO propose une décision pour chaque élève (passage, redoublement…) d'après la moyenne de
   passage de son cycle ; l'école les valide, classe par classe.
2. La **clôture** fige l'année : elle reste consultable, plus rien ne s'y modifie (une
   réouverture motivée reste possible). Avec la 1.1, le journal d'audit de l'année est archivé
   dans un fichier signé.
3. L'année suivante s'ouvre, les réinscriptions sont prêtes, et les arriérés suivent l'élève.

---

## 4. Finance

### Caisse et encaissements (Finance → Caisse)

1. **Ouvrez la caisse** en début de journée, avec le fonds de caisse.
2. **Encaissez** : cherchez l'élève, voyez ce qu'il doit, saisissez le montant et le mode
   (espèces, chèque, virement, Orange Money, Wave, Moov ; la référence est exigée hors espèces).
   GESCO impute le versement sur les échéances les plus anciennes et imprime le reçu. Les frères
   et sœurs du même payeur se règlent ensemble.
3. **Fermez la caisse** en fin de journée : comptez, saisissez le montant trouvé ; l'état de
   clôture s'imprime avec l'écart éventuel.

Un versement saisi par erreur s'annule avec un motif ; la direction en est avertie. Les remises
(fratrie, enfant du personnel…) se règlent sur la facture de l'élève.

- **Finance → Sessions de caisse** : les caisses ouvertes et fermées, leurs états.
- **Finance → Impayés** : ce qui reste dû, par ancienneté (0 à 30 jours, 31 à 60, 61 à 90, plus
  de 90), exportable.
- **Finance → Journal des encaissements** : tous les versements d'une période.

### Dépenses (Finance → Dépenses)

- Saisissez la dépense : date, catégorie, bénéficiaire, montant, compte et mode, justificatif
  (photo ou PDF).
- Sous le seuil de l'école, elle se paie d'un geste ; au-dessus, elle attend l'accord de la
  direction, qui l'accorde ou la refuse avec un motif.
- Payée en espèces, elle sort de la caisse ouverte de qui paie. Une dépense payée s'annule avec
  un motif, et l'argent revient dans son compte.

### Fournisseurs (Finance → Fournisseurs)

Les fournisseurs et leurs factures, payées en une ou plusieurs fois par des dépenses ; ce que
l'école doit à chacun, et les factures échues.

### Autres recettes (Finance → Autres recettes) (1.1)

Dons, locations de salle, subventions : l'argent qui ne vient pas des familles. Il entre dans le
compte choisi (en espèces, dans la caisse ouverte) et figure dans le livre du compte, le journal
du mois et le budget.

### Comptes et soldes (Finance → Comptes et soldes)

- Le solde de chaque compte (caisse, banque, mobile money) et son livre du mois.
- Les **transferts** entre comptes (dépôt de la caisse à la banque, retrait).
- Le **rapprochement** : chaque ligne du livre se coche contre le relevé.
- **Importer un relevé (1.1)** : le relevé Excel ou CSV d'Orange Money, de Wave, de Moov ou de la
  banque. Indiquez une fois les colonnes de la date, du montant et de la référence : GESCO les
  retient pour ce compte. Il coche les lignes qu'il retrouve, et montre à part les lignes du
  relevé que le livre n'a pas (frais bancaires, par exemple).

### Journal et budget (Finance → Journal et budget)

Le journal de trésorerie du mois (recettes et dépenses par catégorie, solde de chaque compte au
début et à la fin), et le budget de l'année : prévu, réalisé, reste. Un dépassement avertit sans
rien bloquer. Tout s'exporte en Excel pour le comptable.

### Cantine (Finance → Cantine)

À activer dans Trésorerie. Les abonnés du jour d'après les factures de cantine, avec ce qu'ils
doivent ; les repas pointés si l'école les compte ; la liste du jour imprimée ; le bilan du mois
(recettes, vivres achetés, repas).

---

## 4bis. Messages aux familles (1.2)

À allumer dans **Paramètres → Établissement**, section « Modules de l'école ». Éteint, aucun SMS
ne part ni n'arrive et ces écrans sont masqués.

**Choisir l'envoi.** Dans les réglages « Messages aux familles » : « Essai sans clé » (rien ne
part réellement, pour apprendre) ou « Clé modem GSM branchée sur le poste ». La clé est une clé
USB 3G avec une puce prépayée de l'école ; dans **Communication → Messages**, section « Clé
modem », choisissez son port, essayez-la, puis gardez ce port. Le crédit de la puce se lit par le
code USSD de l'opérateur (réglage « Code du solde de la puce ») ; sous le seuil réglé, les SMS
s'arrêtent et la direction est prévenue, puis ils repartent quand le crédit revient.

**Ce qui part tout seul**, chaque message pouvant être coupé dans les réglages : l'avis
« bulletin à retirer » à la publication ; la relance d'impayé du payeur (7 jours après
l'échéance, puis tous les 15 jours, 3 fois au plus) ; l'avis d'absence du jour, après un délai
de grâce de 30 minutes pour corriger un relevé. Les SMS partent de 7 h à 20 h, en français sans
accents ; l'écran dit combien de SMS coûtera chaque message.

**Ce qui attend une personne** : les convocations par SMS et, si l'école le veut, les avis de
sanction. Ils arrivent dans **Communication → Messages** « à relire » : on corrige le texte,
puis on l'envoie, ou on l'annule avec un motif.

**La file des SMS** (Communication → Messages) : ce qui attend, ce qui est parti, remis ou
échoué ; un message échoué se renvoie. Les modèles des messages s'y reformulent, et un SMS
d'essai s'envoie à un numéro.

**Sur la fiche d'un élève**, chaque numéro de ses responsables montre s'il est joignable ; un
responsable peut refuser un type de message (par exemple les relances) sans perdre les autres.
Un numéro qui échoue trois fois devient injoignable et ne coûte plus rien ; il se remet à
l'essai une fois corrigé.

**Codes d'accès et consultation par SMS.** Au guichet, remettez à un responsable son code
d'accès : six caractères, affichés une seule fois, avec un billet à imprimer et à lui remettre
en main propre. GESCO ne garde pas le code ; un code perdu se remplace (l'ancien ne sert plus) ou
se révoque. Si l'école ouvre la consultation (réglage « Les parents consultent par SMS »), le
parent écrit au numéro de la puce : **SOLDE** suivi de son code (ce qui reste à payer et la
prochaine échéance), **ABS** suivi de son code (les absences de la période), **NOTE** suivi de
son code (la moyenne du dernier bulletin publié ; le rang seulement si l'école le veut),
**AIDE**. Seuls les numéros du responsable reçoivent une réponse ; un code faux n'est répondu
qu'une fois par jour, cinq codes faux bloquent le numéro jusqu'au lendemain, et chaque numéro a
cinq consultations par jour. Les SMS reçus sont listés dans **Communication → Messages**, le code
masqué.

---

## 5. Personnel et paie

### Registre du personnel (Personnel → Registre du personnel)

- La fiche de chaque membre du personnel, son dossier, sa catégorie et sa fonction.
- **Importer une liste (1.1)** : « Importer une liste… » fournit le modèle Excel à remplir, puis
  vérifie chaque ligne avant de créer les fiches ; les lignes en erreur et les personnes déjà au
  registre sont laissées de côté. Une sauvegarde est faite avant l'import.
- Les **engagements** : permanent ou de vacation, avec leur lettre d'engagement imprimée.

### Heures des vacataires (Personnel → Heures des vacataires)

L'état mensuel des heures, saisi au clavier puis visé. **Avec la 1.1**, quand un vacataire a des
séances dans l'emploi du temps, GESCO propose ses heures d'après ces séances (jours de canicule
compris) ; on les corrige au besoin.

### Absences et congés (Personnel → Absences et congés)

Les absences, retards, permissions et congés du personnel, demandés puis accordés ou refusés.

### Paie (Personnel → Paie)

1. **Éléments de paie** : salaire de base, primes, retenues, cotisations ; chacun s'attribue aux
   personnes concernées.
2. **Paie du mois** : GESCO calcule chaque bulletin ; on revoit, on confirme les absences, puis la
   direction valide. Une paie validée ne se recalcule plus ; une erreur se corrige le mois
   suivant.
3. **Payer** : en une ou plusieurs fois, par la caisse ou hors caisse ; ce qui reste dû se lit.
   Les avances se versent et se remboursent par retenue.
4. **Imprimer** : bulletins de paie, état de paie à signer, état des retenues. Chacun lit ses
   propres bulletins dans « Mes bulletins de paie ».

Chaque salaire et chaque avance payés deviennent une dépense « Salaires » dans la trésorerie.

---

## 6. Protéger les données

### Sauvegarde (Outils → Sauvegarde)

- GESCO sauvegarde seul chaque jour (l'intervalle se règle dans Établissement) et à la fermeture, et garde 7 sauvegardes
  quotidiennes, 4 hebdomadaires et 12 mensuelles.
- **Ajoutez une clé USB comme destination**, puis « Sauvegarder maintenant ». Une sauvegarde qui
  ne quitte jamais le poste ne protège pas d'un vol ou d'une panne.
- Chaque mois, GESCO vérifie qu'une sauvegarde se restaure.
- **Restaurer** sur un autre poste : installez GESCO, choisissez « Restaurer une sauvegarde » au
  premier démarrage, puis le fichier `.gsv` et la phrase de secours de douze mots.

### Journal d'audit (Outils → Journal d'audit)

Qui a fait quoi, quand, avant et après : chaque modification est inscrite dans un journal
enchaîné, que « Vérifier l'intégrité de la chaîne » contrôle. **Avec la 1.1**, les archives
signées des années closes se vérifient aussi : si un seul octet a changé, GESCO le signale.

### Licence (Outils → Licence)

Choisissez le fichier de licence remis (`GESCO-0001.lic`). La licence s'attache au poste. Sans
licence, GESCO fonctionne de la même façon ; un bandeau le rappelle.

---

## 7. Le tableau de bord

**Général → Tableau de bord** résume l'école selon les droits de chacun : effectifs, encaissements
et impayés, scolarité, personnel, paie à valider, soldes des comptes, dépenses du mois, budget,
et les avertissements qui demandent une action. Un module éteint n'y a pas de bloc (1.2).

---

## 8. Les rôles livrés

Chaque utilisateur reçoit un ou plusieurs rôles dans **Paramètres → Utilisateurs**. L'écran
**Paramètres → Rôles** montre ce que chaque rôle permet ; l'administrateur peut créer des rôles
propres à l'école.

| Rôle | Pour qui | En bref |
|---|---|---|
| Administrateur | Celui qui installe et règle GESCO | Tout, y compris les comptes et les réglages |
| Directeur | La direction | Presque tout : scolarité, finances, paie, dépenses à accorder, trésorerie, emploi du temps, cantine |
| Directeur des études | La pédagogie | Matières, notes, bulletins, emploi du temps |
| Secrétaire | L'accueil | Élèves et leur import, inscriptions, absences, cantine, remise des bulletins, convocations, codes d'accès des parents, file des SMS |
| Caissier | Le guichet | Caisse, encaissements, petites dépenses, remise des bulletins |
| Comptable | Les comptes | Facturation, dépenses, trésorerie, paie |
| Enseignant | Chaque professeur | Les notes de ses classes et de ses matières |
| Surveillant général | La vie scolaire | Absences, discipline, convocations, messages préparés à relire et envoyer |
| Gestionnaire RH | Le personnel | Registre, engagements, congés, import du personnel |
| Consultation seule | Le promoteur, un contrôleur | Lire, sans rien modifier (la file des SMS comprise) |

Les montants des salaires ne sont visibles que de ceux qui font la paie ; chacun lit les siens.

---

## 9. En cas de souci

- **« Phrase de secours requise » au démarrage** : la clé des données n'est plus lisible sur ce
  poste (réinstallation de Windows, autre compte). Saisissez la phrase de douze mots recopiée à
  l'installation.
- **Un mot de passe oublié** : l'administrateur le réinitialise dans Utilisateurs. Un compte
  bloqué après trois essais se débloque au même endroit, ou de lui-même après un moment.
- **Une erreur de saisie** : rien ne s'efface dans GESCO. Un versement, une dépense, une recette,
  une sanction s'annulent avec un motif ; le journal garde la trace.
- **Le poste est perdu ou en panne** : réinstallez GESCO sur un autre poste et restaurez la
  dernière sauvegarde de la clé USB avec la phrase de secours (§6).
- **Une question sur un écran** : chaque écran explique ce qu'il attend en tête de ses sections.
- **Pour l'assistance (1.2.1)** : chaque erreur est notée dans le fichier
  `journal-technique.txt` du dossier `%APPDATA%\ml.gesco.poste`. Envoyez ce fichier quand vous
  signalez un souci ; il ne contient ni les données de l'école ni les mots de passe.

---

## 10. Ce qui vient ensuite

La 1.2 apporte les SMS aux familles, le guichet, la remise des bulletins, les convocations, les
codes d'accès et la consultation par SMS, les modules de l'école et l'import du programme ;
la 1.2.1, l'import des classes et des matières ; la 1.2.2, les échéances au 5 et le jour
propre à un élève. Les
SMS ont été essayés avec une clé simulée : avant de compter sur eux, faites un essai avec la clé
et la puce de l'école (SMS d'essai, lecture du crédit, un SOLDE envoyé depuis un téléphone).
Viendront ensuite les rapports et les états pour le CAP et l'académie, puis l'e-learning.
