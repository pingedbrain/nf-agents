# Apply progress: nf-lineage

**Mode**: Strict TDD (runner `cargo test --workspace`). Batch PR A: A.1–A.7 done; A.8–A.9
(FindQuery) moved to PR B by the tasks.md escape hatch (PR A was ≈ 940 lines without them).
Remaining work re-split into B1/B2/C1/C2 (tasks.md, 2026-11-26). Batch PR B1: B1.1–B1.7 done
(incl. A.8–A.9).

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

## Deviations

See design.md "Deviations (2026-11-26, apply PR A)": snapshot masking with `regex` before sorting
(insta `filters` dropped), FindQuery moved to PR B (with upstream notes for it), model structs
`#[non_exhaustive]` + `Default`.

PR B1: see design.md "Deviations (2026-11-26, apply PR B1)" (re-split, no `Did you mean` suffix,
navigation over `spec()` JSON, history `tokenize`, deny warnings for undetected advisories).

## Remaining

B2.1–B2.5 (check + diff), C1.1–C1.4 (view, provenance, contract + CI job), C2.1–C2.5 (MiniStack,
integration CI, CONTRIBUTING, 0.1.0 metadata).
