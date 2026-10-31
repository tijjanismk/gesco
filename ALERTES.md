# Alertes

## Symboles dupliqués

- `ajouter`, `multiplier` dans `commun/montant.rs` et `commun/note.rs` — **faux positif** : ce sont des méthodes de deux types distincts (`Montant`, `Note`), pas des fonctions libres. Aucune action.

## Orphelins

Aucun. Les crates vides n'exportent rien (voir `modules/crates-vides.md`).

## Fichiers les plus sollicités

Tous les fichiers de `gesco-core/src/commun/` sont importés une fois (par `mod.rs` ou entre eux : `id.rs`, `montant.rs`, `note.rs` importent `erreur.rs`). C'est le futur cœur : le modifier touchera tout le noyau.

## Écarts à surveiller

- **Chiffrement et essai sur machine vierge (10 octobre 2026)** : l'essai Windows 10 est abandonné ; l'installeur n'a donc jamais été éprouvé sur un poste sans WebView2 ni internet. La base est chiffrée depuis le même jour (SQLite3 Multiple Ciphers, `docs/adr/2026-10-10-chiffrement-de-la-base.md`). À surveiller : `vendor/libsqlite3-sys` est une copie à refaire à chaque montée de `libsqlite3-sys` (commentaire « GESCO » dans `build.rs`).

