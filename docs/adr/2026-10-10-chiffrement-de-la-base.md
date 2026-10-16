# Chiffrement de la base par SQLite3 Multiple Ciphers

**Date** : 10 octobre 2026
**Statut** : accepté, sur décision du porteur du projet
**Remplace** : SQLCipher (`bundled-sqlcipher-vendored-openssl`), retiré le 7 octobre 2026

## Contexte

La base contient des données de mineurs (blueprint §3.12, R-24) ; l'assertion 2 de la phase 0.5
exige qu'elle naisse chiffrée. SQLCipher a été retiré le 7 octobre 2026 : sa compilation exigeait
OpenSSL, donc Strawberry Perl, absent du poste de développement. Le 10 octobre 2026, le porteur du
projet demande « un chiffrement simple » de la base pour les phases restantes, et abandonne l'essai
sur une machine Windows 10 vierge. Interrogé, il retient le chiffrement de la base entière, fait
tout de suite, avant la phase 4.

## Décision

SQLite3 Multiple Ciphers 2.5.1 (SQLite 3.53.4) est compilé à la place de SQLite :

- `vendor/libsqlite3-sys` est une copie de `libsqlite3-sys` 0.37.0 dont le script de construction
  compile l'amalgame de `sqlite3mc-src` au lieu de `sqlite3.c` ; le reste du fichier, les liaisons
  et l'API ne changent pas. Le `Cargo.toml` racine la substitue par `[patch.crates-io]`.
- Aucun OpenSSL, aucun Perl : le compilateur C déjà utilisé suffit.
- Chiffrement par défaut de la bibliothèque : ChaCha20-Poly1305, clé dérivée par PBKDF2.
- La phrase de passe est la clé de données de l'installation, en hexadécimal, celle que gardent
  l'enveloppe machine et l'enveloppe de secours (étape 11 de la phase 0.5). Rien de nouveau à
  retenir pour l'utilisateur.
- Une base trouvée en clair (installée avant ce changement, ou restaurée d'une archive ancienne)
  est chiffrée sur place à sa première ouverture avec clé : passage hors WAL, `PRAGMA rekey`.
  Une coupure pendant l'opération laisse la base en clair et intacte ; l'ouverture suivante
  recommence.
- `VACUUM INTO` produit un instantané chiffré avec la même clé : les archives `.gsv` contiennent
  désormais une base chiffrée dans une archive chiffrée.
- SQLite est compilé optimisé même en mode développement (`[profile.dev.package.libsqlite3-sys]`).

## Conséquences

- Tests : `crates/gesco-db/tests/chiffrement.rs` (base née chiffrée, illisible sans sa clé ou avec
  une autre ; conversion d'une base en clair ; instantané chiffré). Un test qui touche la base
  directement passe par `base_brute`, qui ouvre la clé du poste de test.
- Chaque connexion dérive la clé : quelques dizaines de millisecondes à l'ouverture du pool.
- Mettre à jour SQLite3 Multiple Ciphers : changer la version de `sqlite3mc-src` dans
  `vendor/libsqlite3-sys/Cargo.toml` ; mettre à jour `libsqlite3-sys` : refaire la copie et la même
  modification de `build.rs`, repérée par le commentaire « GESCO ».
- La perte de la clé de données et de la phrase de secours rend la base illisible : c'est voulu.
