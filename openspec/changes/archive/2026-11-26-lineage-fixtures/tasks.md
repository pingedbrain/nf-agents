# Tasks: Lineage fixtures

Two PRs. Check IDs (P-, F-, G-, V-, C-) are defined in `specs/*/spec.md`. Strict TDD: every
GREEN is preceded by a failing command. Python is 3.9-compatible stdlib only. No Rust changes:
`cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`,
`cargo test --workspace --locked` must stay green in both PRs. Line counts exclude generated
`fixtures/lineage/**` and `openspec/`.

## PR A — core pipeline, generator, 4 core stores, offline check (≈ 780 lines)

### Phase 1: Foundation
- [x] 1.1 Port `fixtures/pipelines/minimal/{main.nf,nextflow.config,data/seed.txt}` from the lab (no host paths; Apache-2.0 header comment).
- [x] 1.2 `.gitattributes`: `fixtures/lineage/** -text`; `.gitignore`: `!/fixtures/lineage/*/work/`. V-ignore, V-attr (with a dummy file, then remove).
- [x] 1.3 `scripts/fixtures/lib.sh`: `die`, `cap` (argv regex guard, writes `.args/.out/.err/.exit`).

### Phase 2: Python helpers (RED → GREEN per test)
- [x] 2.1 RED `CasesTests.test_unsafe_arg_rejected` → GREEN via 1.3.
- [x] 2.2 RED `ExportTests.{test_symlink_dereferenced,test_only_data_files_kept,test_missing_data_file_fails}` → GREEN `scripts/fixtures/export.py` (`data_paths`, `export`, manifest with sha256, sorted).
- [x] 2.3 GREEN `scripts/fixtures/ids.py` (RUN1 = `greeting=hello`, `TASK_<NAME>_RUN<n>`; both envelopes) with RED `IdsTests.test_flat_and_spec_envelopes`.
- [x] 2.4 RED `CheckTests.{test_size_mismatch_fails,test_sha_mismatch_fails,test_unlisted_fileoutput_fails,test_extra_entry_fails,test_traversal_rejected,test_host_path_rejected,test_missing_case_file_fails,test_empty_root_fails}` → GREEN `scripts/fixtures/check.py` (core case list; `--forbid`).
- [x] 2.5 `scripts/fixtures/restore_mtimes.py` + RED/GREEN `RestoreTests.test_restores_ms_and_checks_size`.

### Phase 3: Generator (Docker)
- [x] 3.1 RED: `python3 -I scripts/fixtures/check.py fixtures/lineage` fails (no stores).
- [x] 3.2 `scripts/fixtures/run-core.sh` (2 runs, `ids.py`, 29 non-destructive cases, `export.py`, 2 tamper cases, chown) and `verify.sh` (no-restore check = 1, restore, check all FileOutputs, `list` == oracle).
- [x] 3.3 `scripts/gen-fixtures.sh`: args/tag regex, docker guard, pull + digest, staging (25.04 preview prepend), containers A/B with `--cpus 2`, `check.py --forbid`, atomic swap, cleanup; `--verify`, `--out`. V-nodocker, V-badtag, V-atomic (force a failing tag), V-preview.
- [x] 3.4 G-gen for the core matrix → commit `fixtures/lineage/{25.04.8,25.10.9,26.04.9,26.10.7}`; F-check GREEN; G-verify GREEN; `du -sh` each ≤ 100 KB.

### Phase 4: CI + docs
- [x] 4.1 `ci.yml` `test`: add C-fixtures (unittest discover + `check.py`). V-yaml: still `contents: read`, 0 `secrets.`.
- [x] 4.2 V-probe: flip one byte in a data file → C-fixtures fails naming it; revert.
- [x] 4.3 `fixtures/README.md` (provenance, synthetic, Apache-2.0, manifest + restore recipe, regenerate, never edit by hand). V-readme.
- [x] 4.4 Gates: cargo fmt/clippy/test, C-fixtures, `git diff --stat` ≤ 800 hand-written lines.

## PR B — shape, agent store, fixtures workflow (≈ 510 lines)

### Phase 5: Shape
- [x] 5.1 RED `ShapeTests.{test_volatile_tokens_masked,test_new_field_changes_shape,test_find_order_ignored}` → GREEN `scripts/fixtures/shape.py`.
- [x] 5.2 `check.py` requires `shape.txt` (RED test `CheckTests.test_missing_shape_fails`); `run-core.sh` writes it.

### Phase 6: Agent store
- [x] 6.1 RED `StubTests.{test_chat_completion_json_schema,test_unknown_path_501}` → GREEN `scripts/fixtures/llm_stub.py`.
- [x] 6.2 Port `fixtures/pipelines/agent/{main.nf,nextflow.config}` (langchain4j, `baseUrl` 127.0.0.1, pin `nf-agent@0.2.0`; resolve design open question).
- [x] 6.3 `scripts/fixtures/run-agent.sh` + agent case list in `check.py` (`kind: agent`, RED test `CheckTests.test_agent_cases`); wire into `gen-fixtures.sh` when 26.10.7 is requested.

### Phase 7: Regenerate + determinism
- [x] 7.1 G-gen all → commit 5 stores incl. `26.10.7-agent`; F-check, G-verify GREEN.
- [x] 7.2 G-shape: second G-gen leaves every `shape.txt` unchanged; discard the other diffs.
- [x] 7.3 G-edge locally (non-blocking; note result in PR body).

### Phase 8: Workflow + docs
- [x] 8.1 `.github/workflows/fixtures.yml` (dispatch `force`/`edge`, weekly cron, top `contents: read`, job-level write, concurrency, drift gate on `shape.txt`, `gh` PR create/edit, edge job `continue-on-error`). V-fx-yaml (+ `actionlint` if available).
- [x] 8.2 README: agent store, workflow, bot-PR CI caveat, repo setting.
- [x] 8.3 Gates as 4.4 (done); after merge (orchestrator, pending): C-fx-run with `force: false` → "no format drift".
