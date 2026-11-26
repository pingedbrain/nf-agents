# Design: Workspace skeleton (PR 1)

## Technical Approach

Config-only change plus one stub crate. Policy is declared once at the workspace root and inherited; CI runs exactly the commands contributors run locally. No dependencies. Specs: `rust-workspace`, `ci-pipeline`, `contributor-guide`.

## Architecture Decisions

| Topic | Options | Tradeoff | Decision |
|-------|---------|----------|----------|
| Lint location | per-crate `[lints]` / `#![deny]` in `lib.rs` / `[workspace.lints]` | per-crate drifts | `[workspace.lints]`, members use `[lints] workspace = true` |
| Panic lints | task minimum (`unwrap_used`, `expect_used`, `panic`) / + `indexing_slicing`, `todo`, `unimplemented` / `clippy::pedantic` | pedantic is noisy; indexing/`todo!` also panic | minimum + `indexing_slicing`, `todo`, `unimplemented`, all `deny` |
| Test exemption | `#[allow]` per test module / `clippy.toml` `allow-*-in-tests` | per-module allows are boilerplate | `clippy.toml` `allow-{unwrap,expect,panic,indexing-slicing}-in-tests` (verified on clippy 1.94) |
| unsafe | `deny` / `forbid` | `forbid` can't be re-allowed | `unsafe_code = "forbid"` (no FFI planned) |
| Crate docs | none / `missing_docs = "warn"` | `warn` + CI `-D warnings` fails CI but not local iteration | `missing_docs = "warn"` |
| Toolchain | `stable` / exact `1.100.0` | `stable` lets CI pick up newer lints than a contributor's local toolchain; exact needs a bump PR per release | **Exact `channel = "1.100.0"`** (latest stable, 2026-11-09; orchestrator decision 2026-11-26), `profile = "minimal"`, components rustfmt+clippy. rustup auto-installs it per directory without touching the global default; bumps are normal PRs |
| MSRV | omit / `1.85` / `1.94` | 1.85 is the edition-2024 floor; untested in CI | `rust-version = "1.85"`; MSRV job deferred (open question) |
| CI toolchain install | `dtolnay/rust-toolchain@stable` / `rustup toolchain install` | dtolnay is used via branch refs (`@stable`; only a 2022 `v1` tag), breaking the no-branch-refs rule, and duplicates the toolchain file | plain `rustup toolchain install` (rustup ≥1.28 installs the active toolchain from `rust-toolchain.toml`) |
| Tool install | `EmbarkStudios/cargo-deny-action@v2` + `bnjbvr/cargo-machete` action / `taiki-e/install-action@v2` | two actions vs one step | `taiki-e/install-action@v2` (`tool: cargo-deny,cargo-machete`), then the local commands |
| Action pinning | major tag / SHA | SHA needs Dependabot | major tags (verified 2026-11-26): `actions/checkout@v7`, `Swatinem/rust-cache@v2`, `taiki-e/install-action@v2`; SHA + Dependabot later |
| Lockfile | ignore / commit + `--locked` | bins coming | commit `Cargo.lock`, CI uses `--locked` |
| Stub crate | empty workspace / `nf-lineage` stub | empty workspace doesn't build | `nf-lineage` `0.0.0`, `publish = false` |
| Local runner | `Makefile`/`justfile` / docs | 5 commands, no gain | commands in CONTRIBUTING only |

## Data Flow

PR / push to `main` → `ci.yml` → `lint` (fmt, clippy, deny, machete) and `test` in parallel; each: checkout → `rustup toolchain install` → rust-cache → commands.

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `Cargo.toml` | Create | workspace, resolver 3, `[workspace.package]`, `[workspace.lints]` |
| `Cargo.lock` | Create | generated, committed |
| `rust-toolchain.toml` | Create | exact `1.100.0`, minimal profile, rustfmt + clippy |
| `clippy.toml` | Create | 4 `allow-*-in-tests` keys |
| `crates/nf-lineage/Cargo.toml` | Create | inherits workspace fields, `publish = false`, `[lints] workspace = true` |
| `crates/nf-lineage/src/lib.rs` | Create | crate docs ("pre-alpha") + `version_is_pre_release` test |
| `deny.toml` | Create | licenses / advisories / bans / sources |
| `.github/workflows/ci.yml` | Create | `lint`, `test` |
| `CONTRIBUTING.md` | Create | contributor guide |

No `rustfmt.toml`. Estimated diff ≈ 300 lines.

## Interfaces / Contracts

Workspace root (shape, not final text):

```toml
[workspace]
resolver = "3"
members = ["crates/*"]

[workspace.package]
edition = "2024"
rust-version = "1.85"
license = "Apache-2.0"
repository = "https://github.com/pingedbrain/nf-agents"

[workspace.lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"

[workspace.lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
indexing_slicing = "deny"
todo = "deny"
unimplemented = "deny"
```

`deny.toml` (cargo-deny 0.22 schema; start from `cargo deny init`, delete removed/deprecated keys, confirm with `cargo deny check`):

- `[advisories]`: defaults (vulnerabilities deny), `yanked = "deny"`.
- `[licenses]`: `allow = ["Apache-2.0", "Apache-2.0 WITH LLVM-exception", "MIT", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Unicode-3.0", "Zlib"]`, `unused-allowed-license = "allow"` (no deps yet).
- `[bans]`: `multiple-versions = "warn"`, `wildcards = "deny"`, `allow-wildcard-paths = true` (future intra-workspace path deps).
- `[sources]`: `unknown-registry = "deny"`, `unknown-git = "deny"`, `allow-registry = ["https://github.com/rust-lang/crates.io-index"]`.

CI essentials: top-level `permissions: contents: read`; `concurrency: { group: ${{ github.workflow }}-${{ github.ref }}, cancel-in-progress: true }`; `actions/checkout@v7` with `persist-credentials: false`.

## Testing Strategy

| Layer | What | Approach |
|-------|------|----------|
| Unit | stub crate (T-1) | `version_is_pre_release` asserts `CARGO_PKG_VERSION` starts with `0.` |
| Lint policy | V-probe | TDD red: add `unwrap()` / `unsafe {}` / drop `//!`, see clippy fail, revert |
| Config | V-meta, V-yaml, V-deny, V-doc, V-links | shell/jq/grep checks run in verify |
| CI | C-run | the PR's own run must be green |

Strict TDD for config: each step starts from a failing command before the file is written.

## Migration / Rollout

No migration required. Branch protection requiring `lint`/`test` is a maintainer setting after merge.

## Open Questions

- [x] Toolchain `stable` vs exact pin? Resolved: exact `1.100.0` so local and CI clippy match; bump via PR (Dependabot/Renovate can automate later).

- [ ] Add an MSRV (1.85) CI job, or bump `rust-version` when real deps land?
- [ ] SHA-pin actions + Dependabot for `github-actions` before first release?
