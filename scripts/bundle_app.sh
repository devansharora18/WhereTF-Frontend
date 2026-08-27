#!/usr/bin/env bash
set -euo pipefail
# WhereTF - App Bundle (Frontend Only, Split Mode)
# Builds just the frontend binary (standalone, no backend auto-start)
# User must run the backend service separately via `cd WhereTF-backend && podman-compose up -d`

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DIST="$ROOT/dist"

echo "=== WhereTF App Bundle (Frontend Only) ==="

echo "[1/2] Building frontend (cargo build --release, default features = standalone)..."
(cd "$ROOT" && cargo build --release)
if [ ! -f "$ROOT/target/release/wheretf-frontend" ]; then
  echo "ERROR: frontend binary not found"
  exit 1
fi
ls -lh "$ROOT/target/release/wheretf-frontend"

echo "[2/2] Creating dist/WhereTF-app..."
mkdir -p "$DIST/WhereTF-app"
cp "$ROOT/target/release/wheretf-frontend" "$DIST/WhereTF-app/WhereTF"
chmod +x "$DIST/WhereTF-app/WhereTF"
# Also copy as .exe for Windows users (same binary, just renamed)
cp "$ROOT/target/release/wheretf-frontend" "$DIST/WhereTF-app/WhereTF.exe" 2>/dev/null || true

# Write a .desktop entry the installer can drop in ~/.local/share/applications
# so GNOME Wayland's portal grants the global Alt+K shortcut (requires an app id).
cat > "$DIST/WhereTF-app/wheretf.desktop" <<DESK
[Desktop Entry]
Type=Application
Name=WhereTF
Exec=$DIST/WhereTF-app/WhereTF
Icon=system-search
Terminal=false
StartupWMClass=wheretf-frontend
DESK
# Install the desktop entry (registers the app id for the GlobalShortcuts portal)
mkdir -p "$HOME/.local/share/applications"
cp "$DIST/WhereTF-app/wheretf.desktop" "$HOME/.local/share/applications/wheretf.desktop"

cat > "$DIST/WhereTF-app/README.txt" <<'README'
WhereTF - App (Frontend Only)
=============================
This is the standalone frontend binary. It expects the backend service
to be running separately.

Backend Service:
  cd ../WhereTF-backend && podman-compose up -d
  # or: podman compose up -d / docker-compose up -d
  # Backend will be at http://localhost:8000
  # Check: curl http://localhost:8000/health

Run App:
  ./WhereTF              (Linux/macOS)
  ./WhereTF.exe          (Windows, or WhereTF on Windows)
  # App will show "Backend not running" if service is not up, and will
  # auto-retry every 5s until it is.

Build:
  cargo build --release              # standalone (default)
  cargo build --release --features bundled-backend  # combined (auto-starts backend)

For fully offline single-file with backend included, use:
  bash scripts/bundle_offline.sh  # creates dist/WhereTF-installer.sh (608M)
README

# Create tar.gz
(cd "$DIST" && tar -czf WhereTF-app.tar.gz WhereTF-app)
ls -lh "$DIST/WhereTF-app.tar.gz"
echo ""
echo "Done: $DIST/WhereTF-app/WhereTF (frontend only, standalone)"
echo "      $DIST/WhereTF-app.tar.gz"
echo ""
echo "To run the service separately:"
echo "  cd WhereTF-backend && podman-compose up -d"
echo "Then run the app:"
echo "  ./dist/WhereTF-app/WhereTF"
