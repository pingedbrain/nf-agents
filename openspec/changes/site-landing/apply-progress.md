# Apply progress: site-landing (cumulative: Batch A, PR A; Batch B, PR B)

**Mode**: Strict TDD (test runner: `cd site && npm test`, node:test)
**Scope so far**: tasks 1.1-5.8 (phases 1-5). Batch C (phases 6-7) not started.
**Prior progress**: Batch A (1.1-3.3) merged from the earlier apply run, kept unchanged below. Batch B added in this run.

## Completed tasks

### Batch A (PR A)

- [x] 1.1 `site/package.json` (exact pins per D1), `site/.nvmrc` (26.13.1), `site/package-lock.json`
- [x] 1.2 `site/astro.config.mjs` (site, base `/nf-agents`, trailingSlash always, Starlight), `site/src/content.config.ts`
- [x] 1.3 `site/src/styles/tokens.css` (light `:root`; dark via media query and `data-theme`), `site/src/styles/starlight.css` (`--sl-*` mapping)
- [x] 1.4 Fontsource variable imports (Inter, Space Grotesk, JetBrains Mono) in `starlight.css` (customCss) and `Landing.astro`
- [x] 1.5 `.gitignore` (3 generated paths), `ROADMAP.md` (docs site marked in progress), `CONTRIBUTING.md` (docs site section)
- [x] 1.6 `.github/workflows/site.yml` (D13 paths, `contents: read`, pinned tags, no secrets)
- [x] 2.1 RED `site/scripts/sync-docs.test.mjs`
- [x] 2.2 GREEN `site/scripts/sync-docs.mjs` (`SOURCES`, `transform`, `rewriteLink`, `sync`; copies brand SVGs)
- [x] 2.3 RED `site/scripts/check-dist.test.mjs`
- [x] 2.4 GREEN `site/scripts/check-dist.mjs` (prints all failures, exit 1)
- [x] 3.1 `site/src/components/SiteFooter.astro`, `site/src/components/StarlightFooter.astro`
- [x] 3.2 `site/src/layouts/Landing.astro` (head, inline theme script), placeholder `site/src/pages/index.astro`
- [x] 3.3 Batch gate passed; commit A

### Batch B (PR B)

- [x] 4.1 RED `site/scripts/tokens.test.mjs` (contrast ≥ 4.5:1 per text pair, light, dark media, dark data-theme)
- [x] 4.2 GREEN check: no change to `site/src/styles/tokens.css` needed (all text pairs already pass, see evidence)
- [x] 4.3 `site/src/data/milestones.ts` (versions and status from ROADMAP, `SCORE_PENDING`)
- [x] 5.1 `site/src/components/SiteHeader.astro` (mark-light/mark-dark via `?raw`, CSS switch)
- [x] 5.2 `site/src/components/Hero.astro` (static flow SVG, `role="img"`, `<title>`, `<desc>`)
- [x] 5.3 `site/src/components/Terminal.astro`, `site/src/components/QuickStart.astro` (README command; real `params.greeting` diff)
- [x] 5.4 `site/src/components/HowItWorks.astro`, `Clouds.astro`, `Principles.astro`, `Modules.astro` (RUN_METHODS and QC_REVIEW planned v0.2, scores `pending`)
- [x] 5.5 `site/src/components/Bench.astro` (scores `pending`, leaderboard v0.3), `Audience.astro`, `BuiltFor.astro`, `Explorer.astro` (v0.6); compute/Bedrock/NIST shown as v0.4/v0.5 roadmap
- [x] 5.6 `site/src/styles/landing.css` (16px gutter, stacked table under 640px, focus-visible, reduced-motion base, skip link)
- [x] 5.7 `site/src/pages/index.astro` composes the sections; one `h1` (hero); skip link to `#main`
- [x] 5.8 Batch gate passed; commit B (see git log)

## TDD cycle evidence

| Task | RED (test first, run before implementation) | GREEN | REFACTOR |
|------|---------------------------------------------|-------|----------|
| 2.1 sync-docs tests | Written first. Run before `sync-docs.mjs` existed: fails with `ERR_MODULE_NOT_FOUND` (all 10 cases) | 10/10 pass after 2.2 | none needed |
| 2.2 sync-docs impl | covered by 2.1 | 10/10 pass; real sync of the five sources succeeds | none |
| 2.3 check-dist tests | Written first. Run before `check-dist.mjs` existed: fails with `ERR_MODULE_NOT_FOUND` (all 10 cases) | 10/10 pass after 2.4; `npm test` also passes against real `dist/` (7 pages) | removed two unused imports |
| 2.4 check-dist impl | covered by 2.3 | pass | none |
| 4.1 tokens tests | Written first. Run before `scripts/tokens.mjs` existed: fails with `ERR_MODULE_NOT_FOUND` | after writing `tokens.mjs` (parser + WCAG contrast): 5/5 pass | none |
| 4.2 token values | not needed: the contrast assertions passed on the existing values. Measured ratios: light ink 16.5 on bg, muted 7.0, accent 5.9; dark muted 6.65, accent 8.6 | no change to `tokens.css` | none |
| 1.x, 3.x, 5.x scaffold and components | Not unit-tested. Covered by `npm run build`, `check-dist` on real `dist/`, a 375 px overflow measurement, and one 375 px screenshot | build and dist checks pass; no horizontal overflow at 375 px | none |

