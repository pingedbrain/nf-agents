## Exploration: local-demo (a demo reproducible in < 5 minutes with Docker)

Requirement: anyone can try the project locally. Someone with only Docker and bash clones the repo, runs one command, and sees the whole stage-1 story: a Nextflow pipeline with an AI step runs twice with no API keys, lineage is recorded, and `nf-lineage-mcp` answers "what changed?", "where did this file come from?", "is it intact?", "what did the AI step do?". Then they get the exact `claude mcp add` line to ask Claude Code the same.

### Current State

- Repo `main` has only the workspace skeleton (`crates/nf-lineage` stub, CI `lint` + `test`). Everything the demo needs is planned, not merged:
  - `nf-lineage` (A–C): `list_runs`, `get`, `find`, semantic `diff` (run level: `params.*`, `tasks.<name>.*`, `outputs.<name>` with LID hashes masked), `check` (`--path-map`), `view` (`#output` redirect), `provenance`.
  - `nf-lineage-mcp` (A–B): stdio server, tools `list_runs/view/find/diff/check/provenance`, compact JSON text results, `isError` tool errors, `--store`, `--path-map FROM=TO`, Dockerfile `rust:1.100.0-alpine` → `distroless/static-debian12:nonroot`, CI `docker` job builds `nf-lineage-mcp:ci` + stdio smoke.
  - `nf-llm-replay` (A–B applied in worktree, C–E pending): replay mode has no HTTP client; record fills missing keys; PR E ships the Dockerfile, `examples/fake_upstream.rs` (deterministic: `json_schema` → `{"answer":"no","confidence":0.9}`), `docs/examples/nf-llm-replay/` (3-agent example + cassette + offline `e2e.sh` + `record.sh`) and the CI `docker` job (builds `nf-llm-replay:ci`, pulls `nextflow/nextflow:26.10.7`, runs `e2e.sh`).
  - `lineage-fixtures` (PR A applied in worktree): `fixtures/pipelines/minimal/` (GENERATE → TRANSFORM → SUMMARIZE, `output` block, `params.greeting`), python3-in-the-Nextflow-image helpers with stdlib `unittest` run in CI `test` (precedent for a stdlib Python helper).
- No `demo/`, no top-level demo docs, no `Makefile`/`justfile`.

### Experiments (this exploration, verified locally)

| # | Experiment | Result |
|---|---|---|
| 1 | Minimal pipeline + `agent` block with `nextflow.enable.types = true` | **Fails**: "Legacy parameter is not allowed with `nextflow.enable.types = true`". |
| 2 | Same without `enable.types` (legacy processes + `agent` + `record QcVerdict`) on 26.10.7, langchain4j, stub LLM on 127.0.0.1 | **Works.** Agents, `record` types and structured output do not need `enable.types`. |
| 3 | Two runs (`--greeting hello|bonjour`, `-output-dir results-<g>`) in one container, warm image | **14 s** total incl. `nf-agent@0.2.0` plugin download. 2 `.history` runs, 2 `AgentRun` records (`model: openai/gpt-5-mini`, `outputSchema`, `input.text` = upper.txt content), agent answer in the `<agent>#output` TaskOutput. |
| 4 | Captured agent requests | Body = `{model, messages:[system, user], stream:false, response_format:{json_schema strict}}`; no host paths, no `<cwd>`; prompt differs per greeting → **2 cassette keys**. |
| 5 | `docker manifest inspect nextflow/nextflow:26.10.7` | **amd64 only** → emulated on Apple Silicon; any binary copied into it must be amd64. |
| 6 | Cold release build of an mcp-like dependency tree (object_store aws/gcp/azure + rmcp + clap + tracing) in Docker, 4 CPUs, empty cargo cache | **1 m 37 s** (gnu; musl similar). Two images from source ≈ 3–4 min + base-image pulls. |
| 7 | Cold pull of a sibling Nextflow image (868 MB, `26.11.0-edge`) | **23 s** on the test link (shares base layers; budget 30–90 s). |
| 8 | Nextflow image contents | runs as root, `NXF_HOME=/.nextflow`, python 3.9.25 (Amazon Linux 2023). |

