# Portail des parents : application web installable, React du dépôt, sans notification poussée

**Date** : 15 novembre 2026
**Statut** : proposé le 15 novembre 2026
**Précise** : blueprint ADR-017 (portail en application web progressive), §3.9, §16 (portail
élève, même technique) ; étapes D1 à D4 de `docs/plan-phase-11.md` §2.5
**Spécification** : `SPEC_LOTS_C_D.md`

## Contexte

ADR-017 a déjà tranché : le portail est une application web installable sur l'écran d'accueil,
jamais une application de magasin. Il reste à décider comment la construire, avec quoi, et si
elle envoie des notifications poussées, comme ADR-017 le prévoyait.

Contraintes :

- **Téléphones des parents** : Android pour la plupart, souvent anciens, peu de stockage ;
  quelques iPhone. Forfait compté : chaque centaine de kilo-octets au premier chargement compte.
- **Neuf consultations par an et par parent** (blueprint §3.9) : le portail doit s'ouvrir vite,
  se comprendre sans aide, et montrer le dernier état même sans réseau.
- **Lisibilité** : contrastes et tailles du projet (WCAG niveau AA), écran de 320 pixels.
- **Un seul développeur** : pas de second outillage. Le dépôt a déjà React 19.3.0, Vite 8.3.2,
  TypeScript, Tailwind, Vitest et Playwright pour l'application de bureau.
- **L'école prévient déjà par SMS** quand un bulletin est publié (phase 5). Le parent a donc
  déjà l'avis ; il lui manque le lien.
- Les notifications poussées du web demandent des clés propres au serveur, un service de
  distribution tenu par Google ou Apple, une permission du parent, et, sur iPhone, que
  l'application soit installée.
- Le portail sera aussi la base du portail des élèves de l'e-learning en mode en ligne (§16).

## Options

| | 1. React du dépôt, service worker écrit à la main, pas de notification poussée | 2. Comme 1, avec vite-plugin-pwa (Workbox) et notifications poussées | 3. Pages composées par le serveur, sans cadriciel |
|---|---|---|---|
| Apprentissage | Faible : les outils du bureau, un fichier de service worker d'une centaine de lignes | Moyen : Workbox, clés et abonnements des notifications | Moyen : une bibliothèque de gabarits en Rust, un second style d'écran |
| Exploitation | Rien de plus que le serveur | Abonnements à tenir, envois à surveiller, service de distribution tiers | Rien de plus |
| Maturité | React et Vite : en service dans le dépôt | vite-plugin-pwa 2.0.0 sorti le 3 octobre 2026 (série neuve) ; crate `web-push` 0.11.0 sans sortie depuis février 2025 | Bonne, mais rien du dépôt n'est réutilisé |
| Pour un agent de code | Très bon : mêmes essais (Vitest, Playwright) | Moyen : notifications difficiles à essayer | Moyen : aucun composant ni essai existant |
| Plus gros risque | Un premier chargement trop lourd | Une dépendance mal tenue ; des parents qui refusent la permission | Deux façons de faire des écrans dans un même dépôt |

## Décision

Le portail est une seconde entrée du frontend React du dépôt (`frontend/portail/`), avec un
manifeste et un service worker écrits à la main, sans notification poussée dans la 2.0 ; ses
fichiers sont servis par `gesco-server`.

| Brique | Version | Sortie | Suite prévue |
|---|---|---|---|
| React, React DOM | 19.3.0 (déjà dans le dépôt) | — | Suit le bureau |
| Vite | 8.3.2 (déjà dans le dépôt) | — | Suit le bureau |
| Pour mémoire, non retenus : vite-plugin-pwa | 2.0.0 | 3 octobre 2026 | — |
| Pour mémoire, non retenu : workbox-window | 7.4.1 | 4 mai 2026 | — |
| Pour mémoire, non retenue : crate web-push | 0.11.0 | 22 février 2025 | — |

