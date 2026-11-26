# Verification Report: nf-lineage — PR A

**Mode**: Strict TDD. **Scope**: tasks A.1–A.7 (A.8–A.9 moved to PR B, see tasks.md deviation).
**Date**: 2026-11-26.

## Completeness

| Metric | Value |
|--------|-------|
| PR A tasks in scope | 7 (A.1–A.7) |
| Complete | 7 |
| Moved to PR B | A.8, A.9 (FindQuery; escape hatch) |

## Execution

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | ✅ |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | ✅ |
| `cargo test --workspace --locked` | ✅ 13 passed (11 unit, 2 integration), 0 failed, 0 skipped |
| `cargo deny check` | ✅ advisories, bans, licenses, sources ok |
| `cargo machete` | ✅ no unused deps |
| V-grep (`Path::from(`, `set_var`, `process::exit` in `src/`) | ✅ 0 hits |
| Coverage | ➖ not available (no coverage tool configured) |

## Spec compliance (PR A scenarios)

| Requirement | Scenario | Test | Result |
|-------------|----------|------|--------|
| Lineage IDs | Valid forms | `lid::tests::parses_run_task_and_file_lids`, `parses_output_fragment` | ✅ |
| Lineage IDs | Rejected forms | `lid::tests::rejects_invalid_lids` (all 8 spec forms + 10 more) | ✅ |
| Lineage IDs | Store key | `lid::tests::record_key_maps_output_fragment` | ✅ |
| Lineage IDs | (de)serialize as string | `lid::tests::serializes_as_string` | ✅ |
| Record parsing | Both envelopes | `record::tests::parses_flat_envelope`, `parses_spec_envelope` | ✅ |
| Record parsing | Unknown field kept | `record::tests::unknown_fields_are_preserved_and_reserialized` | ✅ |
| Record parsing | Unknown kind | `record::tests::unknown_kind_is_preserved` | ✅ |
| Record parsing | Unsupported version | `record::tests::unsupported_version_names_version` | ✅ |
| Record parsing | Bad input | `record::tests::invalid_inputs_are_invalid_format` | ✅ |
| Record parsing | All fixtures | `fixtures_parse::every_fixture_record_parses`, `fixture_snapshots` (5 stores, 111 records, AgentRun included) | ✅ |
| Find queries | CLI table / CLI errors | — | ➖ moved to PR B |
| rust-workspace delta | stub T-1 removed | `lib.rs` | ✅ |

10/10 in-scope scenarios compliant.

## TDD compliance

Evidence table present in apply-progress.md. RED observed as compile failures for A.3 and A.5
(tests written before `Lid`/`Record`/model). `every_fixture_record_parses` passed on first run
(A.6 parser already sufficient); its strength was shown by a mutation probe (`size` typed as
string → both fixture tests fail). Snapshot masking also mutation-probed.

## Coherence (design)

| Decision | Followed? | Notes |
|----------|-----------|-------|
| Typed structs, all `Option`, `extra` flatten, Nextflow order | ✅ | + `#[non_exhaustive]`, `Default` |
| Hand-written envelope step | ✅ | `Value`→version→kind→spec-or-root→`from_value` |
| `preserve_order` | ✅ | envelope key order tested |
| Snapshots via insta `filters` | ⚠️ Deviated | masked with `regex` before sorting (stable order); documented |
| FindQuery in PR A | ⚠️ Deviated | escape hatch → PR B; documented |

## Issues

- **CRITICAL**: none.
- **WARNING**: PR A is ≈ 940 hand-written lines (> ~800 budget) even without FindQuery; most of the
  excess is required field docs in `model.rs` (`missing_docs`). PR B grows by ≈ 250 (FindQuery)
  and should apply its own escape hatch early.
- **SUGGESTION**: FindQuery's "Did you mean" suffix: decide in PR B whether to port Nextflow's
  `closest` or match the CLI text up to it (spec scenario says "equals").

## Verdict

PASS WITH WARNINGS — PR A scope green on all gates; over the line budget.

---

# Verification Report: nf-lineage — PR B1

