# Verification Report: nf-lineage-mcp

**Scope**: PR A (A.1–A.10) · **Date**: 2026-11-26 · **Mode**: Strict TDD

## Completeness

| Metric | Value |
|---|---|
| PR A tasks | 10 |
| Complete | 10 |
| PR B tasks (out of scope) | 9 pending |

## Execution

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | pass |
| `cargo test --workspace --locked` | 56 passed, 0 failed, 0 ignored (nf-lineage-mcp: 4 unit, 8 protocol, 1 doctest) |
| `cargo deny check` | advisories, bans, licenses, sources ok |
| `cargo machete` | no unused dependencies |
| V-grep (`src/`) | 0 hits: `println!`, `print!`, `std::io::stdout`, `std::process::exit`, `set_var`, non-test `unwrap/expect` |
| Coverage | not available (`cargo-llvm-cov` not installed) |

## Spec Compliance Matrix (PR A scope)

| Requirement | Scenario | Test | Result |
|---|---|---|---|
| Server identity and tool catalogue | Catalogue | `protocol::server_info_and_tool_catalogue` | PARTIAL by design: 3 of 6 tools until PR B |
| Tools | list_runs | `protocol::list_runs_every_core_version`, `protocol::list_runs_filters` | COMPLIANT |
| Tools | view | `protocol::view_record_and_fragment` | COMPLIANT |
| Tools | find | `protocol::find_matches_oracle` | COMPLIANT |
| Error mapping | Classes over MCP | `protocol::errors_are_tool_results_with_hints` | COMPLIANT for A tools (`check` case lands in PR B) |
| Error mapping | Unknown tool | `protocol::unknown_tool_is_protocol_error` | COMPLIANT |
| Error mapping | Unit mapping | `respond::tests::every_error_variant_has_class_and_hint` | COMPLIANT |
| Response size cap | Over MCP | `protocol::truncation_notice` | COMPLIANT (512-byte cap, see design deviations) |
| Response size cap | Unit | `respond::tests::fit_rules` | COMPLIANT |
| Configuration, stdio hygiene, diff, check, provenance, packaging | — | PR B | out of scope |

Extra unit tests: `server::tests::dates_must_be_iso_days`, `server::tests::pipeline_prefers_repository_then_project_then_script`.

## TDD Compliance

| Task | Evidence | Verdict |
|---|---|---|
| A.2/A.3 | RED compile failure recorded, tests pass | ok |
| A.4/A.5 | RED unresolved imports, test passes | ok |
| A.6–A.8 | RED runtime failures against stubs, tests pass, triangulated over 4 core versions | ok |
| A.9 | tests written after the behaviour existed and passed on first run | WARNING: characterization, not test-first |

Test layers: unit (`respond`, `server`) + protocol over real JSON-RPC (duplex). No mocks; fixtures are the real recorded stores.

## Coherence (Design)

| Decision | Followed | Notes |
|---|---|---|
| Error mapping as `isError` tool results | yes | JSON-RPC only for unknown tool |
| Truncation item-wise / char boundary, notice block | yes | `NOTICE_RESERVE` 256 |
| Compact JSON text, one block | yes | |
| list_runs newest first, sequential reads | yes | `ponytail:` comment |
| find typed fields + `terms` | yes | |
| Hand-written `get_info` | yes | |
| `deny_unknown_fields` | yes | rmcp text, see deviations |
| Logging via tracing | deferred to PR B | no binary in PR A |

## Issues

- CRITICAL: none.
- WARNING: PR A is 924 physical lines (782 non-blank, non-comment) against the ~800 target; A.9 tests were characterization tests.
- SUGGESTION: install `cargo-llvm-cov` for per-file coverage.

## Verdict

PASS WITH WARNINGS.
