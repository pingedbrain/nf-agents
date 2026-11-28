# Roadmap

baqueano is built in milestones. Each one has a goal, a list of deliverables, and technical "done when" criteria that CI or a reproducible run can check. Milestones are ordered, not scheduled: one starts when the contracts it depends on are stable.

The architecture behind this plan is in [docs/architecture.md](docs/architecture.md).

**Status markers:** checked items are merged on `main` (not yet released), **in progress** means partly implemented, and unmarked items are planned.

## Tracks

Work is grouped into tracks that run across milestones.

| Track | Scope | Milestones |
| --- | --- | --- |
| Lineage & tooling | `baqueano-lineage`, `baqueano-mcp`, `baqueano-replay`, fixtures | v0.1, v0.6 |
| Agent runtime & modules | Module contract, A2A runtime images, agent modules | v0.2, v0.5, v0.6 |
| Evals & benchmark | `nf-evals`, graders, labelled cases, NF-AgentBench | v0.2, v0.3 |
| Infrastructure | Compute contract and OpenTofu modules, AWS first | v0.4, v0.6 |
| Security & compliance profiles | Hardened profiles, policy scanning, evidence packs | v0.5 |
| Site & docs | README demo, docs site, leaderboard, lineage explorer | v0.1, v0.2, v0.3, v0.6 |
| Ecosystem integrations | Telemetry exporters, remote agent hosting, editor and coding-agent integrations | v0.2, v0.6 |

## v0.1: Foundations

**Goal:** read Nextflow lineage, expose it to agents over MCP, and make model calls replayable, all testable offline.

Deliverables:

- [x] Cargo workspace and CI (rustfmt, clippy with `-D warnings`, cargo-deny, cargo-machete, tests), contributor guide, code of conduct, security policy, issue templates.
- [x] Lineage fixtures: a minimal 3-process pipeline; stores and `nextflow lineage` CLI output for Nextflow 25.04.8, 25.10.9, 26.04.9 and 26.10.7, plus an `AgentRun` store; an offline generator; a weekly, non-blocking drift check against the latest edge release.
- [x] `baqueano-lineage`: a tolerant record model (both on-disk envelopes, `AgentRun`, unknown kinds and fields preserved); the `LineageStore` trait with an `object_store` backend (local, S3, GCS, Azure); `find`; `check` (Nextflow's metadata checksum); semantic and raw `diff`; `view`; `provenance`; a CLI contract suite; S3 integration tests against MiniStack.
- [x] `baqueano-replay`: `record`, `replay` and `passthrough` modes; OpenAI chat completions (JSON, structured output, tool calls); SSE for the Responses and Messages APIs; normalized request keys; JSONL cassettes without credentials; a distroless image; an offline end-to-end test with Nextflow's `agent` primitive.
- [x] `baqueano-mcp`: a read-only MCP server over stdio with `list_runs`, `view`, `find`, `diff`, `check` and `provenance`, and a configurable cap on response size.
- [x] Local demo with Docker: an MCP client asks what changed between two pipeline runs.
- [ ] Release pipeline: crates on crates.io at `0.1.0`, images on GHCR. **In progress:** the tag workflow is on `main`; `0.1.0` is not published yet.

**Done when:**

- CI is green on every PR without secrets (jobs `lint`, `test`, `contract`, `integration`, `docker`).
- `baqueano-lineage` parses every fixture store, and `list`, `view`, `find`, `diff` and `check` match the recorded `nextflow lineage` output for every Nextflow version in the matrix.
- The semantic diff of the two fixture runs reports `params.greeting` and nothing else.
- In `replay` mode the same request returns a byte-identical response, SSE included. An unrecorded request returns HTTP 599 naming the request and never reaches the network, which is tested on an internal Docker network.
- No cassette contains an `Authorization` header or an API key (enforced by a test).
- Every MCP tool has a test that calls it through the MCP protocol against the fixtures, and `provenance` of a published file reaches the pipeline inputs.
- The demo runs from a clean checkout with Docker only, no API keys, in under 5 minutes.
- `cargo package` succeeds for the three crates, and a `v*` tag publishes crates and images.

## v0.2: Agents & evals

**Goal:** ship the first agent modules as nf-core-compatible processes that speak A2A, each with evals and a pass threshold.

Deliverables:

- [ ] Module contract: `meta.yml` plus JSON Schemas for input and output; the A2A image contract (serve on `127.0.0.1:${A2A_PORT}`, declare one skill with a JSON data part in and a JSON artifact out, read `OPENAI_BASE_URL`, spawn `baqueano-mcp` over stdio); `params.agent_url` to call a remote A2A agent instead.
- [ ] `baqueano-runtime`: the reference runtime image, built on Rig with `rig-rmcp` for MCP tools and `rig-a2a` for serving. It serves A2A v1.0 at the root and v0.3 at `/v03`.
- [ ] A small A2A client for module scripts that negotiates v1.0 or v0.3 from the agent card.
- [ ] `RUN_METHODS`: writes a methods section from lineage, with a deterministic check that every cited tool and version exists in the run.
- [ ] `QC_REVIEW`: reviews MultiQC output against a labelled case set of at least 30 cases.
- [ ] Interop example: the same module served by a Strands Agents image.
- [ ] Provenance link: module output carries the A2A `taskId`, the `contextId` and the cassette ID, and `baqueano-mcp provenance` resolves them.
- [ ] `nf-evals`: the `Grader` trait, JSONL cases, N repetitions, a pass threshold and a spend cap. Its A2A target sends messages built from fixtures and scores the returned artifact with deterministic graders and an LLM judge.
- [ ] `nf-otel`: OpenTelemetry export with the GenAI semantic conventions to any OTLP backend. Off by default.
- [ ] Docs site (Astro + Starlight): landing page, getting started, architecture, module catalog. **in progress** (landing and docs are live; module catalog pending)

**Done when:**

- Each module passes its eval threshold in the eval job, which runs nightly or on a label and is bounded by a spend cap.
- Module nf-test suites pass in PR CI against `baqueano-replay` cassettes, with no API keys.
- The same module passes the same nf-test with the reference runtime and with one non-Rust A2A runtime, changing only the container.
- The A2A client completes a round trip against both a v1.0 and a v0.3 server in CI.
- The `RUN_METHODS` eval fails when a cited tool or version is absent from the run.
- Modules follow the nf-core/modules layout and install with `nf-core modules --git-remote`.
- The docs site builds in CI with link checking.

## v0.3: Benchmark

**Goal:** publish NF-AgentBench, a reproducible benchmark of agents on real pipeline tasks.

Deliverables:

- [ ] Public case format: inputs, labels, data license, provenance and a contribution guide.
- [ ] At least 100 labelled cases across QC review, methods writing and failed-run diagnosis.
- [ ] A benchmark runner on `nf-evals` that runs a model × case matrix and reports cost per case and variance across repeats.
- [ ] A leaderboard covering proprietary and open-weight models, published on the site.

**Done when:**

- Every case validates against the published schema in CI, and every case declares a data license and synthetic or public provenance.
- Any leaderboard row reproduces offline from committed cases and cassettes with one command.
- The leaderboard page is generated from committed results, not edited by hand.

## v0.4: Infrastructure

**Goal:** run pipelines with agent modules in the user's own cloud account through a cloud-agnostic compute contract, with AWS as the first target.

Deliverables:

- [ ] Compute contract: every `infra/compute/<target>` outputs `queue`, `work_dir`, `nextflow_config` and an optional `head_job`, checked by a shared contract test.
- [ ] OpenTofu modules `infra/core` and `infra/compute/aws-batch` (EC2 and Fargate).
- [ ] Local AWS emulation in CI with MiniStack, including the AWS Batch surface the modules need.
- [ ] Amazon Bedrock as a model backend for agent modules and for Nextflow's `agent` primitive.
- [ ] An end-to-end job: nf-core/sarek with `-profile test` on AWS Batch, with agent modules.

**Done when:**

- `tofu test` with mocks passes in PR CI for every target, without cloud credentials.
- Every compute target passes the shared contract test.
- The sarek end-to-end job is green on AWS Batch. It runs weekly and on releases, in a separate job with scoped credentials.
- The default configuration sends no data or telemetry outside the user's account, which a policy test checks.

## v0.5: Compliance profiles

**Goal:** provide hardened deployment profiles with machine-generated evidence for controlled-data environments.

Deliverables:

- [ ] A `secure` profile: encryption at rest and in transit, no NAT gateway, VPC endpoints, immutable logs.
- [ ] Policy scanning in CI: Checkov, Trivy, Conftest (Rego) and Prowler.
- [ ] An evidence pack mapped to NIST SP 800-171, generated for every release.
- [ ] `SAMPLE_CHECK` and `VARIANT_SUMMARY` modules, with evals on expert-labelled cases.

**Done when:**

- The scanners run on every PR and the `secure` profile has no high-severity findings without a documented waiver.
- Every release carries its evidence pack, and regenerating it from the same tag gives the same result.
- The new modules pass their eval thresholds, and their outputs carry the research-use notice.

Profiles are technical controls and evidence. They are not a certification.

## v0.6: Ecosystem

**Goal:** make lineage, agents and compute reachable from more clouds, hosts and tools without changing the core contracts.

Deliverables:

- [ ] A browser lineage explorer: `baqueano-lineage` compiled to WebAssembly.
- [ ] `baqueano-mcp` over Streamable HTTP (MCP 2026-07-28), deployable behind an MCP gateway such as AgentCore Gateway.
- [ ] Agent modules calling remote A2A agents hosted on AWS AgentCore Runtime through `params.agent_url`.
- [ ] Integrations for coding agents and editors: skills for Claude Code, a Kiro power, and optionally an Agent Client Protocol front to drive modules from an editor.
- [ ] Compute targets for GCP Batch, Azure Batch and Kubernetes.
- [ ] Exploratory: a Nextflow `agent` runner plugin that forwards to an A2A agent, once the upstream runner interface stabilizes.

**Done when:**

- The WebAssembly build parses every fixture store in a headless-browser test.
- The Streamable HTTP server passes the same MCP tool tests as the stdio server.
- Each new compute target passes the shared contract test with mocks.

## Extension points

These are the places where new pieces plug in without touching the core. Each one is defined by a contract, and an implementation is accepted when it passes that contract's tests.

| Extension point | Contract | Examples |
| --- | --- | --- |
| Compute target | OpenTofu module with outputs `queue`, `work_dir`, `nextflow_config`, `head_job` (optional); shared contract test | GCP Batch, Azure Batch, Kubernetes |
| Lineage backend | `LineageStore` Rust trait (`list_runs`, `get`, `find`) | a platform API, a database-backed store |
| Grader | `Grader` Rust trait in `nf-evals` | deterministic check, LLM judge, human label |
| Telemetry exporter | OpenTelemetry spans with the GenAI semantic conventions | any OTLP backend, Langfuse, Jaeger |
| Agent runtime | A2A server that honours the module's input and output JSON Schemas, reads `OPENAI_BASE_URL` and uses MCP for tools | Rig (reference), Strands Agents, LangGraph, Google ADK |
| Agent module | nf-core layout, `meta.yml`, input and output JSON Schemas, evals with a threshold | `FUSION_REVIEW`, `CNV_SUMMARY` |
| Compliance profile | Rego policies plus Prowler checks and an evidence mapping | HIPAA, ISO/IEC 27001, GDPR |
| Model backend | OpenAI-compatible endpoint | Amazon Bedrock, local models |

Issues for these are labelled [`extension-point`](../../issues?q=is%3Aissue+is%3Aopen+label%3Aextension-point) and [`help wanted`](../../issues?q=is%3Aissue+is%3Aopen+label%3A%22help+wanted%22).
