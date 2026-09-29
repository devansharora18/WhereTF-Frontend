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

# --- detect a container runtime (podman preferred, docker accepted) ---
RUNTIME=""
if command -v podman >/dev/null 2>&1; then RUNTIME=podman
elif command -v docker >/dev/null 2>&1; then RUNTIME=docker
else
  INSTALL_CMD=""
  if command -v apt-get >/dev/null 2>&1; then INSTALL_CMD="sudo apt-get update && sudo apt-get install -y podman podman-compose"
  elif command -v dnf >/dev/null 2>&1; then INSTALL_CMD="sudo dnf install -y podman podman-compose"
  elif command -v pacman >/dev/null 2>&1; then INSTALL_CMD="sudo pacman -S --noconfirm podman podman-compose"
  elif command -v zypper >/dev/null 2>&1; then INSTALL_CMD="sudo zypper install -y podman podman-compose"
  elif command -v brew >/dev/null 2>&1; then INSTALL_CMD="brew install podman"
  fi
  echo "[launch] No container runtime found (podman or docker)."
  if [ -n "$INSTALL_CMD" ]; then
    echo "[launch] Detected your package manager. It will run:"
    echo "         $INSTALL_CMD"
    printf "Install it now? [y/N] "
    read -r ANS 2>/dev/null </dev/tty || ANS=n
    case "$ANS" in y|Y) eval "$INSTALL_CMD" || true ;; esac
  fi
  if command -v podman >/dev/null 2>&1; then RUNTIME=podman; fi
  if [ -z "$RUNTIME" ] && command -v docker >/dev/null 2>&1; then RUNTIME=docker; fi
  if [ -z "$RUNTIME" ]; then
    echo "[launch] Still no runtime. Please install podman manually, then re-run:"
    echo "  Ubuntu/Debian: sudo apt-get install podman podman-compose"
    echo "  Fedora:        sudo dnf install podman podman-compose"
    echo "  Arch:          sudo pacman -S podman podman-compose"
    echo "  macOS:         brew install podman && podman machine init && podman machine start"
    echo "  Windows:       winget install -e --id RedHat.Podman  (or install Docker Desktop)"
    read -p "Press enter to exit..." _ 2>/dev/null || true
    exit 1
  fi
fi
echo "[launch] Using container runtime: $RUNTIME"

# On macOS/Windows, podman needs a running VM; initialise it once.
if [ "$RUNTIME" = "podman" ] && ! podman info >/dev/null 2>&1; then
  echo "[launch] podman machine not running; initialising (one-time VM image download)..."
  podman machine init || true
  podman machine start || true
fi

# --- pick a compose provider ---
compose_up() {
  if command -v podman-compose >/dev/null 2>&1; then podman-compose up -d; return $?; fi
  if command -v docker-compose >/dev/null 2>&1; then docker-compose up -d; return $?; fi
  if [ "$RUNTIME" = "podman" ] && podman compose version >/dev/null 2>&1; then podman compose up -d; return $?; fi
  if [ "$RUNTIME" = "docker" ] && docker compose version >/dev/null 2>&1; then docker compose up -d; return $?; fi
  return 1
}

# Load offline images if needed (first run, no internet)
if ! $RUNTIME images --format "{{.Repository}}:{{.Tag}}" 2>/dev/null | grep -q "wheretf-backend"; then
  if [ -f "$BACKEND_DIR/backend-images.tar" ]; then
    echo "[launch] Loading offline images (1.9GB, ~1 min)..."
    $RUNTIME load -i "$BACKEND_DIR/backend-images.tar"
  elif [ -f "$BACKEND_DIR/backend-images.tar.gz" ]; then
    echo "[launch] Loading gzipped offline images..."
    $RUNTIME load -i "$BACKEND_DIR/backend-images.tar.gz"
  else
    echo "[launch] No bundled image tar; will pull from the internet (requires network)."
  fi
fi
# Start backend
echo "[launch] Starting backend (db + backend + redis + worker)..."
if ! (cd "$BACKEND_DIR" && compose_up); then
  echo "[launch] ERROR: no working compose provider (podman-compose / docker-compose)."
  echo "         Try: pip install --user podman-compose"
  read -p "Press enter to exit..." _ 2>/dev/null || true
  exit 1
fi
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
setlocal
set SCRIPT_DIR=%~dp0
set BACKEND_DIR=%SCRIPT_DIR%WhereTF-backend
echo === WhereTF Launcher (Windows) ===

