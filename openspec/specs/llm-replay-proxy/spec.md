# llm-replay-proxy Specification

## Purpose

The `nf-llm-replay` HTTP proxy: what each mode does, which requests it must serve correctly, how it
talks to the upstream provider, how it fails, and how it is configured. The original plan as adjusted by
`docs/verification.md` section 4.3 (non-streaming chat completions first, SSE for Responses/Messages).

## Verification checks

| ID | Test | PR |
|----|------|----|
| R-hit | `replay::replay_hit_returns_recorded_status_content_type_and_body` | B |
| R-same | `replay::replay_repeated_request_returns_identical_bytes` | B |
| R-miss | `replay::replay_miss_returns_599_with_key_summary_and_record_command` | B |
| R-net | `replay::replay_never_contacts_upstream` | B |
| R-h2c | `replay::replay_ignores_h2c_upgrade_and_placeholder_bearer` | B |
| R-conc | `replay::replay_serves_64_concurrent_requests` | B |
| R-400 | `replay::invalid_json_body_returns_400` | B |
| R-big | `replay::eight_mib_request_body_is_accepted` | B |
| U-sum | `proxy::tests::miss_summary_handles_chat_responses_and_messages_shapes` | B |
| U-hint | `proxy::tests::default_upstream_hint_depends_on_path` | B |
| U-hdr | `upstream::tests::hop_by_hop_credential_and_encoding_headers_are_not_forwarded` | C |
| U-auth | `upstream::tests::api_key_is_injected_in_the_client_header_style` | C |
| U-dbg | `config::tests::debug_output_redacts_api_key` | B |
| W-rt | `record::record_then_replay_round_trip_chat_completions` | C |
| W-tool | `record::multi_turn_tool_round_trip_replays_offline` | C |
| W-once | `record::concurrent_identical_requests_call_upstream_once` | C |
| W-fill | `record::existing_key_is_served_without_upstream_call` | C |
| W-err | `record::non_2xx_upstream_response_is_returned_but_not_recorded` | C |
| W-auth | `record::env_key_is_injected_and_client_credentials_are_stripped` | C |
| W-fwd | `record::query_string_and_provider_headers_are_forwarded` | C |
| W-502 | `record::unreachable_upstream_returns_502_json` | C |
| P-pass | `record::passthrough_forwards_without_touching_the_cassette` | C |
| E-resp | `sse::responses_stream_record_replay_is_byte_exact` | D |
| E-msg | `sse::messages_stream_with_beta_query_record_replay_is_byte_exact` | D |
| E-chat | `sse::chat_completions_stream_record_replay_is_byte_exact` | D |
| E-live | `sse::passthrough_streams_events_incrementally` | D |
| X-help | `cli::help_lists_flags_and_env_vars` | B |
| X-mode | `cli::invalid_mode_exits_2` | B |
| X-env | `cli::mode_from_env_var_is_honoured` | B |
| X-nocas | `cli::replay_with_missing_cassette_exits_1` | B |
| X-corrupt | `cli::corrupt_cassette_exits_1_with_line_number` | B |
| X-nokey | `cli::record_without_api_key_exits_1` | C |
| X-noup | `cli::record_without_upstream_exits_2` | C |
| X-listen | `cli::port_0_prints_listening_address_and_serves` | B |
| X-nolog | `cli::api_key_never_appears_in_output` | C |
| X-vol | `cli::invalid_volatile_pattern_exits_1_naming_it` | D |

## Requirements

### Requirement: Generic routing

The proxy MUST accept any method and path (including `GET /v1/models`) and treat them uniformly.
It MUST correctly serve, at minimum: `POST /v1/chat/completions` (JSON, with `response_format`
`json_schema` strict, `tools`, multi-turn tool-call round trips, and `stream:true`), `POST /v1/responses`
(SSE) and `POST /v1/messages` incl. `?beta=true` (SSE). Request bodies up to 32 MiB MUST be accepted.
It MUST answer HTTP/1.1 requests carrying `Connection: Upgrade, HTTP2-Settings` / `Upgrade: h2c` as
plain HTTP/1.1, and MUST accept any credential (incl. `Bearer nxf-no-credential`) or none.

