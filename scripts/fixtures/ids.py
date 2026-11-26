#!/usr/bin/env python3
"""Map the fixture runs and tasks to their LIDs (stdlib, Python >= 3.9).

Usage: ids.py [--agent] STORE OUT_JSON  -- writes OUT_JSON and prints KEY=LID lines for the shell.
Core: RUN1 is the run with `greeting=hello`, RUN2 the one with `greeting=bonjour`;
TASK_<PROCESS>_RUN<n> is the TaskRun of that process in run n.
Agent (--agent): RUN1 is the only run, AGENT_<NAME> the AgentRun of agent <name>.
"""

import json
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from export import records  # noqa: E402

GREETINGS = {"hello": 1, "bonjour": 2}


def ids(store):
    runs, tasks = {}, []
    for lid, kind, f in records(store):
        if kind == "WorkflowRun":
            params = {p.get("name"): p.get("value") for p in f.get("params") or [] if isinstance(p, dict)}
            if params.get("greeting") not in GREETINGS:
                raise ValueError(f"{lid}: unexpected greeting {params.get('greeting')!r}")
            runs[lid] = GREETINGS[params["greeting"]]
        elif kind == "TaskRun":
            tasks.append((lid, f.get("name"), f.get("workflowRun")))
    out = {f"RUN{n}": lid for lid, n in runs.items()}
    if sorted(out) != ["RUN1", "RUN2"] or len(runs) != 2:
        raise ValueError(f"expected one hello and one bonjour run, got {sorted(runs.values())}")
    for lid, name, run in tasks:
        m = re.match(r"[A-Z][A-Z0-9_]*", name if isinstance(name, str) else "")
        key = f"TASK_{m.group(0)}_RUN{runs.get(run)}" if m and run in runs else None
        if key is None or key in out:
            raise ValueError(f"{lid}: unexpected or duplicate task {name!r} in {run!r}")
        out[key] = lid
    return dict(sorted(out.items()))


def agent_ids(store):
    out = {}
    for lid, kind, f in records(store):
        name = f.get("name")
        key = "RUN1" if kind == "WorkflowRun" else f"AGENT_{name.upper()}" if kind == "AgentRun" and \
            isinstance(name, str) and re.fullmatch(r"[a-z][a-z0-9_]*", name) else None
        if key in out or (kind == "AgentRun" and key is None):
            raise ValueError(f"{lid}: unexpected or duplicate {kind} {name!r}")
        if key:
            out[key] = lid
    if "RUN1" not in out:
        raise ValueError("no WorkflowRun record")
    return dict(sorted(out.items()))


def main(argv=None):
    args = sys.argv[1:] if argv is None else argv
    agent = args[:1] == ["--agent"]
    args = args[1:] if agent else args
    if len(args) != 2:
        sys.exit("usage: ids.py [--agent] STORE OUT_JSON")
    store, dest = args
    try:
        found = agent_ids(store) if agent else ids(store)
    except (OSError, ValueError) as e:
        sys.exit(f"ids: {e}")
    with open(dest, "w") as fh:
        fh.write(json.dumps(found, indent=2) + "\n")
    print("\n".join(f"{k}={v}" for k, v in found.items()))


if __name__ == "__main__":
    main()
