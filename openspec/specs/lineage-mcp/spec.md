# lineage-mcp Specification

## Purpose

`nf-lineage-mcp`: a read-only MCP server (stdio) that lets AI agents query a Nextflow lineage store through `nf-lineage`: list runs, view records, find records, diff, check and trace provenance. Fixtures: `fixtures/lineage/<ver>/` (change `lineage-fixtures`), recorded root `/nf-agents-fixture`, LIDs from `oracle/ids.json` and `oracle/<case>.args`.

## Verification checks

Unit tests: `crates/nf-lineage-mcp/src/<module>.rs` (`<module>::tests::<name>`). Integration: `crates/nf-lineage-mcp/tests/<file>.rs` (`<file>::<name>`). "Over MCP" = rmcp client and server in-process over `tokio::io::duplex`, real JSON-RPC.

| ID | Kind | Command / name |
|----|------|----------------|
| C-test | CI `test` | `cargo test --workspace --locked` (all T below) |
| C-lint | CI `lint` | fmt, clippy `-D warnings`, `cargo deny check`, `cargo machete` |
| C-docker | CI `docker` | D-* checks of this spec |
| D-build | CI `docker` | `docker build -f crates/nf-lineage-mcp/Dockerfile -t nf-lineage-mcp:ci .` |
| D-user | CI `docker` | `Config.User` is `65532:65532` |
| D-size | CI `docker` | image ≤ 30 MB |
| D-smoke | CI `docker` | scripted JSON-RPC (`initialize`, `notifications/initialized`, `tools/call list_runs`) piped into `docker run -i --rm -v <26.10.7 store>:/store:ro nf-lineage-mcp:ci --store /store`; stdout has 2 run LIDs and `"isError":false` |
| V-pkg | verify | `cargo package -p nf-lineage -p nf-lineage-mcp --locked` |
| V-grep | verify | `src/` has 0 hits for `println!`, `print!`, `std::io::stdout`, `std::process::exit`, `set_var`; 0 non-test `unwrap/expect` |
| V-doc | verify | README greps: `claude mcp add`, `--transport stdio`, `-- nf-lineage-mcp --store`, `docker run -i`, `MAX_MCP_OUTPUT_TOKENS`, `NF_LINEAGE_STORE`, `--path-map` |

## Requirements

### Requirement: Server identity and tool catalogue

`initialize` MUST report `serverInfo.name = "nf-lineage-mcp"`, `version` = crate version, capability `tools`, and `instructions` stating: read-only, the store location, the six tools and when to use each, the LID syntax, and the truncation rule. `tools/list` MUST return exactly `list_runs`, `view`, `find`, `diff`, `check`, `provenance`, each with a description written for LLM agents (purpose, inputs, output shape, an example), an `inputSchema` (JSON Schema 2020-12, every property described, `additionalProperties: false`) and annotations `readOnlyHint: true`, `idempotentHint: true`, `destructiveHint: false`, `openWorldHint: false`.

#### Scenario: Catalogue (T `protocol::server_info_and_tool_catalogue`)
- GIVEN a server over a fixture store
- WHEN the client initializes and lists tools
- THEN name, version, instructions (contain "read-only" and the store) and the six tools with the four annotations and described properties are present

### Requirement: Configuration and startup

Store location MUST come from `--store` or env `NF_LINEAGE_STORE` (flag wins): local path, `file://`, `s3://`, `gs://`, `az://` (via `ObjectLineageStore::open`, cloud credentials from the standard environment). `--max-response-bytes` (env `NF_LINEAGE_MCP_MAX_RESPONSE_BYTES`) MUST default to 32768 and reject values < 1024. `--path-map FROM=TO` MAY repeat; empty side or missing `=` MUST be rejected. Invalid arguments MUST exit 2 (usage); a store that cannot be opened MUST exit 1 with a message on stderr and nothing on stdout. Stdin EOF MUST end the process with exit 0 after in-flight calls finish.

