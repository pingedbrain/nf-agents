# Design: nf-lineage-mcp 0.1

## Technical Approach

Thin adapter over `nf-lineage` (spec `lineage-mcp`). `LineageServer` holds `Arc<dyn LineageStore>` + `Settings`; each `#[tool]` method validates params, makes one library call and hands the outcome to `respond` (size cap or error mapping). No lineage logic is duplicated. Lib + bin in one crate so protocol tests drive the real rmcp router in-process over `tokio::io::duplex`; one test spawns the built binary over stdio. Evidence: `exploration.md` (rmcp behaviour verified locally) and `docs/verification.md` §5.

Architecture context (`docs/architecture.md`): this crate is the framework-neutral tool layer. Agent runtimes consume it over MCP (Rig via `rig-rmcp` is the reference; Strands or any MCP-capable framework works the same); agents are served to modules over A2A. Nothing in the tool contract may assume a specific runtime.

## Architecture Decisions

| Topic | Options | Tradeoff | Decision |
|---|---|---|---|
| Error mapping | JSON-RPC errors (-32602/-32603) / `isError` tool results | MCP 2025-11-25: validation errors SHOULD be tool errors so the model self-corrects; clients render protocol errors opaquely; rmcp 3.12 already turns bad arguments into `isError` | **tool results** `<class>: <msg> Hint: …` for every lineage/validation failure; JSON-RPC only for unknown tool (rmcp) and internal faults. Deliberately not the conventional "invalid params → -32602" |
| Mismatch in `check` | tool error / result | a mismatch is an answer, not a failure | `CheckReport` with `isError: false` |
| Truncation | byte cut / item-wise arrays + byte cut / cursors | cursors = state + params nobody asked for | **arrays item-wise (valid JSON), else cut at char boundary; extra `[truncated] …` block; total ≤ cap** |
| Default cap | 64 KiB / 32 KiB | Claude Code warns at 10k tokens and files results > 50 000 chars | **32 768 bytes**, min 1024 |
| Output encoding | `structuredContent` + text / compact text / pretty text | structured doubles payload, pretty +30 % bytes | **compact JSON text**, one block; raw diff as plain text |
| Path mapping | tool param / server flag | param lets the model stat arbitrary host paths | **`--path-map FROM=TO` only**; storage options only via env |
| list_runs order | store order / newest first | truncation keeps the prefix | **newest first** |
| Pipeline name | new field in nf-lineage / derive here | no such field in records | `workflow.repository` else `metadata.projectName` (local: `main.nf`) |
| list_runs reads | sequential / `buffer_unordered` | needs `futures` dep | **sequential `get` after date filter** (`ponytail:` comment: N reads, parallelize if S3 latency hurts) |
| `find` input | one string / typed fields + `terms` | `parse_args` URL-decodes (`+`→space) | `type`, `labels`, `name` via `FindQuery::new` (no decoding), `terms` via `parse_args`, merged through `terms()` |
| Server info | macro attrs / hand `get_info` | macro needs literals (verified) | **hand-written `get_info`**, instructions include store + cap |
| Unknown fields | ignore / `deny_unknown_fields` | typos silently ignored | **deny** → `isError` from rmcp, mapped class `invalid_argument` in docs |
| Logging | eprintln / tracing | rmcp logs via tracing | `tracing-subscriber` fmt → **stderr**, `NF_LINEAGE_MCP_LOG`, no ANSI |
| E2E transport | rmcp `transport-child-process` / assert_cmd / std `Command` | EOF flush verified | **std `Command`**, scripted `tests/data/session.jsonl` (also used by D-smoke); no extra deps |
| Dockerfile | shared ARG file / own | different entry/env | own, same pattern as nf-llm-replay |
| MSRV | 1.85 / 1.88 | rmcp needs 1.88, members inherit | workspace `rust-version = "1.88"` |
| Packaging check | `-p nf-lineage-mcp` / both crates | unpublished path dep fails alone (verified) | `cargo package -p nf-lineage -p nf-lineage-mcp` |

## Data Flow

```
stdin ─JSON-RPC─▶ rmcp ToolRouter ─Parameters<T> (serde fail → isError)─▶ LineageServer::<tool>
   ─▶ Lid::parse / FindQuery / dates ─▶ nf_lineage::{list_runs,get,find,view,diff,diff_raw,check,provenance}(&dyn LineageStore)
   ─Ok(T: Serialize)─▶ respond::fit_json|fit_text(cap) ─▶ CallToolResult{1–2 text blocks}
   ─Err(ToolError)──▶ respond::tool_error ─▶ CallToolResult{isError, "<class>: … Hint: …"} + tracing::warn
stdout ◀─JSON-RPC── rmcp            logs ─▶ stderr
```

