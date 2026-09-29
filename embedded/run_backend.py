#!/usr/bin/env python3
"""Entrypoint for the frozen embedded backend sidecar.

Usage:
  wheretf-backend [--host 127.0.0.1] [--port 8000] [--tier pro] [--data-dir DIR]
"""
from __future__ import annotations

import argparse
import os


def main() -> None:
    parser = argparse.ArgumentParser(prog="wheretf-backend")
    parser.add_argument("--host", default=os.getenv("WHERETF_HOST", "127.0.0.1"))
    parser.add_argument("--port", type=int, default=int(os.getenv("WHERETF_PORT", "8000")))
    parser.add_argument("--tier", default=os.getenv("WHERETF_TIER", os.getenv("APP_TIER", "pro")))
    parser.add_argument("--data-dir", default=os.getenv("WHERETF_DATA_DIR"))
    parser.add_argument("--skip-preload", action="store_true", default=bool(os.getenv("WHERETF_SKIP_PRELOAD")))
    args = parser.parse_args()

    os.environ["APP_TIER"] = args.tier
    os.environ["WHERETF_PORT"] = str(args.port)
    if args.data_dir:
        os.environ["WHERETF_DATA_DIR"] = args.data_dir
    if args.skip_preload:
        os.environ["WHERETF_SKIP_PRELOAD"] = "1"

    import uvicorn

    from wheretf_embedded.main import app

    uvicorn.run(app, host=args.host, port=args.port, log_level="info")


if __name__ == "__main__":
    main()
