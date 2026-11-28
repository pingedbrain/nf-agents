# baqueano-lineage Specification

## Purpose

Library crate that reads Nextflow data-lineage stores (`lineage/v1beta1`) typed and tolerantly, from local disk or object storage, with `nextflow lineage` parity for `list`/`view`/`find`, a semantic `diff`, `check` and `provenance`. Read-only. Fixtures: `fixtures/lineage/<ver>/` from change `lineage-fixtures` (recorded root `/nf-agents-fixture`, `oracle/<case>.{args,out,err,exit}`, `manifest.json`).

## Verification checks

Unit tests live in `crates/baqueano-lineage/src/<module>.rs` (`<module>::tests::<name>`); integration tests in `crates/baqueano-lineage/tests/<file>.rs` (`<file>::<name>`).

| ID | Kind | Command / name |
|----|------|----------------|
| C-test | CI `test` | `cargo test --workspace --locked` (all T-* below) |
| C-contract | CI `contract` | `cargo test -p baqueano-lineage --test contract --locked` |
| C-it | CI `integration` | `cargo test -p baqueano-lineage --test s3_ministack --locked` with MiniStack |
| C-doc | CI `test` | doctests of `baqueano-lineage` |
| V-pkg | verify | `cargo package -p baqueano-lineage --locked` succeeds; metadata inspected |
| V-grep | verify | grep the crate for `Path::from(`, `set_var`, `std::process::exit` → 0 hits in `src/` |

## Requirements

### Requirement: Lineage IDs

`Lid::parse` MUST accept `lid://<hash>[/<seg>…][#<prop>[.<prop>…]]` where `hash` is 1–128 lowercase hex chars, path segments are non-empty, not `.`/`..`, without `\`, `#`, `?` or control chars, and fragment segments match `[A-Za-z0-9_]+`. Anything else (missing `lid://` prefix, empty hash, `lid:///`, query strings, uppercase hex, trailing `/`, > 4096 bytes) MUST return `InvalidLid` naming the input. `Display` MUST round-trip. `record_key()` MUST map `lid://h#output` to `h#output`, `lid://h/p#frag` to `h/p`. `Lid` MUST (de)serialize as a string.

#### Scenario: Valid forms (T `lid::tests::parses_run_task_and_file_lids`, `parses_output_fragment`)
- GIVEN `lid://6d6d…44`, `lid://6d6d…44/upper/upper.txt`, `lid://82d8…1a#output`
- WHEN parsed
- THEN hash/path/fragment are split and `to_string()` equals the input

#### Scenario: Rejected forms (T `lid::tests::rejects_invalid_lids`)
- GIVEN `foo`, `lid://`, `lid:///`, `lid://ABC`, `lid://h/../x`, `lid://h/a//b`, `lid://h?x=1`, `lid://h/`
- WHEN parsed
- THEN each is `Err(InvalidLid)`; no panic

#### Scenario: Store key (T `lid::tests::record_key_maps_output_fragment`)
- GIVEN `lid://h#output` and `lid://h#params`
- THEN keys are `h#output` and `h`

### Requirement: Record parsing

`Record::parse(bytes, origin)` MUST: require `version == "lineage/v1beta1"` (other string → `UnsupportedVersion` whose message contains the version and origin; missing/non-string → `InvalidFormat`); require a string `kind`; read fields from `spec` when present, else from the root (flat, 25.04); map `WorkflowRun`, `WorkflowOutput`, `TaskRun`, `TaskOutput`, `FileOutput`, `AgentRun` to typed variants and any other kind to `Unknown(raw)`. Every known field MUST be optional/nullable; unknown fields MUST be kept in `extra` and re-emitted by `to_json()`, which MUST produce the `{version, kind, spec}` envelope. A known field of the wrong JSON type, malformed JSON or a non-object MUST yield `InvalidFormat { path: origin, message: <serde message> }`. Parsing MUST NOT panic on any input.

#### Scenario: Both envelopes (T `record::tests::parses_flat_envelope`, `parses_spec_envelope`)
- GIVEN a 25.04 flat TaskRun and a 26.10 `spec` TaskRun with equal fields
- THEN both parse to equal `Record::TaskRun`