## File Changes

| File | PR | Est. |
|---|---|---|
| `Cargo.toml` (`rust-version` 1.88; workspace deps `tokio`, `clap`, `tracing`, `tracing-subscriber` if absent) | A/B | 10 |
| `crates/nf-lineage-mcp/Cargo.toml` | A (+B bin deps) | 35 |
| `src/lib.rs` docs + re-exports | A | 20 |
| `src/respond.rs` + tests | A | 100+60 |
| `src/server.rs` list_runs/view/find + instructions | A | 210 |
| `tests/common/mod.rs` | A (+B materialize) | 95+25 |
| `tests/protocol.rs` | A 190, B 95 | 285 |
| `src/server.rs` diff/check/provenance | B | 90 |
| `src/config.rs` + tests, `src/main.rs` | B | 75+30 |
| `tests/stdio.rs`, `tests/data/session.jsonl` | B | 70+4 |
| `Dockerfile`, `README.md`, `.github/workflows/ci.yml` | B | 22+110+14 |

Totals ≈ **A 720**, **B 635** (Cargo.lock excluded).

## Interfaces / Contracts

```toml
[dependencies]
nf-lineage = { path = "../nf-lineage", version = "0.1.0" }
rmcp = { version = "3.12", default-features = false, features = ["server", "macros", "transport-io"] }
schemars = "1"
serde.workspace = true; serde_json.workspace = true; tracing.workspace = true
tokio = { workspace = true }               # macros, rt-multi-thread
clap = { workspace = true }                # derive, env      (PR B)
anyhow.workspace = true; tracing-subscriber = { workspace = true }   # fmt, env-filter (PR B)
[dev-dependencies]
rmcp = { version = "3.12", default-features = false, features = ["client"] }
tempfile.workspace = true
```

```rust
// server.rs
#[derive(Debug, Clone)]
pub struct Settings { pub store_label: String, pub max_response_bytes: usize, pub check: nf_lineage::CheckOptions }
#[derive(Clone)]
pub struct LineageServer { store: Arc<dyn LineageStore>, settings: Arc<Settings>, tool_router: ToolRouter<Self> }
impl LineageServer { pub fn new(store: Arc<dyn LineageStore>, settings: Settings) -> Self }

#[derive(Deserialize, JsonSchema)] #[serde(deny_unknown_fields)]
pub struct ListRunsArgs { pub since: Option<String>, pub until: Option<String>, pub pipeline: Option<String> }
pub struct LidArgs { pub lid: String }                                   // view, check, provenance
pub struct FindArgs { #[serde(rename = "type")] pub kind: Option<String>, #[serde(default)] pub labels: Vec<String>,
                      pub name: Option<String>, #[serde(default)] pub terms: Vec<String> }
pub struct DiffArgs { pub lid_a: String, pub lid_b: String, #[serde(default)] pub mode: DiffMode }
#[derive(Default)] #[serde(rename_all = "lowercase")] pub enum DiffMode { #[default] Semantic, Raw }
#[derive(Serialize)] struct RunRow { timestamp: String, run_name: String, session_id: String, lid: Lid, pipeline: Option<String> }

#[tool_router] impl LineageServer {   // every tool:
  #[tool(description = "…", annotations(read_only_hint = true, idempotent_hint = true, destructive_hint = false, open_world_hint = false))]
  async fn list_runs(&self, Parameters(a): Parameters<ListRunsArgs>) -> Result<CallToolResult, ErrorData>;
  // view, find, diff, check, provenance alike
}
#[tool_handler(router = self.tool_router)]
impl ServerHandler for LineageServer { fn get_info(&self) -> ServerConfig }  // Implementation::new("nf-lineage-mcp", env!("CARGO_PKG_VERSION"))

// respond.rs
pub(crate) enum ToolError { Lineage(LineageError), Argument(String) }      // From<LineageError>
pub(crate) fn finish_json<T: Serialize>(r: Result<T, ToolError>, cap: usize) -> Result<CallToolResult, ErrorData>; // serialize fail → internal_error
pub(crate) fn finish_text(r: Result<String, ToolError>, cap: usize) -> CallToolResult;
pub(crate) fn fit_json(v: &Value, cap: usize) -> Result<CallToolResult, ErrorData>;
pub(crate) fn fit_text(text: String, cap: usize, hint: &str) -> CallToolResult;   // NOTICE_RESERVE = 256
pub(crate) fn tool_error(e: &ToolError) -> CallToolResult;  // class + hint table below; `_` arm for non_exhaustive

// config.rs (PR B)
#[derive(Debug, clap::Parser)] #[command(name = "nf-lineage-mcp", version, about)]
pub struct Cli {
  #[arg(long, env = "NF_LINEAGE_STORE")] pub store: String,
  #[arg(long, env = "NF_LINEAGE_MCP_MAX_RESPONSE_BYTES", default_value_t = 32768, value_parser = clap::value_parser!(u32).range(1024..))]
  pub max_response_bytes: u32,
  #[arg(long = "path-map", value_name = "FROM=TO", value_parser = parse_path_map)] pub path_map: Vec<(String, PathBuf)>,
}
impl Cli { pub fn settings(&self) -> Settings }
// main.rs: tracing → stderr; ObjectLineageStore::open(&cli.store) (missing local dir fails: LocalFileSystem canonicalizes) .with_context(..)?;
//          LineageServer::new(..).serve(rmcp::transport::stdio()).await?.waiting().await?; anyhow → exit 1
```

