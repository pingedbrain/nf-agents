# Verification Report: lineage-fixtures

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
