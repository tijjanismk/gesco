#!/usr/bin/env bash
# Refuse la fusion si un marqueur de travail inachevé ou un texte de remplissage subsiste (R-56).
set -euo pipefail
cd "$(dirname "$0")/../.."
motifs='TODO|FIXME|XXX|lorem ipsum|John Doe|Acme'
if grep -rnE --include='*.rs' --include='*.ts' --include='*.tsx' --include='*.sql' --include='*.typ' "$motifs" crates frontend/src 2>/dev/null; then
  echo "Marqueur ou texte de remplissage trouvé (R-56)"; exit 1
fi
# Émojis (plans U+1F300–U+1FAFF et U+2600–U+27BF) dans le code source.
if grep -rnP --include='*.rs' --include='*.ts' --include='*.tsx' '[\x{1F300}-\x{1FAFF}\x{2600}-\x{27BF}]' crates frontend/src 2>/dev/null; then
  echo "Émoji trouvé dans le code source (R-56)"; exit 1
fi
