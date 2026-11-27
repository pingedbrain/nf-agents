# Verification of upstream behaviour

Date: 2026-11-26. Scope: the assumptions nf-agents relies on before writing code — Nextflow
data lineage (storage model and `nextflow lineage` CLI), supported Nextflow versions, the
`agent` primitive, rmcp, object_store, MiniStack and the Rust quality
tooling. Rule applied: where observed behaviour contradicts an initial assumption, the observed
behaviour wins.

Method: official docs plus upstream source pinned by tag/commit, and **empirical runs**:
a 3-process pipeline executed twice (two values of one param) with lineage enabled on four
Nextflow releases in the official `nextflow/nextflow:<tag>` Docker images, every
`nextflow lineage` subcommand captured, the `agent` primitive pointed at a dummy HTTP
endpoint to capture its requests, and small Rust probe crates compiled and tested on the
local toolchain (rustc 1.94.1). The experiments were run locally; the pipeline and generator
scripts are kept under `scripts/fixtures/`.

Pinned upstream references used below:

| Project | Ref | Commit |
| --- | --- | --- |
| nextflow-io/nextflow | `v26.10.7` (latest stable, 2026-11-20) | `0ad848ef3c3a22824cd9eb2ca14568385867702d` |
| nextflow-io/nextflow | `v26.04.9` | `95212d89153486fe0ae212ee82f0d966dff77181` |
| nextflow-io/nextflow | `v25.10.9` | `64d39b096478552befd34a8f6d70ead4709e4518` |
| nextflow-io/nextflow | `v25.04.8` | `c08bd01ab6b17c30c526b03c4923e23ef0db4e10` |
| modelcontextprotocol/rust-sdk | `rmcp-v3.12.0` (2026-11-26) | `e81f92432dd8411ee5f021707e998c4a08e68b12` |
| object_store (crates.io) | `0.14.2` (2026-09-15) | — |
| github/spec-kit | `v1.4.5` (2026-11-25) | — |
| ministackorg/ministack | `v1.5.45` (2026-11-26) | — |

## Summary

| # | Initial assumption | Finding | Design impact |
| --- | --- | --- | --- |
| 1 | `lineage.enabled`, `lineage.store.location`, `.data.json`, `lineage/v1beta1`, types WorkflowRun/TaskRun/TaskOutput/WorkflowOutput/FileOutput | All confirmed. Plus: a 6th type **`AgentRun`** (26.10+), an on-disk **envelope change** (flat in 25.04, `{version, kind, spec}` from 25.10), `#output` sibling directories, a `.history/` folder, and checksums that are **metadata hashes (path + size + mtime), not content hashes** | **Yes** — parser must accept both envelopes; `check` must reimplement Nextflow's murmur3 metadata hash; fixtures must preserve mtimes |
| 2 | Subcommands `list, view, find, diff, check, render` | Confirmed, exactly those six. Output of `find` changed format (JSON array in 25.04, one LID per line from 25.10). Several errors exit `0` with the message on stdout | **Yes** — contract tests must normalize per version and compare on stdout text, not only exit codes |
| 3 | Minimum 25.04, define CI matrix | 25.04 is indeed the first release with lineage. Latest stable is **26.10.7**. Four format generations exist across 25.04 / 25.10 / 26.04 / 26.10 | Matrix of 4 (fixtures are generated offline, so it costs nothing per PR) |
| 4 | `agent` primitive in preview; maybe accepts `baseUrl`; maybe chat completions / responses / streaming | Exists since **26.10** (first in 26.08.0-edge). Accepts `agent.baseUrl`. Default runner `langchain4j` calls **`POST {baseUrl}/chat/completions` non-streaming**. The `pi` runner calls **`POST /v1/responses` with SSE** (OpenAI) or **`POST /v1/messages?beta=true` with SSE** (Anthropic) | **Yes** — nf-llm-replay priority is non-streaming chat completions; SSE is needed for the Responses and Messages APIs, not for chat completions |
| 5 | rmcp: latest stable, features, tool API | `rmcp 3.12.0`; `#[tool_router]` / `#[tool]` / `#[tool_handler]`, `Parameters<T>` with schemars 1; in-process tests over `tokio::io::duplex` work | Minor — pin 3.x, disable default features (chrono advisory) |
| 6 | object_store covers local, S3, GCS, Azure | `object_store 0.14.2` with features `aws`, `gcp`, `azure` (local `fs` is default, in-memory always on). Verified local + in-memory + S3 on MiniStack. **`Path::from` percent-encodes `#`**, which breaks `<hash>#output` keys unless `Path::parse` is used | **Yes** — key handling rule; `#[async_trait]` stays (native `async fn` is not dyn-compatible) |
| 7 | Spec Kit: version, init, command names | Spec Kit `v1.4.5`: `uv tool install specify-cli`, `specify init --integration <agent>`, `/speckit.*` commands. | **Yes** — drop `specs/`, use `openspec/specs/` + `openspec/changes/` |
| 8 | MiniStack for S3; insta, assert_cmd, cargo-deny, cargo-machete on edition 2024 | MiniStack `ministackorg/ministack:1.5.45`, port 4566, S3 works with object_store. `insta 1.49.0` and `assert_cmd 2.2.2` OK. `cargo-deny 0.22.1` needs rustc 1.95, `cargo-machete 0.10.1` does not build on 1.94.1 | CI must use current stable Rust (1.100.0) or pin `cargo-deny 0.21.0` / `cargo-machete 0.9.2` locally |

