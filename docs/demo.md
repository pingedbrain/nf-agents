# Local demo

> **Research use only.** No clinical claims. The input data is synthetic, and the AI step's
> answers are replayed from a recorded cassette: no model is called and no API key is needed.

`demo/run.sh` shows what v0.1 does in about half a minute, with Docker and bash only:

1. A small Nextflow pipeline with an AI step runs twice with lineage enabled: once with
   `--greeting hello` (run A) and once with `--greeting bonjour` (run B).
2. The AI step (`QC_NOTE`, model `openai/gpt-5-mini`) is answered by `nf-llm-replay` from
   `demo/cassettes/demo.jsonl`, on an internal Docker network with no route to any model provider.
3. A scripted MCP client (`demo/tour.py`) asks `nf-lineage-mcp` what changed, what the AI step
   answered, where a file came from and whether a file is intact, and checks every answer.
4. The script prints the `claude mcp add` command to ask Claude Code the same questions.

## Run it

Prerequisites: Docker (Engine or Desktop, running), bash, git, and a few GB of free disk for the
images. No Rust toolchain, no Nextflow install, no API keys.

```sh
git clone https://github.com/pingedbrain/nf-agents.git
cd nf-agents
demo/run.sh
```

Everything lands in `demo/.run/` (recreated on every run; override with `DEMO_WORKDIR`).

## What happens

```
run.sh ─ check workdir ─ check docker ─ build (or reuse) images ─ internal network
  ├─ nextflow plugin install nf-agent@0.2.0     (the only step with network access)
  ├─ nf-llm-replay --mode replay   ◀── HTTP ──┐  cassette mounted read-only
  ├─ nextflow run main.nf  (hello, bonjour) ──┘  → demo/.run/.lineage, results-*/
  ├─ tour.py ⇄ nf-lineage-mcp over stdio     (both --network none; store mounted read-only)
  └─ print the Claude Code commands; remove containers and network, on failure too
```

The MCP server container is started with exactly the `docker run -i …` command you give Claude
Code. The tour writes the raw JSON-RPC exchange to `demo/.run/transcript.jsonl`.

## Transcript

