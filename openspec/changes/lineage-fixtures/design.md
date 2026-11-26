# Design: Lineage fixtures

## Technical Approach

A host-side bash driver (`scripts/gen-fixtures.sh`) that only talks to Docker. Everything else
runs **inside** `nextflow/nextflow:<ver>` (bash + python 3.9 stdlib, verified present in all four
images): the pipeline runs at the fixed mount `/nf-agents-fixture`, every `nextflow lineage` case
is captured, the tree is exported to a symlink-free, pruned version dir with an mtime manifest,
tamper cases run on the now-disposable live tree, and a second container proves the committed
layout round-trips through `nextflow lineage check`. Drift is detected on `shape.txt` because
LIDs/UUIDs/run names cannot be pinned (exploration probe). Specs:
`specs/lineage-fixtures/spec.md`, `specs/ci-pipeline/spec.md`.

## Architecture Decisions

| # | Topic | Options | Decision / rationale |
|---|-------|---------|----------------------|
| D1 | Determinism | pin session (`-resume <uuid>`) · always-PR · **shape fingerprint** | `-resume` fails on a fresh dir (verified); always-PR = weekly noise. Shape masks volatile tokens. |
| D2 | Published symlinks | keep absolute · relative · **dereference** | object_store/S3/MiniStack have no symlinks; Windows; `check` hashes the *recorded path string* + size + mtime, so a regular copy with restored mtime is equivalent (verified, 5/5 succeed). |
| D3 | Work dir content | whole `work/` · **FileOutput paths only** | `.command.*` scripts/logs are noise and not referenced by any record. |
| D4 | Helper language | jq/find on host · python image · **python3 in the Nextflow image** | Host stays bash + Docker; no `find`/`jq` in the image; python3 3.9 in all four images. Guard: `command -v python3 \|\| die`. |
| D5 | Export timing | separate container · **same container, before tamper** | Absolute symlinks only resolve at the fixed mount; tamper then runs on the disposable live tree (no extra copy). |
| D6 | Round trip | trust manifest · **verify container on a `cp -r` copy** | `cp -r` drops mtimes like git; proves restore is necessary *and* sufficient (spec "Without restore"). |
| D7 | Agent store | in 26.10.7 store · **separate `26.10.7-agent/`** | Keeps core oracle (`list` = 2 runs, `find` counts) comparable across versions. |
| D8 | LLM for agent | host stub + `--network host` · **stub inside the container on 127.0.0.1** | Works on Docker Desktop too; no host networking; no keys (`nxf-no-credential`). |
| D9 | IDs for tests | parse `.cmd` · **`ids.json` + `<case>.args` JSON** | Rust tests read JSON, never shell strings. |
| D10 | PR mechanism | peter-evans action · **`gh` + `GITHUB_TOKEN`** | No third-party action (supply chain); limitation: bot pushes don't trigger CI → documented in PR body. |
| D11 | Determinism knobs | — | `--cpus 2` (pins `poolSize`, verified), fixed mount, root user, `NXF_ANSI_LOG=false`, no `-e` passthrough of host env. |
| D12 | HTML / logs | commit · **drop** | `render` HTML embeds a vendor template; oracle keeps its out/err/exit. |

## Data Flow

```
gen-fixtures.sh V (host)
 ├─ validate tag, docker pull, digest
 ├─ TMP/root ← stage pipeline (25.04: prepend preview line) + nextflow.config
 ├─ container A  -v TMP/root:/nf-agents-fixture  -v scripts/fixtures:/nf-agents-tools:ro  -v TMP/out:/out
 │    run-core.sh: run hello, run bonjour → ids.py → cap cases → oracle/
 │    export.py  → /out/V/{.lineage, work/…, results-*/…, nextflow.config, oracle/, manifest.json}
 │    tamper (live tree) → /out/V/oracle/check-{tampered,mtime-only}.*
 │    shape.py → /out/V/shape.txt ; chown -R host uid /out
 ├─ TMP/verify ← cp -r TMP/out/V          (mtimes lost)
 ├─ container B  -v TMP/verify:/nf-agents-fixture
 │    verify.sh: check published → exit 1 ; restore_mtimes.py ; check every FileOutput → succeed ; list == oracle/list.out
 ├─ check.py TMP/out/V (+ --forbid "$USER" if not root)
 └─ swap: OUT/V ← TMP/out/V (rm + mv) ; cleanup TMP via container rm
```

## File Changes

