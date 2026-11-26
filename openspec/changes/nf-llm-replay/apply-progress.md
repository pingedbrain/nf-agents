# Apply progress: nf-llm-replay

**Mode**: Strict TDD (runner `cargo test --workspace`). Batches: PR A done (6/6 tasks), PR B done (6/6 tasks), PR C1 done (3/3 tasks), PR C2 done (4/4 tasks), PR D done (5/5 tasks); PR E pending.
Overall: 24/31 tasks. (PR C was split into C1/C2 on 2026-11-26; task IDs unchanged, see tasks.md.)

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

## PR B — replay server + CLI (2026-11-26)

### Files changed

| File | Action | What |
|------|--------|------|
| `Cargo.toml` | Modified | workspace deps: axum 0.8.9, clap 4.6 (`derive, env`), tokio 1.54 (`rt-multi-thread, macros, net, signal`), tracing 0.1.44 (no default features, `std`), tracing-subscriber 0.3.23 (`fmt, env-filter`) |
| `crates/nf-llm-replay/Cargo.toml` | Modified | those deps; dev tokio `io-util, time` |
| `src/config.rs` | Created | `Mode`, `ApiKey` (`Debug` = `***`, `expose`), `Cli` (flags + env, `--upstream` http(s) value parser), `Config::from_cli`, `ConfigError::MissingCassette`; 3 unit tests |
| `src/proxy.rs` | Created | `AppState`, `Backend::Replay`, catch-all `handle` (one `info` line per request, `warn` on miss), `respond`, `ProxyError::{BadRequest, Miss}` + `IntoResponse`, `MissBody`/`MissRequest`, `summarize`, `message_text`, `default_upstream`; 2 unit tests |
| `src/lib.rs` | Modified | `mod proxy`, `StartupError`, `app(&Config)` (loads cassette, 32 MiB `DefaultBodyLimit`, fallback handler) |
| `src/main.rs` | Rewritten | clap parse (usage errors exit 2), tracing to stderr (`RUST_LOG`, default `info`), `app()` before bind, `listening on <addr>` on stdout, graceful SIGINT/SIGTERM, startup errors one line + exit 1 |
| `tests/common/mod.rs` | Created | `start`, `config`, `replay`, `entry`, `write_cassette`, raw HTTP/1.1 `send_raw`/`post` + `Reply`, `counting_upstream` |
| `tests/replay.rs` | Created | R-hit, R-same, R-400, R-big, R-miss, R-net, R-h2c, R-conc |
| `tests/cli.rs` | Created | X-help, X-mode, X-env, X-nocas, X-corrupt, X-listen (+ SIGTERM exit 0, request logged) |

### TDD cycle evidence

| Task | Test file | Layer | Safety net | RED | GREEN | Triangulate | Refactor |
|------|-----------|-------|------------|-----|-------|-------------|----------|
| B.1 | `src/config.rs` | Unit | ✅ 18/18 | ✅ U-dbg + defaults + upstream parser → E0425/E0433 (no `Cli`/`Config`/`Mode`) | ✅ 3/3 | ✅ http and https accepted; `ftp://`, no scheme, `http://`, garbage, bad mode rejected; passthrough needs no cassette, replay does | ➖ |
| B.2 | `tests/common/mod.rs` | helper | ✅ | ➖ helper module (exercised by B.3-B.6) | ✅ | ➖ structural | simplified `send_raw` read loop before first use |
| B.3 | `tests/replay.rs` | Integration | ✅ 21/21 | ✅ R-hit, R-same, R-400, R-big → E0425 (no `app`) | ✅ 4/4 | ✅ second entry (GET, 203, no content-type, query ignored); 3 invalid bodies; R-big re-checked: removing `DefaultBodyLimit` makes it FAIL | ➖ |
| B.4 | `src/proxy.rs`, `tests/replay.rs` | Unit + Integration | ✅ 25/25 | ✅ U-sum, U-hint, R-miss → E0425 (no `summarize`/`default_upstream`/`MissRequest`) | ✅ 20 lib + 5 replay | ✅ chat, `input` string, `input` items, Anthropic text blocks, 1000× multibyte truncation, empty/array/wrong-typed bodies; explicit `--upstream` overrides path hint | `type` moved into `MissBody` (`serde(rename)`) to drop JSON indexing flagged by clippy |
| B.5 | `tests/replay.rs` | Integration | ✅ 30/30 | ⚠️ R-net, R-h2c, R-conc passed on first run: acceptance tests of behaviour already built in B.3 (catch-all handler; hyper ignores `Upgrade: h2c`) | ✅ 8/8 | ✅ R-net proves the counter works (1 manual connection) and that hit + 2 misses add 0; R-conc 64 requests over 3 keys | ➖ |
| B.6 | `tests/cli.rs` | CLI (process) | ✅ 33/33 | ✅ 6/6 FAILED against the stub `main` | ✅ 6/6 | ✅ X-mode via flag and via env; missing file vs corrupt line 2; X-listen also checks SIGTERM → exit 0 and the request log line | `tracing` without `attributes` (removes duplicate `syn`) |

