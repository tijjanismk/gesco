# Une seule commande Tauri et un contrat de commandes déclaré en Rust

Date : 4 octobre 2026 — étape 12 de la phase 0.5.

## Décision

Le webview n'appelle qu'une commande Tauri, `executer(nom, jeton, entree)`. Elle remet l'appel à
`gesco_services::frontiere::Application`, qui applique la garde (session, permission, portée)
puis aiguille vers le gestionnaire de la commande. Les commandes, leurs types d'entrée et de
sortie, sont déclarés une seule fois dans la macro `contrat!` ; le type `Contrat` qui en résulte
est exporté en TypeScript par `specta` avec la table des accès (`frontend/src/types/`). Un test
compare le contrat au catalogue des accès (`commandes::COMMANDES`), un autre les fichiers générés
aux fichiers versionnés.

## Motif

R-21 demande un point de passage unique : un seul point d'entrée rend impossible une commande
oubliée par la garde. ADR-014 demande que le mode réseau local réutilise les mêmes
gestionnaires : la frontière vit dans `gesco-services`, la route HTTP de la phase 11 appellera la
même fonction. ADR-007 et R-37 demandent des types générés : changer une structure Rust casse la
compilation TypeScript.

## Conséquence

`gesco-app` reste mince : fenêtre, trousseau du système, tâches périodiques. Le frontend appelle
`appeler(nom, entree)`, typé par le contrat. `tauri-specta` n'est pas utilisé : il générerait une
commande Tauri par commande métier, ce que la commande unique rend inutile ; `specta` reste
épinglé en `=2.0.0-rc.25` comme le prévoit R-55.

## Alternative écartée

Une commande Tauri par commande métier, chacune gardée à la main : trente-deux occasions
d'oublier la garde, et un second aiguillage à écrire pour le mode réseau local.

## Amendement du 6 octobre 2026 — dialogue d'ouverture de fichier

Le webview reçoit une seconde capacité, `dialog:allow-open` du greffon `tauri-plugin-dialog`
2.8.1, et elle seule. Elle ouvre le sélecteur de fichier du système et rend un chemin ; elle ne
lit ni n'écrit aucun fichier. La restauration transmet ensuite ce chemin à `executer`, où la
garde s'applique comme avant : la règle de R-21 est intacte, aucune donnée ne passe hors de la
frontière. Le greffon tire `tauri-plugin-fs` en dépendance ; aucune de ses permissions n'est
accordée, le webview ne peut donc pas s'en servir.

Côté frontend, l'appel au dialogue vit dans `frontend/src/api/fichiers.ts`, comme tout accès
à Tauri (R-38). En mode réseau local, le navigateur n'a pas de dialogue natif : la saisie du
chemin reste possible et le transport HTTP fournira son propre envoi de fichier.

*Alternative écartée* : `<input type="file">`, retenu pour la licence parce qu'il suffit d'en
lire le texte. Une archive de sauvegarde pèse jusqu'à plusieurs centaines de mégaoctets ; la
faire transiter par le webview pour la réécrire sur le disque doublerait les lectures, alors
que le noyau sait l'ouvrir à son chemin.