| File | Action | Est. lines |
|------|--------|-----------|
| `fixtures/pipelines/minimal/{main.nf,nextflow.config,data/seed.txt}` | Create (port lab) | 65 |
| `fixtures/pipelines/agent/{main.nf,nextflow.config}` | Create (port lab, pin plugin) | 75 |
| `scripts/gen-fixtures.sh` | Create: driver | 150 |
| `scripts/fixtures/lib.sh` | Create: `cap`, `die`, arg validation | 35 |
| `scripts/fixtures/run-core.sh` / `run-agent.sh` / `verify.sh` | Create: in-container steps | 90 / 50 / 30 |
| `scripts/fixtures/{ids,export,restore_mtimes,shape,check,llm_stub}.py` | Create | 40/70/20/90/110/45 |
| `scripts/fixtures/test_fixtures.py` | Create: unittest | 220 |
| `fixtures/README.md` | Create | 90 |
| `.github/workflows/fixtures.yml` | Create | 95 |
| `.github/workflows/ci.yml` | Modify: 2 steps in `test` | 4 |
| `.gitignore` / `.gitattributes` | Modify / Create | 3 / 2 |
| `fixtures/lineage/{25.04.8,25.10.9,26.04.9,26.10.7,26.10.7-agent}/` | Generated | ≈ 45–60 KB each, ≈ 70 files each |

## Interfaces / Contracts

`manifest.json` (format 1):

```json
{ "format": 1, "kind": "core", "nextflow_version": "26.10.7",
  "image": "nextflow/nextflow:26.10.7@sha256:…", "recorded_root": "/nf-agents-fixture",
  "store": ".lineage",
  "files": [ { "path": "results-hello/summary.txt", "size": 9,
               "mtime_ms": 1795726984182, "sha256": "…" } ] }
```

Python helper APIs (3.9-compatible, stdlib only; each also has a `main()` CLI):

```python
# export.py
def data_paths(store: Path, recorded_root: str) -> dict[str, str]  # LID -> rel path; ValueError outside root
def export(root: Path, out: Path, *, kind: str, version: str, image: str, recorded_root: str) -> None
# shape.py
def mask(text: str, run_names: Iterable[str]) -> str   # lid/hex≥16, xx/hex30, UUID, ISO & history timestamps, run names, collapse spaces
def shape(version_dir: Path) -> str
# check.py
def check_version_dir(d: Path, forbid: Sequence[str] = ()) -> list[str]   # [] == OK; never raises on bad input
# restore_mtimes.py
def restore(d: Path) -> None   # os.utime(ns=mtime_ms*1_000_000) after size check
```

Bash: `cap <case> <argv…>` writes `<case>.{args,out,err,exit}`; rejects argv outside
`^[A-Za-z0-9:/#=._-]+$` before running anything.

Hand-off to the `nf-lineage` change (not implemented here), test support signature:

```rust
// crates/nf-lineage/tests/support/fixtures.rs
pub struct Fixture { pub root: tempfile::TempDir, pub manifest: Manifest }
#[derive(serde::Deserialize)]
pub struct Manifest { pub kind: String, pub nextflow_version: String,
    pub recorded_root: String, pub store: String, pub files: Vec<ManifestFile> }
#[derive(serde::Deserialize)]
pub struct ManifestFile { pub path: String, pub size: u64, pub mtime_ms: u64, pub sha256: String }
/// Copies fixtures/lineage/<name> into a temp dir and sets each file's mtime with
/// `File::set_modified(UNIX_EPOCH + Duration::from_millis(mtime_ms))` (std, no extra crate).
pub fn materialize(name: &str) -> std::io::Result<Fixture>;
impl Fixture { pub fn local_path(&self, recorded: &str) -> Option<std::path::PathBuf>; }
```

`check` then hashes the **recorded** path string with the **local** file's size and mtime.

## Testing Strategy

| Layer | What | How |
|-------|------|-----|
| Unit (offline, CI) | export pruning/dereference/missing file; check (size, sha, coverage, traversal, host path, missing case); shape masking/drift/order; `cap` argv guard; stub json_schema + 501 | `test_fixtures.py` builds temp trees; stub on port 0 in a thread; `cap` via `subprocess` bash |
| Integration (Docker, local + fixtures.yml) | real generation, round trip, determinism | G-gen, G-verify, G-shape |
| Repository (CI) | committed fixtures well-formed | F-check in `ci.yml` `test` |

TDD: each python test is written red first; generation tasks start from a failing `check.py`/`--verify`.

## Migration / Rollout

No migration. `fixtures.yml` must be run once manually after merge; enable repo setting
"Allow GitHub Actions to create and approve pull requests".

## Open Questions

- [ ] Explicit `plugins { id 'nf-agent@0.2.0' }` vs auto-load: confirm during apply that pinning does not change agent behaviour (fallback: rely on 26.10.7's own pin).

## Deviations (2026-11-26, apply PR A)

- The Nextflow images have no `cmp`/`diff`: `verify.sh` compares `list` stdout with
  `oracle/list.out` by `sha256sum` (still byte-exact).
- `check.py --forbid` at generation time runs **inside** the image (host needs only bash +
  Docker); the forbidden text is the host user name when it is not `root` (spec), not the host name.
- `.gitignore` also ignores `__pycache__/` (created when the unittest suite imports the helpers).
- The helpers that import siblings (`ids.py`, `check.py`) prepend their own directory to
  `sys.path`, because `python3 -I` does not add the script directory.
