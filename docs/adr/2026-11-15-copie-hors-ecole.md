# Copie hors de l'école : dépôt dans un dossier synchronisé, état d'envoi lu dans Windows

**Date** : 15 novembre 2026
**Statut** : proposé le 15 novembre 2026
**Précise** : l'ADR du 4 octobre 2026 (archive de sauvegarde) et l'ADR du 10 octobre 2026
(chiffrement de la base) ; met en œuvre la réponse du porteur du projet à T-96
(`docs/plan-phase-11.md` §7.4, `docs/orchestration.md`, 14 novembre 2026)
**Spécification** : `SPEC_SAUVEGARDE_HORS_ECOLE.md`

## Contexte

Le porteur a choisi la voie (T-96, 14 novembre 2026) : la copie hors de l'école passe par un
dossier que le logiciel du fournisseur (Google Drive, OneDrive, Dropbox) envoie sur le compte de
l'école. N'y part que l'archive `.gsv`, déjà chiffrée ; jamais la base en clair, jamais la clé.
L'application vérifie, sans bloquer, que la dernière copie est bien arrivée. Cet ADR ne rouvre pas
ce choix : il dit **comment GESCO sait si la copie est partie**, ce qui demande une décision
technique.

Contraintes :

- **GESCO ne parle pas au fournisseur.** C'est le logiciel du fournisseur qui envoie. GESCO ne voit
  que le fichier qu'il a déposé dans le dossier.
- **Le code `unsafe` est interdit** dans tout l'espace de travail (`unsafe_code = "forbid"`,
  `Cargo.toml`). Les fonctions de Windows qui donnent l'état d'un fichier synchronisé (Cloud Files
  API, `CfGetPlaceholderInfo`) ne s'appellent qu'en `unsafe`, avec `windows` 0.62.2 ou
  `windows-sys` 0.61.2. Précédent : l'empreinte du poste lit le registre par `reg.exe` pour éviter
  ce code (`crates/gesco-app/src/empreinte.rs`).
- **Ne pas consommer le forfait en vérifiant.** Un fichier « disponible seulement en ligne » se
  télécharge dès qu'on lit son contenu. Vérifier une copie en relisant ses octets peut donc coûter
  autant que l'envoyer.
- **Le disque du porteur est juste** : rien qui alourdisse la compilation de `gesco-services`.
- **Windows 10 et 11 seulement** ; Windows PowerShell 5.1 est livré avec les deux.
- On ne sait pas encore quels logiciels exposent l'état d'envoi à Windows. OneDrive le fait. Pour
  Google Drive et Dropbox, c'est à mesurer sur un vrai poste (étape 2 de la spécification).

## Options

| | 1. Propriété du Shell lue par PowerShell | 2. Cloud Files API par `windows-sys` | 3. Aucun état d'envoi |
|---|---|---|---|
| Apprentissage | Faible : une commande lancée sans fenêtre, comme `reg.exe` | Moyen : API C, structures, codes d'erreur | Nul |
| Exploitation | Un processus PowerShell de moins d'une seconde, quelques fois par jour | Rien de plus | Rien |
| Maturité | Propriétés du Shell stables depuis Windows 10 ; lecture par PowerShell 5.1 | API stable depuis Windows 10 1709 | — |
| Pour un agent de code | Bon : texte en sortie, facile à essayer avec une sortie simulée | Mauvais : exception à `unsafe_code = "forbid"`, essais difficiles sans le système | Très bon |
| Plus gros risque | Un fournisseur qui ne renseigne pas la propriété : l'état reste « déposée » | Code `unsafe` dans une application qui garde des données de mineurs | L'école croit être protégée alors que rien ne part (risque nommé au §7.2 du plan) |

Écartées sans tableau :

- **GESCO se connecte lui-même au compte Google, Microsoft ou Dropbox** : agrément à obtenir de
  chaque fournisseur, jetons à garder, un code par fournisseur ; le porteur a choisi l'autre voie
  (T-96).
- **Serveur de l'éditeur** : c'est l'étape C1 du lot C, par abonnement ; elle viendra s'ajouter à
  côté (spécification, §10), elle ne remplace pas ce choix.
- **Relire la copie déposée pour en vérifier l'empreinte, à chaque vérification** : peut
  télécharger tout le fichier si le fournisseur l'a libéré du disque.

## Décision