#### Scenario: langchain4j request shape (R-h2c)
- GIVEN a raw HTTP/1.1 request copied from the langchain4j capture (h2c headers, placeholder bearer)
- WHEN sent to a replay server holding its entry
- THEN the reply is `HTTP/1.1 200` with the recorded body

#### Scenario: Parallel agents (R-conc)
- GIVEN 64 concurrent requests for 3 recorded keys
- WHEN served
- THEN every response matches its key's entry

#### Scenario: Large body (R-big)
- GIVEN an 8 MiB JSON request recorded earlier
- WHEN replayed
- THEN it is served (no 413)

### Requirement: Replay mode

Replay MUST answer only from the cassette loaded at startup and MUST NOT open any outbound connection,
whether or not `--upstream` is set. A hit MUST return the recorded status, `content-type` and body
bytes (or the concatenated events, streamed one event per chunk). Each key has exactly one response,
returned for every matching request regardless of order or concurrency.

#### Scenario: Hit (R-hit, R-same)
- GIVEN a cassette with an entry for request Q
- WHEN Q is sent twice
- THEN both responses equal the entry byte-for-byte

#### Scenario: Network never used (R-net)
- GIVEN `--upstream` pointing to a live counting fake upstream
- WHEN hits and misses are replayed
- THEN the fake upstream received 0 connections

### Requirement: Miss contract

A replay miss MUST return HTTP **599**, `content-type: application/json`, body
`{"error":{"type":"nf_llm_replay_miss","message":M,"key":K,"request":{"method","path","model","messages","last_message"},"cassette":C,"record_command":R}}`.
`model` = body `model` or null; `messages` = length of `messages` or `input` array (1 if `input` is a
string, else null); `last_message` = text of the last message/input item (string content, or `text`
parts joined), truncated to 120 characters (char boundary) with `…`, or null. R MUST be
`NF_LLM_REPLAY_API_KEY=<key> nf-llm-replay --mode record --cassette C --upstream U` where U is
`--upstream` if set, else `https://api.anthropic.com` for paths ending `/messages`, else
`https://api.openai.com`. M MUST be one sentence naming method, path, model, key prefix (12 hex) and R,
since Nextflow prints the raw body. The miss MUST be logged at `warn` with the same summary.

#### Scenario: Missing request (R-miss)
- GIVEN an empty cassette and a chat request with 2 messages, last `make this loud: hello`
- WHEN replayed
- THEN status 599, `request.messages` = 2, `last_message` = that text, R contains `--mode record` and the cassette path

#### Scenario: Odd shapes (U-sum, U-hint)
- GIVEN bodies with no `model`, `input` as string, Anthropic content blocks, a 1000-char multibyte message
- WHEN summarized
- THEN fields are filled or null, truncation is at 120 chars, no panic; hints are per path

### Requirement: Record mode

Record MUST load the cassette (created if absent), answer existing keys from it without contacting
the upstream, and forward missing keys to `--upstream` + path + original query with the original
body bytes. Concurrent requests with the same missing key MUST share one upstream call. Only 2xx
responses MUST be stored; other statuses MUST be returned unchanged and not stored. The client
MUST receive exactly what a later replay returns (status, `content-type`, body/events).
Upstream connect/timeout (600 s)/non-UTF-8 errors MUST yield 502 JSON `nf_llm_replay_upstream_error`;
a failed write MUST yield 500 JSON `nf_llm_replay_cassette_error`. An upstream answer larger than 64 MiB (`MAX_UPSTREAM_BODY`) MUST NOT be read whole: it MUST yield 502 `nf_llm_replay_upstream_error` as soon as the limit is passed (T `proxy::tests::upstream_body_is_capped`).

#### Scenario: Round trip (W-rt, W-tool)
- GIVEN a fake upstream answering plain, `json_schema` strict and tool-call requests, then a tool-result follow-up
- WHEN a client records the chain and the same chain is replayed with the fake upstream stopped
- THEN replay responses equal the recorded ones and no 599 occurs

#### Scenario: Coalescing and fill (W-once, W-fill)
- GIVEN 16 identical concurrent requests, then a later repeat
- WHEN recorded
- THEN the fake upstream saw exactly 1 request and all 17 responses are equal

#### Scenario: Upstream failure (W-err, W-502)
- GIVEN an upstream answering 429, then an unreachable upstream
- WHEN recording
- THEN the client gets 429 (nothing stored), then 502 JSON

