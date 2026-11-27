# llm-replay-cassette Specification

## Purpose

How `nf-llm-replay` turns an HTTP request into a stable key, and how request/response pairs are stored
in a reviewable, secret-free JSONL cassette. Acceptance criteria: "volatile fields give the same key"
and "no cassette contains Authorization or keys"; see `docs/verification.md` section 4.3, items 4-5.

## Verification checks

Unit tests live in `crates/nf-llm-replay/src/<module>.rs` (`mod tests`); integration tests in
`crates/nf-llm-replay/tests/<file>.rs`. "PR" = the batch in tasks.md that adds the test.

| ID | Test | PR |
|----|------|----|
| N-sort | `normalize::tests::object_keys_are_sorted_recursively` | A |
| N-ws | `normalize::tests::whitespace_and_key_order_do_not_change_key` | A |
| N-vol | `normalize::tests::default_volatile_fields_do_not_change_key` | A |
| N-nested | `normalize::tests::nested_field_named_like_a_volatile_field_is_kept` | A |
| N-cwd | `normalize::tests::cwd_block_is_replaced_in_every_string_value` | A |
| N-cwd-edge | `normalize::tests::unterminated_cwd_block_is_left_unchanged` | A |
| N-query | `normalize::tests::query_string_is_not_part_of_key` | A |
| N-route | `normalize::tests::method_and_path_are_part_of_key` | A |
| N-empty | `normalize::tests::empty_body_is_keyed_as_empty_string` | A |
| N-bad | `normalize::tests::non_json_body_is_rejected` | A |
| N-golden | `captures::capture_keys_are_stable` (golden keys for the 8 lab captures) | A |
| N-pi | `captures::pi_requests_differing_only_in_cwd_and_cache_key_share_a_key` | A |
| N-ptr | `normalize::tests::json_pointer_volatile_field_is_removed` | D |
| N-pat | `normalize::tests::custom_volatile_pattern_is_replaced` | D |
| K-line | `cassette::tests::entry_round_trips_as_one_json_line` | A |
| K-ver | `cassette::tests::unknown_version_is_rejected_with_line_number` | A |
| K-bad | `cassette::tests::malformed_line_is_rejected_with_line_number` | A |
| K-dup | `cassette::tests::duplicate_key_is_rejected_with_both_line_numbers` | A |
| K-extra | `cassette::tests::unknown_entry_fields_are_ignored` | A |
| K-sorted | `cassette::tests::insert_rewrites_file_sorted_by_key_without_temp_leftovers` | C |
| K-conc | `record::record_concurrent_distinct_requests_produce_valid_sorted_cassette` | C |
| K-guard | `cassette::tests::entry_containing_the_api_key_is_refused` | C |
| S-scan | `secrets::recorded_cassette_contains_no_credentials` | C |
| E-split | `sse::tests::splits_lf_crlf_and_cr_terminated_events` | D |
| E-exact | `sse::tests::concatenated_events_equal_the_original_bytes` | D |
| E-tail | `sse::tests::unterminated_trailing_event_is_kept` | D |
| E-store | `sse::sse_cassette_stores_one_string_per_event` | D |

## Requirements

### Requirement: Canonical request and key

The key MUST be the lowercase hex SHA-256 of `METHOD + " " + PATH + "\n" + CANONICAL_BODY`, where
PATH is the request URI path exactly as received (no percent-decoding) and the query string is
excluded. CANONICAL_BODY MUST be the normalized JSON body serialized with object keys sorted by
byte order at every depth and no insignificant whitespace (numbers and string escapes as
`serde_json` prints them), independent of any `serde_json` map-ordering feature. An empty body MUST
canonicalize to the empty string. A non-empty body that is not valid JSON MUST be rejected (never keyed).
Headers MUST NOT influence the key.

#### Scenario: Reordered and reformatted body (N-sort, N-ws)
- GIVEN two bodies equal as JSON values but with different key order and whitespace
- WHEN keys are computed
- THEN they are equal

#### Scenario: Query and route (N-query, N-route)
- GIVEN `POST /v1/messages?beta=true` and `POST /v1/messages`
- WHEN keyed
- THEN keys are equal; a different method or path gives a different key

#### Scenario: Empty and invalid bodies (N-empty, N-bad)
- GIVEN an empty body, and the body `not json`
- WHEN normalized
- THEN the first keys as `""`; the second returns a typed error, no panic

### Requirement: Volatile parts

Before canonicalization the normalizer MUST: (1) remove each configured volatile field — a bare name
removes that key from the top-level object only; a value starting with `/` is an RFC 6901 JSON
Pointer removing exactly that member (missing targets ignored); (2) then, in every JSON string
value at any depth (never object keys), replace each non-overlapping match of each volatile pattern,
in order, with the literal `<volatile/>`. Defaults (always active): fields `user`, `metadata`,
`prompt_cache_key`; pattern `(?s)<cwd>.*?</cwd>`. Configured fields/patterns MUST be added to the defaults.
An invalid pattern or pointer MUST be a startup error naming it.

