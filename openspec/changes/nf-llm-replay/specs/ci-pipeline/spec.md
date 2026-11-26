# Delta for ci-pipeline

## Verification checks (added)

| ID | Kind | Command / name |
|----|------|----------------|
| C-docker | CI `docker` | the change's PR run shows `docker` green (D-build, D-user, D-size, D-run, D-e2e of llm-replay-packaging) |
| V-yaml | verify | inspect `.github/workflows/ci.yml` (grep/yq), as in the main spec |
| V-deny | verify | `cargo deny check` with the new dependency tree |

## ADDED Requirements

### Requirement: Docker job

Job `docker` MUST run on every PR and push to `main`, on `ubuntu-latest`, with no secrets and no
registry login or push. It MUST: build `crates/nf-llm-replay/Dockerfile` from the repo root as
`nf-llm-replay:ci`; fail if the image user is not `65532:65532` or the image exceeds 25 MB; run
`docs/examples/nf-llm-replay/e2e.sh` with `E2E_REQUIRE_DOCKER=1` and `IMAGE=nf-llm-replay:ci`.
It MUST NOT depend on `lint`/`test` (runs in parallel). It SHOULD finish in under 10 minutes
(`timeout-minutes: 20`).

#### Scenario: PR touching the proxy (C-docker)
- GIVEN a PR
- WHEN CI runs
- THEN `docker` builds the image and the offline e2e passes

#### Scenario: Broken image (V-yaml)
- GIVEN a Dockerfile change that runs as root
- WHEN `docker` runs
- THEN the user check fails the job

#### Scenario: Never publishes (V-yaml)
- GIVEN the `docker` job definition
- WHEN inspected
- THEN it contains no `docker push`, `docker login`, `secrets.` or `packages: write`

## MODIFIED Requirements

### Requirement: Triggers and concurrency

The workflow MUST run on `pull_request` and on `push` to `main`. A newer run for the same ref MUST cancel the in-progress one.
(Previously: the PR scenario listed only `lint` and `test`.)

#### Scenario: PR opened (C-run)
- GIVEN a PR against `main`
- WHEN it is opened or updated
- THEN `lint`, `test` and `docker` run

#### Scenario: Rapid pushes (V-yaml)
- GIVEN two pushes to the same PR in quick succession
- WHEN the second run starts
- THEN the first is cancelled (`concurrency` keyed on workflow + ref, `cancel-in-progress: true`)

### Requirement: Least privilege, no secrets

The workflow MUST declare `permissions: contents: read` at top level and MUST NOT reference `secrets.*`. It MUST NOT need API keys or cloud credentials.
(Previously: the fork scenario covered two jobs; it now covers all three, including `docker`.)

#### Scenario: Fork PR (V-yaml, C-run)
- GIVEN a PR from a fork (no secrets available)
- WHEN CI runs
- THEN all jobs (`lint`, `test`, `docker`) can pass; `grep -c 'secrets\.' ci.yml` is 0

### Requirement: Supply-chain policy

`deny.toml` MUST: allow only permissive licenses compatible with Apache-2.0 (no GPL/LGPL/AGPL/SSPL); deny crates with known vulnerabilities and yanked versions; deny wildcard version requirements; allow only the crates.io registry and no git sources.
(Previously: identical text; the allowlist now also contains `CDLA-Permissive-2.0`, required by `webpki-root-certs` via reqwest's rustls verifier.)

#### Scenario: Clean tree (C-deny)
- GIVEN the workspace as shipped
- WHEN `cargo deny check` runs
- THEN it exits 0

#### Scenario: Policy content (V-deny)
- GIVEN `deny.toml`
- WHEN inspected
- THEN the license allowlist has no copyleft ids, `unknown-registry` and `unknown-git` are `deny`, `allow-registry` is crates.io only

#### Scenario: Data license for root certificates (V-deny)
- GIVEN the reqwest/rustls dependency tree
- WHEN `cargo deny check licenses` runs
- THEN `CDLA-Permissive-2.0` is accepted with a comment in `deny.toml` naming `webpki-root-certs`
