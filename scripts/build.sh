#!/usr/bin/env bash
# Compile Arklandia Engine (binaire statique musl, profil release).
#
# Usage : scripts/build.sh [test]
# Variables : CARGO_TARGET_DIR (défaut <dépôt>/target), TARGET (défaut x86_64-unknown-linux-musl),
#             CARGO_NET_OFFLINE=true pour compiler sans réseau si les dépendances sont en cache.
#
# La génération de pumpkin-data est très gourmande en pile : sans les deux réglages ci-dessous,
# la compilation s'arrête sur un dépassement de pile.
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET="${TARGET:-x86_64-unknown-linux-musl}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$REPO/target}"
export RUST_MIN_STACK="${RUST_MIN_STACK:-536870912}"
ulimit -s unlimited 2>/dev/null || echo "Avertissement : ulimit -s unlimited refusé" >&2

cd "$REPO"
if [ "${1:-}" = test ]; then
    cargo test --target "$TARGET" -p pumpkin-util markup
    cargo test --target "$TARGET" -p pumpkin-config migrate
    cargo test --target "$TARGET" -p pumpkin-command
    cargo test --target "$TARGET" -p pumpkin-world anvil
    exit 0
fi
cargo build --release --target "$TARGET"
echo "-> $CARGO_TARGET_DIR/$TARGET/release/arklandia-engine"