**Mode**: Strict TDD. **Scope**: B1.1–B1.7 (FindQuery incl. former A.8–A.9, LineageStore,
ObjectLineageStore, deny.toml). **Date**: 2026-11-26.

## Completeness

| Metric | Value |
|--------|-------|
| PR B1 tasks in scope | 7 (B1.1–B1.7) + A.8/A.9 (same work) |
| Complete | 7 |
| Remaining (later PRs) | B2.1–B2.5, C1.1–C1.4, C2.1–C2.5 |

## Execution

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | ✅ |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | ✅ |
| `cargo clippy -p nf-lineage --no-default-features --all-targets -- -D warnings` | ✅ (cloud features off) |
| `cargo test --workspace --locked` | ✅ 21 passed (15 unit, 2 fixtures_parse, 3 store, 1 doctest), 0 failed, 0 skipped |
| `cargo deny check` | ✅ exit 0; 2 × `advisory-not-detected` warnings (see WARNING) |
| `cargo machete` | ✅ |
| V-grep (`Path::from(`, `set_var`, `process::exit` in `src/`) | ✅ 0 hits |
| Coverage | ➖ not available |

## Spec compliance (PR B1 scenarios)

| Requirement | Scenario | Test | Result |
|-------------|----------|------|--------|
| Find queries | CLI behaviour table | `query::tests::matches_cli_semantics_table` (28 records, 37 CLI cases) | ✅ |
| Find queries | CLI errors (up to the `Did you mean` suffix, deviation) | `query::tests::parse_args_errors_match_cli_messages` (9 cases) | ✅ |
| Find queries | decode/alias/accumulate, `new` without decoding, Java doubles, navigate | `parse_args_decodes_aliases_and_accumulates`, `java_double_strings` | ✅ |
| Store contract | Local fixtures (all 5 versions, `Box<dyn LineageStore>`, `file://`) | `store::local_store_lists_gets_and_finds` | ✅ |
| Store contract | Directory markers | `store::in_memory_store_ignores_directory_markers` | ✅ |
| Store contract | Errors | `store::store_errors_are_typed` | ✅ |
| Store construction | `Path::parse` only, read-only | V-grep + mutation probe (`Path::from` → 2 tests fail) | ✅ |
| Store construction | S3 on MiniStack / no MiniStack | — | ➖ PR C2 |

8/8 in-scope scenarios compliant. Note: the spec scenario "Local fixtures" says 2 runs; the
`26.10.7-agent` fixture has 1, so the test compares with the `.history` entry count.

## TDD compliance

Evidence rows for B1.1–B1.7 in apply-progress.md. RED observed as compile failures (17 for query,
4 for store). Tests passed on first GREEN; strength shown by mutation probes (int-format, falsy
filter, `Path::from`, history depth, `.history` exclusion — each made a test fail, all reverted).
Layers: unit (`src/query.rs`), integration (`tests/store.rs`, fixtures + InMemory), doctest.

## Coherence (design)

| Decision | Followed? | Notes |
|----------|-----------|-------|
| `#[async_trait]` dyn-compatible trait, 3 methods | ✅ | `Box<dyn LineageStore>` in test |
| `.history` private to `list_runs` | ✅ | |
| object_store only, `Path::parse` keys | ✅ | |
| S3 markers: last segment `.data.json`, history one segment | ✅ | |
| `from_env` + explicit options, no `set_var` | ✅ | |
| Cloud features default-on, forwarded | ✅ | |
| Find numbers / hand-rolled `url_decode` | ✅ | sign quirk not ported (ponytail note) |
| `buffer_unordered(16)` | ✅ | |
| PR layout B/C | ⚠️ Deviated | re-split into B1/B2/C1/C2; documented |

## Issues

- **CRITICAL**: none.
- **WARNING**: B1 is ≈ 906 hand-written lines (src ≈ 545 incl. docs, tests ≈ 360), above the
  ~800 budget but within ~900 tolerance; no coherent seam to cut further.
- **WARNING**: cargo-deny 0.22.1 does not report the transitive unmaintained `chrono`/`humantime`
  advisories, so the requested explicit ignores print `advisory-not-detected` warnings (exit 0).
  Alternative: drop the ignores (or set `unmaintained = "workspace"`) — orchestrator's call.
