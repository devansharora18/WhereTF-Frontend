#!/usr/bin/env bash
set -euo pipefail

# Build the embedded backend sidecar (single executable) with PyInstaller.
# Self-contained: installs everything from embedded/requirements.txt, so it does
# NOT need the backend repo or a prebuilt backend image.
#
# Output: <repo>/embedded/dist/wheretf-backend
#
# Optional: WHERETF_BASE_IMAGE to reuse an image that already has the deps
# (e.g. localhost/wheretf-backend-backend:latest) for faster local iteration.

HERE="$(cd "$(dirname "$0")" && pwd)"     # <repo>/embedded
OUT="$HERE/dist"
BASE_IMAGE="${WHERETF_BASE_IMAGE:-docker.io/library/python:3.11-slim}"
BUILD_IMAGE="localhost/wheretf-embedded-build:latest"

echo "=== WhereTF embedded sidecar builder ==="
echo "Embedded dir: $HERE"
echo "Base image:   $BASE_IMAGE"

cat > /tmp/wheretf-embedded-build.Containerfile <<EOF
FROM $BASE_IMAGE
RUN apt-get update && apt-get install -y --no-install-recommends binutils && rm -rf /var/lib/apt/lists/*
COPY requirements.txt /tmp/requirements.txt
RUN pip install --no-cache-dir --extra-index-url https://download.pytorch.org/whl/cpu -r /tmp/requirements.txt
RUN pip install --no-cache-dir pyinstaller sqlite-vec
# Bake NLTK data so the frozen app never downloads at runtime.
RUN python -m nltk.downloader punkt punkt_tab wordnet omw-1.4 || true
EOF

podman build -t "$BUILD_IMAGE" -f /tmp/wheretf-embedded-build.Containerfile "$HERE" 2>&1 | tail -5

mkdir -p "$OUT"
podman run --rm \
  -v "$HERE:/src:z" \
  -v "$OUT:/out:z" \
  "$BUILD_IMAGE" \
  bash -c '
    set -e
    cd /src
    rm -rf /tmp/pyi
    pyinstaller wheretf-backend.spec --noconfirm --clean --distpath /out --workpath /tmp/pyi
    ls -lh /out
  '

echo ""
echo "Sidecar: $OUT/wheretf-backend"
ls -lh "$OUT/wheretf-backend" 2>&1 || true
