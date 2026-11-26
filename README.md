<p align="center">
  <img src="brand/logo/wordmark-light.svg#gh-light-mode-only" alt="nf-agents" height="56">
  <img src="brand/logo/wordmark-dark.svg#gh-dark-mode-only" alt="nf-agents" height="56">
</p>

<p align="center"><strong>AI agents inside your Nextflow pipelines. With evals and lineage.</strong></p>

> **Status: pre-alpha.** Nothing here is usable yet. APIs, formats and module contracts will change.
> This is research software: it makes no clinical claims and must not be used for diagnosis.

## What it is

nf-agents is an open source toolkit for running AI agents **inside** Nextflow pipelines, in your own cloud account:

- **Lineage tooling (Rust):** read Nextflow's data lineage store, diff and check runs, and expose it to agents over MCP.
- **Agent modules:** nf-core-compatible modules (`RUN_METHODS`, `QC_REVIEW`, ...) that ship with evals and a pass threshold.
- **Deterministic tests:** `nf-llm-replay` records model responses once and replays them, so PR tests need no API keys.
- **Infrastructure:** OpenTofu modules behind a cloud-agnostic compute contract, AWS first.

Your data never leaves your account. No third-party telemetry by default.

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
