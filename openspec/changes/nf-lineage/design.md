# Design: nf-lineage 0.1

## Technical Approach

Contracts first (spec `nf-lineage`): `Lid` + `Record` + `FindQuery` are pure and IO-free (PR A); `LineageStore` and its single `object_store` implementation, `check`, `diff` sit on top (PR B); `view`, `provenance`, the CLI contract suite, MiniStack test, CI and release metadata close it (PR C). Every free function takes `&dyn LineageStore`, so other backends (Seqera Platform, DB) plug in without touching them. Read-only: the crate only calls `list`, `list_with_delimiter`, `get` on object stores and `metadata` on local files.

## Architecture Decisions

| Topic | Options | Tradeoff | Decision |
|-------|---------|----------|----------|
| Record model | typed structs / raw `Value` / per-version modules | typed = clear API, type drift → error | typed, every field `Option`, `#[serde(flatten)] extra: Map`; fields declared in Nextflow order |
| Envelope | serde `untagged`/`tag` / hand-written step | serde can't do "spec or root" + version check with good errors | `Record::parse`: `Value` → version → kind → `spec`-or-root (minus `version`/`kind`) → `from_value::<T>` |
| JSON key order | default / `serde_json/preserve_order` | indexmap dep vs alphabetical maps in `get`/`diff_raw` | `preserve_order` (records print like on disk) |
| Trait | native `async fn` / `#[async_trait]` | native not dyn-compatible (E0038, verified) | `async-trait 0.1`; trait = the 3 planned methods only |
| `.history` | trait method / inside `list_runs` | CLI reads it only for `list` | private helper of `ObjectLineageStore::list_runs` |
| Backends | object_store + `std::fs` / object_store only | one code path, `#` verified on local/memory/S3 | object_store only; keys via `Path::parse` (never `Path::from`) |
| S3 markers | trust listing / filter | Nextflow writes zero-byte `dir/` objects | records = last segment `.data.json`; history = exactly one segment under `.history` |
| Cloud config | `parse_url_opts` / `*Builder::from_env().with_url()` + `with_config` | `parse_url_opts` ignores env | `from_env` + explicit options (`open_with_options`), no `set_var` (unsafe in 2024) |
| Cloud features | always on / cargo features | aws-lc C build, licenses | `aws`, `gcp`, `azure` default-on, forwarded to `object_store` |
| murmur3 | hand-rolled / `murmur3 0.5` | 45 lines + vectors vs tiny MIT/Apache dep | `murmur3 0.5`; `u128::to_le_bytes` (verified = Guava) |
| `check` on S3 | `Unsupported` / implement | spike conclusive (exploration §2) | hash `/<bucket>/<key>`, size+mtime from `list_with_delimiter(parent)` (HEAD loses ms on MiniStack); `gs`/`az` → `Unsupported` |
| `check` result | bool / error / report | mismatch is an outcome, not a failure | `CheckReport { status: Valid\|Mismatch\|FileMissing\|NoChecksum }`; bad kind/algo → `Err` |
| Semantic diff | blacklist volatile / whitelist projection | blacklist leaks new volatile fields | whitelist per kind, LID hashes masked (`lid://*`) |
| Raw diff | port JGit / `similar` | byte-identical not needed (spec: +/- multiset) | `similar 2` unified diff, CLI header |
| Find numbers | `Value` display / Java rules | parity with Gson `Double` | ints for `size`/`maxIterations`, else Java `Double.toString` (12 lines) |
| URL decoding | `percent-encoding` / hand-rolled | Java `+`→space and error on bad `%` | hand-rolled `url_decode` (~20 lines) |
| `provenance` | MCP crate / library | needed by MCP and stage-2 `rig-nf-lineage` | library, BFS over `get`, visited set |
| MiniStack test | testcontainers / docker CLI / env-gated | deps + flakiness vs simplicity | gated on `NF_LINEAGE_MINISTACK_URL`; unset → skip; set → readiness wait (raw TCP `PUT /<bucket>`, ≤ 60 s) then hard fail |
| `contract` job | skip in `test` / duplicate | `test` spec says `cargo test --workspace` | duplicate (seconds); named status check |
| Snapshots | hand masking / insta `filters` | regex only in dev | `insta` `json`+`filters`, one snapshot per version |

## Data Flow

```
open("s3://b/store") ─→ ObjectLineageStore{Arc<dyn ObjectStore>, prefix}
get(lid) ─→ Path::parse(prefix/lid.record_key()/.data.json) ─→ get ─→ Record::parse
find(q)  ─→ list(prefix) ─filter .data.json─→ get×16 concurrent ─→ q.matches ─→ sorted Lids
check(lid) ─→ get ─→ FileOutput.path ─┬ local/file:// ─ path_map ─→ fs::metadata (follows symlinks)
                                      └ s3://b/k ─→ list_with_delimiter(b/parent) ─→ size, mtime_ms
           ─→ murmur3(utf16le(hashed) ‖ le64 size ‖ le64 ms) ─→ CheckReport
provenance(file) ─→ FileOutput ─source/taskRun→ TaskRun|AgentRun ─input lids→ FileOutput … (BFS)
```

