# Verification Report: release-pipeline

**Change**: release-pipeline
**Version**: release-pipeline and ci-pipeline delta specs (spec.md)
**Mode**: Strict TDD (`cargo test --workspace`; `python3 -I -m unittest discover -s scripts/release -p 'test_*.py'`)
**Branch**: feat/release-pipeline (HEAD b753984)
**Date**: 2026-11-28

---

## Completeness

| Metric | Value |
|--------|-------|
| Tasks total | 20 |
| Tasks complete | 20 |
| Tasks incomplete | 0 |

---

## Build & Tests Execution

**Format**: ✅ `cargo fmt --all -- --check` exit 0
**Lint**: ✅ `cargo clippy --workspace --all-targets --locked -- -D warnings` exit 0
**Build (package)**: ✅ `cargo package --workspace --locked` exit 0
**Tests (Rust)**: ✅ `cargo test --workspace --locked` 142 passed / 0 failed
**Tests (release unittest)**: ✅ 9 passed / 0 failed / 0 skipped
**Tests (demo unittest)**: ✅ `python3 -I -m unittest discover -s demo -p 'test_*.py'` OK
**Fixtures**: ✅ `python3 -I scripts/fixtures/check.py fixtures/lineage` exit 0
**Coverage**: ➖ Not available (no coverage tool configured)

### Release gate (scripts/release/check.sh)
| Invocation | Expected | Actual |
|------------|----------|--------|
| `check.sh` | 0 | 0 (`release check ok: 0.1.0`) |
| `check.sh v0.1.0` | 0 | 0 |
| `check.sh v0.1.1` | 1 | 1 (names nf-lineage, nf-lineage-mcp, nf-llm-replay) |
| `check.sh v9.9.9` | 1 | 1 (names all three) |
| `check.sh v0.1.0 extra` | 2 | 2 |

### Packaging
- `cargo package --list -p nf-llm-replay --locked`: no `tests/` path; includes `README.md`, `src/*.rs`, cargo-generated files.

### Linters (Docker)
- ✅ `actionlint .github/workflows/ci.yml .github/workflows/release.yml` (rhysd/actionlint:latest) exit 0
- ✅ `shellcheck demo/run.sh scripts/release/check.sh` (koalaman/shellcheck:stable) exit 0

---

## TDD Compliance

| Check | Result | Details |
|-------|--------|---------|
| TDD Evidence reported | ✅ | Found in apply-progress.md |
| All tasks have tests | ⚠️ | Code tasks 1.1-3.1 have tests; 3.2, 4.1, 5.1-5.6, 6.1 declared N/A (docs or declarative config) |
| RED confirmed (tests exist) | ✅ | `scripts/release/test_check.py` exists; RED run recorded (9 errors before script existed) |
| GREEN confirmed (tests pass) | ✅ | 9/9 unittest pass on execution |
| Triangulation adequate | ✅ | 9 cases covering pass, fail, and exit-2 paths |
| Safety Net for modified files | ⚠️ | `Cargo.toml` edit (3.1) had no separate baseline run; 142 Rust tests green after all edits |

**TDD Compliance**: 4/6 checks fully passed, 2 WARNING

### Assertion Quality (Step 5f)
No tautologies, no ghost loops, no mock-heavy tests. Two gaps in assertion strength (see WARNINGs W3):
- `test_tag_without_prerelease_suffix_fails` asserts exit code only, not the crate-naming message.

**Assertion quality**: 0 CRITICAL, 1 WARNING

### Test Layer Distribution
| Layer | Tests | Files | Tools |
|-------|-------|-------|-------|
| Unit | 9 | 1 | unittest (`scripts/release/test_check.py`, subprocess against check.sh) |
| Integration | — | — | not installed |
| E2E | — | — | not installed |
| **Total** | **9** | **1** | |

### Quality Metrics
**Linter**: ✅ clippy and actionlint/shellcheck clean
**Type Checker**: ➖ `cargo check` covered by clippy (not run separately)

---

## Spec Compliance Matrix

### release-pipeline

