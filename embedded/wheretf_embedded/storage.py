"""Embedded storage layer: SQLite + sqlite-vec (+ FTS5) instead of Postgres/pgvector.

Keeps the same data shapes the FastAPI routes need, but everything lives in a
single local .db file so the backend can ship inside one executable.
"""
from __future__ import annotations

import json
import os
import sqlite3 as _stdlib_sqlite3
import struct
import time
import uuid
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable

# Prefer pysqlite3 when available: the macOS system sqlite3 is built without
# loadable-extension support, so sqlite-vec can't load there otherwise.
try:
    import pysqlite3 as sqlite3  # type: ignore
except Exception:
    sqlite3 = _stdlib_sqlite3  # type: ignore

# ---- paths -----------------------------------------------------------------

def data_dir() -> Path:
    base = os.getenv("WHERETF_DATA_DIR")
    if base:
        return Path(base)
    xdg = os.getenv("XDG_DATA_HOME")
    root = Path(xdg) if xdg else Path.home() / ".local" / "share"
    return root / "wheretf"


def db_path() -> Path:
    d = data_dir()
    d.mkdir(parents=True, exist_ok=True)
    return d / "wheretf.db"


def temp_dir() -> Path:
    d = data_dir() / "temp"
    d.mkdir(parents=True, exist_ok=True)
    return d


# ---- connection ------------------------------------------------------------

_conn: sqlite3.Connection | None = None


def connect() -> sqlite3.Connection:
    global _conn
    if _conn is not None:
        return _conn
    conn = sqlite3.connect(str(db_path()), check_same_thread=False)
    conn.row_factory = sqlite3.Row
    conn.execute("PRAGMA journal_mode=WAL")
    conn.execute("PRAGMA foreign_keys=ON")
    # Load the sqlite-vec extension for vector math.
    try:
        import sqlite_vec  # type: ignore

        if not hasattr(conn, "enable_load_extension"):
            raise RuntimeError(
                "this Python's sqlite3 was built without loadable-extension "
                "support (common on macOS); install 'pysqlite3-binary'"
            )
        conn.enable_load_extension(True)
        sqlite_vec.load(conn)
        conn.enable_load_extension(False)
    except Exception as exc:  # pragma: no cover
        raise RuntimeError(f"sqlite-vec failed to load: {exc}") from exc
    _conn = conn
    return conn


def init_db() -> None:
    conn = connect()
    conn.executescript(
        """
        CREATE TABLE IF NOT EXISTS meta (
            key   TEXT PRIMARY KEY,
            value TEXT
        );
        CREATE TABLE IF NOT EXISTS files (
            id            TEXT PRIMARY KEY,
            file_path     TEXT UNIQUE NOT NULL,
            file_hash     TEXT NOT NULL,
            mime_type     TEXT NOT NULL,
            last_modified TEXT,
            indexed_at    TEXT,
            tags          TEXT DEFAULT '[]',
            state         TEXT DEFAULT 'pending',
            context       TEXT
        );
        CREATE INDEX IF NOT EXISTS files_path_idx ON files(file_path);

        CREATE TABLE IF NOT EXISTS file_content (
            rowid         INTEGER PRIMARY KEY AUTOINCREMENT,
            file_id       TEXT NOT NULL REFERENCES files(id) ON DELETE CASCADE,
            chunk_index   INTEGER NOT NULL,
            content_text  TEXT NOT NULL,
            embedding     BLOB
        );
        CREATE INDEX IF NOT EXISTS content_file_idx ON file_content(file_id);

        CREATE VIRTUAL TABLE IF NOT EXISTS content_fts USING fts5(content_text);

        CREATE TABLE IF NOT EXISTS file_relationships (
            source_file_id TEXT NOT NULL,
            target_file_id TEXT NOT NULL,
            similarity_score REAL NOT NULL,
            relation_type TEXT DEFAULT 'semantic',
            UNIQUE(source_file_id, target_file_id)
        );

        CREATE TABLE IF NOT EXISTS watched_folders (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            folder_path TEXT UNIQUE NOT NULL,
            created_at TEXT
        );
        """
    )
    conn.commit()
    _ensure_vector_dim()


def _meta_get(key: str) -> str | None:
    row = connect().execute("SELECT value FROM meta WHERE key=?", (key,)).fetchone()
    return row["value"] if row else None


def _meta_set(key: str, value: str) -> None:
    connect().execute(
        "INSERT INTO meta(key,value) VALUES(?,?) "
        "ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        (key, value),
    )
    connect().commit()