## File Changes

| File | Action | PR | Est. lines |
|------|--------|----|-----------|
| `crates/nf-lineage/Cargo.toml` | Modify | A,B,C | 40 |
| `src/lib.rs` (docs, re-exports, `pub use object_store`) | Rewrite | A,B,C | 50 |
| `src/error.rs` | Create | A (+B variants) | 50 |
| `src/lid.rs` (+tests) | Create | A | 100+60 |
| `src/model.rs` (10 structs) | Create | A | 190 |
| `src/record.rs` (+tests) | Create | A | 85+85 |
| `src/query.rs` (+tests) | Create | A | 160+90 |
| `tests/common/mod.rs`, `tests/fixtures_parse.rs` | Create | A (+C) | 70 |
| `src/store.rs`, `tests/store.rs` | Create | B | 210+100 |
| `src/check.rs`, `tests/check.rs` | Create | B | 110+85 |
| `src/diff.rs`, `tests/diff.rs` | Create | B | 180+60 |
| `deny.toml` | Modify | B | 15 |
| `src/view.rs`, `src/provenance.rs`, `tests/provenance.rs` | Create | C | 55+80+40 |
| `tests/contract.rs`, `tests/s3_ministack.rs` | Create | C | 230+120 |
| `.github/workflows/ci.yml`, `CONTRIBUTING.md`, `crates/nf-lineage/README.md` | Modify/Create | C | 50+15+80 |

Totals ≈ A 790, B 775, C 690 (snapshots, `Cargo.lock` excluded).

## Interfaces / Contracts

