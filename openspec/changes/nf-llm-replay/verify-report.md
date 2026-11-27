# Verification Report: nf-llm-replay

**Mode**: Strict TDD. Scope so far: PR A (A.1-A.6). Later batches merge their sections below.

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
