"""WhereTF embedded backend package.

Reuses the existing `app.processing` pipeline (extractors/embeddings) but stores
everything in local SQLite + sqlite-vec, with an in-process indexer instead of
Redis/Celery + Postgres. Intended to be frozen into a single executable.
"""
from __future__ import annotations

import os
import sys
from pathlib import Path


def _ensure_backend_on_path() -> None:
    """Make the vendored `app` engine importable (dev layout / PyInstaller bundle)."""
    here = Path(__file__).resolve()
    candidates = []
    env = os.getenv("WHERETF_BACKEND_DIR")
    if env:
        candidates.append(Path(env))
    # dev layout: <repo>/embedded/wheretf_embedded/__init__.py
    #   parents[0] = wheretf_embedded, parents[1] = embedded  (contains app/)
    if len(here.parents) >= 2:
        candidates.append(here.parents[1])
    if len(here.parents) >= 3:
        candidates.append(here.parents[2])
    candidates.append(Path.cwd())
    candidates.append(Path.cwd() / "embedded")
    for c in candidates:
        if (c / "app" / "processing").is_dir():
            if str(c) not in sys.path:
                sys.path.insert(0, str(c))
            return


_ensure_backend_on_path()

# Default tier for the embedded build (overridable at runtime).
os.environ.setdefault("APP_TIER", os.getenv("WHERETF_TIER", "pro"))

# When frozen by PyInstaller, bundled NLTK data lives next to the app; point
# NLTK at it so it never tries to download at runtime.
if getattr(sys, "frozen", False):
    meipass = getattr(sys, "_MEIPASS", None)
    if meipass:
        nltk_dir = os.path.join(meipass, "nltk_data")
        if os.path.isdir(nltk_dir):
            os.environ["NLTK_DATA"] = nltk_dir

# Neutralize nltk.download(): app/processing/expansion.py calls it at import
# time, which hits the network (and fails offline/behind proxies). The data is
# bundled, so make those calls no-ops.
try:
    import nltk as _nltk

    _nltk.download = lambda *a, **k: True  # type: ignore[assignment]
except Exception:
    pass


def _install_model_load_lock() -> None:
    """Serialize ModelCache loads.

    Concurrent loads (background preload thread + indexer worker + search
    request) can race and one of them fails with
    `TypeError: super(type, obj): obj must be an instance or subtype of type`
    for HF remote-code models. A lock makes the first load win and the rest
    reuse the cached instance.
    """
    import threading

    try:
        from app.processing.cache import ModelCache
    except Exception:
        return
    if getattr(ModelCache, "_embedded_lock_installed", False):
        return
    lock = threading.Lock()

    for name in ("get_encoder", "get_ocr_reader"):
        original = getattr(ModelCache, name)

        def _locked(_orig=original):
            with lock:
                return _orig()

        setattr(ModelCache, name, staticmethod(_locked))

    ModelCache._embedded_lock_installed = True  # type: ignore[attr-defined]


_install_model_load_lock()
