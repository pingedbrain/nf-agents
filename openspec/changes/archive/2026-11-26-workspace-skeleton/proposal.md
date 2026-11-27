# Proposal: Workspace skeleton (PR 1)

## Intent

No Rust code or CI exists yet. Every later stage-1 PR needs a Cargo workspace, shared lints and a secret-free CI gate. This is the first code PR (license and code of conduct already exist).

## Scope

### In Scope
- Root `Cargo.toml` workspace: resolver 3, edition 2024, shared `[workspace.package]` and `[workspace.lints]`.
- `rust-toolchain.toml` (stable + rustfmt + clippy) and `clippy.toml` (lint relaxations in tests).
- Stub library `crates/nf-lineage` (crate docs, "pre-alpha", one trivial test, no dependencies).
- `deny.toml` (licenses, advisories, bans, sources).
- `.github/workflows/ci.yml` with `lint` and `test` jobs.
- `CONTRIBUTING.md` (English).

### Out of Scope
- Any real `nf-lineage` code (types, parser, `LineageStore`) — PR 3+.
- Jobs `contract`, `integration`, `docker`, `fixtures`, `release` — later PRs.
- MSRV CI job, crates.io publishing, `docs/verification.md`.
- A `Makefile`/`justfile` (commands documented in CONTRIBUTING instead).

## Capabilities

### New Capabilities
- `rust-workspace`: workspace layout, toolchain, shared package metadata, lint policy, stub crate.
- `ci-pipeline`: PR/push CI gates (fmt, clippy, cargo-deny, cargo-machete, tests), permissions and supply-chain policy.
- `contributor-guide`: what CONTRIBUTING.md must tell contributors.

### Modified Capabilities
None.

> **Decision update (2026-11-26)**: the toolchain is pinned to the exact release `1.100.0` in `rust-toolchain.toml`, not the floating `stable` channel. The `stable` drift risk below is mitigated by the pin; toolchain bumps happen via normal PRs. See design.md (Decisions table and open questions).

## Approach

Minimal files, no dependencies. Lints declared once in the workspace and inherited with `[lints] workspace = true`; tests exempted via `clippy.toml` keys (verified on clippy 1.94). CI installs tools with `taiki-e/install-action` and runs the same commands contributors run locally.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `Cargo.toml`, `rust-toolchain.toml`, `clippy.toml` | New | Workspace + toolchain + lint config |
| `crates/nf-lineage/` | New | Stub library crate |
| `deny.toml` | New | cargo-deny policy |
| `.github/workflows/ci.yml` | New | CI |
| `CONTRIBUTING.md` | New | Contributor guide |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| `stable` channel drifts: CI (1.100) flags lints local 1.94 does not | Med | CONTRIBUTING says `rustup update stable` |
| cargo-deny config keys change between versions | Med | Generate from installed cargo-deny 0.22, run `cargo deny check` locally before PR |

## Rollback Plan

All files are new and nothing depends on them yet: revert the PR commit. No data or infra touched.

## Dependencies

- None (no crates). CI needs public GitHub Actions only.

## Success Criteria

- [ ] `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace`, `cargo deny check`, `cargo machete` pass locally.
- [ ] CI `lint` and `test` jobs green on the PR, no secrets used.
- [ ] Diff ≤ 800 changed lines.
