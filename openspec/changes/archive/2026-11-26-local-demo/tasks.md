# Tasks: local-demo

Strict TDD (RED → GREEN). Test and check names come from `specs/local-demo/spec.md`. Estimates count hand-written lines only (no `openspec/`, no generated cassette).

**Preconditions (all merged):** `nf-lineage` C, `nf-lineage-mcp` B, `nf-llm-replay` C–E (record mode, `examples/fake_upstream.rs`, Dockerfiles, `docker` job), `lineage-fixtures` A.

**Each PR ends with:**
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --locked -- -D warnings`
- `cargo test --workspace --locked`
- DM-unit
- DM-sh

## PR A — demo, tour, CI · est. ≈ 570 lines

Phase 1: tour (pure, no Docker)
- [x] A.1 RED: `demo/test_tour.py` `FakeServer` + `TourTests.test_happy_path_prints_answers_and_exits_0`. Canned responses use the `lineage-mcp` shapes (rows, `SemanticDiff`, `Provenance`, `CheckReport`).
- [x] A.2 GREEN: `demo/tour.py` `Session` (`request`/`notify`/`tool`, transcript), `run_tour` steps, `main` (`--workdir`; stderr output).
- [x] A.3 RED→GREEN: `test_unexpected_diff_fails`, `test_tool_error_fails_with_server_message`, `test_server_eof_fails_cleanly` (`TourError`, exit 1, no traceback). The append is guarded to paths under the workdir.

Phase 2: entry script
- [x] A.4 RED: `RunScriptTests.test_refuses_foreign_workdir`, `test_fails_fast_without_docker`.
- [x] A.5 GREEN: `demo/run.sh` covering:
  - the workdir guard + marker, then the Docker check;
  - `DEMO_RECORD`/`UPSTREAM` validation;
  - image build or override;
  - network and the `trap` cleanup.
- [x] A.6 `demo/main.nf` (minimal processes + `record QcVerdict` + `QC_NOTE`, no `enable.types`), `demo/nextflow.config` (`lineage.enabled`, `agent.runner/baseUrl`), `demo/data/seed.txt`.
- [x] A.7 Extend `run.sh`:
  - replay container + readiness poll;
  - Nextflow container (two runs, in-container `chown` trap);
  - miss hint;
  - FIFO-wired mcp + tour (D3 order, comment);
  - hand-off print.
- [x] A.8 Record `demo/cassettes/demo.jsonl` with `NF_LLM_REPLAY_API_KEY=unused DEMO_RECORD=1 UPSTREAM=http://127.0.0.1:18181`, using the fake upstream (`cargo run -p nf-llm-replay --example fake_upstream -- --port 18181`). Run it twice to confirm DM-record (identical bytes). If a provider key is available, record `gpt-5-mini` instead (same answers).
- [x] A.9 RED→GREEN: `crates/nf-llm-replay/tests/secrets.rs` renamed to `committed_cassettes_contain_no_credentials`, scanning both cassette dirs (DM-cas).

Phase 3: wiring + verification
- [x] A.10 `.gitignore`: add `demo/.run/`.
- [x] A.11 `.github/workflows/ci.yml`:
  - `test`: unittest + `shellcheck demo/run.sh`;
  - `docker`: last step `Local demo` with `MCP_IMAGE=nf-lineage-mcp:ci REPLAY_IMAGE=nf-llm-replay:ci`, plus a grep for `claude mcp add`;
  - V-yaml.
- [x] A.12 Local DM-run (both `:demo` builds and overrides), DM-miss (drop one line → hint shown). Save the stdout/stderr transcript for PR B.
- [ ] A.13 C-demo green on the PR run (pending: needs the PR's CI run); record the step duration. Diff ≤ 800.
- Escape hatch: if over 800, move A.9 to its own PR.

> Deviation (2026-11-26): record mode runs on the host network (the fake upstream binds 127.0.0.1), records into a fresh workdir cassette and copies it on success; replay mode uses an `--internal` network with a plugin prefetch. See design.md "Deviations". PR B runs as a separate batch (line budget).

## PR B — README + docs/demo.md · est. ≈ 220 lines

- [x] B.1 Measure DM-time, stating the machine for each:
  - warm (second run);
  - cold from source (`docker builder prune -af`, `docker rmi` the demo, Rust and Nextflow images);
  - cold with published images if available (else write "pending release");
  - the CI step time.
- [x] B.2 `docs/demo.md`:
  - what happens (diagram from design);
  - full transcript from A.12;
  - timings table;
  - Claude Code: both `claude mcp add` variants, 4 prompts (changes, provenance, AI step, integrity), `claude mcp remove`;
  - re-recording (`DEMO_RECORD=1`, fake or real upstream, which upstream recorded the committed cassette);
  - cleanup;
  - troubleshooting (Apple Silicon emulation, WSL2, plugin download, 599 drift);
  - research-use banner.
- [x] B.3 `README.md` `## Try it in 5 minutes`: prerequisites, `demo/run.sh`, abridged output, Claude Code lines, honest timings, `MCP_IMAGE` fast path, link to `docs/demo.md`.
- [x] B.4 DM-doc greps; every spec scenario mapped to a passing check; diff ≤ 800.

> Deviation (2026-11-26, PR B): B.1 did not run `docker builder prune -af` (it wipes the whole host build cache). `docs/demo.md` reports warm (24 s), `:ci` overrides (21 s) and first run building both images (180 s, base images cached); fully cold, published images, the CI step, Apple Silicon and WSL2 are stated as not measured or pending.
