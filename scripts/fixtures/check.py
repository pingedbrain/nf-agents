#!/usr/bin/env python3
"""Offline well-formedness check of committed lineage fixtures (stdlib, Python >= 3.9).

Usage: check.py FIXTURES_ROOT [--forbid TEXT]...  -- checks every version dir under the root.
Needs no Docker and no network; never raises on malformed input, reports it instead.
"""

import argparse
import hashlib
import json
import os
import re
import sys
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from export import data_paths, records  # noqa: E402
from shape import shape  # noqa: E402

CORE_CASES = (
    "version", "help", "list", "view-workflowrun", "view-workflowrun-output", "view-published-file",
    "view-published-file-subdir", "view-taskrun", "view-taskrun-summarize", "view-taskoutput",
    "view-task-fileoutput", "view-field", "view-not-lid", "view-missing", "find-taskrun",
    "find-workflowrun", "find-fileoutput", "find-fileoutput-label", "find-taskoutput",
    "find-workflowoutput", "find-bad-field", "find-noeq", "diff-runs", "diff-tasks",
    "check-published", "check-taskfile", "check-taskrun", "check-tampered", "check-mtime-only",
    "render-file", "render-run",
)
AGENTS = ("qa", "qa-structured", "shouty")
AGENT_CASES = ("version", "list", "find-agentrun", "render-agentrun") + tuple(
    f"view-agentrun{part}-{a}" for a in AGENTS for part in ("", "-output"))
SAFE_ARG = re.compile(r"[A-Za-z0-9:/#=._-]+")
# Host path shapes, not bare user names: "runner" is a legit AgentRun field, "/home/runner/" is not.
HOST_MARKERS = ("/home/", "/Users/", "C:\\", "/runner/", "/__w/", "/github/workspace")
REQUIRED = ("manifest.json", "nextflow.config", "oracle", ".lineage", "shape.txt")


def safe_rel(p):
    return (isinstance(p, str) and "\\" not in p and not p.startswith("/")
            and all(part not in ("", ".", "..") for part in p.split("/")))


def check_version_dir(d, forbid=()):
    """Return the list of problems in one version dir ([] == OK)."""
    try:
        return _check(Path(d), tuple(forbid))
    except Exception as e:  # malformed input is reported, never raised
        return [f"unreadable fixture: {type(e).__name__}: {e}"]


def _check(d, forbid):
    errs = [f"missing {r}" for r in REQUIRED if not (d / r).exists()]
    if errs:
        return errs
    m = json.loads((d / "manifest.json").read_text(encoding="utf-8"))
    if (not isinstance(m, dict) or m.get("format") != 1 or m.get("kind") not in ("core", "agent")
            or m.get("store") != ".lineage" or not isinstance(m.get("files"), list)
            or not all(isinstance(m.get(k), str) for k in ("nextflow_version", "image", "recorded_root"))):
        return ["manifest.json: bad header"]
    listed = _check_files(d, m["files"], errs)
    fileoutputs = set(data_paths(d / ".lineage", m["recorded_root"]).values())
    errs += [f"{p}: FileOutput not in manifest" for p in sorted(fileoutputs - set(listed))]
    errs += [f"{p}: manifest entry is not a FileOutput" for p in sorted(set(listed) - fileoutputs)]
    _check_tree(d, set(listed), HOST_MARKERS + forbid, errs)
    _check_records(d, m["kind"], errs)
    _check_oracle(d / "oracle", m["kind"], errs)
    if not errs and (d / "shape.txt").read_text(encoding="utf-8") != shape(d):  # shape needs a full oracle
        errs.append("shape.txt is stale (rerun scripts/fixtures/shape.py)")
    return errs