GESCO dépose l'archive dans le dossier, vérifie au dépôt qu'elle y est intacte (empreinte), puis
lit l'état d'envoi que Windows donne pour ce fichier, par une commande PowerShell lancée sans
fenêtre ; quand Windows ne donne rien, l'état reste « déposée dans le dossier ».

| Brique | Version | Sortie | Suite prévue |
|---|---|---|---|
| Aucune dépendance nouvelle | — | — | — |
| Windows PowerShell | 5.1, livré avec Windows 10 et 11 | — | Suit la vie de Windows ; rien à livrer |
| Pour mémoire, non retenues : `windows` | 0.62.2 (dernière stable, déjà dans `Cargo.lock` par Tauri) | 6 octobre 2025 | — |
| Pour mémoire, non retenue : `windows-sys` | 0.61.2 (dernière stable, déjà dans `Cargo.lock`) | — | — |

Vérifié le 15 novembre 2026 sur crates.io (`max_stable_version`). Non vérifié en ligne : la
présence de PowerShell 5.1 sur Windows 10 et 11, fait connu et déjà vrai sur le poste du porteur ;
le nom exact de la propriété du Shell et ses valeurs pour chaque fournisseur, à établir par essai à
l'étape 2.

Ce que la décision fixe :

- **Trois niveaux, du plus sûr au moins sûr.** Au dépôt : copie par fichier `.partiel`, renommage,
  empreinte égale à celle de l'archive du poste (ce que fait déjà `ServiceSauvegarde::archiver`).
  Ensuite : présence et taille, lues dans les métadonnées, jamais dans le contenu. Enfin : l'état
  d'envoi donné par Windows, quand il existe.
- **La lecture de l'état vit dans `gesco-app`**, derrière un trait que `gesco-services` connaît.
  Les essais de services utilisent une lecture simulée ; ils ne lancent jamais PowerShell.
- **Un échec de lecture ne dit rien de faux.** Pas de PowerShell, sortie illisible, délai dépassé :
  l'état reste « déposée dans le dossier ».
- **Peu d'appels** : quelques minutes après le dépôt, puis au démarrage et une fois par jour, tant
  que la dernière copie n'est pas « envoyée ».

## Justification

L'école doit savoir si sa copie est vraiment partie : c'est ce que le porteur demande, et c'est le
risque principal de cette voie. Le niveau 1 existe déjà et ne coûte rien. Le niveau 2 évite de
télécharger ce qu'on vient d'envoyer. Le niveau 3 donne la vraie réponse quand Windows la connaît,
sans code `unsafe` et sans dépendance : une commande Windows, comme pour l'empreinte du poste.
L'option 2 donnerait la même réponse, mais au prix d'une exception à une règle tenue depuis le
début, pour un gain de quelques centaines de millisecondes. L'option 3 laisserait l'école croire
qu'elle est protégée. Quand un fournisseur ne dit rien à Windows, l'écran le dit honnêtement
(« déposée dans le dossier »), et le guide montre comment vérifier sur le site du fournisseur.

## Conséquences

Positives : aucune dépendance, aucun code `unsafe` ; les essais de services restent rapides ; la
même colonne d'état servira à l'étape C1, où le serveur de l'éditeur confirmera lui-même l'envoi.

Négatives : un processus PowerShell lancé par moments sur le poste principal ; un antivirus strict
peut le bloquer (l'état reste alors « déposée ») ; selon le fournisseur, l'état peut ne jamais
dépasser « déposée » ; le nom de la propriété du Shell est à confirmer par essai, et pourrait
changer avec une version de Windows.

## Contre-argument conservé

Lancer PowerShell pour lire une propriété est un détour ; l'API est faite pour cela, et quelques
lignes `unsafe` bien isolées dans `gesco-app` se relisent. Elle n'a pas gagné parce que la règle
`unsafe_code = "forbid"` protège tout le reste, qu'une exception en appelle d'autres, et que l'état
d'envoi n'est qu'une indication : il ne commande rien.

## Conditions de reconsidération

- L'essai de l'étape 2 montre que ni OneDrive, ni Google Drive, ni Dropbox ne renseignent la
  propriété lue par PowerShell : passer à l'option 2 ou rester sur l'option 3.
- PowerShell est bloqué sur une part notable des postes des écoles (antivirus, stratégie).
- Le projet accepte un jour du code `unsafe` pour une autre raison : l'option 2 devient alors moins
  chère.
- L'étape C1 (serveur de l'éditeur) est livrée et les écoles l'adoptent : la vérification par
  Windows ne sert plus qu'aux écoles sans abonnement.
