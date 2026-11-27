# Proposal: nf-lineage-mcp 0.1

## Intent

Agents (Claude Code, Kiro, Rig, AgentCore) cannot ask about Nextflow runs. A read-only stdio MCP server over `nf-lineage` answers: which runs exist, what a record says, what changed between runs, whether outputs are intact, where a file came from.

## Architecture Context

`nf-lineage-mcp` is the tool layer of nf-agents (`docs/architecture.md`). Agent runtimes consume it over MCP: Rig through `rig-rmcp` is the reference runtime, and any MCP-capable framework (Strands, LangGraph, Claude Code, editors) reaches it the same way. Agents themselves are served to Nextflow modules over A2A; MCP is never the agent boundary. The server therefore stays framework-neutral: no tool, parameter or output depends on a particular agent runtime.

## Scope

### In Scope
- Binary crate `nf-lineage-mcp` 0.1.0 (lib + bin), rmcp 3.12, stdio only.
- Tools `list_runs`, `view`, `find`, `diff`, `check`, `provenance`: schemars params, LLM-oriented descriptions, annotations `readOnlyHint`/`idempotentHint`/`!destructiveHint`/`!openWorldHint`.
- `--store`/`NF_LINEAGE_STORE`, `--max-response-bytes` (default 32 KiB) with truncation notice, operator-only `--path-map`, logs to stderr only.
- Error mapping (tool errors with hints; JSON-RPC errors only for protocol/internal faults); never panic.
- Tests: every tool over MCP (duplex) against fixtures; raw stdio e2e of the built binary.
- README (`claude mcp add`, Docker), Dockerfile (musl static, `distroless/static:nonroot`), CI `docker` job extension.

### Out of Scope
- Streamable HTTP, resources/prompts, `outputSchema`, auth.
- `render`, writes of any kind, launching runs.
- Publishing to crates.io/GHCR (`release` change).
- Path mapping or storage options as tool parameters.

## Capabilities

### New Capabilities
- `lineage-mcp`: server contract (tools, params, outputs, errors, truncation, config, logging, packaging, docs).

### Modified Capabilities
- `ci-pipeline`: `docker` job also builds and smoke-tests the `nf-lineage-mcp` image.

## Approach

Thin adapter: `LineageServer { Arc<dyn LineageStore>, Settings }`; each tool = validate → one `nf-lineage` call → `fit` (size cap) or `tool_error`. Two PRs: **A** crate, `list_runs`/`view`/`find`, truncation, errors, protocol tests; **B** `diff`/`check`/`provenance`, stdio e2e, Dockerfile, README, CI.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/nf-lineage-mcp/` | New | crate, tests, Dockerfile, README |
| `Cargo.toml` | Modified | workspace deps; `rust-version` 1.88 |
| `Cargo.lock` | Modified | rmcp, schemars |
| `.github/workflows/ci.yml` | Modified | `docker` job steps |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Upstream changes late/different | Med | hard ordering in tasks; API pinned in design |
| rmcp API churn | Med | pin `3.12`, `--locked`, probe notes |
| Image > 30 MB (object_store clouds) | Low | size gate; measure in PR B |
| Clients expecting JSON-RPC `-32602` for invalid params | Low | MCP spec 2025-11-25 + rmcp behaviour cited in design |
| Slow `list_runs` on big stores | Low | concurrent reads; documented |

## Rollback Plan

Revert PR B then PR A: removes the crate, workspace entries and `docker` job steps. No data, no consumers.

## Dependencies

- Merged: `nf-lineage` (A–C), `lineage-fixtures`, `nf-llm-replay` PR B (workspace deps) and PR E (`docker` job, `.dockerignore`) — otherwise PR B creates them.

## Success Criteria

- [ ] Every tool tested through MCP against fixtures.
- [ ] `diff` of the two minimal runs = `params.greeting` only, every core version.
- [ ] `provenance` of `summary.txt` reaches `seed.txt`.
- [ ] Stdio e2e and Docker smoke green; README has `claude mcp add`.
- [ ] fmt, clippy, tests, deny, machete green; each PR ≤ ~800 lines.
