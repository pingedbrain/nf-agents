# Apply progress: nf-lineage

**Mode**: Strict TDD (runner `cargo test --workspace`). Batch PR A: A.1–A.7 done; A.8–A.9
(FindQuery) moved to PR B by the tasks.md escape hatch (PR A was ≈ 940 lines without them).

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

## Deviations

See design.md "Deviations (2026-11-26, apply PR A)": snapshot masking with `regex` before sorting
(insta `filters` dropped), FindQuery moved to PR B (with upstream notes for it), model structs
`#[non_exhaustive]` + `Default`.

## Remaining

A.8–A.9 (now first in PR B), B.1–B.9, C.1–C.8.
