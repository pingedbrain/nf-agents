# Apply progress: nf-lineage-mcp

Mode: Strict TDD (`cargo test --workspace`).

## Batches

| Batch | Date | Tasks | Status |
|---|---|---|---|
| PR A | 2026-11-26 | A.1–A.10 | done |
| PR B | 2026-11-26 | B.1–B.6 | done |
| PR C | — | C.1–C.4 | pending |

## PR A — files

| File | Action | What |
|---|---|---|
| `Cargo.toml` | Modified | `rust-version` 1.88 (rmcp MSRV); workspace deps `rmcp` (no default features), `schemars` |
| `crates/nf-lineage-mcp/Cargo.toml` | Created | lib-only crate 0.1.0 |
| `crates/nf-lineage-mcp/src/lib.rs` | Created | crate docs, re-exports |
| `crates/nf-lineage-mcp/src/respond.rs` | Created | `ToolError`, `finish_json`, `fit_json`, `fit_text`, `tool_error` + unit tests |
| `crates/nf-lineage-mcp/src/server.rs` | Created | `Settings`, `LineageServer`, args, `list_runs`/`view`/`find`, `get_info` + unit tests |
| `crates/nf-lineage-mcp/tests/common/mod.rs` | Created | fixtures, oracle readers, duplex `start`, `call` helpers |
| `crates/nf-lineage-mcp/tests/protocol.rs` | Created | 8 protocol tests over MCP |

## TDD Cycle Evidence

| Task | RED | GREEN | Triangulate / Refactor |
|---|---|---|---|
| A.1 | structural (manifests, docs) | builds | Triangulation skipped: config only |
| A.2/A.3 | `respond::tests::{fit_rules, every_error_variant_has_class_and_hint}` failed to compile (no `fit_json`/`fit_text`/`tool_error`/`ToolError`) | 2 passed | fit_rules covers small/array/oversized-first-item/odd caps 1024+1025; refactor: error test compacted |
| A.4/A.5 | `protocol::server_info_and_tool_catalogue`: unresolved `LineageServer`, `Settings` | passed with stubbed tools | — |
| A.6 | `list_runs_*` + `server::tests::dates_must_be_iso_days`: `parse_day`/`ToolError` missing; stubs returned `null` | 3 passed on 4 core versions | `pipeline_prefers_repository_then_project_then_script` (repository/project/script/none); refactor: `RunRow` flattens `RunSummary` |
| A.7 | `view_record_and_fragment` failed (`Null` instead of `WorkflowRun`) | passed | record + `#params` fragment |
| A.8 | `find_matches_oracle` failed (stub `null`) | passed after the test accepted 25.04's JSON-array oracle format | typed fields, `terms`, AgentRun store, 4 core versions |
| A.9 | `errors_are_tool_results_with_hints`, `unknown_tool_is_protocol_error`, `truncation_notice` written after the behaviour existed (A.3/A.6–A.8): passed on first run | 3 passed | 6 error classes + unknown field; array and byte truncation |
| A.10 | V-grep: 0 hits (`println!`, `print!`, `stdout`, `process::exit`, `set_var`, non-test `unwrap/expect`) | — | — |

## PR B — files

| File | Action | What |
|---|---|---|
| `Cargo.toml` | Modified | workspace deps `anyhow`, `clap`, `tracing`, `tracing-subscriber` (nf-llm-replay versions) |
| `crates/nf-lineage-mcp/Cargo.toml` | Modified | binary deps, rmcp `transport-io`, dev-dep `tempfile` |
| `crates/nf-lineage-mcp/src/lib.rs` | Modified | docs for six tools, `Settings.check`, re-exports `DiffArgs`, `DiffMode` |
| `crates/nf-lineage-mcp/src/respond.rs` | Modified | `finish_text`; `tracing::warn!` per failed call |
| `crates/nf-lineage-mcp/src/server.rs` | Modified | `Settings.check`, `DiffArgs`/`DiffMode`, tools `diff`, `check`, `provenance`, instructions |
| `crates/nf-lineage-mcp/src/config.rs` | Created | `Cli`, `parse_path_map`, `Cli::settings` + unit test |
| `crates/nf-lineage-mcp/src/main.rs` | Created | stderr tracing, open store, serve stdio, exit codes |
| `crates/nf-lineage-mcp/tests/common/mod.rs` | Modified | `materialize`, `start_with` |
| `crates/nf-lineage-mcp/tests/protocol.rs` | Modified | six-tool catalogue, `check` error case, diff/check/provenance tests |
| `crates/nf-lineage-mcp/tests/stdio.rs` | Created | built binary over stdio: scripted session, startup failures |
| `crates/nf-lineage-mcp/tests/data/session.jsonl` | Created | scripted JSON-RPC session (also for the PR C Docker smoke) |