- **SUGGESTION**: port Nextflow's `closest` hint later if exact CLI text matters to consumers.

## Verdict

PASS WITH WARNINGS — B1 green on every gate; slightly over the line budget.

---

# Verification Report: nf-lineage — PR B2

**Mode**: Strict TDD. **Scope**: B2.1–B2.5 (check + semantic diff; `diff_raw` moved to C1.0 by
the escape hatch). **Date**: 2026-11-26.

## Completeness

| Metric | Value |
|--------|-------|
| PR B2 tasks in scope | 5 (B2.1–B2.5) |
| Complete | 5 (B2.5 without `diff_raw`) |
| Moved | `diff_raw` + `similar` + raw-diff test → C1.0 (dated deviation in tasks.md/design.md) |
| Remaining (later PRs) | C1.0–C1.4, C2.1–C2.5 |

## Execution

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | ✅ |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | ✅ |
| `cargo clippy -p nf-lineage --no-default-features --all-targets -- -D warnings` | ✅ |
| `cargo test --workspace --locked` | ✅ 29 passed (20 unit, 1 check, 2 diff, 2 fixtures_parse, 3 store, 1 doctest), 0 failed, 0 skipped |
| `cargo test -p nf-lineage --no-default-features` | ✅ 27 passed (doctest/feature-gated excluded by count) |
| `cargo deny check` | ✅ advisories, bans, licenses, sources ok (same 2 `advisory-not-detected` warnings as B1) |
| `cargo machete` | ✅ |
| V-grep (`Path::from(`, `set_var`, `process::exit` in `src/`) | ✅ 0 hits |
| Coverage | ➖ not available |

## Spec compliance (PR B2 scenarios)

| Requirement | Scenario | Test | Result |
|-------------|----------|------|--------|
| Check | Golden vectors (4 incl. S3 `/bucket/key`, empty, unicode + pre-epoch) | `check::tests::golden_vectors` | ✅ |
| Check | Fixtures: all FileOutputs `Valid`; touch/append → `Mismatch`; delete → `FileMissing`; TaskRun → `UnexpectedKind` | `check::fixture_outcomes` (4 core versions × 10 files) | ✅ |
| Check | No checksum / unsupported algorithm or mode / `gs`,`az`,`http` → typed errors | `check::tests::rejects_what_it_cannot_verify` | ✅ |
| Check | S3 size+mtime from parent listing (not HEAD) | `check::tests::object_stat_lists_the_parent_prefix` (InMemory) | ⚠️ PARTIAL — end-to-end S3 is C2 `s3_ministack::s3_check_valid_and_mismatch` |
| Check | `path_map` prefix remap | `check::tests::path_map_remaps_on_component_boundaries` + fixture test | ✅ |
| Diff | Minimal runs: exactly `params.greeting` hello→bonjour | `diff::minimal_runs_differ_only_in_greeting` (4 core versions) | ✅ |
| Diff | Task pair, kind mismatch | `diff::task_pairs_and_kind_mismatch` | ✅ |
| Diff | Volatile fields / LID hashes / null-vs-absent ignored | `diff::tests::projection_ignores_volatile_fields_and_lid_hashes` | ✅ |
| Diff | Raw parity | — | ➖ C1.0 / C1.3 `contract_diff` |

7/8 in-scope scenarios fully compliant, 1 partial by design (live S3 in C2).

## TDD compliance

Evidence rows B2.1–B2.5 in apply-progress.md. RED observed as compile failures (21 for check, 10
for diff). Mutation probes each made a test fail (BE hash bytes, truncated pre-epoch ms, first
listed object, no LID masking, `sessionId` leaked into the projection, kept nulls — the last one
survived at first and a null-vs-absent assertion was added). Assertion audit: no tautologies; the
fixture loops are guarded (`files.len() == 10` asserted for core fixtures, `versions()` asserts
≥ 5). Layers: unit (`src/check.rs`, `src/diff.rs`), integration on fixtures (`tests/check.rs`,
`tests/diff.rs`).

## Coherence (design)

