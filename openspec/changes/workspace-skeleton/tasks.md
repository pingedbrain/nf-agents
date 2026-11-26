# Tasks: Workspace skeleton (PR 1)

Single PR, est. ≈300 changed lines (budget ≤ 800). Check IDs (T-, C-, V-) are defined in `specs/*/spec.md`. Strict TDD: every GREEN step is preceded by a command that fails.

## Phase 1: Infrastructure (workspace + stub)

- [x] 1.1 RED: run `cargo test --workspace` at repo root; confirm it fails (no workspace).
- [x] 1.2 RED: write `crates/nf-lineage/src/lib.rs` with only the `#[cfg(test)]` test `version_is_pre_release` (T-1).
- [x] 1.3 GREEN: create `Cargo.toml` (resolver 3, `members = ["crates/*"]`, `[workspace.package]`, `[workspace.lints]` per design), `rust-toolchain.toml`, `crates/nf-lineage/Cargo.toml` (`0.0.0`, `publish = false`, `[lints] workspace = true`); `cargo test --workspace` passes, `Cargo.lock` generated.
- [x] 1.4 RED→GREEN: `cargo clippy --workspace --all-targets --locked -- -D warnings` fails on `missing_docs`; add crate `//!` docs stating "pre-alpha"; passes.
- [x] 1.5 RED→GREEN (V-probe): add `.unwrap()` in a test; clippy fails; add `clippy.toml` with the 4 `allow-*-in-tests` keys; passes; remove probe.
- [x] 1.6 V-probe: temporarily add `.unwrap()` in non-test code, then `unsafe {}`; confirm clippy/compile fails each time; revert.
- [x] 1.7 `cargo fmt --all -- --check` passes (C-fmt); V-meta via `cargo metadata --no-deps --format-version 1 | jq` shows inherited fields and zero deps.

## Phase 2: Supply chain + CI

- [x] 2.1 RED: install `cargo-deny`, `cargo-machete`; `cargo deny check` fails (no `deny.toml`/policy).
- [x] 2.2 GREEN: create `deny.toml` (from `cargo deny init`, trimmed to design policy); `cargo deny check` passes (C-deny, V-deny); `cargo machete` clean (C-machete).
- [x] 2.3 Create `.github/workflows/ci.yml`: triggers, `permissions: contents: read`, concurrency, jobs `lint` and `test` with the exact commands from 1.4/1.7/2.2 and C-test; actions `checkout@v7` (`persist-credentials: false`), `rust-cache@v2`, `install-action@v2`; toolchain via `rustup toolchain install`.
- [x] 2.4 V-yaml: `grep -c 'secrets\.'` = 0; every `uses:` matches `@v[0-9]` or 40-hex SHA; if `actionlint` is available, run it.

## Phase 3: Docs

- [x] 3.1 RED: run V-doc greps (contributor-guide table) against missing `CONTRIBUTING.md`; all fail.
- [x] 3.2 GREEN: write `CONTRIBUTING.md` (8 required items; pre-PR commands copied verbatim from `ci.yml`; toolchain note — `rustup update stable` superseded by the exact pin in `rust-toolchain.toml`); V-doc greps all match.
- [x] 3.3 V-links: every relative link in `CONTRIBUTING.md` resolves (incl. `CODE_OF_CONDUCT.md`, `SECURITY.md`).

## Phase 4: Verification

- [x] 4.1 Run full local gate: C-fmt, C-clippy, C-test, C-deny, C-machete all exit 0.
- [x] 4.2 Check diff size: `git diff --stat` (incl. untracked) ≤ 800 lines.
- [ ] 4.3 After PR opens (orchestrator): C-run — `lint` and `test` green on the PR.
