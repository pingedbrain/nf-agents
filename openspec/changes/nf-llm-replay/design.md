# Design: nf-llm-replay

## Technical Approach

One axum `fallback` handler serves every method/path. A `Normalizer` turns (method, path, body) into a
canonical string + SHA-256 key (spec llm-replay-cassette). A `Backend` enum decides per mode:
`Replay` (immutable `Cassette`, no HTTP client exists), `Record` (`Recorder` with per-key
single-flight + `Upstream`), `Passthrough` (`Upstream` only). Streaming is detected from the upstream
`content-type`, never from the request. Lib + bin in one crate so integration tests run the router
in-process on an ephemeral port against an in-process fake upstream. Evidence for every choice:
`exploration.md` (captures + 5 lab experiments).

## Architecture Decisions

| Topic | Options | Tradeoff | Decision |
|---|---|---|---|
| Routing | typed per-endpoint handlers / generic catch-all | typed = 3x code, brittle vs preview APIs | **catch-all**; endpoint knowledge only in miss summary + upstream hint |
| Duplicates | list per key in order / last-write-wins / one entry + fill + single-flight | list: parallel identical tasks may swap answers; LWW: record run ≠ replay run | **one entry per key; record answers known keys from cassette; `tokio::sync::OnceCell` per key coalesces in-flight duplicates** |
| Record of existing cassette | truncate / fill missing | truncate re-bills everything | **fill missing** (delete file to re-record); matches the 599 hint |
| Non-2xx in record | store / don't | stored 429/500 make replay fail forever | **return, don't store** |
| SSE storage | raw body string / parsed `data` JSON / raw event strings incl. terminator | parsed loses bytes; raw body unreadable | **raw event strings incl. terminator** → concat is byte-exact by construction |
| JSON response storage | parsed JSON / raw string | parsed is prettier but not byte-exact | **raw string** (`body`) |
| Request in cassette | none / raw / normalized canonical | raw leaks `<cwd>` host paths | **normalized canonical body as `RawValue`** (reviewable, path-free, explains misses) |
| Canonical JSON | `serde_json::to_string(Value)` / own sorted writer | `preserve_order` may be unified in by other workspace crates and silently change keys | **own writer, keys sorted explicitly** |
| Query string | keyed / dropped | `beta=true` must not matter; Gemini puts `key=` in query | **not keyed, not stored**; forwarded upstream only |
| Upstream auth | pass client credential / inject from env | client holds placeholder `nxf-no-credential`; passing through puts real keys in Nextflow config | **inject `NF_LLM_REPLAY_API_KEY`, strip client credentials**; header style mirrors client (`x-api-key`/`api-key`/Bearer) |
| Redirects | follow / none | following could resend injected key elsewhere | **`redirect::Policy::none()`** |
| Compression | accept gzip / identity | stored bytes must be plain | **strip `accept-encoding`**, reqwest without gzip features |
| Ordering + concurrency | append under lock / rewrite sorted via temp+rename | append order = completion order (noisy diffs) | **BTreeMap, rewrite temp + `sync_all` + rename under `tokio::sync::Mutex`** (cassettes are small) |
| Secret guard | test only / test + runtime check | runtime check is 1 line | **both**: refuse line containing the key |
| TLS | reqwest rustls (aws-lc-rs) / ring via `rustls-no-provider` | aws-lc builds on alpine musl with only `musl-dev` (lab) | **reqwest default `rustls`**; `deny.toml` += `CDLA-Permissive-2.0` |
| Image | distroless/cc glibc / static musl on distroless/static | static = 10.5 MB in lab | **`rust:1.100.0-alpine` → `distroless/static-debian12:nonroot`** |
| e2e placement | separate workflow / `docker` job | job already has the image | **inside `docker` job**, PR-blocking; plugin prefetch is the only networked step |
| Example cassette source | real provider / deterministic fake upstream | agents have no key; fake is reproducible | **fake upstream** (`examples/fake_upstream.rs`); README shows real re-recording |
| Errors | anyhow everywhere / thiserror lib + anyhow bin | project rule | **thiserror in lib, anyhow in `main.rs`** |

