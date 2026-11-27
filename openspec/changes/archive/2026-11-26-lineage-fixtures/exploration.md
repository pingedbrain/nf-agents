## Exploration: lineage-fixtures

Scope: offline lineage fixtures for every supported Nextflow format generation, as described in
`docs/verification.md` (sections 1-3), plus the `fixtures` CI job.

### Current State

- The repo has no `fixtures/`, no `scripts/` and a single workflow (`.github/workflows/ci.yml`,
  jobs `lint` and `test`, `permissions: contents: read`). `crates/nf-lineage` is a stub. Nothing
  reads fixtures yet. The next change (`nf-lineage`) needs them.
- `.gitignore` ignores `work/`, `results/`, `.nextflow*` repo-wide. **A fixture tree that contains
  a `work/` directory is silently ignored by git** unless we add a negation.
- An empirical lab lives outside the repo (a local scratch directory, not committed):
  - `pipeline/minimal/main.nf`: GENERATE → TRANSFORM → SUMMARIZE with an `output` block
    (`upper` → `upper/`, `summary` → `.` with label `summary`), plus `params.greeting`, and
    `data/seed.txt` (3 words).
  - `scripts/run-version.sh` and `inside.sh`: two runs (`--greeting hello|bonjour`) per version,
    then ~35 `nextflow lineage` invocations captured as `cli/<case>.{cmd,out,err,exit}`.
  - `agent-capture/`: `capture_server.py` (python stdlib OpenAI-compatible stub) plus a
    3-agent pipeline (`qa`, `qa_structured`, `shouty` with a tool) that produced `AgentRun`
    records on 26.10.7 with the default `langchain4j` runner, no API key.
- Lab defects that the repo version must fix:
  1. The lab mounts the host directory at **the same absolute host path**, so every record
     carries `/home/<user>/…` (WorkflowRun `metadata`, `commandLine`, `FileOutput.path`, …).
  2. The tamper cases (`check-tampered`, `check-mtime-only`) **modify the store's own published
     files in place**: the result tree cannot be committed afterwards.
  3. Published outputs are **absolute symlinks** into `work/`
     (`results-hello/upper/upper.txt -> /…/work/16/f17a…/upper.txt`). In git they are broken
     links on any other machine.
  4. `render` writes HTML that embeds a Seqera-copyrighted template; no test needs it.

### Facts verified during this exploration (new, 2026-11-26)

Probe in the scratchpad with `nextflow/nextflow:26.10.7`, tree mounted at `/nf-agents-fixture`:

| Question | Result |
| --- | --- |
| Can task hashes be pinned (`-resume <fixed-uuid>` on a fresh work dir)? | **No.** `Can't find a run with the specified id … Execution can't be resumed`, exit 1. Task hashes include the session UUID, so LIDs, session IDs, run names and timestamps change on every regeneration. Byte-identical fixtures are impossible. |
| Does `docker run --cpus 2` pin `WorkflowRun.config.poolSize`? | **Yes** (`"poolSize":2`; the lab had `12.0` from the host). |
| Do absolute published symlinks resolve when the tree is mounted elsewhere? | **No** — export must run with the tree mounted at the fixed path (or resolve targets against the recorded root). |
| Dereference published symlinks + keep only FileOutput-referenced work files + record `{path,size,mtime_ms}` → plain copy (mtimes lost) → restore mtimes with `os.utime(ns=mtime_ms*1e6)` → mount at the fixed path → `nextflow lineage check` on every `FileOutput` | **All 5 FileOutputs: `Checksum validation succeed`.** The manifest approach works end to end. |
| Tools inside the official images (25.04.8 … 26.10.7) | Amazon Linux 2023, **`python3` 3.9 present in all four**, GNU coreutils 8.32, **no `find`**, no `jq`. |
| `FileOutput` records already carry `size` and `modifiedAt` (ns) | Yes; the manifest is still needed so `check` tests do not depend on the parser under test to set up their own preconditions. |

Sizes (lab, per version): `.lineage` 13–17 KB in 28 files; oracle text ≈ 20 KB (without run logs
and HTML); FileOutput-referenced data files: 10 files of 9–35 bytes. Agent store: 5.5 KB, 7 files.

### Affected Areas