| Decision | Followed? | Notes |
|----------|-----------|-------|
| `murmur3 0.5`, `u128::to_le_bytes` | ✅ | golden vectors match Nextflow 26.10.7 on S3 and local |
| `check` on S3: hash `/<bucket>/<key>`, `list_with_delimiter(parent)` | ✅ | reuses `open_with_options` (`store` field `pub(crate)`) |
| `gs`/`az` → `Unsupported` | ✅ | |
| `CheckReport { status }`; bad kind/algo → `Err` | ✅ | |
| Semantic diff: whitelist projection, LID masked | ✅ | tables as dot-path strings over `spec()` (deviation noted) |
| Raw diff with `similar 2` | ⚠️ Moved | to C1.0 (escape hatch) |

## Issues

- **CRITICAL**: none.
- **WARNING**: B2 ≈ 870 hand-written lines (src ≈ 500 incl. ~110 doc lines, tests ≈ 330), over
  the ~800 budget even after moving `diff_raw` out; under the ~900 stop line.
- **WARNING**: `check` on a directory FileOutput hashes it like a file (Nextflow hashes
  directories differently) → would report `Mismatch`; no fixture has one. Follow-up.
- **SUGGESTION**: `check` reports `NoChecksum` before testing existence; the CLI fails on a missing
  file first. Documented deviation; switch the order if exact CLI parity matters.

## Verdict

PASS WITH WARNINGS — B2 green on every gate; `diff_raw` deferred to C1.0; slightly over budget.

---

# Verification Report: nf-lineage — PR C1

**Mode**: Strict TDD. **Scope**: C1.0–C1.4 (`diff_raw`, `view`, `provenance`, CLI contract suite,
CI `contract` job). **Date**: 2026-11-26.

## Completeness

| Metric | Value |
|--------|-------|
| PR C1 tasks in scope | 5 (C1.0–C1.4) |
| Complete | 5 |
| Remaining (later PRs) | C2.1–C2.5 |

## Execution

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | ✅ |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | ✅ |
| `cargo clippy -p nf-lineage --no-default-features --all-targets -- -D warnings` | ✅ |
| `cargo test --workspace --locked` | ✅ 39 passed (20 unit, 1 check, 5 contract, 3 diff, 2 fixtures_parse, 3 provenance, 3 store, 1 view, 1 doctest), 0 failed, 0 skipped |
| `cargo test -p nf-lineage --test contract --locked` (C-contract) | ✅ 5 passed |
| `cargo test -p nf-lineage --no-default-features` | ✅ 36 passed |
| `cargo deny check` | ✅ advisories, bans, licenses, sources ok |
| `cargo machete` | ✅ |
| V-grep (`Path::from(`, `set_var`, `process::exit` in `src/`) | ✅ 0 hits; no `unwrap`/`expect`/`panic!`/indexing in `view.rs`, `provenance.rs`, `diff_raw` |
| V-yaml: `ci.yml` parses, jobs `lint`/`test`/`contract`, `contract` runs `cargo test -p nf-lineage --test contract --locked`, `grep -c 'secrets\.'` = 0 | ✅ |
| Coverage | ➖ not available |

## Spec compliance (PR C1 scenarios)

| Requirement | Scenario | Test | Result |
|-------------|----------|------|--------|
| Diff | Raw parity (`diff-runs`, `diff-tasks`, 4 core versions) | `contract::contract_diff` | ✅ (trailing `,` trimmed — spec reworded, see deviation) |
| Diff | `diff_raw` header + unified diff | `diff::raw_diff_has_cli_header_and_record_lines` | ✅ |
| View and provenance | Reaches pipeline input | `provenance::published_file_reaches_pipeline_input` (4 core versions) | ✅ |
| View and provenance | Non-FileOutput → `UnexpectedKind`; no revisiting | `provenance::task_file_stops_at_its_task_and_runs_are_rejected`, `provenance::cycles_are_visited_once` | ✅ |
| View and provenance | View parity (every `view-*` case, 5 versions incl. agent; errors → `NotFound`/`InvalidLid`) | `contract::contract_view` | ✅ |
| View and provenance | Fragment validation (`InvalidQuery`), `#output` → `[]`, `null`, `#type` | `view::fragments_validate_redirect_and_navigate` | ✅ |
| CLI contract | All versions: list / find / check | `contract::contract_list`, `contract_find`, `contract_check` | ✅ |
| ci-pipeline: Contract job | Oracle mismatch fails naming version and case | mutation probe (broken `label` alias → `<version> find-fileoutput-label: …` on 4 versions) + V-yaml | ✅ |