---

## 1. Nextflow data lineage (storage model)

### 1.1 Initial assumption

`lineage.enabled = true`, a default store location configurable with `lineage.store.location`,
some folder structure, files named `.data.json`, a version field `lineage/v1beta1`, and record
types `WorkflowRun`, `TaskRun`, `TaskOutput`, `WorkflowOutput`, `FileOutput`.

### 1.2 Finding

Sources: [Data lineage tutorial](https://docs.seqera.io/nextflow/tutorials/data-lineage)
(the older `nextflow.io/docs/latest/data-lineage.html` URL now redirects there),
[config reference](https://docs.seqera.io/nextflow/reference/config/lineage),
`modules/nf-lineage/src/main/nextflow/lineage/` at `v26.10.7` (`DefaultLinStore.groovy`,
`serde/LinTypeAdapterFactory.groovy`, `model/v1beta1/*.groovy`, `DefaultLinHistoryLog.groovy`,
`LinHistoryRecord.groovy`, `LinObserver.groovy`), and lineage stores generated locally.

**Configuration** (confirmed): `lineage.enabled` (default `false`) and
`lineage.store.location` (default `./.lineage`, relative to the launch directory). The location
goes through `FileHelper.toCanonicalPath`, so any Nextflow-supported filesystem URI
(`s3://…`, etc.) is accepted. The `nextflow lineage` CLI reads `./nextflow.config` too: if
lineage is not enabled there it prints `Error lineage store not loaded - Check Nextflow
configuration` (exit code 0).

**On-disk layout** (verified on all four versions from the generated stores):

```
.lineage/
├── .history/<runHash>                       # one TSV line per run (see 2.2 `list`)
├── <runHash>/.data.json                     # WorkflowRun
├── <runHash>#output/.data.json              # WorkflowOutput (only with an `output` block)
├── <runHash>/<path rel. to outputDir>/.data.json   # FileOutput of a published file
├── <taskHash>/.data.json                    # TaskRun (or AgentRun)
├── <taskHash>#output/.data.json             # TaskOutput
└── <taskHash>/<path rel. to task workdir>/.data.json  # FileOutput of a task output
```

- `<runHash>` and `<taskHash>` are 32 lowercase hex chars. `<taskHash>` is the task hash, i.e.
  the work directory `work/81/cc82b1…` maps to `lid://81cc82b1…`.
- The metadata file is always `.data.json` (the store scans for names starting with `.data.json`).
- `#output` is a **literal `#` in a directory name**, not a URL fragment on disk.
- Files are compact single-line JSON (Gson, nulls serialized, `<`/`>` escaped as `<`/`>`).
- A run without an `output` block produces no `WorkflowOutput` and no published `FileOutput`.
- `.history/<runHash>` contains `TIMESTAMP\tRUN NAME\tSESSION ID\tLINEAGE ID`, timestamp format
  `yyyy-MM-dd HH:mm:ss z` (e.g. `2026-11-26 20:29:36 UTC`).

**LIDs**: `lid://<hash>` for runs/tasks; `lid://<hash>/<relative/path>` for files;
`lid://<hash>#<field>[.<field>…]` navigates into a record (`#output`, `#params`,
`#workflow.scriptFiles`; field names are validated against the model, see `LinPropertyValidator`).
`lid://<hash>#output` on a run/task returns the `output` list of the sibling `#output` record.

**Envelope and version** — this differs from the initial assumption of a single format:

| Nextflow | On-disk envelope | Introduced by |
| --- | --- | --- |
| 25.04.x | flat: `{"version":"lineage/v1beta1","kind":"TaskRun", <fields…>}` | #5715, #6037 |
| ≥ 25.10 | nested: `{"version":"lineage/v1beta1","kind":"TaskRun","spec":{<fields…>}}` | #6075 (commit `632861350e`, 2025-07-09) |

The version string is `lineage/v1beta1` in every release (constant `LinModel.VERSION`).
Nextflow's own reader rejects any other version (`Invalid or missing 'version' JSON property`)
and accepts both envelopes (if `spec` is absent it reads the fields from the root).

**Record types and fields** (exact field names, extracted mechanically from the generated
stores of each version):

| kind | fields (`spec`) | version notes |
| --- | --- | --- |
| `WorkflowRun` | `workflow{scriptFiles[DataPath], repository, commitId}`, `sessionId`, `name`, `params[Parameter]`, `config{…}`, `metadata{…}` | `metadata` from 26.04 (#6545, platform info) |
| `WorkflowOutput` | `createdAt`, `workflowRun`, `output[Parameter]` | all |
| `TaskRun` | `sessionId`, `name`, `codeChecksum{Checksum}`, `script`, `eval{map}`, `input[Parameter]`, `container`, `conda`, `spack`, `architecture`, `globalVars{}`, `binEntries[DataPath]`, `workflowRun`, `moduleId` | `moduleId` from 26.04 (#7160); `eval` only in 26.10 (#7211) |
| `TaskOutput` | `taskRun`, `workflowRun`, `createdAt`, `output[Parameter]`, `labels` | all |
| `FileOutput` | `path`, `checksum{Checksum}`, `source`, `workflowRun`, `taskRun`, `size`, `createdAt`, `modifiedAt`, `labels` | all |
| `AgentRun` | `sessionId`, `name`, `codeChecksum`, `runner`, `model`, `resolvedModel`, `instruction`, `goal`, `promptTemplate`, `maxIterations`, `outputSchema` (JSON schema as a string), `tools[]`, `skills[]`, `input[Parameter]`, `container`, `workflowRun`, `moduleId` | **new in 26.10** (#7477) |

Helper types: `Checksum{value, algorithm, mode}`, `DataPath{path, checksum}`,
`Parameter{type, name, value}`. (`Workflow` is registered as a kind in the serializer but is
only ever embedded inside `WorkflowRun`.)

Value conventions observed:

- References between records are strings `lid://…` (`workflowRun`, `taskRun`, `source`).
- `WorkflowRun.workflow.scriptFiles[].path` and `DataPath.path` are **`file://` URIs**;
  `FileOutput.path` is a **plain absolute path** (no scheme for local files).
- `Parameter.type` is a Java simple type name for workflow params (`String`, `Path`) and for
  typed outputs, and the qualifier for legacy process I/O (`path`, `val`, `env`, `eval`,
  `stdout`). `Parameter.name` is `null` for legacy process outputs, even with `emit:`.
  `Parameter.value` is a literal, a `lid://` string (file produced upstream), or a list of
  `DataPath` (file that came from outside the lineage store, e.g. a pipeline input).
- Published `FileOutput`: `taskRun = null`, `source = lid://<taskHash>/<file>`.
  Task `FileOutput`: `source = taskRun = lid://<taskHash>`. This chain
  (published file → task file → TaskRun → inputs) is what `provenance` must walk.
- Timestamps are ISO-8601 with nanoseconds and `Z`.
- `WorkflowRun.config` and `metadata` contain machine-specific values (absolute paths,
  `homeDir`, `userName`, `commandLine`). Secrets are masked by Nextflow (`SecretHelper.hideSecrets`).
- Agent results are stored as a normal `TaskOutput` (e.g. `{"type":"String","name":"$out","value":"…"}`).

**Checksums — the most important surprise.** `Checksum.algorithm` is `"nextflow"` and `mode`
is `"standard"` in every record we generated. In standard mode Nextflow's `HashBuilder`
(`modules/nf-commons/src/main/nextflow/util/HashBuilder.java`, `hashFileMetadata`) hashes
**file metadata, not content**: Guava `murmur3_128` (seed 0) over
`putUnencodedChars(absolute path)` (UTF-16LE) + `putLong(size)` + `putLong(mtime in ms)`
(little-endian), printed as the 16 hash bytes in hex. We reproduced it bit-for-bit
(a standalone reimplementation matched every task file of a run). Consequences, confirmed empirically:

- `nextflow lineage check` fails after a mere `touch` / copy that changes the mtime even if the
  content is identical, and passes again when the mtime is restored.
- Published files are **symlinks** into `work/` (default publish mode for the `output` block),
  so `check` effectively validates the work file.
- Files committed to git lose their mtimes: fixture-based `check` tests need an mtime manifest.
- On `master` (after the 26.10 branch) commit `5fffa38844` (#8018, 2026-11-05) switches lineage
  task checksums to "the process cache mode", so future releases may emit other `mode` values
  (`lenient`, `deep`, `sha256`). Inputs inside the project directory are already hashed by
  content (sha256) in standard mode (`isAssetFile`).

### 1.3 Design impact

Yes:

1. `Record` deserialization must accept **both envelopes** (flat and `spec`) and must
   dispatch on `kind`, with `Unknown(serde_json::Value)` for new kinds. `AgentRun` is a
   first-class variant, not `Unknown`.
2. `version != "lineage/v1beta1"` → `UnsupportedVersion` (same behavior as Nextflow).
3. Keep every field optional/nullable (Gson writes `null`, older versions omit fields) and
   preserve unknown fields (`#[serde(flatten)] extra`).
4. Model two path flavours: `file://` URIs (`DataPath`, `scriptFiles`) and plain paths (`FileOutput.path`).
5. `check` must implement the `nextflow`/`standard` metadata hash (murmur3 x64_128) over the
   **recorded path string** plus the **current** size and mtime of the file it can reach, and
   return a clear "unsupported checksum mode/algorithm" error (a typed error) for anything
   else. Fixtures must ship an mtime manifest so `check` can be tested from a git checkout.
   Doing this for files on S3 needs one more verification (what string Nextflow hashes for an S3 path,
   and whether it uses S3 `LastModified`) — see [open items](#open-items).
6. Keys/paths in the store contain `#` (see section 6 for the object_store consequence).
7. `list_runs` can be built from `.history/*` (exactly what `nextflow lineage list` does),
   no need to scan all records.

---

## 2. `nextflow lineage` CLI (contract-test oracle)

### 2.1 Initial assumption

Subcommands `list`, `view`, `find`, `diff`, `check`, `render`, with flags and outputs to be used
as the oracle.

### 2.2 Finding

Sources: [CLI reference](https://docs.seqera.io/nextflow/reference/cli/lineage),
`modules/nf-lineage/src/main/nextflow/lineage/cli/LinCommandImpl.groovy` at each tag, and the
captured stdout, stderr and exit code of every subcommand on each version.

Exactly six subcommands, **no flags** besides `-h`. `nextflow lineage -h` (26.10.7):

```
Commands:
  check   Checks the integrity of an lineage file path
  diff    Show differences between two lineage descriptions
  find    Find lineage metadata descriptions matching with a query
  list    List the executions with lineage enabled
  render  Render the lineage graph for a workflow output
  view    Print the description of a Lineage ID (lid)
```

| Subcommand | Output (stdout unless noted) | Exit code |
| --- | --- | --- |
| `list` | Table, header `TIMESTAMP \tRUN NAME \tSESSION ID \tLINEAGE ID` (cells space-padded, tab-separated), one row per `.history` entry sorted by timestamp. Empty: `No workflow runs found in lineage history log` | 0 |
| `view <lid>` | The record as pretty JSON (2-space indent, same envelope as on disk: flat in 25.04, `spec` from 25.10). `view lid://h#output` prints only the `output` array; `#field` prints that sub-value | 0 |
| `view` (missing LID) | `Error loading <lid> - Lineage record <key> not found` on **stdout** | **0** |
| `view foo` (not a LID) | `ERROR ~ Identifier is not a lineage URL` (stdout up to 26.04, stderr in 26.10) | 1 |
| `find k=v [k=v…]` | **25.04: a pretty JSON array of LIDs. ≥ 25.10: one LID per line.** Order = filesystem walk (unspecified). `label=` is an alias of `labels=`; keys may be dotted paths; values are URL-decoded; repeated keys are allowed | 0 |
| `find foo=bar` | `Error searching for foo=bar. Property 'foo' doesn't exist in the lineage model.` | **0** |
| `diff <lid1> <lid2>` | Git-style unified diff (JGit Myers) of the two **pretty-printed records**: `diff --git <key1> <key2>` / `--- <key1>` / `+++ <key2>` / hunks (keys without `lid://`) | 0 |
| `check <file-lid>` | `Checksum validation succeed` | 0 |
| `check` mismatch | stderr: `Checksum of '<path>' does not match with lineage metadata` | 1 |
| `check` on a non-FileOutput | stderr: `Unexpected lineage object type: nextflow.lineage.model.v1beta1.TaskRun` | 1 |
| `render <lid> [file]` | Writes an HTML graph (default `./lineage.html`); prints `Rendered lineage graph for <lid> to <file>`. Accepts FileOutput, TaskRun, WorkflowRun, and AgentRun (26.10) | 0 |

Every subcommand behaved identically across 25.04.8, 25.10.9, 26.04.9 and 26.10.7 except the
`find` output format and the record envelope shown by `view`/`diff`. The CLI front-end moved
to `modules/nf-cli-v1` in 26.10 (#5971) without behavior changes.

`diff` between the two runs of the minimal pipeline is **not** "the
parameter and nothing else": it also shows `sessionId`, `name`, `config.runName`,
`config.outputDir`, `metadata.start`, `metadata.commandLine`, etc. `diff` between the two
`GENERATE` tasks shows `sessionId`, `script`, the `greeting` input and `workflowRun`.

### 2.3 Design impact

Yes:

- Contract tests compare **sets** of LIDs for `find` (order is unspecified) and must parse
  both `find` formats.
- `view` oracles are JSON: compare as parsed JSON, not text (envelope differs per version).
- Exit codes are not a reliable error signal for `view`/`find`; the oracle stores stdout,
  stderr and exit code separately.
- The MCP `diff` tool cannot be a textual record diff if the acceptance criterion "shows that
  parameter and nothing else" is kept: it must be a **semantic diff** (params, tool versions /
  containers, outputs) that ignores volatile fields (session id, run name, timestamps,
  command line, output dir). The criterion "shows that parameter and nothing else" applies to
  that semantic diff (and to `WorkflowRun.params`), not to `nextflow lineage diff`. Keep a raw
  mode for parity with `nextflow lineage diff`.
- `render` is out of scope for the oracle (HTML); not needed by the MCP tools.

---

## 3. Nextflow versions and CI matrix

### 3.1 Initial assumption

Lineage is compatible from 25.04; define a matrix with the latest stable.

### 3.2 Finding

- Lineage first shipped in **25.04.0** (`AddedInVersion 25.04`, PR #5715). Confirmed working
  on 25.04.8. On 25.04 the `output` block still needs `nextflow.preview.output = true`
  **in the script** (setting it in a config file is not enough).
- Releases (GitHub, 2026-11-26): latest stable **26.10.7** (2026-11-20); other maintained lines
  26.04.9 (2026-10-22), 25.10.9 (2026-10-20), 25.04.8 (2025-10-06, last of its line);
  latest edge 26.11.0-edge (2026-11-17).
- Docker Hub `nextflow/nextflow` has tags for every release, including all of the above and
  `-edge` tags. The images run the local executor without Docker-in-Docker. The bundled
  Docker CLI is 19.03 (works against a 29.x daemon via the socket).
- Format generations (from the runs): 25.04 flat envelope + JSON-array `find`;
  25.10 `spec` envelope; 26.04 adds `WorkflowRun.metadata`, `TaskRun.moduleId`;
  26.10 adds `TaskRun.eval` and `AgentRun`.

### 3.3 Design impact

Fixtures are generated offline (manual/weekly job) and PR CI only reads the stored stores and
oracles, so covering every format generation is free per PR:

| Nextflow | Why |
| --- | --- |
| `25.04.8` | minimum; flat envelope, JSON-array `find` |
| `25.10.9` | first `spec` envelope |
| `26.04.9` | `metadata`, `moduleId` |
| `26.10.7` | latest stable; `AgentRun`, `eval` |
| `26.11.0-edge` (non-blocking, weekly) | early warning for format drift (e.g. checksum modes from #8018) |

A three-version matrix (`25.04.8`, `26.04.9`, `26.10.7`) would also be viable, since 25.10 is a
strict subset of 26.04 field-wise.

Fixture generation: the minimal pipeline must use an `output` block (otherwise there is no
`WorkflowOutput` and no published `FileOutput`); on 25.04 the script needs
`nextflow.preview.output = true`. Fixtures are generated under a fixed absolute path inside the
container, with a `nextflow.config` that sets `lineage.enabled = true` copied into the launch
directory (the CLI needs it). An agent fixture (26.10.7, `langchain4j`, against
`nf-llm-replay`) covers `AgentRun`.

---

## 4. The `agent` primitive (what `nf-llm-replay` must emulate)

### 4.1 Initial assumption

A preview `agent` primitive; unknown whether it accepts a `baseUrl` for an OpenAI-compatible
endpoint and which endpoints it calls (chat completions, responses, streaming).

### 4.2 Finding

Sources: [Agents docs](https://docs.seqera.io/nextflow/agent) (`docs/agent.mdx` at
`v26.10.7`), `adr/20260801-agent-primitive.md`, `plugins/nf-agent/src/main/nextflow/agent/ChatModelFactory.groovy`,
`plugins/nf-agent-pi/harness/runner.mjs`, `modules/nextflow/src/main/groovy/nextflow/agent/AgentConfig.groovy`
and `AgentRunnerRequest.groovy`, plus requests captured with a dummy HTTP endpoint.

- Introduced by #7477 (commit `b148cfbaf2`, 2026-08-19), first in **26.08.0-edge**, first stable
  **26.10.0**; still marked preview (`WARN: Agents are a preview feature`). Not available in
  25.x/26.04.
- Two runners, each a plugin auto-loaded from config: `langchain4j` (**default**, plugin
  `nf-agent`, 26.10.7 pulls `nf-agent@0.2.0`; registry latest is 0.3.0) runs in the JVM;
  `pi` (plugin `nf-agent-pi@0.6.0`) runs each agent in the container
  `public.cr.seqera.io/nextflow/nf-agent-pi:0.6.0` and talks to the driver over a TLS RPC broker.
- Config scope `agent { … }`: `runner`, `model` (`<provider>/<model>`), `apiKey`, `apiProvider`
  (`anthropic|azure|gemini|google|mistral|openai|openrouter`), **`baseUrl`**, `requestTimeout`
  (120 s), `maxIterations` (20), `maxToolOutputInlineSize`, `trace`, `rpc.*`, plus task
  directives and `withName`/`withLabel` selectors. Endpoint/credential resolution order:
  `agent.baseUrl`/`agent.apiKey` → `NXF_AGENT_BASE_URL`/`NXF_AGENT_API_KEY` →
  `<PROVIDER>_BASE_URL`/`<PROVIDER>_API_KEY`.
- **langchain4j only speaks the OpenAI wire protocol** (`openai/` prefix required; anything
  else aborts with "only the `openai` wire protocol is supported"). It uses langchain4j
  `OpenAiChatModel` 1.17.0, never streaming.
- With a custom `baseUrl` and no key, Nextflow sends the placeholder credential
  `nxf-no-credential` (`AgentRunnerRequest.PLACEHOLDER_API_KEY`). If `OPENAI_API_KEY` is set in
  the environment but `baseUrl` points to an unrecognized host, that key is **withheld** and
  langchain4j aborts unless `agent.apiKey`/`NXF_AGENT_API_KEY`/`agent.apiProvider` is set.
  The `pi` runner does not use the placeholder: it needs some `apiKey` even for a local endpoint.
- The resolved endpoint is part of the agent task hash (`-resume` cache key).

Captured requests (Nextflow 26.10.7, dummy endpoint, `agent.baseUrl = 'http://127.0.0.1:18080/v1'`):

| Runner / model | Method + path | Streaming | Notable headers | Body shape |
| --- | --- | --- | --- | --- |
| `langchain4j`, `openai/gpt-5-mini` | `POST /v1/chat/completions` | no (`"stream": false`) | `Authorization: Bearer nxf-no-credential`, `User-Agent: langchain4j-openai`, `Connection: Upgrade, HTTP2-Settings`, `Upgrade: h2c` (Java HttpClient) | `{model, messages:[system?, user], stream:false}`; structured output adds `response_format:{type:"json_schema", json_schema:{name:"Output", strict:true, schema}}`; tools add `tools:[{type:"function", function:{name, description, parameters}}]` |
| `pi`, `openai/gpt-5-mini` | `POST /v1/responses` (OpenAI **Responses API**) | **yes** (`"stream": true`, SSE) | `authorization: Bearer <key>`, `x-stainless-*`, `session_id`, `x-client-request-id`, `user-agent: pi (…)` | `{model, input:[{role:"developer"}, {role:"user", content:[{type:"input_text"}]}], stream, prompt_cache_key, store:false, max_output_tokens, reasoning:{effort, summary}, include:["reasoning.encrypted_content"], tools?}`; structured output is a `final_answer` function tool |
| `pi`, `anthropic/claude-sonnet-4-5` (`baseUrl` without `/v1`) | `POST /v1/messages?beta=true` | **yes** (SSE) | `x-api-key`, `anthropic-version: 2023-06-01`, `anthropic-beta: interleaved-thinking-2025-05-14` | `{model, messages, system:[…, cache_control], max_tokens, stream:true, thinking:{type:"enabled", budget_tokens, display}}` |

The user message always ends with the inputs serialized as JSON (`\n\nInput (JSON):\n…` on
langchain4j, `\n\nInput:\n…` on pi). Requests from different agent tasks arrive **concurrently**.
The `pi` requests contain **volatile data**: `prompt_cache_key`, the `session_id` /
`x-client-request-id` headers (fresh UUIDs) and the task work directory inside a
`<cwd>…/work/xx/<hash></cwd>` block of the system/developer prompt.

The lineage side works as documented: each agent run is an `AgentRun` record
(checked with `nextflow lineage view`) and its result a `TaskOutput`.

### 4.3 Design impact: exactly what `nf-llm-replay` must emulate

Yes, the priorities flip compared to the initial assumption (streaming first):

1. **Must (stage 1):** `POST /v1/chat/completions`, **non-streaming JSON**, including
   `response_format` (strict JSON schema) and `tools` / tool-call round trips. This is the
   default runner and the only one that needs no container. Tolerate `Upgrade: h2c` on
   HTTP/1.1 (ignore it) and the placeholder `Bearer nxf-no-credential`.
2. **Should (stage 1 or early stage 2):** `POST /v1/responses` with **SSE** (pi + OpenAI) and
   `POST /v1/messages` with **SSE** (pi + Anthropic; query string `beta=true` must be ignored
   for matching).
3. **Optional:** SSE for chat completions (`"stream": true`). Nextflow 26.10 never sends it;
   keep it only if generic OpenAI clients are a goal.
4. Request key = hash of the normalized request: method + path (without query) + body with
   sorted keys, minus configurable volatile fields. Defaults must include
   `prompt_cache_key`, `user`, `metadata`, and a regex that strips `<cwd>…</cwd>` from
   prompt text. Never hash headers (they carry `session_id` and credentials).
5. Matching must be by key, not by arrival order (concurrent tasks).
6. Upstream routing in `record` mode: forward the path as received to `--upstream`
   (`https://api.openai.com` or `https://api.anthropic.com`).
7. Docs example: `agent.baseUrl = 'http://localhost:<port>/v1'` with no key for langchain4j;
   for pi also set `agent.apiKey` to any dummy value and make the proxy reachable from the
   runner container (host network or a reachable host name).
8. The acceptance criterion "a pipeline example pointing to `nf-llm-replay`" is now confirmed
   feasible and should become a real test (needs only Docker + the Nextflow image).

---

## 5. rmcp (official Rust MCP SDK)

### 5.1 Initial assumption

Find the latest stable version, needed features (server, stdio, macros) and the current tool API.

### 5.2 Finding

Sources: crates.io API, [rust-sdk](https://github.com/modelcontextprotocol/rust-sdk) at
`rmcp-v3.12.0` (`crates/rmcp/README.md`, `crates/rmcp/tests/test_tool_macros.rs`,
`tests/common/calculator.rs`), and a small probe crate (compiled and tested).

- Latest: **`rmcp 3.12.0`** (2026-11-26; 3.x ships roughly weekly), edition 2024, MSRV 1.88.
  Implements MCP spec `2026-07-28` and negotiates older versions (our probe answered a
  `2025-06-18` initialize).
- Default features: `base64, chrono, macros, server`. `chrono` triggers RUSTSEC-2026-0543
  (unmaintained) in cargo-deny; the README recommends turning defaults off.
- Needed: `server` (tool system, includes `schemars`), `macros`, `transport-io` (stdio server).
  For tests: `client` (dev-dependency). `transport-streamable-http-server` later.
- API (3.x): `#[tool_router]` on the impl block, `#[tool(description = …)]` per method,
  `Parameters<T>` with `T: Deserialize + schemars::JsonSchema` (schemars 1.x, doc comments
  become descriptions, JSON Schema draft 2020-12), return `String` or
  `Result<CallToolResult, ErrorData>`, `#[tool_handler(router = self.tool_router, name = …,
  version = …, instructions = …)]` on `impl ServerHandler`. Text content is
  `ContentBlock::text` (there is no `Content` type in 3.x); `ServerHandler::get_info` returns
  `ServerConfig`. Without `name`/`version` the server reports itself as `rmcp`.

Minimal server that compiles and passes (probe crate `rmcp_probe`, `src/lib.rs`):

```toml
rmcp = { version = "3.12", default-features = false, features = ["server", "macros", "transport-io"] }
schemars = "1"
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread", "io-std"] }
# [dev-dependencies]
rmcp = { version = "3.12", default-features = false, features = ["client"] }
```

```rust
use rmcp::{
    ErrorData, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ContentBlock},
    schemars, tool, tool_handler, tool_router,
};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ViewArgs {
    /// Lineage ID, e.g. `lid://<hash>[/<path>]`.
    pub lid: String,
}

#[derive(Debug, Clone)]
pub struct LineageServer { tool_router: ToolRouter<Self> }

impl Default for LineageServer {
    fn default() -> Self { Self { tool_router: Self::tool_router() } }
}

#[tool_router]
impl LineageServer {
    #[tool(description = "View a lineage record (read-only)")]
    async fn view(&self, Parameters(ViewArgs { lid }): Parameters<ViewArgs>)
        -> Result<CallToolResult, ErrorData> {
        if !lid.starts_with("lid://") {
            return Err(ErrorData::invalid_params(format!("not a lineage URL: {lid}"), None));
        }
        Ok(CallToolResult::success(vec![ContentBlock::text(format!("{{\"lid\":\"{lid}\"}}"))]))
    }
}

#[tool_handler(router = self.tool_router, name = "nf-lineage-mcp", version = "0.0.0",
               instructions = "read-only lineage probe")]
impl ServerHandler for LineageServer {}

// main.rs: LineageServer::default().serve(rmcp::transport::stdio()).await?.waiting().await?;
```

In-process protocol test (client and server over an in-memory duplex pipe; real JSON-RPC):

```rust
use rmcp::{ServiceExt, model::CallToolRequestParams};

#[tokio::test]
async fn view_over_mcp() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (server_io, client_io) = tokio::io::duplex(64 * 1024);
    let server = tokio::spawn(async move {
        rmcp_probe::LineageServer::default().serve(server_io).await?.waiting().await?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    });
    let client = ().serve(client_io).await?;            // `()` is a no-op ClientHandler
    let tools = client.list_all_tools().await?;
    assert_eq!(tools[0].name, "view");
    let args = serde_json::json!({ "lid": "lid://abc" }).as_object().cloned().unwrap();
    let res = client.call_tool(CallToolRequestParams::new("view").with_arguments(args)).await?;
    assert_eq!(res.content[0].as_text().unwrap().text, r#"{"lid":"lid://abc"}"#);
    client.cancel().await?;
    server.await??;
    Ok(())
}
```

`ErrorData::invalid_params` reaches the client as JSON-RPC error `-32602`. The stdio binary
was also driven with raw JSON-RPC lines over stdio.

### 5.3 Design impact

Minor: pin `rmcp = "3.12"` with `default-features = false`; map `LineageError` to
`ErrorData` (`invalid_params` for bad LIDs, `resource_not_found`/`internal_error` otherwise);
the "call over the MCP protocol" criterion is satisfied with the duplex pattern above, plus one
end-to-end test that spawns the binary over stdio (feature `transport-child-process`).

---

## 6. object_store

### 6.1 Initial assumption

`object_store` supports local, S3, GCS and Azure with the chosen version; plus an
`#[async_trait] LineageStore` trait.

### 6.2 Finding

Sources: crates.io API and the `object_store-0.14.2` sources, plus a probe crate run against a
real 26.10.7 store and MiniStack.

- Latest **`object_store 0.14.2`** (2026-09-15), edition 2024, MSRV 1.85.
- Features: `fs` (default: `LocalFileSystem`), `aws`, `gcp`, `azure`, `http`; `memory::InMemory`
  needs no feature. `aws`/`gcp`/`azure` pull reqwest + rustls with `aws-lc-rs` (`ring` exists
  as an alternative crypto feature).
- API: `ObjectStore` (dyn-compatible, itself `#[async_trait]`) has `list(prefix) ->
  BoxStream<Result<ObjectMeta>>`, `list_with_delimiter`, `get_opts`; the convenience
  `get`/`put`/`head` now live in the extension trait **`ObjectStoreExt`** (must be imported).
  `object_store::parse_url_opts(&url, opts)` builds a store from `s3://…`, `gs://…`,
  `az://…`, `file://…` URLs.
- Custom S3 endpoint: `AmazonS3Builder::new().with_endpoint("http://localhost:4566")
  .with_allow_http(true).with_region(…).with_bucket_name(…).with_access_key_id(…)
  .with_secret_access_key(…)` (path-style is the default; `with_virtual_hosted_style_request`
  toggles it). `AmazonS3Builder::from_env()` maps every `AWS_*` variable
  (`AWS_ENDPOINT_URL`, `AWS_ALLOW_HTTP`, …), and the same keys work as `parse_url_opts` options
  (`aws_endpoint`, `aws_allow_http`).
- **`#` gotcha (verified):** `Path::from("h#output/.data.json")` percent-encodes to
  `h%23output/.data.json` and reading it returns `NotFound` on local disk; `Path::parse(…)`
  keeps `#` verbatim and works on local, in-memory and S3. Locations returned by `list` are
  already correct.
- Probe results: local store listed 28 objects and read `#output` records; S3 on MiniStack:
  uploaded the same 28 objects, listed and read them back (keys keep the literal `#`).
- **Native `async fn` in traits is not dyn-compatible**: `Box<dyn LineageStore>` with native
  `async fn` fails with `E0038` on rustc 1.94.1. With
  `#[async_trait]` (0.1.92) it works.

### 6.3 Design impact

- Keep `#[async_trait]` on `LineageStore` (object_store does the same). Alternative only if
  generics are acceptable everywhere (no `dyn`).
- Rule for the implementation: **build keys with `Path::parse`, never `Path::from`, or reuse
  `ObjectMeta.location`**; add a unit test for a `#output` key on every backend.
- `--store` accepts a URL and goes through `parse_url_opts`, with `AWS_*`/`GOOGLE_*`/`AZURE_*`
  env vars for credentials and endpoints.
- `object_store` depends on `chrono` (RUSTSEC-2026-0543) and `humantime`
  (RUSTSEC-2026-0542): `deny.toml` needs explicit, commented ignores for these two advisories.

---

## 7. GitHub Spec Kit vs OpenSpec

### 7.1 Initial assumption

Use GitHub Spec Kit (`specs/` with one directory per component); find version, init command
and command names.

### 7.2 Finding

- Spec Kit **v1.4.5** (2026-11-25). Install: `uv tool install specify-cli` (or one-shot
  `uvx --from git+https://github.com/github/spec-kit.git specify init …`); init:
  `specify init <dir> --integration <agent>` (`--here` for an existing repo, e.g.
  `--integration claude`). Layout: `.specify/` (memory/constitution, templates, scripts) and
  `specs/<NNN-feature>/{spec,plan,tasks}.md`.
- Commands (dotted form; Copilot/Claude skills use the hyphenated form `/speckit-specify`):
  `/speckit.constitution`, `/speckit.specify`, `/speckit.clarify`, `/speckit.plan`,
  `/speckit.checklist`, `/speckit.tasks`, `/speckit.analyze`, `/speckit.implement`,
  `/speckit.converge` (new: assess implementation vs artifacts). Extensions add e.g. `/speckit.bug.*`.
- Nextflow itself uses Spec Kit (`.specify/` is present at `v26.10.7`).
- The repository keeps its specifications in `openspec/specs/<capability>/spec.md`, which are the
  source of truth. (OpenSpec itself is at v2.2.0, npm `@fission-ai/openspec`.)

### 7.3 Design impact

Yes: replace `specs/` with `openspec/` (`openspec/specs/nf-lineage`, `openspec/specs/nf-llm-replay`,
`openspec/specs/nf-lineage-mcp`). Do not run `specify init` (two SDD layouts would confuse contributors).

---

## 8. MiniStack and Rust quality tooling

### 8.1 MiniStack (S3 integration tests)

- Repo `ministackorg/ministack` (MIT), release **v1.5.45** (2026-11-26). Docker image
  **`ministackorg/ministack:1.5.45`** (`latest`, `1.5`, and `-full` variants), all services on
  **port 4566**, health at `/_ministack/health`, reset at `POST /_ministack/reset`. Any access
  key works (a 12-digit key selects the account). S3 supports buckets, objects, multipart,
  ListObjects v1/v2, versioning, etc.
- Verified: bucket creation with an unsigned `PUT /lineage`, then object_store
  put/list/get against `http://localhost:<port>` with `test`/`test` credentials.
- Design impact: none; the planned approach works. Use it as a GitHub Actions service container
  (`ministackorg/ministack:1.5.45`, port 4566, health check on `/_ministack/health`).

### 8.2 insta, assert_cmd, cargo-deny, cargo-machete

Probe: a small test-tooling crate (edition 2024, rustc 1.94.1).

| Tool | Latest | Result on rustc 1.94.1 / edition 2024 |
| --- | --- | --- |
| `insta` | 1.49.0 (feature `json`) | OK (`assert_json_snapshot!`) |
| `assert_cmd` | 2.2.2 | OK (`Command::cargo_bin`, no deprecation warning) |
| `cargo-deny` | 0.22.1 | needs **rustc ≥ 1.95** → installed **0.21.0**, works (`check` passes with license allowlist and the two advisory ignores) |
| `cargo-machete` | 0.10.1 | **fails to build** on 1.94.1 (`path_is_empty` unstable there) → **0.9.2** works |

Latest stable Rust is **1.100.0** (2026-11-12). Design impact: CI uses `stable` (all latest
versions fine); add `rust-toolchain.toml` only if we want local/CI parity, or document that
contributors on older toolchains install `cargo-deny@0.21.0` / `cargo-machete@0.9.2`.
Set `rust-version` in the workspace (rmcp needs ≥ 1.88). Resulting stack: `rmcp 3.12` (no default
features), `object_store 0.14` (`aws`, `gcp`, `azure`), `async-trait 0.1`, `insta 1.49`,
`assert_cmd 2.2`; `deny.toml` ignores RUSTSEC-2026-0542/0543 with a note.

---

## Open items

- What string and mtime Nextflow hashes for a `FileOutput` on S3 (S3 path `toString` and
  `LastModified`), needed for `check` on object storage. Verify by running the minimal pipeline
  with `lineage.store.location = 's3://…'` and `-output-dir s3://…` against MiniStack
  (`aws.client.endpoint`, `aws.client.s3PathStyleAccess`).
- `master` commit `5fffa38844` (#8018) will change lineage checksum modes in the next stable;
  re-run these verifications against `26.11.0-edge`/27.04 when available.
- The `agent` primitive is preview: re-capture requests on every new stable release.
