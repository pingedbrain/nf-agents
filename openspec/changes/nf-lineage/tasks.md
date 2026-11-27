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
- [ ] A.8 RED `src/query.rs` tests `matches_cli_semantics_table` (findexp table on synthetic records, no host paths), `parse_args_errors_match_cli_messages`.
- [ ] A.9 GREEN `FindQuery::{new, parse_args, matches, terms}`, `navigate`, `validate_property`, `url_decode`, Java double strings.
- Escape hatch: if > 800 lines, move A.8–A.9 to the start of PR B.
- **Deviation (2026-11-26, apply PR A):** escape hatch applied. A.1–A.7 alone came to ≈ 940 hand-written lines (model docs are ~110 of them), so A.8–A.9 (FindQuery) move to the start of PR B. PR B then needs its own escape hatch (`diff_raw` → PR C) and likely `diff` as a whole → PR C to stay near 800.

## PR B — LineageStore over object_store + check + diff · est. ≈ 775 lines (+ ≈ 250 for A.8–A.9, moved here)

- [ ] A.8 / A.9 (moved from PR A, do first) — FindQuery, see PR A.

- [ ] B.1 `Cargo.toml`: async-trait, object_store 0.14, futures, url, murmur3 0.5, similar 2; features `aws`/`gcp`/`azure` (default); dev tokio, tempfile. `LineageError::{Storage, Io}`.
- [ ] B.2 `deny.toml`: advisory ignores RUSTSEC-2026-0542/0543 with `reason`; crate-scoped license exceptions only for what `cargo deny check` reports (expected `webpki-root-certs` CDLA-Permissive-2.0).
- [ ] B.3 RED `tests/store.rs`: `local_store_lists_gets_and_finds` (all versions), `in_memory_store_ignores_directory_markers`, `store_errors_are_typed`, a `Box<dyn LineageStore>` use.
- [ ] B.4 GREEN `src/store.rs`: trait, `RunSummary`, `ObjectLineageStore::{new, open, open_with_options}`, history parsing, `get`, `find` (`buffer_unordered(16)`), `Path::parse` only.
- [ ] B.5 RED `src/check.rs` `golden_vectors` (`/lineage/results/summary.txt`,9,1795726937562 → `c47bf938e54f24686b01873ab8d1ba53`; `/nf-agents-fixture/results-hello/summary.txt`,9,1795726427044 → `87105ea50d9e8dca5da368e47d34d9bc`; `""`,0,0 → `d618a97df21bbd4bb61c79cdeca965b4`; `/data/ünïcode 🧬.txt`,35,-1500 → `9ad8de60441a469eb4b8c0e1bca74c1a`); `tests/check.rs` `fixture_outcomes`.
- [ ] B.6 GREEN `check`, `CheckOptions`, `CheckReport`, `CheckStatus`, `nextflow_standard_checksum`, local stat + `path_map`, S3 stat via `list_with_delimiter`, `UnsupportedChecksum`/`Unsupported`.
- [ ] B.7 RED `tests/diff.rs` `minimal_runs_differ_only_in_greeting` (core versions), task-pair and kind-mismatch cases; unit projection test.
- [ ] B.8 GREEN `src/diff.rs`: `diff_records`, `diff`, `diff_raw` (`similar`, CLI header).
- [ ] B.9 Doctest on `ObjectLineageStore::new` with InMemory.
- Escape hatch: if > 800, move `diff_raw` (+ its test) to PR C.

## PR C — view, provenance, contract, MiniStack, CI, release-ready · est. ≈ 690 lines

- [ ] C.1 RED `tests/provenance.rs` `published_file_reaches_pipeline_input`; GREEN `src/provenance.rs`.
- [ ] C.2 GREEN `src/view.rs` (fragment validation, `#output` redirect, navigate) driven by C.3 view cases.
- [ ] C.3 RED→GREEN `tests/contract.rs`: oracle loader (`.args` JSON), temp copy + manifest mtime restore, normalizers; `contract_list`, `contract_view`, `contract_find`, `contract_diff`, `contract_check`.
- [ ] C.4 `tests/s3_ministack.rs`: skip when `NF_LINEAGE_MINISTACK_URL` unset; readiness wait (raw TCP `PUT` bucket, ≤ 60 s); `s3_store_roundtrip`, `s3_check_valid_and_mismatch`. Run locally against `ministackorg/ministack:1.5.45`.
- [ ] C.5 `.github/workflows/ci.yml`: jobs `contract`, `integration` (service container, dummy creds, no secrets).
- [ ] C.6 `CONTRIBUTING.md` MiniStack section (contributor-guide delta greps).
- [ ] C.7 `Cargo.toml` `0.1.0`, drop `publish = false`, add description/readme/keywords/categories/docs; `README.md` with example; crate docs complete; `cargo package -p nf-lineage --locked` (V-pkg), V-grep.
- [ ] C.8 Verify every spec scenario maps to a passing test; diff ≤ 800 lines.
