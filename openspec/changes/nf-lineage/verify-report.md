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