```toml
[dependencies]
async-trait = "0.1"; futures = "0.3"; murmur3 = "0.5"; similar = "2"; thiserror = "2"; url = "2"
object_store = "0.14"                      # default `fs`
serde = { version = "1", features = ["derive"] }
serde_json = { version = "1", features = ["preserve_order"] }
[features] default = ["aws", "gcp", "azure"]; aws = ["object_store/aws"]; gcp = ["object_store/gcp"]; azure = ["object_store/azure"]
[dev-dependencies]
insta = { version = "1.49", features = ["json", "filters"] }; tempfile = "3"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

```rust
// error.rs
#[non_exhaustive] #[derive(Debug, thiserror::Error)]
pub enum LineageError {
    InvalidLid { input: String, reason: String },
    InvalidQuery { message: String },            // Display = CLI text
    NotFound { target: String },
    InvalidFormat { path: String, message: String },
    UnsupportedVersion { path: String, version: String },
    UnexpectedKind { lid: String, expected: &'static str, found: String },
    UnsupportedChecksum { path: String, algorithm: String, mode: String },
    Unsupported(String),
    Storage(#[from] object_store::Error),        // PR B
    Io { path: String, #[source] source: std::io::Error }, // PR B
}
// lid.rs — derives Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord (by canonical string)
impl Lid { pub fn parse(s: &str) -> Result<Self, LineageError>; pub fn hash(&self) -> &str;
  pub fn path(&self) -> Option<&str>; pub fn fragment(&self) -> Option<&str>; pub fn record_key(&self) -> String }
// + FromStr, Display, TryFrom<&str>, Serialize, Deserialize (string)
// model.rs — all fields Option<_>, camelCase, extra: Map<String, Value>
pub struct WorkflowRun { workflow: Option<Workflow>, session_id, name, params: Option<Vec<Parameter>>, config: Option<Value>, metadata: Option<Value>, extra }
pub struct Workflow { script_files: Option<Vec<DataPath>>, repository, commit_id, extra }
pub struct WorkflowOutput { created_at, workflow_run, output: Option<Vec<Parameter>>, extra }
pub struct TaskRun { session_id, name, code_checksum: Option<Checksum>, script, eval: Option<Value>, input: Option<Vec<Parameter>>,
  container, conda, spack, architecture, global_vars: Option<Value>, bin_entries: Option<Vec<DataPath>>, workflow_run, module_id, extra }
pub struct TaskOutput { task_run, workflow_run, created_at, output: Option<Vec<Parameter>>, labels: Option<Vec<String>>, extra }
pub struct FileOutput { path, checksum: Option<Checksum>, source, workflow_run, task_run, size: Option<u64>, created_at, modified_at, labels, extra }
pub struct AgentRun { session_id, name, code_checksum, runner, model, resolved_model, instruction, goal, prompt_template,
  max_iterations: Option<i64>, output_schema, tools: Option<Vec<String>>, skills: Option<Vec<String>>, input, container, workflow_run, module_id, extra }
pub struct Checksum { value, algorithm, mode, extra }  pub struct DataPath { path, checksum: Option<Checksum>, extra }
pub struct Parameter { r#type /* "type" */, name, value: Option<Value>, extra }
// record.rs
#[non_exhaustive] pub enum Record { WorkflowRun(_), WorkflowOutput(_), TaskRun(_), TaskOutput(_), FileOutput(_), AgentRun(_), Unknown(Value) }
impl Record { pub const VERSION: &str = "lineage/v1beta1";
  pub fn parse(bytes: &[u8], origin: &str) -> Result<Self, LineageError>; pub fn kind(&self) -> &str;
  pub fn spec(&self) -> Value; pub fn to_json(&self) -> Value }   // + Serialize = to_json
// query.rs
impl FindQuery { pub fn new<K: Into<String>, V: Into<String>>(p: impl IntoIterator<Item=(K,V)>) -> Result<Self, LineageError>;
  pub fn parse_args<S: AsRef<str>>(args: impl IntoIterator<Item=S>) -> Result<Self, LineageError>;
  pub fn matches(&self, r: &Record) -> bool; pub fn terms(&self) -> &[(String, Vec<String>)] }
pub(crate) fn navigate(v: &Value, path: &[&str]) -> Value; pub(crate) fn validate_property(seg: &str) -> Result<(), LineageError>;
// PROPERTIES = workflow sessionId name params config metadata createdAt workflowRun output codeChecksum script eval input
//   container conda spack architecture globalVars binEntries moduleId taskRun labels path checksum source size modifiedAt
//   runner model resolvedModel instruction goal promptTemplate maxIterations outputSchema tools skills value algorithm mode
//   type scriptFiles repository commitId class
// store.rs
#[async_trait] pub trait LineageStore: Send + Sync {
  async fn list_runs(&self) -> Result<Vec<RunSummary>, LineageError>;
  async fn get(&self, lid: &Lid) -> Result<Record, LineageError>;
  async fn find(&self, query: &FindQuery) -> Result<Vec<Lid>, LineageError>; }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)] pub struct RunSummary { pub timestamp: String, pub run_name: String, pub session_id: String, pub lid: Lid }
impl ObjectLineageStore { pub fn new(store: Arc<dyn ObjectStore>, prefix: object_store::path::Path) -> Self;
  pub fn open(location: &str) -> Result<Self, LineageError>;
  pub fn open_with_options<K: AsRef<str>, V: Into<String>>(location: &str, opts: impl IntoIterator<Item=(K,V)>) -> Result<Self, LineageError> }
// check.rs
#[derive(Debug, Clone, Default)] pub struct CheckOptions { pub path_map: Vec<(String, PathBuf)>, pub storage_options: Vec<(String, String)> }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)] pub struct CheckReport { pub lid: Lid, pub path: String, pub status: CheckStatus, pub expected: Option<String>, pub actual: Option<String> }
#[non_exhaustive] pub enum CheckStatus { Valid, Mismatch, FileMissing, NoChecksum }
pub async fn check(store: &dyn LineageStore, lid: &Lid, opts: &CheckOptions) -> Result<CheckReport, LineageError>;
pub fn nextflow_standard_checksum(hashed_path: &str, size: u64, mtime_ms: i64) -> String;
// diff.rs
#[derive(Serialize)] pub struct SemanticDiff { pub a: Lid, pub b: Lid, pub changes: Vec<Change> }
#[derive(Serialize)] pub struct Change { pub path: String, pub a: Option<Value>, pub b: Option<Value> }
pub fn diff_records(a: &Record, b: &Record) -> Vec<Change>;
pub async fn diff(store: &dyn LineageStore, a: &Lid, b: &Lid) -> Result<SemanticDiff, LineageError>;
pub async fn diff_raw(store: &dyn LineageStore, a: &Lid, b: &Lid) -> Result<String, LineageError>;
// view.rs / provenance.rs (PR C)
pub async fn view(store: &dyn LineageStore, lid: &Lid) -> Result<Value, LineageError>;
#[derive(Serialize)] pub struct Provenance { pub target: Lid, pub nodes: Vec<ProvenanceNode> }
#[derive(Serialize)] pub struct ProvenanceNode { pub lid: Lid, pub kind: String, pub name: Option<String>, pub inputs: Vec<ProvenanceInput> }
#[derive(Serialize)] #[serde(tag = "type", rename_all = "snake_case")]
pub enum ProvenanceInput { Lineage { name: Option<String>, lid: Lid }, External { name: Option<String>, path: String, checksum: Option<Checksum> }, Value { name: Option<String>, value: Value } }
pub async fn provenance(store: &dyn LineageStore, file: &Lid) -> Result<Provenance, LineageError>;
```

Semantic projection (keys of `Change.path`): WorkflowRun `params.<name>`, `workflow.scriptFiles.<path>`, `workflow.repository`, `workflow.commitId`, `nextflow.version` (from `metadata.nextflow.version`); TaskRun `name, codeChecksum, script, container, conda, spack, architecture, moduleId, input.<name|#i>, binEntries.<path>`; AgentRun `name, codeChecksum, runner, model, resolvedModel, instruction, goal, promptTemplate, maxIterations, outputSchema, tools, skills, container, moduleId, input.*`; FileOutput `size, labels, source`; Task/WorkflowOutput `output.<name|#i>, labels`; Unknown: spec top-level keys minus `sessionId, createdAt, modifiedAt, workflowRun, taskRun`. Run level: `tasks.<name>.{count, codeChecksum, container, conda, spack, architecture, moduleId, runner, model, resolvedModel}` (sorted unique values, scalar if one) and `outputs.<name>` (from `#output`).

Java double: `|x|∈[1e-3,1e7)` → `format!("{x}")` + `.0` if integral; else `format!("{x:E}")` with `.0` inserted in a dot-less mantissa (`1E7`→`1.0E7`).

## Testing Strategy

| Layer | What | Approach |
|-------|------|----------|
| Unit | Lid, envelope, query semantics, Java number strings, golden checksums, projection | table-driven `#[cfg(test)]`; query table from `scratch-nf-lineage/findexp` |
| Fixture | all stores parse; snapshots | `tests/fixtures_parse.rs`, insta filters for hex32/UUID/ISO/run names |
| Store | local fixtures, InMemory markers, errors, dyn use | `tests/store.rs` (`#[tokio::test]`, `Box<dyn LineageStore>`) |
| Check | mtime restore via `File::set_modified`, tamper, delete | `tests/check.rs` on `tempfile` copies, `path_map` |
| Contract | list/view/find/diff/check vs `oracle/` | `tests/contract.rs`, aggregated failures `(version, case)` |
| Integration | S3 roundtrip + S3 check | `tests/s3_ministack.rs`, env-gated |
| Docs | public examples | doctests on InMemory (`#[tokio::main]`), `no_run` for `open` |

CI: add `contract` (`cargo test -p nf-lineage --test contract --locked`) and `integration` (service `ministackorg/ministack:1.5.45`, `ports: ["4566:4566"]`, env `NF_LINEAGE_MINISTACK_URL`, `cargo test -p nf-lineage --test s3_ministack --locked`); same checkout/toolchain/cache steps as `test`.

## Migration / Rollout

No migration. PRs land A → B → C after `lineage-fixtures`. `publish = false` is removed in PR C only; publishing is a later release change.

## Open Questions

- [ ] `gs://`/`az://` hashed path string for `check` (needs a GCS/Azure Nextflow run) — follow-up.
- [ ] Support `lenient`/`deep`/`sha256` once Nextflow ≥ 27.04 emits them (#8018).
- [ ] Mixed-timezone `.history` ordering (we sort by timestamp text; CLI by parsed zone).

## Deviations (2026-11-26, apply PR A)

- **Fixture layout** matches the plan (`fixtures/lineage/<ver>/.lineage/<hash>[#output]/[<path>/].data.json`,
  `.history/<hash>` one line, no trailing newline); nothing to adapt.
- **Snapshots**: insta `filters` are applied after serialization, so the record order (sorted by
  path, which contains hashes) would change on every fixture regeneration. The test masks each
  record's JSON first (dev-dep `regex`, already a transitive dep of insta `filters`), sorts the
  masked strings, then snapshots: insta features are `json` only. Run names are masked from the
  `.history` entries.
- `Record::parse` reads the bytes straight into a `serde_json::Map`, so a non-object (`[]`) is an
  `InvalidFormat` with serde's "invalid type" message.
- `Lid` keeps its canonical string as the first field, so the derived `Ord`/`Hash` are the
  canonical string's.
- Model structs are `#[non_exhaustive]` + `Default` (new Nextflow fields are not breaking).
- **FindQuery moved to PR B** (tasks.md escape hatch). Notes for it from `upstream-src` 26.10.7:
  `navigateCollection` does *not* flatten nested lists (a list value inside a list is kept as one
  element, whose Groovy `toString` never equals a query value); the CLI prefixes errors with
  `Error searching for <args[0]>.` (first argument, not the failing one); the property error may
  end with ` -- Did you mean one of these?  <names>` (Levenshtein `closest` over a `HashSet`,
  e.g. `config.runName` → `name`): match the CLI text up to that suffix or port `closest`.
  Java `URLDecoder` errors: `URLDecoder: Illegal hex characters in escape (%) pattern - Error at
  index 0 in: "zz"`, `URLDecoder: Incomplete trailing escape (%) pattern`.
