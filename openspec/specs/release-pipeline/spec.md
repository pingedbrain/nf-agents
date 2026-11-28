# release-pipeline Specification

## Purpose

Tag-driven release of nf-agents: crates.io (three crates, `0.1.0`), GHCR images for `nf-lineage-mcp` and `nf-llm-replay`, and a GitHub Release. `ci.yml` never publishes.

## Verification checks

| ID | Kind | Command / name |
|----|------|----------------|
| R-check | test (offline) | `scripts/release/check.sh` exits 0 on the tree; names the crate when `keywords` is missing |
| R-tag | test (offline) | `check.sh v9.9.9` exits non-zero; `check.sh v<version>` exits 0 |
| R-pkg | test (offline) | `cargo package --workspace --locked`; `--list` of `nf-llm-replay` has no `tests/` |
| R-yaml | verify (static) | inspect `.github/workflows/release.yml` |
| R-tag-main | verify (real tag) | tag on a commit not on `main`: `check` fails |
| R-crates | verify (real tag) | `v0.1.0`: three crates at `0.1.0` on crates.io |
| R-images | verify (real tag) | `v0.1.0`: both GHCR images tagged `0.1.0` and `latest`, attested |
| R-release | verify (real tag) | `v0.1.0`: Release with generated notes; `v0.1.0-rc.1`: prerelease, `latest` unchanged |

## Requirements

### Requirement: Publishable crate metadata

`nf-llm-replay` MUST NOT set `publish = false`, MUST declare `readme`, `keywords`, `categories` and `include = ["/src", "/README.md"]`, and ship a crate `README.md`. Every publishable crate MUST declare `readme`, `keywords` and `categories`.

#### Scenario: Metadata complete (R-check)
- GIVEN the workspace
- WHEN `scripts/release/check.sh` runs
- THEN it exits 0

#### Scenario: Missing keyword (R-check)
- GIVEN a publishable crate without `keywords`
- THEN `check.sh` exits non-zero and names the crate

#### Scenario: Test files excluded (R-pkg)
- WHEN `cargo package -p nf-llm-replay --list` runs
- THEN no path under `tests/` is listed

### Requirement: Tag check

`scripts/release/check.sh [tag]` MUST, given a tag, require the tag to equal `v<version>` for every publishable crate.

#### Scenario: Tag matches (R-tag)
- GIVEN all crate versions are `X.Y.Z`
- WHEN `check.sh vX.Y.Z` runs
- THEN it exits 0

#### Scenario: Tag mismatch (R-tag)
- GIVEN a crate at a version other than the tag
- THEN `check.sh` exits non-zero and names the crate

### Requirement: Release trigger and job order

`release.yml` MUST trigger only on `push` of tags `v*` and MUST define `check`, `crates`, `images`, `release`, each `needs` the previous. `check` MUST fail when the tag commit is not on `main`.

#### Scenario: Job gating (R-yaml)
- GIVEN `crates` fails
- THEN `images` and `release` do not run

#### Scenario: Tag off main (R-tag-main)
- GIVEN a `v*` tag on a commit not reachable from `main`
- THEN `check` fails and nothing is published

### Requirement: Crate publishing

The `crates` job MUST run in environment `release`, publish with `cargo publish --workspace`, and authenticate with crates.io Trusted Publishing (`rust-lang/crates-io-auth-action`). It MUST use `secrets.CARGO_REGISTRY_TOKEN` only as a fallback when that secret is set. No long-lived token MUST be committed.

#### Scenario: Trusted publishing (R-yaml)
- GIVEN no `CARGO_REGISTRY_TOKEN` secret
- THEN `crates` authenticates without a token

#### Scenario: Three crates published (R-crates)
- GIVEN tag `v0.1.0`
- THEN all three crates are at `0.1.0` on crates.io

### Requirement: Image publishing

The `images` job MUST build with `docker build` and push `ghcr.io/pingedbrain/nf-lineage-mcp` and `ghcr.io/pingedbrain/nf-llm-replay` with `docker push`. Tags MUST be `X.Y.Z`; `latest` MUST be applied only when the tag has no `-`. Provenance MUST be attached with `actions/attest-build-provenance`.

#### Scenario: Stable and prerelease tags (R-images, R-release)
- GIVEN tag `v0.1.0` THEN both images carry `0.1.0`, `latest` and a provenance attestation
- GIVEN tag `v0.1.0-rc.1` THEN images carry `0.1.0-rc.1` only, and the Release is a prerelease

### Requirement: GitHub Release

The `release` job MUST run `gh release create --generate-notes`, adding `--prerelease` when the tag contains `-`.

#### Scenario: Stable release (R-release)
- GIVEN tag `v0.1.0`
- THEN the Release exists and is not a prerelease

### Requirement: Least privilege per job

`release.yml` MUST NOT grant write scopes at workflow level; each job MUST declare only the `permissions` it uses.

#### Scenario: Permissions scoped (R-yaml)
- GIVEN `release.yml`
- THEN top-level `permissions` grants no write scope
