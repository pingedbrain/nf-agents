## Exploration: nf-lineage (typed, tolerant reader for Nextflow lineage stores)

Date: 2026-11-26. Inputs: the project's stage-1 scope (lineage reader, `find`/`check`/`diff`),
the empirical verification notes (Nextflow 25.04–26.10 behaviour, object_store and MiniStack
probes), the extension-point contracts, and the parallel change `lineage-fixtures` (proposal +
spec, in progress). Experiments for this change ran outside the repository and are not committed.

### Current State

- `crates/nf-lineage` is a stub: `0.0.0`, `publish = false`, zero dependencies, one test
  (`version_is_pre_release`). Main spec `rust-workspace` pins that stub shape ("nf-lineage stub").
- Workspace: edition 2024, `rust-version = 1.85`, toolchain `1.100.0`, lints deny
  `unwrap/expect/panic/indexing/todo/unimplemented`, `unsafe_code = forbid`, `missing_docs = warn`
  (fails CI through `-D warnings`). `deny.toml` allowlist has no `CDLA-Permissive-2.0`/`OpenSSL`
  and no advisory ignores yet.
- CI (`ci.yml`): jobs `lint` and `test` only.
- Fixtures do not exist yet. `lineage-fixtures` fixes the contract this change consumes:
  `fixtures/lineage/<ver>/` (core `25.04.8`, `25.10.9`, `26.04.9`, `26.10.7`, plus
  `26.10.7-agent`) with `.lineage/`, data files at their path relative to the **recorded root
  `/nf-agents-fixture`** (no symlinks), `oracle/<case>.{args,out,err,exit}` (`.args` = JSON argv)
  + `ids.json`, `manifest.json` (`files[{path,size,mtime_ms,sha256}]`), `shape.txt`.

### Verified facts that shape the design (beyond the earlier verification notes)

1. **`check` is fully derivable from the record.** For all 40 `FileOutput` records of four local
   test stores, `murmur3_x64_128(utf16le(path) ‖ le64(size) ‖ le64(floor_ms(modifiedAt)))` equals
   `checksum.value` (a throwaway script: 40 ok, 0 bad). So `size`/`modifiedAt`
   in the record are exactly the hashed inputs.
2. **Open item resolved — `check` on S3 works.** Ran the minimal pipeline on Nextflow 26.10.7 with
   `lineage.store.location = 's3://lineage/store'` and `-output-dir s3://lineage/results` against
   MiniStack 1.5.45 (`aws.client.endpoint`, `aws.client.s3PathStyleAccess = true`). Results:
   - Published `FileOutput.path` is `s3://lineage/results/summary.txt`; `modifiedAt` has ms
     precision (`2026-11-26T21:02:17.562Z`).
   - The hashed string is **`/<bucket>/<key>`** (`S3Path.toAbsolutePath()`), not the `s3://` URI:
     `murmur3("/lineage/results/summary.txt", 9, 1795726937562) = c47bf938e54f24686b01873ab8d1ba53`
     = the record (other candidates do not match).
   - `nextflow lineage check` on the S3 file and on the local task file both succeed; `view`, `find`,
     `list` work on an S3 store.
   - object_store 0.14.2 reproduces it: `list_with_delimiter(parent)` returns `last_modified` with ms
     (`…17.562`) and gives the right hash; **`head` only has second precision on MiniStack**
     (`Last-Modified` header) and gives a wrong hash. `list(Some(file_path))` returns nothing (prefix
     is treated as a directory). → `check` must stat S3 objects through a parent listing.
   - The murmur3 crate (`0.5.2`) `murmur3_x64_128(..., 0)` → `u128::to_le_bytes()` hex = Guava's
     `HashCode.toString()` (verified on the S3 vector).
   - Nextflow writes **directory-marker objects** on S3 (`store/`, `store/<hash>/`,
     `store/.history/`, zero bytes). object_store lists them with the trailing `/` stripped
     (e.g. `store/82fc…#output`, size 0). Key filters must accept only `…/.data.json` records and
     `.history/<one segment>` entries.
