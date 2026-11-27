# Apply progress: lineage-fixtures

Mode: **Strict TDD** (runner: `python3 -I -m unittest discover -s scripts/fixtures -p 'test_*.py'`;
`cargo test --workspace --locked` unchanged and green; no Rust touched).

## Batches

| Batch | Tasks | Status |
|-------|-------|--------|
| PR A | 1.1–4.4 | done (2026-11-26) |
| PR B | 5.1–8.3 | done (2026-11-26); C-fx-run after merge (orchestrator) |

## PR A — completed tasks

- [x] 1.1 pipeline ported (`fixtures/pipelines/minimal/`, SPDX Apache-2.0 header, no host paths)
- [x] 1.2 `.gitattributes` (`fixtures/lineage/** -text`), `.gitignore` (`!/fixtures/lineage/*/work/`, `__pycache__/`); V-ignore exit 1, V-attr `unset` (dummy file, then on real fixtures)
- [x] 1.3 `scripts/fixtures/lib.sh` (`die`, `cap` with argv regex guard)
- [x] 2.1–2.5 `export.py`, `ids.py`, `check.py`, `restore_mtimes.py` + `test_fixtures.py` (16 tests)
- [x] 3.1–3.4 `run-core.sh`, `verify.sh`, `scripts/gen-fixtures.sh`; 4 core stores generated
- [x] 4.1–4.4 CI C-fixtures steps, V-probe, `fixtures/README.md`, gates

## PR B — completed tasks

- [x] 5.1 `scripts/fixtures/shape.py` (`mask`, `shape`; masks UUID, `xx/hex30`, hex≥16, ISO/`UTC` timestamps, run names, spaces; `find-*` lines sorted without trailing commas)
- [x] 5.2 `check.py` requires `shape.txt` and fails when it is stale; `run-core.sh` writes it
- [x] 6.1 `scripts/fixtures/llm_stub.py` (non-streaming chat completions, schema-valid JSON, 501 otherwise, 127.0.0.1 only)
- [x] 6.2 `fixtures/pipelines/agent/{main.nf,nextflow.config}` (3 agents, langchain4j, stub baseUrl, `nf-agent@0.2.0` pinned; open question resolved: pin is behaviour-neutral)
- [x] 6.3 `run-agent.sh`, `ids.py --agent`, `AGENT_CASES` + AgentRun record checks in `check.py`, `verify.sh` agent branch, `gen-fixtures.sh` builds `26.10.7-agent` with 26.10.7
- [x] host-leak decision: `HOST_MARKERS` += `/runner/`, `/__w/`, `/github/workspace`; `--forbid "$HOME"` (not `/`, `/root`) instead of the user name
- [x] 7.1–7.3 five stores regenerated; F-check, G-verify, G-shape, G-edge
- [x] 8.1–8.3 `.github/workflows/fixtures.yml`, README sections, gates

## TDD Cycle Evidence

| Task | RED | GREEN | REFACTOR |
|------|-----|-------|----------|
| 2.1 `CasesTests.test_unsafe_arg_rejected` | stub `lib.sh` → FAIL (`0 == 0 : a b`) | `lib.sh` → OK | — |
| 2.2 `ExportTests.*` (3) | stub `export.py` → NotImplementedError | `export.py` → 3 OK | dedupe by path, `main()` exits naming LID+path |
| 2.3 `IdsTests.test_flat_and_spec_envelopes` | stub → error | `ids.py` → OK | argv count guard |
| 2.4 `CheckTests.*` (8 named + `test_valid_dir_passes`, `test_forbidden_name_rejected`) | stub → errors | `check.py` → 10 OK | — |
| 2.5 `RestoreTests.test_restores_ms_and_checks_size` | stub → NotImplementedError | `restore_mtimes.py` → OK | — |
| 3.1 F-check on empty tree | `check.py fixtures/lineage` → exit 1 "no version dirs" | after 3.4 → exit 0, `ok` ×4 | — |
| 3.2/3.3 generator | first run: `cmp: command not found` → round trip failed | `sha256sum` compare → `round trip ok: 10 FileOutputs` | — |

