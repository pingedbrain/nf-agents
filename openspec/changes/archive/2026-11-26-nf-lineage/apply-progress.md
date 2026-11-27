# Apply progress: nf-lineage

**Mode**: Strict TDD (runner `cargo test --workspace`). Batch PR A: A.1–A.7 done; A.8–A.9
(FindQuery) moved to PR B by the tasks.md escape hatch (PR A was ≈ 940 lines without them).
Remaining work re-split into B1/B2/C1/C2 (tasks.md, 2026-11-26). Batch PR B1: B1.1–B1.7 done
(incl. A.8–A.9). Batch PR B2: B2.1–B2.5 done; `diff_raw` moved to C1.0 by the escape hatch
(B2 ≈ 905 lines with it, ≈ 870 without). Batch PR C1: C1.0–C1.4 done (`diff_raw`, `view`,
`provenance`, CLI contract suite, CI `contract` job; ≈ 787 lines, no escape hatch). Batch PR C2:
C2.1–C2.5 done (MiniStack S3 tests, CI `integration` job, CONTRIBUTING MiniStack section,
`0.1.0` metadata + crate README; ≈ 350 lines). All tasks complete.

## Files changed (PR A)

| File | Action | What |
|------|--------|------|
| `Cargo.toml` | Modified | `[workspace.dependencies]` (add-as-used): insta (`json`), regex, serde (`derive`), serde_json, thiserror |
| `crates/nf-lineage/Cargo.toml` | Modified | deps serde, serde_json (`preserve_order`), thiserror; dev insta, regex |
| `crates/nf-lineage/src/lib.rs` | Rewritten | crate docs, re-exports; stub test T-1 deleted |
| `crates/nf-lineage/src/error.rs` | Created | `LineageError` (all variants except `Storage`/`Io`) |
| `crates/nf-lineage/src/lid.rs` | Created | `Lid` parse/accessors/`record_key`, FromStr/Display/TryFrom/serde + 5 tests |
| `crates/nf-lineage/src/model.rs` | Created | 10 record structs, Nextflow field order, `extra` flatten |
| `crates/nf-lineage/src/record.rs` | Created | `Record::{parse, kind, spec, to_json}`, `Serialize` + 6 tests |
| `crates/nf-lineage/tests/common/mod.rs` | Created | fixture root / versions / store dir / record files |
| `crates/nf-lineage/tests/fixtures_parse.rs` | Created | `every_fixture_record_parses`, `fixture_snapshots` |
| `crates/nf-lineage/tests/snapshots/*.snap` | Generated | one per fixture version (5), reviewed: no host paths, volatile values masked |

## TDD cycle evidence