#### Scenario: Unknown field kept (T `record::tests::unknown_fields_are_preserved_and_reserialized`)
- GIVEN a FileOutput with `"futureField": {"x": 1}`
- THEN `extra["futureField"]` holds it and `to_json()["spec"]["futureField"]` equals it

#### Scenario: Unknown kind (T `record::tests::unknown_kind_is_preserved`)
- GIVEN `kind: "DatasetRun"`
- THEN `Record::Unknown` with the raw JSON and `kind()` = `DatasetRun`

#### Scenario: Unsupported version (T `record::tests::unsupported_version_names_version`)
- GIVEN `version: "lineage/v2"`
- THEN `Err(UnsupportedVersion)` and `to_string()` contains `lineage/v2`

#### Scenario: Bad input (T `record::tests::invalid_inputs_are_invalid_format`)
- GIVEN empty bytes, `[]`, `{"version":…}` without kind, `size: "nine"`, 10 000 nested arrays
- THEN `Err(InvalidFormat)` with origin and serde message; no panic

#### Scenario: All fixtures (T `fixtures_parse::every_fixture_record_parses`, `fixtures_parse::fixture_snapshots`)
- GIVEN every `.data.json` under `fixtures/lineage/*/.lineage`
- THEN each parses to a known variant (AgentRun included) and the per-version insta snapshot (hashes, UUIDs, timestamps, run names filtered) matches

### Requirement: Find queries

`FindQuery::parse_args` MUST follow `nextflow lineage find`: `k=v` (no `=` → `InvalidQuery` "Parameter <arg> doesn't contain '=' separator"); URL-decode key and value (`+`→space, `%XX`; malformed → `InvalidQuery`); `label`→`labels`; repeated keys accumulate; each dot segment of a key MUST be a model property (else `InvalidQuery` "Property '<seg>' doesn't exist in the lineage model."). `FindQuery::new(pairs)` MUST apply alias and validation without decoding. `matches(record)` MUST: treat key `type` as the record kind (case-sensitive); navigate dot paths flattening arrays (falsy items dropped, single result unwrapped); never match falsy values (null, false, 0, "", empty array/object); match arrays when they contain all expected values; match scalars only with one expected value by string equality; stringify numbers as integers for `size`/`maxIterations` and as Java `Double.toString` elsewhere.

#### Scenario: CLI behaviour table (T `query::tests::matches_cli_semantics_table`)
- GIVEN the 26.10.7 records and the 37 non-error queries recorded from the CLI (e.g. `params.value=12.0`, `size=0`, `labels=summary labels=other`, `input.value.path=…`), stored with host paths scrubbed in `crates/baqueano-lineage/tests/data/find_cli.json`
- THEN `matches` selects exactly the CLI's LID set

#### Scenario: CLI errors (T `query::tests::parse_args_errors_match_cli_messages`)
- GIVEN `type`, `foo=bar`, `config.runName=x`, `name=%zz`
- THEN `InvalidQuery` whose message equals the CLI text after `Error searching for <arg>. `, up to (excluding) any ` -- Did you mean one of these?  <names>` suffix
- *Deviation (2026-11-26, apply PR B1):* the CLI's Levenshtein suggestion suffix (Groovy `closest`) is not ported in 0.1; messages match up to that suffix.

### Requirement: Store contract

