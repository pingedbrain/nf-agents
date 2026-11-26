# Verification Report: nf-llm-replay

**Mode**: Strict TDD. Scope so far: PR A (A.1-A.6), PR B (B.1-B.6). Later batches merge their sections below.

## PR A — normalization + cassette model (2026-11-26)

### Completeness

| Metric | Value |
|--------|-------|
| PR A tasks | 6 |
| Complete | 6 |
| Incomplete (PR A) | 0 |
| Change overall | 6/31 (PR B-E pending, out of scope) |

### Build and tests (executed in the worktree)

| Check | Command | Result |
|-------|---------|--------|
| C-fmt | `cargo fmt --all -- --check` | ✅ exit 0 |
| C-clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` | ✅ exit 0 |
| C-test | `cargo test --workspace --locked` | ✅ 18 passed, 0 failed, 0 skipped (nf-lineage 1, nf-llm-replay lib 15, `tests/captures.rs` 2) |
| C-deny | `cargo deny check` | ✅ advisories, bans, licenses, sources ok |
| C-machete | `cargo machete` | ✅ no unused dependencies |
| build warnings | `cargo build --workspace --locked` | ✅ 0 warnings |

Coverage: ➖ not available (no coverage tool configured).

### Mutation spot checks (tests must fail when the logic breaks)

| Mutation | Result |
|----------|--------|
| drop `prompt_cache_key` from default volatile fields | ✅ N-vol fails |
| accept status 0-99 | ✅ K-bad fails |
| disable duplicate detection | ✅ K-dup fails |
| remove the explicit key sort in `write_canonical` | ⚠️ no test fails (see W-1) |

### TDD compliance

apply-progress.md has a TDD Cycle Evidence table with 6/6 task rows. Every test file it names
exists and passes now. RED evidence: compile errors (A.2, A.6), assertion failures (A.3 N-vol/N-cwd,
A.4 N-query, A.5 N-golden). N-pi passed on first run and is reported as such (acceptance test of A.3
behavior on real data). A.1 is structural (triangulation skipped with reason). Triangulation: ≥ 2
cases per behavior in every row.

Test layers: Unit 15 tests (`src/normalize.rs` 10, `src/cassette.rs` 5); Integration 2 tests
(`tests/captures.rs`). Assertions compare concrete values (hex keys, canonical strings, line
numbers); no tautologies, no ghost loops (`capture_keys_are_stable` first asserts the fixture
list equals the 8 golden names).

### Spec compliance matrix (PR A checks, llm-replay-cassette)

| Requirement | Scenario | Test | Result |
|-------------|----------|------|--------|
| Canonical request and key | Reordered and reformatted body (N-sort) | `normalize::tests::object_keys_are_sorted_recursively` | ✅ COMPLIANT |
| Canonical request and key | Reordered and reformatted body (N-ws) | `normalize::tests::whitespace_and_key_order_do_not_change_key` | ✅ COMPLIANT |
| Canonical request and key | Query and route (N-query) | `normalize::tests::query_string_is_not_part_of_key` | ✅ COMPLIANT |
| Canonical request and key | Query and route (N-route) | `normalize::tests::method_and_path_are_part_of_key` | ✅ COMPLIANT |
| Canonical request and key | Empty and invalid bodies (N-empty) | `normalize::tests::empty_body_is_keyed_as_empty_string` | ✅ COMPLIANT |
| Canonical request and key | Empty and invalid bodies (N-bad) | `normalize::tests::non_json_body_is_rejected` | ✅ COMPLIANT |
| Volatile parts | Default fields (N-vol) | `normalize::tests::default_volatile_fields_do_not_change_key` | ✅ COMPLIANT |
| Volatile parts | Nested name is not volatile (N-nested) | `normalize::tests::nested_field_named_like_a_volatile_field_is_kept` | ✅ COMPLIANT |
| Volatile parts | Task work dir in the prompt (N-cwd) | `normalize::tests::cwd_block_is_replaced_in_every_string_value` | ✅ COMPLIANT |
| Volatile parts | Task work dir in the prompt (N-pi) | `captures::pi_requests_differing_only_in_cwd_and_cache_key_share_a_key` | ✅ COMPLIANT |
| Volatile parts | Unterminated block (N-cwd-edge) | `normalize::tests::unterminated_cwd_block_is_left_unchanged` | ✅ COMPLIANT |
| Volatile parts | Key stability across releases (N-golden) | `captures::capture_keys_are_stable` | ✅ COMPLIANT |
| Cassette format | Round trip (K-line) | `cassette::tests::entry_round_trips_as_one_json_line` | ✅ COMPLIANT |
| Cassette format | Bad cassettes (K-ver) | `cassette::tests::unknown_version_is_rejected_with_line_number` | ✅ COMPLIANT |
| Cassette format | Bad cassettes (K-bad) | `cassette::tests::malformed_line_is_rejected_with_line_number` | ✅ COMPLIANT |
| Cassette format | Bad cassettes (K-dup) | `cassette::tests::duplicate_key_is_rejected_with_both_line_numbers` | ✅ COMPLIANT |
| Cassette format | Bad cassettes (K-extra) | `cassette::tests::unknown_entry_fields_are_ignored` | ✅ COMPLIANT |

**17/17 PR A scenarios compliant.** Not in scope yet: N-ptr, N-pat (D); K-sorted, K-conc, K-guard,
S-scan (C); E-* (D); all llm-replay-proxy, llm-replay-packaging and ci-pipeline checks (B-E).
No other spec assigns a check to PR A.

### Correctness (static)

| Requirement | Status | Notes |
|-------------|--------|-------|
| Canonical request and key | ✅ | own sorted writer; query stripped at `?`; path not decoded; headers not an input; empty body → `""`; non-JSON → `NormalizeError::Body` |
| Volatile parts | ⚠️ partial by plan | defaults implemented; configured additions + invalid pointer/pattern errors are PR D |
| Cassette format | ✅ | field order v/key/request/response; `null` body; `content_type` nullable; unknown fields ignored; v, status 100-999, exactly one of body/events, duplicates → errors with 1-based line(s); `Io` names the path |
| No panics on external input | ✅ | no unwrap/expect/indexing in lib code (clippy deny lints pass); recursion bounded by serde_json's 128-level parse limit |
| Fixtures path-free | ✅ | no `/home/` or user name in `tests/fixtures/captures/`; headers dropped |

### Coherence (design)

| Decision | Followed? | Notes |
|----------|-----------|-------|
| Canonical JSON: own writer, keys sorted explicitly | ✅ | |
| Query string not keyed | ✅ | |
| Request in cassette = normalized canonical body as `RawValue` | ✅ | type in place; writing entries is PR C |
| JSON response stored as raw string; SSE as `events` | ✅ | types in place |
| thiserror in lib, anyhow in bin | ✅ | |
| `[workspace.dependencies]` full design list | ⚠️ Deviated | only used deps listed (cargo 1.100 warns on unused); documented in design.md + tasks.md |
| `Normalizer::new(extra_fields, extra_patterns)` | ⚠️ Deviated (staged) | `new()` until D.5; documented |

### Issues

**CRITICAL**: None.

**WARNING**:
- W-1: The explicit key sort in `write_canonical` is not observable by any test, because without
  serde_json's `preserve_order` feature `Map` is already a `BTreeMap` (byte order). The sort is a
  guard against feature unification from another workspace crate; only code review protects it.
  N-golden would catch a regression only if both the sort were removed and `preserve_order` enabled.

**SUGGESTION**:
- S-1: Line errors from serde_json read like `cassette line 2: EOF while parsing ... at line 1 column 40`;
  the inner "line 1" is always 1. Could strip it in a later batch if users find it confusing.
- S-2: Integration test files need a file-level `#![allow(clippy::unwrap_used, ...)]` for helper fns
  (clippy's `allow-*-in-tests` covers only `#[test]` fns); keep this pattern in PR B-D.

### Verdict

**PASS WITH WARNINGS** — PR A is complete, all gates green, 17/17 scoped scenarios compliant.

## PR B — replay server + CLI (2026-11-26)

### Completeness

| Metric | Value |
|--------|-------|
| PR B tasks | 6 |
| Complete | 6 |
| Incomplete (PR B) | 0 |
| Change overall | 12/31 (PR C-E pending, out of scope) |

### Build and tests (executed in the worktree)

| Check | Command | Result |
|-------|---------|--------|
| C-fmt | `cargo fmt --all -- --check` | ✅ exit 0 |
| C-clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` | ✅ exit 0 |
| C-test | `cargo test --workspace --locked` | ✅ 37 passed, 0 failed, 0 skipped (nf-lineage 1, nf-llm-replay lib 20, `captures` 2, `cli` 6, `replay` 8) |
| C-deny | `cargo deny check` | ✅ advisories, bans, licenses, sources ok; 0 warnings (duplicate `syn` removed by turning off `tracing`'s `attributes` feature) |
| C-machete | `cargo machete` | ✅ no unused dependencies |
| build warnings | `cargo build --workspace --locked` | ✅ 0 warnings |

Coverage: ➖ not available (no coverage tool configured). No test needs Docker, cards or keys.

### Mutation spot checks

| Mutation | Result |
|----------|--------|
| `last_message` excerpt 121 chars | ✅ U-sum fails |
| `input` string counted as 2 messages | ✅ U-sum fails |
| anthropic hint for any path containing `/messages` | ✅ U-hint fails |
| `ApiKey` `Debug` prints the key | ✅ U-dbg fails |
| no SIGTERM branch in `shutdown()` | ✅ X-listen fails (no graceful exit 0) |
| recorded `content-type` not set on replay | ✅ R-hit fails |
| no `DefaultBodyLimit` (checked during apply) | ✅ R-big fails |

### TDD compliance

apply-progress.md has TDD Cycle Evidence rows for B.1-B.6; every named test file exists and passes.
RED evidence: compile errors (B.1, B.3, B.4), 6/6 runtime failures against the stub `main` (B.6).
B.5 (R-net, R-h2c, R-conc) passed on first run and is reported as such: acceptance tests of
behaviour built in B.3; R-net includes a positive control (a manual connection moves the counter to 1).
B.2 is a helper module, exercised by every B.3-B.6 test. Triangulation ≥ 2 cases per behaviour.

Test layers: Unit 5 (`config` 3, `proxy` 2); Integration 8 (`tests/replay.rs`, in-process router on
`127.0.0.1:0`, raw HTTP/1.1); Process 6 (`tests/cli.rs`, real binary). Assertions compare concrete
values (status, header, body bytes, JSON fields, exit codes); no tautologies; R-conc checks all 64 replies.

### Spec compliance matrix (PR B checks, llm-replay-proxy)

| Requirement | Scenario | Test | Result |
|-------------|----------|------|--------|
| Generic routing | langchain4j request shape (R-h2c) | `replay::replay_ignores_h2c_upgrade_and_placeholder_bearer` | ✅ COMPLIANT |
| Generic routing | Parallel agents (R-conc) | `replay::replay_serves_64_concurrent_requests` | ✅ COMPLIANT |
| Generic routing | Large body (R-big) | `replay::eight_mib_request_body_is_accepted` | ✅ COMPLIANT |
| Generic routing | invalid JSON (R-400) | `replay::invalid_json_body_returns_400` | ✅ COMPLIANT |
| Replay mode | Hit (R-hit) | `replay::replay_hit_returns_recorded_status_content_type_and_body` | ✅ COMPLIANT |
| Replay mode | Hit (R-same) | `replay::replay_repeated_request_returns_identical_bytes` | ✅ COMPLIANT |
| Replay mode | Network never used (R-net) | `replay::replay_never_contacts_upstream` | ✅ COMPLIANT |
| Miss contract | Missing request (R-miss) | `replay::replay_miss_returns_599_with_key_summary_and_record_command` | ✅ COMPLIANT |
| Miss contract | Odd shapes (U-sum) | `proxy::tests::miss_summary_handles_chat_responses_and_messages_shapes` | ✅ COMPLIANT |
| Miss contract | Odd shapes (U-hint) | `proxy::tests::default_upstream_hint_depends_on_path` | ✅ COMPLIANT |
| Upstream credentials | Secret never printed (U-dbg) | `config::tests::debug_output_redacts_api_key` | ✅ COMPLIANT |
| Configuration | CLI errors (X-help) | `cli::help_lists_flags_and_env_vars` | ✅ COMPLIANT (volatile flags arrive in D.5) |
| Configuration | CLI errors (X-mode) | `cli::invalid_mode_exits_2` | ✅ COMPLIANT |
| Configuration | CLI errors (X-env) | `cli::mode_from_env_var_is_honoured` | ✅ COMPLIANT (invalid env mode → 2; env mode/cassette/port start the server) |
| Configuration | CLI errors (X-nocas) | `cli::replay_with_missing_cassette_exits_1` | ✅ COMPLIANT |
| Configuration | CLI errors (X-corrupt) | `cli::corrupt_cassette_exits_1_with_line_number` | ✅ COMPLIANT |
| Configuration | Ephemeral port (X-listen) | `cli::port_0_prints_listening_address_and_serves` | ✅ COMPLIANT |

**17/17 PR B scenarios compliant.** Not in scope yet: U-hdr, U-auth, W-*, P-pass, X-nokey, X-noup,
X-nolog (C); E-*, X-vol (D); packaging and CI (E).

### Correctness (static)

| Requirement | Status | Notes |
|-------------|--------|-------|
| Generic routing | ✅ | axum `fallback`, any method/path; 32 MiB `DefaultBodyLimit`; HTTP/1.1 only (h2c upgrade ignored) |
| Replay mode | ⚠️ partial by plan | immutable `Cassette`, no HTTP client exists in the binary; `events` entries are concatenated into one body until D.3 streams one chunk per event |
| Miss contract | ✅ | 599 + JSON fields as specified; message one sentence; `warn` log with the same message |
| Configuration and process | ⚠️ partial by plan | flags/env/defaults, exit 2/1, `listening on`, stderr logs (one `info` line per request: mode, method, path, key prefix, outcome, status, ms; no headers/bodies), SIGINT/SIGTERM; volatile flags D.5; key/upstream requirements C |
| No panics on external input | ✅ | clippy deny lints pass on lib/bin; `json!` only on `String`/`Option` fields (infallible) |
| Read-only / no secrets | ✅ | replay never writes; key only from env, `Debug` = `***`; no key flag |

### Coherence (design)

| Decision | Followed? | Notes |
|----------|-----------|-------|
| Catch-all routing | ✅ | |
| Replay never builds an HTTP client | ✅ | reqwest not in the tree yet |
| thiserror in lib, anyhow in bin | ✅ | `StartupError`/`ConfigError` thiserror; `main` uses anyhow |
| `Config.upstream: reqwest::Url` | ⚠️ Deviated | validated `String` until reqwest lands (PR C); documented |
| `lib::serve` | ⚠️ Deviated | not added; `main` builds `app()` before binding so startup errors precede `listening on`; documented |
| Dev-deps `assert_cmd` | ⚠️ Deviated | std `Command` + `CARGO_BIN_EXE_*` used instead; documented |

### Issues

**CRITICAL**: None.

**WARNING**:
- W-2: PR size ≈ 1,190 hand-written lines vs the ≈ 800 budget (estimate was ≈ 700; ≈ 590 are tests and
  test helpers). All six tasks are coherent; splitting now would mean reverting parts of `proxy.rs`.
- W-3: Replay of SSE (`events`) entries returns one concatenated body until D.3 (byte-identical, but not
  one chunk per event yet).

**SUGGESTION**:
- S-3: `shutdown()` uses `tokio::signal::unix`; the crate does not compile on Windows. Fine for the
  Linux/Docker target; add `#[cfg(unix)]` if a non-Unix build is ever needed.
- S-4: Requests over 32 MiB get axum's plain-text 413, not the `nf_llm_replay_*` JSON error shape.

### Verdict

**PASS WITH WARNINGS** — PR B is complete, all gates green, 17/17 scoped scenarios compliant.