| Task | RED | GREEN | Triangulate | Refactor |
|------|-----|-------|-------------|----------|
| A.1/A.2 | structural (deps, error enum); triangulation skipped: no logic | builds, clippy clean | — | — |
| A.3/A.4 | lid tests written first → 15 compile errors (`Lid` missing) | `Lid` → 5/5 pass | 18 rejected forms incl. `\`, `?`, control chars, `#`, `a..b` fragment, 4 KiB, 129-hex hash; Ord check | none |
| A.5/A.6 | record tests written first → 13 compile errors (`Record`, model missing) | model + `Record` → 11/11 lib tests pass | flat vs spec equality, unknown kind flat + spec, 9 invalid inputs incl. 10 000 nested arrays | none |
| A.7 | `fixture_snapshots` failed (no snapshots); `every_fixture_record_parses` passed at once (parser from A.6 already handled all 111 records) | snapshots written with `INSTA_UPDATE=always`, reviewed, re-run green | mutation probes: `size: Option<String>` → both tests fail; weakened hash mask → snapshot test fails; both reverted | mask-then-sort for regeneration-stable order |
| B1.1/B1.2 (A.8/A.9) | query tests written first → 17 compile errors (`FindQuery`, `navigate`, `java_double` missing) | `FindQuery` → 4/4 pass, all 37 CLI cases | 9 error cases (incl. separator-before-property order, `%a` incomplete), alias/accumulate/`new` without decoding, empty query, unknown kind `type=`; mutation probes: all numbers as ints → `params.value=12` case fails; no falsy filter in `navigate` → table fails; both reverted | `PROPERTIES` as one whitespace-separated string; `Case` type alias |
| B1.3/B1.4 | structural (deps, features, error variants, deny policy); verified by `cargo deny check` (licenses failed on `webpki-root-certs` before the exception) | deny exit 0 | — | — |
| B1.5/B1.6 | store tests written first → 4 compile errors (`LineageStore`, `ObjectLineageStore`, `object_store` re-export missing) | `store.rs` → 3/3 pass on all 5 fixture versions | markers, nested history, `.history/x/.data.json`, other prefix, 4 malformed history lines, bad record in `find`, schemes/options; mutation probes: `Path::from` keys → 2 fail; history depth check removed → 1 fails; `.history` exclusion removed → 1 fails; all reverted | `find` uses a named async helper (closure hit the HRTB `FnOnce` error); `open` builds a `Result` store so feature-less builds have no unreachable code |
| B1.7 | doctest written with B1.6 (InMemory, `futures::executor::block_on`) | passes | — | — |
| B2.1 | structural (murmur3 0.5, tempfile 3 dev; similar added then removed with `diff_raw`) | deny/machete clean | — | — |
| B2.2/B2.3 | `check.rs` unit tests (`golden_vectors`, `object_stat_lists_the_parent_prefix`, `rejects_what_it_cannot_verify`, `path_map_remaps_on_component_boundaries`) + `tests/check.rs` `fixture_outcomes` written first → 21 compile errors (`check`, `CheckOptions`, `object_stat`, `floor_ms`, `local_path` missing) | `check` → 4/4 unit + fixture test pass; every FileOutput of the 4 core versions `Valid` on the mtime-restored temp copy | 4 golden vectors incl. empty path and pre-epoch mtime, floor of −1.4995 s → −1500; touch/append/delete/TaskRun; `gs`/`az`/`http`/bucket-only, 3 unsupported checksums, `STANDARD` accepted; mutation probes: BE hash bytes → golden fails; truncating pre-epoch ms → golden fails; first listed object instead of exact key → `object_stat` test fails; all reverted | none |
| B2.4/B2.5 | `diff.rs` unit test + `tests/diff.rs` (`minimal_runs_differ_only_in_greeting`, `task_pairs_and_kind_mismatch`, raw-diff test) written first → 10 compile errors (`diff_records`, `project`, `mask`, `diff`, `diff_raw` missing) | 1/1 unit + 3/3 integration pass on all 4 core versions | volatile `sessionId`/`workflowRun`/LID hashes ignored, `container` change found, unnamed input `#1`, unknown kind; mutation probes: no LID masking → unit fails; `sessionId` added to the WorkflowRun projection → minimal-runs test fails; keeping `null` survived at first → added a null-vs-absent assertion, then fails; all reverted | tables as whitespace-separated dot paths (−40 lines); `diff_raw` + its test moved to C1.0 |
| C1.0 | `raw_diff_has_cli_header_and_record_lines` written first → 1 compile error (`diff_raw` missing) | `similar` unified diff + CLI header → 3/3 diff tests | identical records → header only; first GREEN failed on a wrong test assertion (`kind` outside the hunk context) → assertion fixed to count context lines; mutation probe: key without `lid://` stripped → `contract_diff` header fails on every version; reverted | none |
| C1.1 | `tests/provenance.rs` (`published_file_reaches_pipeline_input`, `task_file_stops_at_its_task_and_runs_are_rejected`) written first → 3 compile errors (`provenance`, `Provenance`, `ProvenanceInput` missing) | BFS over `get` → 2/2 on all 4 core versions | `cycles_are_visited_once` (InMemory FileOutput ↔ TaskRun cycle); WorkflowRun → `UnexpectedKind`, missing → `NotFound`, task file → GENERATE only; mutation probes: no visited check → cycle test hangs (killed by timeout); no source/taskRun dedupe → cycle test fails; both reverted | none |
| C1.2/C1.3 | `tests/contract.rs` (5 contract tests) written first → compile error (`view` missing) | `view` → `contract_view` green at once; `contract_diff` failed on 25.04/25.10 `diff-tasks` (trailing `,` after `workflowRun`) → comparison drops a trailing `,` (deviation) → 5/5 | `tests/view.rs` edge cases (missing `#output` → `[]`, `#output.value`, array navigation, `#type`, `null`, flat record, unknown property, missing record); mutation probes: no output redirect → view-*output cases fail; `spec()` instead of `to_json()` → record views fail; `label` alias broken → find-fileoutput-label fails; no mtime bump → check-mtime-only fails; list columns swapped → list fails; no `#output`→`[]` mapping → view test fails; no fragment validation → view test fails; all reverted, each failure names `<version> <case>` | `changed_lines` un-escape as a table |
| C1.4 | structural (CI job); YAML parsed, `secrets.` count 0; triangulation skipped: no logic | — | — | — |
| C2.1 | `tests/s3_ministack.rs` written against the existing S3 code paths (store from B1, S3 `check` from B2); RED = new behaviour never exercised against a real S3 API; first run against `ministackorg/ministack:1.5.45` (port 14566) green | 2/2 pass with `NF_LINEAGE_MINISTACK_URL`; unset → skip notice, 2/2 pass | unreachable `http://localhost:1` → both FAIL after 60 s (not skip); mutation probes: S3 mtime truncated to seconds → `s3_check_valid_and_mismatch` fails (MiniStack listings keep ms); hashed S3 path without the bucket → fails; both reverted | none |
| C2.2/C2.3 | structural (CI job, docs); actionlint (docker `rhysd/actionlint`) 0 errors, `secrets.` count 0; V-doc greps 1/1/1; triangulation skipped: no logic | — | — | — |
| C2.4 | README examples added as `#[cfg(doctest)]` item | 2 README doctests compile + store doctest pass; `cargo package -p nf-lineage --locked` ok; `RUSTDOCFLAGS=-D warnings cargo doc` clean | mutation probe: `list_runz` in the README → doctest FAILED; reverted | `include` limits the package to `src/` + README |

