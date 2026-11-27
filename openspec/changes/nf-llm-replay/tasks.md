# Tasks: nf-llm-replay

Five PRs (A-E). Estimates are hand-written code + tests
(excluding openspec/, `Cargo.lock`, `tests/fixtures/`, the example cassette). Each PR ends with
`cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`,
`cargo test --workspace --locked`, `cargo deny check`, `cargo machete` green. Check IDs are in
`specs/*/spec.md`. Strict TDD: every GREEN step follows its named failing test (RED).
Every `tests/*.rs` / `examples/*.rs` starts with a `//!` line.

## PR A — normalization + cassette model (library only) · ≈ 500 lines

- [x] A.1 Root `Cargo.toml` `[workspace.dependencies]` (design list); `crates/nf-llm-replay/Cargo.toml` (`0.1.0`, `publish = false`, lib + `[[bin]]`, only deps used in A); `src/lib.rs` docs ("pre-alpha", research use) + `src/main.rs` stub returning `Ok(())`.
- [x] A.2 RED→GREEN `src/normalize.rs`: N-sort, N-ws, N-empty, N-bad (`write_canonical` sorts keys itself; hex via `{b:02x}`).
- [x] A.3 RED→GREEN default volatile rules: N-vol, N-nested, N-cwd, N-cwd-edge (`<volatile/>` replacement, string values only).
- [x] A.4 RED→GREEN key recipe: N-query, N-route (`METHOD␠PATH\nCANONICAL`).
- [x] A.5 Generate `tests/fixtures/captures/*.json` from the 8 lab captures ({method, path, body}; replace lab host paths with `/lab/...`); RED→GREEN `tests/captures.rs`: N-pi, then N-golden (commit computed keys).
- [x] A.6 RED→GREEN `src/cassette.rs` types + `Cassette::load/get`: K-line, K-ver, K-bad, K-dup, K-extra (1-based line numbers, `status` 100-999, exactly one of body/events).

> Deviation (2026-11-26, PR A): `[workspace.dependencies]` lists only the dependencies a crate
> already uses (cargo 1.100 warns `unused workspace dependency` for every other entry); each PR adds
> its own. `Normalizer::new()` takes no arguments until D.5 adds `--volatile-field/--volatile-pattern`
> (see design.md).

## PR B — replay server + CLI · ≈ 700 lines

- [x] B.1 RED→GREEN `src/config.rs`: `Cli`/`Mode`/`ApiKey`/`Config::from_cli`; U-dbg.
- [x] B.2 `tests/common/mod.rs`: `start(config) -> SocketAddr` (in-process `app()` on `127.0.0.1:0`), `write_cassette(entries)`, counting TCP listener used as dead upstream.
- [x] B.3 RED→GREEN `src/proxy.rs` replay path + `ProxyError::IntoResponse`: R-hit, R-same, R-400, R-big (32 MiB limit).
- [x] B.4 RED→GREEN miss contract: U-sum, U-hint, R-miss (message names method, path, model, key prefix, record command).
- [x] B.5 RED→GREEN R-net (counter stays 0), R-h2c (raw TCP request copied from langchain4j capture), R-conc (64 parallel).
- [x] B.6 `src/main.rs` (anyhow, tracing to stderr, `listening on` to stdout, SIGINT/SIGTERM graceful); RED→GREEN `tests/cli.rs`: X-help, X-mode, X-env, X-nocas, X-corrupt, X-listen.

> Deviation (2026-11-26, PR B): no `assert_cmd` (std `Command` + `env!("CARGO_BIN_EXE_nf-llm-replay")`
> suffice) and no HTTP client dev-dep (tests use a raw HTTP/1.1 client in `tests/common`, needed for
> R-h2c anyway). `--volatile-field/--volatile-pattern` flags land with D.5 (X-help checks the flags that
> exist; D.5 extends it). Record/passthrough return `StartupError::Unsupported` until PR C. Actual size
> ≈ 1,190 hand-written lines (≈ 590 tests/helpers), not ≈ 700.

## PR C1 — upstream client + Recorder (library only) · ≈ 450 lines

- [x] C.1 `deny.toml`: add `CDLA-Permissive-2.0` with comment; `cargo deny check` green after adding reqwest.
- [x] C.2 RED→GREEN `src/upstream.rs`: U-hdr, U-auth; `Upstream` (600 s timeout, 10 s connect, no redirects, no compression) + `send` unit test against a one-shot local listener (base + path + query, injected key, 3xx not followed).
- [x] C.4 RED→GREEN `Recorder` (`cassette.rs`): K-sorted, K-guard (+ per-key cells shared and pre-filled from the file; open creates an absent file).

## PR C2 — record + passthrough wiring · ≈ 650 lines