## PR B — TDD Cycle Evidence

| Task | RED | GREEN | Triangulate / Refactor |
|---|---|---|---|
| B.1 | `diff_runs_shows_only_greeting`, `diff_raw_has_cli_header` written first; protocol tests failed to compile (`Settings` has no field `check`, tools missing) | 2 passed on 4 core versions | semantic + raw + invalid `mode`; refactor: `lid_pair`, `tool_error` for bad LIDs |
| B.2 | `check_outcomes` + `materialize`, `check` case in `errors_are_tool_results_with_hints`: same compile failure | passed | `valid` and `file_missing` on 4 core versions; `unexpected_kind` on a TaskRun |
| B.3 | `provenance_reaches_pipeline_input`, catalogue expecting six tools: same compile failure | passed | 4 core versions |
| B.4 | `config::tests::rejects_invalid_flags`: `cannot find type Cli` | passed | 1023/10 bytes, `foo`, `=x`, `/a=`, no store, two maps |
| B.5 | `stdio::{scripted_session_over_stdio, unopenable_store_fails_fast}` written against a stub `fn main() {}` (first compile failed on a test-side borrow, fixed before GREEN) | 2 passed | usage error exit 2; manual run: `"isError":false`, logs only on stderr |
| B.6 | V-grep: 0 hits in `src/` (`println!`, `print!`, `std::io::stdout`, `std::process::exit`, `set_var`, non-test `unwrap/expect`) | — | — |

## Deviations

See `design.md` "Deviations (2026-11-26, PR A)". Size: ~930 physical lines (incl. tests, docs, blank lines) against the ~800 target; deferred items (`Settings.check`, `finish_text`, tracing, `transport-io`) moved into PR B tasks.

PR B: see `design.md` "Deviations (2026-11-26, PR B)" (PR B/C split, `config.rs` in the binary, startup errors via `writeln!`). Size: ~520 hand-written lines.

## PR C — files

| File | Action | What |
|---|---|---|
| `crates/nf-lineage-mcp/Dockerfile` | Created | static musl build on `rust:1.100.0-alpine` → `gcr.io/distroless/static-debian12:nonroot`, user `65532:65532`, OCI labels, no `EXPOSE` |
| `.github/workflows/ci.yml` | Modified | `docker` job: build `nf-lineage-mcp:ci`, user/size (≤ 31 457 280 B) check, stdio smoke against `fixtures/lineage/26.10.7/.lineage` mounted `:ro` |
| `crates/nf-lineage-mcp/README.md` | Created | install, `claude mcp add` (native, S3, Docker `-i`), `add-json`, flags/env, tools, error classes, truncation vs `MAX_MCP_OUTPUT_TOKENS`, security, research-use note |
| `crates/nf-lineage-mcp/Cargo.toml` | Modified | `readme`, `include = ["/src", "/README.md"]` |

`.dockerignore` is unchanged: it already sends only `Cargo.toml`, `Cargo.lock` and `crates/` (minus tests and examples), which both images need.

## PR C — TDD Cycle Evidence

PR C adds no Rust code; its checks are the spec's D-* and V-* commands, run after writing the artifacts.

| Task | RED | GREEN | Notes |
|---|---|---|---|
| C.1 | D-build could not run (no Dockerfile) | D-build ok; D-user `65532:65532`; D-size 21 151 114 B ≤ 30 MB; D-smoke 2 LIDs, `"isError":false` | `object_store` cloud features + rustls fit well under the gate |
| C.2 | — | actionlint 0 findings; the smoke step script run locally under `bash -euo pipefail` | output kept in a shell variable, no stray file in the checkout |
| C.3 | V-doc greps had no README to match | all 7 V-doc greps match | `claude mcp add` syntax checked against the CLI help: options before the name, `--` before the command; `--env` is variadic, so another option separates it from the name |
| C.4 | — | `cargo package -p nf-lineage -p nf-lineage-mcp --locked` builds both crates (`--allow-dirty` locally, uncommitted tree) | every spec scenario maps to a passing test |

## Remaining

None. All PR A–C tasks are complete.
