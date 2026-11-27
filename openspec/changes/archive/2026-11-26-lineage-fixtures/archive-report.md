# Archive Report: lineage-fixtures

**Date**: 2026-11-26
**Artifact store**: openspec
**Verdict at archive**: PASS WITH WARNINGS (verify-report.md: 0 CRITICAL; tasks 1.1–8.3 complete)

## Evidence

- Committed stores and `nextflow lineage` oracles for 25.04.8, 25.10.9, 26.04.9, 26.10.7 and `26.10.7-agent`; offline checks (C-fixtures) in the CI `test` job; `fixtures.yml` for manual and weekly regeneration with a non-blocking edge job.

## Post-merge follow-up

- C-fx-run: a `workflow_dispatch` run of `fixtures.yml` with `force: false` after merge, expected to end with "no format drift" in the job summary. It needs the workflow on the default branch, so it could not run before archiving.

## Specs synced to openspec/specs/

| Domain | Action | Requirements |
|--------|--------|--------------|
| lineage-fixtures | Created (new capability) | copied from the delta |
| ci-pipeline | Updated | ADDED "Fixtures regeneration workflow", "Regeneration opens a PR only on drift", "Edge early warning"; MODIFIED "Least privilege, no secrets" (scoped to `ci.yml`) and "Test job" (C-fixtures); checks C-fixtures, V-fx-yaml, C-fx-run added; Purpose updated |

Reconciled: the delta's fork scenario said "both jobs", written when `ci.yml` had only `lint` and `test`. The merged scenario names all five `ci.yml` jobs (`lint`, `test`, `contract`, `integration`, `docker`); `fixtures.yml` (`regenerate`, `edge`) is covered by its own requirements. "Test job" was then superseded by the `local-demo` version (archived in the same step), which keeps the "Corrupted fixture" scenario.

## Archived to

`openspec/changes/archive/2026-11-26-lineage-fixtures/`