Deviations from strict ordering:
- Batch A: the phase 1 and phase 3 scaffold files were written before the phase 2 RED tests. The phase 2 test files were still written and observed failing before their implementation modules existed.
- Batch B: the 4.1 RED step fails on the missing helper module rather than on a token value, because every text pair already meets 4.5:1. The 4.2 GREEN step therefore made no change. This is the correct outcome of the task ("adjust only if RED fails"), but the RED did not fail on a contrast value.

## Gate

Batch A:
- `npm ci` from lockfile: pass, 0 vulnerabilities
- `npm test`: 20/20 node:test pass; `dist OK: 7 html pages checked`
- `npm run build`: pass (7 pages, Pagefind index, sitemap)
- `node --check` on every script and `astro.config.mjs`: pass

Batch B:
- `npm ci`: pass, 0 vulnerabilities
- `npm run build`: pass (7 pages). The two warnings from batch A remain (`i18n` collection, `docs -> 404`).
- `npm test`: 25/25 node:test pass; `dist OK: 7 html pages checked`
- Copy grep on `dist/index.html` (fixed-string): no `ghcr.io`, `[ORG]`, `In progress`, `Help wanted`, `EKS`, `Sarek`, `Anthropic`, `Gemini`, `Mistral`, `NF-Agents`, `Nf-agents`, `Coming soon`. `pending` appears 7 times. `git clone` and `params.greeting` appear once each.
- Layout at 375 px: `scrollWidth` equals `clientWidth` (no horizontal overflow). One full-height screenshot reviewed.
- Hand-written lines in batch B: about 1390 (scripts, data, components, styles, page), above the ~800 budget. See deviations.

## Deviations from design

Batch A:
- `check-dist.mjs` exports `checkRoutes(files)` for the five docs routes and `checkText(path, text)` for css/js denylist and external `url()`. The design lists the routes under `checkHtml`. The same checks run on every build.
- `rewriteLink` decides blob versus bare GitHub URL by file extension and trailing slash, with no filesystem lookup. Extension-less repo files (for example `LICENSE`) would get a bare URL. No current source has such a link.
- No `@astrojs/sitemap` dependency: Starlight 0.42 already emits `sitemap-index.xml`.
- `spec.md` "Unrelated PR" scenario was not edited (D13 spec owner item). The workflow path filter follows D13.

Batch B:
- `site/scripts/tokens.mjs` was added next to `tokens.test.mjs` (design listed only the test). It exports `parseTokenSets` and `contrastRatio`, following the same pattern as `sync-docs.mjs`.
- Header brand uses the mark SVG plus the plain text `nf-agents` in ink. The design's faint `nf-` prefix is not used as text, so `--nf-faint` is not a text colour on the page.
- Copy follows the copy-corrections table in `specs/project-site/spec.md`: only claims backed by README.md and ROADMAP.md; the demo transcript is the real one from README; planned features carry their roadmap version; no form controls for features that do not exist yet.
- Module status shows `Planned, v0.2` or `Planned, v0.5`, not "In progress", per the copy table. Scores are `pending` for every module.
- Compute targets follow ROADMAP: AWS v0.4; GCP, Azure and Kubernetes v0.6. Models: Bedrock v0.4. Lineage storage (local, S3, GCS, Azure) v0.1, in review.
- Hand-written code is about 1390 lines for batch B against a ~800 budget (design estimated 760). The overage is mostly component markup and CSS for 12 sections. Not trimmed further in this run.

## Issues and risks

- Build warnings (not failures): `[content] The collection "i18n" does not exist` and `Entry docs -> 404 was not found`. Starlight expects a `404` doc. Left for a follow-up.
- Only Node 24.14.0 was available locally; `.nvmrc` pins 26.13.1, which CI uses.
- `--ignore-scripts` with `npm ci` was not tested (design open question, non-blocking).
- Starlight dark-mode mapping is not visually verified yet. Verify phase covers it.
- Lighthouse and dark-mode screenshots are not measured. Only the 375 px overflow check and one light-scheme screenshot were done. Verify phase covers Lighthouse (≥ 95) and reduced-motion screenshots.
- The `Planned` copy relies on ROADMAP wording. If the roadmap changes, `milestones.ts` and the component copy must change together.

## Remaining tasks

- Batch C: 6.1-7.2 (OG image, hero animation). Not started.
- Post-merge (orchestrator): enable Pages after PR A; verify phase (Lighthouse, screenshots).
