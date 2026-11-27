# Verification Report: nf-llm-replay

**Mode**: Strict TDD. Scope so far: PR A (A.1-A.6), PR B (B.1-B.6), PR C1 (C.1, C.2, C.4). Later batches merge their sections below.

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

## PR C1 — upstream client + Recorder (2026-11-26)

PR C was split into C1/C2 (tasks.md deviation); this section covers C.1, C.2, C.4.

### Completeness

| Metric | Value |
|--------|-------|
| PR C1 tasks | 3 |
| Complete | 3 |
| Incomplete (PR C1) | 0 |
| Change overall | 15/31 (PR C2, D, E pending, out of scope) |

### Build and tests (executed in the worktree)

| Check | Command | Result |
|-------|---------|--------|
| C-fmt | `cargo fmt --all -- --check` | ✅ exit 0 |
| C-clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` | ✅ exit 0 |
| C-test | `cargo test --workspace --locked` | ✅ 44 passed, 0 failed, 0 skipped (nf-lineage 1, lib 27, captures 2, cli 6, replay 8) |
| C-deny | `cargo deny check` | ✅ ok (warning: duplicate `syn` 2/3, see W-4) |
| C-machete | `cargo machete` | ✅ no unused dependencies |
| build warnings | `cargo build --workspace --locked` | ✅ 0 warnings |

Coverage: ➖ not available.

### Mutation spot checks

| Mutation | Result |
|----------|--------|
| disable the secret guard in `persist` | ✅ K-guard fails |
| no rollback of `saved` after a failed write | ✅ K-sorted fails (assertion added during verify; initially survived) |
| forward `proxy-*` headers | ✅ U-hdr fails |
| prefer `api-key` over `x-api-key` | ✅ U-auth + send test fail |
| default redirect policy | ✅ send test fails |
| keep the URL in reqwest errors | ✅ `send_error_omits_the_url` fails |

### TDD compliance

apply-progress.md has TDD evidence rows for C.1-C.4 (C1 subset); every RED was a compile failure
against missing types (E0425/E0433/E0599), confirmed by execution before GREEN. C.1 is structural.

### Spec compliance matrix (PR C1 checks)

| Requirement | Scenario | Test | Result |
|-------------|----------|------|--------|
| Upstream credentials and headers | Injection (U-hdr) | `upstream::tests::hop_by_hop_credential_and_encoding_headers_are_not_forwarded` | ✅ COMPLIANT |
| Upstream credentials and headers | Injection (U-auth) | `upstream::tests::api_key_is_injected_in_the_client_header_style` | ✅ COMPLIANT |
| Record mode (forward base + path + query, original bytes) | unit level of W-fwd | `upstream::tests::send_targets_base_path_and_query_without_following_redirects` | ⚠️ PARTIAL (W-fwd integration test is C2) |
| Stable ordering and safe concurrent writes | Sorted atomic insert (K-sorted) | `cassette::tests::insert_rewrites_file_sorted_by_key_without_temp_leftovers` | ✅ COMPLIANT |
| No secrets in cassettes | Echoed key (K-guard) | `cassette::tests::entry_containing_the_api_key_is_refused` | ⚠️ PARTIAL: refusal + file untouched proven; "client gets HTTP 500" needs the C2 wiring |

**Compliance summary**: 3/5 compliant, 2 partial by design (the remaining halves are C2 scenarios).

### Correctness (static)

| Requirement | Status | Notes |
|------------|--------|-------|
| Key only from env, injected in client style, credentials/hop-by-hop/host/length/encoding dropped | ✅ | `DROPPED` + `proxy-*`; key values `set_sensitive` |
| No redirects, no compression, 600 s / 10 s timeouts | ✅ | reqwest without gzip features; constants `TIMEOUT`/`CONNECT_TIMEOUT` (not unit-tested) |
| Cassette sorted, temp sibling + rename, one in-process lock | ✅ | `tokio::sync::Mutex<BTreeMap>`, `<cassette>.tmp`, `sync_all` |
| Entry containing the key refused, nothing written | ✅ | checked on the serialized line before taking the lock |
| Record loads the cassette, created if absent | ✅ | `Recorder::open`; known keys pre-fill their `OnceCell` |
| No panics on external input | ✅ | poisoned mutex → `into_inner`; no unwrap/expect/indexing outside tests |

### Coherence (design)

| Decision | Followed? | Notes |
|----------|-----------|-------|
| Inject env key, strip client credentials | ✅ | |
| `redirect::Policy::none()` / identity encoding | ✅ | |
| BTreeMap + temp + `sync_all` + rename under tokio Mutex | ✅ | |
| Per-key `OnceCell` single-flight | ✅ | `Recorder::cell`; coalescing itself is exercised in C2 (W-once) |
| Secret guard test + runtime | ✅ runtime + unit | S-scan integration is C2 |
| reqwest default rustls; `CDLA-Permissive-2.0` | ✅ | features `rustls, http2` only (`stream` → D.4) |
| `Upstream` / `forward_headers` signatures | ⚠️ Deviated | `new` returns `UpstreamError`, `forward_headers` is a method, `send` returns `reqwest::Error`; documented in design.md |
| `Config.upstream: reqwest::Url` | ⚠️ Deviated | still a validated `String`; documented |

### Issues

**CRITICAL**: None.

**WARNING**:
- W-4: `cargo deny` duplicate-version warning `syn` 2.0/3.0 is back; `syn` 2 enters only through
  Android-only `jni` (`rustls-platform-verifier`). Non-blocking (`multiple-versions = "warn"`); fix with
  `[graph] targets` in deny.toml if wanted.
- W-5: An empty `NF_LLM_REPLAY_API_KEY` would make the guard refuse every entry (`contains("")`); C2's
  X-nokey config check must treat an empty key as missing.

**SUGGESTION**:
- S-5: `upstream` is a `pub` module only so it is not dead code before C2 wires it; C2 can make it private.
- S-6: If the final `rename` fails, `<cassette>.tmp` is left behind (it is overwritten on the next write).

### Verdict

**PASS WITH WARNINGS** — PR C1 is complete, all gates green, 3/5 scoped scenarios fully compliant and
2 partial by design (their integration halves are C2 tasks).

## PR C2 — record + passthrough wiring (2026-11-26)

**Mode**: Strict TDD. Scope: C.3, C.5, C.6, C.7 (+ C1 follow-ups W-4, W-5, S-5).

### Completeness

| Metric | Value |
|--------|-------|
| Tasks in scope | 4 (C.3, C.5, C.6, C.7) |
| Complete | 4 |
| Overall | 19/31 (PR D, E pending) |

### Build and tests (executed in the worktree)

| Command | Result |
|---------|--------|
| `cargo fmt --all -- --check` | ✅ exit 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | ✅ exit 0 |
| `cargo test --workspace --locked` | ✅ 60 passed, 0 failed, 0 ignored (28 lib, 2 captures, 9 cli, 11 record, 8 replay, 1 secrets, 1 nf-lineage) |
| `cargo deny check` | ✅ advisories/bans/licenses/sources ok, **no warnings** (W-4 fixed by `[graph] targets`) |
| `cargo machete` | ✅ no unused dependencies |
| record + cli + secrets re-run ×3 | ✅ stable (no flakes; W-once uses a 200 ms fake delay) |

Coverage: ➖ no coverage tool configured.

### Mutation spot checks

| Mutation | Caught by |
|----------|-----------|
| Bypass `OnceCell` coalescing (check `cell.get()`, else fetch directly) | W-once FAILED (fake hits > 1) |
| Store `path_and_query` instead of `path` in the entry | S-scan FAILED (`sk-client`, `key=` in cassette) |
| `tracing::debug!` the exposed key in `main` | X-nolog FAILED |

### TDD compliance

| Task | RED | GREEN | Triangulate | Notes |
|------|-----|-------|-------------|-------|
| config (MissingUpstream, empty key) | ✅ new unit test FAILED first | ✅ | ✅ 2 modes, empty/non-empty key | existing passthrough test updated: spec requires `--upstream` |
| C.3 | ➖ helper | ✅ exercised by 13 tests | ➖ | |
| C.5 | ✅ 10 tests FAILED (`Unsupported`) | ✅ 11/11 | ✅ | + echoed-key 500 test |
| C.6 | ✅ FAILED with C.5 | ✅ | ✅ JSON + non-JSON | |
| C.7 | ⚠️ passed on first run | ✅ | ✅ | acceptance tests of C.5 behaviour; mutation-checked above |

4/4 tasks have TDD evidence in apply-progress.md.

### Test layer distribution (C2)

Unit 1 (`config.rs`); integration 12 (`tests/record.rs` 11, `tests/secrets.rs` 1, in-process server + in-process fake upstream); CLI/process 3 (`tests/cli.rs`). No E2E in this batch.

### Spec compliance matrix (PR C2 checks)

| Check | Scenario | Test | Result |
|-------|----------|------|--------|
| W-rt | Round trip | `record::record_then_replay_round_trip_chat_completions` | ✅ COMPLIANT (plain + `json_schema`; fake hit count unchanged during replay) |
| W-tool | Round trip | `record::multi_turn_tool_round_trip_replays_offline` | ✅ COMPLIANT |
| W-once | Coalescing and fill | `record::concurrent_identical_requests_call_upstream_once` | ✅ COMPLIANT (16 concurrent + 1 repeat, 1 upstream hit, 17 equal) |
| W-fill | Coalescing and fill | `record::existing_key_is_served_without_upstream_call` | ✅ COMPLIANT |
| W-err | Upstream failure | `record::non_2xx_upstream_response_is_returned_but_not_recorded` | ✅ COMPLIANT |
| W-502 | Upstream failure | `record::unreachable_upstream_returns_502_json` | ✅ COMPLIANT |
| W-auth | Injection | `record::env_key_is_injected_and_client_credentials_are_stripped` | ✅ COMPLIANT |
| W-fwd | Injection | `record::query_string_and_provider_headers_are_forwarded` | ✅ COMPLIANT |
| P-pass | No cassette I/O | `record::passthrough_forwards_without_touching_the_cassette` | ⚠️ PARTIAL by design (incremental SSE half is E-live, D.4) |
| K-conc | Parallel recording | `record::record_concurrent_distinct_requests_produce_valid_sorted_cassette` | ✅ COMPLIANT |
| K-guard (integration) | Echoed key | `record::echoed_key_returns_500_and_is_not_recorded` | ✅ COMPLIANT (unit half in C1) |
| S-scan | Secret scan | `secrets::recorded_cassette_contains_no_credentials` | ✅ COMPLIANT |
| X-nokey | CLI errors | `cli::record_without_api_key_exits_1` | ✅ COMPLIANT (unset and empty; no file created) |
| X-noup | CLI errors | `cli::record_without_upstream_exits_2` | ✅ COMPLIANT (record and passthrough) |
| X-nolog | Secret never printed | `cli::api_key_never_appears_in_output` | ✅ COMPLIANT (`RUST_LOG=trace`, stdout + stderr) |

**Compliance summary**: 14/15 compliant, 1 partial by design.

### Correctness (static)

| Requirement | Status | Notes |
|------------|--------|-------|
| Record: load/create, known keys offline, forward path + query + original bytes | ✅ | `Recorder::open` after key check; `send(path_and_query, body)` |
| Single-flight per key | ✅ | `OnceCell::get_or_try_init`; unpolled `fetch` future is dropped when the cell is filled |
| Only 2xx stored; others returned unchanged | ✅ | `NotStored(Box<Response>)`, headers minus hop-by-hop |
| Client gets what replay returns | ✅ | record answers via the same `respond(entry)` as replay |
| 502 upstream / non-UTF-8, 500 write/secret | ✅ | `From<reqwest::Error>` strips URL; source chain included |
| Passthrough: same header rules, no cassette I/O, non-JSON forwarded | ✅ | no normalization; body buffered until D.4 |
| Key only from env, required, never printed | ✅ | empty = missing; sensitive header values |
| `--upstream` required for record/passthrough (exit 2) | ✅ | `ConfigError::MissingUpstream` |
| No panics on external input | ✅ | no unwrap/expect/indexing outside tests |

### Coherence (design)

| Decision | Followed? | Notes |
|----------|-----------|-------|
| Catch-all handler, `Backend` per mode | ✅ | inner `Arc`s dropped (deviation note) |
| Fill missing, per-key single-flight | ✅ | |
| Non-2xx returned, not stored | ✅ | |
| Inject env key, strip client credentials | ✅ | |
| `ProxyError` variants 400/599/502/500 | ⚠️ Deviated | + `NotStored`; documented in design.md (PR C2) |
| SSE stored as `events` | ➖ | PR D (D.3); record stores `body` meanwhile |
| `mod upstream` private (S-5) | ✅ | |

### Issues

**CRITICAL**: None.

**WARNING**:
- W-6: Batch size ≈ 910 inserted lines vs ≈ 650 estimated (~800 budget); ≈ 630 are tests + fake upstream, expanded by rustfmt. All tasks landed green; nothing deferred.

**SUGGESTION**:
- S-7: If a coalesced leader gets a non-2xx, each waiting duplicate retries the upstream in turn (tokio `OnceCell` semantics). Acceptable (errors are not cached); revisit only if 429 storms matter.
- S-8: The log outcome `recorded` is also used for coalesced followers that did not call upstream; cosmetic.

Resolved from C1: W-4 (`[graph] targets`), W-5 (empty key = missing), S-5 (`upstream` private).

### Verdict

**PASS WITH WARNINGS** — PR C2 is complete; all gates green; 14/15 scoped scenarios compliant, 1 partial by design (E-live half in PR D). Only warning is the batch size.

## PR D — SSE streaming + normalization flags (2026-11-26)

Scope: D.1-D.5 (llm-replay-cassette E-split, E-exact, E-tail, E-store, N-ptr, N-pat; llm-replay-proxy
E-resp, E-msg, E-chat, E-live, X-vol, X-help extension).

### Completeness

| Metric | Value |
|--------|-------|
| Tasks in batch | 5 |
| Tasks complete | 5 |
| Tasks incomplete | 0 |

### Build and tests (executed in the worktree)

| Command | Result |
|---------|--------|
| `cargo fmt --all -- --check` | ✅ |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | ✅ no warnings |
| `cargo test --workspace --locked` | ✅ 72 passed, 0 failed (nf-lineage 1; lib 34, captures 2, cli 10, record 11, replay 8, secrets 1, sse 5) |
| `cargo deny check` | ✅ advisories, bans, licenses, sources ok |
| `cargo machete` | ✅ no unused dependencies |

### Mutation spot checks

| Mutation | Result |
|----------|--------|
| Record keeps SSE as `body` (pre-D.3 code) | E-store FAILED |
| Buffered `relay` (`reply.bytes()`, pre-D.4 code) | E-live FAILED (timeout while upstream paused) |
| Top-level volatile removal disabled | N-ptr, N-pat, N-vol FAILED |
| Regex error printed multi-line | N-pat FAILED |

### TDD compliance

| Task | RED | GREEN | Triangulate | Notes |
|------|-----|-------|-------------|-------|
| D.1 | ✅ E0425 | ✅ 3/3 | ✅ | |
| D.2 | ➖ helper | ✅ exercised by 5 tests | ➖ | |
| D.3 | ✅ E-store FAILED | ✅ | ✅ | E-resp/E-msg/E-chat passed before the change (byte-exact via concatenated body): regression guards |
| D.4 | ✅ E-live FAILED | ✅ | ✅ | |
| D.5 | ✅ compile errors | ✅ | ✅ | mutation-checked |

5/5 tasks have TDD evidence in apply-progress.md.

### Spec compliance matrix (PR D checks)

| Check | Scenario | Test | Result |
|-------|----------|------|--------|
| E-split | Mixed terminators | `sse::tests::splits_lf_crlf_and_cr_terminated_events` | ✅ COMPLIANT |
| E-exact | Mixed terminators | `sse::tests::concatenated_events_equal_the_original_bytes` | ✅ COMPLIANT |
| E-tail | Mixed terminators | `sse::tests::unterminated_trailing_event_is_kept` | ✅ COMPLIANT |
| E-store | Stored per event | `sse::sse_cassette_stores_one_string_per_event` | ✅ COMPLIANT (5 events, no `body`) |
| E-resp | Byte-exact SSE | `sse::responses_stream_record_replay_is_byte_exact` | ✅ COMPLIANT |
| E-msg | Byte-exact SSE | `sse::messages_stream_with_beta_query_record_replay_is_byte_exact` | ✅ COMPLIANT (CRLF event kept) |
| E-chat | Byte-exact SSE | `sse::chat_completions_stream_record_replay_is_byte_exact` | ✅ COMPLIANT |
| E-live | No cassette I/O | `sse::passthrough_streams_events_incrementally` | ✅ COMPLIANT (closes the P-pass partial from C2) |
| N-ptr | Configured additions | `normalize::tests::json_pointer_volatile_field_is_removed` | ✅ COMPLIANT |
| N-pat | Configured additions | `normalize::tests::custom_volatile_pattern_is_replaced` | ✅ COMPLIANT |
| X-vol | CLI errors | `cli::invalid_volatile_pattern_exits_1_naming_it` | ✅ COMPLIANT (flag + env; one stderr line; bad pointer too) |
| X-help | CLI errors | `cli::help_lists_flags_and_env_vars` | ✅ COMPLIANT (now every flag) |

**Compliance summary**: 12/12 compliant.

### Correctness (static)

| Requirement | Status | Notes |
|------------|--------|-------|
| `text/event-stream` stored as raw events incl. terminator, tail kept, concat byte-exact | ✅ | `split_events` cuts only after ASCII line endings (`get`, no indexing) |
| Replay: same bytes, `content-type`, no delays | ✅ | `Body::from_stream(stream::iter(events))` |
| Passthrough streams as it arrives, no cassette I/O | ✅ | `Body::from_stream(bytes_stream())`; mid-stream upstream errors end the body |
| Volatile fields: name = top level only, pointer = exact member, missing ignored; defaults kept | ✅ | |
| Patterns in string values only, in order, `<volatile/>` | ✅ | |
| Invalid pattern/pointer → startup error naming it, exit 1, one line | ✅ | |
| No panics on external input | ✅ | no unwrap/expect/indexing outside tests |

### Coherence (design)

| Decision | Followed? | Notes |
|----------|-----------|-------|
| SSE stored as raw event strings incl. terminator | ✅ | |
| Streaming detected from upstream `content-type`, not the request | ✅ | prefix match |
| `Normalizer::new(extra_fields, extra_patterns)`, `VolatileField`, `NormalizeError::Field` | ✅ | array-index removal added (deviation note) |
| Record forwards and stores (buffered) | ⚠️ Deviated (documented) | record buffers, then answers from the entry; only passthrough streams live |

### Issues

**CRITICAL**: None.

**WARNING**: None (batch ≈ 575 changed lines, within the ~800 budget).

**SUGGESTION**:
- S-7 (from C2) still open: a non-2xx coalesced leader makes each waiting duplicate retry once.
- S-9: replay's one-chunk-per-event framing is not asserted (hyper may coalesce small writes); bytes are.

Resolved from C2: S-8 (coalesced waiters log `hit`), P-pass partial (E-live).

### Verdict

**PASS** — PR D is complete; all gates green; 12/12 scoped scenarios compliant.
