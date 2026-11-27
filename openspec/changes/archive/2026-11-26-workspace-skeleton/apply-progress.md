# Apply progress: workspace-skeleton

**Mode**: Strict TDD (runner `cargo test --workspace`). Batch 1 (only batch). 17/17 tasks done. 4.3 (C-run) completed by the orchestrator after the PR opened: PR https://github.com/pingedbrain/nf-agents/pull/1, CI run 45892532356, `lint` and `test` both success.

## Files changed

| File | Action | What |
|------|--------|------|
| `Cargo.toml` | Created | workspace, resolver 3, `[workspace.package]`, `[workspace.lints]` per design |
| `Cargo.lock` | Created | generated (lockfile v4, one package) |
| `rust-toolchain.toml` | Created | `channel = "1.100.0"`, minimal profile, rustfmt + clippy |
| `clippy.toml` | Created | 4 `allow-*-in-tests` keys |
| `crates/nf-lineage/Cargo.toml` | Created | `0.0.0`, `publish = false`, inherited fields, `[lints] workspace = true` |
| `crates/nf-lineage/src/lib.rs` | Created | pre-alpha crate docs + T-1 `version_is_pre_release` |
| `deny.toml` | Created | cargo-deny 0.22 policy (advisories/licenses/bans/sources) |
| `.github/workflows/ci.yml` | Created | `lint` + `test` jobs, `contents: read`, concurrency |
| `CONTRIBUTING.md` | Created | 8 required items, CI commands verbatim, openspec flow |
| `openspec/changes/workspace-skeleton/design.md` | Modified | toolchain decision → exact `1.100.0`; open question resolved |
| `openspec/changes/workspace-skeleton/specs/rust-workspace/spec.md` | Modified | toolchain requirement: exact stable pin instead of `stable` channel |

`.gitignore` (`.atl/`) was already modified before this batch; not touched.

## TDD cycle evidence

| Task | RED (failing first) | GREEN | Triangulate / probes | Refactor |
|------|---------------------|-------|----------------------|----------|
| 1.1 | `cargo test --workspace` → "could not find Cargo.toml" | — | — | — |
| 1.2 | T-1 written in `lib.rs` with no manifest → still fails | — | — | — |
| 1.3 | (from 1.2) | manifests + toolchain → `cargo test` 1 passed, `Cargo.lock` generated, rustc 1.100.0 | Stale-lockfile probe: version → 0.0.1, `--locked` fails (exit 101); reverted, lock unchanged | none needed |
| 1.4 | C-clippy fails: "missing documentation for the crate" | `//!` docs with "pre-alpha" → exit 0 | Docs-removed probe fails again (exit 101) | none |
| 1.5 | test with `unwrap`/`expect`/`panic!`/`v[0]` → 4 clippy errors without `clippy.toml` | `clippy.toml` → exit 0 (rechecked after `cargo clean`) | probe removed | none |
| 1.6 | non-test `unwrap` → `clippy::unwrap_used` error; `#[allow(unsafe_code)] unsafe {}` → E0453 (forbid cannot be overridden) + `unsafe_code` error | reverted, clean | — | — |
| 1.7 | — | C-fmt exit 0; V-meta: edition 2024, Apache-2.0, repo URL, rust_version 1.85, publish [], 0 deps | — | — |
| 2.1 | `cargo deny check` (no config) → licenses FAILED, exit 4 | — | — | — |
| 2.2 | (from 2.1) | `deny.toml` → "advisories ok, bans ok, licenses ok, sources ok", no config warnings; `cargo machete` clean | — | trimmed `cargo deny init` output to design policy |
| 2.3/2.4 | — | YAML parses; `secrets.` count 0; all `uses:` match `@vN`; actionlint (docker) exit 0; argless `rustup toolchain install` verified locally with rustup 1.29 | — | — |
| 3.1 | V-doc/sync script against missing `CONTRIBUTING.md` → all checks fail | — | — | — |
| 3.2/3.3 | (from 3.1) | V-doc, CI-command sync and V-links all pass | — | — |
| 4.1 | — | C-fmt, C-clippy (both forms), C-test, C-deny, C-machete all exit 0 | — | — |
| 4.2 | — | diff (excl. openspec/, Cargo.lock): 9 files, +187 lines ≤ 800 | — | — |
| 4.3 | — | CI run 45892532356 on PR #1: `lint` success, `test` success (C-run) | — | — |

Triangulation skipped for config tasks: purely structural, single possible output; probes above serve as the negative cases.

## Deviations from design

- Toolchain pinned to exact `1.100.0` instead of `stable` (orchestrator override); design + spec updated. CONTRIBUTING describes the pin instead of `rustup update stable`.
- No `cargo deny init` file committed verbatim: trimmed to the design policy (kept `confidence-threshold = 0.8`, `allow-git = []`).

## Issues found

- `cargo install` of cargo-machete 0.10.1 fails on rustc 1.94.1; installed with `cargo +1.100.0 install --locked`. CI uses `taiki-e/install-action` (prebuilt binaries), so unaffected.
- `rustup toolchain install` self-updated rustup 1.29.0 → 1.29.3 locally (global default toolchain unchanged: `stable`).
- `.github/pull_request_template.md` lists `cargo clippy --all-targets -- -D warnings` / `cargo test --workspace` without `--workspace`/`--locked`; not in scope, still passes.
