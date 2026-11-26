# Delta for ci-pipeline

Base: requirement "Docker job" as added by change `nf-llm-replay` (PR E). If `nf-llm-replay` is not archived when this change archives, merge this block into its ADDED requirement instead.

## Verification checks (added)

| ID | Kind | Command / name |
|----|------|----------------|
| C-docker | CI `docker` | the PR run shows `docker` green incl. D-build, D-user, D-size, D-smoke of `lineage-mcp` |
| V-yaml | verify | inspect `.github/workflows/ci.yml` |

## MODIFIED Requirements

### Requirement: Docker job

Job `docker` MUST run on every PR and push to `main`, on `ubuntu-latest`, with no secrets and no registry login or push. It MUST: build `crates/nf-llm-replay/Dockerfile` from the repo root as `nf-llm-replay:ci`; fail if the image user is not `65532:65532` or the image exceeds 25 MB; run `docs/examples/nf-llm-replay/e2e.sh` with `E2E_REQUIRE_DOCKER=1` and `IMAGE=nf-llm-replay:ci`. It MUST also build `crates/nf-lineage-mcp/Dockerfile` as `nf-lineage-mcp:ci`, fail if its user is not `65532:65532` or it exceeds 30 MB, and run the D-smoke stdio session against `fixtures/lineage/26.10.7/.lineage` mounted read-only. It MUST NOT depend on `lint`/`test` (runs in parallel). It SHOULD finish in under 10 minutes (`timeout-minutes: 20`).
(Previously: the job built and checked only the `nf-llm-replay` image.)

#### Scenario: PR touching the proxy (C-docker)
- GIVEN a PR
- WHEN CI runs
- THEN `docker` builds the image and the offline e2e passes

#### Scenario: PR touching the MCP server (C-docker)
- GIVEN a PR
- WHEN CI runs
- THEN `docker` builds `nf-lineage-mcp:ci` and the stdio smoke returns 2 run LIDs

#### Scenario: Broken image (V-yaml)
- GIVEN a Dockerfile change that runs as root
- WHEN `docker` runs
- THEN the user check fails the job

#### Scenario: Never publishes (V-yaml)
- GIVEN the `docker` job definition
- WHEN inspected
- THEN it contains no `docker push`, `docker login`, `secrets.` or `packages: write`