### Test summary

- Tests written in PR B: 19 (5 unit, 8 replay integration, 6 CLI); workspace total 37, all passing.
- Pure functions: `summarize`, `message_text`, `default_upstream`, `parse_upstream`, `Config::from_cli`.

### Deviations from design

Recorded in design.md ("Deviation (2026-11-26, PR B)") and tasks.md: `upstream` as validated
`String`; no `lib::serve` (bind after `app()`); `Backend` replay-only and `StartupError::Unsupported`
until PR C; events concatenated until D.3; volatile flags in D.5; `tracing` default features off;
no `assert_cmd` / HTTP client dev-deps.

### Issues found

- Size: ≈ 1,190 hand-written lines vs ≈ 700 estimated (≈ 590 are tests/helpers; rustfmt expands the
  table-like test fixtures). PR C (≈ 760 est.) will likely overshoot similarly.
- `json!` on a derived `Serialize` struct is infallible for these field types; no panic path on input.

## PR C1 — upstream client + Recorder (2026-11-26)

### Files changed

| File | Action | What |
|------|--------|------|
| `Cargo.toml` | Modified | workspace dep reqwest 0.13.5 (no default features; `rustls, http2`); tokio += `sync, fs, io-util` |
| `crates/nf-llm-replay/Cargo.toml` | Modified | `reqwest`; dev tokio keeps only `time` (`io-util` is now a normal feature) |
| `deny.toml` | Modified | `CDLA-Permissive-2.0` (webpki-root-certs, via reqwest rustls) with comment |
| `Cargo.lock` | Modified | generated |
| `src/upstream.rs` | Created | `Upstream::new/send`, `forward_headers`, `UpstreamError::{Key, Client}`; 600 s / 10 s timeouts, no redirects, no compression, URL stripped from errors; 4 unit tests |
| `src/cassette.rs` | Modified | `Recorder::open/cell/persist`, `Cell`, `write_sorted` (temp sibling + `sync_all` + rename), `CassetteError::{Write, Secret}`; 3 unit tests |
| `src/config.rs` | Modified | `ApiKey` field `pub(crate)` |
| `src/lib.rs` | Modified | `pub mod upstream` |

### TDD cycle evidence

| Task | Test file | Layer | Safety net | RED | GREEN | Triangulate | Refactor |
|------|-----------|-------|------------|-----|-------|-------------|----------|
| C.1 | — | config | ✅ 37/37 | — | `cargo deny check` ok with reqwest | ➖ structural (manifest/policy); confirmed `webpki-root-certs` is the only CDLA crate | ➖ |
| C.2 | `src/upstream.rs` | Unit | ✅ 37/37 | ✅ U-hdr, U-auth, send, send-error → E0425/E0433 (no `Upstream`/`UpstreamError`) | ✅ 4/4 | ✅ Bearer / `x-api-key` / `api-key` / both / no credential; all 15 dropped names + `proxy-*`; invalid key → `UpstreamError::Key`; base with path prefix + trailing `/`; mutation: default redirect policy and missing `without_url` each make a test FAIL | test asserts compare `to_str()` |
| C.4 | `src/cassette.rs` | Unit | ✅ 41/41 | ✅ K-sorted, K-guard, cells → E0425/E0599 (no `Recorder`, `Write`, `Secret`) | ✅ 27/27 lib | ✅ pre-existing line kept + 3 inserts out of order; key echoed in response vs in request body; `sk-test-other` accepted; failed write (dir removed) → `Write` naming the file and the entry is rolled back (mutation-checked); corrupt file → `Line 1`; absent file created empty | ➖ |

### Test summary

- Tests written in PR C1: 7 unit (4 upstream, 3 cassette); workspace total 44, all passing.
- Pure functions: `Upstream::forward_headers`.

### Deviations from design

Recorded in design.md ("Deviation (2026-11-26, PR C1)") and tasks.md (PR C split + C1 note).

### Issues found / notes for C2

- Empty `NF_LLM_REPLAY_API_KEY` would make the secret guard refuse every entry: treat empty as missing (X-nokey).
- Map `reqwest::Error` from `Upstream::send` to `ProxyError::Upstream` (502); `UpstreamError` and `CassetteError::Write` need `StartupError` variants when wiring `app()`.
- `cargo deny` duplicate warning `syn` 2/3 is back (Android-only `jni` via `rustls-platform-verifier`); optional fix: `[graph] targets` in deny.toml.

## PR C2 — record + passthrough wiring (2026-11-26)

