# Archive Report: release-pipeline

**Date**: 2026-11-28
**Artifact store**: openspec
**Verdict at archive**: PASS WITH WARNINGS (verify-report.md: 0 CRITICAL; tasks 1.1-7.5 (20/20) complete; 13 of 18 scenario rows COMPLIANT (static or executed), 1 PARTIAL, 4 PENDING a real `v0.1.0` tag)

## Evidence

- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked` (142 passed) pass in verify.
- `python3 -I -m unittest discover -s scripts/release -p 'test_*.py'` (9 passed); demo unittest and fixture check pass.
- `scripts/release/check.sh`: 0 on tree, 0 for `v0.1.0`, 1 for `v0.1.1` and `v9.9.9`, 2 for two args.
- `cargo package --workspace --locked` passes; `nf-llm-replay` package has no `tests/`.
- actionlint (`ci.yml`, `release.yml`) and shellcheck (`demo/run.sh`, `scripts/release/check.sh`) pass via Docker.
- Static review of `release.yml`: job order and gating, trusted publishing with token fallback, per-job permissions, `permissions: {}` at top level.
- No publish, tag, push, or PR was performed.

## Specs synced to openspec/specs/

| Domain | Action | Details |
|--------|--------|---------|
| release-pipeline | Created (new capability) | Copied from the change (no prior main spec); 6 requirements |
| ci-pipeline | Updated | MODIFIED `Test job` replaced: adds `check.sh` (R-check), `cargo package --workspace --locked` (R-pkg), and no-publish scenarios; `(Previously ...)` note dropped |

## Carried-forward items (from verify-report.md)

- Real-tag verification before or at the `v0.1.0` push: R-tag-main, R-crates, R-images, R-release.
- W1: cargo auto-detects `README.md`, so `check.sh` cannot detect a missing `readme` key when a README sits beside the manifest.
- W2: `v0.1.0-rc.1` cannot pass `check.sh` while the workspace is at `0.1.0`; the prerelease path is verified only by reading the YAML.
- W3: tag-mismatch scenario does not assert the crate name in an automated test.
- W4, W5: Strict TDD RED gap for config tasks; no baseline run before the `Cargo.toml` edit.
- Suggestions S1-S4 in verify-report.md (test step before `crates` publish, local `docker build` before the first tag, automated name check, automated `tests/` exclusion check).
- Maintainer steps in proposal.md (environment `release`, crates.io token and trusted publishers, tag ruleset, push `v0.1.0`, GHCR visibility) remain outside this change.

## SDD Cycle Complete

Proposal, specs, design, tasks, apply, and verify are complete. Change archived; main specs updated.
