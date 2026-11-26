#!/usr/bin/env python3
"""Restore data file mtimes from manifest.json (stdlib, Python >= 3.9).

Usage: restore_mtimes.py VERSION_DIR. git does not keep mtimes, but `nextflow lineage check`
hashes path + size + mtime, so a checked-out fixture needs this before `check` can succeed.
"""

import json
import os
import sys
from pathlib import Path


def restore(d):
    d = Path(d)
    for e in json.loads((d / "manifest.json").read_text(encoding="utf-8"))["files"]:
        p = e["path"]
        if p.startswith("/") or ".." in p.split("/"):
            raise ValueError(f"unsafe manifest path {p!r}")
        f = d / p
        if f.is_symlink() or f.stat().st_size != e["size"]:
            raise ValueError(f"{p}: not a regular file of {e['size']} bytes")
        ns = e["mtime_ms"] * 1_000_000
        os.utime(f, ns=(ns, ns))


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit("usage: restore_mtimes.py VERSION_DIR")
    try:
        restore(sys.argv[1])
    except (OSError, ValueError, KeyError) as e:
        sys.exit(f"restore_mtimes: {e}")