### Files changed

| File | Action | What |
|------|--------|------|
| `src/config.rs` | Modified | `ConfigError::MissingUpstream` (record/passthrough); empty `NF_LLM_REPLAY_API_KEY` → `None`; 1 unit test |
| `src/lib.rs` | Modified | `app()` builds `Replay`/`Record`/`Passthrough`; `StartupError::{MissingApiKey, Upstream}` replace `Unsupported`; key + upstream checked before `Recorder::open`; `mod upstream` private |
| `src/proxy.rs` | Modified | `Backend::{Record, Passthrough}`; `ProxyError::{Upstream (502), Cassette (500), NotStored}` + `From<reqwest::Error>` (URL stripped); `answer` (per-mode dispatch, outcome), `record` (2xx → `Entry` → `persist`), `relay` (upstream answer minus hop-by-hop headers) |
| `src/cassette.rs` | Modified | `VERSION` is `pub(crate)` |
| `deny.toml` | Modified | `[graph] targets` (Linux gnu/musl, macOS): drops the Android-only `syn` duplicate warning |
| `tests/common/fake_upstream.rs` | Created | axum fake: plain / `json_schema` / tool call `call_1` / tool-result echo; knobs `hits`, `last` (uri, headers, body), `status`, `delay_ms`, `echo_key`; `spawn()`, `router()` |
| `tests/common/mod.rs` | Modified | `KEY`, `proxy(mode, cassette, upstream)`, `post_with` (extra header lines) |
| `tests/record.rs` | Created | W-rt, W-tool, W-once, W-fill, W-err, W-auth, W-fwd, W-502, P-pass, K-conc + echoed key → 500 |
| `tests/secrets.rs` | Created | S-scan |
| `tests/cli.rs` | Modified | X-nokey (unset and empty), X-noup (record and passthrough), X-nolog (`RUST_LOG=trace`, stdout+stderr) |

### TDD cycle evidence

| Task | Test file | Layer | Safety net | RED | GREEN | Triangulate | Refactor |
|------|-----------|-------|------------|-----|-------|-------------|----------|
| (config, for C.5/C.7) | `src/config.rs` | Unit | ✅ 44/44 | ✅ `record_and_passthrough_need_upstream_and_empty_key_is_missing` FAILED | ✅ 4/4 (existing passthrough test given `--upstream`, now required by spec) | ✅ record and passthrough; empty vs non-empty key | ➖ |
| C.3 | `tests/common/fake_upstream.rs` | helper | ✅ | ➖ helper (exercised by C.5-C.7) | ✅ | ➖ structural | ➖ |
| C.5 | `tests/record.rs` | Integration | ✅ 45/45 | ✅ 10 record tests FAILED (`StartupError::Unsupported`) | ✅ 11/11 | ✅ plain + `json_schema` + 2-turn tool chain replayed offline (fake hit count unchanged); 16 concurrent + 1 later repeat; pre-existing line kept byte-for-byte + new key added; 429 then retry records; echoed key → 500; Bearer vs `x-api-key` style; closed port → 502; 32 distinct keys sorted, no temp file. Mutation: bypassing `OnceCell` coalescing makes W-once FAIL | boxed `NotStored` (clippy `result_large_err`) |
| C.6 | `tests/record.rs` | Integration | ✅ | ✅ P-pass FAILED with the C.5 batch (`Unsupported`) | ✅ | ✅ JSON answer + non-JSON body forwarded byte-for-byte with the upstream's own 400; query forwarded; cassette not created | ➖ |
| C.7 | `tests/secrets.rs`, `tests/cli.rs` | Integration + CLI | ✅ 56/56 | ⚠️ S-scan, X-nokey, X-noup, X-nolog passed on first run: acceptance tests of behaviour built in C.5/config. Mutations: storing path+query in the entry makes S-scan FAIL; logging the key at `debug` makes X-nolog FAIL | ✅ 60/60 | ✅ 3 credential styles + `?key=` + `?beta=true`; unset and empty key; record and passthrough without upstream; trace output checked to contain `TRACE` and `recorded` | ➖ |

### Test summary

- Tests written in PR C2: 16 (1 unit, 11 record, 1 secrets, 3 CLI); workspace total 60, all passing.
- Pure functions: `reply` (fake), `ProxyError::outcome`.

### Deviations from design

Recorded in design.md ("Deviation (2026-11-26, PR C2)") and tasks.md.

### Issues found

- Size ≈ 910 inserted lines vs ≈ 650 estimated (≈ 630 are tests + fake; rustfmt expands the JSON fixtures). All
  C2 tasks landed; nothing was moved to a later batch.
