#!/usr/bin/env python3
"""Export a live Nextflow run tree into a committable fixture version dir (stdlib, Python >= 3.9).

Copies `.lineage/` verbatim and `nextflow.config`, and copies every data file (the `path` of a
FileOutput record) as a regular file: published symlinks are dereferenced, keeping the target's
bytes and mtime. Everything else under `work/` is dropped. Writes `manifest.json` (format 1).
"""

import argparse
import hashlib
import json
import os
import shutil
import sys
from pathlib import Path


def records(store):
    """Yield (lid, kind, fields) for every record; accepts flat (25.04) and `spec` envelopes."""
    store = Path(store)
    for dirpath, dirnames, filenames in os.walk(store):
        dirnames[:] = sorted(n for n in dirnames if n != ".history")
        if ".data.json" not in filenames or Path(dirpath) == store:
            continue
        rec = json.loads(Path(dirpath, ".data.json").read_text(encoding="utf-8"))
        lid = "lid://" + Path(dirpath).relative_to(store).as_posix()
        if not isinstance(rec, dict):
            raise ValueError(f"{lid}: record is not a JSON object")
        yield lid, rec.get("kind"), rec["spec"] if isinstance(rec.get("spec"), dict) else rec


def data_paths(store, recorded_root):
    """FileOutput LID -> data file path relative to the recorded root."""
    prefix = recorded_root.rstrip("/") + "/"
    out = {}
    for lid, kind, fields in records(store):
        if kind != "FileOutput":
            continue
        path = fields.get("path")
        rel = path[len(prefix):] if isinstance(path, str) and path.startswith(prefix) else ""
        if not rel or ".." in rel.split("/"):
            raise ValueError(f"{lid}: path {path!r} is outside {recorded_root}")
        out[lid] = rel
    return out


def export(root, out, *, kind, version, image, recorded_root):
    root, out = Path(root), Path(out)
    by_path = {rel: lid for lid, rel in data_paths(root / ".lineage", recorded_root).items()}
    out.mkdir(parents=True, exist_ok=True)
    shutil.copytree(root / ".lineage", out / ".lineage")
    shutil.copy2(root / "nextflow.config", out / "nextflow.config")
    files = []
    for rel in sorted(by_path):
        src, dst = root / rel, out / rel
        if not src.is_file():
            raise FileNotFoundError(f"{by_path[rel]}: data file {recorded_root.rstrip('/')}/{rel} not found")
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src, dst)  # follows symlinks: target bytes and mtime
        data = dst.read_bytes()
        files.append({"path": rel, "size": len(data), "mtime_ms": src.stat().st_mtime_ns // 1_000_000,
                      "sha256": hashlib.sha256(data).hexdigest()})
    manifest = {"format": 1, "kind": kind, "nextflow_version": version, "image": image,
                "recorded_root": recorded_root, "store": ".lineage", "files": files}
    (out / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--kind", required=True, choices=("core", "agent"))
    ap.add_argument("--version", required=True)
    ap.add_argument("--image", required=True)
    ap.add_argument("--recorded-root", required=True)
    ap.add_argument("root")
    ap.add_argument("out")
    a = ap.parse_args(argv)
    try:
        export(a.root, a.out, kind=a.kind, version=a.version, image=a.image, recorded_root=a.recorded_root)
    except (OSError, ValueError) as e:
        sys.exit(f"export: {e}")


if __name__ == "__main__":
    main()
