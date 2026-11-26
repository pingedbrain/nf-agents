## Exploration: nf-llm-replay (record / replay / passthrough LLM proxy)

Date: 2026-11-26. Inputs: the original plan; `docs/verification.md` section 4;
the 8 real requests captured in the lab (Nextflow 26.10.7);
`plugins/nf-agent/.../ChatModelFactory.groovy` at `v26.10.7`; three throwaway experiments run
for this change (below).

### Current State

- Workspace has one stub crate (`crates/nf-lineage`, no deps). `members = ["crates/*"]`, so a new
  `crates/nf-llm-replay` is picked up automatically. Workspace lints deny `unwrap/expect/panic/
  indexing_slicing/todo/unimplemented` outside tests, `unsafe_code = forbid`, `missing_docs = warn`
  (fatal in CI via `-D warnings`). Toolchain pinned `1.100.0`. `rust-version = "1.85"`.
- CI (`.github/workflows/ci.yml`) has `lint` (fmt, clippy, cargo-deny, cargo-machete) and `test`.
  No `docker` job yet. `deny.toml` license allowlist: Apache-2.0, MIT, BSD-2/3, ISC, Unicode-3.0,
  Zlib, Apache-2.0 WITH LLVM-exception. No `[workspace.dependencies]` yet.
- No main spec for the replay proxy exists (`openspec/specs/` has ci-pipeline, contributor-guide,
  rust-workspace). The ci-pipeline main spec will get a delta (new `docker` job).

### What the real clients send (from the captures)

| Runner | Request | Volatile parts | Credentials | Notes |
|---|---|---|---|---|
| `langchain4j` (default) | `POST /v1/chat/completions`, `stream:false`, 3 shapes: plain, `response_format.json_schema{name:"Output",strict:true}`, `tools:[{type:function}]` | none in body | `Authorization: Bearer nxf-no-credential` (24 chars) | Java HttpClient sends `Connection: Upgrade, HTTP2-Settings`, `Upgrade: h2c`, `HTTP2-Settings`; no `Accept-Encoding` |
| `pi` + OpenAI | `POST /v1/responses`, `stream:true` | `prompt_cache_key` (UUID), `<cwd>\n/abs/…/work/xx/hash\n</cwd>` at the end of `input[0].content` (role `developer`); headers `session_id`, `x-client-request-id` | `authorization: Bearer <key>` | sends `accept-encoding: gzip, deflate`; structured output = `final_answer` function tool; a retry turn appends a user message (different body, same session) |
| `pi` + Anthropic | `POST /v1/messages?beta=true`, `stream:true` | `<cwd>` block inside `system[0].text`; `cache_control` blocks are stable | `x-api-key` | needs `anthropic-version`, `anthropic-beta` forwarded upstream |

- No client called `GET /v1/models` (nor any GET). A generic catch-all router handles it anyway.
- Requests from different agent tasks arrive concurrently (3 tasks submitted at once in the log).
- Tool-call round trips: the second request carries the assistant `tool_calls` (ids minted by the
  model) and a `role:"tool"` message with `tool_call_id`. Because replay returns the recorded
  ids, the follow-up request is byte-identical to the recorded one, so key matching is stable
  across the whole multi-turn chain. No id rewriting needed.
- `ChatModelFactory` (26.10.7): if `OPENAI_API_KEY` is set and `baseUrl` is an unrecognized host,
  the key is withheld and the run aborts unless `agent.apiKey`/`NXF_AGENT_API_KEY`/`apiProvider`
  is set. The example must run Nextflow with no provider key in its environment.

### Experiments run for this change (lab, outside the repo)

1. `llm-replay-probe`, a probe crate in a local scratch directory (axum 0.8.9, reqwest 0.13.5, tokio 1.54):
   - raw TCP HTTP/1.1 request with `Connection: Upgrade, HTTP2-Settings` + `Upgrade: h2c` +
     `Bearer nxf-no-credential` gets a plain `HTTP/1.1 599` answer (hyper ignores the h2c upgrade);
   - status 599 is constructible (`StatusCode::from_u16(599)`) and reqwest reads it;
   - an SSE body streamed as `Body::from_stream` of raw event chunks (mixed `\n\n` and `\r\n\r\n`
     terminators) arrives byte-exact through reqwest.
2. Same probe built `FROM rust:1.100.0-alpine` (+ `musl-dev` only) into
   `gcr.io/distroless/static-debian12:nonroot`: reqwest's default rustls/aws-lc-rs provider
   compiles on musl without cmake; final image **10.5 MB**, runs as `nonroot:nonroot`.
   `cargo deny check licenses` with the repo `deny.toml` rejects only `webpki-root-certs`
   (`CDLA-Permissive-2.0`, a target-specific dep of `rustls-platform-verifier`).
3. `replay-e2e-probe` (a local scratch directory): `nextflow plugin install nf-agent@0.2.0` into a host dir,
   then `nextflow run` with `--network none`, `NXF_OFFLINE=true`, `NXF_PLUGINS_DIR=/plugins`
   (read-only mount): the plugin loads from the local dir with no registry call; the run only
   fails at name resolution of the agent endpoint (expected). So a fully offline e2e is feasible:
   prefetch plugins (with network), then run Nextflow + the replay container on an
   `--internal` Docker network. (Setting `NXF_HOME` instead re-downloads the framework, so the
   plugin dir must be passed with `NXF_PLUGINS_DIR`.)