A real run on the machine described under [Timings](#timings), images already built. Only the
checkout path was replaced with `/path/to/nf-agents`.

```text
$ demo/run.sh
==> building nf-lineage-mcp:demo from source (first time: a few minutes)
==> building nf-llm-replay:demo from source (first time: a few minutes)
==> fetching the nf-agent plugin (the only networked step)
Downloading plugin nf-agent@0.2.0
==> starting nf-llm-replay in replay mode (internal network, no internet)
==> running demo/main.nf twice: --greeting hello, then --greeting bonjour
N E X T F L O W  ~  version 26.10.7
Launching `/src/main.nf` [agitated_raman] - revision: 380cdd93f2
WARN: Agents are a preview feature -- syntax and behavior may change in future releases
[02/50b7cf] Submitted process > GENERATE
[7d/6e3ca6] Submitted process > TRANSFORM
[04/5e1a13] Submitted process > SUMMARIZE
[d6/448204] Submitted process > QC_NOTE
QC verdict: [answer:no, confidence:0.9]

Outputs:

  /demo/results-hello

  upper: upper/upper.txt

  summary: summary.txt
N E X T F L O W  ~  version 26.10.7
Launching `/src/main.nf` [agitated_gilbert] - revision: 380cdd93f2
WARN: Agents are a preview feature -- syntax and behavior may change in future releases
[a5/cb79c9] Submitted process > GENERATE
[e2/0ccbdd] Submitted process > TRANSFORM
[40/a4bfc9] Submitted process > QC_NOTE
[aa/f639a1] Submitted process > SUMMARIZE
QC verdict: [answer:no, confidence:0.9]

Outputs:

  /demo/results-bonjour

  upper: upper/upper.txt

  summary: summary.txt
==> asking nf-lineage-mcp about the two runs (scripted MCP session, transcript in /path/to/nf-agents/demo/.run/transcript.jsonl)
1. list_runs: run A agitated_raman (lid://b39e82b08e476e3af186314183e72991), run B agitated_gilbert (lid://2385fd1470135349edfcd93be720f943)
2. diff A -> B: params.greeting: "hello" → "bonjour" (nothing else changed)
3. AI step QC_NOTE: model openai/gpt-5-mini, answer "no" (confidence 0.9) to "any lowercase letters?"
4. provenance of run B summary.txt: SUMMARIZE ← TRANSFORM ← GENERATE ← seed.txt
5. check run A summary.txt: Valid (/demo/results-hello/summary.txt)
6. check again after appending a line to it: Mismatch (the file no longer matches its lineage)
tour passed: 6/6 answers as expected

Demo finished in 24s. Research use only, no clinical claims: synthetic data, and the AI answers were replayed from a recorded cassette (demo/cassettes/demo.jsonl),
no model was called.

Ask Claude Code the same questions (the server is read-only and has no network):
  claude mcp add --transport stdio nf-lineage -- docker run -i --rm --network none -v "/path/to/nf-agents/demo/.run:/demo:ro" nf-lineage-mcp:demo --store /demo/.lineage
or with the native binary (cargo install --path "/path/to/nf-agents/crates/nf-lineage-mcp"):
  claude mcp add --transport stdio nf-lineage -- nf-lineage-mcp --store "/path/to/nf-agents/demo/.run/.lineage" --path-map /demo="/path/to/nf-agents/demo/.run"

Then ask, for example:
  What changed between my last two Nextflow runs?
  Where did summary.txt of the latest run come from?
  What did the AI step answer, and which model was it?
  Is results-hello/summary.txt still intact?
Remove it with: claude mcp remove nf-lineage
```

Run names, task hashes and LIDs change on every run. Step 6 appends a line to the demo's own
`demo/.run/results-hello/summary.txt` (the tour refuses any path outside the workdir); the MCP
server itself never modifies anything.

## Timings

Measured on 2026-11-26 on a 12-thread desktop with 16 GB RAM, Linux, Docker Engine 29.7.

| Case | Time | Notes |
| --- | --- | --- |
| Warm (second and later runs) | 24 s | images built, Nextflow image cached; plugin download included |
| `MCP_IMAGE` / `REPLAY_IMAGE` set to existing images | 21 s | no build check |
| First run, building both images from source | 180 s | Rust, distroless and Nextflow base images already pulled |
| Fully cold (no images, no build cache) | not measured | adds pulling `nextflow/nextflow:26.10.7` (≈ 880 MB; its pull took ≈ 23 s, measured separately) and the Rust base image |
| Cold with published images | pending | needs the release pipeline (images on GHCR) |
| CI (`docker` job, step `Local demo`, `:ci` images) | pending | recorded from the first PR run |
| Apple Silicon, WSL2 | not measured | reports welcome |

## Ask Claude Code

Register the server with one of the two lines the script prints (absolute paths), then start
Claude Code anywhere:

```sh
# Docker image, no network, store mounted read-only
claude mcp add --transport stdio nf-lineage -- docker run -i --rm --network none \
  -v "$PWD/demo/.run:/demo:ro" nf-lineage-mcp:demo --store /demo/.lineage

# Native binary; --path-map translates the /demo paths recorded inside the containers
cargo install --path crates/nf-lineage-mcp
claude mcp add --transport stdio nf-lineage -- nf-lineage-mcp \
  --store "$PWD/demo/.run/.lineage" --path-map /demo="$PWD/demo/.run"
```

Example prompts:

- "What changed between my last two runs?"
- "Where did results/summary.txt come from?" (the demo publishes it as
  `results-bonjour/summary.txt` and `results-hello/summary.txt`)
- "What did the AI step answer, and which model was it?"
- "Are my published files still intact?" (after the tour, `results-hello/summary.txt` is not)

Remove it with `claude mcp remove nf-lineage`. Other stores (S3, GCS, Azure) and the JSON form are
in the [`nf-lineage-mcp` README](../crates/nf-lineage-mcp/README.md#use-with-claude-code).

## Re-recording the cassette

The committed `demo/cassettes/demo.jsonl` was recorded from the deterministic fake upstream in
`crates/nf-llm-replay/examples/fake_upstream.rs`, not from a real model. Its fixed structured
answer (`{"answer":"no","confidence":0.9}`) is the correct answer to the question asked, so a real
model gives the same verdict.

If the pipeline, the agent plugin or the request format changes, replay returns HTTP 599 for the
new request and `run.sh` stops with a `DEMO_RECORD=1` hint. To re-record from the fake upstream
(needs a Rust toolchain; the result is byte-identical when nothing changed):

```sh
cargo run -p nf-llm-replay --example fake_upstream -- --port 18181 &
NF_LLM_REPLAY_API_KEY=unused DEMO_RECORD=1 UPSTREAM=http://127.0.0.1:18181 demo/run.sh
```

From a real provider:

```sh
NF_LLM_REPLAY_API_KEY=sk-... DEMO_RECORD=1 UPSTREAM=https://api.openai.com demo/run.sh
```

Record mode runs the proxy and Nextflow on the host network (so a local upstream on `127.0.0.1` is
reachable), passes the key to the proxy by name only, and copies the new cassette over
`demo/cassettes/demo.jsonl` only when both runs succeed. The tour still expects the answer `no`.
Cassettes never store the key; `cargo test` fails if a committed cassette contains a credential or
a host path. State the upstream (and, for a real model, the date) in your PR.

## Cleanup

```sh
rm -rf demo/.run
docker rmi nf-lineage-mcp:demo nf-llm-replay:demo
claude mcp remove nf-lineage
```

The script already removes its containers and network on exit, on failure too.

## Troubleshooting

- **`docker is required`**: install Docker and start the daemon (`docker info` must work without
  `sudo`).
- **`refusing to delete …`**: `DEMO_WORKDIR` points at a non-empty directory that the demo did not
  create. Pick a new or empty directory.
- **Plugin download fails**: the `nf-agent@0.2.0` plugin is fetched from the Nextflow plugin
  registry, the only step that needs the internet. Behind a proxy, make sure Docker containers can
  reach it.
- **HTTP 599 / `the cassette has no answer`**: the request no longer matches the cassette. Re-record
  it (above).
- **Apple Silicon**: the Nextflow image is amd64-only and runs under emulation, which is slower.
  Enable Rosetta for x86/amd64 emulation in Docker Desktop. The Rust images build natively.
- **WSL2**: clone and run inside the Linux filesystem (`~/…`, not `/mnt/c/…`) with Docker Desktop's
  WSL integration enabled; bind mounts from `/mnt/c` are slow and may break file ownership.
- **Files owned by root in `demo/.run`**: should not happen (Nextflow hands them back on exit); if a
  run was killed with `SIGKILL`, remove them with
  `docker run --rm -v "$PWD/demo:/d" alpine rm -rf /d/.run`.
