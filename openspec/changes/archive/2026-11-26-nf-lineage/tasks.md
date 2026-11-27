# Tasks: nf-lineage 0.1

Strict TDD (RED → GREEN). Test names/IDs from `specs/nf-lineage/spec.md`. Paths relative to `crates/nf-lineage/` unless rooted. Each PR ends with `cargo fmt --all`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check`, `cargo machete` green. Precondition: `lineage-fixtures` merged (`fixtures/lineage/<ver>/`).

## PR A — types + parser + FindQuery · est. ≈ 790 lines

- [x] A.1 `Cargo.toml`: add serde (derive), serde_json (`preserve_order`), thiserror 2; dev insta (`json`,`filters`). Delete stub test T-1.
- [x] A.2 `src/error.rs`: `LineageError` variants except `Storage`/`Io`; `src/lib.rs` crate docs + re-exports.
- [x] A.3 RED `src/lid.rs` tests `parses_run_task_and_file_lids`, `parses_output_fragment`, `rejects_invalid_lids`, `record_key_maps_output_fragment`, serde-as-string.
- [x] A.4 GREEN `Lid` (parse, accessors, `record_key`, FromStr/Display/TryFrom/serde).
- [x] A.5 RED `src/record.rs` tests: `parses_flat_envelope`, `parses_spec_envelope`, `unknown_kind_is_preserved`, `unknown_fields_are_preserved_and_reserialized`, `unsupported_version_names_version`, `invalid_inputs_are_invalid_format`.
- [x] A.6 GREEN `src/model.rs` (10 structs, documented fields, Nextflow field order) + `Record::{parse, kind, spec, to_json}`, `Serialize`.
- [x] A.7 RED `tests/common/mod.rs` (fixture root/versions helpers) + `tests/fixtures_parse.rs` `every_fixture_record_parses`, `fixture_snapshots`; GREEN by fixing parser; `cargo insta accept` after review.
- [x] A.8 (→ B1.1) RED `src/query.rs` tests `matches_cli_semantics_table` (findexp table on synthetic records, no host paths), `parse_args_errors_match_cli_messages`.
- [x] A.9 (→ B1.2) GREEN `FindQuery::{new, parse_args, matches, terms}`, `navigate`, `validate_property`, `url_decode`, Java double strings.
- Escape hatch: if > 800 lines, move A.8–A.9 to the start of PR B.
- **Deviation (2026-11-26, apply PR A):** escape hatch applied. A.1–A.7 alone came to ≈ 940 hand-written lines (model docs are ~110 of them), so A.8–A.9 (FindQuery) move to the start of PR B. PR B then needs its own escape hatch (`diff_raw` → PR C) and likely `diff` as a whole → PR C to stay near 800.

## Re-split of the remaining work (Deviation 2026-11-26, before PR B1)

The remaining scope (FindQuery moved in from A, store, check, diff, diff_raw, view, provenance,
contract tests, MiniStack test, CI jobs, release metadata) is ≈ 1,700 hand-written lines, too much
for B + C at ≤ ~800 each. It is re-batched into four PRs along the module seams; the old task IDs
are kept in brackets for traceability. `deny.toml` moves into B1 because `object_store` (and with it
the `chrono`/`humantime` advisories and the cloud licenses) arrives in B1.

## PR B1 — FindQuery + LineageStore + ObjectLineageStore · est. ≈ 650 lines

- [x] B1.1 [A.8] RED `src/query.rs` tests `matches_cli_semantics_table` (CLI table recorded from the 26.10.7 `findexp` store, host paths scrubbed, in `tests/data/find_cli.json`), `parse_args_errors_match_cli_messages`, Java double strings.
- [x] B1.2 [A.9] GREEN `FindQuery::{new, parse_args, matches, terms}`, `navigate`, `validate_property`, `url_decode`, Java double strings.
- [x] B1.3 [B.1 part] `Cargo.toml`: async-trait, object_store 0.14, futures, url; features `aws`/`gcp`/`azure` (default); dev tokio. `LineageError::{Storage, Io}`.
- [x] B1.4 [B.2] `deny.toml`: advisory ignores RUSTSEC-2026-0542/0543 with `reason`; crate-scoped license exceptions only for what `cargo deny check` reports.
- [x] B1.5 [B.3] RED `tests/store.rs`: `local_store_lists_gets_and_finds` (all versions), `in_memory_store_ignores_directory_markers`, `store_errors_are_typed`, a `Box<dyn LineageStore>` use.
- [x] B1.6 [B.4] GREEN `src/store.rs`: trait, `RunSummary`, `ObjectLineageStore::{new, open, open_with_options}`, history parsing, `get`, `find` (`buffer_unordered(16)`), `Path::parse` only.
- [x] B1.7 [B.9] Doctest on `ObjectLineageStore::new` with InMemory.

## PR B2 — check + diff · est. ≈ 650 lines

- [x] B2.1 [B.1 rest] `Cargo.toml`: murmur3 0.5, similar 2 (→ C1 with `diff_raw`); dev tempfile. `deny.toml` only if `cargo deny check` reports something new.
- [x] B2.2 [B.5] RED `src/check.rs` `golden_vectors` (`/lineage/results/summary.txt`,9,1795726937562 → `c47bf938e54f24686b01873ab8d1ba53`; `/nf-agents-fixture/results-hello/summary.txt`,9,1795726427044 → `87105ea50d9e8dca5da368e47d34d9bc`; `""`,0,0 → `d618a97df21bbd4bb61c79cdeca965b4`; `/data/ünïcode 🧬.txt`,35,-1500 → `9ad8de60441a469eb4b8c0e1bca74c1a`); `tests/check.rs` `fixture_outcomes`.
- [x] B2.3 [B.6] GREEN `check`, `CheckOptions`, `CheckReport`, `CheckStatus`, `nextflow_standard_checksum`, local stat + `path_map`, S3 stat via `list_with_delimiter`, `UnsupportedChecksum`/`Unsupported`.
- [x] B2.4 [B.7] RED `tests/diff.rs` `minimal_runs_differ_only_in_greeting` (core versions), task-pair and kind-mismatch cases; unit projection test.
- [x] B2.5 [B.8] GREEN `src/diff.rs`: `diff_records`, `diff` (`diff_raw` → C1.0).
- Escape hatch: if > 800, move `diff_raw` (+ its test) to C1.
- **Deviation (2026-11-26, apply PR B2):** escape hatch applied. With `diff_raw` B2 came to ≈ 905 hand-written lines; without it ≈ 870. `diff_raw` (`similar 2`, CLI `diff --git` header, unified diff of the pretty-printed records, test `raw_diff_has_cli_header_and_record_lines`) moves to C1.0 next to `contract_diff`, which checks it against the oracle anyway.

## PR C1 — view, provenance, CLI contract suite · est. ≈ 640 lines

- [x] C1.0 [B2.5 part] RED `tests/diff.rs` `raw_diff_has_cli_header_and_record_lines`; GREEN `diff_raw` (`similar 2` dep, CLI header `diff --git k1 k2`/`--- k1`/`+++ k2`, `TextDiff::from_lines(..).unified_diff()`). Reuse `tests/common::fixture_copy` (temp copy + manifest mtime restore) in C1.3.

- [x] C1.1 [C.1] RED `tests/provenance.rs` `published_file_reaches_pipeline_input`; GREEN `src/provenance.rs`.
- [x] C1.2 [C.2] GREEN `src/view.rs` (fragment validation, `#output` redirect, navigate) driven by C1.3 view cases.
- [x] C1.3 [C.3] RED→GREEN `tests/contract.rs`: oracle loader (`.args` JSON), temp copy + manifest mtime restore, normalizers; `contract_list`, `contract_view`, `contract_find`, `contract_diff`, `contract_check`.
- [x] C1.4 [C.5 part] `.github/workflows/ci.yml`: job `contract`.
- **Deviation (2026-11-26, apply PR C1):** no escape hatch needed (≈ 790 hand-written lines incl. `tests/view.rs`, added to triangulate view edge cases the oracle lacks, and a provenance cycle test). Raw-diff parity compares changed lines with a trailing `,` trimmed (see design.md "Deviations (2026-11-26, apply PR C1)").