- Données initiales : la grille de la 3e à la 6e est une grille par défaut non officielle (Q-37). Ne jamais en faire une contrainte de validation (horaires seulement : les autres règles restent bloquantes).
- Les décisions de §24.7 (arrivée en cours d'année, transferts) ajoutent `MoyenneReportee`, `CertificatTransfert`, des champs à `Inscription` et quatre paramètres d'établissement : à poser dans les migrations des phases 2 à 4, aucune n'existe encore.
- **Toolchain** : `rust-toolchain.toml` épingle 1.99.0 en édition 2024 (§5.4) ; l'environnement de construction de ces livraisons disposait de la 1.98.1 seulement. Aucun écart de code ; à vérifier au premier `cargo build` sous rustup.
- **Crates `gesco-server` et `gesco-sync` retirées (31 octobre 2026)** : deux squelettes vides (`fn main() {}`, une ligne), compilés à chaque vérification pour rien, `gesco-server` entraînant axum et `gesco-services` (donc Typst) alors que le disque du porteur du projet se remplit à chaque compilation. Elles reviendront à la phase 11 (serveur cloud, consommation de l'outbox) ; l'historique git les garde.

## Corrections documentaires du 4 octobre 2026 (blueprint v0.17, règles v0.3, spec v1.1)

- **Q-38, tranchée le 4 octobre 2026 (recommandation retenue).** La clé de chiffrement de la v0.16 (secret d'établissement + secret machine) rendait toute sauvegarde indéchiffrable sur une autre machine, donc UC-07 impossible. Conception de remplacement en blueprint §17.3 et spec §6.3 (clé de données, enveloppe machine, enveloppe de secours, phrase de douze mots, certificat imprimé), règle R-57. L'étape 11 l'implémente.
- **R-54 amendée : code en français.** La règle v0.2 exigeait des identifiants en anglais ; le blueprint §5.5, le §6, la spec et le code (`Montant`, `Note`, `Horodatage`, `ajouter`, `depuis_francs`…) étaient tous en français. Le code existant, en infraction de la règle telle qu'écrite, est conforme à la règle amendée. Un `docs/modele-donnees/glossaire.md` reste à créer.
- **Collision de numéros levée.** Les risques du blueprint §23 passent de `R-xx` à `RA-xx` (spec §6.2 et §11 corrigées : RA-13, RA-12). Les règles « R-47 versions » et « R-48 anti vibe coding » de `prompts/PROMPT_ONESHOT_GESCO.md` portaient des numéros déjà pris (import, sauvegarde) : elles deviennent R-55 et R-56 dans `ARCHITECTURE_RULES.md`. **Le prompt n'est pas dans le lot corrigé** : à réaligner (R-55, R-56, RA-xx, blueprint v0.17).
- `gesco-server` est le binaire du serveur **cloud** (phase 11). Le mode LAN (ADR-014) est embarqué dans `gesco-app`, un seul binaire pour le poste. Le commentaire « LAN + cloud » du §6 et de la carte était contradictoire avec ADR-014 et §15.1.
- Phases du §7 réalignées sur §22.2 (décalage d'une unité depuis M5) ; modules `notification.*`, `scolarite.remise_convocation`, `core.guichet` (phase 5) et `core.licence` (phase 10) ajoutés.
- `PeriodeAcademique.type` admet `mensuelle` (Q-22) ; le blueprint §8.1 ne listait que trimestre et semestre alors que la spec §4.2 et §24.6 l'exigeaient.
- Catalogue de permissions §11.2 complété (assiduité, sanctions, convocations, remise de documents, verrouillage de période, notifications, imports, portabilité, licence). À reporter dans la table `permission` à l'étape 8, puis par phase.
- `carte.json` : `montant.rs` liste un export nommé `fn` — artefact de l'analyseur (signature `pub fn` coupée sur deux lignes, probablement `diviser`). Corriger l'analyseur, pas la carte ; la carte se régénère.
- `DOMAINE.md` citait des numéros de ligne du blueprint et des règles ; ils sont périmés dès cette révision. Remplacés par des références de section et de règle.
- Q-39 (qui saisit les notes sur un poste unique) et Q-40 (durée des phases) sont ouvertes en §24.8 ; aucune n'affecte la phase 0.5. Pour Q-40, noter le temps réel de chaque étape dans `ETAT_AVANCEMENT.md` dès maintenant.

## Livraison du 4 octobre 2026 — étapes 6 à 9

- `Garde::autoriser` lit les permissions en base à chaque appel : correct et simple en 0.5, à mettre en cache par session quand la phase 2 multipliera les commandes (invalidation sur `RoleAttribue`).
- `Session.role_actif` = premier rôle de l'utilisateur ; un utilisateur multi-rôles choisira son rôle actif en phase 1 (écran de session), l'audit ne porte donc pas encore ce choix.
- Les secrets (`MotDePasse`, `Pin`) masquent leur `Debug` ; `Argon2::default()` ≈ 19 Mio, 2 passes — la cible de 250 ms (§6.1) se mesure sur le matériel réel, pas ici.
- `JournalEffets` reste l'exécuteur des tests ; l'exécuteur de production est `FileEffets` (étape 10).

## Livraison du 4 octobre 2026 — étapes 10 à 12

- **Q-41, appliquée par défaut, à confirmer.** Régénérer la phrase de secours invalide l'ancienne pour la clé du poste et les sauvegardes suivantes ; les archives antérieures restent ouvrables avec l'ancienne phrase jusqu'à leur rotation. Le texte du certificat et l'écran de régénération le disent. Changer de clé de données à chaque régénération est l'alternative (blueprint §24.8).
- **SQLCipher n'est pas actif.** `sqlx` compile SQLite sans SQLCipher : la clé passée par `PRAGMA key` est ignorée et la base du poste est en clair. Les **archives de sauvegarde sont chiffrées** indépendamment (ADR du 4 octobre 2026). Activer SQLCipher (fonctionnalité `bundled-sqlcipher-vendored-openssl` de `libsqlite3-sys`) demandera de convertir les bases existantes par `sqlcipher_export`, sauvegarde préalable comprise. **Reporté le 7 octobre 2026 par décision du porteur du projet** : l'assertion 2 du §0.2 (« base chiffrée ») reste fausse, la phase 0.5 ne peut donc pas être validée tant que SQLCipher n'est pas actif. Prérequis de construction sous Windows : un Perl natif (Strawberry Perl) pour compiler OpenSSL ; celui de Git ne suffit pas (modules manquants). Une conversion par `sqlcipher_export` a été écrite et testée en partie le 7 octobre, puis retirée avec la dépendance ; elle est à réécrire au moment de l'activation.
- **File d'effets en mémoire.** Un document demandé par un service (numéro annulé) est composé par une tâche de fond après le commit. Si l'application s'arrête entre les deux, l'effet est perdu ; l'`outbox` permettra de le rejouer quand `gesco-sync` le consommera. Les échecs d'effet et des tâches périodiques s'affichent dans les avertissements du bandeau jusqu'à la fermeture.
- **Le certificat de clé de secours n'est jamais écrit sur le disque** : composé en mémoire, affiché, imprimé depuis l'aperçu. L'audit garde l'identifiant d'enveloppe et la version du modèle, jamais les mots (R-25).
- **Trousseau.** Windows : gestionnaire d'identifiants (DPAPI) par `keyring-core` et `windows-native-keyring-store`. Ailleurs : trousseau en mémoire, la phrase est demandée à chaque démarrage (ADR-016 : Windows seul en 0.5).
- **Empreinte machine** de l'audit = nom du poste, jusqu'à l'empreinte matérielle de la licence (étape 13).
- ~~**Restauration : chemin saisi à la main.**~~ Sélecteur de fichier ajouté le 6 octobre 2026 (`tauri-plugin-dialog` 2.8.1, `dialog:allow-open` seule ; amendement de l'ADR « frontière à commande unique »). La saisie du chemin reste possible.
- **Frontend.** TypeScript 6.0.3 et non 7.0.2 (dérogation R-55, `ARCHITECTURE_RULES.md` §14). `eslint-plugin-react` ne prend pas ESLint 10 en charge : les chaînes en dur sont détectées par un test qui parcourt le JSX avec l'API TypeScript (`src/i18n/catalogue.test.ts`). `pnpm` 12 a inscrit dans `pnpm-workspace.yaml` quatre paquets publiés moins de 24 h avant l'installation (politique `minimumReleaseAge`) ; l'exclusion peut être retirée après ce délai. shadcn/ui, Radix, Tremor, TanStack Table, React Hook Form et Zod ne sont pas installés : aucun écran de la 0.5 n'en a besoin (R-56, règle 7). Export CSV seul (dérogation R-41).
- **Types générés.** `specta-typescript` refuse les entiers 64 bits : le test de génération les rend en `number` (horodatages en millisecondes, tailles), et remplace `serde_json::Value`, récursif, par `ValeurJson`.

## Session du 5 octobre 2026 — étape 13 en cours

- **Clé privée de licence hors dépôt.** Générée le 4 octobre 2026 dans `%USERPROFILE%\.gesco-editeur\cle-privee-licence.hex` du poste de développement. Sa perte oblige à changer `cle-publique-licence.txt`, donc à réémettre toutes les licences ; sa fuite permet d'émettre des licences. La sauvegarder hors ligne.
- **La licence ne bloque rien, par décision** (bandeau seulement). Le contournement est un risque assumé (§3.13). Une licence dont la signature ne se vérifie plus compte comme absente.
- **L'empreinte machine change à la réinstallation de Windows** (`MachineGuid`) : c'est le cas « à réémettre » prévu, sans blocage.
- **Les tests n'avaient jamais tourné sous Windows** avant le 5 octobre 2026 (CI Linux). Le premier passage a révélé le bug des sauvegardes amovibles (corrigé). Un job Windows de la CI qui lance aussi les tests éviterait la récidive.
- **`build.rs` de `gesco-app`** n'avait pas de documentation et échouait sous `clippy -D warnings` (corrigé).
- **Écart avec le blueprint** : §7 range `core.licence` en phase 10, la spécification de la phase 0.5 (§13) la demande à l'étape 13. La spécification a été suivie.

## Session du 7 octobre 2026

- **Restauration d'une archive locale du jour : corrigé.** La sauvegarde de sécurité qui précède une restauration appliquait la rotation ; la politique quotidienne supprimait alors l'archive du même jour, parfois celle que l'on restaurait (« Le fichier spécifié est introuvable »). Constaté sur le poste de développement, où l'archive `gesco-20261007-010741-EEAPET.gsv` a été perdue. La sauvegarde `AvantRestauration` n'applique plus la rotation ; test `restaurer_une_archive_locale_du_jour_sans_phrase`. Les tests ne le voyaient pas : ils copiaient l'archive hors du dossier local avant de restaurer.
- **Build Tools 2022** réinstallés (MSVC 14.44, SDK 10.0.26100). Dans Git Bash, `/usr/bin/link` masque l'éditeur de liens : compiler depuis PowerShell.
- **Phase 1 livrée le 7 octobre 2026, en autonomie la nuit, à la demande du porteur du projet.** À relire à son retour :
  - **Écart corrigé de la phase 0.5** : un code PIN faux pouvait être retenté sans limite ; désormais trois PIN faux ferment la session (`SPEC_PHASE_1.md` §7).
  - **Règle ajoutée** : on n'accorde, ni dans un rôle ni par attribution, une permission que l'on ne détient pas soi-même (§5.3). Sans elle, quiconque compose les rôles pourrait s'élever.
  - **Logo dans la base**, pas en pièce jointe sur disque (§2.2) : il voyage avec les sauvegardes.
  - **Liste des mots de passe courants** : environ 130 entrées écrites à la main, et non mille (§6).
  - **Ouverture d'une année précédée d'une sauvegarde** (nouveau motif `AvantOuvertureAnnee`).
  - **À corriger plus tard, hors périmètre** : un compte sans `templates:manage` (une secrétaire) voit le bouton « Imprimer un document d'essai », que la garde refuserait ; les avertissements d'administration (certificat, sauvegarde, licence) s'affichent à tous les comptes, même ceux qui ne peuvent rien y faire ; les lignes de tableau ne se sélectionnent qu'à la souris (§19.1 : le clavier avant la souris).
  - **Captures** : la zone de défilement est dépliée pour photographier l'écran entier ; la barre latérale y paraît alors plus courte que la page, ce qui n'arrive pas dans l'application.
- **Documents réels de trois écoles** : prérequis abandonné le 7 octobre 2026 par le porteur du projet, remplacé par des questions de terrain (`docs/questions-terrain.md`, blueprint §27). T-01 à T-10 (phase 2) répondues le même jour, avec des écarts à la recommandation : codes de classe du lycée « 10CG », « 12TSE » (T-04) ; aucun effectif maximal (T-05) ; bloc des responsables au choix sur la fiche imprimée (T-08) ; réinscription selon l'école (T-09) ; jardin d'enfants inscrit dès la phase 2, avec photo obligatoire et personnes qui viennent chercher l'enfant (T-02, T-10, Q-03 révisée).
- **Phase 2 engagée avant la validation de la phase 0.5** (7 octobre 2026, à la demande du porteur du projet) : la base n'est pas chiffrée et l'essai sur machine vierge reste à faire. **Aucune école ne reçoit GESCO avec des données d'élèves tant que ces deux points ne sont pas réglés** (`SPEC_PHASE_2.md` §0.2) : ce sont des données de mineurs.
- **Inscriptions validées sans facture jusqu'à la phase 3** (R-17, `SPEC_PHASE_2.md` §0.3) : la phase 3 doit facturer les inscriptions déjà validées par une montée des bases installées.
- **Dérogation R-41 (export CSV seul)** : close le 8 octobre 2026 par l'étape 7 de la phase 2 (export Excel sur toutes les listes).

## Session des 8 et 9 octobre 2026 — phase 2 livrée

- **Phase 2 livrée en neuf étapes, en attente de validation du porteur du projet.** Les douze assertions du §0.4 sont démontrées (`ETAT_AVANCEMENT.md`). Rien n'est livrable à une école tant que la base n'est pas chiffrée (§0.2).
- **UC-01 chronométré par Playwright** : une à deux secondes, borne basse de l'enchaînement, pas le geste d'une secrétaire ; la cible des 5 minutes au clavier reste à mesurer sur le terrain.
- **Webcam** : la prise de photo n'est pas couverte par Playwright (pas de caméra garantie sur le poste d'essai) ; à essayer à la main.
- **Import** : un téléphone de responsable sans nom crée un responsable au nom de l'élève et au prénom « Parent », tuteur, responsable légal et payeur ; l'école corrige sur la fiche. Les dialogues du système (ouverture, enregistrement) sont remplacés dans les essais Playwright par une réponse directe à l'appel IPC de Tauri : tout le reste passe par le vrai noyau.
- **Q-45, Q-46, Q-47** appliquées par défaut ; **Q-39** tranchée en partie le 9 octobre 2026 (la direction saisit les notes, T-23), le reste à confirmer en phase 4 ; **Q-40** (durée des phases) toujours ouverte : la phase 2 a pris trois jours de travail assisté (7 au 9 octobre 2026).
- **Phase 2 validée par le porteur du projet le 9 octobre 2026.** Essais manuels en cours avant la phase 3.
- **Chiffrement laissé de côté, décision confirmée le 9 octobre 2026** : « ça ne fait pas partie de notre projet pour le moment ». Le porteur s'interroge sur la difficulté de l'intégrer plus tard. Ce qu'il faut savoir : techniquement, SQLCipher se branche au niveau de `sqlx` (dépendance `bundled-sqlcipher-vendored-openssl`, Strawberry Perl requis pour compiler OpenSSL) et le code applicatif ne change presque pas ; le coût qui grandit avec le temps est la **conversion des bases déjà installées** (une migration unique par `sqlcipher_export`, à faire sur chaque poste, avec sauvegarde préalable) et le fait que chaque école installée avant le chiffrement aura eu des données de mineurs en clair sur son disque. Tant qu'aucune école n'est installée, le report ne coûte rien ; la condition du §0.2 reste la bonne garde.
- **« Phrase de secours requise » au démarrage, 9 octobre 2026 : cause trouvée.** L'application de bureau Claude est un paquet MSIX ; les programmes qu'elle lance voient un `%APPDATA%` virtualisé (`%LOCALAPPDATA%\Packages\Claude_…\LocalCache\Roaming`), alors que le trousseau Windows ne l'est pas. Deux espaces de données `ml.gesco.poste` ont ainsi coexisté (l'un lancé depuis la session, l'autre ouvert par le porteur), chacun écrivant sa clé dans **la même** entrée du trousseau : le second a rendu le premier illisible. Les deux espaces sont mis de côté (renommés `.ancien-…`, rien supprimé) ; l'application repart d'un premier démarrage neuf, lancée par l'Explorateur. Chez une école, il n'y a pas de conteneur, mais **le défaut de fond demeure : une seule entrée de trousseau par compte Windows**. Deux espaces de données sur un même compte (réinstallation dans un autre dossier, deuxième établissement) s'écrasent l'un l'autre. Correction proposée : nommer l'entrée d'après l'identifiant de la clé, en relisant l'ancien nom pour les postes déjà installés.

## Phase 5, étape 3 (18 octobre 2026) — clé modem vérifiée contre un modem simulé

- **Essai sur une vraie clé différé** (décision du porteur du projet du 17 octobre 2026). Le pilotage de la clé (commandes AT, trames PDU, accusés `+CDS`, crédit par `AT+CUSD`) est vérifié contre le modem simulé de `gesco-sms`. À confirmer sur la première vraie clé : qu'elle expose bien un port série AT (C-1) ; la réponse USSD de l'opérateur (certaines clés la rendent en UCS-2 hexadécimal, que `lire_montant` ne lit pas) ; les codes `+CMS ERROR` d'un crédit épuisé (21 et 50 retenus, ils varient d'un opérateur à l'autre) ; la commande `AT+CNMI=2,1,0,1,0` (certaines clés veulent d'autres valeurs pour signaler les accusés).
- **Blocage de la puce pour envoi en masse (C-4), 23 octobre 2026** : selon le porteur du projet, les opérateurs ne bloquent pas, puisqu'ils vendent des forfaits à SMS illimités. Le débit étalé (6 SMS par minute, réglable) et l'alerte de crédit restent en place : un forfait a une clause d'usage raisonnable et une date de fin. À vérifier sur la première puce de l'école.
- **SMS reçus (étape 7)** : la clé est relue par `AT+CMGL=4` à l'ouverture de la session puis à chaque annonce `+CMTI`, et chaque SMS lu est effacé (`AT+CMGD`). À confirmer sur une vraie clé : la mémoire où elle range les SMS reçus (certaines clés les gardent dans la mémoire du téléphone plutôt que sur la puce : il faudra alors `AT+CPMS="SM","SM","SM"` à la préparation) ; qu'elle n'envoie pas les SMS directement (`+CMT`) malgré `AT+CNMI=2,1,…` ; un SMS long reçu en plusieurs parties n'est pas recollé (une consultation tient en une partie).
- **Pile en compilation de débogage** : la réponse à une consultation traverse beaucoup de dépôts ; son calcul est mis sur le tas (`Box::pin`). Un scénario de test très long déborde la pile de 2 Mo du fil de test : il se découpe en étapes (voir `tests/consultation.rs`).
- **Appels bloquants** : un envoi par la clé attend sa réponse (30 secondes au plus) dans la tâche de la file, qui tient alors une connexion à la base. Sans gêne avec le modem simulé, qui répond aussitôt ; avec une vraie clé, si l'envoi s'avère lent, passer les appels de la clé dans une tâche à part (`spawn_blocking`).
