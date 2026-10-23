# GESCO 1.2.3 — notes de version

Version du 23 octobre 2026. Elle revoit les réglages livrés de la sécurité et des sauvegardes, et
permet à un administrateur qui a perdu son mot de passe de le remplacer.

## Ce qui change

**Mot de passe administrateur oublié.** Sur l'écran de connexion, « Mot de passe administrateur
oublié ? » : saisissez les douze mots du certificat de clé de secours, l'identifiant de
l'administrateur et son nouveau mot de passe. Celui qui détient les douze mots peut déjà
restaurer toutes les données : rien de plus ne s'ouvre. Le remplacement est noté au journal
d'audit et le tableau de bord le signale pendant sept jours. Gardez le certificat en lieu sûr.

**Certificat de clé de secours.** La case « J'ai imprimé le certificat et je l'ai rangé hors du
poste », cochée au premier démarrage ou après une nouvelle phrase, suffit : le bandeau ne
réclame plus de confirmation ailleurs.

**Sécurité et sauvegardes, pour une installation neuve.**
- La session se verrouille après 15 minutes sans activité (au lieu de 2), et se ferme après
  24 heures (au lieu de 8).
- Un mot de passe compte au moins 6 caractères, sans exigence de difficulté : `000000` est
  accepté. L'école peut refuser les mots de passe les plus courants dans ses paramètres.
- La sauvegarde automatique a lieu chaque jour (au lieu de chaque heure), avec la même
  conservation : 7 quotidiennes, 4 hebdomadaires, 12 mensuelles.

**Corrigé.** Les deux durées de session réglées dans Paramètres → Établissement n'étaient pas
appliquées : GESCO restait à 2 minutes et 8 heures. Elles s'appliquent maintenant dès qu'on les
change.

## Mettre à jour

Installez la 1.2.3 par-dessus la version en place : les données sont gardées. Une école déjà
installée garde ses réglages de sécurité et de sauvegarde ; pour passer aux nouvelles valeurs,
réglez-les dans **Paramètres → Établissement**, sections « Sécurité » et « Sauvegardes ».