### Requirement: Upstream credentials and headers

The upstream key MUST come only from env `NF_LLM_REPLAY_API_KEY` (no flag) and is required in record and
passthrough. Client `authorization`, `x-api-key`, `api-key`, `x-goog-api-key` and `cookie` MUST NOT be
forwarded; the key MUST be injected as `x-api-key` if the client sent `x-api-key`, `api-key` if it
sent `api-key`, else `authorization: Bearer <key>`. Hop-by-hop headers (`connection`, `upgrade`,
`http2-settings`, `keep-alive`, `te`, `trailer`, `transfer-encoding`, `proxy-*`), `host`,
`content-length` and `accept-encoding` MUST NOT be forwarded; all others (e.g. `anthropic-version`,
`anthropic-beta`) MUST be. The key MUST never be logged or printed (`Debug` shows `***`).

#### Scenario: Injection (W-auth, U-auth, U-hdr, W-fwd)
- GIVEN env key `sk-test-SECRET-123`; a client sending `Bearer nxf-no-credential`, and one sending `x-api-key: dummy` + `anthropic-version` to `/v1/messages?beta=true`
- WHEN recorded
- THEN upstream sees `Bearer sk-test-SECRET-123` resp. `x-api-key: sk-test-SECRET-123`, the query and `anthropic-version`, never the client values or h2c headers

#### Scenario: Secret never printed (X-nolog, U-dbg)
- GIVEN record mode with `RUST_LOG=trace`
- WHEN requests run
- THEN stdout/stderr and `{:?}` of the config never contain the key

### Requirement: Passthrough mode

Passthrough MUST forward like record (same header and credential rules), stream the upstream body to the
client as it arrives, and MUST NOT read or write the cassette. Non-JSON bodies MUST be forwarded unchanged.

#### Scenario: No cassette I/O (P-pass, E-live)
- GIVEN a cassette path that does not exist and an SSE upstream that pauses after event 1
- WHEN proxied
- THEN event 1 reaches the client before the pause ends and no file is created

### Requirement: Streaming replay

For `text/event-stream` entries, record and replay MUST deliver the same bytes as the upstream sent;
replay MUST NOT add delays.

#### Scenario: Byte-exact SSE (E-resp, E-msg, E-chat)
- GIVEN recorded streams for `/v1/responses`, `/v1/messages?beta=true` and chat completions `stream:true`
- WHEN replayed offline
- THEN each body equals the upstream bytes exactly with `content-type: text/event-stream`

### Requirement: Configuration and process behaviour

Flags (env): `--mode` (`NF_LLM_REPLAY_MODE`, `record|replay|passthrough`, default `replay`),
`--cassette` (`NF_LLM_REPLAY_CASSETTE`, required unless passthrough), `--upstream`
(`NF_LLM_REPLAY_UPSTREAM`, http(s) URL with no query, fragment or userinfo, required for record/passthrough; T `config::tests::upstream_must_be_an_http_url`), `--port`
(`NF_LLM_REPLAY_PORT`, default 8080, 0 = ephemeral), `--bind` (`NF_LLM_REPLAY_BIND`, default
`127.0.0.1`), `--volatile-field` (`NF_LLM_REPLAY_VOLATILE_FIELDS`, comma-separated, repeatable),
`--volatile-pattern` (`NF_LLM_REPLAY_VOLATILE_PATTERNS`, newline-separated, repeatable). Usage errors
exit 2; startup errors (missing/corrupt cassette, missing key, bad pattern) exit 1 with one clear
line on stderr. After binding, the process MUST print `listening on <ip>:<port>` to stdout. Logs go to
stderr (`RUST_LOG`, default `info`), one line per request (mode, method, path, key prefix, outcome,
status, ms), never headers or bodies. SIGINT/SIGTERM MUST shut down gracefully.

#### Scenario: CLI errors (X-help, X-mode, X-env, X-nocas, X-corrupt, X-nokey, X-noup, X-vol)
- GIVEN each invalid invocation
- WHEN run
- THEN exit code and message are as specified; `--help` lists every flag with its env var

#### Scenario: Ephemeral port (X-listen)
- GIVEN `--port 0` and a cassette
- WHEN started
- THEN stdout shows `listening on 127.0.0.1:<p>` and a request to `<p>` is served
