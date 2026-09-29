#!/usr/bin/env bash
set -euo pipefail

# Wrap the embedded binary into a macOS .app bundle and a .dmg.
# Must run on macOS (uses sips/iconutil/hdiutil).
#
# Usage: make_macos_app.sh <binary> <outdir>
#   <binary>  e.g. dist/WhereTF-macos
#   <outdir>  e.g. dist
#
# Codesigning: set MACOS_SIGN_IDENTITY (e.g. "Developer ID Application: …") to
# sign; notarization is done separately (notarytool) if credentials are present.

BIN="${1:-dist/WhereTF-macos}"
OUTDIR="${2:-dist}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP="$OUTDIR/WhereTF.app"
ICON_SRC="$ROOT/assets/icon.png"

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/WhereTF"
chmod +x "$APP/Contents/MacOS/WhereTF"

# --- icon -> .icns ---
if [ -f "$ICON_SRC" ] && command -v iconutil >/dev/null 2>&1; then
  ICONSET="$OUTDIR/AppIcon.iconset"
  rm -rf "$ICONSET"; mkdir -p "$ICONSET"
  gen() { sips -z "$1" "$1" "$ICON_SRC" --out "$2" >/dev/null 2>&1; }
  gen 16   "$ICONSET/icon_16x16.png"
  gen 32   "$ICONSET/icon_16x16@2x.png"
  gen 32   "$ICONSET/icon_32x32.png"
  gen 64   "$ICONSET/icon_32x32@2x.png"
  gen 128  "$ICONSET/icon_128x128.png"
  gen 256  "$ICONSET/icon_128x128@2x.png"
  gen 256  "$ICONSET/icon_256x256.png"
  gen 512  "$ICONSET/icon_256x256@2x.png"
  gen 512  "$ICONSET/icon_512x512.png"
  gen 1024 "$ICONSET/icon_512x512@2x.png"
  iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/AppIcon.icns" || true
  rm -rf "$ICONSET"
fi

# --- Info.plist ---
cat > "$APP/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>WhereTF</string>
    <key>CFBundleDisplayName</key><string>WhereTF</string>
    <key>CFBundleIdentifier</key><string>com.wheretf.app</string>
    <key>CFBundleVersion</key><string>0.1.0</string>
    <key>CFBundleShortVersionString</key><string>0.1.0</string>
    <key>CFBundleExecutable</key><string>WhereTF</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleIconFile</key><string>AppIcon</string>
    <key>LSMinimumSystemVersion</key><string>11.0</string>
    <key>NSHighResolutionCapable</key><true/>
    <key>LSApplicationCategoryType</key><string>public.app-category.utilities</string>
</dict>
</plist>
PLIST

# --- codesign (optional) ---
if [ -n "${MACOS_SIGN_IDENTITY:-}" ]; then
  echo "[macapp] codesigning with $MACOS_SIGN_IDENTITY"
  codesign --force --deep --options runtime --timestamp \
    --sign "$MACOS_SIGN_IDENTITY" "$APP"
fi

# --- .dmg ---
mkdir -p "$OUTDIR"
rm -f "$OUTDIR/WhereTF.dmg"
hdiutil create -volname "WhereTF" -srcfolder "$APP" -ov -format UDZO "$OUTDIR/WhereTF.dmg" >/dev/null

# --- notarize (optional) ---
if [ -n "${APPLE_ID:-}" ] && [ -n "${APPLE_TEAM_ID:-}" ] && [ -n "${APPLE_APP_PASSWORD:-}" ]; then
  echo "[macapp] notarizing WhereTF.dmg"
  xcrun notarytool submit "$OUTDIR/WhereTF.dmg" \
    --apple-id "$APPLE_ID" --team-id "$APPLE_TEAM_ID" --password "$APPLE_APP_PASSWORD" \
    --wait || true
  xcrun stapler staple "$OUTDIR/WhereTF.dmg" || true
fi

echo "[macapp] $APP and $OUTDIR/WhereTF.dmg"
