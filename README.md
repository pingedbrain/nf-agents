<p align="center">
  <img src="brand/logo/wordmark-light.svg#gh-light-mode-only" alt="nf-agents" height="56">
  <img src="brand/logo/wordmark-dark.svg#gh-dark-mode-only" alt="nf-agents" height="56">
</p>

<p align="center"><strong>AI agents inside your Nextflow pipelines. With evals and lineage.</strong></p>

## What it is

nf-agents is an open source toolkit for running AI agents **inside** Nextflow pipelines, with provenance and evals, in your own cloud account.

- **Lineage tooling (Rust).** Read Nextflow's data lineage store from local disk, S3, GCS or Azure. Diff and check runs, trace any output back to its inputs, and expose all of it to agents as read-only MCP tools.
- **Agent modules.** nf-core-compatible processes such as `RUN_METHODS` and `QC_REVIEW`. Each one has typed JSON input and output schemas, and ships with evals and a pass threshold.
- **Open agent protocol.** Modules talk to agents over A2A v1.0 (v0.3 accepted). Rig is the reference runtime, and any A2A framework can serve the same module.
- **Deterministic tests.** `nf-llm-replay` records model responses once and replays them, so PR tests need no API keys and no network.
- **Infrastructure.** OpenTofu modules behind a cloud-agnostic compute contract, AWS first.

Your data stays in your account. There is no third-party telemetry by default.

## Status

**Pre-alpha.** APIs, formats and module contracts will change. This is research software: it makes no clinical claims and must not be used for diagnosis.

Implemented and in review:

- Lineage fixtures for Nextflow 25.04.8 through 26.10.7, including agent runs.
- `nf-lineage`: a tolerant parser, a store on local disk and object storage, `find`, `view`, `provenance`, `check` and semantic and raw `diff`, with contract tests against the `nextflow lineage` CLI and S3 tests on MiniStack.
- `nf-llm-replay`: record and replay for OpenAI-compatible chat completions, plus streaming for the Responses and Messages APIs, with a Docker image and an offline test against Nextflow's `agent` primitive.

Next up are the `nf-lineage-mcp` server and a local demo that runs with Docker only and no API keys.

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
