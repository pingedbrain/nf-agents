# Archive Report: nf-llm-replay

**Date**: 2026-11-26
**Artifact store**: openspec
**Verdict at archive**: PASS WITH WARNINGS (verify-report.md, PR E section: 0 CRITICAL; all
batches A–E applied and verified; the only open warning was the first GitHub run of the `docker`
job)

## Evidence

- Implemented in six batches: A (normalization + cassette model), B (replay server + CLI), C1
  (upstream client + recorder), C2 (record + passthrough), D (SSE streaming + normalization
  flags), E (Docker image, offline Nextflow example, CI `docker` job).
- During `nf-lineage-mcp` PR C the image was rebuilt from the shared `.dockerignore` context:
  user `65532:65532`, 15 731 642 B, `--help` runs.

## Specs synced to openspec/specs/

| Domain | Action | Requirements |
|--------|--------|--------------|
| llm-replay-cassette | Created (new capability) | copied from the delta; the "PR" column note now names this change instead of its tasks file |
| llm-replay-proxy | Created (new capability) | copied from the delta |
| llm-replay-packaging | Created (new capability) | copied from the delta |
| ci-pipeline | Updated | +1 added ("Docker job", merged together with the `nf-lineage-mcp` extension); 3 modified: "Triggers and concurrency" and "Least privilege, no secrets" scenarios now list every job (`lint`, `test`, `contract`, `integration`, `docker`), "Supply-chain policy" gains the scenario "Data license for root certificates" (its other text, already extended by `nf-lineage`, is kept); purpose line and C-docker row added |

Destructive-merge check: nothing removed. The delta's "Supply-chain policy" text predates the
`nf-lineage` archive (scoped exceptions, advisory ignores); the main spec keeps the newer text and
only adds the scenario. The "(Previously: …)" notes were not carried into the main specs.

## Archived to

`openspec/changes/archive/2026-11-26-nf-llm-replay/`