### Affected Areas

- `demo/` (new) — entry script, pipeline, cassette, scripted MCP tour + its unittest.
- `README.md` — "Try it in 5 minutes" section.
- `docs/demo.md` (new) — transcript, timings, Claude Code prompts, re-recording, troubleshooting.
- `.github/workflows/ci.yml` — one step in `docker` (run the demo with the `:ci` images), one in `test` (tour unittest), shellcheck.
- `.gitignore` — `demo/.run/`.
- `crates/nf-llm-replay/tests/secrets.rs` — S-cas also scans `demo/cassettes/*.jsonl`.

### Approaches

1. **Bash entry point + two containers wired by host FIFOs (chosen)** — `demo/run.sh` builds/pulls images, runs both pipeline runs + replay on a private network, then pipes a stdlib Python tour (in the already-pulled Nextflow image) to `docker run -i --network none nf-lineage-mcp` through two host FIFOs.
   - Pros: Docker + bash only; exercises the *exact* `docker run -i …` command printed for `claude mcp add`; each container keeps its native arch (Apple Silicon safe); no docker socket mount; no extra images.
   - Cons: FIFO open order must be right (documented, CI-tested on Linux); macOS bash 3.2 constraints.
   - Effort: Low–Medium.
2. **Copy the static binary out of the image and run it inside the Nextflow container next to the tour** — one container, plain subprocess pipes.
   - Pros: simplest client code.
   - Cons: breaks on Apple Silicon (arm64 binary in an amd64-only image, experiment 5); does not prove the Docker variant users paste.
   - Effort: Low.
3. **Tour container spawns `docker run -i` via a mounted docker socket.**
   - Pros: single-process client.
   - Cons: socket mount (root-equivalent) in a demo; socket path differs on rootless Docker/Podman/Desktop.
   - Effort: Low.
4. **docker compose** — services for replay, nextflow, mcp.
   - Pros: declarative.
   - Cons: no help wiring an interactive stdio client; second entry point; needs the compose plugin.
   - Effort: Medium.
5. **Pure-bash MCP client** (FIFOs + `read`/`grep -o`), Python only to pretty-print.
   - Pros: one fewer moving part at runtime.
   - Cons: brittle JSON handling in bash 3.2; untestable without Docker.
   - Effort: Medium.

Agent step options: (a) plain-text answer — the deterministic fake upstream answers "FASTQ is a text format." (nonsense here); (b) **structured `QcVerdict {answer, confidence}` asking "Does this synthetic file contain any lowercase letters? yes/no"** — the fake upstream's fixed `{"answer":"no","confidence":0.9}` is *correct*, a real model gives the same answer, so the cassette is honest whichever upstream recorded it. Chosen: (b).

### Recommendation

Approach 1 with agent option (b). One PR (~700 lines) with an escape hatch to split docs. The demo runs as a step of the existing CI `docker` job reusing the `:ci` images (adds ≈ 40–60 s; the Nextflow image is already pulled by the `nf-llm-replay` e2e), so it is PR-blocking, not nightly. Timings: warm ≈ 30–45 s (measured 14 s for the Nextflow part); cold with published GHCR images ≈ 2 min; cold from source ≈ 5–7 min (two Rust release builds) — documented honestly; the "< 5 minutes" promise is stated for warm cache or published images.

### Risks

- Upstream ordering: needs `nf-lineage` C, `nf-lineage-mcp` B, `nf-llm-replay` C–E merged (record mode + fake upstream + images + `docker` job).
- `agent` is preview: a new Nextflow/`nf-agent` plugin changes request bodies → cassette misses (599). Pinned image tag; 599 message names the record command.
- Plugin download from the Nextflow registry is the only networked runtime step (besides image pulls); registry outage fails the demo (clear message).
- Apple Silicon: Nextflow image is emulated (slower, untested in CI).
- Cold-from-source exceeds 5 minutes; mitigated by GHCR images once the release change publishes them.

### Ready for Proposal

Yes — scope, approach and measurements are settled; proceed to proposal.
