# Actes délivrés et vérification par le QR : un registre unique des actes, et le conseil de classe en deux tables

**Date** : 20 novembre 2026
**Statut** : proposé le 20 novembre 2026, accepté le 20 novembre 2026 (avec la spécification v1.1)
**Précise** : choix du porteur du 20 novembre 2026 (onze documents de l'école) ;
`PROJECT_BLUEPRINT.md` §13.2, §25.9 ; ADR-004 (numérotation) ; migrations `0001_socle`
(`document`, compteurs), `0021_departs` (`certificat_transfert`), `0039_discipline` ; R-14, R-20,
R-45, R-46
**Spécification** : `SPEC_DOCUMENTS_ECOLE.md`

## Contexte

- GESCO garde déjà chaque document produit dans la table `document` (chemin, empreinte, modèle,
  version) et imprime un QR `GESCO:DOC:<identifiant>`. Mais rien ne dit si un document est encore
  valable : la table n'a ni état, ni motif d'annulation, ni lien vers un remplaçant.
- La numérotation sans trou existe (compteurs par type et exercice). Un seul acte a sa table : le
  certificat de transfert (`certificat_transfert`, une ligne par inscription).
- La nouvelle phase ajoute neuf actes numérotés (scolarité, inscription, radiation, passage,
  réussite, lettre de sanction, procès-verbal, trois actes du personnel), tous vérifiables, tous
  annulables avec un motif, jamais supprimés.
- Le procès-verbal du conseil de classe demande une saisie que GESCO n'a pas : séance, présents,
  observations, observations par élève. Le reste (moyennes, rangs, sanctions, décisions) existe
  et doit être relu, pas recopié.
- Principe du porteur : la flexibilité sans incohérence. Un acte ne doit jamais dire autre chose
  que les données au jour de sa délivrance, et l'écran de vérification doit le montrer tel quel.
- GESCO est déjà installé en 2.0 dans l'école de test : la migration doit reprendre les
  certificats de transfert sans changer leur numéro.
- Aucune dépendance n'est ajoutée : rien à vérifier sur endoflife.date ni crates.io.

## Options

| | 1. Registre unique `acte_delivre` + `conseil_classe` | 2. Une table par acte, comme `certificat_transfert` | 3. Colonnes d'état ajoutées à `document`, numéro dans le JSON |
|---|---|---|---|
| Apprentissage | Faible : une table, un service | Faible par table, mais dix fois | Faible |
| Exploitation | Trois tables ; une seule requête pour vérifier, lister, annuler | Dix tables ; la vérification interroge chacune | Aucune table ; `document` mêle listes et actes |
| Maturité | Même forme que les tables versionnées existantes (état, motif, remplaçant) | Forme existante | Change le sens d'une table du socle |
| Pour un agent de code | Bon : un seul chemin pour délivrer, réimprimer, annuler | Moyen : code répété, oublis probables | Moyen : numéros et bénéficiaires cherchés dans du JSON |
| Plus gros risque | Un acte nouveau qui oublie de s'enregistrer au registre | Une table oubliée par la vérification : un vrai papier dit « inconnu » | Un numéro en double non vu, faute d'index unique |

## Décision

Option 1, dans la migration suivante libre :

- `acte_delivre` : identifiant, établissement, type (code du numéro), numéro (unique par
  établissement), document (`document.id`, unique), entité et identifiant du bénéficiaire
  (inscription, sanction, membre du personnel, conseil de classe), mentions attestées figées en
  JSON (ce que dit le papier, pour la vérification sans relire le PDF), signataire imprimé, état
  (`valide`, `annule`, `remplace`), date et motif d'annulation, acte de remplacement, colonnes
  d'audit ordinaires. Déclencheur : aucune suppression ; aucune modification des mentions.
- `conseil_classe` : classe, période, date, lieu, président, secrétaire, présents (JSON),
  observations, état (`saisi`, `arrete`), acte du procès-verbal arrêté ; un seul conseil vivant
  par classe et période.
- `conseil_classe_eleve` : observation courte par élève. Moyennes, rangs, distinctions, sanctions
  et décisions **ne sont pas stockés** : ils se relisent des bulletins publiés, de `sanction` et
  de `inscription` (R-20) ; l'acte arrêté les fige dans ses mentions.
- `certificat_transfert` reste ; la migration crée un acte `CTR` pour chaque ligne, avec le même
  numéro et le même document.
- Le QR garde `GESCO:DOC:<identifiant du document>` ; la vérification cherche par référence ou
  par numéro.

| Élément | Choix | Fin de support | Relais |
|---|---|---|---|
| Dépendances | Aucune nouvelle | — | — |

## Justification

1. Un seul endroit sait si un papier est vrai : la vérification ne peut pas oublier un type.
2. Annuler, remplacer, réimprimer suivent un seul chemin, essayé une fois pour tous les actes.
3. Les mentions figées permettent de montrer à la vérification ce que dit le papier, même si
   l'élève a changé de classe ou de nom depuis.
4. Ne stocker du conseil que ce qui se saisit garde le procès-verbal d'accord avec les bulletins
   par construction.
5. Garder `certificat_transfert` évite de toucher un acte déjà délivré en 2.0.
6. Aucune dépendance, aucune donnée personnelle de plus dans le QR.

## Conséquences

Positives : vérification, liste des actes délivrés et annulation pour tous les documents, d'un
coup ; aucun numéro existant ne change ; procès-verbal toujours égal aux bulletins.

Négatives : les mentions figées doublent une partie de ce que dit le PDF (quelques centaines
d'octets par acte) ; le certificat de transfert vit dans deux tables, tenues ensemble par la même
transaction ; tout acte futur doit passer par le service du registre (un essai parcourt le
catalogue et vérifie que chaque modèle numéroté y est enregistré).

## Contre-argument conservé

« Une table par acte donne des colonnes typées et lisibles en SQL, sans JSON. » Vrai ; mais la
vérification, la liste et l'annulation devraient connaître chaque table, et l'oubli d'une seule
ferait dire « inconnu » à un vrai papier, la pire erreur pour l'école.

## Conditions de reconsidération

- La vérification passe en ligne (service en ligne) : publier un extrait de `acte_delivre` sans
  donnée personnelle, et ajouter une adresse au QR par une nouvelle version des modèles.
- Un acte demande une recherche fréquente sur une mention (par exemple toutes les attestations de
  salaire d'un mois) : sortir cette mention du JSON en colonne indexée.
- Le conseil de classe doit saisir des décisions qui n'existent nulle part ailleurs : les ajouter
  à `conseil_classe_eleve`, sans dupliquer celles des inscriptions.
