# Archive Report: workspace-skeleton

**Date**: 2026-11-26
**Artifact store**: openspec
**Verdict at archive**: PASS (verify-report.md: 0 CRITICAL, 0 WARNING; 17/17 tasks complete)

## Evidence

- Task 4.3 (C-run) satisfied by CI run 45892532356 on PR https://github.com/pingedbrain/nf-agents/pull/1 (branch `chore/workspace-skeleton`): `lint` success, `test` success.

## Specs synced to openspec/specs/

| Domain | Action | Requirements |
|--------|--------|--------------|
| ci-pipeline | Created (new capability) | 5 (triggers and concurrency, least privilege, lint job, test job, toolchain/action pinning, supply-chain policy: 6 total) |
| contributor-guide | Created (new capability) | 2 |
| rust-workspace | Created (new capability) | 5 |

No existing main specs were present, so each delta spec was copied verbatim. No requirements were modified or removed, so no destructive merge warning applied.

## Archived to

`openspec/changes/archive/2026-11-26-workspace-skeleton/` (proposal.md, design.md, tasks.md, specs/, apply-progress.md, verify-report.md, state.yaml, archive-report.md)

## Notes

- proposal.md carries a decision-update note: toolchain pinned to exact `1.100.0` instead of `stable` (design.md).
- Open follow-ups from design.md (not blocking): MSRV CI job, SHA-pinning + Dependabot for actions.
- Git: nothing committed. The change is on branch `chore/workspace-skeleton`; the archive move and new `openspec/specs/` files are uncommitted working-tree changes for the orchestrator to commit.
