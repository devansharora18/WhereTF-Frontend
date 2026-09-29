#!/usr/bin/env bash
set -euo pipefail

# WhereTF - build 3 tier-specific offline installers: lite / balanced / pro.
#
# The frontend binary and the backend container image are identical across tiers;
# the tier is selected at runtime via APP_TIER. So each installer is the same
# bundle with APP_TIER pinned (for BOTH the backend and the Celery worker) in the
# bundled docker-compose.yml, plus a tier-specific name.
#
# Output (in dist/):
#   WhereTF-lite/          WhereTF-lite-offline.tar.gz       WhereTF-lite-installer.sh
#   WhereTF-balanced/      WhereTF-balanced-offline.tar.gz   WhereTF-balanced-installer.sh
#   WhereTF-pro/           WhereTF-pro-offline.tar.gz        WhereTF-pro-installer.sh
#
# Prereqs: the backend image must already be built (see WhereTF-backend/Dockerfile).

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BACKEND_DIR="$ROOT/WhereTF-backend"
DIST="$ROOT/dist"
TIERS=(lite balanced pro)

echo "=== WhereTF tier bundle builder ==="

# 0. Make sure all image tag variants exist.
if ! podman images --format "{{.Repository}}:{{.Tag}}" | grep -q "wheretf-backend-backend"; then
  echo "ERROR: backend image not built. Build it first:"
  echo "  (cd WhereTF-backend && podman build -t localhost/wheretf-backend-backend:latest .)"
  exit 1
fi
podman tag localhost/wheretf-backend-backend:latest localhost/wheretf-backend-backend:latest 2>/dev/null || true
podman tag localhost/wheretf-backend-backend:latest localhost/wheretf-backend_backend:latest 2>/dev/null || true
podman tag localhost/wheretf-backend-backend:latest localhost/wheretf-backend-worker:latest 2>/dev/null || true
podman tag localhost/wheretf-backend-backend:latest localhost/wheretf-backend_worker:latest 2>/dev/null || true

# 1. Refresh the offline images tar with the current image.
echo "[1/3] Saving backend images tar (1.9GB)..."
podman save -o "$BACKEND_DIR/backend-images.tar" \
  localhost/wheretf-backend-backend:latest \
  localhost/wheretf-backend_backend:latest \
  localhost/wheretf-backend-worker:latest \
  localhost/wheretf-backend_worker:latest \
  docker.io/pgvector/pgvector:pg17 \
  docker.io/library/redis:7-alpine
ls -lh "$BACKEND_DIR/backend-images.tar"

# 2. Build the base offline bundle (frontend + backend + images).
echo "[2/3] Building base offline bundle..."
bash "$ROOT/scripts/bundle_offline.sh" >/tmp/wheretf_base_bundle.log 2>&1 || {
  echo "base bundle failed; tail:"; tail -20 /tmp/wheretf_base_bundle.log; exit 1; }
BASE="$DIST/WhereTF-offline"
[ -d "$BASE" ] || { echo "ERROR: $BASE missing"; exit 1; }

# 3. Derive one bundle + installer per tier.
echo "[3/3] Deriving tier bundles..."
for tier in "${TIERS[@]}"; do
  TIERDIR="$DIST/WhereTF-$tier"
  echo "  -> $tier"
  rm -rf "$TIERDIR"
  cp -a "$BASE" "$TIERDIR"

  # Pin the tier for both services (replaces every 'APP_TIER: ...' line).
  sed -i "s/^\(\s*\)APP_TIER: .*/\1APP_TIER: $tier/" "$TIERDIR/WhereTF-backend/docker-compose.yml"

  # Make the code bind-mount SELinux-friendly on Fedora/RHEL hosts (no-op elsewhere).
  sed -i "s|- \.:/app$|- .:/app:z|g" "$TIERDIR/WhereTF-backend/docker-compose.yml"

  # Make it clear in the README which tier this is.
  sed -i "1i WhereTF — $tier tier\n==========================\nThis bundle is pinned to APP_TIER=$tier for both the backend and worker.\n" \
    "$TIERDIR/README.txt"

  # Portable tar.gz
  (cd "$DIST" && tar -czf "WhereTF-$tier-offline.tar.gz" "WhereTF-$tier")

  # Self-extracting single-file installer
  INSTALLER="$DIST/WhereTF-$tier-installer.sh"
  cat > "$INSTALLER" <<HEADER
#!/usr/bin/env bash
# WhereTF ($tier) offline self-extracting installer.
# Usage: bash WhereTF-$tier-installer.sh [--target DIR]
set -e
TARGET="\${1:-\$HOME/WhereTF-$tier}"
if [ "\$1" = "--target" ]; then TARGET="\$2"; fi
echo "=== WhereTF ($tier) Installer ==="
echo "Target: \$TARGET"
mkdir -p "\$TARGET"
echo "Extracting..."
ARCHIVE_LINE=\$(awk '/^__ARCHIVE_BELOW__/ {print NR + 1; exit 0; }' "\$0")
tail -n +\$ARCHIVE_LINE "\$0" | tar -xz -C "\$TARGET"
echo "Installed to \$TARGET"
echo "Launching..."
bash "\$TARGET/WhereTF-$tier/launch.sh"
exit 0
__ARCHIVE_BELOW__
HEADER
  cat "$DIST/WhereTF-$tier-offline.tar.gz" >> "$INSTALLER"
  chmod +x "$INSTALLER"
done

echo ""
echo "=== Done ==="
for tier in "${TIERS[@]}"; do
  printf "  %-9s %-32s %s\n" "$tier" \
    "$DIST/WhereTF-$tier-offline.tar.gz ($(du -h "$DIST/WhereTF-$tier-offline.tar.gz" | cut -f1))" \
    "$DIST/WhereTF-$tier-installer.sh"
done
echo ""
echo "Each installer is self-contained for its tier. APP_TIER is pinned in the bundled compose."
echo "Note: the first backend start downloads that tier's model from Hugging Face (needs internet once)."