**Deviation (2026-11-26, PR A).** (1) Cargo 1.100 warns `unused workspace dependency` for every
`[workspace.dependencies]` entry no crate uses, so PR A lists only `anyhow, regex, serde, serde_json,
sha2, tempfile, thiserror`; later PRs add the rest of the list below with the same versions/features.
(2) Until D.5 the constructor is `Normalizer::new() -> Result<Self, NormalizeError>` (defaults only,
`NormalizeError::{Body, Pattern}`); D.5 changes it to the `new(extra_fields, extra_patterns)` signature
below and adds `VolatileField` + `NormalizeError::Field`. Writing those now would be untested code
(N-ptr/N-pat are PR D). (3) `CassetteError::Secret` arrives with `Recorder` in PR C.

## Data Flow

    client ─▶ axum fallback(handle) ─▶ Normalizer::key(method, path, body)
                 │ 400 on non-JSON (record/replay)
       ┌─────────┼──────────────────────────────┐
     Replay    Record                       Passthrough
   Cassette   Recorder.cell(key)            Upstream::send ─▶ stream body back
   hit→entry  ├ known → entry               (no cassette I/O, raw body)
   miss→599   └ OnceCell::get_or_try_init:
                Upstream::send(original bytes, filtered headers + key)
                2xx → Entry{events|body} → guard → BTreeMap → temp+rename → entry
                else → Err(response) → returned unchanged
     entry ─▶ respond(status, content_type, body | stream(events))

## File Changes

| File | Action | PR |
|---|---|---|
| `Cargo.toml` | add `[workspace.dependencies]` | A |
| `deny.toml` | `CDLA-Permissive-2.0` + comment | C (reqwest lands) |
| `crates/nf-llm-replay/Cargo.toml` | new, `0.1.0`, `publish = false`, `[lints] workspace = true` | A |
| `src/lib.rs` | crate docs, `app`, `serve` | A/B |
| `src/normalize.rs` | `Normalizer`, canonical writer, key | A (+ flags D) |
| `src/cassette.rs` | `Entry`, `Cassette::load/get`; `Recorder` | A / C |
| `src/config.rs` | `Cli`, `Mode`, `ApiKey`, `Config` | B |
| `src/proxy.rs` | `AppState`, `Backend`, `handle`, `ProxyError`, miss summary | B / C |
| `src/upstream.rs` | `Upstream`, header filter, key injection | C |
| `src/sse.rs` | `split_events` | D |
| `src/main.rs` | parse, tracing, bind, signals | B |
| `tests/common/mod.rs` | `start(config)`, cassette helpers | B |
| `tests/common/fake_upstream.rs` | deterministic fake (chat JSON → C; SSE → D) | C/D |
| `tests/fixtures/captures/*.json` | 8 lab requests, host paths scrubbed (generated, not counted) | A |
| `tests/{captures,replay,cli,record,secrets,sse}.rs` | integration | A-D |
| `examples/fake_upstream.rs` | `#[path]` reuse of the fake, `--port` | E |
| `crates/nf-llm-replay/Dockerfile`, `.dockerignore` | image | E |
| `docs/examples/nf-llm-replay/{main.nf,nextflow.config,cassettes/example.jsonl,e2e.sh,record.sh,README.md}` | example | E |
| `.github/workflows/ci.yml` | `docker` job | E |

Every `tests/*.rs`, `examples/*.rs` starts with a `//!` line (`missing_docs` is fatal under `-D warnings`).

## Interfaces / Contracts

Dependencies (`[workspace.dependencies]`, versions verified in the lab):
`axum = "0.8.9"`, `tokio = { "1.54", features = [rt-multi-thread, macros, net, signal, sync, fs] }`,
`reqwest = { "0.13.5", default-features = false, features = [rustls, http2, stream] }`,
`clap = { "4.6", features = [derive, env] }`, `serde = { "1", features = [derive] }`,
`serde_json = { "1", features = [raw_value] }`, `sha2 = "0.11.2"` (Output has no `LowerHex`: hex via
`{b:02x}` loop), `regex = "1.13"`, `thiserror = "2"`, `anyhow = "1"`, `tracing = "0.1.44"`,
`tracing-subscriber = { "0.3.23", default-features = false, features = [fmt, env-filter] }`,
`futures-util = "0.3"`, `bytes = "1"`. Dev: `assert_cmd = "2.2.2"`, `tempfile = "3.27"`.