#### Scenario: Default fields (N-vol)
- GIVEN two requests differing only in top-level `user`, `metadata` or `prompt_cache_key`
- WHEN keyed
- THEN keys are equal

#### Scenario: Nested name is not volatile (N-nested)
- GIVEN a tool schema with a property named `user`
- WHEN normalized
- THEN the property is kept and changing its schema changes the key

#### Scenario: Task work dir in the prompt (N-cwd, N-pi)
- GIVEN pi requests whose `developer` content / Anthropic `system[0].text` end with `<cwd>\n/abs/work/xx/hash\n</cwd>`
- WHEN two such requests differ only in that path and `prompt_cache_key`
- THEN keys are equal and the stored body contains `<volatile/>`, not the path

#### Scenario: Unterminated block (N-cwd-edge)
- GIVEN a string `see <cwd> without end`
- WHEN normalized
- THEN it is unchanged

#### Scenario: Configured additions (N-ptr, N-pat)
- GIVEN `--volatile-field /metadata/user_id` and `--volatile-pattern 'run-[0-9]+'`
- WHEN normalizing
- THEN only that nested member is removed and each match becomes `<volatile/>`

#### Scenario: Key stability across releases (N-golden)
- GIVEN the 8 lab captures (host paths scrubbed) under `tests/fixtures/captures/`
- WHEN keyed with defaults
- THEN each key equals its committed golden value (changing it is a cassette-breaking change)

### Requirement: Cassette format

A cassette MUST be UTF-8 JSONL: one entry per line, `\n`-terminated, blank lines ignored. Entry fields,
in this order: `v` (integer, MUST be `1`), `key`, `request` {`method`, `path`, `body` = the normalized
canonical body embedded as JSON, or `null` when empty}, `response` {`status`, `content_type`
(nullable), and exactly one of `body` (string, exact bytes) or `events` (array of strings)}.
Unknown fields MUST be ignored. Load errors (unreadable file, malformed line, `v ≠ 1`, both/neither of
`body`/`events`, duplicate key) MUST fail startup with the 1-based line number(s); no panic.

#### Scenario: Round trip (K-line)
- GIVEN an entry
- WHEN serialized and parsed
- THEN it is one line and equal to the original

#### Scenario: Bad cassettes (K-ver, K-bad, K-dup, K-extra)
- GIVEN a line with `"v":2`, a truncated line, two lines with one key, and a line with an extra field
- WHEN loaded
- THEN the first three fail naming the line(s); the fourth loads

### Requirement: Stable ordering and safe concurrent writes

The cassette file MUST always be sorted by `key`. Each new entry MUST be persisted by rewriting the
whole file to a temporary sibling and renaming it over the cassette, serialized by one in-process lock,
so readers never see a partial file and concurrent recordings cannot interleave. Replay MUST NOT
create or write the file. Multiple processes writing one cassette are unsupported.

#### Scenario: Sorted atomic insert (K-sorted)
- GIVEN entries inserted in arbitrary order
- WHEN the file is read
- THEN lines are sorted by key and no temporary file remains

#### Scenario: Parallel recording (K-conc)
- GIVEN 32 distinct requests recorded concurrently
- WHEN the cassette is reloaded
- THEN it has 32 valid, sorted entries

### Requirement: No secrets in cassettes

Cassettes MUST NOT contain request or response headers (only `content_type`), query strings, or
credential values. Before writing, an entry whose serialized line contains the configured upstream
API key MUST be refused (request fails, nothing written).

#### Scenario: Secret scan after recording (S-scan)
- GIVEN a record session with env key `sk-test-SECRET-123` and clients sending `Authorization`, `x-api-key` and `?key=` queries
- WHEN the cassette is scanned
- THEN it contains none of: the key value, the client credentials, `authorization`, `x-api-key`, `Bearer `, `key=`

#### Scenario: Echoed key (K-guard)
- GIVEN an upstream response body that echoes the API key
- WHEN recording
- THEN the entry is not written and the client gets HTTP 500 saying why (without the key)

### Requirement: Streamed responses

A `text/event-stream` response MUST be stored as `events`: the raw text of each event in order, each
string including its terminating blank line (`\n\n`, `\r\n\r\n` or `\r\r`); a trailing unterminated
fragment MUST be kept as a final element. Concatenating `events` MUST reproduce the upstream bytes exactly.

#### Scenario: Mixed terminators (E-split, E-exact, E-tail)
- GIVEN an SSE body with `\n\n`, `\r\n\r\n` and `\r\r` terminators and an unterminated tail
- WHEN split
- THEN one element per event and the concatenation equals the input bytes

#### Scenario: Stored per event (E-store)
- GIVEN a recorded `/v1/responses` stream of 5 events
- WHEN the cassette is read
- THEN `response.events` has 5 strings and no `body`
