<p align="center">
  <img src="brand/logo/wordmark-light.svg#gh-light-mode-only" alt="baqueano" height="56">
  <img src="brand/logo/wordmark-dark.svg#gh-dark-mode-only" alt="baqueano" height="56">
</p>

<p align="center"><strong>AI agents inside your Nextflow pipelines. With evals and lineage.</strong></p>

<p align="center"><em>A baqueano is the guide of the pampas who knows the terrain and reads the trail.</em></p>

## What it is

baqueano is an open source toolkit for running AI agents **inside** Nextflow pipelines, with provenance and evals, in your own cloud account.

- **Lineage tooling (Rust).** Read Nextflow's data lineage store from local disk, S3, GCS or Azure. Diff and check runs, trace any output back to its inputs, and expose all of it to agents as read-only MCP tools.
- **Agent modules.** nf-core-compatible processes such as `RUN_METHODS` and `QC_REVIEW`. Each one has typed JSON input and output schemas, and ships with evals and a pass threshold.
- **Open agent protocol.** Modules talk to agents over A2A v1.0 (v0.3 accepted). Rig is the reference runtime, and any A2A framework can serve the same module.
- **Deterministic tests.** `baqueano-replay` records model responses once and replays them, so PR tests need no API keys and no network.
- **Infrastructure.** OpenTofu modules behind a cloud-agnostic compute contract, AWS first.

Your data stays in your account. There is no third-party telemetry by default.

## Try it in 5 minutes

You need Docker (running) and bash. No API keys, no Rust toolchain, no Nextflow install.

```sh
git clone https://github.com/pingedbrain/baqueano.git && cd baqueano && demo/run.sh
```

The script runs a small Nextflow pipeline with an AI step twice (`--greeting hello`, then `--greeting bonjour`) with lineage enabled. The AI step's answers are replayed by `baqueano-replay` from a recorded cassette on an internal Docker network, so no model is called. A scripted MCP session then asks `baqueano-mcp` about the two runs:

```text
1. list_runs: run A agitated_raman (lid://b39e82b0…), run B agitated_gilbert (lid://2385fd14…)
2. diff A -> B: params.greeting: "hello" → "bonjour" (nothing else changed)
3. AI step QC_NOTE: model openai/gpt-5-mini, answer "no" (confidence 0.9) to "any lowercase letters?"
4. provenance of run B summary.txt: SUMMARIZE ← TRANSFORM ← GENERATE ← seed.txt
5. check run A summary.txt: Valid (/demo/results-hello/summary.txt)
6. check again after appending a line to it: Mismatch (the file no longer matches its lineage)
tour passed: 6/6 answers as expected
```

It then prints the commands to ask Claude Code the same questions, with your absolute paths filled in:

```sh
# Docker image (the server is read-only and has no network)
claude mcp add --transport stdio baqueano-lineage -- docker run -i --rm --network none -v "$PWD/demo/.run:/demo:ro" baqueano-mcp:demo --store /demo/.lineage
# or the native binary (cargo install --path crates/baqueano-mcp)
claude mcp add --transport stdio baqueano-lineage -- baqueano-mcp --store "$PWD/demo/.run/.lineage" --path-map /demo="$PWD/demo/.run"
```

Then ask "What changed between my last two runs?" or "Are my published files still intact?". Other stores and options are in the [`baqueano-mcp` README](crates/baqueano-mcp/README.md#use-with-claude-code).

Timings on a 12-thread Linux desktop: about 3 minutes on the first run, which builds both images from source, and 24 s after that. To skip the build, set `MCP_IMAGE` and `REPLAY_IMAGE` to images you already have. The input data is synthetic and the demo is for research use only. The full transcript, timings, troubleshooting and how to re-record the cassette (`DEMO_RECORD=1`) are in [docs/demo.md](docs/demo.md).

## Status

**Pre-alpha.** APIs, formats and module contracts will change. This is research software: it makes no clinical claims and must not be used for diagnosis.

Implemented on `main` (not yet released):

- Lineage fixtures for Nextflow 25.04.8 through 26.10.7, including agent runs.
- `baqueano-lineage`: a tolerant parser, a store on local disk and object storage, `find`, `view`, `provenance`, `check` and semantic and raw `diff`, with contract tests against the `nextflow lineage` CLI and S3 tests on MiniStack.
- `baqueano-replay`: record and replay for OpenAI-compatible chat completions, plus streaming for the Responses and Messages APIs, with a Docker image and an offline test against Nextflow's `agent` primitive.
- `baqueano-mcp`: a read-only MCP server over stdio with `list_runs`, `view`, `find`, `diff`, `check` and `provenance`, with a Docker image.
- The local demo above, run on every PR in CI.

Next up is the release pipeline: crates on crates.io and images on GHCR.

## Roadmap

Work is planned in ordered milestones:

1. v0.1, foundations: lineage, MCP, replay.
2. v0.2, agent modules and evals.
3. v0.3, a public benchmark.
4. v0.4, infrastructure.
5. v0.5, compliance profiles.
6. v0.6, ecosystem integrations.

Each milestone has technical "done when" criteria. See [ROADMAP.md](ROADMAP.md) for the plan and the extension points open to contributions, and [docs/architecture.md](docs/architecture.md) for the design and the agent architecture decision.

## How to contribute

It's early, so the best contributions right now are feedback and use cases:

- Open an issue describing how you would use agents in your pipelines.
- Propose a new agent module or compute target using the issue templates.
- Read the [Code of Conduct](CODE_OF_CONDUCT.md) and the [security policy](SECURITY.md).

PRs should be small, include tests, and declare any AI assistance used (see the PR template).

## License

[Apache-2.0](LICENSE).

---

An independent community project. Not affiliated with or endorsed by Seqera. Nextflow is a registered trademark of Seqera Labs; cloud and model names belong to their owners.