## PR C2 — MiniStack, integration CI, release-ready · est. ≈ 450 lines

- [x] C2.1 [C.4] `tests/s3_ministack.rs`: skip when `NF_LINEAGE_MINISTACK_URL` unset; readiness wait (raw TCP `PUT` bucket, ≤ 60 s); `s3_store_roundtrip`, `s3_check_valid_and_mismatch`. Run locally against `ministackorg/ministack:1.5.45`.
- [x] C2.2 [C.5 part] `.github/workflows/ci.yml`: job `integration` (service container, no secrets; dummy creds in the test).
- [x] C2.3 [C.6] `CONTRIBUTING.md` MiniStack section (contributor-guide delta greps).
- [x] C2.4 [C.7] `Cargo.toml` `0.1.0`, drop `publish = false`, add description/readme/keywords/categories/docs; `README.md` with example; crate docs complete; `cargo package -p nf-lineage --locked` (V-pkg), V-grep.
- [x] C2.5 [C.8] Verify every spec scenario maps to a passing test; each PR diff ≤ 800 lines.
- **Deviation (2026-11-26, apply PR C2):** no escape hatch needed (≈ 350 hand-written lines). Dummy
  credentials are fixed in the test instead of the workflow; the package ships only `src/` and the
  README (see design.md "Deviations (2026-11-26, apply PR C2)").
