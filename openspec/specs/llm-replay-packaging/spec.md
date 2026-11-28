# llm-replay-packaging Specification

## Purpose

How `baqueano-replay` ships as a container and how a Nextflow pipeline uses it offline. Planned deliverables
("small Docker image", "example in docs/") made mandatory and testable by `docs/verification.md`
section 4.3, items 7-8.

## Verification checks

| ID | Kind | Command / name | PR |
|----|------|----------------|----|
| D-build | CI `docker` | `docker build -f crates/baqueano-replay/Dockerfile -t baqueano-replay:ci .` | E |
| D-user | CI `docker` | `docker image inspect` → `Config.User` is `65532:65532` | E |
| D-size | CI `docker` | image size ≤ 25 MB | E |
| D-run | CI `docker` | `docker run --rm baqueano-replay:ci --help` exits 0 | E |
| D-e2e | CI `docker` | `E2E_REQUIRE_DOCKER=1 IMAGE=baqueano-replay:ci docs/examples/baqueano-replay/e2e.sh` | E |
| D-skip | verify | `PATH=<dir without docker> docs/examples/baqueano-replay/e2e.sh` prints `SKIP` and exits 0 | E |
| D-lint | verify | `shellcheck docs/examples/baqueano-replay/*.sh` (if installed) | E |
| D-doc | verify | grep README for `agent.baseUrl`, `--mode record`, `BAQUEANO_REPLAY_API_KEY`, `pi` | E |
| S-cas | CI `test` | `secrets::committed_example_cassette_contains_no_credentials` | E |

## Requirements

### Requirement: Container image

`crates/baqueano-replay/Dockerfile` MUST be multi-stage: build stage `rust:1.100.0-alpine` producing a
statically linked musl binary with `--locked`; final stage `gcr.io/distroless/static-debian12:nonroot`
containing only the binary (plus the base image's CA certificates), running as UID/GID `65532`,
`ENTRYPOINT ["/baqueano-replay"]`, env `BAQUEANO_REPLAY_BIND=0.0.0.0`, `BAQUEANO_REPLAY_PORT=8080`,
`EXPOSE 8080`, OCI labels `source`, `licenses=Apache-2.0`, `description`. The build context MUST be
the repo root with a `.dockerignore` excluding `target/`, `.git/` and everything not needed to build.
The image MUST NOT contain a shell, secrets or cassettes.

#### Scenario: Build and run (D-build, D-run, D-user, D-size)
- GIVEN a clean checkout
- WHEN the image is built and `--help` is run
- THEN the build succeeds, `--help` exits 0, the user is `65532:65532` and size ≤ 25 MB

#### Scenario: Replay in a container
- GIVEN the image, a cassette mounted read-only at `/cassettes/x.jsonl`
- WHEN started with `--mode replay --cassette /cassettes/x.jsonl`
- THEN it listens on `0.0.0.0:8080` and never writes the mount (covered by D-e2e)

### Requirement: Example pipeline

`docs/examples/baqueano-replay/` MUST contain: `main.nf` with three agents mirroring the captures
(`qa` plain text, `qa_structured` returning a record → `json_schema` strict, `shouty` with a process
tool → tool-call round trip), `nextflow.config` setting only `agent.baseUrl` (from env
`REPLAY_BASE_URL`, default `http://localhost:8080/v1`) and no `apiKey`, a committed cassette
`cassettes/example.jsonl`, `e2e.sh`, `record.sh`, and `README.md`. The README MUST show replay
with Docker, re-recording against a real provider (`record.sh` with `BAQUEANO_REPLAY_API_KEY`
and `--upstream`), the note that `OPENAI_API_KEY` must not be set for Nextflow when `baseUrl` is
custom, the `pi` runner settings (`agent.apiKey` dummy, reachable host name), and that content is
synthetic research material.

#### Scenario: Cassette is secret-free (S-cas)
- GIVEN the committed example cassette
- WHEN scanned like S-scan
- THEN no credential patterns or absolute host paths are found

#### Scenario: Docs (D-doc)
- GIVEN the README
- WHEN grepped
- THEN every required literal is present

### Requirement: Offline end-to-end script

`e2e.sh` MUST: exit 0 printing `SKIP: docker not available` when `docker` is missing or not running
unless `E2E_REQUIRE_DOCKER=1` (then exit 1); use `IMAGE` (default: build `baqueano-replay:e2e`); prefetch
plugin `nf-agent@0.2.0` with `nextflow/nextflow:26.10.7` into a temp dir (the only networked step);
create a uniquely named `--internal` Docker network; start the replay container there (alias `replay`,
cassette read-only); run `nextflow run main.nf` in `nextflow/nextflow:26.10.7` on that network with
`NXF_OFFLINE=true`, `NXF_PLUGINS_DIR` (read-only) and `REPLAY_BASE_URL=http://replay:8080/v1`;
assert Nextflow exit 0, the three expected outputs in its stdout, and zero `outcome="miss"` lines
in the replay logs; always clean up containers, network and temp dir (trap), restoring file ownership.

#### Scenario: Offline pipeline run (D-e2e)
- GIVEN the image and the committed cassette
- WHEN `e2e.sh` runs
- THEN the pipeline succeeds with no model provider reachable and the script exits 0

#### Scenario: Missing entry surfaces clearly (D-e2e, manual)
- GIVEN the cassette with one line removed
- WHEN `e2e.sh` runs
- THEN it exits non-zero and Nextflow's error shows the 599 message with the record command

#### Scenario: No Docker (D-skip)
- GIVEN no `docker` on PATH
- WHEN `e2e.sh` runs without `E2E_REQUIRE_DOCKER`
- THEN it prints `SKIP` and exits 0

### Requirement: Recording helper

`record.sh` MUST run the same pipeline against `baqueano-replay --mode record` (host binary or image)
with `--upstream` from `UPSTREAM` (default: the fake upstream from
`cargo run -p baqueano-replay --example fake_upstream`, so no key is needed) and MUST rewrite
`cassettes/example.jsonl`. It MUST never print `BAQUEANO_REPLAY_API_KEY`.

#### Scenario: Regenerate without keys
- GIVEN Docker and cargo, no API key
- WHEN `record.sh` runs
- THEN the cassette is regenerated deterministically (same bytes as committed)