def _ensure_vector_dim() -> None:
    """Keep the DB's embedding dimension consistent with the active tier.

    A tier change swaps the embedding model (384 vs 768 dims); vectors from a
    different model are meaningless and make cosine distance fail. If the stored
    dimension differs from the current tier, drop the stale vectors and mark the
    files as pending so they get re-indexed.
    """
    try:
        from app.config import AppConfig

        target = str(AppConfig.VECTOR_DIM)
    except Exception:
        return
    conn = connect()
    current = _meta_get("vector_dim")
    if current is None:
        _meta_set("vector_dim", target)
        return
    if current == target:
        return
    chunks = conn.execute("SELECT COUNT(*) AS n FROM file_content").fetchone()["n"]
    if chunks:
        conn.execute("DELETE FROM content_fts")
        conn.execute("DELETE FROM file_content")
        conn.execute("UPDATE files SET state='pending'")
        conn.commit()
        print(
            f"[storage] embedding dimension changed {current} -> {target}; "
            f"cleared {chunks} stale vectors, files marked pending for re-index"
        )
    _meta_set("vector_dim", target)


def serialize_vec(vec: Iterable[float]) -> bytes:
    return struct.pack(f"<{len(vec)}f", *[float(x) for x in vec])


def now_iso() -> str:
    return datetime.now(timezone.utc).isoformat()


# ---- files -----------------------------------------------------------------

def upsert_file_row(file_path: str, file_hash: str, mime_type: str,
                    last_modified: Any, state: str = "pending") -> str:
    conn = connect()
    row = conn.execute("SELECT id FROM files WHERE file_path=?", (file_path,)).fetchone()
    lm = last_modified.isoformat() if hasattr(last_modified, "isoformat") else str(last_modified)
    if row:
        conn.execute(
            "UPDATE files SET file_hash=?, mime_type=?, last_modified=?, state=? WHERE id=?",
            (file_hash, mime_type, lm, state, row["id"]),
        )
        conn.commit()
        return row["id"]
    fid = str(uuid.uuid4())
    conn.execute(
        "INSERT INTO files(id,file_path,file_hash,mime_type,last_modified,indexed_at,tags,state) "
        "VALUES(?,?,?,?,?,?, '[]', ?)",
        (fid, file_path, file_hash, mime_type, lm, now_iso(), state),
    )
    conn.commit()
    return fid


def set_state(file_path: str, state: str) -> None:
    conn = connect()
    conn.execute("UPDATE files SET state=? WHERE file_path=?", (state, file_path))
    conn.commit()


def get_all_files() -> list[dict]:
    conn = connect()
    rows = conn.execute(
        "SELECT id,file_path,mime_type,tags,context,last_modified,state FROM files "
        "ORDER BY last_modified DESC"
    ).fetchall()
    out = []
    for r in rows:
        out.append({
            "id": r["id"],
            "file_path": r["file_path"],
            "mime_type": r["mime_type"],
            "tags": json.loads(r["tags"] or "[]"),
            "context": r["context"],
            "last_modified": r["last_modified"],
            "state": r["state"],
        })
    return out


def get_file(file_path: str) -> dict | None:
    conn = connect()
    r = conn.execute("SELECT * FROM files WHERE file_path=?", (file_path,)).fetchone()
    return dict(r) if r else None


def get_file_by_id(file_id: str) -> dict | None:
    conn = connect()
    r = conn.execute("SELECT * FROM files WHERE id=?", (file_id,)).fetchone()
    return dict(r) if r else None


def needs_indexing(file_path: str, file_hash: str) -> bool:
    row = get_file(file_path)
    if row is None:
        return True
    return row["file_hash"] != file_hash


def delete_file_by_path(file_path: str) -> bool:
    conn = connect()
    row = conn.execute("SELECT id FROM files WHERE file_path=?", (file_path,)).fetchone()
    if not row:
        return False
    delete_by_id(row["id"])
    return True


def delete_by_id(file_id: str) -> None:
    conn = connect()
    conn.execute("DELETE FROM content_fts WHERE rowid IN "
                 "(SELECT rowid FROM file_content WHERE file_id=?)", (file_id,))
    conn.execute("DELETE FROM file_content WHERE file_id=?", (file_id,))
    conn.execute("DELETE FROM file_relationships WHERE source_file_id=? OR target_file_id=?",
                 (file_id, file_id))
    conn.execute("DELETE FROM files WHERE id=?", (file_id,))
    conn.commit()


def rename_file(old_path: str, new_path: str) -> bool:
    conn = connect()
    row = conn.execute("SELECT id FROM files WHERE file_path=?", (old_path,)).fetchone()
    if not row:
        return False
    conn.execute("UPDATE files SET file_path=? WHERE id=?", (new_path, row["id"]))
    conn.commit()
    return True


def update_metadata(file_id: str, tags: list[str] | None, context: str | None) -> bool:
    conn = connect()
    row = conn.execute("SELECT id FROM files WHERE id=?", (file_id,)).fetchone()
    if not row:
        return False
    if tags is not None:
        conn.execute("UPDATE files SET tags=? WHERE id=?", (json.dumps(tags), file_id))
    if context is not None:
        conn.execute("UPDATE files SET context=? WHERE id=?", (context, file_id))
    conn.commit()
    return True


