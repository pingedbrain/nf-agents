#!/usr/bin/env python3
"""Shape fingerprint of a fixture version dir (stdlib, Python >= 3.9).

Usage: shape.py VERSION_DIR  -- prints shape.txt. The shape keeps what a format change would
alter (record field paths and types, store layout, CLI exit codes and masked output) and drops
what changes on every regeneration (LIDs, hashes, UUIDs, timestamps, run names, find order).
"""

import hashlib
import json
import os
import re
import sys
from collections import Counter
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from export import records  # noqa: E402

MASKS = (
    (re.compile(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}"), "<uuid>"),
    (re.compile(r"\b[0-9a-f]{2}/[0-9a-f]{30}\b"), "<workdir>"),
    (re.compile(r"[0-9a-f]{16,}"), "<hex>"),
    (re.compile(r"\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}(\.\d+)?(Z| UTC|[+-]\d{2}:?\d{2})?"), "<time>"),
)


def mask(text, run_names):
    for rx, token in MASKS:
        text = rx.sub(token, text)
    names = sorted({n for n in run_names if n}, key=len, reverse=True)
    if names:
        text = re.sub(r"\b(" + "|".join(map(re.escape, names)) + r")\b", "<run>", text)
    return re.sub(r"[ \t]+", " ", text)


def _fields(value, path, out):
    if isinstance(value, dict):
        out.add((path, "object"))
        for k, v in value.items():
            _fields(v, f"{path}.{k}" if path else k, out)
    elif isinstance(value, list):
        out.add((path, "array"))
        for v in value:
            _fields(v, path + "[]", out)
    else:
        kind = ("null" if value is None else "bool" if isinstance(value, bool)
                else "number" if isinstance(value, (int, float)) else "str")
        out.add((path, kind))


def shape(version_dir):
    d = Path(version_dir)
    store = d / ".lineage"
    names = [f.get("name") for _, k, f in records(store) if k == "WorkflowRun" and isinstance(f.get("name"), str)]
    fields, layout = {}, Counter()
    for dirpath, dirnames, filenames in os.walk(store):
        dirnames.sort()
        for name in sorted(filenames):
            p = Path(dirpath, name)
            layout[mask(p.relative_to(store).as_posix(), names)] += 1
            if name == ".data.json":
                rec = json.loads(p.read_text(encoding="utf-8"))
                kind = rec.get("kind") if isinstance(rec, dict) else None
                _fields(rec, "", fields.setdefault(str(kind), set()))
    lines = ["# shape format 1"]
    lines += [f"record {k} {p} {t}" for k in sorted(fields) for p, t in sorted(fields[k]) if p]
    lines += [f"store {p} x{n}" for p, n in sorted(layout.items())]
    for ex in sorted((d / "oracle").glob("*.exit")):
        case = ex.name[: -len(".exit")]
        digests = []
        for ext in ("out", "err"):
            text = mask((d / "oracle" / f"{case}.{ext}").read_bytes().decode("utf-8", "replace"), names)
            if case.startswith("find-"):  # order varies; 25.04 prints a JSON array (no comma on the last)
                text = "\n".join(sorted(line.rstrip(",") for line in text.splitlines()))
            digests.append(hashlib.sha256(text.encode()).hexdigest())
        lines.append(f"case {case} exit={ex.read_text().strip()} out={digests[0]} err={digests[1]}")
    return "\n".join(lines) + "\n"


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit("usage: shape.py VERSION_DIR")
    sys.stdout.write(shape(sys.argv[1]))
