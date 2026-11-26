# Apply progress: lineage-fixtures

Mode: **Strict TDD** (runner: `python3 -I -m unittest discover -s scripts/fixtures -p 'test_*.py'`;
`cargo test --workspace --locked` unchanged and green; no Rust touched).

## Batches

| Batch | Tasks | Status |
|-------|-------|--------|
| PR A | 1.1–4.4 | done (2026-11-26) |
| PR B | 5.1–8.3 | pending |

## PR A — completed tasks

- [x] 1.1 pipeline ported (`fixtures/pipelines/minimal/`, SPDX Apache-2.0 header, no host paths)
- [x] 1.2 `.gitattributes` (`fixtures/lineage/** -text`), `.gitignore` (`!/fixtures/lineage/*/work/`, `__pycache__/`); V-ignore exit 1, V-attr `unset` (dummy file, then on real fixtures)
- [x] 1.3 `scripts/fixtures/lib.sh` (`die`, `cap` with argv regex guard)
- [x] 2.1–2.5 `export.py`, `ids.py`, `check.py`, `restore_mtimes.py` + `test_fixtures.py` (16 tests)
- [x] 3.1–3.4 `run-core.sh`, `verify.sh`, `scripts/gen-fixtures.sh`; 4 core stores generated
- [x] 4.1–4.4 CI C-fixtures steps, V-probe, `fixtures/README.md`, gates

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

Final: 16/16 unit tests OK on host Python 3.12 **and** Python 3.9.23 (inside `nextflow/nextflow:25.04.8`).

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

## Deviations

See design.md "Deviations (2026-11-26, apply PR A)".

## Remaining (PR B)

5.1–5.2 shape, 6.1–6.3 agent store, 7.1–7.3 regenerate + determinism, 8.1–8.3 workflow + docs.