# ---- chunks ----------------------------------------------------------------

def replace_chunks(file_id: str, chunks: list[dict]) -> None:
    """chunks: [{chunk_index, content_text, embedding: list[float]}]"""
    conn = connect()
    conn.execute("DELETE FROM content_fts WHERE rowid IN "
                 "(SELECT rowid FROM file_content WHERE file_id=?)", (file_id,))
    conn.execute("DELETE FROM file_content WHERE file_id=?", (file_id,))
    for c in chunks:
        cur = conn.execute(
            "INSERT INTO file_content(file_id,chunk_index,content_text,embedding) VALUES(?,?,?,?)",
            (file_id, c["chunk_index"], c["content_text"], serialize_vec(c["embedding"])),
        )
        conn.execute("INSERT INTO content_fts(rowid,content_text) VALUES(?,?)",
                     (cur.lastrowid, c["content_text"]))
    conn.execute("UPDATE files SET indexed_at=? WHERE id=?", (now_iso(), file_id))
    conn.commit()


def has_chunks(file_id: str) -> bool:
    conn = connect()
    r = conn.execute("SELECT 1 FROM file_content WHERE file_id=? LIMIT 1", (file_id,)).fetchone()
    return r is not None


# ---- search ----------------------------------------------------------------

def vector_search(query_vec: list[float], k: int) -> list[dict]:
    conn = connect()
    q = serialize_vec(query_vec)
    rows = conn.execute(
        """
        SELECT f.file_path, f.mime_type, f.tags, c.chunk_index, c.content_text,
               vec_distance_cosine(c.embedding, ?) AS dist
        FROM file_content c JOIN files f ON f.id = c.file_id
        WHERE c.embedding IS NOT NULL
        ORDER BY dist ASC LIMIT ?
        """,
        (q, k),
    ).fetchall()
    return [_row_to_result(r, score=1.0 - float(r["dist"])) for r in rows]


def keyword_search(query: str, k: int) -> list[dict]:
    conn = connect()
    # Escape FTS5 syntax by quoting each token.
    tokens = [t for t in "".join(ch if ch.isalnum() else " " for ch in query).split() if t]
    if not tokens:
        return []
    match = " OR ".join(f'"{t}"' for t in tokens)
    try:
        rows = conn.execute(
            """
            SELECT f.file_path, f.mime_type, f.tags, c.chunk_index, c.content_text,
                   bm25(content_fts) AS rank
            FROM content_fts
            JOIN file_content c ON c.rowid = content_fts.rowid
            JOIN files f ON f.id = c.file_id
            WHERE content_fts MATCH ?
            ORDER BY rank ASC LIMIT ?
            """,
            (match, k),
        ).fetchall()
    except sqlite3.OperationalError:
        return []
    return [_row_to_result(r, score=1.0 / (1.0 + abs(float(r["rank"] or 0.0))))
            for r in rows]


def hybrid_search(query_vec: list[float], query: str, k: int) -> list[dict]:
    vec = vector_search(query_vec, k * 2)
    kw = keyword_search(query, k * 2)
    rrf: dict[str, dict] = {}
    for results in (vec, kw):
        for rank, r in enumerate(results):
            key = f"{r['file_path']}|{r.get('chunk_index')}"
            if key not in rrf:
                rrf[key] = dict(r, score=0.0)
            rrf[key]["score"] += 1.0 / (60 + rank)
    merged = sorted(rrf.values(), key=lambda x: x["score"], reverse=True)
    return merged[:k]


def _row_to_result(r: sqlite3.Row, score: float) -> dict:
    return {
        "file_path": r["file_path"],
        "mime_type": r["mime_type"],
        "tags": json.loads(r["tags"] or "[]"),
        "chunk_index": r["chunk_index"],
        "content_text": (r["content_text"] or "")[:2000],
        "score": round(float(score), 6),
    }


# ---- watched folders -------------------------------------------------------

def add_watch_folder(folder_path: str) -> tuple[bool, int | None]:
    conn = connect()
    if conn.execute("SELECT 1 FROM watched_folders WHERE folder_path=?", (folder_path,)).fetchone():
        return False, None
    cur = conn.execute("INSERT INTO watched_folders(folder_path,created_at) VALUES(?,?)",
                       (folder_path, now_iso()))
    conn.commit()
    return True, cur.lastrowid


def remove_watch_folder(folder_path: str) -> bool:
    conn = connect()
    cur = conn.execute("DELETE FROM watched_folders WHERE folder_path=?", (folder_path,))
    conn.commit()
    return cur.rowcount > 0


def get_watch_folders() -> list[dict]:
    conn = connect()
    rows = conn.execute("SELECT id,folder_path,created_at FROM watched_folders ORDER BY id").fetchall()
    return [{"id": r["id"], "folder_path": r["folder_path"], "created_at": r["created_at"]} for r in rows]
