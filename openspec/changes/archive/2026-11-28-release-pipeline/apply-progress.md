# Apply progress: release-pipeline

Mode: Strict TDD (test runner: `python3 -I -m unittest discover -s scripts/release -p 'test_*.py'`, `cargo test --workspace`). Single batch; all tasks 1.1-7.5 complete. Branch `feat/release-pipeline`, nothing pushed.

## TDD Cycle Evidence

| Task | RED | GREEN | TRIANGULATE | SAFETY NET |
|------|-----|-------|-------------|------------|
| 1.1 / 1.2 `test_check.py` (9 cases) | Before `check.sh` existed: `Ran 9 tests ... FAILED (errors=9)`, each `FileNotFoundError: .../scripts/release/check.sh` | n/a (test-only commit) | n/a | N/A, new file |
| 2.1 `check.sh` (jq over `cargo metadata --no-deps`) | After script: 2 failures. `test_missing_readme_names_crate` failed because cargo auto-detects `README.md` when `readme` is unset; the fixture wrote one. Fixed the fixture (commit `7692be1`), not the script | 8/9 pass; only `test_repo_root_passes` fails (expected until 3.1, prints `nf-llm-replay: publish = false`) | 9 cases: prerelease tag match, tag mismatch, versions differ, missing readme / keywords / categories (each names the crate), `publish = false`, two args (exit 2), repo root | N/A, new file |
| 2.2 re-run | temp-workspace cases green | same as above | n/a | n/a |
| 3.1 `nf-llm-replay/Cargo.toml` | Repo-root case RED: `nf-llm-replay: publish = false`, exit 1 | Manifest updated: `publish` removed, `readme`, `keywords`, `categories`, `include`; `check.sh` exits 0; 9/9 unittest pass | Repo root plus the eight temp-workspace cases | `cargo test --workspace --locked` 142 passed, 0 failed (after all edits; no separate baseline captured before the manifest edit) |
| 3.2 crate `README.md` | n/a (docs) | `cargo package --list -p nf-llm-replay` includes `README.md` | Triangulation skipped: prose, no logic | n/a |
| 3.3 verify GREEN | n/a | `check.sh` exit 0; unittest OK | n/a | n/a |
| 4.1 `ci.yml` test job | n/a (config). Verified by running the same commands locally | actionlint (docker `rhysd/actionlint`) exit 0; `shellcheck` (docker `koalaman/shellcheck`) exit 0 | Triangulation skipped: declarative config | `ci.yml` diff is additive (+5/-1 lines); no `secrets.`, no `cargo publish` |
| 5.1-5.5 `release.yml` | n/a: design says workflow YAML is not unit-tested (R-yaml is static, R-tag-main/R-crates/R-images/R-release need a real tag). Deviation from the strict RED gate, flagged | actionlint exit 0; YAML parses; jobs `check`→`crates`→`images`→`release` with `needs` | Triangulation skipped: declarative config | n/a |
| 5.6 static review | n/a | Scripted check: no `${{ }}` inside any `run:`; every checkout has `persist-credentials: false`; top-level `permissions: {}`; write scopes per job only | n/a | n/a |
| 6.1 `ROADMAP.md` | n/a | one line checked | n/a | n/a |
| 7.1-7.5 local verification | see below | see below | n/a | n/a |

## Local verification (exit codes)

| Command | Exit |
|---------|------|
| `cargo fmt --all -- --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (142 passed, 0 failed) |
| `python3 -I -m unittest discover -s scripts/release -p 'test_*.py'` | 0 (9 tests) |
| `scripts/release/check.sh` | 0 (`release check ok: 0.1.0`) |
| `scripts/release/check.sh v0.1.0` | 0 |
| `scripts/release/check.sh v9.9.9` | 1 |
| `scripts/release/check.sh v0.1.1` | 1 |
| `scripts/release/check.sh v0.1.0 extra` | 2 |
| `shellcheck demo/run.sh scripts/release/check.sh` | 0 (docker image; `shellcheck` not installed on host) |
| `cargo package --workspace --locked` | 0 |
| `cargo package --list -p nf-llm-replay --locked` | 0; lists `Cargo.toml(.orig)`, `Cargo.lock`, `.cargo_vcs_info.json`, `README.md`, `src/*.rs`; no `tests/`, no `Dockerfile`, no `examples/` |
| `actionlint .github/workflows/ci.yml .github/workflows/release.yml` | 0 (docker `rhysd/actionlint:latest`) |

## Deviations from design

- `check.sh` also rejects an empty `categories` (orchestrator decision, matches spec). One extra test case.
- `check.sh` fails on any member with `publish = false` (design's wording), not only on publishable crates.
- Cargo auto-detects `README.md` when `readme` is unset, so a crate that ships a README without declaring `readme` passes `check.sh`. Metadata cannot show whether the key was written. Known gap, see risks.

## Risks

- Missing-`readme` detection only works when no `README.md` sits next to the manifest (cargo's auto-detection). Currently all three crates declare `readme`, so no impact on `v0.1.0`.
- `release.yml` is unexecuted: R-tag-main, R-crates, R-images, R-release and the trusted-publisher setup need a real tag and maintainer steps (see proposal rollout).
- `cargo publish --workspace` on a partial failure leaves already-published crates. Design says fix forward with a patch version.
- `shellcheck` and `actionlint` ran through Docker images, not host binaries.
