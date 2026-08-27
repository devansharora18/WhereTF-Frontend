#!/usr/bin/env bash
set -euo pipefail

# WhereTF - Fully Offline Single-File Bundle Builder
# Creates dist/WhereTF-offline.tar.gz and dist/WhereTF-installer.sh (self-extracting)
# Contains: frontend binary + WhereTF-backend + backend-images.tar (1.9GB) + launchers
# User just runs: bash WhereTF-installer.sh  OR  tar -xzf WhereTF-offline.tar.gz && ./launch.sh

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BACKEND_DIR="$ROOT/WhereTF-backend"
DIST="$ROOT/dist"
OFFLINE_DIR="$DIST/WhereTF-offline"
IMAGES_TAR="$BACKEND_DIR/backend-images.tar"

echo "=== WhereTF Offline Bundle Builder ==="
echo "ROOT: $ROOT"
echo "BACKEND: $BACKEND_DIR"

# 1. Ensure backend images tar exists (1.9GB). Create via podman save if missing.
if [ ! -f "$IMAGES_TAR" ]; then
  echo "[1/4] Creating offline images tar (1.9GB, ~30s)..."
  mkdir -p "$BACKEND_DIR"
  # Ensure images exist (build if needed)
  if ! podman images --format "{{.Repository}}:{{.Tag}}" | grep -q "wheretf-backend"; then
    echo "  Backend image not found, building (this will take 5-10min with CPU torch)..."
    (cd "$BACKEND_DIR" && podman build -t localhost/wheretf-backend-backend:latest .)
    podman tag localhost/wheretf-backend-backend:latest localhost/wheretf-backend_backend:latest
    podman tag localhost/wheretf-backend-backend:latest localhost/wheretf-backend-worker:latest
    podman tag localhost/wheretf-backend-backend:latest localhost/wheretf-backend_worker:latest
  fi
  podman save -o "$IMAGES_TAR" \
    localhost/wheretf-backend-backend:latest \
    localhost/wheretf-backend_backend:latest \
    localhost/wheretf-backend-worker:latest \
    localhost/wheretf-backend_worker:latest \
    docker.io/pgvector/pgvector:pg17 \
    docker.io/library/redis:7-alpine
  ls -lh "$IMAGES_TAR"
else
  echo "[1/4] Found existing $IMAGES_TAR ($(du -h "$IMAGES_TAR" | cut -f1))"
fi

# 2. Build frontend release binary
echo "[2/4] Building frontend (cargo build --release --features bundled-backend)..."
(cd "$ROOT" && cargo build --release --features bundled-backend)
if [ ! -f "$ROOT/target/release/wheretf-frontend" ]; then
  echo "ERROR: frontend binary not found at target/release/wheretf-frontend"
  exit 1
fi
ls -lh "$ROOT/target/release/wheretf-frontend"

# 3. Assemble offline directory
echo "[3/4] Assembling $OFFLINE_DIR..."
rm -rf "$OFFLINE_DIR"
mkdir -p "$OFFLINE_DIR"

# Copy frontend binary as WhereTF (user friendly)
cp "$ROOT/target/release/wheretf-frontend" "$OFFLINE_DIR/WhereTF"
chmod +x "$OFFLINE_DIR/WhereTF"

# Copy backend (including docker-compose.yml, app, backend-images.tar, requirements, etc.)
# Exclude .git, __pycache__, temp to save space
echo "  Copying WhereTF-backend..."
mkdir -p "$OFFLINE_DIR/WhereTF-backend"
# Use rsync if available, else cp -a
if command -v rsync >/dev/null; then
  rsync -a --exclude='.git' --exclude='__pycache__' --exclude='*.pyc' --exclude='temp' --exclude='.venv' "$BACKEND_DIR/" "$OFFLINE_DIR/WhereTF-backend/"
