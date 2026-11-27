# rust-workspace Specification

## Purpose

Cargo workspace, toolchain, shared metadata and lint policy that every nf-agents crate inherits. Crate behaviour lives in each crate's own capability (e.g. `nf-lineage`).

## Verification checks

| ID | Kind | Command / name |
|----|------|----------------|
| C-fmt | CI `lint` | `cargo fmt --all -- --check` |
| C-clippy | CI `lint` | `cargo clippy --workspace --all-targets --locked -- -D warnings` |
| C-test | CI `test` | `cargo test --workspace --locked` |
| C-machete | CI `lint` | `cargo machete` |
| V-meta | verify | `cargo metadata --no-deps --format-version 1` inspected with `jq` |
| V-probe | verify (TDD red step) | temporary offending line, run C-clippy, expect failure, revert |

## Requirements

### Requirement: Workspace builds and tests

The repository root MUST be a Cargo workspace (resolver `"3"`) with members under `crates/`, and `cargo test --workspace` MUST pass without secrets, with `Cargo.lock` committed.

#### Scenario: Clean checkout (C-test)
- GIVEN a fresh clone with the pinned toolchain
- WHEN `cargo test --workspace --locked` runs
- THEN it exits 0 and runs at least one test

#### Scenario: Stale lockfile (C-test)
- GIVEN a manifest change without an updated `Cargo.lock`
- WHEN `cargo test --workspace --locked` runs
- THEN it fails

### Requirement: Shared package metadata

Every member MUST inherit from `[workspace.package]`: edition `2024`, license `Apache-2.0`, repository `https://github.com/pingedbrain/nf-agents`, and `rust-version`.

#### Scenario: Metadata inherited (V-meta)
- GIVEN any workspace member
- WHEN its metadata is read
- THEN edition, license, repository and rust_version equal the workspace values

### Requirement: Lint policy

Every member MUST inherit `[workspace.lints]`. Non-test code MUST NOT compile under C-clippy if it uses `unsafe`, `unwrap()`, `expect()`, `panic!` or panicking index/slice access, or if the crate lacks crate-level docs. Test code (`#[cfg(test)]`, `tests/`) MAY use `unwrap`, `expect`, `panic!` and indexing.

#### Scenario: unwrap in library code (V-probe)
- GIVEN `.unwrap()` added to a non-test function
- WHEN C-clippy runs
- THEN it fails naming `clippy::unwrap_used`

#### Scenario: unsafe block (V-probe)
- GIVEN an `unsafe {}` block in any target
- WHEN the crate compiles
- THEN it fails on `unsafe_code` and `#[allow]` cannot override it

#### Scenario: unwrap in tests (C-clippy)
- GIVEN `.unwrap()` inside a `#[cfg(test)]` module
- WHEN C-clippy runs
- THEN it passes

#### Scenario: Missing crate docs (V-probe)
- GIVEN the crate-level `//!` docs removed
- WHEN C-clippy runs
- THEN it fails on `missing_docs`

### Requirement: Toolchain and formatting

`rust-toolchain.toml` MUST pin an exact stable release (e.g. `1.100.0`, not the floating `stable` channel) with `rustfmt` and `clippy` components, so local and CI lints match. All code MUST be rustfmt-formatted with default settings.

#### Scenario: Fresh rustup environment (C-fmt, C-clippy)
- GIVEN rustup with no toolchain preinstalled
- WHEN the toolchain is installed from `rust-toolchain.toml`
- THEN `cargo fmt` and `cargo clippy` are available

#### Scenario: Unformatted code (C-fmt)
- GIVEN a source file not matching rustfmt output
- WHEN C-fmt runs
- THEN it fails