```rust
// normalize.rs
pub struct Normalizer { fields: Vec<VolatileField>, patterns: Vec<regex::Regex> }
enum VolatileField { TopLevel(String), Pointer(String) }
#[derive(Debug, thiserror::Error)]
pub enum NormalizeError {
    #[error("request body is not valid JSON: {0}")] Body(serde_json::Error),
    #[error("invalid volatile pattern `{pattern}`: {source}")] Pattern { pattern: String, source: regex::Error },
    #[error("invalid volatile field `{0}` (use a name or a JSON pointer starting with `/`)")] Field(String),
}
pub struct Normalized { pub key: String, pub canonical: String /* "" when empty */ }
impl Normalizer {
    pub fn new(extra_fields: &[String], extra_patterns: &[String]) -> Result<Self, NormalizeError>;
    pub fn normalize(&self, method: &str, path: &str, body: &[u8]) -> Result<Normalized, NormalizeError>;
}
fn write_canonical(v: &serde_json::Value, out: &mut String); // sorts keys itself

// cassette.rs
#[derive(Serialize, Deserialize)] pub struct Entry { pub v: u32, pub key: String, pub request: RecordedRequest, pub response: RecordedResponse }
#[derive(Serialize, Deserialize)] pub struct RecordedRequest { pub method: String, pub path: String, pub body: Option<Box<RawValue>> }
#[derive(Serialize, Deserialize)] pub struct RecordedResponse {
    pub status: u16, pub content_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub body: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub events: Option<Vec<String>>,
}
#[derive(Debug, thiserror::Error)] pub enum CassetteError { Io{path,source}, Line{line: usize, reason: String}, Duplicate{key, first: usize, second: usize}, Secret }
pub struct Cassette { entries: BTreeMap<String, Arc<Entry>> }
impl Cassette { pub fn load(path: &Path) -> Result<Self, CassetteError>; pub fn get(&self, key: &str) -> Option<Arc<Entry>>; }
pub struct Recorder { path: PathBuf, secret: ApiKey, saved: tokio::sync::Mutex<BTreeMap<String, Arc<Entry>>>,
                      cells: std::sync::Mutex<HashMap<String, Arc<tokio::sync::OnceCell<Arc<Entry>>>>> }
impl Recorder { pub fn open(path: PathBuf, secret: ApiKey) -> Result<Self, CassetteError>; // creates if absent
                pub fn cell(&self, key: &str) -> Arc<OnceCell<Arc<Entry>>>; // poisoned lock → into_inner, never panic
                pub async fn persist(&self, entry: Arc<Entry>) -> Result<(), CassetteError>; }

// config.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)] pub enum Mode { Record, Replay, Passthrough }
#[derive(Clone)] pub struct ApiKey(String); // Debug/Display = "***"; fn expose(&self) -> &str
#[derive(Debug, clap::Parser)] pub struct Cli { /* flags + env per spec; no api-key flag */ }
#[derive(Debug)] pub struct Config { pub mode: Mode, pub cassette: Option<PathBuf>, pub upstream: Option<reqwest::Url>,
    pub bind: IpAddr, pub port: u16, pub volatile_fields: Vec<String>, pub volatile_patterns: Vec<String>, pub api_key: Option<ApiKey> }
impl Config { pub fn from_cli(cli: Cli, api_key: Option<String>) -> Result<Self, ConfigError>; }

// upstream.rs
pub struct Upstream { client: reqwest::Client, base: reqwest::Url, key: ApiKey } // timeout 600 s, connect 10 s, no redirects
impl Upstream { pub async fn send(&self, method: &Method, path_and_query: &str, headers: &HeaderMap, body: Bytes) -> Result<reqwest::Response, ProxyError>; }
pub(crate) fn forward_headers(client: &HeaderMap, key: &ApiKey) -> HeaderMap;

// proxy.rs
pub(crate) enum Backend { Replay(Arc<Cassette>), Record(Arc<Recorder>, Arc<Upstream>), Passthrough(Arc<Upstream>) }
pub(crate) enum ProxyError { BadRequest(String) /*400*/, Miss(Box<MissBody>) /*599*/, Upstream(String) /*502*/, Cassette(String) /*500*/ }
impl IntoResponse for ProxyError { /* {"error":{"type":"nf_llm_replay_<kind>","message":..}} */ }

// lib.rs
pub fn app(config: &Config) -> Result<axum::Router, StartupError>; // DefaultBodyLimit 32 MiB
pub async fn serve(config: Config, listener: TcpListener, shutdown: impl Future<Output = ()> + Send + 'static) -> Result<(), StartupError>;
// sse.rs
pub fn split_events(body: &str) -> Vec<String>; // terminators \n\n, \r\n\r\n, \r\r; tail kept
```

