# ci-pipeline Specification

## Purpose

GitHub Actions gates every PR and every push to `main` with lint and test jobs that need no secrets, plus the cargo-deny supply-chain policy.

## Verification checks

| ID | Kind | Command / name |
|----|------|----------------|
| C-run | CI | the change's own PR run shows `lint` and `test` green |
| C-deny | CI `lint` | `cargo deny check` |
| V-yaml | verify | inspect `.github/workflows/ci.yml` (grep/yq) |
| V-deny | verify | inspect `deny.toml` + run `cargo deny check` locally |

## Requirements

### Requirement: Triggers and concurrency

The workflow MUST run on `pull_request` and on `push` to `main`. A newer run for the same ref MUST cancel the in-progress one.

#### Scenario: PR opened (C-run)
- GIVEN a PR against `main`
- WHEN it is opened or updated
- THEN `lint` and `test` run

#### Scenario: Rapid pushes (V-yaml)
- GIVEN two pushes to the same PR in quick succession
- WHEN the second run starts
- THEN the first is cancelled (`concurrency` keyed on workflow + ref, `cancel-in-progress: true`)

### Requirement: Least privilege, no secrets

The workflow MUST declare `permissions: contents: read` at top level and MUST NOT reference `secrets.*`. It MUST NOT need API keys or cloud credentials.

#### Scenario: Fork PR (V-yaml, C-run)
- GIVEN a PR from a fork (no secrets available)
- WHEN CI runs
- THEN both jobs can pass; `grep -c 'secrets\.' ci.yml` is 0

### Requirement: Lint job

Job `lint` MUST run, and fail on failure of any of: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo deny check`, `cargo machete`.

#### Scenario: All clean (C-run)
- GIVEN code satisfying rust-workspace
- WHEN `lint` runs
- THEN all four steps pass

#### Scenario: Unused dependency (V-yaml)
- GIVEN a member declaring a dependency it never uses
- WHEN `cargo machete` runs
- THEN it exits non-zero and `lint` fails

### Requirement: Test job

Job `test` MUST run `cargo test --workspace --locked` and fail if any test fails.

#### Scenario: Failing test (C-run, V-yaml)
- GIVEN a failing test in any member
- WHEN `test` runs
- THEN the job fails

### Requirement: Toolchain source and action pinning

CI MUST install the toolchain from `rust-toolchain.toml` (single source of truth). Every third-party action MUST be referenced by a version tag (`@vN` or more specific) or a commit SHA, never a branch. Cargo build artifacts SHOULD be cached.

#### Scenario: Action refs (V-yaml)
- GIVEN every `uses:` line
- WHEN inspected
- THEN each ref matches `@v[0-9]…` or a 40-hex SHA

### Requirement: Supply-chain policy

`deny.toml` MUST: allow only permissive licenses compatible with Apache-2.0 (no GPL/LGPL/AGPL/SSPL); deny crates with known vulnerabilities and yanked versions; deny wildcard version requirements; allow only the crates.io registry and no git sources.

#### Scenario: Clean tree (C-deny)
- GIVEN the workspace as shipped
- WHEN `cargo deny check` runs
- THEN it exits 0

#### Scenario: Policy content (V-deny)
- GIVEN `deny.toml`
- WHEN inspected
- THEN the license allowlist has no copyleft ids, `unknown-registry` and `unknown-git` are `deny`, `allow-registry` is crates.io only
