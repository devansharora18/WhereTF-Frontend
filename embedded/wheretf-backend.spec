# PyInstaller spec for the WhereTF embedded backend sidecar.
# Lives in the frontend repo: <repo>/embedded/. Build with:
#   pyinstaller wheretf-backend.spec --noconfirm   (run from embedded/)
# See build_sidecar.sh.

import sys
from pathlib import Path
from PyInstaller.utils.hooks import collect_all, collect_submodules

EMBEDDED = Path(SPECPATH)          # <repo>/embedded
ENGINE = EMBEDDED / "app"          # vendored backend engine (config + processing)

datas = []
binaries = []
hiddenimports = []

# Heavy ML / doc libs: pull in everything they need.
for pkg in [
    "sentence_transformers",
    "transformers",
    "torch",
    "torchvision",
    "easyocr",
    "timm",
    "nltk",
    "sqlite_vec",
    "sklearn",
    "scipy",
    "cv2",
    "PIL",
    "pdfplumber",
    "pdfminer",
    "docx",
    "pptx",
    "openpyxl",
    "uvicorn",
    "fastapi",
    "pydantic",
]:
    try:
        d, b, h = collect_all(pkg)
        datas += d
        binaries += b
        hiddenimports += h
    except Exception:
        pass

# Our packages.
hiddenimports += collect_submodules("wheretf_embedded")
hiddenimports += collect_submodules("app")

# Bundle the vendored engine so `app.processing` is importable.
datas += [(str(ENGINE), "app")]

# Bundle NLTK data (punkt, wordnet, omw-1.4) so the frozen app never downloads
# at runtime (which fails offline / behind proxies). The Dockerfile bakes this
# into the image at /root/nltk_data.
import os
_nltk_src = os.getenv("NLTK_DATA", "/root/nltk_data")
if os.path.isdir(_nltk_src):
    for _root, _dirs, _files in os.walk(_nltk_src):
        for _fn in _files:
            _full = os.path.join(_root, _fn)
            _rel = os.path.relpath(_full, _nltk_src)
            datas.append((_full, os.path.join("nltk_data", os.path.dirname(_rel))))

a = Analysis(
    [str(EMBEDDED / "run_backend.py")],
    pathex=[str(EMBEDDED)],
    binaries=binaries,
    datas=datas,
    hiddenimports=hiddenimports,
    hookspath=[],
    runtime_hooks=[],
    excludes=["tkinter", "matplotlib", "PyQt5", "PySide6"],
    noarchive=False,
)
pyz = PYZ(a.pure)

exe = EXE(
    pyz,
    a.scripts,
    a.binaries,
    a.datas,
    [],
    name="wheretf-backend",
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=False,
    runtime_tmpdir=None,
    console=True,
    onefile=True,
)