def _check_files(d, entries, errs):
    listed = []
    for e in entries:
        p = e.get("path") if isinstance(e, dict) else None
        if not safe_rel(p):
            errs.append(f"manifest: unsafe path {p!r}")
            continue
        listed.append(p)
        f = d / p
        if f.is_symlink() or not f.is_file():
            errs.append(f"{p}: missing or not a regular file")
            continue
        data = f.read_bytes()
        if e.get("size") != len(data):
            errs.append(f"{p}: size {len(data)} != manifest {e.get('size')!r}")
        elif e.get("sha256") != hashlib.sha256(data).hexdigest():
            errs.append(f"{p}: sha256 does not match the manifest")
        if not isinstance(e.get("mtime_ms"), int) or e["mtime_ms"] < 0:
            errs.append(f"{p}: bad mtime_ms")
    if listed != sorted(set(listed)):
        errs.append("manifest: files not sorted by path or not unique")
    return listed


def _check_tree(d, listed, markers, errs):
    for dirpath, dirnames, filenames in os.walk(d):
        for name in sorted(dirnames + filenames):
            full = Path(dirpath, name)
            rel = full.relative_to(d).as_posix()
            if full.is_symlink():
                errs.append(f"{rel}: symlink")
            elif name.startswith(".nextflow") or name.endswith((".html", ".log")):
                errs.append(f"{rel}: file not allowed in a fixture")
            elif full.is_file():
                if rel.startswith("work/") and rel not in listed:
                    errs.append(f"{rel}: work file not in manifest")
                blob = full.read_bytes()
                errs += [f"{rel}: contains host marker {mk!r}" for mk in markers if mk.encode() in blob]


def _check_records(d, kind, errs):
    runs, published, outputs, agents = [], {}, 0, []
    for lid, k, f in records(d / ".lineage"):
        if k == "WorkflowRun":
            runs.append(lid)
            config = f.get("config")
            if not isinstance(config, dict) or config.get("poolSize") != 2:
                errs.append(f"{lid}: config.poolSize is not 2")
        elif k == "AgentRun":
            agents.append(f)
        elif k == "WorkflowOutput":
            outputs += 1
        elif k == "FileOutput" and f.get("taskRun") is None:
            published[f.get("workflowRun")] = published.get(f.get("workflowRun"), 0) + 1
    if kind == "core":
        if not outputs:
            errs.append("no WorkflowOutput record")
        errs += [f"{r}: fewer than 2 published FileOutputs" for r in runs if published.get(r, 0) < 2]
    elif not (len(agents) >= 2 and any(a.get("tools") for a in agents)
              and any(a.get("outputSchema") for a in agents)):
        errs.append("need >= 2 AgentRun records, one AgentRun with tools and one with outputSchema")


def _check_oracle(o, kind, errs):
    for case in CORE_CASES if kind == "core" else AGENT_CASES:
        missing = [f"oracle/{case}.{ext}: missing" for ext in ("args", "out", "err", "exit")
                   if not (o / f"{case}.{ext}").is_file()]
        errs += missing
        if missing:
            continue
        if not re.fullmatch(r"[0-9]+\n", (o / f"{case}.exit").read_text()):
            errs.append(f"oracle/{case}.exit: not an exit code")
        args = json.loads((o / f"{case}.args").read_text())
        if not isinstance(args, list) or not all(isinstance(a, str) and SAFE_ARG.fullmatch(a) for a in args):
            errs.append(f"oracle/{case}.args: not a JSON array of safe arguments")
    ids = json.loads((o / "ids.json").read_text()) if (o / "ids.json").is_file() else {}
    need = {"RUN1", "RUN2"} if kind == "core" else {"RUN1"}
    if not isinstance(ids, dict) or not need <= set(ids):
        errs.append(f"oracle/ids.json: missing or without {sorted(need)}")


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("root")
    ap.add_argument("--forbid", action="append", default=[], help="extra text that must not appear")
    a = ap.parse_args(argv)
    root = Path(a.root)
    dirs = sorted(p for p in root.iterdir() if p.is_dir()) if root.is_dir() else []
    if not dirs:
        print(f"check: no version dirs under {root}", file=sys.stderr)
        return 1
    failed = False
    for v in dirs:
        errs = check_version_dir(v, [f for f in a.forbid if f])
        for e in errs:
            print(f"{v.name}: {e}", file=sys.stderr)
        print(f"{'FAIL' if errs else 'ok'} {v.name}")
        failed = failed or bool(errs)
    return int(failed)


if __name__ == "__main__":
    sys.exit(main())
