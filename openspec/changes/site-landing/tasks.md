# Tasks: site-landing

TDD: tests precede code. Batch gate: `npm ci && npm test && npm run build` in `site/`, then commit. Paths under `site/`.

D13 adopted ("Unrelated PR" scenario widened). Align "Check on pull requests" path list in 1.6.

## Batch A (PR A): scaffold, docs sync, placeholder, workflow

## Phase 1: Foundation

- [x] 1.1 `package.json` (pins per D1, `test` script per design), `.nvmrc`, lockfile.
- [x] 1.2 `astro.config.mjs` (site, base `/nf-agents`, trailingSlash always, Starlight), `src/content.config.ts`.
- [x] 1.3 `src/styles/tokens.css` (light `:root`; dark via media and `data-theme`), `src/styles/starlight.css` mapping `--sl-*`.
- [x] 1.4 Fontsource variable imports (Inter, Space Grotesk, JetBrains Mono) in layout and `customCss`.
- [x] 1.5 `.gitignore` (3 generated paths), `ROADMAP.md` (docs-site in progress), `CONTRIBUTING.md` (site section).
- [x] 1.6 `.github/workflows/site.yml` per design (D13 paths, least privilege, pinned tags).

## Phase 2: Docs sync and dist check (TDD)

- [x] 2.1 RED `scripts/sync-docs.test.mjs`: H1→title, missing H1 throws naming file, link rewrite (`../x.md`, `docs/x.md`, `../../issues?q`, dir→`tree/`, anchor, code fence untouched, relative image throws), missing source throws.
- [x] 2.2 GREEN `scripts/sync-docs.mjs` (`SOURCES`, `transform`, `rewriteLink`, `sync`); copy `brand/logo/*.svg` → `src/generated/brand/`, `favicon.svg` → `public/`.
- [x] 2.3 RED `scripts/check-dist.test.mjs`: `checkHtml` (one h1, footer marker, `/nf-agents/` prefixes resolve, external-host and logo denylists, five docs routes), `pngSize`.
- [x] 2.4 GREEN `scripts/check-dist.mjs`: prints all failures, exit 1.

## Phase 3: Placeholder and wiring

- [x] 3.1 `src/components/SiteFooter.astro` (disclaimer, trademark line, `data-site-footer`) and `StarlightFooter.astro` override.
- [x] 3.2 `src/layouts/Landing.astro` (head, inline theme script) and placeholder `src/pages/index.astro`.
- [x] 3.3 Batch gate; commit A.

## Batch B (PR B): landing sections, responsive, a11y

## Phase 4: Data and contrast (TDD)

- [x] 4.1 RED `scripts/tokens.test.mjs`: parse `tokens.css`, assert ≥4.5:1 per text pair, light and dark.
- [x] 4.2 GREEN: adjust values in `tokens.css` only if RED fails.
- [x] 4.3 `src/data/milestones.ts` (versions, planned/pending).

## Phase 5: Sections

- [x] 5.1 `SiteHeader.astro` (mark-light/mark-dark via `?raw`, CSS switch).
- [x] 5.2 `Hero.astro` with static flow SVG (`role="img"`, `<title>`).
- [x] 5.3 `Terminal.astro`, `QuickStart.astro`: `git clone … && demo/run.sh`, real `params.greeting` diff from `docs/demo.md`.
- [x] 5.4 `HowItWorks.astro`, `Clouds.astro`, `Principles.astro`, `Modules.astro` (planned v0.2).
- [x] 5.5 `Bench.astro` (scores `pending`), `Audience.astro`, `BuiltFor.astro`, `Explorer.astro` (v0.6, leaderboard v0.3, compute/Bedrock/NIST as v0.4/v0.5).
- [x] 5.6 `src/styles/landing.css`: responsive, 16px gutter, no horizontal scroll at 375px, focus styles, reduced-motion base.
- [x] 5.7 Compose sections in `src/pages/index.astro`; exactly one h1.
- [x] 5.8 Batch gate; commit B.

## Batch C (PR C): OG image, hero animation

## Phase 6: OG (TDD)

- [ ] 6.1 RED: `check-dist.test.mjs` OG case (1200x630 `og.png`, `og:image` = `https://pingedbrain.github.io/nf-agents/og.png`).
- [ ] 6.2 GREEN: `og/og.svg`, `scripts/og.mjs` (WOFF→TTF, resvg, `npm run og`); commit `public/og.png`.
- [ ] 6.3 OG/Twitter meta in `Landing.astro` and Starlight `head`; OG check in `check-dist.mjs`.

## Phase 7: Hero animation

- [ ] 7.1 `src/components/PipelineFlow.astro`: edge twins with dasharray/dashoffset, pre-blurred diamond glow, reduced motion (`animation: none`, twins hidden, glow 0.5). Swap into `Hero.astro`.
- [ ] 7.2 Batch gate; commit C.

## Post-merge (orchestrator)

After PR A: enable Pages (design Migration section). Verify phase: Lighthouse, screenshots.