Error classes: `InvalidLid`→`invalid_lid` (hint: `lid://<hash>[/<path>][#<field>]`, get LIDs from list_runs/find); `InvalidQuery`→`invalid_query` (keys = model properties: type, name, labels, params.<n>, workflowRun…); `Argument`→`invalid_argument`; `NotFound`→`not_found` (list_runs/find); `UnexpectedKind`→`unexpected_kind` (`find type=FileOutput workflowRun=<run lid>`); `InvalidFormat`/`UnsupportedVersion`→`invalid_record`; `UnsupportedChecksum`/`Unsupported`→`unsupported`; `Storage`/`Io`→`storage` (store unreachable/credentials; retry); other→`error`.

Tool descriptions (first line + example): `list_runs` "List pipeline runs recorded in the lineage store, newest first. Use first to get run LIDs."; `view` "Show one lineage record (WorkflowRun, TaskRun, FileOutput…) or a field via #fragment, e.g. lid://<hash>#params."; `find` "Find records by type/labels/name or key=value terms (dotted keys allowed, e.g. params.greeting=hello). All filters must match."; `diff` "What changed between two runs or tasks: params, code, containers/tool versions, outputs; volatile fields ignored. mode=raw gives the CLI-style text diff."; `check` "Verify a FileOutput's checksum against the file on disk/object storage; returns Valid, Mismatch, FileMissing or NoChecksum."; `provenance` "Trace the tasks and inputs that produced a file (FileOutput LID) back to the pipeline inputs."

Docker (`crates/nf-lineage-mcp/Dockerfile`): `rust:1.100.0-alpine` + `apk add --no-cache musl-dev`, `COPY Cargo.toml Cargo.lock crates/`, `cargo build --release --locked -p nf-lineage-mcp`; final `gcr.io/distroless/static-debian12:nonroot`, `COPY` binary to `/nf-lineage-mcp`, `USER 65532:65532`, OCI labels, `ENTRYPOINT ["/nf-lineage-mcp"]`. CI `docker` job adds: build, user, size (≤ 31 457 280 B), smoke `docker run -i --rm -v "$PWD/fixtures/lineage/26.10.7/.lineage:/store:ro" nf-lineage-mcp:ci --store /store < crates/nf-lineage-mcp/tests/data/session.jsonl` → exactly 2 distinct `lid://<32hex>` and `"isError":false`. If the `docker` job does not exist yet (nf-llm-replay PR E unmerged), PR B creates it with only these steps.

README: install (`cargo install --path crates/nf-lineage-mcp`), flags/env, `claude mcp add --transport stdio nf-lineage -- nf-lineage-mcp --store /path/.lineage`; S3: `claude mcp add --env NF_LINEAGE_STORE=s3://bucket/lineage --env AWS_PROFILE=… --transport stdio nf-lineage -- nf-lineage-mcp` (another option between `--env` and the name); Docker: `claude mcp add --transport stdio nf-lineage -- docker run -i --rm -v /abs/.lineage:/store:ro nf-lineage-mcp:local --store /store`; `claude mcp add-json` equivalent; tools table; errors; truncation vs `MAX_MCP_OUTPUT_TOKENS`; `--path-map`; read-only/security; research-use note.

