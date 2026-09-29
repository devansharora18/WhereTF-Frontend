<#
WhereTF Windows installer.
Distributed inside WhereTF-windows.zip alongside: WhereTF.exe, WhereTF-backend\ (incl. backend-images.tar),
and the bundled docker-compose.yml.

Usage (from an extracted WhereTF-windows folder):
  powershell -ExecutionPolicy Bypass -File .\install.ps1
  powershell -ExecutionPolicy Bypass -File .\install.ps1 -Tier lite
#>
param(
  [string]$Target = "$env:USERPROFILE\WhereTF",
  [ValidateSet('lite','balanced','pro')] [string]$Tier
)

$ErrorActionPreference = "Stop"
$here = Split-Path -Parent $MyInvocation.MyCommand.Path

Write-Host "=== WhereTF Windows Installer ==="

if (-not $Tier) {
  Write-Host ""
  Write-Host "Choose a backend tier:"
  Write-Host "  1) lite      - all-MiniLM-L6-v2, 384-dim, OCR       (~700 MB RAM)"
  Write-Host "  2) balanced  - nomic-embed-vision, 768-dim, vision  (~1.2 GB RAM, no OCR)"
  Write-Host "  3) pro       - jina-clip-v1, 768-dim, OCR + vision  (~2.0 GB RAM)"
  $choice = Read-Host "Enter 1/2/3 (default 3=pro)"
  switch ($choice) {
    "1" { $Tier = "lite" }
    "2" { $Tier = "balanced" }
    default { $Tier = "pro" }
  }
}
Write-Host "Tier: $Tier"
Write-Host "Target: $Target"

# 1. Check for a container runtime CLI (podman preferred, docker fallback).
$compose = $null
if (Get-Command podman -ErrorAction SilentlyContinue) {
  $compose = "podman"
} elseif (Get-Command docker -ErrorAction SilentlyContinue) {
  $compose = "docker"
} else {
  Write-Host "No container runtime found (podman or docker)." -ForegroundColor Yellow
  if (Get-Command winget -ErrorAction SilentlyContinue) {
    $ans = Read-Host "Install Podman via winget now? [y/N]"
    if ($ans -match '^(y|Y)') {
      winget install -e --id RedHat.Podman
      Write-Host "After install, open 'Podman Desktop' once so its VM initialises, then re-run."
    }
  }
  if (Get-Command podman -ErrorAction SilentlyContinue) { $compose = "podman" }
  elseif (Get-Command docker -ErrorAction SilentlyContinue) { $compose = "docker" }
  else {
    Write-Host "Please install Podman Desktop or Docker Desktop, then re-run install.ps1." -ForegroundColor Red
    exit 1
  }
}
Write-Host "Using container runtime: $compose"

# Initialise the podman VM if needed (Windows/macOS run containers in a VM).
if ($compose -eq "podman") {
  try { & podman info *> $null; if ($LASTEXITCODE -ne 0) { throw } }
  catch {
    Write-Host "Initialising the podman VM (one-time image download)..."
    & podman machine init 2>$null
    & podman machine start 2>$null
  }
}

# 2. Copy the app to the target directory.
New-Item -ItemType Directory -Force -Path $Target | Out-Null
Copy-Item -Recurse -Force "$here\*" $Target

$backend = Join-Path $Target "WhereTF-backend"
$composeFile = Join-Path $backend "docker-compose.yml"

# 3. Pin the chosen tier for backend + worker.
if (Test-Path $composeFile) {
  (Get-Content $composeFile) -replace '^(\s*)APP_TIER: .*', "`$1APP_TIER: $Tier" |
    Set-Content $composeFile
  Write-Host "Configured APP_TIER=$Tier"
}

# 4. Load the offline images if they are not present yet.
$hasImages = (& $compose images --format "{{.Repository}}:{{.Tag}}") -match "wheretf-backend"
if (-not $hasImages) {
  $tar = Join-Path $backend "backend-images.tar"
  if (Test-Path $tar) {
    Write-Host "Loading offline images (1.9 GB, ~1 min)..."
    & $compose load -i $tar
  } else {
    Write-Host "No bundled image tar; will pull from the internet on first run."
  }
}

# 5. Start the backend stack.
Write-Host "Starting backend (db + backend + redis + worker)..."
Push-Location $backend
if ($compose -eq "podman") { & podman compose up -d } else { & docker compose up -d }
Pop-Location

# 6. Launch the app.
$exe = Join-Path $Target "WhereTF.exe"
if (Test-Path $exe) {
  Write-Host "Launching WhereTF..."
  Start-Process $exe
} else {
  Write-Host "WhereTF.exe not found at $exe" -ForegroundColor Yellow
}
