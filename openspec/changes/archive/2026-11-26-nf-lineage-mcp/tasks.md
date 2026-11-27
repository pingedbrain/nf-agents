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

## PR B — diff, check, provenance, binary, stdio e2e · est. ≈ 600 lines

- [x] B.1 RED→GREEN `diff_runs_shows_only_greeting` (all core versions), `diff_raw_has_cli_header`; `DiffArgs`, `DiffMode`; `finish_text` for the raw diff.
- [x] B.2 RED `common::materialize` + `check_outcomes`; extend `errors_are_tool_results_with_hints` with TaskRun `check`; GREEN `Settings.check` + `check` tool (operator-only `path_map`).
- [x] B.3 RED→GREEN `provenance_reaches_pipeline_input`; catalogue test expects the six tools.
- [x] B.4 Crate deps `clap`, `anyhow`, `tracing`, `tracing-subscriber`, `tokio`, rmcp `transport-io` (workspace entries, nf-llm-replay versions). RED `src/config.rs::tests::rejects_invalid_flags`; GREEN `Cli`, `parse_path_map`, `Cli::settings`.
- [x] B.5 RED `tests/stdio.rs::scripted_session_over_stdio`, `unopenable_store_fails_fast` + `tests/data/session.jsonl`; GREEN `src/main.rs` (stderr tracing, open, serve stdio).
- [x] B.6 V-grep on `src/`; every PR B spec scenario maps to a passing test; diff ≤ 800.

- Result (2026-11-26): PR B landed at ~520 hand-written lines (src ~290, tests ~230).

## PR C — Docker image, CI `docker` job, README, packaging · est. ≈ 200 lines

- [x] C.1 `crates/nf-lineage-mcp/Dockerfile` (design: static musl on `gcr.io/distroless/static-debian12:nonroot`); local D-build, D-user, D-size, D-smoke when Docker is available.
- [x] C.2 `.github/workflows/ci.yml` `docker` job: build, user, size, smoke steps piping `tests/data/session.jsonl` into `docker run -i` against the 26.10.7 fixture store (create the job if absent: no secrets, `timeout-minutes: 20`); V-yaml.
- [x] C.3 `README.md` with `claude mcp add` instructions, native and Docker variants (V-doc); crate metadata (readme, keywords, categories).
- [x] C.4 V-pkg `cargo package -p nf-lineage -p nf-lineage-mcp --locked`; every spec scenario maps to a passing test.

Deviation (2026-11-26, PR B/C split): PR B as planned (B.1–B.9) plus the items moved from PR A exceeds the ~800-line budget. The Docker image, the CI `docker` job, the README and the packaging check move to a new PR C; they depend only on the binary from PR B. The `docker` job pattern lives in the `nf-llm-replay` stack, which this branch stack does not contain yet, so PR C either extends that job or creates it.

- Result (2026-11-26): PR C landed at ~130 hand-written lines (Dockerfile 18, CI 17, README ~100, crate metadata 3). Image: 21 151 114 B, user `65532:65532`; D-smoke returns 2 distinct run LIDs and `"isError":false`. `cargo package` passes (crate `include` limited to `src/` and `README.md`, like `nf-lineage`, because the tests read the repository fixtures).
