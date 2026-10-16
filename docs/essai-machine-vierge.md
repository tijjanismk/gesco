# Essai d'acceptation sur machine vierge

> **Abandonné le 10 octobre 2026** par décision du porteur du projet. La procédure reste pour mémoire.

Manipulation reproductible des assertions 1, 2, 3 et 9 du §0.2 de `SPEC_PHASE_0_5.md`, et test
d'acceptation de l'installeur (§13) : Windows 10 vierge, sans WebView2, sans connexion internet.
Les autres assertions sont couvertes par des tests automatisés (`ETAT_AVANCEMENT.md`).

## Préparer

Sur le poste de développement :

1. Construire l'installeur : `cd frontend && pnpm installeur`, depuis PowerShell (dans Git Bash,
   `/usr/bin/link` masque l'éditeur de liens de MSVC). Résultat attendu :
   `target/release/bundle/nsis/GESCO_<version>_x64-setup.exe`, WebView2 hors ligne inclus.
2. Noter la taille et l'empreinte SHA-256 de l'installeur (`Get-FileHash`).
3. Graver l'installeur dans une image ISO, ou le copier sur une clé USB : c'est le seul moyen de
   le faire entrer dans la machine virtuelle, qui n'aura aucun réseau.

Dans VMware Workstation :

1. Créer une machine Windows 10 **1809** 64 bits depuis l'ISO officielle de cette version. Une
   version ultérieure mise à jour par Windows Update peut contenir WebView2 et fausser l'essai.
2. 2 Go de mémoire, 1 cœur : le matériel modeste d'une école.
3. Retirer la carte réseau de la machine avant le premier démarrage. Ne pas installer les VMware
   Tools (ils apportent des composants système).
4. Créer un compte Windows **standard**, sans droits d'administrateur.
5. Vérifier qu'aucun WebView2 n'est présent : la clé
   `HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}`
   n'existe pas.
6. Prendre un instantané nommé **vierge**.

## Dérouler

Cocher chaque ligne ; une ligne en échec s'écrit dans « Écarts » avec ce qui a été vu.

### Installation — assertion 1

| # | Action | Attendu |
|---|---|---|
| 1.1 | Session standard, lancer l'installeur depuis l'ISO ou la clé | Assistant en français ; aucune demande d'élévation |
| 1.2 | Terminer l'installation | WebView2 installé sans réseau ; GESCO dans `%LOCALAPPDATA%\GESCO` ; raccourci dans le menu Démarrer |
| 1.3 | Lancer GESCO | La fenêtre s'ouvre sur « Premier démarrage » en moins de 10 secondes |

### Premier lancement — assertion 2

| # | Action | Attendu |
|---|---|---|
| 2.1 | Créer l'établissement et le compte administrateur | Base créée dans `%APPDATA%\ml.gesco.poste` |
| 2.2 | Lire la phrase de secours, cocher la confirmation | Les douze mots ne s'affichent plus ensuite ; les noter pour l'étape 9 |
| 2.3 | Ouvrir le fichier de base avec un éditeur hexadécimal | **Contenu chiffré** : l'en-tête `SQLite format 3` est absent |

L'étape 2.3 échouera tant que SQLCipher n'est pas actif (`ALERTES.md`) : la base du poste est
aujourd'hui en clair. Les archives de sauvegarde, elles, sont chiffrées.

### Session — assertion 3

| # | Action | Attendu |
|---|---|---|
| 3.1 | Se connecter, puis définir un code PIN dans « Mon compte » | PIN accepté (4 à 6 chiffres) |
| 3.2 | Ne plus toucher le poste pendant 2 minutes | Écran « Poste verrouillé » |
| 3.3 | Saisir le PIN, chronométrer jusqu'à l'écran de travail | Moins de 3 secondes |
| 3.4 | Saisir un PIN faux | « Code PIN incorrect. » ; la session reste verrouillée |

### Licence

| # | Action | Attendu |
|---|---|---|
| L.1 | Avant toute licence | Bandeau « Aucune licence n'est installée » ; l'application reste utilisable |
| L.2 | Outils → Licence : relever l'empreinte du poste | 8 groupes de 4 caractères |
| L.3 | Sur le poste de développement : `gesco-licence emettre --cle %USERPROFILE%\.gesco-editeur\cle-privee-licence.hex --etablissement "<nom>" --empreinte <empreinte> --sortie demo.lic` ; copier `demo.lic` sur la clé | Fichier émis |
| L.4 | Installer `demo.lic` depuis l'écran Licence | « Licence valide pour ce poste. » ; le bandeau disparaît ; le journal d'audit trace l'installation |

### Sauvegarde et restauration sur poste vierge — assertion 9

| # | Action | Attendu |
|---|---|---|
| 9.1 | Créer un second utilisateur, puis Journal → « Vérifier l'intégrité de la chaîne » | Noter le nombre de lignes vérifiées |
| 9.2 | Sauvegarde → ajouter la clé USB comme destination amovible, « Sauvegarder maintenant » | Une archive `.gsv` sur la clé ; l'historique indique « Réussie » |
| 9.3 | Revenir à l'instantané **vierge**, installer GESCO | « Premier démarrage » |
| 9.4 | « Restaurer une sauvegarde », « Parcourir… », choisir l'archive de la clé | Aperçu : établissement, date, identifiant d'enveloppe identique à celui du certificat |
| 9.5 | Saisir les douze mots, « Restaurer » | Retour à la connexion |
| 9.6 | Se connecter avec le compte administrateur d'origine | Les deux utilisateurs sont présents ; la vérification de la chaîne passe avec au moins le nombre de lignes noté en 9.1 |
| 9.7 | Recommencer 9.3 à 9.5 avec une phrase fausse, trois fois | Refus, puis attente imposée avant un nouvel essai |

La comparaison d'empreinte du contenu restauré avec celle du manifeste est faite par le test
`restauration_sur_un_poste_vierge_avec_la_seule_phrase_de_secours` (`gesco-services/tests/frontiere.rs`).

## Consigner

| Date | Installeur (version, SHA-256) | Machine | Résultat | Écarts |
|---|---|---|---|---|
| | | Windows 10 1809 x64, VMware, sans réseau | | |
