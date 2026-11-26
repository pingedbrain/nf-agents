# Delta for ci-pipeline

The base text differs per requirement:
- "Docker job" is the version ADDED by `nf-llm-replay` and MODIFIED by `nf-lineage-mcp`.
- "Test job" is the version MODIFIED by `lineage-fixtures`.

If those changes are not archived yet when this one archives, merge these blocks into theirs.

## Verification checks (added)

| ID | Kind | Command / name |
|----|------|----------------|
| C-demo | CI `docker` | the PR run shows the step `Local demo` green (DM-run of `local-demo`) |
| C-demo-unit | CI `test` | DM-unit and DM-sh of `local-demo` |
| V-yaml | verify | inspect `.github/workflows/ci.yml` |

## MODIFIED Requirements

### Requirement: Docker job

Job `docker` MUST run on every PR and every push to `main`, on `ubuntu-latest`, with no secrets and no registry login or push. It MUST:
- build `crates/nf-llm-replay/Dockerfile` from the repo root as `nf-llm-replay:ci`, fail if the image user is not `65532:65532` or the image exceeds 25 MB, and run `docs/examples/nf-llm-replay/e2e.sh` with `E2E_REQUIRE_DOCKER=1` and `IMAGE=nf-llm-replay:ci`;
- build `crates/nf-lineage-mcp/Dockerfile` as `nf-lineage-mcp:ci`, fail if its user is not `65532:65532` or it exceeds 30 MB, and run the D-smoke stdio session against `fixtures/lineage/26.10.7/.lineage` mounted read-only;
- as its last step, run `demo/run.sh` with `MCP_IMAGE=nf-lineage-mcp:ci` and `REPLAY_IMAGE=nf-llm-replay:ci` (no image rebuild) and fail if the script fails or its stdout lacks the `claude mcp add` hand-off.

It MUST NOT depend on `lint`/`test`, so it runs in parallel with them. It SHOULD finish in under 10 minutes (`timeout-minutes: 20`).
(Previously: the job built and checked both images but did not run the local demo.)

#### Scenario: PR touching the proxy (C-docker)
- GIVEN a PR
- WHEN CI runs
- THEN `docker` builds the image and the offline e2e passes

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

### Requirement: Test job

Job `test` MUST run `cargo test --workspace --locked`, the offline fixture checks (C-fixtures) and the demo checks (C-demo-unit: tour unittest and `shellcheck demo/run.sh`). It MUST fail if any of them fails. The fixture and demo checks MUST NOT need Docker or network.
(Previously: `cargo test` + offline fixture checks.)

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