8/8 in-scope scenarios compliant.

## TDD compliance

Evidence rows C1.0–C1.4 in apply-progress.md; test files exist and pass now. RED observed as
compile failures (`diff_raw`, `provenance`/`Provenance`/`ProvenanceInput`, `view` missing). One
GREEN failure was a wrong test assertion (fixed in the test, noted); one was a real parity gap
(`contract_diff`, trailing comma) resolved by a documented comparison rule. Triangulation: view
edge cases and a provenance cycle in InMemory stores beyond the fixture chains. Mutation probes:
9 probes (view redirect, envelope, `[]` mapping, fragment validation, find alias, check mtime
reproduction, list columns, diff key, provenance visited set/dedupe) each made a test fail.
Assertion audit: case loops are guarded (`cases()` asserts a non-empty set per prefix; every case
ends in a match arm that records a failure); no tautologies. Layers: integration over fixtures
(`contract`, `provenance`, `diff`) and over InMemory stores (`view`, provenance cycle).

## Coherence (design)

| Decision | Followed? | Notes |
|----------|-----------|-------|
| Raw diff with `similar 2`, CLI header | ✅ | `TextDiff::from_lines(..).unified_diff()`, keys = LID without `lid://` |
| Raw diff input | ⚠️ Decided | canonical `to_json()` for every version, not stored bytes (CLI re-encodes too); trait unchanged |
| `view`/`provenance`/`diff_raw` signatures (MCP-facing) | ✅ | exactly as design.md Interfaces |
| `provenance`: library, BFS over `get`, visited set | ✅ | |
| `contract` job duplicates `test` as a named check | ✅ | |

## Issues

- **CRITICAL**: none.
- **WARNING**: ≈ 787 hand-written lines, at the edge of the ~800 budget (contract suite 314).
- **SUGGESTION**: `provenance` fails on a dangling reference (`NotFound`) instead of returning a
  partial graph; revisit if agents hit incomplete stores.
- **SUGGESTION**: `diff_raw` text for 25.04/25.10 stores differs from that CLI in layout (`spec`
  envelope, indentation, extra `null` fields); documented, the changed-value lines match.

## Verdict

PASS WITH WARNINGS — every C1 gate green; CLI parity for list/view/find/diff/check proven on all
fixture versions; within budget.

---

# Verification Report: nf-lineage — PR C2 (final batch)

**Mode**: Strict TDD. **Scope**: tasks C2.1–C2.5, plus the whole-change scenario map (C2.5).
**Date**: 2026-11-26.

## Completeness

| Metric | Value |
|--------|-------|
| PR C2 tasks in scope | 5 (C2.1–C2.5) |
| Complete | 5 |
| Whole change | all tasks A–C2 `[x]` (A.8/A.9 done as B1.1/B1.2) |

## Execution

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | ✅ |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | ✅ |
| `cargo test --workspace --locked` (no MiniStack) | ✅ 43 passed, 0 failed; both `s3_ministack` tests print a skip notice and pass |
| same with `NF_LINEAGE_MINISTACK_URL` → `ministackorg/ministack:1.5.45` | ✅ 43 passed, both S3 tests executed |
| `NF_LINEAGE_MINISTACK_URL=http://localhost:1` | ✅ both S3 tests FAIL after 60 s ("not ready"), not skipped |
| `cargo deny check` | ✅ advisories, bans, licenses, sources ok (the known "no crate matched" warnings for ignored advisories) |
| `cargo machete` | ✅ |
| `RUSTDOCFLAGS="-D warnings" cargo doc -p nf-lineage --no-deps` | ✅ |
| Doctests (store + 2 README examples) | ✅ 3 passed |
| V-pkg `cargo package -p nf-lineage --locked` | ✅ (run with `--allow-dirty` before commit); ships `src/`, `README.md`, `Cargo.toml`, `Cargo.lock` |
| V-grep (`Path::from(`, `set_var`, `std::process::exit` in `src/`) | ✅ 0 hits |
| V-yaml: actionlint (`rhysd/actionlint` via Docker) | ✅ 0 errors; `grep -c 'secrets\.'` = 0; image pinned `ministackorg/ministack:1.5.45` |
| V-doc greps in `CONTRIBUTING.md` | ✅ `ministackorg/ministack:1.5.45`, `NF_LINEAGE_MINISTACK_URL`, `s3_ministack` all match |
| V-deny policy content | ✅ unchanged since B1 (`unknown-registry`/`unknown-git` = deny, crates.io only, no copyleft) |
| Coverage | ➖ not available |