| 5.1 `ShapeTests.*` (3) | stub `shape.py` → NotImplementedError | `shape.py` → 3 OK | — |
| 5.1 (G-shape finding) `test_find_order_ignored` + JSON-array case | first version passed trivially (names did not swap) → fixed test → FAIL (hash differs) | `rstrip(",")` before sort → OK | — |
| 5.2 `CheckTests.test_missing_shape_fails`, `test_stale_shape_fails` | `[]` returned → FAIL | `REQUIRED` + stale compare → OK; stale gated on a complete oracle (shape reads every case) | — |
| host markers `CheckTests.test_host_path_shapes_only` | `/__w/`, `/github/workspace` not caught → FAIL | `HOST_MARKERS` extended → OK; `runner` word with `--forbid /home/runner` passes | — |
| 6.1 `StubTests.*` (2) | stub → NotImplementedError | `llm_stub.py` → 2 OK (schema object/array/number + plain text triangulated) | — |
| 6.3 `CheckTests.test_agent_cases`, `IdsTests.test_agent_ids` | `AGENT_CASES` / `agent_ids` missing → errors | `check.py`, `ids.py` → OK (missing case file and missing tools AgentRun both fail) | `_check_oracle` takes `kind` |
| 7.1 F-check | committed stores without `shape.txt` → `missing shape.txt` ×4, exit 1 | after G-gen → `ok` ×5 | — |

PR A final: 16/16 unit tests OK on host Python 3.12 **and** Python 3.9.23 (inside `nextflow/nextflow:25.04.8`).

## Integration evidence (Docker)

| Check | Result |
|-------|--------|
| G-gen (core matrix) | 4/4 versions written, 5m14s; each: 10 FileOutputs round-tripped, check `ok` |
| G-verify | 4× `round trip ok: 10 FileOutputs` (incl. "without restore exits 1") |
| F-check | exit 0, `ok` ×4 |
| Sizes (apparent) | 28K, 28K, 35K, 35K (≤ 100 KB); 165 files each |
| V-nodocker | `PATH` without docker → `docker not found`, exit 1, nothing written |
| V-badtag | `latest; rm -rf /` → `invalid version tag`, exit 1, before Docker |
| V-atomic | `--out` copy + `99.99.99` (pull fails) and `24.10.5` (run fails: no stable output block) → exit 1, dir byte-identical |
| V-preview | committed `main.nf` starts with SPDX line; 25.04.8 store has `WorkflowOutput` records (only possible with the staged preview flag) |
| V-probe | one byte flipped in `26.10.7/results-hello/summary.txt` → `sha256 does not match`, exit 1; reverted → exit 0 |
| V-yaml | `permissions: contents: read`, 0 `secrets.` |
| V-readme | `Apache-2.0`, `synthetic`, `scripts/gen-fixtures.sh`, `mtime_ms`, `never edit` all found |
| Host leakage | `grep -rIl '<user>\|/home/\|/Users/' fixtures/` → no match |

## Integration evidence PR B (Docker)

| Check | Result |
|-------|--------|
| Unit | 26/26 OK on Python 3.12 (host) and 3.9.23 (`nextflow/nextflow:25.04.8`) |
| G-gen (all) | 5/5 stores written, 5m40s; core 10 FileOutputs round-tripped each; agent 3 AgentRuns, 0 FileOutputs |
| F-check | `ok` ×5, exit 0 |
| G-verify | 4× `round trip ok: 10 FileOutputs`, 1× `0 FileOutputs` (agent), 1m25s |
| G-shape | 2nd full generation into `--out`: 4/5 identical at first, `25.04.8 find-fileoutput` differed (JSON-array comma) → fixed test-first → 5/5 identical |
| G-edge | `26.11.0-edge` generates + round-trips; shape diff vs 26.10.7 only in `version`, `view-workflowrun`, `diff-runs` (version strings), no record-field change |
| Stale-shape probe | one byte appended to an agent oracle file on a copy → `shape.txt is stale`, exit 1 |
| Sizes (apparent) | 36K, 38K, 47K, 47K, agent 19K (≤ 100 KB) |
| Host leakage | `grep -rIlE '<user>|/home/|/Users/|<dir>' fixtures/` → none |
| V-attr | `26.10.7-agent/oracle/list.out: text: unset` |
| V-fx-yaml | `on` = {workflow_dispatch, schedule}; top `contents: read`; regenerate job `contents: write` + `pull-requests: write`; edge job `contents: read`, `continue-on-error: true`; 0 `secrets.`; only `actions/checkout@v7`; `actionlint` 1.7.7 (Docker) clean |
| Drift-step logic | `git status --porcelain --untracked-files=all -- 'fixtures/lineage/*/shape.txt'` lists all 5 versions on the new (untracked) shape files |
| Repo setting | `can_approve_pull_request_reviews: false` → the PR step will fail with the documented error until enabled |

## Deviations

See design.md "Deviations (2026-11-26, apply PR A)" and "Deviations (2026-11-26, apply PR B)";
spec deviation note under "No host leakage".

## Remaining

Orchestrator after merge: enable "Allow GitHub Actions to create and approve pull requests",
then C-fx-run (`workflow_dispatch`, `force: false`) → expect "no format drift".