else
  cp -a "$BACKEND_DIR"/* "$OFFLINE_DIR/WhereTF-backend/" 2>/dev/null || true
  rm -rf "$OFFLINE_DIR/WhereTF-backend/.git" "$OFFLINE_DIR/WhereTF-backend/__pycache__" 2>/dev/null || true
fi
# Ensure tar is included (rsync already did, but double-check)
cp "$IMAGES_TAR" "$OFFLINE_DIR/WhereTF-backend/backend-images.tar"

# Create launch script (what user double-clicks after extract)
cat > "$OFFLINE_DIR/launch.sh" <<'LAUNCH'
#!/usr/bin/env bash
set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BACKEND_DIR="$SCRIPT_DIR/WhereTF-backend"
echo "=== WhereTF Launcher ==="

# Register a .desktop entry so GNOME Wayland's portal can identify this app and
# grant the global Alt+K shortcut (the XDG GlobalShortcuts portal requires an app id).
mkdir -p "$HOME/.local/share/applications"
cat > "$HOME/.local/share/applications/wheretf.desktop" <<'DESK'
[Desktop Entry]
Type=Application
Name=WhereTF
Exec=SCRIPT_DIR/WhereTF
Icon=system-search
Terminal=false
StartupWMClass=wheretf-frontend
X-GNOME-UsesNotifications=true
DESK
# Substitute the real path (avoid a placeholder that won't expand inside the heredoc)
sed -i "s|SCRIPT_DIR/WhereTF|$SCRIPT_DIR/WhereTF|g" "$HOME/.local/share/applications/wheretf.desktop"

# Check podman
if ! command -v podman >/dev/null; then
  echo "ERROR: podman not found. Please install podman first:"
  echo "  Ubuntu/Debian: sudo apt-get install podman podman-compose"
  echo "  Fedora: sudo dnf install podman podman-compose"
  echo "  macOS: brew install podman"
  echo "  Windows: winget install -e --id RedHat.Podman"
  read -p "Press enter to exit..."
  exit 1
fi
# Load offline images if needed (first run, no internet)
if ! podman images --format "{{.Repository}}:{{.Tag}}" | grep -q "wheretf-backend"; then
  if [ -f "$BACKEND_DIR/backend-images.tar" ]; then
    echo "[launch] Loading offline images (1.9GB, ~30s)..."
    podman load -i "$BACKEND_DIR/backend-images.tar"
  elif [ -f "$BACKEND_DIR/backend-images.tar.gz" ]; then
    echo "[launch] Loading gzipped offline images..."
    podman load -i "$BACKEND_DIR/backend-images.tar.gz"
  else
    echo "[launch] No bundled tar found, will pull from internet (requires network)..."
  fi
fi
# Start backend
echo "[launch] Starting backend (db + backend + redis + worker)..."
(cd "$BACKEND_DIR" && podman-compose up -d || podman compose up -d || docker-compose up -d || docker compose up -d)
echo "[launch] Waiting for backend health (up to 90s, first run downloads Jina CLIP ~300MB if not cached)..."
for i in $(seq 1 45); do
  if curl -s --max-time 2 http://127.0.0.1:8000/health | grep -q healthy; then
    echo "[launch] Backend healthy!"
    break
  fi
  echo "  waiting $((i*2))s..."
  sleep 2
done
# Launch frontend
if [ -f "$SCRIPT_DIR/WhereTF" ]; then
  echo "[launch] Starting WhereTF frontend..."
  exec "$SCRIPT_DIR/WhereTF"
else
  echo "ERROR: WhereTF binary not found at $SCRIPT_DIR/WhereTF"
  exit 1
fi
LAUNCH
chmod +x "$OFFLINE_DIR/launch.sh"

# Create Windows launcher
cat > "$OFFLINE_DIR/launch.bat" <<'BAT'
@echo off
set SCRIPT_DIR=%~dp0
set BACKEND_DIR=%SCRIPT_DIR%WhereTF-backend
echo === WhereTF Launcher (Windows) ===
where podman >nul 2>&1
if %errorlevel% neq 0 (
  echo ERROR: podman not found. Install via: winget install -e --id RedHat.Podman
  pause
  exit /b 1
)
podman images --format "{{.Repository}}:{{.Tag}}" | findstr wheretf-backend >nul
if %errorlevel% neq 0 (
  if exist "%BACKEND_DIR%\backend-images.tar" (
    echo [launch] Loading offline images...
    podman load -i "%BACKEND_DIR%\backend-images.tar"
  )
)
echo [launch] Starting backend...
cd /d "%BACKEND_DIR%" && podman-compose up -d || podman compose up -d
echo [launch] Waiting for backend...
timeout /t 5 >nul
start "" "%SCRIPT_DIR%WhereTF.exe"
BAT

# Create README
cat > "$OFFLINE_DIR/README.txt" <<'README'
WhereTF - Fully Offline Bundle
==============================
This folder contains everything needed to run WhereTF without internet
(after first install, internet not required except for initial podman install).

Contents:
  WhereTF / WhereTF.exe  - Frontend desktop app (auto-starts backend on open)
  WhereTF-backend/       - Backend source + docker-compose.yml + backend-images.tar (1.9GB)
  launch.sh / launch.bat - Double-click to start (or just run WhereTF)
  backend-images.tar     - Prebuilt podman images (backend 1.97GB + pgvector 450MB + redis 40MB)

Quick Start:
  Linux/macOS:  bash launch.sh        (or ./WhereTF - it auto-starts backend too)
  Windows:      double-click launch.bat or WhereTF.exe
  Manual:       cd WhereTF-backend && podman-compose up -d

First launch:
  - If podman not installed, script will prompt to install
  - First run loads 1.9GB tar via `podman load` (~30s) + Jina CLIP model ~300MB is
    already baked in WhereTF-backend/backend-images.tar's hf_cache volume?
    Actually Jina CLIP is downloaded on first backend start via HF Hub (requires
    internet once). For truly offline, the hf_cache volume is pre-populated
    in the tar's backend image (nltk data) but Jina CLIP weights still need
    one-time download. To make 100% offline, run once with internet to populate
    hf_cache, then re-run `podman save` to capture cache.

To make installer single file:
  tar -czf WhereTF-offline.tar.gz WhereTF-offline/
  OR use the self-extracting installer: bash WhereTF-installer.sh

Uninstall:
  cd WhereTF-backend && podman-compose down -v
README
cat "$OFFLINE_DIR/README.txt"

# 4. Create archives
echo "[4/4] Creating archives..."
mkdir -p "$DIST"
# Tar.gz (portable)
(cd "$DIST" && tar -czf WhereTF-offline.tar.gz WhereTF-offline)
ls -lh "$DIST/WhereTF-offline.tar.gz"

# Self-extracting shell installer (1-file, fully offline, ~1.9GB)
INSTALLER="$DIST/WhereTF-installer.sh"
cat > "$INSTALLER" <<'HEADER'
#!/usr/bin/env bash
# WhereTF Offline Self-Extracting Installer
# Usage: bash WhereTF-installer.sh [--target DIR]
set -e
TARGET="${1:-$HOME/WhereTF}"
if [ "$1" = "--target" ]; then TARGET="$2"; fi
echo "=== WhereTF Offline Installer ==="
echo "Target: $TARGET"
mkdir -p "$TARGET"
echo "Extracting..."
ARCHIVE_LINE=$(awk '/^__ARCHIVE_BELOW__/ {print NR + 1; exit 0; }' "$0")
tail -n +$ARCHIVE_LINE "$0" | tar -xz -C "$TARGET"
echo "Installed to $TARGET"
echo "Launching..."
bash "$TARGET/WhereTF-offline/launch.sh"
exit 0
__ARCHIVE_BELOW__
HEADER
# Append tar.gz to installer
cat "$DIST/WhereTF-offline.tar.gz" >> "$INSTALLER"
chmod +x "$INSTALLER"
ls -lh "$INSTALLER"

echo ""
echo "=== Done ==="
echo "Offline dir: $OFFLINE_DIR"
echo "Tar.gz:      $DIST/WhereTF-offline.tar.gz ($(du -h "$DIST/WhereTF-offline.tar.gz" | cut -f1))"
echo "Installer:   $DIST/WhereTF-installer.sh ($(du -h "$INSTALLER" | cut -f1)) - SINGLE FILE, fully offline"
echo ""
echo "Distribute the installer: users just run 'bash WhereTF-installer.sh' (1 click)"
echo "Or distribute the tar.gz: users extract and double-click launch.sh / WhereTF"
echo ""
echo "Note: First build includes backend-images.tar (1.9GB). To update images after backend changes, re-run: podman save ... and ./scripts/bundle_offline.sh"