4. Same dir, `--internal` Docker network, the probe image (answers every request with
   `599 {"error":{"message":"miss"}}`) as alias `replay`, Nextflow 26.10.7 with
   `agent.baseUrl = 'http://replay:8080/v1'`: langchain4j maps 599 to
   `InternalServerException`, **retries twice** (3 requests per miss), then the task fails and
   Nextflow prints the **raw response body** in its error block. So the 599 JSON body is what the
   user sees: it must lead with a human message that names the request and the record command.
   Whole offline run took ~7 s once images are local.

### Affected Areas

- `crates/nf-llm-replay/` — new binary crate (lib + bin, so integration tests run the server in-process).
- `Cargo.toml` — `[workspace.dependencies]` for shared versions (axum, tokio, reqwest, …).
- `Cargo.lock` — new deps.
- `deny.toml` — add `CDLA-Permissive-2.0` (only `webpki-root-certs`) to the license allowlist;
  aws-lc-sys/ring licenses are already covered (all-permissive AND-expressions).
- `.github/workflows/ci.yml` — new `docker` job (build image + offline e2e).
- `docs/examples/nf-llm-replay/` — example pipeline, config, committed cassette, e2e script, README.
- `crates/nf-llm-replay/Dockerfile`, `.dockerignore` (repo root).

### Approaches

1. **Per-endpoint typed handlers** (serde structs for chat completions, responses, messages).
   - Pros: can validate bodies, extract summaries precisely.
   - Cons: big surface, breaks on every preview-API change, violates "unknown fields tolerated";
     3x the code; chat-completions SSE and `/v1/models` would each need code.
   - Effort: High.
2. **Generic catch-all proxy keyed on (method, path, canonical JSON body)**; stream detection by
   upstream `content-type: text/event-stream`; endpoint knowledge limited to the miss summary
   (`model`, `messages`/`input`/`system`) and the default upstream hint.
   - Pros: one code path for every endpoint (chat completions JSON/SSE, responses, messages,
     models); tolerant of unknown fields; smallest code.
   - Cons: cannot reject nonsense paths (acceptable: upstream is fixed by flag, not an open relay).
   - Effort: Low/Medium.
3. **Reuse an existing VCR tool** (e.g. a Python/Go recorder) inside the image.
   - Pros: no code.
   - Cons: the original plan requires a Rust crate published to crates.io; no `<cwd>` normalization; secrets
     policy not ours.
   - Effort: Low, but out of scope.

Duplicates (identical normalized request seen more than once):

| Option | Parallel identical calls | Record run == replay run | Cost |
|---|---|---|---|
| a. list per key, replay in order | which task gets which answer depends on scheduling → outputs can swap between tasks | yes | per-key cursor |
| b. one entry per key; record forwards every request, last write wins | deterministic in replay, but the record run returned different answers than replay will | no | trivial |
| **c. one entry per key, record = "fill missing" with single-flight per key** | identical concurrent calls coalesce into one upstream call and all get the same answer | **yes** | `OnceCell` per key |

### Recommendation

Approach 2 with duplicate policy (c): replay and record both serve exactly one response per key;
record mode forwards only keys not already in the cassette (also avoids paying twice and makes the
599 hint "run once in record mode" add only what is missing). Stream responses stored as a list of
raw SSE event strings (each including its terminator) so concatenation is byte-exact by construction.
Client credentials are never forwarded; the upstream key comes only from `NF_LLM_REPLAY_API_KEY`
and is injected in the same header style the client used. Canonical JSON written by our own
sorted writer (cannot rely on `serde_json` map order: `preserve_order` may be unified in by other
workspace crates). Volatile defaults: top-level `user`, `metadata`, `prompt_cache_key`, plus the
`<cwd>…</cwd>` text rule applied to every JSON string value.

Ship in five PRs to stay ≤ ~800 hand-written lines each:
A normalization + cassette model (library only), B replay server + CLI, C record + passthrough,
D SSE + normalization flags, E Docker image + offline example/e2e + CI `docker` job.

5. Lab run of the example pipeline (3 agents incl. a process tool) against a deterministic fake
   chat-completions upstream (`replay-e2e-probe/fake.py`, offline plugin dir, host network): exit 0,
   outputs `FASTQ is a text format.`, `{"result":"HELLO"}`, `[answer:no, confidence:0.9]`. The tool
   round trip's 2nd request carries `assistant.tool_calls[id=call_1]` + `tool` message with
   `tool_call_id: call_1` and content `{"result":"HELLO"}`. Note: `-with-agent-trace` appends
   "Before each tool call, briefly state your reasoning…" to the system prompt, so Nextflow CLI
   flags change keys: record and replay must use the same flags.

### Risks

- Preview `agent` primitive: request shapes change between Nextflow releases → cassettes go stale
  (mitigation: 599 names the exact missing request and the record command; re-capture each stable).
- Headers that change model output (`anthropic-beta`, `openai-beta`) are not part of the key
  (the original plan mandates headers excluded). Documented limitation.
- `deny.toml` needs `CDLA-Permissive-2.0` (root-certificate data license, permissive).
- Plugin registry availability in CI for the e2e prefetch step (network, no secrets).
- `serde_json/preserve_order` feature unification would silently change key bytes if the
  canonical writer relied on `Map` iteration order.

### Ready for Proposal

Yes. Scope, endpoint priorities and the PR split are clear; no open question is blocking.
