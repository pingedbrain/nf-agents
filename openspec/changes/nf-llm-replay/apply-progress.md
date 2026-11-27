# Apply progress: nf-llm-replay

**Mode**: Strict TDD (runner `cargo test --workspace`). Batches: PR A done (6/6 tasks); PR B-E pending.
Overall: 6/31 tasks.

## PR A — normalization + cassette model (2026-11-26)

### Files changed

| File | Action | What |
|------|--------|------|
| `Cargo.toml` | Modified | `[workspace.dependencies]`: anyhow, regex, serde, serde_json (`raw_value`), sha2, tempfile, thiserror |
| `Cargo.lock` | Modified | generated |
| `crates/nf-llm-replay/Cargo.toml` | Created | `0.1.0`, `publish = false`, inherited package fields, `[lints] workspace = true` |
| `crates/nf-llm-replay/src/lib.rs` | Created | crate docs (pre-alpha, research/testing use), `pub mod cassette; pub mod normalize;` |
| `crates/nf-llm-replay/src/main.rs` | Created | stub `fn main() -> anyhow::Result<()>` |
| `crates/nf-llm-replay/src/normalize.rs` | Created | `Normalizer::new/normalize`, `Normalized`, `NormalizeError::{Body, Pattern}`, own sorted canonical writer, default volatile fields + `<cwd>` pattern, key recipe; 10 unit tests |
| `crates/nf-llm-replay/src/cassette.rs` | Created | `Entry`, `RecordedRequest`, `RecordedResponse`, `CassetteError::{Io, Line, Duplicate}`, `Cassette::load/get` (v = 1, status 100-999, exactly one of body/events, duplicates); 5 unit tests |
| `crates/nf-llm-replay/tests/captures.rs` | Created | N-golden (8 committed keys, fixture list must match), N-pi |
| `crates/nf-llm-replay/tests/fixtures/captures/*.json` | Created (generated) | 8 lab requests as `{method, path, body}`; the local scratch directory path replaced by `/lab`; headers dropped |

Fixtures were generated with a throwaway Python script (not committed) from
the captured requests in a local scratch directory; the script asserted no user name or `/home/`
remains. Golden keys were cross-checked with an independent Python implementation of the recipe
(`json.dumps(sort_keys=True, separators=(',', ':'), ensure_ascii=False)` + same volatile rules): all 8 match.

### TDD cycle evidence

| Task | Test file | Layer | Safety net | RED | GREEN | Triangulate | Refactor |
|------|-----------|-------|------------|-----|-------|-------------|----------|
| A.1 | — | config | ✅ workspace 1/1 | — | `cargo test` builds new crate | ➖ structural (manifests, stub main) | ➖ |
| A.2 | `src/normalize.rs` | Unit | N/A (new) | ✅ N-sort, N-ws, N-empty, N-bad → E0425 (no `Normalizer`) | ✅ 4/4 | ✅ nested arrays/objects, unicode + escaped quote, different-content key differs, empty vs `{}` differ, fixed sha256 of `GET /v1/models\n` | ➖ |
| A.3 | `src/normalize.rs` | Unit | ✅ 4/4 | ✅ N-vol, N-nested, N-cwd, N-cwd-edge → 2 FAILED (N-vol, N-cwd) | ✅ 8/8 | ✅ each default field alone + combined, non-volatile field changes key, `<cwd>` in object key kept, two blocks in one string | ➖ |
| A.4 | `src/normalize.rs` | Unit | ✅ 8/8 | ✅ N-query, N-route → N-query FAILED (query stripping removed for the RED, it had been written with A.2) | ✅ 10/10 | ✅ `beta=true`, `key=` query, method/path/percent-encoding variants, fixed sha256 | ➖ |
| A.5 | `tests/captures.rs` | Integration | ✅ 10/10 | ✅ N-golden with empty golden values → FAILED; N-pi passed on first run (behavior already built in A.3; it is the acceptance test on real data) | ✅ 2/2 | ✅ N-pi on OpenAI (`developer` + `prompt_cache_key`) and Anthropic (`system[0].text`) captures | ➖ |
| A.6 | `src/cassette.rs` | Unit | ✅ 12/12 | ✅ K-line, K-ver, K-bad, K-dup, K-extra → E0425 (no `Cassette`) | ✅ 15/15 lib + 2 integration | ✅ JSON and SSE entries, `null` body/content_type, both/neither body+events, status 99/1000, missing file → `Io` naming the path | removed unused `Default` derive |

### Test summary

- Tests written: 17 (15 unit, 2 integration); all passing.
- Pure functions: `write_canonical`, `parse_line`, `Normalizer::normalize`.

### Deviations from design

Recorded in design.md ("Deviation (2026-11-26, PR A)") and tasks.md:
- `[workspace.dependencies]` holds only used deps (cargo 1.100 warns on unused entries).
- `Normalizer::new()` has no arguments until D.5; `CassetteError::Secret` arrives in PR C.

### Issues found

- Clippy's `allow-*-in-tests` does not cover helper fns in integration test files; `tests/captures.rs` has a file-level `#![allow(clippy::unwrap_used, clippy::indexing_slicing)]` with a comment. PR B-D test files will need the same.
- The 8 captures contain no two requests with the same normalized body, so N-pi builds the second request by rewriting the work dir and `prompt_cache_key` of a real capture.

### Remaining

PR B (B.1-B.6), PR C (C.1-C.7), PR D (D.1-D.5), PR E (E.1-E.7).