`LineageStore` MUST be dyn-compatible (`#[async_trait]`, `Send + Sync`) with `list_runs`, `get`, `find`. `list_runs` MUST read `.history/<one segment>` entries (4 tab-separated non-empty tokens, trailing whitespace ignored), sort by timestamp text then LID, return `[]` when `.history` is absent, and `InvalidFormat` naming the entry when malformed. `get` MUST read `<prefix>/<record_key>/.data.json` (`NotFound` naming the LID if absent). `find` MUST evaluate every `…/.data.json` record (excluding `.history`) and return matching LIDs sorted and de-duplicated; a record that fails to parse MUST fail the call with its error. An object whose key has no valid LID (a `#` in a path segment, a `?`, `\`, control character or other segment `Lid::parse` rejects) MUST be skipped with a `tracing::warn!` naming the key, and the listing MUST continue; only the `<hash>#output` key is a `#` key.

#### Scenario: Local fixtures (T `store::local_store_lists_gets_and_finds`)
- GIVEN `ObjectLineageStore::open("<fixture>/.lineage")` for each version
- THEN `list_runs` has 2 runs, `get` of a `#output` LID returns a TaskOutput, `find(type=WorkflowRun)` returns both runs

#### Scenario: Unrepresentable keys are skipped (T `store::find_skips_unrepresentable_objects`, `store::find_keeps_listing_when_local_objects_are_unrepresentable`)
- GIVEN records under keys `<h>/a#b/.data.json` and `<h>/c?d/.data.json` beside `<h>/.data.json`, and a local fixture copy with `ffff/odd#x` and `ffff/odd?x` objects
- THEN `find` returns only the valid LIDs, unchanged from the fixture result, with no error

#### Scenario: Directory markers (T `store::in_memory_store_ignores_directory_markers`)
- GIVEN an InMemory store with zero-byte `store`, `store/.history`, `store/<h>#output` objects plus records
- THEN listings ignore the markers

#### Scenario: Errors (T `store::store_errors_are_typed`)
- GIVEN a missing LID, a malformed history file, an unknown URL scheme, a missing local dir, an unknown option key
- THEN `NotFound`, `InvalidFormat`, `Unsupported`, `NotFound`, `Unsupported` respectively

### Requirement: Store construction

`ObjectLineageStore::open(location)` MUST accept a local path, `file://`, `s3://bucket[/prefix]`, `gs://…`, `az://…`/`abfs[s]://…` (cloud schemes behind default-on features `aws`, `gcp`, `azure`), reading cloud settings from the environment (`AWS_ENDPOINT_URL`, `AWS_ALLOW_HTTP`, credentials, …). `open_with_options(location, pairs)` MUST layer explicit `object_store` config keys over the environment. Object keys MUST be built with `Path::parse` (never `Path::from`). The crate MUST NOT write, delete or create objects.

#### Scenario: S3 on MiniStack (C-it `s3_ministack::s3_store_roundtrip`)
- GIVEN the 26.10.7 fixture uploaded to MiniStack and `BAQUEANO_LINEAGE_MINISTACK_URL` set
- THEN `open_with_options("s3://…/store", endpoint opts)` lists, gets a `#output` record and finds as locally

#### Scenario: No MiniStack (C-test)
- GIVEN `BAQUEANO_LINEAGE_MINISTACK_URL` unset
- THEN the integration tests print a skip notice and pass

### Requirement: Check

`check(store, lid, opts)` MUST load the record (`UnexpectedKind` unless FileOutput), and: no checksum → `NoChecksum`; algorithm ≠ `nextflow` or mode ∉ {absent, `standard` (any case)} → `UnsupportedChecksum`; otherwise compute murmur3_x64_128 (seed 0, hex of `u128` LE bytes) over UTF-16LE of the hashed path + LE i64 size + LE i64 floor(mtime ms). Local paths (plain or `file://`, after `opts.path_map` prefix remap) hash the recorded path and stat the file following symlinks (a recorded local path with a `..` component MUST be refused with `Unsupported` before `path_map` applies, T `check::tests::rejects_parent_components_before_path_map`); `s3://b/k` hashes `/b/k` and takes size/mtime from a listing of the parent prefix; other schemes → `Unsupported`. Missing file → `FileMissing`. Result: `Valid` or `Mismatch` with expected/actual.

#### Scenario: Golden vectors (T `check::tests::golden_vectors`)
- GIVEN (`/lineage/results/summary.txt`, 9, 1795726937562)
- THEN `c47bf938e54f24686b01873ab8d1ba53` (Nextflow 26.10.7 on S3)

#### Scenario: Fixtures (T `check::fixture_outcomes`)
- GIVEN a temp copy with mtimes restored from `manifest.json` and `path_map` `/nf-agents-fixture` → copy
- THEN every FileOutput is `Valid`; after `set_modified(now)` → `Mismatch`; after appending a byte → `Mismatch`; after delete → `FileMissing`; a TaskRun LID → `UnexpectedKind`

#### Scenario: S3 data (C-it `s3_ministack::s3_check_valid_and_mismatch`)
- GIVEN an S3 object and a FileOutput built from its listing
- THEN `Valid`; after rewriting the object, `Mismatch`

### Requirement: Diff

`diff_records(a, b)` MUST return changes over a whitelist projection — WorkflowRun: `params.<name>`, `workflow.scriptFiles.<path>`, `workflow.repository`, `workflow.commitId`, `nextflow.version`; TaskRun/AgentRun: identity, code, tool and input fields; outputs/files: outputs, size, labels — with `lid://<hash>` masked, never reporting `sessionId`, run names, timestamps, `commandLine`, `outputDir`, `workflowRun`, `taskRun`. Different kinds → one `kind` change. `diff(store, a, b)` MUST add, for two WorkflowRuns, `tasks.<name>.<field>` and `outputs.<name>`. `diff_raw` MUST return `diff --git k1 k2` / `--- k1` / `+++ k2` plus a unified diff of the pretty-printed records.

#### Scenario: Minimal runs (T `diff::minimal_runs_differ_only_in_greeting`)
- GIVEN RUN1 and RUN2 of every core fixture
- THEN `diff` has exactly one change `params.greeting` `hello`→`bonjour`

#### Scenario: Raw parity (C-contract `contract::contract_diff`)
- GIVEN oracle `diff-runs`, `diff-tasks`
- THEN header equals and the multisets of trimmed `+`/`-` lines (trailing `,` dropped) equal after Gson-unescaping

### Requirement: View and provenance

`view(store, lid)` MUST return the canonical record JSON, or for a fragment validate segments like `find`, redirect `output` on WorkflowRun/TaskRun/AgentRun to the `#output` record (`[]` when absent) and navigate (`null` when nothing). `provenance(store, file_lid)` MUST walk FileOutput → source/taskRun → TaskRun/AgentRun → inputs breadth-first without revisiting, classifying inputs as `Lineage`, `External` (DataPath) or `Value`; non-FileOutput → `UnexpectedKind`.

#### Scenario: Reaches pipeline input (T `provenance::published_file_reaches_pipeline_input`)
- GIVEN RUN1 `summary.txt` of each core fixture
- THEN nodes include SUMMARIZE, TRANSFORM, GENERATE and an `External` input ending `seed.txt`

#### Scenario: View parity (C-contract `contract::contract_view`)
- GIVEN every `view-*` oracle case
- THEN results equal the parsed oracle JSON after normalization (envelope, numbers as f64, null ≡ absent), and error cases map to `NotFound`/`InvalidLid`/`InvalidQuery`

### Requirement: CLI contract

For every fixture version, `list`, `find-*`, `check-*` oracle cases MUST agree: list rows equal; find LID sets equal (JSON array or lines); error stdout contains our message; check exit 0 ↔ `Valid`, `check-tampered`/`check-mtime-only` reproduced on a temp copy ↔ `Mismatch`, `check-taskrun` ↔ `UnexpectedKind`.

#### Scenario: All versions (C-contract `contract::contract_list`, `contract_find`, `contract_check`)
- GIVEN each `fixtures/lineage/*/oracle`
- THEN all cases pass; a failure names version and case

### Requirement: Safety and packaging

No `unwrap`/`expect`/`panic`/indexing in `src/`; errors are `thiserror` `LineageError` (`#[non_exhaustive]`). Crate version `0.1.0`, publishable, with description, license, repository, README, keywords, categories and `docs.rs`-ready docs; every public item documented; examples compile as doctests.

#### Scenario: Package (V-pkg, C-doc, V-grep)
- GIVEN the crate
- THEN `cargo package` succeeds, doctests pass, V-grep finds nothing