| Requirement | Scenario | Test / Evidence | Result |
|-------------|----------|-----------------|--------|
| Publishable crate metadata | Metadata complete (R-check) | `test_repo_root_passes`; `check.sh` exit 0 | ✅ COMPLIANT |
| Publishable crate metadata | Missing keyword (R-check) | `test_missing_keywords_names_crate` | ✅ COMPLIANT |
| Publishable crate metadata | Test files excluded (R-pkg) | `cargo package --list -p nf-llm-replay` (command run, no `tests/`) | ✅ COMPLIANT (command evidence, no automated test) |
| Tag check | Tag matches (R-tag) | `test_matching_prerelease_tag_passes`; `check.sh v0.1.0` exit 0 | ✅ COMPLIANT |
| Tag check | Tag mismatch (R-tag) | `test_tag_without_prerelease_suffix_fails` (exit 1 only); naming verified by manual run | ⚠️ PARTIAL |
| Release trigger and job order | Job gating (R-yaml) | Static review of `release.yml` `needs` chain | ✅ COMPLIANT (static) |
| Release trigger and job order | Tag off main (R-tag-main) | Real tag required | ⏳ PENDING |
| Crate publishing | Trusted publishing (R-yaml) | Static review: auth action gated on no token | ✅ COMPLIANT (static) |
| Crate publishing | Three crates published (R-crates) | Real tag `v0.1.0` required | ⏳ PENDING |
| Image publishing | Stable and prerelease tags (R-images) | Real tag required | ⏳ PENDING |
| GitHub Release | Stable release (R-release) | Real tag required | ⏳ PENDING |
| Least privilege per job | Permissions scoped (R-yaml) | Static review: `permissions: {}` top level, per-job grants | ✅ COMPLIANT (static) |

### ci-pipeline (MODIFIED Test job)

| Requirement | Scenario | Test / Evidence | Result |
|-------------|----------|-----------------|--------|
| Test job | Failing test (C-run, V-yaml) | `ci.yml` runs `cargo test --workspace --locked`; executed, 142 passed | ✅ COMPLIANT |
| Test job | Corrupted fixture (C-fixtures) | `fixtures/lineage` check executed, exit 0 | ✅ COMPLIANT |
| Test job | Broken tour logic (C-demo-unit) | `TourTests.test_unexpected_diff_fails` present and passing | ✅ COMPLIANT |
| Test job | Missing crate keyword (R-check) | `ci.yml` step `scripts/release/check.sh` plus `test_missing_keywords_names_crate` | ✅ COMPLIANT |
| Test job | Unpackageable crate (R-pkg) | `ci.yml` step `cargo package --workspace --locked`; executed, exit 0 | ✅ COMPLIANT |
| Test job | No publish from CI (V-yaml) | grep `ci.yml` for `secrets.`, `cargo publish`, `CARGO_REGISTRY_TOKEN`: no match | ✅ COMPLIANT |

**Compliance summary**: 13/18 scenario rows compliant (static or executed), 1 partial (tag mismatch naming), 4 pending a real tag (R-tag-main, R-crates, R-images, R-release), 0 failing, 0 untested.

---

## Correctness (Static — Structural Evidence)

| Requirement | Status | Notes |
|------------|--------|-------|
| Publishable crate metadata | ✅ Implemented | `nf-llm-replay/Cargo.toml` has no `publish = false`; readme, keywords, categories, include present; README.md shipped |
| Tag check | ✅ Implemented | `check.sh` tag equality per crate; exit 1 names crate |
| Release trigger and job order | ✅ Implemented | `on: push: tags: ['v*']`; check → crates → images → release |
| Crate publishing | ✅ Implemented | environment `release`; `rust-lang/crates-io-auth-action@v1` gated; `--workspace --locked`; token fallback only via `secrets.CARGO_REGISTRY_TOKEN` |
| Image publishing | ✅ Implemented | `docker build`/`docker push`; `:latest` only without `-`; `actions/attest-build-provenance@v4` |
| GitHub Release | ✅ Implemented | `gh release create --verify-tag --generate-notes`, `--prerelease` when tag has `-` |
| Least privilege per job | ✅ Implemented | top-level `permissions: {}`; `release` alone has `contents: write` |
| ci-pipeline Test job | ✅ Implemented | ci.yml test job adds unittest, check.sh, cargo package, shellcheck line; no publish or secrets |

