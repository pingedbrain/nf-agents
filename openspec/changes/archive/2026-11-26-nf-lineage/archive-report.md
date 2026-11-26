# Archive Report: nf-lineage

**Date**: 2026-11-26
**Artifact store**: openspec
**Verdict at archive**: PASS WITH WARNINGS (verify-report.md, PR C2 section: 0 CRITICAL, 1 WARNING;
all tasks A.1–C2.5 complete; 32/32 spec scenarios mapped to passing tests or checks)

## Evidence

- Implemented in five PRs: A (types + parser), B1 (FindQuery + store), B2 (check + semantic
  diff), C1 (raw diff, view, provenance, CLI contract suite, CI `contract`), C2 (MiniStack S3
  tests, CI `integration`, CONTRIBUTING section, `0.1.0` metadata + README).
- S3 tests ran locally against `ministackorg/ministack:1.5.45`; `ci.yml` is actionlint-clean.
- Open warning: the `integration` job has not run on GitHub yet; the first C2 PR run confirms it.

## Specs synced to openspec/specs/

| Domain | Action | Requirements |
|--------|--------|--------------|
| nf-lineage | Created (new capability) | 10 (copied from the delta) |
| ci-pipeline | Updated | +2 added (Contract job, Integration job), 1 modified (Supply-chain policy: scoped license exceptions, advisory ignores with reasons, new "Scoped exceptions" scenario); verification table gains C-contract, C-it; purpose line updated |
| contributor-guide | Updated | +1 added (Integration test instructions) |
| rust-workspace | Updated | 1 removed (nf-lineage stub, with T-1 and its references); purpose line updated |

Destructive-merge check: the only removal is the `nf-lineage stub` requirement, which the delta
removes explicitly because the `nf-lineage` capability ("Safety and packaging") replaces it. The
"(Previously: …)" and dated deviation notes of the deltas were not carried into the main specs.

`lineage-fixtures` (not archived yet) also edits `ci-pipeline`; it must merge its delta on top of
the main spec as updated here.

## Archived to

`openspec/changes/archive/2026-11-26-nf-lineage/` (exploration.md, proposal.md, specs/, design.md,
tasks.md, apply-progress.md, verify-report.md, state.yaml, archive-report.md)
