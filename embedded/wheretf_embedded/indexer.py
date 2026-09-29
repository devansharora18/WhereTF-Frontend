"""In-process background indexer (replaces Redis + Celery).

A single worker thread pulls jobs from a queue, runs the existing
`app.processing.pipeline.process` (extract -> embed), and writes results to SQLite.
"""
from __future__ import annotations

import logging
import os
import queue
import threading
import uuid
from pathlib import Path

from . import storage

logger = logging.getLogger(__name__)

_jobs: "queue.Queue[tuple[str, str, str]]" = queue.Queue()
_tasks: dict[str, dict] = {}
_lock = threading.Lock()
_started = False


def task_status(task_id: str) -> dict:
    return _tasks.get(task_id, {"task_id": task_id, "status": "PENDING", "result": None})


def _set_task(task_id: str, status: str, result=None) -> None:
    with _lock:
        _tasks[task_id] = {"task_id": task_id, "status": status, "result": result}


def enqueue(temp_path: str, original_path: str) -> str:
    task_id = uuid.uuid4().hex
    _set_task(task_id, "PENDING")
    _jobs.put((task_id, temp_path, original_path))
    return task_id


def _process(task_id: str, temp_path: str, original_path: str) -> None:
    from app.processing.pipeline import process  # reused, DB-agnostic

    _set_task(task_id, "PROCESSING")
    storage.upsert_file_row(
        original_path, "pending", "unknown", storage.now_iso(), state="processing"
    )
    try:
        file_rows, content_rows = process(temp_path)
        # pipeline used the temp path; rewrite to the real original path.
        file_hash = file_rows[0]["file_hash"] if file_rows else "unknown"
        mime_type = file_rows[0]["mime_type"] if file_rows else "unknown"
        last_modified = file_rows[0]["last_modified"] if file_rows else storage.now_iso()

        file_id = storage.upsert_file_row(
            original_path, file_hash, mime_type, last_modified, state="processing"
        )
        chunks = [
            {
                "chunk_index": cr["chunk_index"],
                "content_text": cr["content_text"],
                "embedding": cr["embedding"],
            }
            for cr in content_rows
        ]
        storage.replace_chunks(file_id, chunks)
        storage.set_state(original_path, "indexed")
        _set_task(task_id, "SUCCESS", {"file_path": original_path, "chunks": len(chunks)})
        logger.info("[indexer] indexed %s (%d chunks)", original_path, len(chunks))
    except Exception as exc:  # noqa: BLE001
        logger.exception("[indexer] failed for %s", original_path)
        storage.set_state(original_path, "failed")
        _set_task(task_id, "FAILURE", {"error": str(exc)})
    finally:
        try:
            if os.path.exists(temp_path):
                os.remove(temp_path)
        except OSError:
            pass


def _worker() -> None:
    while True:
        task_id, temp_path, original_path = _jobs.get()
        try:
            _process(task_id, temp_path, original_path)
        finally:
            _jobs.task_done()


def start() -> None:
    global _started
    if _started:
        return
    _started = True
    threading.Thread(target=_worker, name="wheretf-indexer", daemon=True).start()
    logger.info("[indexer] worker started (in-process, no Redis/Celery)")


def index_existing(skip_hashes: set[str] | None = None) -> None:
    """Optional one-shot folder scan (not used by the watcher flow)."""
    # Kept intentionally minimal; the Rust watcher drives uploads instead.
    return
