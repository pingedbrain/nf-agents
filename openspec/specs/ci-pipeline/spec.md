# ci-pipeline Specification

## Purpose

GitHub Actions gates every PR and every push to `main` with five jobs in `ci.yml`, none of which need secrets: `lint`, `test` (including the offline fixture and demo checks), a `nextflow lineage` contract job, an S3 integration job (MiniStack) and a Docker job that builds and smoke-tests the container images and runs the local demo. A separate `fixtures.yml` workflow (manual and weekly) regenerates the lineage fixtures and warns about edge-release drift. `deny.toml` holds the cargo-deny supply-chain policy.

## Verification checks

| ID | Kind | Command / name |
|----|------|----------------|
| C-run | CI | the change's own PR run shows `lint` and `test` green |
| C-deny | CI `lint` | `cargo deny check` |
| C-contract | CI `contract` | `cargo test -p nf-lineage --test contract --locked` |
| C-it | CI `integration` | `cargo test -p nf-lineage --test s3_ministack --locked` with MiniStack |
| C-docker | CI `docker` | the PR run shows `docker` green: D-build, D-user, D-size, D-run, D-e2e of llm-replay-packaging and D-build, D-user, D-size, D-smoke of lineage-mcp |
| C-fixtures | CI `test` | `python3 -I -m unittest discover -s scripts/fixtures -p 'test_*.py'` and `python3 -I scripts/fixtures/check.py fixtures/lineage` |
| C-demo-unit | CI `test` | DM-unit and DM-sh of local-demo |
| C-demo | CI `docker` | the PR run shows the step `Local demo` green (DM-run of local-demo) |
| C-fx-run | CI (manual) | a `workflow_dispatch` run of `fixtures.yml` |
| V-yaml | verify | inspect `.github/workflows/ci.yml` (grep/yq) |
| V-fx-yaml | verify | inspect `.github/workflows/fixtures.yml` (grep/yq; `actionlint` if available) |
| V-deny | verify | inspect `deny.toml` + run `cargo deny check` locally |

## Requirements

### Requirement: Triggers and concurrency

The workflow MUST run on `pull_request` and on `push` to `main`. A newer run for the same ref MUST cancel the in-progress one.

#### Scenario: PR opened (C-run)
- GIVEN a PR against `main`
- WHEN it is opened or updated
- THEN `lint`, `test`, `contract`, `integration` and `docker` run

#### Scenario: Rapid pushes (V-yaml)
- GIVEN two pushes to the same PR in quick succession
- WHEN the second run starts
- THEN the first is cancelled (`concurrency` keyed on workflow + ref, `cancel-in-progress: true`)

### Requirement: Least privilege, no secrets

`ci.yml` MUST declare `permissions: contents: read` at top level and MUST NOT reference `secrets.*`. It MUST NOT need API keys or cloud credentials. (`fixtures.yml` is governed by "Fixtures regeneration workflow".)

#### Scenario: Fork PR (V-yaml, C-run)
- GIVEN a PR from a fork (no secrets available)
- WHEN CI runs
- THEN all five `ci.yml` jobs (`lint`, `test`, `contract`, `integration`, `docker`) can pass; `grep -c 'secrets\.' ci.yml` is 0

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

Job `test` MUST run `cargo test --workspace --locked`, the offline fixture checks (C-fixtures) and the demo checks (C-demo-unit: tour unittest and `shellcheck demo/run.sh`). It MUST fail if any of them fails. The fixture and demo checks MUST NOT need Docker or network.

#### Scenario: Failing test (C-run, V-yaml)
- GIVEN a failing test in any member
- WHEN `test` runs
- THEN the job fails

#### Scenario: Corrupted fixture (C-fixtures)
- GIVEN a PR that changes one byte of a data file under `fixtures/lineage/`
- WHEN `test` runs
- THEN the job fails naming the file

#### Scenario: Broken tour logic (C-demo-unit)
- GIVEN a PR that makes `tour.py` accept a diff with extra changes
- WHEN `test` runs
- THEN `TourTests.test_unexpected_diff_fails` fails the job

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

### Requirement: Docker job

Job `docker` MUST run on every PR and every push to `main`, on `ubuntu-latest`, with no secrets and no registry login or push. It MUST:
- build `crates/nf-llm-replay/Dockerfile` from the repo root as `nf-llm-replay:ci`, fail if the image user is not `65532:65532` or the image exceeds 25 MB, and run `docs/examples/nf-llm-replay/e2e.sh` with `E2E_REQUIRE_DOCKER=1` and `IMAGE=nf-llm-replay:ci`;
- build `crates/nf-lineage-mcp/Dockerfile` as `nf-lineage-mcp:ci`, fail if its user is not `65532:65532` or it exceeds 30 MB, and run the D-smoke stdio session against `fixtures/lineage/26.10.7/.lineage` mounted read-only;
- as its last step, run `demo/run.sh` with `MCP_IMAGE=nf-lineage-mcp:ci` and `REPLAY_IMAGE=nf-llm-replay:ci` (no image rebuild) and fail if the script fails or its stdout lacks the `claude mcp add` hand-off.

