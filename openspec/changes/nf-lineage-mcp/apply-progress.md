# Apply progress: nf-lineage-mcp

Mode: Strict TDD (`cargo test --workspace`).

## Batches

| Batch | Date | Tasks | Status |
|---|---|---|---|
| PR A | 2026-11-26 | A.1–A.10 | done |
| PR B | — | B.1–B.9 | pending |

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

## Deviations

See `design.md` "Deviations (2026-11-26, PR A)". Size: ~930 physical lines (incl. tests, docs, blank lines) against the ~800 target; deferred items (`Settings.check`, `finish_text`, tracing, `transport-io`) moved into PR B tasks.

## Remaining

PR B: B.1–B.9 (diff, check, provenance, CLI/binary, stdio e2e, Dockerfile, CI, README).