## Spec compliance (PR C2 scenarios)

| Requirement | Scenario | Test / check | Result |
|-------------|----------|--------------|--------|
| Store construction | S3 on MiniStack | `s3_ministack::s3_store_roundtrip` (26.10.7 store; `list_runs`, `find` all + `type=FileOutput`, every `#output` record equal to local) | ✅ |
| Store construction | No MiniStack | `s3_ministack::*` without the variable → skip notice, pass | ✅ |
| Check | S3 data | `s3_ministack::s3_check_valid_and_mismatch` (`Valid`, then `Mismatch` after rewrite) | ✅ |
| Integration job | Service up | CI job `integration` (actionlint clean); same command run locally against MiniStack 1.5.45 | ✅ (local), CI pending |
| Integration job | No secrets | `grep -c 'secrets\.'` = 0, image `1.5.45` | ✅ |
| Integration test instructions | Newcomer runs S3 tests | V-doc greps | ✅ |
| Safety and packaging | Package | V-pkg, doctests, V-grep | ✅ |

## Whole-change scenario map (C2.5)

Every `#### Scenario` in the four delta specs was mapped to its named test(s) or check: 23
scenarios name 28 tests, all of which exist and passed in the runs above (`lid` ×4, `record` ×6,
`fixtures_parse` ×2, `query` ×2, `store` ×3, `s3_ministack` ×2, `check` ×2, `diff` ×1,
`provenance` ×1, `contract` ×5); the 9 check-only scenarios (oracle mismatch → `contract::*`
failures name version and case; service up; no secrets; C-deny; V-deny ×2; V-doc; C-test skip;
V-pkg) pass as listed. 32/32 scenarios compliant. PR line counts: A ≈ 940 (escape hatch), B1 ≈ 906, B2 ≈ 870, C1 ≈ 787,
C2 ≈ 350.

## TDD compliance

C2.1 tests were written against existing S3 code paths (store from B1, S3 `check` from B2), so the
first MiniStack run was green; their strength is shown by mutation probes (S3 mtime truncated to
seconds → check test fails; hashed path without the bucket → fails) and by the unreachable
endpoint run. README doctests mutation-probed (`list_runz` → FAILED). Evidence in
apply-progress.md.

## Coherence (design)

| Decision | Followed? | Notes |
|----------|-----------|-------|
| MiniStack test env-gated, raw TCP `PUT` readiness ≤ 60 s, hard fail when set | ✅ | |
| S3 check: `/<bucket>/<key>`, size + mtime from the parent listing | ✅ | exercised against MiniStack |
| CI `integration` with service container, same steps as `test` | ✅ | |
| Dummy credentials in the workflow | ⚠️ Deviated | fixed in the test code (hermetic against a contributor's `AWS_*`); spec reworded, design deviation note |
| `publish = false` removed, 0.1.0 metadata | ✅ | + `include` (tests need repo fixtures) |

## Issues

**CRITICAL**: none.

**WARNING**:
- The `integration` job has not run on GitHub yet (validated with actionlint and the same command
  locally); first PR run confirms it.

**SUGGESTION**:
- `categories` uses `science::bioinformatics`; crates.io validates categories only at publish time.

## Verdict

**PASS WITH WARNINGS** — all C2 tasks done and the whole change maps 32/32 scenarios to passing
tests or checks; ready to archive.