## Files changed (PR B1)

| File | Action | What |
|------|--------|------|
| `Cargo.toml` | Modified | workspace deps async-trait, futures, object_store 0.14, tokio, url |
| `crates/nf-lineage/Cargo.toml` | Modified | features `aws`/`gcp`/`azure` (default); deps async-trait, futures, object_store, url; dev tokio |
| `crates/nf-lineage/src/error.rs` | Modified | `Storage(#[from] object_store::Error)`, `Io { path, source }` |
| `crates/nf-lineage/src/lib.rs` | Modified | docs, `FindQuery`/store re-exports, `pub use object_store` |
| `crates/nf-lineage/src/query.rs` | Created | `FindQuery::{new, parse_args, terms, matches}`, `navigate`, `validate_property`, `url_decode`, Java double strings + 4 tests |
| `crates/nf-lineage/src/store.rs` | Created | `LineageStore` (async-trait), `RunSummary`, `ObjectLineageStore::{new, open, open_with_options}`, history, `get`, `find` + doctest |
| `crates/nf-lineage/tests/store.rs` | Created | `local_store_lists_gets_and_finds`, `in_memory_store_ignores_directory_markers`, `store_errors_are_typed` |
| `crates/nf-lineage/tests/data/find_cli.json` | Generated | 28 findexp records + 37 recorded CLI cases, host paths scrubbed |
| `deny.toml` | Modified | RUSTSEC-2026-0542/0543 ignores with reasons; `webpki-root-certs` CDLA-Permissive-2.0 exception |

## Files changed (PR B2)

