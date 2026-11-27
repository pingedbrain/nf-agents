# local-demo Specification

## Purpose

A reproducible local demo of stage 1. It needs Docker and bash only: no API keys and no Rust toolchain. A Nextflow pipeline with an AI step runs twice, with lineage enabled and the model answers replayed from a committed cassette by `nf-llm-replay`. A scripted MCP session against `nf-lineage-mcp` then shows the value of the stage, and the script prints how to ask Claude Code the same questions.

Terms: **workdir** = `DEMO_WORKDIR` (default `<repo>/demo/.run`), mounted at `/demo` in every container. **Run A / Run B** = the `hello` / `bonjour` runs.

> Deviation (2026-11-26): reconciled with the implementation: internal network + plugin prefetch, host-network record mode into a fresh cassette, miss detection on `outcome="miss"`, Valid/Mismatch both on Run A, three extra unit tests.

## Verification checks

Unit tests: `demo/test_tour.py` (`<Class>.<test>`), stdlib `unittest`, no Docker.

| ID | Kind | Command / name |
|----|------|----------------|
| DM-run | CI `docker` | `MCP_IMAGE=nf-lineage-mcp:ci REPLAY_IMAGE=nf-llm-replay:ci demo/run.sh` → exit 0; stdout has `claude mcp add --transport stdio nf-lineage -- docker run -i` |
| DM-unit | CI `test` | `python3 -I -m unittest discover -s demo -p 'test_*.py'` |
| DM-sh | CI `test` | `shellcheck demo/run.sh` |
| DM-cas | CI `test` | `secrets::committed_cassettes_contain_no_credentials` (crate `nf-llm-replay`) |
| DM-miss | verify (Docker) | remove one cassette line → `run.sh` exits ≠ 0 and prints the `DEMO_RECORD=1` hint |
| DM-record | verify (Docker) | `DEMO_RECORD=1` against the fake upstream → cassette bytes unchanged |
| DM-time | verify (Docker) | warm and cold timings measured and recorded in `docs/demo.md` |
| DM-doc | verify | README/`docs/demo.md` grep (literals below) |

## Requirements

### Requirement: Entry point

`demo/run.sh` MUST work from any cwd with bash ≥ 3.2 and Docker. It MUST validate the workdir before anything else, then fail with exit 1 and `docker is required` when Docker is missing or not running. Images: `MCP_IMAGE` / `REPLAY_IMAGE` MUST be used as given; when unset, the script MUST build `nf-lineage-mcp:demo` and `nf-llm-replay:demo` from the repo Dockerfiles. It MUST NOT forward host environment variables to containers (except as stated under "Re-recording").

#### Scenario: No Docker (T `RunScriptTests.test_fails_fast_without_docker`)
- GIVEN a stub `docker` first on `PATH` that exits 1 (daemon down)
- WHEN `run.sh` runs with an empty temp `DEMO_WORKDIR`
- THEN exit 1, stderr has `docker is required`

#### Scenario: Record without upstream (T `RunScriptTests.test_record_requires_upstream`)
- GIVEN `DEMO_RECORD=1` and no `UPSTREAM`
- THEN exit 1 naming `UPSTREAM`, before any container starts

#### Scenario: CI images (DM-run)
- GIVEN both `:ci` images
- WHEN `run.sh` runs with the overrides
- THEN no `docker build` happens and it exits 0

### Requirement: Workdir safety

The workdir MUST be recreated on every run. A non-empty directory without the marker file `.nf-agents-demo` MUST NOT be deleted: exit 1 with `refusing to delete`. After the run, every file in the workdir MUST be owned by the invoking user, and the Docker network and containers MUST be removed, on failure too.

#### Scenario: Foreign directory (T `RunScriptTests.test_refuses_foreign_workdir`)
- GIVEN `DEMO_WORKDIR` = a temp dir containing `keep.txt`
- WHEN `run.sh` runs
- THEN exit 1, `keep.txt` still exists

### Requirement: Pipeline and replay

The script MUST run `demo/main.nf` twice in `nextflow/nextflow:26.10.7` with `lineage.enabled = true`, first with `--greeting hello` and then with `--greeting bonjour`, each with `-output-dir results-<greeting>`. The pipeline MUST be GENERATE → TRANSFORM → SUMMARIZE (as in the minimal fixture) plus the agent `QC_NOTE` (`openai/gpt-5-mini`, runner `langchain4j`, structured output `QcVerdict {answer, confidence}`). That agent is asked whether `upper.txt` contains lowercase letters, and is reached through `agent.baseUrl = http://replay:8080/v1` on an `--internal` Docker network (the pinned `nf-agent@0.2.0` plugin is fetched first, then Nextflow runs with `NXF_OFFLINE=true`). `nf-llm-replay` MUST run `--mode replay` with `demo/cassettes/demo.jsonl` mounted read-only. No container receives an API key. Inputs are synthetic.

#### Scenario: Offline answers (DM-run)
- GIVEN the committed cassette
- WHEN both runs finish
- THEN each prints `QC verdict: [answer:no, confidence:0.9]` and the replay logs have 0 `outcome="miss"` lines

#### Scenario: Cassette drift (DM-miss)
- GIVEN a cassette missing one entry
- WHEN `run.sh` runs
- THEN it exits ≠ 0 and prints the 599 message and `DEMO_RECORD=1 … demo/run.sh`