#### Scenario: Bad store (T `stdio::unopenable_store_fails_fast`)
- GIVEN `--store /nonexistent/x`
- WHEN the binary starts
- THEN exit 1, stderr names the path, stdout is empty

#### Scenario: Flag validation (T `config::tests::rejects_invalid_flags`)
- GIVEN `--max-response-bytes 10`, `--path-map foo`, `--path-map =x`, no store
- THEN each parse fails; `--path-map /a=/b` twice parses to two pairs

### Requirement: Stdio protocol hygiene

Stdout MUST carry only JSON-RPC messages; all logs MUST go to stderr (filter env `NF_LINEAGE_MCP_LOG`, default `info`). The process MUST NOT panic on any input.

#### Scenario: Scripted session (T `stdio::scripted_session_over_stdio`)
- GIVEN the built binary, `NF_LINEAGE_STORE` = 26.10.7 store
- WHEN `initialize`, `notifications/initialized`, `tools/list`, `tools/call list_runs` are piped and stdin closes
- THEN every stdout line parses as JSON-RPC, responses for ids 1–3 exist, the call is not an error, exit 0

### Requirement: Tools

| Tool | Params | Result (JSON text) |
|---|---|---|
| `list_runs` | `since?`, `until?` (`YYYY-MM-DD`, inclusive, on the history timestamp date), `pipeline?` (case-insensitive substring of `workflow.repository`, else `metadata.projectName`, else the main script file name) | array of `{timestamp, run_name, session_id, lid, pipeline}` newest first; `[]` when none |
| `view` | `lid` (fragment allowed) | `nf_lineage::view` output |
| `find` | `type?`, `labels?` (all required), `name?`, `terms?` (`key=value`, CLI syntax incl. dotted keys and URL-decoding); ≥ 1 filter | sorted array of LIDs |
| `diff` | `lid_a`, `lid_b`, `mode?` = `semantic` (default) \| `raw` | `SemanticDiff` JSON, or the raw unified diff text |
| `check` | `lid` (FileOutput) | `CheckReport` JSON; `Mismatch`/`FileMissing`/`NoChecksum` are results, not errors |
| `provenance` | `lid` (FileOutput) | `Provenance` JSON |

`check` MUST apply only the operator's `--path-map`; no tool parameter MAY change paths, storage options or credentials. No tool MAY write, delete or create anything.

#### Scenario: list_runs (T `protocol::list_runs_every_core_version`, `protocol::list_runs_filters`)
- GIVEN each core store
- THEN 2 runs newest first equal `oracle/list.out`; `since` after both → `[]`; `pipeline: "MAIN"` → 2; `pipeline: "sarek"` → `[]`

#### Scenario: view (T `protocol::view_record_and_fragment`)
- GIVEN `RUN1` and `RUN1#params` (26.10.7)
- THEN a `WorkflowRun` record and an array containing `greeting`

#### Scenario: find (T `protocol::find_matches_oracle`)
- GIVEN `{type: FileOutput, labels: [summary]}`, `{terms: [type=TaskRun, workflowRun=<RUN1>]}`, agent store `{type: AgentRun}`
- THEN sets equal `find-fileoutput-label`, `find-taskrun` oracles; ≥ 1 AgentRun

#### Scenario: Semantic diff (T `protocol::diff_runs_shows_only_greeting`)
- GIVEN `RUN1`, `RUN2` of every core version
- THEN exactly one change `params.greeting` `hello` → `bonjour`

#### Scenario: Raw diff (T `protocol::diff_raw_has_cli_header`)
- GIVEN `mode: raw`
- THEN text starts with `diff --git` and contains `greeting`

#### Scenario: check (T `protocol::check_outcomes`)
- GIVEN published `summary.txt` materialized with manifest mtimes and `--path-map /nf-agents-fixture=<tmp>`
- THEN `Valid`; with a map to an empty dir `FileMissing`, `isError` false

