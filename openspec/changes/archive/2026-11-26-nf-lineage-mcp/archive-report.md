# Archive Report: nf-lineage-mcp

**Date**: 2026-11-26
**Artifact store**: openspec
**Verdict at archive**: PASS (verify-report.md, PR C section: 0 CRITICAL, 0 WARNING; tasks A.1–C.4,
20/20 complete; every `lineage-mcp` scenario maps to a passing test or check)

## Evidence

- Implemented in three PRs: A (server, `list_runs`, `view`, `find`, truncation, error mapping), B
  (`diff`, `check`, `provenance`, stdio binary and end-to-end test), C (Docker image, CI `docker`
  steps, README, packaging check).
- Image built locally: 21 151 114 B, user `65532:65532`; the stdio smoke session against the
  26.10.7 fixture store returns 2 run LIDs with `"isError":false`. `ci.yml` is actionlint-clean.
- Open item: the extended `docker` job has not run on GitHub yet; the PR C run confirms it.

## Specs synced to openspec/specs/

| Domain | Action | Requirements |
|--------|--------|--------------|
| lineage-mcp | Created (new capability) | copied from the delta |
| ci-pipeline | Updated | "Docker job" added in its combined form (the `nf-llm-replay` ADDED requirement extended by this change's MODIFIED block: second image, 30 MB gate, stdio smoke, new scenario "PR touching the MCP server"); C-docker row covers both images |

`nf-llm-replay` was archived in the same step, so this change's ci-pipeline delta was merged on
top of that change's "Docker job" requirement, as the delta instructs. No requirement was removed.

`lineage-fixtures` (not archived yet) also edits `ci-pipeline` ("Least privilege, no secrets",
"Test job"); it must merge its delta on top of the main spec as updated here (its fork scenario
must keep the job list that now includes `contract`, `integration` and `docker`).

## Archived to

`openspec/changes/archive/2026-11-26-nf-lineage-mcp/`
