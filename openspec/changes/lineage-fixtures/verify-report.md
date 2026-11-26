# Verification Report: lineage-fixtures

**Scope**: PR A + PR B (all tasks 1.1–8.3) · **Mode**: Strict TDD · **Date**: 2026-11-26 · **Verdict (PR B)**: PASS WITH WARNINGS

## PR B (tasks 5.1–8.3)

### Completeness

| Metric | Value |
|--------|-------|
| Tasks total | 27 |
| Complete | 27 (8.3's post-merge C-fx-run belongs to the orchestrator) |
| Incomplete | 0 |

### Build & Tests Execution

| Command | Result |
|---------|--------|
| `cargo fmt --all -- --check` | ✅ |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | ✅ |
| `cargo test --workspace --locked` | ✅ (no Rust changes) |
| `cargo deny check` | ✅ advisories, bans, licenses, sources ok |
| `cargo machete` | ✅ |
| `python3 -I -m unittest discover -s scripts/fixtures -p 'test_*.py'` | ✅ 26 passed, 0 failed, 0 skipped (Python 3.12 host; 3.9.23 in `nextflow/nextflow:25.04.8`) |
| F-check | ✅ `ok` ×5 |
| G-gen | ✅ 5 stores, 5m40s |
| G-verify | ✅ 4× `10 FileOutputs`, agent `0 FileOutputs` |
| G-shape | ✅ 5/5 `shape.txt` identical across two full generations (after the find-comma fix) |
| G-edge | ✅ (non-blocking) `26.11.0-edge` generates; shape diff only in version-bearing cases |
| V-fx-yaml | ✅ triggers, permissions, 0 secrets, `actions/*` only; `actionlint` 1.7.7 clean |

**Coverage** (branch, unit only): shape 93%, export 92%, check 86%, llm_stub 82%, ids 65%, restore_mtimes 66%, total 84%. Uncovered: CLI `main()`s and error branches, exercised by G-gen/G-verify.

### TDD Compliance

| Check | Result | Details |
|-------|--------|---------|
| TDD Evidence reported | ✅ | apply-progress.md, PR B rows |
| RED confirmed | ✅ | stubs → 23 errors; F-check on committed stores → `missing shape.txt` ×4 |
| GREEN confirmed | ✅ | 26/26 |
| Triangulation | ✅ | masks (list line pair + exit codes), find vs non-find order, JSON-array find, stub schema vs plain, agent missing case vs missing tools |
| Trivial-pass guard | ✅ | first JSON-array test passed for the wrong reason; caught and rewritten to a real RED before the fix |

### Spec Compliance Matrix (PR B scope)

| Requirement | Scenario | Evidence | Result |
|-------------|----------|----------|--------|
| Shape fingerprint | Stable across regenerations | `ShapeTests.test_volatile_tokens_masked` + G-shape | ✅ |
| Shape fingerprint | Real drift detected | `ShapeTests.test_new_field_changes_shape`, `test_find_order_ignored` | ✅ |
| Version dir layout | `shape.txt` required | `CheckTests.test_missing_shape_fails`, `test_stale_shape_fails` | ✅ |
| AgentRun store | Offline agent run | `StubTests.test_chat_completion_json_schema`; G-gen: `find-agentrun.out` 3 LIDs, `qa_structured` output `{"answer": "stub-ok", "confidence": 0.5}`; no credentials passed to the container | ✅ |
| AgentRun store | Unsupported stub path | `StubTests.test_unknown_path_501` | ✅ |
| AgentRun store | Layout/manifest/oracle/shape rules | `CheckTests.test_agent_cases`, `IdsTests.test_agent_ids`, F-check, G-verify | ✅ |
| Machine-independent records | No host leakage (revised) | `CheckTests.test_host_path_shapes_only`, `test_host_path_rejected`; grep clean | ✅ |
| Git hygiene | No EOL conversion (agent store) | `text: unset` | ✅ |
| ci: Fixtures regeneration workflow | Triggers and permissions; Action allowlist | V-fx-yaml + actionlint | ✅ |
| ci: Regeneration opens a PR only on drift | Generation failure | steps gated on success (default) | ✅ (static) |
| ci: Regeneration opens a PR only on drift | No drift / Drift | C-fx-run | ⏳ after merge (orchestrator) |
| ci: Edge early warning | Edge drift reported | G-edge locally; job `continue-on-error`, `contents: read` | ✅ local / ⏳ C-fx-run |

**Compliance**: 11/11 locally verifiable scenarios compliant; 2 need C-fx-run after merge.

### Coherence (Design)

| Decision | Followed? | Notes |
|----------|-----------|-------|
| D1 shape fingerprint | ✅ | drift gate on `shape.txt` |
| D7 separate `26.10.7-agent/` | ✅ | |
| D8 stub in the container on 127.0.0.1 | ✅ | |
| D10 `gh` + `GITHUB_TOKEN`, no third-party action | ✅ | |
| Open question (plugin pin) | ✅ resolved | pin behaviour-neutral |
| Deviations | ⚠️ | host-leak by path shape + `$HOME`; stale-shape check; 3 agents; non-streaming stub; JSON-array comma; G-shape via `--out` — all in design.md "Deviations (apply PR B)" |

### Issues

**CRITICAL**: None.

**WARNING**:
- Repo setting "Allow GitHub Actions to create and approve pull requests" is off (`can_approve_pull_request_reviews: false`): on drift, `fixtures.yml` pushes `automation/lineage-fixtures` and then fails with an error naming the setting (by design, documented in `fixtures/README.md`).
- The drift and no-drift PR scenarios are only verifiable by C-fx-run after merge.

**SUGGESTION**:
- `CheckTests.test_empty_root_fails` prints `ok 26.10.7` to stdout during the test run (pre-existing noise).
- G-edge depends on a fixed default tag (`26.11.0-edge`); bump it when a newer edge ships.

---

## PR A (tasks 1.1–4.4), kept from the first verify

**Scope**: PR A (tasks 1.1–4.4) · **Mode**: Strict TDD · **Date**: 2026-11-26 · **Verdict**: PASS WITH WARNINGS

## Completeness

| Metric | Value |
|--------|-------|
| PR A tasks | 16 / 16 complete |
| PR B tasks (out of scope) | 0 / 11 (pending by plan) |

## Build & Tests Execution

| Command | Result |
|---------|--------|
| `cargo fmt --all -- --check` | ✅ |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | ✅ |
| `cargo test --workspace --locked` | ✅ (no Rust changes) |
| `cargo deny check` | ✅ advisories, bans, licenses, sources ok |
| `cargo machete` | ✅ |
| `python3 -I -m unittest discover -s scripts/fixtures -p 'test_*.py'` | ✅ 16 passed, 0 failed, 0 skipped (Python 3.12 host and 3.9.23 in `nextflow/nextflow:25.04.8`) |
| F-check `python3 -I scripts/fixtures/check.py fixtures/lineage` | ✅ `ok` ×4 |
| G-gen `scripts/gen-fixtures.sh` | ✅ 4 versions, 5m14s |
| G-verify `scripts/gen-fixtures.sh --verify` | ✅ 4× `round trip ok: 10 FileOutputs` |

**Coverage** (branch, `coverage.py`, unit tests only): check 83%, export 92%, ids 64%, restore_mtimes 66%, total 81%.
Uncovered lines are mostly the `main()` CLIs of `ids.py`/`restore_mtimes.py` and error branches; the CLIs are exercised by G-gen/G-verify in Docker.

## TDD Compliance

| Check | Result | Details |
|-------|--------|---------|
| TDD Evidence reported | ✅ | apply-progress.md table |
| All tasks have tests | ✅ | 2.1–2.5 unit; 3.x/4.x via G-*/V-* commands |
| RED confirmed | ✅ | stubs → 16 errors/failures; F-check on empty tree exit 1 |
| GREEN confirmed | ✅ | 16/16 pass now |
| Triangulation | ✅ | envelopes flat+spec; 4 unsafe argv; 2 traversal forms; size vs sha |
| Safety net for modified files | ➖ | only `.gitignore`/`ci.yml` modified (V-ignore, V-yaml) |

### Test Layer Distribution

| Layer | Tests | Tool |
|-------|-------|------|
| Unit (offline, CI) | 16 | unittest |
| Integration (Docker) | G-gen, G-verify, V-atomic, V-nodocker, V-badtag | bash + Docker |
| Repository (CI) | F-check | check.py |

### Assertion Quality

No tautologies; every negative test asserts on the specific message (path + reason). `CheckTests.test_valid_dir_passes` guards the negative tests against passing for the wrong reason.

## Spec Compliance Matrix (PR A scope)

| Requirement | Scenario | Evidence | Result |
|-------------|----------|----------|--------|
| Minimal pipeline | Every core version produces published outputs | G-gen + F-check (`no WorkflowOutput`, `< 2 published FileOutputs` checks) | ✅ |
| Minimal pipeline | 25.04 preview flag (V-preview) | committed `main.nf` line 1 is SPDX; 25.04.8 store has WorkflowOutput | ✅ |
| Generator interface | No Docker (V-nodocker) | `docker not found`, exit 1, nothing written | ✅ |
| Generator interface | Invalid tag (V-badtag) | `invalid version tag: latest; rm -rf /`, exit 1 | ✅ |
| Generator interface | Failure keeps old fixtures (V-atomic) | `99.99.99` (pull) and `24.10.5` (run fails): exit 1, dir byte-identical | ✅ |
| Machine-independent records | No host leakage | `CheckTests.test_host_path_rejected`, `test_forbidden_name_rejected`; F-check; grep clean | ✅ |
| Machine-independent records | Pool size pinned | F-check (`config.poolSize is not 2`) on real stores | ✅ |
| Version dir layout | Published outputs dereferenced | `ExportTests.test_symlink_dereferenced` | ✅ |
| Version dir layout | Work pruned | `ExportTests.test_only_data_files_kept` | ✅ |
| Version dir layout | Missing data file | `ExportTests.test_missing_data_file_fails` | ✅ |
| mtime manifest | Manifest matches files | `CheckTests.test_size_mismatch_fails`, `test_sha_mismatch_fails`; V-probe | ✅ |
| mtime manifest | Coverage | `CheckTests.test_unlisted_fileoutput_fails`, `test_extra_entry_fails` | ✅ |
| mtime manifest | Path traversal | `CheckTests.test_traversal_rejected` | ✅ |
| mtime manifest | Round trip | G-verify | ✅ |
| mtime manifest | Without restore | G-verify (`verify.sh` asserts exit 1 first) | ✅ |
| CLI oracle | Complete case set | `CheckTests.test_missing_case_file_fails`; F-check (31 cases) | ✅ |
| CLI oracle | Tamper isolated | `check-tampered.exit` = 1 in all 4; G-verify passes | ✅ |
| CLI oracle | Unsafe argv | `CasesTests.test_unsafe_arg_rejected` | ✅ |
| Git hygiene | Work files tracked (V-ignore) | `git check-ignore` exit 1 on real file | ✅ |
| Git hygiene | No EOL conversion (V-attr) | `text: unset` | ✅ |
| Git hygiene | README content (V-readme) | all 5 strings found | ✅ (README covers core stores; agent/workflow parts in PR B) |
| ci-pipeline: Test job | Corrupted fixture (C-fixtures) | V-probe: one byte → `sha256 does not match`, exit 1 | ✅ |
| ci-pipeline: Least privilege | Fork PR (V-yaml) | `contents: read`, 0 `secrets.` | ✅ |
| Shape fingerprint / AgentRun / fixtures.yml | — | PR B | ➖ out of scope |

**Compliance**: 23/23 in-scope scenarios compliant.

## Coherence (Design)

| Decision | Followed? | Notes |
|----------|-----------|-------|
| D1 shape fingerprint | ➖ | PR B |
| D2 dereference | ✅ | `shutil.copy2` follows symlinks |
| D3 FileOutput-only work files | ✅ | |
| D4 python3 in the image | ✅ | `command -v python3 || die` |
| D5 export before tamper, same container | ✅ | |
| D6 round trip on `cp -r` copy | ✅ | |
| D9 `ids.json` + `.args` JSON | ✅ | |
| D11 determinism knobs | ✅ | `--cpus 2`, fixed mount, default user, `NXF_ANSI_LOG=false`, no env passthrough |
| D12 drop HTML/logs | ✅ | render HTML to container `/tmp`; check rejects `.html`/`.log` |
| Deviations | ⚠️ | `sha256sum` instead of `cmp` (absent in images); `--forbid` check runs in the image; `__pycache__/` ignore — documented in design.md |

## Issues

**CRITICAL**: None.

**WARNING**:
- PR size: 927 changed lines incl. README (73) and the ported pipeline/seed (72); hand-written code + tests ≈ 782 (budget ~800).
- `--forbid <user>` is a plain substring match: a short or common user name (e.g. `runner` on GitHub Actions) could false-positive once the agent store (PR B) is generated in CI. Revisit in PR B.

**SUGGESTION**:
- Unit coverage of `ids.py`/`restore_mtimes.py` CLIs (64–66%) relies on the Docker integration run.

## Verdict

PASS WITH WARNINGS — PR A is complete, green on every gate, and the 4 core stores round-trip through `nextflow lineage check`.
