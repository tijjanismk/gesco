#!/bin/sh
# Vérifie la composition verticale avec les commandes de HyperFrames, qui ne lisent que
# l'index.html d'un dossier : copie compositions/vertical.html dans .verif-vertical/index.html,
# chemins ../assets/ ramenés à assets/, avec ses assertions de mouvement (vertical.motion.json),
# puis lance la commande demandée sur ce dossier.
#   sh outils/verifier-vertical.sh check --json
#   sh outils/verifier-vertical.sh snapshot --at 2,9,17
set -e
ici="$(cd "$(dirname "$0")/.." && pwd)"
miroir="$ici/.verif-vertical"
mkdir -p "$miroir"
sed 's|\.\./assets/|assets/|g' "$ici/compositions/vertical.html" > "$miroir/index.html"
cp "$ici/hyperframes.json" "$miroir/"
cp "$ici/compositions/vertical.motion.json" "$miroir/index.motion.json"
ln -sfn "$ici/assets" "$miroir/assets"
cd "$ici"
commande="$1"
shift
exec npx hyperframes "$commande" "$miroir" "$@"
