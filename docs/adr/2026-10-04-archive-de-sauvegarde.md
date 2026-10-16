# Archive de sauvegarde chiffrée par la clé de données, indépendamment de SQLCipher

Date : 4 octobre 2026 — étape 11 de la phase 0.5.

## Décision

Une sauvegarde est un fichier `.gsv` : un en-tête, un manifeste JSON en clair, puis le contenu
(instantané de la base par `VACUUM INTO`, modèles sur mesure) compressé et découpé en blocs d'un
mébioctet chiffrés par XChaCha20-Poly1305 avec la clé de données. Le nonce de chaque bloc porte
le préfixe de l'archive et le rang du bloc ; les données associées portent l'empreinte du
manifeste et le marqueur de dernier bloc. Le manifeste embarque l'enveloppe de secours, jamais
l'enveloppe machine.

## Motif

R-57 et l'assertion 9 : le fichier et la phrase de secours suffisent sur une machine vierge. Le
chiffrement de l'archive ne dépend pas de SQLCipher, inactif tant que la crate SQLite n'est pas
compilée avec lui : la sauvegarde est chiffrée dès maintenant, quel que soit l'état de la base.
La construction par blocs authentifiés détecte une archive altérée, tronquée, réordonnée ou
dont le manifeste a été modifié, et se lit en flux sans charger la base en mémoire.

## Conséquence

L'assistant de restauration lit le manifeste sans clé (établissement, date, version, identifiant
d'enveloppe à rapprocher du certificat). La restauration vérifie l'empreinte du fichier, migre une
base antérieure, contrôle l'intégrité et la chaîne d'audit, puis bascule par renommage ;
l'ancienne base est conservée à côté. Le format porte un numéro (`FORMAT_ARCHIVE`) : un
changement crée un format 2 et garde la lecture du format 1.

## Alternative écartée

Copier le fichier SQLCipher tel quel : la sauvegarde n'aurait été chiffrée qu'une fois SQLCipher
actif, et la compression d'un fichier déjà chiffré ne gagne rien.
