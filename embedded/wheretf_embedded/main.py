"""FastAPI app for the embedded WhereTF backend (SQLite + sqlite-vec, no Redis/Celery)."""
from __future__ import annotations

import logging
import os
import uuid
from contextlib import asynccontextmanager
from pathlib import Path

from fastapi import FastAPI, File, Form, HTTPException, Query, Request, UploadFile
from fastapi.responses import JSONResponse, StreamingResponse

from . import indexer, storage

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("wheretf.embedded")


@asynccontextmanager
async def lifespan(app: FastAPI):
    storage.init_db()
    indexer.start()
    # Pre-load the embedding model in the BACKGROUND so /health responds
    # immediately (the UI must not wait on a model download at boot).
    if os.getenv("WHERETF_SKIP_PRELOAD"):
        logger.info("[boot] preload skipped (WHERETF_SKIP_PRELOAD set)")
    else:
        import threading

        def _preload() -> None:
            try:
                from app.config import AppConfig
                from app.processing.cache import ModelCache

                logger.info("[boot] tier=%s model=%s (loading in background)", AppConfig.TIER, AppConfig.MODEL_NAME)
                ModelCache.get_encoder()
                if AppConfig.ENABLE_OCR:
                    ModelCache.get_ocr_reader()
                logger.info("[boot] models loaded")
            except Exception:
                logger.exception("[boot] model preload failed (will lazy-load)")

        threading.Thread(target=_preload, name="wheretf-preload", daemon=True).start()
    yield
    logger.info("[shutdown] bye")


app = FastAPI(title="WhereTF Backend (embedded)", lifespan=lifespan)


# --- helpers ----------------------------------------------------------------

def _embed_query(text: str) -> list[float]:
    from app.processing.cache import ModelCache

    model = ModelCache.get_encoder()
    vec = model.encode([text])
    return [float(x) for x in vec[0]]


def _run_search(query: str, mode: str, top_k: int, expanded: str | None = None) -> list[dict]:
    q = expanded or query
    if mode == "keyword":
        return storage.keyword_search(q, top_k)
    vec = _embed_query(q)
    if mode == "vector":
        return storage.vector_search(vec, top_k)
    return storage.hybrid_search(vec, q, top_k)


# --- system -----------------------------------------------------------------

@app.get("/health")
def health():
    return {"status": "healthy", "service": "WhereTF Backend", "database_connected": True}


# --- search -----------------------------------------------------------------

@app.post("/search/normal/")
def normal_search(
    query: str = Query(...),
    mode: str = Query("hybrid"),
    top_k: int = Query(5),
):
    if mode not in ("vector", "keyword", "hybrid"):
        raise HTTPException(status_code=400, detail="Invalid mode")
    try:
        results = _run_search(query, mode, top_k)
    except Exception as exc:  # noqa: BLE001
        raise HTTPException(status_code=500, detail=f"Normal Search Error: {exc}")
    return {"status": "success", "search_type": "normal", "query": query, "mode": mode, "results": results}


@app.post("/search/power/")
def power_search(
    query: str = Query(...),
    mode: str = Query("hybrid"),
    top_k: int = Query(5),
):
    if mode not in ("vector", "keyword", "hybrid"):
        raise HTTPException(status_code=400, detail="Invalid mode")
    expanded = query
    try:
        from app.processing.expansion import generate_hypothetical_document

        expanded = generate_hypothetical_document(query) or query
    except Exception:
        logger.exception("expansion failed; falling back to raw query")
    try:
        results = _run_search(query, mode, top_k, expanded=expanded)
    except Exception as exc:  # noqa: BLE001
        raise HTTPException(status_code=500, detail=f"Power Search Error: {exc}")
    return {
        "status": "success",
        "search_type": "power",
        "query": query,
        "expanded_query": expanded,
        "mode": mode,
        "results": results,
    }


# --- upload / status --------------------------------------------------------

@app.post("/upload/")
async def upload_file(file: UploadFile = File(...), original_path: str = Form(...)):
    safe = f"{uuid.uuid4().hex}_{os.path.basename(file.filename or 'unnamed')}"
    temp_path = storage.temp_dir() / safe
    try:
        with open(temp_path, "wb") as buf:
            buf.write(await file.read())
        # Register the file row immediately (state=pending) for dashboards.
        storage.upsert_file_row(original_path, "pending", "unknown", storage.now_iso(), state="pending")
        task_id = indexer.enqueue(str(temp_path), original_path)
    except Exception as exc:  # noqa: BLE001
        raise HTTPException(status_code=500, detail=f"Upload failed: {exc}")
    return {"status": "success", "message": "File queued for indexing", "filename": safe, "task_id": task_id}


@app.get("/status/{task_id}")
def get_task_status(task_id: str):
    return indexer.task_status(task_id)


# --- files ------------------------------------------------------------------

@app.get("/files/")
def get_all_files():
    data = storage.get_all_files()
    return {"status": "success", "count": len(data), "data": data}


@app.delete("/files/{file_id}")
def delete_file(file_id: str):
    if not storage.get_file_by_id(file_id):
        raise HTTPException(status_code=404, detail="File not found")
    storage.delete_by_id(file_id)
    return {"status": "success", "message": "File and vectors wiped successfully."}


@app.patch("/files/{file_id}")
def update_file_metadata(file_id: str, payload: dict):
    if not storage.update_metadata(file_id, payload.get("tags"), payload.get("context")):
        raise HTTPException(status_code=404, detail="File not found")
    return {"status": "success", "message": "Metadata updated", "file_id": file_id}


@app.post("/files/delete-by-path")
def delete_file_by_path(req: dict):
    if not storage.delete_file_by_path(req.get("file_path", "")):
        raise HTTPException(status_code=404, detail="File not found")
    return {"status": "success", "message": "File deleted successfully."}


@app.post("/files/rename")
def rename_file(req: dict):
    if not storage.rename_file(req.get("old_path", ""), req.get("new_path", "")):
        raise HTTPException(status_code=404, detail="File not found")
    return {"status": "success", "message": "File renamed successfully."}


@app.post("/files/needs-indexing")
def needs_indexing(req: dict):
    return {"needs_indexing": storage.needs_indexing(req.get("file_path", ""), req.get("file_hash", ""))}


@app.get("/files/{file_id}/related")
def get_related_files(file_id: str):
    if not storage.get_file_by_id(file_id):
        raise HTTPException(status_code=404, detail="File not found")
    return {"status": "success", "file_id": file_id, "related_files": []}


# --- watch ------------------------------------------------------------------

@app.post("/watch/folder")
def add_watch_folder(req: dict):
    ok, fid = storage.add_watch_folder(req.get("folder_path", ""))
    if not ok:
        raise HTTPException(status_code=409, detail="Folder is already being watched.")
    return {"success": True, "id": fid, "folder_path": req.get("folder_path", "")}


@app.delete("/watch/folder")
def remove_watch_folder(req: dict):
    if not storage.remove_watch_folder(req.get("folder_path", "")):
        raise HTTPException(status_code=404, detail="Folder not found.")
    return {"success": True}


@app.get("/watch/folders")
def get_watch_folders():
    return storage.get_watch_folders()


def main() -> None:
    import uvicorn

    port = int(os.getenv("WHERETF_PORT", "8000"))
    uvicorn.run(app, host="127.0.0.1", port=port, log_level="info")


if __name__ == "__main__":
    main()
