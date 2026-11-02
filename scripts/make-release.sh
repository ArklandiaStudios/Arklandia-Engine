#!/usr/bin/env bash
# Prépare release/arklandia-engine-<version>/ : binaire, SHA256SUMS.txt, RELEASE-NOTES.md,
# LICENSE, NOTICE.md, CHANGES.md, pumpkin.toml.example.
#
# Usage : scripts/make-release.sh
# Variables : CARGO_TARGET_DIR, TARGET, RELEASE_DIR (défaut <dépôt>/release),
#             SKIP_BUILD=1 pour réutiliser le binaire déjà compilé.
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET="${TARGET:-x86_64-unknown-linux-musl}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$REPO/target}"
RELEASE_DIR="${RELEASE_DIR:-$REPO/release}"

# Version du Cargo.toml du workspace, sans la métadonnée de build (+mc26.3) : 0.1.0
FULL="$(sed -n '/^\[workspace.package\]/,/^\[/{s/^version *= *"\(.*\)"/\1/p}' "$REPO/Cargo.toml" | head -n 1)"
[ -n "$FULL" ] || { echo "Version introuvable dans Cargo.toml" >&2; exit 1; }
VERSION="${FULL%%+*}"
NAME="arklandia-engine-$VERSION-linux-x86_64-musl"

[ "${SKIP_BUILD:-0}" = 1 ] || "$REPO/scripts/build.sh"
BIN="$CARGO_TARGET_DIR/$TARGET/release/arklandia-engine"
[ -x "$BIN" ] || { echo "Binaire introuvable : $BIN" >&2; exit 1; }

OUT="$RELEASE_DIR/arklandia-engine-$VERSION"
rm -rf "$OUT"; mkdir -p "$OUT"
cp "$BIN" "$OUT/$NAME"
cp "$REPO/LICENSE" "$REPO/NOTICE.md" "$REPO/CHANGES.md" "$OUT/"
cp "$REPO/docs/pumpkin.toml.example" "$OUT/pumpkin.toml.example"
cat > "$OUT/RELEASE-NOTES.md" <<NOTES
# Arklandia Engine $VERSION (pré-version)

> **Pré-version** : fonctionnelle mais encore en développement, à tester avant tout usage en production.

Version modifiée de Pumpkin (base \`204a94e\`) pour Minecraft Java (build $FULL). Binaire statique \`$NAME\` (Linux x86_64, musl), sous GNU GPL v3.

## Installation

1. Copier \`$NAME\` dans le dossier du serveur et le rendre exécutable (\`chmod +x\`).
2. Démarrer : \`pumpkin.toml\` est créé au premier lancement (exemple commenté : \`pumpkin.toml.example\`).
3. Copier les plugins Ark (\`*.wasm\`) dans \`plugins/\` et ajouter leurs \`allowed_permissions\` si besoin.

Le fichier de configuration s'appelle toujours \`pumpkin.toml\` et les plugins déjà compilés se chargent sans recompilation.

## Licence

GNU GPL v3 (voir \`LICENSE\` et \`NOTICE.md\`). Toute distribution du binaire à un tiers doit s'accompagner du code source complet correspondant. Détail des modifications : \`CHANGES.md\`.

## Vérification

\`sha256sum -c SHA256SUMS.txt\`
NOTES
(cd "$OUT" && sha256sum "$NAME" LICENSE NOTICE.md CHANGES.md pumpkin.toml.example RELEASE-NOTES.md > SHA256SUMS.txt)
echo "-> $OUT"
