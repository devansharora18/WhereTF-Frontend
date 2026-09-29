#!/usr/bin/env python3
"""Assemble a self-extracting single-file app:

    [ frontend ][ sidecar bytes ][ magic(16) | offset(u64) | length(u64) | build_id(u64) ]

Cross-platform (used by scripts/embed_sidecar.sh and CI).

Usage: python3 embed_sidecar.py <frontend-bin> <sidecar-bin> <output>
"""
import hashlib
import os
import struct
import sys

MAGIC = b"WHERETFEMBEDSID\x01"


def sha256_head(path: str, limit: int | None = None) -> bytes:
    h = hashlib.sha256()
    read = 0
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
            read += len(chunk)
            if limit and read >= limit:
                break
    return h.digest()


def main() -> int:
    if len(sys.argv) != 4:
        print(__doc__)
        return 2
    frontend, sidecar, out = sys.argv[1], sys.argv[2], sys.argv[3]

    os.makedirs(os.path.dirname(os.path.abspath(out)) or ".", exist_ok=True)
    # Copy the frontend to the output first.
    with open(frontend, "rb") as src, open(out, "wb") as dst:
        for chunk in iter(lambda: src.read(1 << 20), b""):
            dst.write(chunk)

    offset = os.path.getsize(out)
    length = os.path.getsize(sidecar)
    build_id = int.from_bytes(sha256_head(sidecar)[:8], "little")

    with open(out, "ab") as dst, open(sidecar, "rb") as src:
        for chunk in iter(lambda: src.read(1 << 20), b""):
            dst.write(chunk)
        dst.write(struct.pack("<16sQQQ", MAGIC, offset, length, build_id))

    if os.name == "posix":
        os.chmod(out, 0o755)

    print(f"[embed] {out}: frontend={offset} sidecar={length} build_id={build_id}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
