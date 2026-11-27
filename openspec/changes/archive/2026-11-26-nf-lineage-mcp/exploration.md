## Exploration: nf-lineage-mcp

### Current State

- Workspace (`main`): one stub crate `crates/nf-lineage`, `rust-version = "1.85"`, toolchain 1.100.0, lints deny `unwrap/expect/panic/indexing`, `missing_docs` warn (fatal under `-D warnings`). CI jobs `lint`, `test`.
- Upstream changes this one builds on (all planned, none merged):
  - `nf-lineage` (3 PRs): `Lid`, `Record`, `FindQuery::{new, parse_args, terms, matches}`, `#[async_trait] LineageStore::{list_runs, get, find}`, `RunSummary {timestamp, run_name, session_id, lid}`, `ObjectLineageStore::{open, open_with_options}`, `check(store, lid, &CheckOptions{path_map, storage_options}) -> CheckReport`, `diff`/`diff_raw`, `view`, `provenance`. Errors: `#[non_exhaustive] LineageError` with `InvalidLid, InvalidQuery, NotFound, InvalidFormat, UnsupportedVersion, UnexpectedKind, UnsupportedChecksum, Unsupported, Storage, Io`. `view` and `provenance` land in its PR C, so this change needs **all of nf-lineage** merged.
  - `lineage-fixtures`: `fixtures/lineage/<ver>/{.lineage, manifest.json, oracle/<case>.{args,out,err,exit}, oracle/ids.json}`; recorded root `/nf-agents-fixture`; core versions 25.04.8, 25.10.9, 26.04.9, 26.10.7 (2 runs each, `greeting=hello|bonjour`) plus `26.10.7-agent`. `ids.json` maps `RUN1`, `RUN2`, `TASK_<P>_RUN<n>` to LIDs; published file LIDs are `<RUN>/summary.txt` (see `oracle/view-published-file.args`).
  - `nf-llm-replay`: introduces `[workspace.dependencies]` (PR A: anyhow, serde, serde_json, thiserror, tempfile…; PR B: tokio, clap, tracing, tracing-subscriber), the musl-static `distroless/static-debian12:nonroot` Docker pattern, root `.dockerignore`, and the CI `docker` job (PR E).
- WorkflowRun records carry no "pipeline name" field as such: `metadata.projectName` (`main.nf` for a local script, `nf-core/sarek` for a remote one) and `workflow.repository` are the usable signals. `.history` timestamps are text `YYYY-MM-DD HH:MM:SS <zone>`.

### Empirical checks (verified locally with a throwaway rmcp server/client probe)

| Question | Result (rmcp 3.12.0) |
|---|---|
| Tool annotations | `#[tool(annotations(read_only_hint = true, idempotent_hint = true, destructive_hint = false, open_world_hint = false))]` → `annotations` in `tools/list` ✔ |
| Dynamic server version/instructions | `tool_handler(version = env!(..))` does not compile (literal only) → hand-written `get_info` (macro skips its own when present) ✔ |
| Bad arguments (wrong type, unknown field with `deny_unknown_fields`) | returned as `isError: true` tool result ("failed to deserialize parameters: …"), **not** -32602 (router `into_tool_argument_error`, SEP-1303) |
| Unknown tool | JSON-RPC `-32602 tool not found` |
| Explicit `Err(ErrorData::invalid_params)` | JSON-RPC `-32602` (`docs/verification.md` §5) |
| Negotiated protocol (rmcp client) | `2025-11-25` |
| Scripted stdio with immediate EOF | all responses flushed, exit 0, in-flight calls complete (tested with an 800 ms tool) → a raw JSON-RPC e2e test needs no client feature |
| `cargo package` of a crate with an unpublished path dep | works only as `cargo package -p nf-lineage -p nf-lineage-mcp` (tmp registry); `-p nf-lineage-mcp` alone fails |

External facts: MCP spec 2025-11-25 "Error Handling": input validation errors **SHOULD** be tool execution errors (`isError: true`) so models can self-correct; protocol errors are for unknown tools/malformed requests/server errors. Claude Code: `claude mcp add [options] <name> -- <command> [args...]` (options before the name; `--env` must not be immediately followed by the name), warns at 10 000 tokens of MCP output, default `MAX_MCP_OUTPUT_TOKENS` 25 000, and persists text results > 50 000 chars to a file.

### Affected Areas

- `crates/nf-lineage-mcp/` — new binary crate (lib + bin so protocol tests run in-process).
- `Cargo.toml` — `[workspace.dependencies]` entries; `rust-version` to 1.88 (rmcp MSRV).
- `Cargo.lock`, `deny.toml` (only if `cargo deny` reports something new; rmcp without default features avoids `chrono`).
- `.github/workflows/ci.yml` — `docker` job: second image + stdio smoke.

### Approaches

1. **Thin adapter over `nf-lineage`** — one `LineageServer { Arc<dyn LineageStore>, Settings }`, six `#[tool]` methods, each = parse params → one library call → `fit` (size cap) or `tool_error` (error mapping). All behaviour lives in the library.
   - Pros: ~250 lines of server code; library already tested against the CLI oracle; `dyn LineageStore` keeps backends pluggable.
   - Cons: `list_runs` pipeline filter needs one `get` per run (N reads).
   - Effort: Low/Medium.
2. **Rich server (resources, prompts, structured output schemas, streamable HTTP)** — expose records as MCP resources, `outputSchema` per tool.
   - Pros: richer clients.
   - Cons: out of scope for 0.1 (stdio only), doubles payload (`structuredContent` + text), more surface to test.
   - Effort: High.

Error mapping options: (a) everything `-32602`/`-32603` JSON-RPC errors — clients render them opaquely, agent can't self-correct; (b) **tool errors (`isError`) for every lineage/validation failure, JSON-RPC errors only for what rmcp itself rejects (unknown tool) and internal bugs** — matches spec 2025-11-25 and rmcp's own handling of bad arguments.

Truncation options: (a) raw byte cut of every payload — breaks JSON; (b) **arrays trimmed item-wise (valid JSON), other payloads cut at a char boundary, plus a separate notice block**; (c) pagination cursors — speculative.

`check` path mapping options: (a) tool parameter — lets the model make the server stat arbitrary host paths (existence oracle); (b) **operator-only `--path-map FROM=TO`** — mapping is a deployment fact, not a per-call choice.

### Recommendation

Approach 1 with error option (b), truncation (b), path-map (b). Default response cap 32 KiB (below Claude Code's 10k-token warning and 50k-char persist threshold). Raw JSON-RPC stdio e2e via `std::process::Command` (no `transport-child-process`, no `assert_cmd`). Docker: copy the nf-llm-replay pattern. PR A: crate + `list_runs`/`view`/`find` + truncation + errors + protocol tests; PR B: `diff`/`check`/`provenance` + e2e + Dockerfile + README + CI.

### Risks

- Hard dependency on three unmerged changes (nf-lineage A–C, lineage-fixtures, nf-llm-replay B/E for workspace deps and `docker` job).
- rmcp ships weekly; macro/model names shift between minors → pin `3.12`, lock file.
- Image size with object_store `aws`/`gcp`/`azure` + rustls unknown (llm-replay: 10.5 MB without object_store).
- `list_runs` cost on large S3 stores (N `get`s, 16 concurrent).
- Choosing tool errors over the conventional "invalid params → -32602": justified by the MCP spec and rmcp behaviour, recorded in the design.

### Ready for Proposal

Yes.
