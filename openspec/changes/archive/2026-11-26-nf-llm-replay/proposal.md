# Proposal: nf-llm-replay (record / replay / passthrough LLM proxy)

## Intent

Agent-module tests must be deterministic and need no API keys (project principles). `nf-llm-replay`
sits between Nextflow's `agent` primitive (`agent.baseUrl`) and the model provider: it records real
responses once into a reviewable cassette and replays them offline. The original plan as adjusted by
`docs/verification.md` section 4.3 (non-streaming chat completions first; SSE for
`/v1/responses` and `/v1/messages`; `<cwd>` normalization; the Nextflow example is mandatory).

## Scope

### In Scope
- Binary crate `crates/nf-llm-replay` (lib + bin), version `0.1.0`, `publish = false` until the release change.
- Modes `replay` (never touches the network; miss → HTTP 599 + JSON naming the request and the record command), `record` (fill missing keys, single-flight per key), `passthrough`.
- Any method/path proxied generically; guaranteed and tested: `POST /v1/chat/completions` (JSON; `response_format` json_schema strict, tools, multi-turn tool round trips, concurrency, `Upgrade: h2c`, `Bearer nxf-no-credential`), SSE for `POST /v1/responses`, `POST /v1/messages?beta=true` and chat-completions `stream:true` (free with the generic design).
- Key = sha256 of method + path (no query) + canonical JSON with volatile parts removed (defaults: top-level `user`, `metadata`, `prompt_cache_key`; `<cwd>…</cwd>` text rule); configurable.
- JSONL cassette sorted by key, atomic rewrite, no credentials/headers/query strings; secret guard.
- Config via flags + `NF_LLM_REPLAY_*` env; upstream key from env only, injected; client credentials never forwarded.
- Docker image (musl static, distroless `nonroot`, ~10 MB); CI `docker` job builds it and runs the offline e2e.
- `docs/examples/nf-llm-replay/`: minimal pipeline (plain, structured, tool-calling agents), committed cassette, `e2e.sh` (Nextflow 26.10.7 + replay container on an `--internal` network), `record.sh`.

### Out of Scope
- crates.io/GHCR publishing (release change), multi-arch images.
- Headers in the key; WebSocket/Realtime APIs; gzip upstream bodies; response-time simulation.
- e2e for the `pi` runner (needs its container + RPC broker on host network): documented only.
- Multiple processes sharing one cassette in record mode.

## Capabilities

### New Capabilities
- `llm-replay-proxy`: modes, HTTP surface, upstream forwarding/auth, 599 miss contract, CLI/env config, no-network guarantee.
- `llm-replay-cassette`: request normalization and keying, cassette format, ordering, concurrency safety, secret policy, SSE event storage.
- `llm-replay-packaging`: Docker image, offline Nextflow example and e2e script.

### Modified Capabilities
- `ci-pipeline`: new `docker` job (image build + offline e2e, no secrets); `deny.toml` allowlist gains `CDLA-Permissive-2.0`.

## Approach

Generic catch-all axum handler; endpoint knowledge only for the miss summary and default-upstream hint. SSE detected by upstream `content-type`, stored as raw event strings (terminator included) → byte-exact replay. See exploration.md for captures and experiments.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/nf-llm-replay/` | New | crate, tests, `Dockerfile` |
| `Cargo.toml`, `Cargo.lock` | Modified | `[workspace.dependencies]`, deps |
| `deny.toml` | Modified | `CDLA-Permissive-2.0` |
| `.dockerignore` | New | build context |
| `.github/workflows/ci.yml` | Modified | `docker` job |
| `docs/examples/nf-llm-replay/` | New | example + e2e |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Preview `agent` request shapes change → stale cassettes | High | 599 names request + record command; re-capture per stable |
| Output-affecting headers (`anthropic-beta`) not keyed | Med | documented; the original plan mandates header-free keys |
| Plugin registry down during CI e2e prefetch | Low | retry; failure clearly labelled |
| `serde_json/preserve_order` unified by other crates | Med | own sorted canonical writer + test |

## Rollback Plan

New crate and example are additive: revert the PR(s). CI `docker` job and the `deny.toml` line revert with them; nothing else depends on them.

## Dependencies

- axum 0.8, tokio 1, reqwest 0.13 (rustls), clap 4 (`derive`, `env`), tracing, sha2, regex; Docker images `rust:1.100.0-alpine`, `gcr.io/distroless/static-debian12:nonroot`, `nextflow/nextflow:26.10.7`.

## Success Criteria

- [ ] Every acceptance criterion of the original plan maps to a passing test (see specs).
- [ ] Offline e2e passes in CI with no secrets; replay container on an internal network.
- [ ] Five PRs, each ≤ ~800 hand-written lines, each green (fmt, clippy `-D warnings`, tests).
