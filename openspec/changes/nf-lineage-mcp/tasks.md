# Tasks: nf-lineage-mcp 0.1

Strict TDD (RED → GREEN). Test names from `specs/lineage-mcp/spec.md`. Paths relative to `crates/nf-lineage-mcp/` unless rooted. Each PR ends with `cargo fmt --all`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check`, `cargo machete` green. Preconditions: `nf-lineage` PRs A–C and `lineage-fixtures` merged; `nf-llm-replay` PR B (workspace deps) and PR E (`docker` job, `.dockerignore`) merged or handled as noted.

## PR A — server with list_runs, view, find · est. ≈ 720 lines

- [x] A.1 Root `Cargo.toml`: `rust-version = "1.88"`; add `tokio`/`tracing` to `[workspace.dependencies]` if absent (nf-llm-replay versions). Crate `Cargo.toml` 0.1.0 (lib only), deps per design; `src/lib.rs` docs.
- [x] A.2 RED `src/respond.rs` tests `fit_rules`, `every_error_variant_has_class_and_hint`.
- [x] A.3 GREEN `ToolError`, `fit_json`, `fit_text`, `tool_error`, `finish_json`, `finish_text` (total ≤ cap, char boundary, `_` arm).
- [x] A.4 RED `tests/common/mod.rs` (fixtures root, core versions, `ids`, oracle readers, `start` over duplex, `call` helpers) + `tests/protocol.rs::server_info_and_tool_catalogue`.
- [x] A.5 GREEN `src/server.rs`: `Settings`, `LineageServer::new`, instructions, hand `get_info`, router with the 3 tools stubbed; annotations + descriptions.
- [x] A.6 RED→GREEN `list_runs_every_core_version`, `list_runs_filters`; date validation unit test; `pipeline_of`.
- [x] A.7 RED→GREEN `view_record_and_fragment`.
- [x] A.8 RED→GREEN `find_matches_oracle` (typed fields via `FindQuery::new`, `terms` via `parse_args`, ≥ 1 filter).
- [x] A.9 RED→GREEN `errors_are_tool_results_with_hints` (A-tool cases; `check` case added in B), `unknown_tool_is_protocol_error`, `truncation_notice`.
- [x] A.10 V-grep on `src/`; diff ≤ 800.
- Escape hatch: if > 800, move `truncation_notice` + `fit_text` cut path to PR B.
- Deviation (2026-11-26): PR A landed at ~930 physical lines (tests ~45 %); the escape hatch was not used because `fit_text` bounds every non-array result (`view`). `Settings.check`, `finish_text` and tracing moved to PR B; see design deviations.

## PR B — diff, check, provenance, binary, e2e, Docker, README · est. ≈ 635 lines

- [ ] B.1 RED→GREEN `diff_runs_shows_only_greeting` (all core versions), `diff_raw_has_cli_header`; `DiffArgs`, `DiffMode`.
- [ ] B.2 RED `common::materialize` + `check_outcomes`; extend `errors_are_tool_results_with_hints` with TaskRun `check`; GREEN `Settings.check` + `check` tool; `finish_text` for raw diff (B.1).
- [ ] B.3 RED→GREEN `provenance_reaches_pipeline_input`.
- [ ] B.4 Crate deps `clap`, `anyhow`, `tracing`, `tracing-subscriber`, `tokio`, rmcp `transport-io` (workspace entries if absent). RED `src/config.rs::tests::rejects_invalid_flags`; GREEN `Cli`, `parse_path_map`, `Cli::settings`.
- [ ] B.5 RED `tests/stdio.rs::scripted_session_over_stdio`, `unopenable_store_fails_fast` + `tests/data/session.jsonl`; GREEN `src/main.rs` (stderr tracing, open, serve stdio).
- [ ] B.6 `Dockerfile` (design); local D-build, D-user, D-size, D-smoke when Docker is available.
- [ ] B.7 `.github/workflows/ci.yml` `docker` job: build, user, size, smoke steps (create the job if absent: no secrets, `timeout-minutes: 20`); V-yaml.
- [ ] B.8 `README.md` (V-doc); crate metadata (readme, keywords, categories); V-pkg `cargo package -p nf-lineage -p nf-lineage-mcp --locked`.
- [ ] B.9 Every spec scenario maps to a passing test; diff ≤ 800.
- Escape hatch: if > 800, move B.6–B.8 (Docker, CI, README) to PR C.
