# Apply progress: site-landing (Batch A, PR A)

**Mode**: Strict TDD (test runner: `cd site && npm test`, node:test)
**Scope**: tasks 1.1-3.3 (phases 1-3). Batches B and C not started.
**Prior progress**: none (first batch).

## Completed tasks

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

## TDD cycle evidence

| Task | RED (test first, run before implementation) | GREEN | REFACTOR |
|------|---------------------------------------------|-------|----------|
| 2.1 sync-docs tests | Written first. Run before `sync-docs.mjs` existed: fails with `ERR_MODULE_NOT_FOUND` (all 10 cases) | 10/10 pass after 2.2 | none needed |
| 2.2 sync-docs impl | covered by 2.1 | 10/10 pass; real sync of the five sources succeeds | none |
| 2.3 check-dist tests | Written first. Run before `check-dist.mjs` existed: fails with `ERR_MODULE_NOT_FOUND` (all 10 cases) | 10/10 pass after 2.4; `npm test` also passes against real `dist/` (7 pages) | removed two unused imports |
| 2.4 check-dist impl | covered by 2.3 | pass | none |
| 1.x, 3.x scaffold | Not unit-tested. Covered by `npm run build` plus `check-dist` on real `dist/` | build and dist checks pass | none |

Deviation from strict ordering: the phase 1 and phase 3 scaffold files were written before the phase 2 RED tests. The phase 2 test files were still written and observed failing before their implementation modules existed.

## Gate

- `npm ci` from lockfile: pass, 0 vulnerabilities
- `npm test`: 20/20 node:test pass; `dist OK: 7 html pages checked`
- `npm run build`: pass (7 pages, Pagefind index, sitemap)
- `node --check` on every script and `astro.config.mjs`: pass

## Deviations from design

- `check-dist.mjs` exports `checkRoutes(files)` for the five docs routes and `checkText(path, text)` for css/js denylist and external `url()`. The design lists the routes under `checkHtml`. The same checks run on every build.
- `rewriteLink` decides blob versus bare GitHub URL by file extension and trailing slash, with no filesystem lookup. Extension-less repo files (for example `LICENSE`) would get a bare URL. No current source has such a link.
- No `@astrojs/sitemap` dependency: Starlight 0.42 already emits `sitemap-index.xml`.
- `spec.md` "Unrelated PR" scenario was not edited (D13 spec owner item). The workflow path filter follows D13.

## Issues and risks

- Build warnings (not failures): `[content] The collection "i18n" does not exist` and `Entry docs -> 404 was not found`. Starlight expects a `404` doc. Left for a follow-up.
- Only Node 24.14.0 was available locally; `.nvmrc` pins 26.13.1, which CI uses.
- `--ignore-scripts` with `npm ci` was not tested (design open question, non-blocking).
- Starlight dark-mode mapping is not visually verified yet. Verify phase covers it.

## Remaining tasks

- Batch B: 4.1-5.8
- Batch C: 6.1-7.2