## Testing Strategy

| Layer | What | Approach |
|---|---|---|
| Unit | `fit_*` rules, error classes/hints, date parsing, `Cli` flags | `#[cfg(test)]` in `respond.rs`, `server.rs`, `config.rs` |
| Protocol | every tool, catalogue, errors, truncation, unknown tool | `tests/protocol.rs`: `common::start(store, settings)` → `().serve(duplex)`; oracle sets from `oracle/*.out`, LIDs from `ids.json`/`.args`; loops over core versions (manifest `kind == "core"`) |
| Check | Valid / FileMissing | `common::materialize(ver)`: copy manifest files to a `TempDir`, `File::set_modified(UNIX_EPOCH + mtime_ms)`, `path_map` `/nf-agents-fixture` → tempdir |
| E2E | stdout purity, env store, exit codes | `tests/stdio.rs` with `env!("CARGO_BIN_EXE_nf-lineage-mcp")`, session file to stdin, `wait_with_output` under a 30 s timeout thread |
| Image | build/user/size/smoke | CI `docker` (Docker only; local runs optional) |

Strict TDD: each GREEN step starts from the named failing test.

## Migration / Rollout

No migration. Lands after nf-lineage C; image is not published (release change).

## Deviations (2026-11-26, PR A)

Reality checked against the merged `nf-lineage` 0.1.0 API and the fixtures:

- **Pipeline name fallback.** Nextflow 25.04 and 25.10 records have no `metadata.projectName`, so `pipeline_of` falls back to the file name of `workflow.scriptFiles[0].path` (`main.nf`). Order: `workflow.repository` → `metadata.projectName` → main script name. Without it `pipeline: "MAIN"` would match nothing on two core versions.
- **Missing run record.** A `.history` entry whose WorkflowRun record is absent yields `pipeline: null` instead of failing the whole `list_runs` call; other store errors still fail it.
- **`RunRow`** flattens `nf_lineage::RunSummary` (`#[serde(flatten)]`) plus `pipeline`, instead of copying its four fields.
- **Deferred to PR B** (no user in PR A): `Settings.check`, `finish_text`, `tracing` logging (it arrives with the binary and its stderr subscriber), rmcp feature `transport-io`.
- **Workspace deps.** `rmcp` (`default-features = false`) and `schemars` are `[workspace.dependencies]` entries; the crate enables `server` + `macros`, tests add `client`. `tokio` is a dev-dependency only.
- **Truncation test cap.** The fixture `find{type: FileOutput}` result is ~530 bytes, below a 1024-byte cap, so `protocol::truncation_notice` builds the server with `max_response_bytes = 512` through `Settings`. The 1024-byte minimum is a CLI rule (PR B `config.rs`), not a library one.
- **Unknown fields.** rmcp rejects them before the tool runs, as `isError: true` with its own text (`failed to deserialize parameters: unknown field ...`), so that text carries no `invalid_argument:` prefix. The class is documented; the test asserts `isError` and the field name.
- **Oracle formats.** Nextflow 25.04 prints `find` results as a JSON array, later versions one LID per line; the test accepts both.

## Deviations (2026-11-26, PR B)

- **PR split.** The Dockerfile, the CI `docker` job, the README and the packaging check move to PR C (see `tasks.md`); PR B ships the six tools, the binary and the stdio end-to-end test.
- **`config.rs` belongs to the binary**, not the library (`mod config;` in `main.rs`), so `clap` stays out of the library API. The unit test keeps its name, `config::tests::rejects_invalid_flags`, and runs in the binary's test target.
- **`--path-map`** rejects an empty FROM or TO and a missing `=`; values are taken verbatim.
- **Startup failures** are written to stderr with `writeln!` instead of `tracing`, so the reason shows even when `NF_LINEAGE_MCP_LOG` turns logging off. `tool_error` logs every failed call with `tracing::warn!` (class and message).
- **`tokio` features** stay `macros` + `rt-multi-thread`; rmcp's `transport-io` feature brings stdio.
- **Tests.** `check_outcomes` runs on every core version; `unopenable_store_fails_fast` also asserts that a usage error exits 2 with empty stdout; `diff_raw_has_cli_header` also asserts that an unknown `mode` is an `isError` result naming the value.

## Open Questions

- [ ] `anthropic/maxResultSizeChars` tool `_meta` to raise Claude Code's 50k-char threshold — not needed while the cap is 32 KiB.
- [ ] Streamable HTTP (AgentCore Gateway) — later stage.
