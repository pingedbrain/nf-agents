# Tasks: release-pipeline (single PR, strict TDD)

## Phase 1: RED

- [ ] 1.1 Create `scripts/release/test_check.py`: stdlib unittest, temp workspaces via `tempfile`, runs `check.sh` via subprocess. Six cases from design Testing Strategy (R-check, R-tag, two-args exit 2).
- [ ] 1.2 Run `python3 -I -m unittest discover -s scripts/release -p 'test_*.py'`; confirm it FAILS on current tree (no script; repo-root case fails on `nf-llm-replay`).

## Phase 2: check.sh (GREEN for temp-workspace cases)

- [ ] 2.1 Create `scripts/release/check.sh` (bash, `set -euo pipefail`): `cargo metadata --no-deps` + `jq`; exit 1 on `publish = false`, empty description/license/repository/readme/keywords, version mismatch across members, tag != `v<version>`; exit 2 on >1 arg; exit 0. (R-check, R-tag)
- [ ] 2.2 Re-run unittest: temp-workspace cases pass; repo-root case still fails (expected until 3.1).

## Phase 3: nf-llm-replay crate

- [ ] 3.1 Modify `crates/nf-llm-replay/Cargo.toml`: drop `publish = false`; add `readme`, `keywords`, `categories`, `include = ["/src", "/README.md"]`. (R-check, R-pkg).
- [ ] 3.2 Create `crates/nf-llm-replay/README.md` (~25 lines, English). Covers readme requirement.
- [ ] 3.3 Verify GREEN: `scripts/release/check.sh` exits 0; full unittest passes.

## Phase 4: CI

- [ ] 4.1 Modify `.github/workflows/ci.yml` `test` job: add unittest step, `scripts/release/check.sh`, `cargo package --workspace --locked`; add `scripts/release/check.sh` to shellcheck line. No write permissions added. (ci-pipeline Test job, V-yaml).

## Phase 5: release.yml

- [ ] 5.1 Create `.github/workflows/release.yml`: `on: push: tags: ['v*']`, top-level `permissions: {}`, `concurrency: {group: release, cancel-in-progress: false}`, jobs `check` → `crates` → `images` (matrix of 2) → `release`. (R-yaml).
- [ ] 5.2 `check` job: `fetch-depth: 0`, fail unless `git merge-base --is-ancestor "$GITHUB_SHA" origin/main`; run `check.sh "$GITHUB_REF_NAME"`. (R-tag-main).
- [ ] 5.3 `crates` job: environment `release`; token step writes only `bootstrap=true` when `CARGO_REGISTRY_TOKEN` set; `rust-lang/crates-io-auth-action@v1` gated on that; `cargo publish --workspace --locked` with `steps.auth.outputs.token || secrets.CARGO_REGISTRY_TOKEN`. (R-yaml, R-crates).
- [ ] 5.4 `images` job: per-job permissions (`packages`, `id-token`, `attestations`, `artifact-metadata` write); `version=${GITHUB_REF_NAME#v}` via env; tags `:version`, plus `:latest` only without `-`; `docker build` with OCI version/revision labels; `docker push`; digest via `buildx imagetools inspect`; `actions/attest-build-provenance@v4`. (R-images).
- [ ] 5.5 `release` job: `contents: write`, no checkout; `gh release create "$GITHUB_REF_NAME" --verify-tag --generate-notes`, `--prerelease` when tag has `-`. (R-release).
- [ ] 5.6 Static review of `release.yml`: no `${{ }}` tag/matrix values inside `run:`; `persist-credentials: false` on checkouts. (R-yaml).

## Phase 6: Roadmap

- [ ] 6.1 Modify `ROADMAP.md`: check the release item (one line).

## Phase 7: Local verification (no publish, no push)

- [ ] 7.1 `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --locked -- -D warnings` (match `ci.yml` flags); `cargo test --workspace --locked`.
- [ ] 7.2 `python3 -I -m unittest discover -s scripts/release -p 'test_*.py'`; `scripts/release/check.sh` exits 0; `check.sh v9.9.9` exits 1; `check.sh v<current version>` exits 0.
- [ ] 7.3 `shellcheck demo/run.sh scripts/release/check.sh`.
- [ ] 7.4 `cargo package --workspace --locked`; `cargo package --list -p nf-llm-replay` has no `tests/` path.
- [ ] 7.5 `actionlint .github/workflows/ci.yml .github/workflows/release.yml` if installed; otherwise report skipped.

## Reconciliation notes (spec vs design)

- Attest action: `actions/attest-build-provenance@v4` in spec, design and proposal; no conflict.
- Tag-on-main: design and spec place it in `release.yml` `check` job; proposal's "safety" row is satisfied by that. Used `release.yml`.
- Categories: spec requires `categories` on every publishable crate, but design's `check.sh` list omits it. Followed design (check does not enforce categories). Gap to decide at apply/verify.

## Maintainer steps (outside this PR)

Environment `release`, crates.io token, tag ruleset, push `v0.1.0`, trusted publishers and token revoke, GHCR visibility (see proposal).