#### Scenario: provenance (T `protocol::provenance_reaches_pipeline_input`)
- GIVEN `<RUN1>/summary.txt` of every core version
- THEN nodes include SUMMARIZE, TRANSFORM, GENERATE and an `external` input ending `seed.txt`

### Requirement: Error mapping

Lineage and validation failures MUST be tool results with `isError: true` and text `<class>: <message> Hint: <next step>`; JSON-RPC errors MUST be used only for unknown tools (rmcp `-32602`) and internal faults (`-32603`). Classes: `invalid_lid`, `invalid_query`, `invalid_argument` (bad date, no `find` filter, bad `mode`, unknown/mistyped field; the last two are rejected by rmcp before the tool runs, still as `isError: true` but with rmcp's own text, see design deviations 2026-11-26), `not_found` (hint: `list_runs`/`find`), `unexpected_kind` (hint: FileOutput LID via `find`), `invalid_record`, `unsupported`, `storage`. Messages MUST name the offending input and MUST NOT contain credentials.

#### Scenario: Classes over MCP (T `protocol::errors_are_tool_results_with_hints`)
- GIVEN `view{lid:"foo"}`, `view{lid:"lid://0000…"}`, `find{terms:["foo=bar"]}`, `find{}`, `list_runs{since:"yesterday"}`, `check{lid:<TaskRun>}`, `view{lidd:…}`
- THEN `isError` true with classes `invalid_lid`, `not_found`, `invalid_query`, `invalid_argument` ×2, `unexpected_kind`; `view{lidd:…}` is `isError` true naming the unknown field

#### Scenario: Unknown tool (T `protocol::unknown_tool_is_protocol_error`)
- GIVEN `tools/call` `render`
- THEN JSON-RPC error `-32602`

#### Scenario: Unit mapping (T `respond::tests::every_error_variant_has_class_and_hint`)
- GIVEN one `LineageError` per variant
- THEN each maps to its class with a non-empty hint

### Requirement: Response size cap

A result whose text exceeds the cap MUST be cut so all content blocks total ≤ cap: arrays keep the longest prefix of whole items (valid JSON); an object over the cap keeps its other keys and cuts its largest array field the same way, with the notice naming that field; other payloads (and objects without an array field) are cut at a UTF-8 boundary. A final text block MUST start `[truncated]`, state shown/total items or bytes and how to narrow (filters, `#fragment`). Untruncated results MUST have exactly one block.

#### Scenario: Over MCP (T `protocol::truncation_notice`)
- GIVEN a server with `max_response_bytes` 512 (set through `Settings`: the fixture result is ~530 bytes; the 1024 minimum is a CLI rule), `find{type: FileOutput}` and `view{RUN1}`
- THEN valid JSON array prefix + notice "of <n> items"; byte-cut text + notice; totals ≤ 512

#### Scenario: Object over the cap (T `respond::tests::object_keeps_other_keys_and_cuts_its_largest_array`)
- GIVEN `{"target":"x","nodes":[…500 items…]}` over the cap
- THEN the text parses as JSON, keeps `target`, and `nodes` is a non-empty prefix of the original items

#### Scenario: Unit (T `respond::tests::fit_rules`)
- GIVEN small payload, large array, multi-byte text at the boundary
- THEN untouched; item-wise prefix; no split code point

### Requirement: Packaging and docs

Crate `0.1.0`, publishable metadata, README (V-doc, research-use note). Dockerfile multi-stage: `rust:1.100.0-alpine` → `gcr.io/distroless/static-debian12:nonroot`, static musl, `--locked`, `ENTRYPOINT ["/nf-lineage-mcp"]`, user `65532:65532`, OCI labels, no shell, no `EXPOSE`.

#### Scenario: Image (D-build, D-user, D-size, D-smoke)
- GIVEN a clean checkout
- THEN the image builds, runs non-root, ≤ 30 MB, answers the scripted session

#### Scenario: Package (V-pkg, V-grep, V-doc)
- THEN all pass
