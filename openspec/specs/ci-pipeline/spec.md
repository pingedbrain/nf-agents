# ci-pipeline Specification

## Purpose

GitHub Actions gates every PR and every push to `main` with lint and test jobs, a `nextflow lineage` contract job and an S3 integration job (MiniStack), none of which need secrets, plus the cargo-deny supply-chain policy.

## Verification checks

| ID | Kind | Command / name |
|----|------|----------------|
| C-run | CI | the change's own PR run shows `lint` and `test` green |
| C-deny | CI `lint` | `cargo deny check` |
| C-contract | CI `contract` | `cargo test -p nf-lineage --test contract --locked` |
| C-it | CI `integration` | `cargo test -p nf-lineage --test s3_ministack --locked` with MiniStack |
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

### Requirement: Contract job

Job `contract` MUST run on every PR and push to `main`, install the pinned toolchain, and run
`cargo test -p nf-lineage --test contract --locked` against the committed `fixtures/lineage/*` oracles.
It MUST NOT need Docker, network beyond crates.io, or secrets. (The `test` job also runs these
tests; `contract` exists as a named status check.)

#### Scenario: Oracle mismatch (V-yaml, C-contract)
- GIVEN a change that makes `find` return a different LID set for one fixture
- WHEN CI runs
- THEN `contract` fails naming the version and case

### Requirement: Integration job

Job `integration` MUST run on every PR and push to `main` with a service container
`ministackorg/ministack:1.5.45` on port 4566, set `NF_LINEAGE_MINISTACK_URL=http://localhost:4566`,
and run `cargo test -p nf-lineage --test s3_ministack --locked`. It MUST NOT use `secrets.*`: the tests
use fixed dummy credentials (`test`/`test`) in their own code, so the workflow defines none. The tests themselves MUST wait for
MiniStack readiness (bounded, ≤ 60 s) and fail, not skip, when the variable is set but MiniStack is unreachable.

#### Scenario: Service up (C-it, V-yaml)
- GIVEN the PR run
- WHEN `integration` runs
- THEN both `s3_ministack` tests execute (not skipped) and pass

#### Scenario: No secrets (V-yaml)
- GIVEN `ci.yml`
- THEN `grep -c 'secrets\.'` is 0 and the service image is pinned to `1.5.45`

### Requirement: Supply-chain policy

`deny.toml` MUST: allow only permissive licenses compatible with Apache-2.0 (no GPL/LGPL/AGPL/SSPL); deny crates with known vulnerabilities and yanked versions; deny wildcard version requirements; allow only the crates.io registry and no git sources. Licenses outside the allowlist MAY be accepted only as crate-scoped `exceptions` with a comment naming the crate and why. Advisories MAY be ignored only by explicit ID with a `reason` (initially RUSTSEC-2026-0542 `humantime` and RUSTSEC-2026-0543 `chrono`, unmaintained, pulled by `object_store`).

#### Scenario: Clean tree (C-deny)
- GIVEN the workspace as shipped
- WHEN `cargo deny check` runs
- THEN it exits 0

#### Scenario: Policy content (V-deny)
- GIVEN `deny.toml`
- WHEN inspected
- THEN the license allowlist has no copyleft ids, `unknown-registry` and `unknown-git` are `deny`, `allow-registry` is crates.io only

#### Scenario: Scoped exceptions (V-deny)
- GIVEN `deny.toml`
- WHEN inspected
- THEN every `exceptions` entry names one crate and has a comment, and every advisory ignore has an ID and `reason`
