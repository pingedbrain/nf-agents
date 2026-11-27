# Archive Report: site-landing

**Date**: 2026-11-27
**Artifact store**: openspec
**Verdict at archive**: PASS WITH WARNINGS (verify-report.md: 0 CRITICAL; tasks 1.1-7.2 (29/29) complete; 16 of 20 scenarios COMPLIANT, 4 PARTIAL)

## Evidence

- `site/`: Astro + Starlight scaffold, docs sync (`scripts/sync-docs.mjs`), dist check (`scripts/check-dist.mjs`), tokens contrast test, landing sections, hero pipeline flow, Open Graph image (`public/og.png`, 1200x630).
- `npm ci && npm run build && npm test` pass in verify: 29/29 node:test, `dist OK: 7 html pages checked`.
- `cargo test --workspace` pass.
- Lighthouse (built site): accessibility 100 on all pages; performance 99-100 except one desktop landing run at 82 that reran at 100.
- `.github/workflows/site.yml`: least privilege (`contents: read`, `pages: write` and `id-token: write` only in `deploy`), pinned action tags, no `secrets.*`.

## Post-merge follow-ups

- W1: re-measure desktop landing performance on the CI runner or a quiet machine (one run scored 82).
- W2: run the 375 px screenshot, reduced-motion screenshot and horizontal-overflow check (S-shot), not re-run in verify.
- W3: confirm the `check` and `deploy` jobs on a GitHub Actions run (P-check, P-deploy, Pages-not-enabled error).
- W4: design D11 vs `scripts/og.mjs` (`wawoff2` dependency): update the design or drop the dependency.
- W5, W6, S1-S3: see verify-report.md.
- Enable GitHub Pages (source: GitHub Actions) before the first `main` deploy.

## Specs synced to openspec/specs/

| Domain | Action | Requirements |
|--------|--------|--------------|
| project-site | Created (new capability) | 6 requirements, copied from the change (no prior main spec) |
| site-pipeline | Created (new capability) | 5 requirements, copied from the change (no prior main spec) |

## Archived to

`openspec/changes/archive/2026-11-27-site-landing/`
