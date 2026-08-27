#!/usr/bin/env bash
set -euo pipefail
# WhereTF - Service Bundle (Backend Only, Docker)
# Packages the backend service (4 containers) for standalone `podman-compose up -d`
# This is the "service" half of the split. The frontend app is separate.

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BACKEND_DIR="$ROOT/WhereTF-backend"
DIST="$ROOT/dist"

echo "=== WhereTF Service Bundle (Backend Only) ==="
echo "Backend dir: $BACKEND_DIR"

if [ ! -f "$BACKEND_DIR/docker-compose.yml" ]; then
  echo "ERROR: $BACKEND_DIR/docker-compose.yml not found"
  exit 1
fi

# Ensure images exist, build if needed
if ! podman images --format "{{.Repository}}:{{.Tag}}" | grep -q "wheretf-backend"; then
  echo "[1/3] Backend image not found, building (CPU, ~5min)..."
  (cd "$BACKEND_DIR" && podman build -t localhost/wheretf-backend-backend:latest .)
  podman tag localhost/wheretf-backend-backend:latest localhost/wheretf-backend_backend:latest
  podman tag localhost/wheretf-backend-backend:latest localhost/wheretf-backend-worker:latest
  podman tag localhost/wheretf-backend-backend:latest localhost/wheretf-backend_worker:latest
fi

# Create service tar (backend source + compose, no frontend)
SERVICE_DIR="$DIST/WhereTF-service"
echo "[2/3] Assembling $SERVICE_DIR..."
rm -rf "$SERVICE_DIR"
mkdir -p "$SERVICE_DIR"
# Copy backend
if command -v rsync >/dev/null; then
  rsync -a --exclude='.git' --exclude='__pycache__' --exclude='*.pyc' --exclude='temp' --exclude='backend-images.tar' --exclude='backend-images.tar.gz' "$BACKEND_DIR/" "$SERVICE_DIR/"
else
  mkdir -p "$SERVICE_DIR"
  cp -a "$BACKEND_DIR"/* "$SERVICE_DIR/" 2>/dev/null || true
  rm -rf "$SERVICE_DIR/.git" "$SERVICE_DIR/__pycache__" 2>/dev/null || true
fi

cat > "$SERVICE_DIR/README.txt" <<'README'
WhereTF - Service (Backend Only)
================================
This is the backend service for WhereTF. Run it separately from the frontend app.

Services (via docker-compose.yml):
  - db:       pgvector/pgvector:pg17  (5433->5432)
  - backend:  FastAPI + Jina CLIP 768 + NLTK (8000->8000)
  - redis:    redis:7-alpine          (6379->6379)
  - worker:   Celery worker (background indexing)

Quick Start (requires podman + podman-compose or docker):
  podman-compose up -d
  # or
  podman compose up -d
  # or
  docker-compose up -d

Check:
  curl http://localhost:8000/health          # should be {"status":"healthy",...}
  curl http://localhost:8000/files/          # list indexed files
  podman ps                                  # 4 containers Up
  podman logs wheretf-backend -f
  podman logs wheretf-worker -f

Stop:
  podman-compose down          # keep data
  podman-compose down -v       # also delete DB volume

For offline (no internet):
  # On a machine with internet, save images:
  podman save -o backend-images.tar localhost/wheretf-backend-backend:latest localhost/wheretf-backend_backend:latest localhost/wheretf-backend-worker:latest localhost/wheretf-backend_worker:latest docker.io/pgvector/pgvector:pg17 docker.io/library/redis:7-alpine
  # Copy backend-images.tar next to docker-compose.yml, then on offline machine:
  podman load -i backend-images.tar
  podman-compose up -d

Frontend App (separate):
  The frontend is a standalone binary in ../WhereTF-app/WhereTF
  It expects this service to be running at http://localhost:8000
  Run: ./WhereTF-app/WhereTF
  If service not running, it shows "Backend not running" and retries.

For combined single-file offline, use:
  bash ../scripts/bundle_offline.sh  # creates dist/WhereTF-installer.sh (608M, fully offline)
README

# Create tar.gz
mkdir -p "$DIST"
(cd "$DIST" && tar -czf WhereTF-service.tar.gz WhereTF-service)
ls -lh "$DIST/WhereTF-service.tar.gz"

# Optionally create a lightweight offline images tar for service-only offline
read -p "Create offline images tar for service? (1.9GB, y/N) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
  echo "[3/3] Saving offline images tar..."
  podman save -o "$SERVICE_DIR/backend-images.tar" \
    localhost/wheretf-backend-backend:latest \
    localhost/wheretf-backend_backend:latest \
    localhost/wheretf-backend-worker:latest \
    localhost/wheretf-backend_worker:latest \
    docker.io/pgvector/pgvector:pg17 \
    docker.io/library/redis:7-alpine
  ls -lh "$SERVICE_DIR/backend-images.tar"
  (cd "$DIST" && tar -czf WhereTF-service-offline.tar.gz WhereTF-service)
  ls -lh "$DIST/WhereTF-service-offline.tar.gz"
  echo "Service offline bundle: $DIST/WhereTF-service-offline.tar.gz"
fi

echo ""
echo "Done: $DIST/WhereTF-service/"
echo "      $DIST/WhereTF-service.tar.gz"
echo ""
echo "To run the service:"
echo "  cd $SERVICE_DIR && podman-compose up -d"
echo "Then run the frontend app separately:"
echo "  ../WhereTF-app/WhereTF  or  bash ../scripts/bundle_app.sh"
