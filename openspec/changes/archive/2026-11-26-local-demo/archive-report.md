# Archive Report: local-demo

**Date**: 2026-11-26
**Artifact store**: openspec
**Verdict at archive**: PASS WITH WARNINGS (verify-report.md: 0 CRITICAL; tasks A.1–A.12 and B.1–B.4 complete; every `local-demo` scenario maps to a passing test or check)

## Evidence

- PR A: `demo/run.sh`, `demo/main.nf`, `demo/tour.py` (+ unittest), the committed cassette, the extended secrets test and the CI steps (`test`: tour unittest + shellcheck; `docker`: `Local demo`).
- PR B: README `## Try it in 5 minutes`, `docs/demo.md` (real transcript, timings, Claude Code prompts, re-recording, cleanup, troubleshooting), README "Status" and ROADMAP v0.1 markers updated.
- Local runs: DM-run exit 0 (24 s warm, 21 s with `:ci` images, 180 s building both images), DM-miss, DM-record (byte-identical cassette from the fake upstream).

## Post-merge follow-ups

- A.13 / C-demo: confirm the `Local demo` step is green on a GitHub CI run and fill its duration into the `docs/demo.md` timings table (currently "pending").
- Fully cold timing, published-image timing (needs the release pipeline), Apple Silicon and WSL2 are not measured; `docs/demo.md` says so.

## Specs synced to openspec/specs/

| Domain | Action | Requirements |
|--------|--------|--------------|
| local-demo | Created (new capability) | copied from the delta |
| ci-pipeline | Updated | "Test job" and "Docker job" replaced by this change's MODIFIED versions (demo checks in `test`, `Local demo` step in `docker`, scenarios "Broken tour logic" and "README demo cannot rot"); checks C-demo, C-demo-unit added. Merged on top of `lineage-fixtures`, archived in the same step. No requirement removed. |

## Archived to

`openspec/changes/archive/2026-11-26-local-demo/`