- [x] C.3 `tests/common/fake_upstream.rs` (chat JSON only: plain, `json_schema`, tool call `call_1`, tool-result echo; counters, last headers/URI, forced status, echo-key).
- [x] C.5 RED→GREEN record path in `proxy.rs` (`tests/record.rs`): W-rt, W-tool, W-fill, W-once, W-err, W-502, W-auth, W-fwd, K-conc.
- [x] C.6 RED→GREEN passthrough: P-pass (buffered streaming check moves to D).
- [x] C.7 RED→GREEN `tests/secrets.rs` S-scan; `tests/cli.rs` X-nokey, X-noup, X-nolog.

> Deviation (2026-11-26, PR C split): PR B landed at ≈ 1,190 lines vs ≈ 700 estimated, so the
> ≈ 760-line PR C is split. **C1** = the upstream client and the `Recorder` as library code with unit
> tests (C.1, C.2, C.4); **C2** = wiring record/passthrough into `app()`/`proxy.rs`/CLI, the fake
> upstream and the integration/secret-scan tests (C.3, C.5-C.7). Task IDs are unchanged. `deny.toml`
> moves with reqwest into C1.

> Deviation (2026-11-26, PR C1): `Upstream::new(base: &str, key) -> Result<_, UpstreamError>` validates
> the key as a header value once; `send` returns `reqwest::Error` (URL stripped) and C2 maps it to the
> 502. `pub mod upstream` (private would be dead code until C2 wires it). `Config.upstream` stays a
> validated `String`. reqwest features `rustls, http2` (`stream` lands with D.4). Actual size ≈ 470 lines.

> Deviation (2026-11-26, PR C2): `--upstream` missing in record/passthrough is a usage error
> (`ConfigError::MissingUpstream`, exit 2); an empty `NF_LLM_REPLAY_API_KEY` counts as missing
> (`StartupError::MissingApiKey`, exit 1, checked before the cassette is created); `StartupError::Unsupported`
> is gone. `upstream` is a private module. The fake upstream adds a `delay_ms` knob (so W-once exercises
> coalescing, not just fill) and records the last body (P-pass checks non-JSON bodies are forwarded
> unchanged). Extra test `record::echoed_key_returns_500_and_is_not_recorded` (500 mapping end to end).
> `deny.toml` gets `[graph] targets` (Linux gnu/musl + macOS) to drop the Android-only `syn` duplicate.
> Actual size ≈ 910 inserted lines (≈ 630 tests/fake), not ≈ 650; all C2 tasks landed, nothing moved.

## PR D — SSE streaming + normalization flags · ≈ 420 lines

- [x] D.1 RED→GREEN `src/sse.rs`: E-split, E-exact, E-tail.
- [x] D.2 Fake upstream SSE: chat `stream:true`, `/v1/responses` (5 events), `/v1/messages` (Anthropic event names, CRLF on one event), pause gate.
- [x] D.3 RED→GREEN record stores `events`, replay streams one chunk per event (`tests/sse.rs`): E-store, E-resp, E-msg, E-chat.
- [x] D.4 RED→GREEN passthrough streams `bytes_stream()`: E-live.
- [x] D.5 RED→GREEN `--volatile-field` / `--volatile-pattern` (+ env): N-ptr, N-pat, X-vol.

> Deviation (2026-11-26, PR D): record still buffers the upstream answer (one shared entry per key) and
> splits it into `events`; only passthrough and relayed non-2xx answers stream live. SSE tests use reqwest
> (already a normal dependency) as the client; the raw test client learned chunked decoding because relayed
> answers are no longer content-length framed. E-resp/E-msg/E-chat passed before D.3 (concatenated `body`
> was already byte-exact): they are regression guards, E-store and E-live were the RED drivers. Suggestion
> S-8 from C2 done (coalesced waiters log `hit`); S-7 not done. Actual size ≈ 575 changed lines; nothing moved.

## PR E — Docker image, offline example, CI · ≈ 380 lines

- [ ] E.1 `examples/fake_upstream.rs` (`#[path]` reuse, `--port`).
- [ ] E.2 `docs/examples/nf-llm-replay/main.nf` (qa, qa_structured, shouty + `uppercase` process tool), `nextflow.config` (`REPLAY_BASE_URL`, no key), `record.sh`; run it to produce `cassettes/example.jsonl` (twice: identical bytes).
- [ ] E.3 RED→GREEN `tests/secrets.rs` S-cas (no credentials, no absolute host paths).
- [ ] E.4 `crates/nf-llm-replay/Dockerfile`, root `.dockerignore`; D-build, D-run, D-user, D-size locally.
- [ ] E.5 `e2e.sh` (skip logic, plugin prefetch, `--internal` network, asserts 3 outputs + 0 misses, trap cleanup + chown); D-skip, D-e2e, D-lint; manual: delete a cassette line → 599 visible.
- [ ] E.6 `README.md` (replay, record, `OPENAI_API_KEY` warning, same Nextflow flags for record/replay, Docker record mounts dir + `--user`, pi notes, research-only); D-doc.
- [ ] E.7 `.github/workflows/ci.yml` `docker` job (`timeout-minutes: 20`, no secrets/push); V-yaml; C-docker on the PR run.