Vérifié le 15 novembre 2026 sur npm (vite-plugin-pwa accepte Vite 8) et crates.io. Non vérifié :
le poids réel du premier chargement (à mesurer à l'étape D1) ; le comportement sur les téléphones
des parents de l'école pilote (à mesurer à l'étape D3).

Ce que la décision fixe :

- **Une seconde entrée Vite**, construite à part du bureau ; elle n'emporte ni `@tauri-apps`, ni
  le transport Tauri, ni les polices Fira (polices du téléphone, pour le poids). Elle réutilise
  les composants simples et les couleurs du bureau quand ils ne tirent rien de lourd.
- **Poids** : premier chargement de 150 Ko compressés au plus, tout compris ; les visites
  suivantes ne téléchargent que les vues qui ont changé.
- **Service worker écrit à la main** : il garde l'application et la dernière vue reçue ; sans
  réseau, le portail montre ce dernier état avec sa date ; « Se déconnecter » efface tout ce que
  le téléphone garde.
- **Installation** : Android propose l'ajout à l'écran d'accueil ; une page d'aide courte montre
  le geste sur iPhone. Le portail marche aussi sans être installé, dans le navigateur.
- **Pas de notification poussée dans la 2.0** : l'avis reste le SMS que l'école envoie déjà à la
  publication d'un bulletin, auquel s'ajoute le lien du portail quand l'école l'a allumé. Dans le
  portail, ce qui est nouveau depuis la dernière visite est marqué.
- **Sécurité de la page** : aucun script, police, image ou mesure d'audience venus d'ailleurs ;
  politique de contenu stricte ; HTTPS seulement.
- **Même règle d'écriture** que le bureau : aucun texte affiché ne cite de source ; mots simples,
  montants en francs CFA, dates en toutes lettres.

## Justification

ADR-017 a choisi le web installable pour ne pas porter un second produit ; prendre les outils déjà
en service dans le dépôt suit la même logique : mêmes essais, mêmes composants, rien de nouveau à
apprendre. Un service worker qui garde une page et un document tient en une centaine de lignes ;
Workbox apporterait une dépendance neuve pour un besoin si petit. Les notifications poussées
ajouteraient un service tiers, des clés, une permission que beaucoup refusent, et une crate peu
tenue, pour prévenir d'un bulletin que le SMS annonce déjà. Les pages composées par le serveur
seraient légères, mais créeraient une seconde façon de faire des écrans, que le portail des élèves
devrait ensuite reprendre.

## Conséquences

Positives : aucune dépendance nouvelle ; les essais du bureau couvrent le portail ; le portail des
élèves (e-learning en ligne) partira de la même base ; rien qui dépende de Google ou d'Apple pour
fonctionner.

Négatives : React pèse une part notable du budget de 150 Ko ; le service worker écrit à la main
demande des essais soignés (mise à jour de l'application, cache périmé) ; sans notification
poussée, le parent qui n'a pas reçu le SMS ne sait pas qu'un bulletin l'attend ; ADR-017 est
précisé sur ce point, à reporter dans le blueprint.

## Contre-argument conservé

Une notification poussée est gratuite à l'envoi, alors que le SMS coûte à l'école ; elle ferait du
portail un canal qui remplace peu à peu le SMS. Elle n'a pas gagné pour la 2.0 parce que le SMS
part déjà, que la notification demande une installation et une permission que peu de parents
donneront (§3.9), et qu'elle ajoute une dépendance mal tenue et un service tiers à exploiter.

## Conditions de reconsidération

- L'école pilote mesure que plus de la moitié des parents ont installé le portail : la
  notification poussée devient rentable face au coût des SMS.
- Le premier chargement dépasse 150 Ko compressés sans solution simple : regarder une
  bibliothèque d'écrans plus légère pour le portail seul.
- Le service worker écrit à la main cause des caches périmés chez les parents : reprendre Workbox.
- Le portail des élèves (§16) exige autre chose que ce socle.
