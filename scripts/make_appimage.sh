#!/usr/bin/env bash
set -euo pipefail

# Wrap the embedded Linux binary into a single-file AppImage.
#
# Usage: make_appimage.sh <binary> <output.AppImage>
#
# Requires (or downloads) appimagetool. Set APPIMAGETOOL to a preinstalled binary.

BIN="${1:-dist/WhereTF-linux}"
OUT="${2:-dist/WhereTF.AppImage}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ICON="$ROOT/assets/icon.png"
APPDIR="$ROOT/dist/WhereTF.AppDir"

rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin" \
         "$APPDIR/usr/share/icons/hicolor/256x256/apps" \
         "$APPDIR/usr/share/applications"

cp "$BIN" "$APPDIR/usr/bin/WhereTF"
chmod +x "$APPDIR/usr/bin/WhereTF"

if [ -f "$ICON" ]; then
  cp "$ICON" "$APPDIR/wheretf.png"
  cp "$ICON" "$APPDIR/usr/share/icons/hicolor/256x256/apps/wheretf.png"
fi

cat > "$APPDIR/wheretf.desktop" <<'DESK'
[Desktop Entry]
Type=Application
Name=WhereTF
Comment=AI-powered file search
Exec=WhereTF
Icon=wheretf
Categories=Utility;
Terminal=false
DESK
# AppImage wants the .desktop at the root too
cp "$APPDIR/wheretf.desktop" "$APPDIR/usr/share/applications/wheretf.desktop"

cat > "$APPDIR/AppRun" <<'RUN'
#!/usr/bin/env bash
HERE="$(dirname "$(readlink -f "$0")")"
exec "$HERE/usr/bin/WhereTF" "$@"
RUN
chmod +x "$APPDIR/AppRun"

# --- get appimagetool ---
TOOL="${APPIMAGETOOL:-}"
if [ -z "$TOOL" ] || [ ! -x "$TOOL" ]; then
  TOOL="$ROOT/dist/appimagetool"
  if [ ! -x "$TOOL" ]; then
    echo "[appimage] downloading appimagetool..."
    curl -fL -o "$TOOL" \
      https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage
    chmod +x "$TOOL"
  fi
fi

export ARCH="${ARCH:-x86_64}"
echo "[appimage] building $OUT"
# appimagetool is itself an AppImage; extract-and-run avoids needing FUSE.
"$TOOL" --appimage-extract-and-run "$APPDIR" "$OUT" 2>/dev/null \
  || "$TOOL" "$APPDIR" "$OUT"

echo "[appimage] $OUT"
ls -lh "$OUT"