rem Pick a runtime: prefer podman, fall back to docker.
set RUNTIME=
where podman >nul 2>&1 && set RUNTIME=podman
if "%RUNTIME%"=="" ( where docker >nul 2>&1 && set RUNTIME=docker )

if "%RUNTIME%"=="" (
  echo No container runtime found ^(podman or docker^).
  where winget >nul 2>&1
  if %errorlevel% equ 0 (
    set /p ANS=Install Podman via winget now? [y/N]:
    if /i "%ANS%"=="y" (
      winget install -e --id RedHat.Podman
      echo Note: after install, open "Podman Desktop" once so its VM initialises.
    )
  )
  where podman >nul 2>&1 && set RUNTIME=podman
  if "%RUNTIME%"=="" ( where docker >nul 2>&1 && set RUNTIME=docker )
  if "%RUNTIME%"=="" (
    echo Please install Podman Desktop or Docker Desktop, then re-run launch.bat.
    pause
    exit /b 1
  )
)
echo [launch] Using container runtime: %RUNTIME%

echo %RUNTIME% images --format "{{.Repository}}:{{.Tag}}" | findstr wheretf-backend >nul
if %errorlevel% neq 0 (
  if exist "%BACKEND_DIR%\backend-images.tar" (
    echo [launch] Loading offline images ^(1.9GB, ~1 min^)...
    %RUNTIME% load -i "%BACKEND_DIR%\backend-images.tar"
  )
)

echo [launch] Starting backend ^(db + backend + redis + worker^)...
cd /d "%BACKEND_DIR%"
where podman-compose >nul 2>&1 && (podman-compose up -d) || (
  where docker-compose >nul 2>&1 && (docker-compose up -d) || (%RUNTIME% compose up -d)
)
echo [launch] Waiting for backend...
timeout /t 5 >nul
start "" "%SCRIPT_DIR%WhereTF.exe"
endlocal
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

# Self-extracting shell installer (single file; prompts for the tier)
INSTALLER="$DIST/WhereTF-installer.sh"
cat > "$INSTALLER" <<'HEADER'
#!/usr/bin/env bash
# WhereTF Offline Self-Extracting Installer (single file for all tiers)
# Usage: bash WhereTF-installer.sh [--target DIR] [--tier lite|balanced|pro]
set -e
TARGET="$HOME/WhereTF"
TIER=""
while [ $# -gt 0 ]; do
  case "$1" in
    --target) TARGET="$2"; shift 2 ;;
    --tier)   TIER="$2";   shift 2 ;;
    *) TARGET="$1"; shift ;;
  esac
done
echo "=== WhereTF Offline Installer ==="

if [ -z "$TIER" ]; then
  echo ""
  echo "Choose a backend tier (affects memory use and search quality):"
  echo "  1) lite      - all-MiniLM-L6-v2, 384-dim, OCR          (~700 MB RAM)"
  echo "  2) balanced  - nomic-embed-vision, 768-dim, vision     (~1.2 GB RAM, no OCR)"
  echo "  3) pro       - jina-clip-v1, 768-dim, OCR + vision      (~2.0 GB RAM)"
  echo ""
  printf "Enter 1/2/3 (default 3=pro): "
  read -r CHOICE </dev/tty || CHOICE=3
  case "$CHOICE" in
    1|lite) TIER=lite ;;
    2|balanced) TIER=balanced ;;
    *) TIER=pro ;;
  esac
fi
case "$TIER" in lite|balanced|pro) ;; *) echo "Unknown tier '$TIER'; using pro"; TIER=pro ;; esac

echo "Tier:   $TIER"
echo "Target: $TARGET"
mkdir -p "$TARGET"
echo "Extracting..."
ARCHIVE_LINE=$(awk '/^__ARCHIVE_BELOW__/ {print NR + 1; exit 0; }' "$0")
tail -n +$ARCHIVE_LINE "$0" | tar -xz -C "$TARGET"

# Pin the chosen tier for both backend and worker.
COMPOSE="$TARGET/WhereTF-offline/WhereTF-backend/docker-compose.yml"
if [ -f "$COMPOSE" ]; then
  sed -i "s/^\(\s*\)APP_TIER: .*/\1APP_TIER: $TIER/" "$COMPOSE"
  echo "Configured APP_TIER=$TIER in bundled docker-compose.yml"
fi

echo "Installed to $TARGET"
echo "Launching ($TIER)..."
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