3. **`find` semantics** (26.10.7, `LinCommandImpl.find` → `LinPropertyValidator` → `DefaultLinStore.search`
   → `LinUtils.checkParams/navigate`), confirmed with 40 CLI queries (a dedicated `findexp` test store):
   - Args `k=v`; no `=` → `Parameter <arg> doesn't contain '=' separator`; key and value are
     `URLDecoder`-decoded (`+` → space, `%XX`); `label` → `labels`; repeated keys accumulate.
   - Every dot-separated key segment must be a model property name (union of all model classes plus
     Groovy's `class`), else `Property '<seg>' doesn't exist in the lineage model.` — so
     `config.runName`, `metadata.nextflow.version`, `kind=…` are **rejected**; `class=x` is accepted.
   - `type` (whole key) = record kind; case-sensitive (`type=filEoutput` → nothing).
   - Navigation flattens collections (`params.value`, `input.value.path`); falsy elements dropped;
     a single result is unwrapped.
   - Match: falsy value (null, 0, `""`, `false`, empty list/map) never matches (`size=0` → nothing);
     collection → `containsAll(expected)` on stringified elements; scalar → only one expected value,
     `toString` equality.
   - **Numbers in untyped positions (`Parameter.value`, maps) are Gson `Double`s**: `12` only matches
     `12.0`, `12345678` only `1.2345678E7`; typed `Long` fields (`size`) stay integers (`size=9`).
   - Output order is a filesystem walk → compare as sets.
4. **`view` semantics**: `lid://h#frag` loads `h`, validates fragment segments like `find`, and for
   WorkflowRun/TaskRun/AgentRun with first segment `output` navigates the sibling `h#output` record
   (`[]` if absent). Errors print `Error loading <lid> - <msg>` on stdout with exit 0. `lid:///`,
   `lid://` and uppercase hashes are errors/not found; `?query` is ignored with a warning. View
   re-renders numbers in untyped positions as doubles (`"poolSize": 12.0`, `"build": 15252.0`) and
   adds `null` for absent known fields, drops unknown fields → contract compare must normalize.
5. **`list`** reads `.history/<runHash>` (single line, **no trailing newline**,
   `TIMESTAMP\tRUN NAME\tSESSION ID\tLINEAGE ID`, Groovy `tokenize` drops empty tokens), sorted by
   timestamp; a parse failure aborts the remaining entries (warn only).
6. **`diff` CLI** = unified diff of the Gson pretty-printed records (HTML-escaped `<` etc.). Between
   the two minimal runs it shows `sessionId`, `name`, `params.greeting`, `config.params.greeting`,
   `config.runName`, `config.outputDir`, `metadata.{runName,start,commandLine,outputDir,sessionId}`.
   A whitelist semantic projection (params, script checksums, Nextflow version, per-task
   codeChecksum/container/conda/spack/architecture/moduleId, outputs with LID hashes masked) yields
   only `params.greeting` for that pair.
7. `check` CLI: no checksum → succeeds; algorithm ≠ `nextflow` → error; missing file → error; non
   FileOutput → `Unexpected lineage object type: …TaskRun`. Symlinks are followed for size/mtime but
   the recorded (unresolved) path string is hashed.

### Affected Areas

- `crates/nf-lineage/` — whole crate (src modules, tests, README, Cargo metadata).
- `Cargo.toml` (workspace) — none required; `Cargo.lock` regenerated.
- `deny.toml` — advisory ignores RUSTSEC-2026-0542/0543 and crate-scoped license exceptions pulled by
  `object_store` features (`webpki-root-certs`: CDLA-Permissive-2.0; possibly `aws-lc-sys`: OpenSSL).
- `.github/workflows/ci.yml` — new jobs `contract`, `integration` (MiniStack service container).
- `CONTRIBUTING.md` — how to run the MiniStack integration test locally.
- `openspec/specs/rust-workspace` (stub requirement retired), `ci-pipeline`, `contributor-guide`, new `nf-lineage`.

### Approaches

1. **Typed structs + tolerant envelope (recommended)** — one struct per kind with all-`Option` fields
   and `#[serde(flatten)] extra`, a hand-written envelope step (version check, flat vs `spec`,
   dispatch on `kind`, `Unknown(Value)` fallback). FindQuery/view navigation operate on the
   record's JSON projection (Groovy semantics reimplemented in ~150 lines).
   - Pros: typed access for diff/provenance/MCP; exact error typing; unknown fields kept.
   - Cons: a future type change in a known field becomes `InvalidFormat` (clear, but not silent).
   - Effort: Medium.
2. **Raw `serde_json::Value` everywhere + accessor helpers** — no structs.
   - Pros: never fails on type drift; less code.
   - Cons: the project requires a typed `Record`; weak API for downstream crates; easy to misuse.
   - Effort: Low.
3. **Port Nextflow's Groovy model 1:1 (versioned modules per Nextflow release)** —
   - Pros: maximal fidelity. Cons: four near-identical copies; the format is one schema
     (`lineage/v1beta1`) with additive fields. Effort: High.

Store access: `object_store` 0.14 everywhere (local, S3, GCS, Azure, InMemory) vs a separate
`std::fs` path for local. object_store already handles local and `#` keys (with `Path::parse`),
so one implementation is enough.

`check` on object storage: (a) supported for `s3://` data paths via parent listing — verified bit-for-bit
above; (b) typed `Unsupported` for everything but local. → (a) for S3, `Unsupported` for `gs://`, `az://`
and other schemes (no oracle yet).

### Recommendation

Approach 1 with a single `ObjectLineageStore` over `object_store`. Keep the planned trait exactly
(`list_runs`, `get`, `find`) — `.history` parsing is an implementation detail of `list_runs`; `diff`,
`check`, `view`, `provenance` are free functions over `&dyn LineageStore`. Put `provenance` in the
library (pure graph walk over `get`; reused by `nf-lineage-mcp` and stage-2 `rig-nf-lineage`).
Ship `check` for local and S3 data paths; `gs://`/`az://` → `LineageError::Unsupported`.
Three PRs (A types+parser+FindQuery, B store+check+diff, C view+provenance+contract+MiniStack+CI+release
metadata); the budget is tight and needs the escape hatches recorded in `tasks.md`.

### Risks

- Hard dependency on `lineage-fixtures` landing first (layout, recorded root, oracle file names).
- Line budget: ~2.3k hand-written lines over three PRs; PR A is at the 800 limit.
- `object_store` `aws`/`gcp`/`azure` pull reqwest + rustls/aws-lc (C build, longer CI, extra
  licenses) and chrono/humantime advisories.
- Find fidelity edge cases (Java `Double.toString`, Groovy `toString` of maps/objects) — implement
  numbers/bools/strings exactly, document maps/objects as non-matching.
- Nextflow `master` (#8018) will emit other checksum modes → `UnsupportedChecksum` until added.
- MiniStack `HEAD` mtime precision differs from AWS; rely on listings only.

### Ready for Proposal

Yes. Tell the user: the S3 `check` open item is resolved (hash `/<bucket>/<key>` + listing mtime),
`provenance` goes into the library, and the change depends on `lineage-fixtures` being merged first.
