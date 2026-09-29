#!/usr/bin/env bash
set -euo pipefail

# Produce a SINGLE self-extracting binary: the Rust frontend with the frozen
# Python backend appended, plus a 32-byte trailer the Rust side reads at runtime.
#
#   [ frontend ][ sidecar bytes ][ magic(16) | offset(u64 LE) | length(u64 LE) ]
#
# Prereqs:
#   - frontend built with the embedded-backend feature:
#       cargo build --release --features embedded-backend
#   - sidecar built:  bash embedded/build_sidecar.sh
#
# Output: dist/WhereTF

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FRONTEND="$ROOT/target/release/wheretf-frontend"
SIDECAR="$ROOT/embedded/dist/wheretf-backend"
OUTDIR="$ROOT/dist"
OUT="$OUTDIR/WhereTF"

[ -f "$FRONTEND" ] || { echo "ERROR: build frontend first: cargo build --release --features embedded-backend"; exit 1; }
[ -f "$SIDECAR" ]  || { echo "ERROR: build sidecar first: bash embedded/build_sidecar.sh"; exit 1; }

mkdir -p "$OUTDIR"
python3 "$ROOT/scripts/embed_sidecar.py" "$FRONTEND" "$SIDECAR" "$OUT"

echo ""
echo "Single-file app: $OUT"
ls -lh "$OUT"
