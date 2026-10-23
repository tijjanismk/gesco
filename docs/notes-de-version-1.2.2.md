# GESCO 1.2.2 — notes de version

Version du 23 octobre 2026. Elle change la date des échéances et la forme des matricules, au
choix de chaque école.

## Ce qui change

**Échéances au 5 du mois.** Les échéances d'une grille de frais neuve tombent le 5 du mois au
lieu du 15. Le jour se règle dans **Paramètres → Établissement** (« Jour du mois des échéances
des nouvelles grilles de frais », du 1er au 28). Sur une grille en brouillon, le bouton « Mettre
toutes les échéances à ce jour » déplace toutes les dates d'un clic. Les grilles déjà en vigueur
ne changent pas.

**Un jour d'échéance pour un seul élève.** Un parent payé le 10 peut payer le 10 : dans les frais
de la fiche de l'élève, saisissez le jour, puis « Décaler ses échéances ». Ses échéances de
scolarité pas encore payées passent à ce jour de leur mois ; celles déjà payées, même en partie,
gardent leur date ; les remises suivent.

**Élève arrivé en cours d'année.** Une nouvelle règle, livrée par défaut aux écoles neuves : la
tranche du mois d'arrivée compte, même si son jour est passé (un élève inscrit le 14 octobre paie
la tranche du 5 octobre). L'école garde le choix de sa règle dans Établissement, et une autre
règle peut être choisie pour un élève à la validation de son inscription.

**Matricules à la barre oblique.** Le matricule généré d'une école neuve s'écrit `2026/00042`.
Le séparateur se règle dans Établissement : barre oblique, tiret, point ou rien. Une école qui a
déjà des élèves garde le tiret à la mise à jour, pour que ses matricules restent d'une seule
forme.

**SMS d'essai prioritaire.** Le SMS d'essai et la réponse à un parent qui consulte par SMS
passent devant les relances en attente.

## Mettre à jour

Installez la 1.2.2 par-dessus la version en place : les données sont gardées. Les écoles déjà
installées gardent leur règle d'arrivée en cours d'année et le tiret de leurs matricules ; le
jour des échéances ne vaut que pour leurs prochaines grilles.
