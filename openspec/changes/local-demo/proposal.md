# Proposal: local-demo

## Intent

The project requires a reproducible local demo: one command, Docker only, no API keys, < 5 minutes on a warm cache. It runs a pipeline with an AI step twice, then shows `nf-lineage-mcp` answering what changed, where a file came from, whether it is intact and what the AI step did, and prints the `claude mcp add` line.

## Scope

### In Scope
- `demo/run.sh`: images (build, or `MCP_IMAGE`/`REPLAY_IMAGE`); two runs (`greeting` hello/bonjour) in `nextflow/nextflow:26.10.7`, lineage on, `QC_NOTE` agent on `nf-llm-replay` replay; MCP tour; prints `claude mcp add` (Docker + native).
- `demo/{main.nf,nextflow.config,data/seed.txt,cassettes/demo.jsonl}` (synthetic).
- `demo/tour.py` (stdlib MCP client): `list_runs → diff → find/view AgentRun → provenance → check` (Valid, then Mismatch after a labelled edit); asserts answers.
- `demo/test_tour.py`; `DEMO_RECORD=1` re-recording.
- README "Try it in 5 minutes"; `docs/demo.md` (transcript, measured timings, prompts).
- CI: demo in `docker`; unittest + shellcheck in `test`.

### Out of Scope
- Publishing (release change), Makefile/compose, Windows outside WSL2, Apple Silicon CI, S3, `pi` runner, live model calls.

## Capabilities

### New Capabilities
- `local-demo`: entry script, pipeline + cassette, MCP tour, safety (workdir guard, read-only mounts, no secrets), docs, timings.

### Modified Capabilities
- `ci-pipeline`: `docker` job runs the demo; `test` job runs the tour unittest and shellcheck.

## Approach

Private network for Nextflow + replay; tour container ↔ `docker run -i --network none nf-lineage-mcp` via two host FIFOs (same command printed for Claude Code; native arch per container). A structured agent verdict keeps the cassette honest from fake or real upstream.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `demo/` | New | script, pipeline, cassette, tour, test |
| `README.md`, `docs/demo.md` | Modified/New | quickstart, transcript |
| `.github/workflows/ci.yml` | Modified | 3 steps |
| `.gitignore` | Modified | `demo/.run/` |
| `crates/nf-llm-replay/tests/secrets.rs` | Modified | scan demo cassette |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Preview `agent` request drift → 599 | Med | pinned image; hint names `DEMO_RECORD=1` |
| Plugin registry outage | Low | clear failure, documented |
| Cold source build > 5 min | High | documented; GHCR override |
| FIFO ordering deadlock | Low | fixed open order; CI e2e |
| `rm -rf` of a user dir | Low | marker-file guard |

## Rollback Plan

Revert the PR; nothing depends on it.

## Dependencies

- `nf-lineage` C, `nf-lineage-mcp` B, `nf-llm-replay` C–E, `lineage-fixtures` A merged.

## Success Criteria

- [ ] `demo/run.sh` exits 0 in CI with the `:ci` images; tour assertions pass.
- [ ] Diff = only `params.greeting`; provenance reaches `seed.txt`; check Valid then Mismatch.
- [ ] `docs/demo.md` records measured warm (< 5 min) and cold timings.
- [ ] One PR ≤ ~800 hand-written lines, green.