- `fixtures/pipelines/minimal/` (new) — the 3-process pipeline, seed data, `nextflow.config`.
- `fixtures/pipelines/agent/` (new) — agent pipeline, config pinned to `nf-agent@0.2.0`, LLM stub.
- `scripts/gen-fixtures.sh` (+ small python helpers run **inside** the Nextflow image) (new).
- `fixtures/lineage/<ver>/` for 25.04.8, 25.10.9, 26.04.9, 26.10.7 and `26.10.7-agent/` (new, generated).
- `fixtures/README.md` (new) — provenance, license, regeneration.
- `.github/workflows/fixtures.yml` (new) — `workflow_dispatch` + weekly; opens a PR.
- `.github/workflows/ci.yml` (modified) — cheap fixtures well-formedness step.
- `.gitignore` (modified) — re-include `fixtures/lineage/*/work/`.
- `.gitattributes` (new) — `fixtures/lineage/** -text` (CRLF/whitespace normalization would
  change sizes and break `check`; `list` oracle has significant trailing spaces).
- No Rust crate is touched.

### Approaches

1. **Determinism: pin hashes** (fixed session id / `-resume`) — Rejected: verified impossible.
2. **Determinism: path-stable, machine-independent records + drift detection on a normalized
   "shape"** — Run in a fixed mount (`/nf-agents-fixture`), `--cpus 2`, fixed user (root) and
   image; commit a `shape.txt` per store (record kinds × sorted field paths + oracle cases with
   volatile tokens masked); the weekly job opens a PR only when the shape changes (or on a
   manual `force`).
   - Pros: no weekly noise PRs; the PR diff shows exactly what drifted; meaningful "if they change".
   - Cons: ~60 lines of python for normalization.
   - Effort: Medium.
3. **Always regenerate and open a PR** — Rejected: every run differs (UUIDs, hashes, times),
   so it would open a PR every week.

Published outputs in git:

| Option | Pros | Cons |
| --- | --- | --- |
| Keep absolute symlinks | faithful | broken on every checkout |
| Rewrite as relative symlinks | faithful, tiny | object_store and S3 have no symlinks (MiniStack upload), Windows checkouts with `core.symlinks=false` |
| **Dereference to regular files, restore mtimes from manifest** | works on local FS, in-memory, S3; verified with `nextflow lineage check` | 4 tiny duplicated files per version |

Which work files to keep: **only files whose absolute path is the `path` of a `FileOutput`
record** (task outputs). Drop `.command.*`, `.exitcode`, staged input links, `.nextflow/`, logs.

Host requirements: **bash + Docker only**. Everything else (JSON, mtimes, stub server, shape)
runs as python 3.9 stdlib **inside** the Nextflow image that is already pulled.

`AgentRun` fixture: feasible offline for LLMs. Run a ~30-line stdlib stub
(`POST /v1/chat/completions`, non-streaming, json_schema-aware) **inside the same container** on
`127.0.0.1`, `agent.baseUrl` → stub, no key (Nextflow sends `nxf-no-credential`). Only network
needed: Docker pull and the one-time `nf-agent@0.2.0` plugin download (pinned), same class as
image pulls. Separate store `fixtures/lineage/26.10.7-agent/` so the 26.10.7 core oracle
(`list` = 2 runs, `find` counts) stays comparable across versions.

Oracle: keep every lab case; drop `render` HTML (keep its stdout/stderr/exit); tamper cases run
on a throw-away copy of the tree; add machine-readable `args` and an `ids.json` (RUN1, RUN2,
TASK_* LIDs) so Rust tests never parse `.cmd` strings.

### Recommendation

Approach 2 with dereferenced outputs, FileOutput-only work files, an mtime manifest (with
sha256 to catch git corruption), a generation-time **round-trip verification** (restore mtimes
from the committed layout, mount at the fixed path, `nextflow lineage check` every FileOutput,
`list` equals the oracle), and a separate offline agent store. The fixtures workflow uses only
first-party tooling (`actions/checkout` + `gh`), `GITHUB_TOKEN` with `contents: write` and
`pull-requests: write`. The weekly `26.11.0-edge` run is a separate non-blocking job that only
reports a shape diff.

### Risks

- PRs opened with `GITHUB_TOKEN` do **not** trigger `ci.yml`; the repo setting "Allow GitHub
  Actions to create and approve pull requests" must be on. Both need README/maintainer notes.
- `python3` is a distro package in the image, not a contract; a future image could drop it
  (guard with a clear error; fallback: pinned `python:3.13-slim` helper image).
- `nf-agent` plugin download at generation time (registry availability); `agent` is preview.
- Hidden paths (`.lineage`, `.history`, `.data.json`) are skipped by some tools
  (`actions/upload-artifact` ≥ 4.4 needs `include-hidden-files: true`).
- `#` in directory names (`<hash>#output`) — fine for git; object_store needs `Path::parse`
  (owned by the nf-lineage change).
- Root-owned files from containers: generator must `chown` back or work in a container.

### Ready for Proposal

Yes. Decisions above are backed by probes; no open question blocks the proposal.