### Requirement: Cassette

`demo/cassettes/demo.jsonl` MUST hold exactly the 2 entries the two runs need. It MUST contain no credentials and no absolute host paths. `docs/demo.md` MUST say which upstream (fake, or model + date) recorded it.

#### Scenario: Secret-free (DM-cas)
- GIVEN the committed cassettes (`docs/examples/nf-llm-replay/cassettes/*.jsonl`, `demo/cassettes/*.jsonl`)
- WHEN scanned
- THEN no credential pattern or host path is found

### Requirement: Re-recording

With `DEMO_RECORD=1` and `UPSTREAM=<url>`, the replay container MUST run `--mode record --upstream $UPSTREAM` on the host network (so a local upstream on `127.0.0.1` is reachable). It MUST receive `NF_LLM_REPLAY_API_KEY` by name only, run as the invoking user, record into a fresh cassette in the workdir, and the script MUST copy it to `demo/cassettes/demo.jsonl` only when both runs succeed. The script MUST NOT print the key. Without `UPSTREAM` it MUST exit 1 before starting containers.

#### Scenario: Deterministic regeneration (DM-record)
- GIVEN the fake upstream (`cargo run -p nf-llm-replay --example fake_upstream -- --port 18181`)
- WHEN `NF_LLM_REPLAY_API_KEY=unused DEMO_RECORD=1 UPSTREAM=http://127.0.0.1:18181 demo/run.sh`
- THEN the cassette is rewritten byte-identical to the committed one

### Requirement: MCP tour

`demo/tour.py` (python ≥ 3.9 stdlib, run in the Nextflow image) MUST speak MCP over stdio to `docker run -i --rm --network none -v <workdir>:/demo:ro <MCP_IMAGE> --store /demo/.lineage`. It MUST: `initialize`; call `list_runs` (expect 2); `diff` A→B (expect exactly `params.greeting` `hello`→`bonjour`); `find` `type=AgentRun, workflowRun=<B>` then `view` it and its `#output` (expect model `openai/gpt-5-mini`, answer `no`); `provenance` of `<B>/summary.txt` (expect SUMMARIZE, TRANSFORM, GENERATE, external `seed.txt`); `check` Run A's `summary.txt` → `Valid`; append a labelled line to that file (only if its path is under the workdir), then `check` it again → `Mismatch`. Each step MUST print a short human summary. Any unexpected answer, tool error (`isError`) or server EOF MUST exit 1 naming the step. The JSON-RPC transcript MUST be written to `<workdir>/transcript.jsonl`.

#### Scenario: Happy path (T `TourTests.test_happy_path_prints_answers_and_exits_0`, DM-run)
- GIVEN canned server answers matching the expectations
- THEN the summaries (`params.greeting: "hello" → "bonjour"`, `Valid`, `Mismatch`) are printed and the exit code is 0

#### Scenario: Unexpected diff (T `TourTests.test_unexpected_diff_fails`)
- GIVEN a diff with an extra change
- THEN exit 1 naming `diff` and the extra path

#### Scenario: Tool error (T `TourTests.test_tool_error_fails_with_server_message`)
- GIVEN `isError: true` on `provenance`
- THEN exit 1 printing the server text

#### Scenario: Unexpected answer shape (T `TourTests.test_unexpected_answer_shape_names_the_step`)
- GIVEN `find` answers an object instead of a list
- THEN exit 1 naming `find`, no traceback

#### Scenario: Edit guard (T `TourTests.test_never_edits_files_outside_the_workdir`)
- GIVEN `check` reports a path outside the workdir
- THEN exit 1 with `outside`, and the file is unchanged

#### Scenario: Server gone (T `TourTests.test_server_eof_fails_cleanly`)
- GIVEN EOF before a response
- THEN exit 1, `server closed`, no traceback

### Requirement: Claude Code hand-off

On success, the script MUST print, with the absolute workdir:
- `claude mcp add --transport stdio nf-lineage -- docker run -i --rm --network none -v "<workdir>:/demo:ro" <MCP_IMAGE> --store /demo/.lineage`
- the native variant `nf-lineage-mcp --store "<workdir>/.lineage" --path-map /demo="<workdir>"` (install via `cargo install`)
- ≥ 3 example prompts, including "What changed between my last two Nextflow runs?"
- `claude mcp remove nf-lineage`

#### Scenario: Printed (DM-run)
- THEN stdout contains both `claude mcp add` lines and the workdir path

### Requirement: Honesty and docs

The script, README and `docs/demo.md` MUST state: research use only, no clinical claims, synthetic data, answers replayed from a recorded cassette. README MUST have `## Try it in 5 minutes` (prerequisites, one command, abridged output, Claude Code commands, link to `docs/demo.md`). `docs/demo.md` MUST have the full transcript, a timings table (warm, cold with published images, cold from source; machine stated), the prompts, re-recording, cleanup, and troubleshooting (Apple Silicon emulation, WSL2, plugin download).

#### Scenario: Docs (DM-doc, DM-time)
- GIVEN README and `docs/demo.md`
- THEN they contain `demo/run.sh`, `claude mcp add`, `research use`, `synthetic`, `MCP_IMAGE`, `DEMO_RECORD=1`, and a warm timing < 5 min