- Record buffers the whole upstream body; SSE answers are stored as `body` until D.3 (spec'd behaviour lands there).

## PR D — SSE streaming + normalization flags (2026-11-26)

### Files changed

| File | Action | What |
|------|--------|------|
| `Cargo.toml` | Modified | workspace dep `futures-util 0.3.34` (no default features); reqwest += `stream` |
| `crates/nf-llm-replay/Cargo.toml` | Modified | `futures-util` |
| `Cargo.lock` | Modified | generated |
| `src/sse.rs` | Created | `split_events` (line based: `\n`/`\r\n`/`\r`, blank line ends an event, tail kept); 3 unit tests |
| `src/proxy.rs` | Modified | record stores `text/event-stream` as `events`; `respond` streams one chunk per event; `relay` streams `bytes_stream()` (infallible now); log outcome `recorded` only for the filling request (S-8) |
| `src/normalize.rs` | Modified | `Normalizer::new(extra_fields, extra_patterns)`, `VolatileField::{TopLevel, Pointer}` (RFC 6901, object member or in-range array index), `NormalizeError::Field`, one-line regex errors; 2 unit tests |
| `src/config.rs` | Modified | `--volatile-field` (`NF_LLM_REPLAY_VOLATILE_FIELDS`, comma) / `--volatile-pattern` (`NF_LLM_REPLAY_VOLATILE_PATTERNS`, newline), `Config.volatile_fields/patterns`; 1 unit test |
| `src/lib.rs` | Modified | `mod sse`; normalizer built from the config flags |
| `tests/common/fake_upstream.rs` | Modified | SSE answers (`CHAT_EVENTS`, `RESPONSES_EVENTS` ×5, `MESSAGES_EVENTS` with one CRLF event), `sse_events`, `pause` + `resume` gate after event 1 |
| `tests/common/mod.rs` | Modified | raw client decodes chunked bodies (`dechunk`); `Normalizer::new(&[], &[])` |
| `tests/sse.rs` | Created | E-resp, E-msg, E-chat, E-store, E-live (reqwest client) |
| `tests/cli.rs` | Modified | X-help lists the volatile flags + env vars; X-vol (flag and env, pattern and pointer) |
| `tests/captures.rs` | Modified | `Normalizer::new(&[], &[])` |

### TDD cycle evidence

| Task | Test file | Layer | Safety net | RED | GREEN | Triangulate | Refactor |
|------|-----------|-------|------------|-----|-------|-------------|----------|
| D.1 | `src/sse.rs` | Unit | ✅ 60/60 | ✅ E-split, E-exact, E-tail → E0425 (no `split_events`) | ✅ 3/3 | ✅ empty body, leading blank lines, `\r\n` + `\n` mixed terminator, multibyte, comment event, bare tail without newline | ➖ |
| D.2 | `tests/common/fake_upstream.rs` | helper | ✅ | ➖ helper (exercised by D.3/D.4) | ✅ | ➖ structural | ➖ |
| D.3 | `tests/sse.rs` | Integration | ✅ 63/63 | ✅ E-store FAILED (`events` missing, `body` set); ⚠️ E-resp/E-msg/E-chat passed before the change (concatenated body was already byte-exact): regression guards | ✅ 5/5 | ✅ 3 endpoints, `?beta=true`, CRLF event, fake hit count unchanged during replay | `respond` clones the events into an owned stream |
| D.4 | `tests/sse.rs` | Integration | ✅ | ✅ E-live FAILED (timeout: buffered `relay` held headers until the paused upstream finished) | ✅ | ✅ event 1 before resume + full body after + no cassette file; P-pass and W-err still green with chunked relays (raw client dechunks) | ➖ |
| D.5 | `src/normalize.rs`, `src/config.rs`, `tests/cli.rs` | Unit + CLI | ✅ 68/68 | ✅ N-ptr, N-pat, config flags, X-vol → E0061/E0599/E0609 (no args, no `Field`, no config fields) | ✅ 72/72 | ✅ escaped `~0`/`~1` tokens, array index, missing target, non-object body, bare extra name top-level only; 3 invalid fields; two patterns + defaults; flag vs env, pattern vs pointer. Mutations: TopLevel removal disabled → N-ptr, N-pat, N-vol FAIL; multi-line regex error → N-pat FAIL | ➖ |

### Test summary

- Tests written in PR D: 12 (6 unit, 5 SSE integration, 1 CLI; X-help extended); workspace total 72, all passing.
- Pure functions: `split_events`, `VolatileField::parse/remove`, `one_line`, `sse_events` (fake), `dechunk` (test client).

### Deviations from design

Recorded in design.md ("Deviation (2026-11-26, PR D)") and tasks.md.

### Issues found

- None blocking. One-chunk-per-event in replay is implemented but not asserted (hyper may coalesce writes).
- C2 S-7 (non-2xx leader → each coalesced waiter retries upstream) left as is.

### Remaining

PR E (E.1-E.7).