| File | Action | What |
|------|--------|------|
| `Cargo.toml` | Modified | workspace deps murmur3 0.5, tempfile 3 |
| `crates/nf-lineage/Cargo.toml` | Modified | dep murmur3; dev tempfile |
| `crates/nf-lineage/src/lib.rs` | Modified | docs; `check`/`diff` re-exports |
| `crates/nf-lineage/src/store.rs` | Modified | `ObjectLineageStore.store` → `pub(crate)` (S3 check reuses `open_with_options`) |
| `crates/nf-lineage/src/check.rs` | Created | `check`, `CheckOptions`, `CheckReport`, `CheckStatus`, `nextflow_standard_checksum`, local/`file://`/`s3://` stat, `path_map` + 4 tests |
| `crates/nf-lineage/src/diff.rs` | Created | `diff_records`, `diff` (tasks/outputs at run level), `SemanticDiff`, `Change`, LID masking + 1 test |
| `crates/nf-lineage/tests/common/mod.rs` | Modified | `fixture_copy` (temp copy, manifest mtimes restored); `#![allow(dead_code)]` |
| `crates/nf-lineage/tests/check.rs` | Created | `fixture_outcomes` |
| `crates/nf-lineage/tests/diff.rs` | Created | `minimal_runs_differ_only_in_greeting`, `task_pairs_and_kind_mismatch` |

## Files changed (PR C1)

| File | Action | What |
|------|--------|------|
| `Cargo.toml` | Modified | workspace dep similar 2 |
| `crates/nf-lineage/Cargo.toml` | Modified | dep similar |
| `crates/nf-lineage/src/diff.rs` | Modified | `diff_raw` (CLI header + `similar` unified diff of `to_json()`) |
| `crates/nf-lineage/src/view.rs` | Created | `view` (fragment validation, `#output` redirect, `#type`, navigate) |
| `crates/nf-lineage/src/provenance.rs` | Created | `provenance`, `Provenance`, `ProvenanceNode`, `ProvenanceInput` |
| `crates/nf-lineage/src/lib.rs` | Modified | docs; `view`, `provenance`, `diff_raw` re-exports |
| `crates/nf-lineage/tests/diff.rs` | Modified | `raw_diff_has_cli_header_and_record_lines` |
| `crates/nf-lineage/tests/provenance.rs` | Created | `published_file_reaches_pipeline_input`, `task_file_stops_at_its_task_and_runs_are_rejected`, `cycles_are_visited_once` |
| `crates/nf-lineage/tests/view.rs` | Created | `fragments_validate_redirect_and_navigate` |
| `crates/nf-lineage/tests/contract.rs` | Created | `contract_list`, `contract_view`, `contract_find`, `contract_diff`, `contract_check` |
| `.github/workflows/ci.yml` | Modified | job `contract` |

## Files changed (PR C2)

| File | Action | What |
|------|--------|------|
| `crates/nf-lineage/tests/s3_ministack.rs` | Created | env-gated `s3_store_roundtrip`, `s3_check_valid_and_mismatch`; raw `PUT /<bucket>` readiness wait (≤ 60 s) |
| `.github/workflows/ci.yml` | Modified | job `integration` (service `ministackorg/ministack:1.5.45`, `NF_LINEAGE_MINISTACK_URL`) |
| `CONTRIBUTING.md` | Modified | "S3 integration tests (MiniStack)" section |
| `crates/nf-lineage/Cargo.toml` | Modified | `0.1.0`, publishable; description, readme, documentation, keywords, categories, `include` |
| `crates/nf-lineage/README.md` | Created | overview, examples (doctests), features |
| `crates/nf-lineage/src/lib.rs` | Modified | `#[cfg(doctest)]` README include |
| `Cargo.lock` | Modified | nf-lineage version |

## Deviations

See design.md "Deviations (2026-11-26, apply PR A)": snapshot masking with `regex` before sorting
(insta `filters` dropped), FindQuery moved to PR B (with upstream notes for it), model structs
`#[non_exhaustive]` + `Default`.

PR B2: see design.md "Deviations (2026-11-26, apply PR B2)" (`diff_raw` → C1, check order and
scheme rules, JSON-table projection).

PR C1: see design.md "Deviations (2026-11-26, apply PR C1)" (`diff_raw` diffs the canonical
envelope for every version; contract diff drops a trailing `,`; view/provenance details).

PR C2: see design.md "Deviations (2026-11-26, apply PR C2)" (dummy credentials in the test, fixed
buckets + raw-PUT readiness, S3 check expectation from the listing, package `include`).

PR B1: see design.md "Deviations (2026-11-26, apply PR B1)" (re-split, no `Did you mean` suffix,
navigation over `spec()` JSON, history `tokenize`, deny warnings for undetected advisories).

## Remaining

None. All tasks (A–C2) complete; ready for verify and archive.