Example cassette line (wrapped here; one line on disk):
`{"v":1,"key":"9f…","request":{"method":"POST","path":"/v1/chat/completions","body":{"messages":[…],"model":"gpt-5-mini","stream":false}},"response":{"status":200,"content_type":"application/json","body":"{\"id\":\"chatcmpl-fake\",…}"}}`

Fake upstream (tests + example; deterministic, `created: 0`, id `chatcmpl-fake`): chat JSON — tools and no
`tool` message → `tool_calls[{id:"call_1", name: tools[0], args {required[0]: text after last ": " of the
first line of the last user message}}]`; a `tool` message → content = last tool content; `json_schema` →
`{"answer":"no","confidence":0.9}`; else `FASTQ is a text format.`. `stream:true` and `/v1/responses`,
`/v1/messages` → fixed SSE events (PR D). Knobs: hit counter, last headers/URI, forced status, echo-key,
pause gate after event 1.

Docker: build stage `apk add --no-cache musl-dev`, copy `Cargo.toml Cargo.lock crates/`, `cargo build --release --locked -p nf-llm-replay`; final stage as spec. e2e network: `docker network create --internal nfllm-e2e-$$`.
Record mode in a container must mount the cassette's **directory** (rename over a bind-mounted file fails with `EBUSY`) and run with `--user $(id -u):$(id -g)`; README states this.

## Testing Strategy

| Layer | What | Approach |
|---|---|---|
| Unit | canonical JSON, volatile rules, key, cassette parse/validate, sorted atomic write, secret guard, SSE split, header filter, key injection, miss summary, `ApiKey` redaction | `mod tests` per module, `tempfile` dirs |
| Golden | key stability for 8 real captures | `tests/captures.rs`, committed expected hex |
| Integration | replay hit/miss/599/no-network/h2c raw TCP/concurrency/body limit; record round trips (json_schema, tools, multi-turn), single-flight, fill, non-2xx, 502, auth injection, passthrough; SSE byte-exact ×3 endpoints, incremental passthrough; secret scan | in-process `app()` + `axum::serve` on `127.0.0.1:0`; fake upstream counts connections (0 in replay) |
| CLI | flags/env, exit codes, `listening on`, key never printed | `assert_cmd` + spawned process for `--port 0` and log scan |
| E2E | Nextflow 26.10.7 offline against the image | `e2e.sh` in CI `docker` job (skips without Docker locally) |

Strict TDD: each task in tasks.md starts from the failing named test.

## Migration / Rollout

No migration. Cassette `v: 1` and the key recipe are a compatibility contract guarded by `capture_keys_are_stable`; changing either requires bumping `v`.

## Open Questions

- [ ] Should a GHA build cache (`docker/build-push-action` + `type=gha`) be added once the `docker` job exceeds ~5 min?
- [ ] Record the example cassette against real `gpt-5-mini` once a maintainer key is available (README documents it; not blocking).
- [ ] Nextflow CLI flags (`-with-agent-trace`) change the system prompt and thus keys: document only, or add a default volatile pattern for that sentence? (Documented only for now.)