---

## Coherence (Design)

| Decision | Followed? | Notes |
|----------|-----------|-------|
| Trusted publishing with token bootstrap via `$GITHUB_OUTPUT` boolean | ✅ Yes | token never echoed |
| `cargo metadata --no-deps` + jq in check.sh | ✅ Yes | |
| On-main check in release.yml, not check.sh | ✅ Yes | `fetch-depth: 0` |
| Plain docker build/push, labels for version and revision | ✅ Yes | |
| `include = ["/src", "/README.md"]` for nf-llm-replay | ✅ Yes | |
| File Changes table | ⚠️ Deviated (minor) | `ROADMAP.md` line carries an extra "In progress" note (intended per orchestrator) |
| check.sh rejects empty `categories` | ⚠️ Deviated (intended) | Added to match spec; recorded in tasks.md reconciliation notes |
| check.sh rejects any member with `publish = false` | ⚠️ Deviated (minor) | Design said publishable crates only; apply recorded this |

---

## Issues Found

**CRITICAL** (must fix before archive):
None.

**WARNING** (should fix):
- **W1 — readme detection gap**: cargo auto-detects `README.md` when `readme` is unset, so `check.sh` cannot detect a missing `readme` declaration for a crate that ships a README next to its manifest. Spec requirement "Every publishable crate MUST declare readme" is only partially enforced. All three crates currently declare it, so no impact on v0.1.0. Known gap per apply-progress.md.
- **W2 — rc tag scenario unreachable without a version bump**: `check.sh` requires `tag == v<version>`, so `v0.1.0-rc.1` fails while the workspace is at `0.1.0`. The spec scenario for prerelease (images `0.1.0-rc.1` only, prerelease Release) is achievable only by bumping the workspace to the rc version, which publishes rc crates to crates.io. Scenario is ambiguous; the `--prerelease` path is verified only by reading the YAML.
- **W3 — tag-mismatch naming not automated**: spec R-tag mismatch says `check.sh` "names the crate"; `test_tag_without_prerelease_suffix_fails` asserts exit 1 only. Naming confirmed manually (`check.sh v9.9.9`).
- **W4 — Strict TDD RED gap for config tasks**: tasks 4.1, 5.1-5.6 have no RED evidence (declared N/A: declarative YAML). Apply flagged this as a deviation from the strict gate. Accepted by design (R-tag-main, R-crates, R-images, R-release need a real tag).
- **W5 — safety net not captured for `Cargo.toml` edit (3.1)**: no baseline `cargo test` run before the manifest change; only the post-change 142-pass run exists.

**SUGGESTION** (nice to have):
- **S1**: `crates` job publishes without running tests; relies on the `test` job in ci.yml on PRs. Consider a `cargo test --workspace --locked` step before publish for defense in depth on tags.
- **S2**: Run `docker build -f crates/nf-lineage-mcp/Dockerfile .` and `crates/nf-llm-replay/Dockerfile` locally (no push) before the first tag; the image build is only verified by the real tag.
- **S3**: Add an automated assertion that the tag-mismatch output names the crate (closes W3).
- **S4**: Automate the `cargo package --list` no-`tests/` check as a unittest, so R-pkg does not rely on manual runs.

---

## Real-tag items (PENDING, not failures)

R-tag-main, R-crates, R-images, R-release require a real `v0.1.0` tag and the maintainer steps in proposal.md. Not verified here, by design. No tag, push, or publish was performed.

Intended state: ROADMAP.md release item stays unchecked with "In progress" until 0.1.0 is published.

---

## Verdict

**PASS WITH WARNINGS**

All static, offline, and command-based checks pass (fmt, clippy, 142 Rust tests, 9 release unittests, demo and fixture checks, check.sh matrix, cargo package, actionlint, shellcheck). No CRITICAL issues. Four real-tag scenarios remain PENDING by design.