It MUST NOT depend on `lint`/`test`, so it runs in parallel with them. It SHOULD finish in under 10 minutes (`timeout-minutes: 20`).

#### Scenario: PR touching the proxy (C-docker)
- GIVEN a PR
- WHEN CI runs
- THEN `docker` builds `nf-llm-replay:ci` and the offline e2e passes

#### Scenario: PR touching the MCP server (C-docker)
- GIVEN a PR
- WHEN CI runs
- THEN `docker` builds `nf-lineage-mcp:ci` and the stdio smoke returns 2 run LIDs

#### Scenario: README demo cannot rot (C-demo)
- GIVEN a PR that breaks any piece the demo uses (pipeline, cassette, a tool contract, an image)
- WHEN `docker` runs
- THEN the `Local demo` step fails

#### Scenario: Broken image (V-yaml)
- GIVEN a Dockerfile change that runs as root
- WHEN `docker` runs
- THEN the user check fails the job

#### Scenario: Never publishes (V-yaml)
- GIVEN the `docker` job definition
- WHEN inspected
- THEN it contains no `docker push`, `docker login`, `secrets.` or `packages: write`

### Requirement: Fixtures regeneration workflow

`.github/workflows/fixtures.yml` MUST trigger only on `workflow_dispatch` (inputs `force`:
boolean, default `false`; `edge`: string, default `26.11.0-edge`) and on a weekly `schedule`;
it MUST NOT trigger on `pull_request` or `push`. Top-level `permissions` MUST be
`contents: read`; only the regeneration job MAY raise them, to exactly `contents: write` and
`pull-requests: write`. It MUST NOT reference any secret other than the built-in `GITHUB_TOKEN`
and MUST NOT use third-party actions (only `actions/*` plus the preinstalled `gh` CLI). Runs MUST
NOT overlap (`concurrency` group without cancelling a run in progress).

#### Scenario: Triggers and permissions (V-fx-yaml)
- GIVEN `fixtures.yml`
- THEN `on` has exactly `workflow_dispatch` and `schedule`, top-level permissions are `contents: read`, and `grep -E 'secrets\.' | grep -v GITHUB_TOKEN` is empty

#### Scenario: Action allowlist (V-fx-yaml)
- GIVEN every `uses:` line
- THEN each starts with `actions/` and has a `@vN` or 40-hex ref

### Requirement: Regeneration opens a PR only on drift

The regeneration job MUST run `scripts/gen-fixtures.sh` (which includes the round trip), then
C-fixtures. If any `fixtures/lineage/*/shape.txt` differs from the checked-out ref, or `force`
is true, it MUST commit `fixtures/` to branch `automation/lineage-fixtures`, force-push it and
open (or update the existing) PR against the triggering ref, with a body listing the changed
versions, the image digests and a note that CI must be re-triggered by a maintainer because
`GITHUB_TOKEN` pushes do not start workflows. Otherwise it MUST NOT push or open a PR and MUST
write "no format drift" to the job summary. Any generation or check failure MUST fail the job
without pushing.

#### Scenario: No drift (C-fx-run)
- GIVEN fixtures generated with the same images
- WHEN the weekly run executes
- THEN the job succeeds, nothing is pushed and the summary says "no format drift"

#### Scenario: Drift (C-fx-run, V-fx-yaml)
- GIVEN a `shape.txt` change (or `force: true`)
- WHEN the run executes
- THEN exactly one open PR from `automation/lineage-fixtures` exists afterwards, containing only `fixtures/` changes

#### Scenario: Generation failure (V-fx-yaml)
- GIVEN `scripts/gen-fixtures.sh` exits non-zero
- THEN the job fails and no push step runs (steps gated on success)

### Requirement: Edge early warning

A separate job MUST, on `schedule` and on `workflow_dispatch` with a non-empty `edge` input,
generate fixtures for that edge tag into a temporary directory, and write the diff between its
`shape.txt` and `fixtures/lineage/26.10.7/shape.txt` to the job summary. It MUST be non-blocking
(`continue-on-error: true`), MUST have `contents: read` only and MUST NOT commit, push or open PRs.

#### Scenario: Edge drift reported (C-fx-run, V-fx-yaml)
- GIVEN the edge image emits a new checksum `mode`
- WHEN the edge job runs
- THEN the summary shows the shape diff and the workflow result is not failed by this job

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

#### Scenario: Data license for root certificates (V-deny)
- GIVEN the reqwest/rustls dependency tree
- WHEN `cargo deny check licenses` runs
- THEN `CDLA-Permissive-2.0` is accepted with a comment in `deny.toml` naming `webpki-root-certs`
