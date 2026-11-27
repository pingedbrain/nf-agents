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

---

# Verification Report: nf-lineage-mcp, PR B

**Scope**: PR B (B.1–B.6) · **Date**: 2026-11-26 · **Mode**: Strict TDD. PR C (C.1–C.4: Docker, CI `docker` job, README, packaging) is out of scope.

## Completeness

| Metric | Value |
|---|---|
| PR B tasks | 6 |
| Complete | 6 |
| PR C tasks (out of scope) | 4 pending |

## Execution

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | pass |
| `cargo test --workspace --locked` | 63 passed, 0 failed, 0 ignored (nf-lineage-mcp: 4 lib unit, 1 bin unit, 12 protocol, 2 stdio, 1 doctest) |
| `cargo deny check` | advisories, bans, licenses, sources ok |
| `cargo machete` | no unused dependencies |
| V-grep (`src/`) | 0 hits: `println!`, `print!`, `std::io::stdout`, `std::process::exit`, `set_var`, non-test `unwrap/expect` |
| Manual stdio run (26.10.7 store) | stdout is JSON-RPC only, `"isError":false`, 2 run LIDs, logs on stderr, exit 0; `--store /nonexistent/x` exit 1 with the path on stderr |
| Coverage | not available (`cargo-llvm-cov` not installed) |

## Spec Compliance Matrix (PR B scope)

| Requirement | Scenario | Test | Result |
|---|---|---|---|
| Server identity and tool catalogue | Catalogue | `protocol::server_info_and_tool_catalogue` (six tools) | COMPLIANT |
| Configuration and startup | Bad store | `stdio::unopenable_store_fails_fast` | COMPLIANT (also: usage error exits 2) |
| Configuration and startup | Flag validation | `config::tests::rejects_invalid_flags` | COMPLIANT |
| Stdio protocol hygiene | Scripted session | `stdio::scripted_session_over_stdio` | COMPLIANT |
| Tools | Semantic diff | `protocol::diff_runs_shows_only_greeting` | COMPLIANT (4 core versions) |
| Tools | Raw diff | `protocol::diff_raw_has_cli_header` | COMPLIANT |
| Tools | check | `protocol::check_outcomes` | COMPLIANT (4 core versions) |
| Tools | provenance | `protocol::provenance_reaches_pipeline_input` | COMPLIANT (4 core versions) |
| Error mapping | Classes over MCP | `protocol::errors_are_tool_results_with_hints` (TaskRun `check` → `unexpected_kind`) | COMPLIANT |
| Tools / Error mapping / Size cap (PR A scenarios) | list_runs, view, find, unknown tool, truncation, unit | PR A tests, still passing | COMPLIANT |
| Packaging and docs | Image, Package | PR C | out of scope |

## TDD Compliance

| Task | Evidence | Verdict |
|---|---|---|
| B.1–B.3 | protocol tests written first; RED compile failure (`Settings` has no field `check`, tools absent) | ok |
| B.4 | RED `cannot find type Cli` | ok |
| B.5 | stdio tests written against a stub `main`; first compile failed on a test-side borrow (fixed), so no runtime RED was recorded | WARNING: RED not observed at runtime |
| B.6 | V-grep 0 hits | ok |

Test layers: unit (`respond`, `server`, `config`), protocol over real JSON-RPC (duplex), end-to-end over the built binary's stdio. No mocks; fixtures are the recorded stores.

## Coherence (Design)

| Decision | Followed | Notes |
|---|---|---|
| Mismatch/FileMissing are results | yes | `isError: false` |
| `--path-map` operator-only | yes | no tool parameter touches `CheckOptions` |
| Raw diff as plain text, capped | yes | `finish_text` |
| Logging via tracing to stderr, `NF_LINEAGE_MCP_LOG` | yes | startup failure via `writeln!(stderr)`, see deviations |
| E2E with std `Command` + session file | yes | 30 s timeout thread |
| `config.rs` placement | deviated | binary module, not library (keeps `clap` out of the API); documented |

## Issues

- CRITICAL: none.
- WARNING: B.5 RED not observed at runtime (see TDD table).
- SUGGESTION: install `cargo-llvm-cov` for per-file coverage.

## Verdict

PASS WITH WARNINGS.
