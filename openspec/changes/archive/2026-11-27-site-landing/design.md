# Design: site-landing

## Technical Approach

A static Astro site in `site/`. The landing page (`src/pages/index.astro`) is outside Starlight. Starlight serves the generated docs under `/docs/`. A prebuild script (`scripts/sync-docs.mjs`) copies the five repo sources and the `brand/logo/` SVGs into gitignored paths. `npm test` runs `node --test` over the pure script functions, then `scripts/check-dist.mjs` checks `dist/`. Specs: `specs/project-site/spec.md`, `specs/site-pipeline/spec.md`. No runtime JavaScript ships except Starlight's own scripts (search, theme picker) and a tiny inline theme script on the landing page.

## Architecture Decisions

| # | Topic | Options | Decision / rationale |
|---|---|---|---|
| D1 | Versions (checked with `npm view` on 2026-11-27; each release is ≥ 14 days old) | latest / **newest release ≥ 14 days old, exact pins** | `astro@7.4.11` (2026-11-11). `@astrojs/starlight@0.42.11` (2026-11-12, peer `astro ^7.2.10`; 0.43.x is < 14 days old). `@fontsource-variable/{inter,space-grotesk,jetbrains-mono}@5.3.0`. Exact versions in `package.json`, and `package-lock.json` is committed. npm installs peer `@astrojs/markdown-remark` and locks it. |
| D2 | Node | 24 / **26.13.1 (LTS "Lithium")** | `site/.nvmrc` = `26.13.1`. `setup-node` reads it. `engines.node: ">=22.23.4"` (Astro's own floor), so a local Node 24 still works. |
| D3 | Routes | Starlight at root / **landing `src/pages/index.astro`, docs in `src/content/docs/docs/`** | `site: https://pingedbrain.github.io`, `base: /nf-agents`, `trailingSlash: 'always'` (Pages serves `dir/index.html`; dev and prod resolve URLs the same way). `docs/demo.md` → `/docs/` (Getting started, so no redirect is needed). The others go to `/docs/architecture/`, `/docs/nf-lineage-mcp/`, `/docs/nf-llm-replay/` and `/docs/roadmap/`. The sidebar lists them explicitly. |
| D4 | Link rewriting | regex per pattern / remark plugin / **resolve with `new URL(target, BLOB + src)`** | `BLOB = https://github.com/pingedbrain/nf-agents/blob/main/`. One rule covers `../x.md`, `docs/x.md` and `../../issues?…` (→ `github.com/pingedbrain/nf-agents/issues?…`). If the result is a mapped source, the link becomes `/nf-agents/docs/<slug>/` + hash. Starlight slugs headings with github-slugger, so anchors still match. A relative link ending in `/` uses `tree/`. Absolute URLs, `#frag` and `mailto:` are kept. Code fences and inline code are skipped. A relative image or HTML `src` makes the sync fail: no source has one, and serving it from GitHub would break the no-external-hosts rule. |
| D5 | Title | keep H1 / **first H1 → `title`** | If the first non-empty line is not `# …`, the sync fails naming the file. The title is written with `JSON.stringify` (valid YAML). Frontmatter also gets `editUrl: …/edit/main/<src>`, so "Edit page" opens the real source. |
| D6 | Brand assets | copy into `site/` / Vite fs outside root / **sync copies `brand/logo/*.svg` → `src/generated/brand/`, `favicon.svg` → `public/`** | Single source, gitignored output. The header inlines `mark-light`/`mark-dark` via `?raw` and switches between them with CSS. |
| D7 | Tokens / theme | two stylesheets / **`src/styles/tokens.css` + `starlight.css` mapping `--sl-*`** | `:root` holds the light set. `@media (prefers-color-scheme: dark) :root:not([data-theme='light'])` and `:root[data-theme='dark']` hold the dark set. The light values come from the reference (bg `#F5F6F2`, ink `#141816`, accent `#0B6B58`). The dark values: bg `#080B0A`, ink `#EEF2EF`, muted `#8E9893` (6.65:1), accent `#3FBF9B`. The brand hexes match `brand/logo/`. `--nf-faint #7A847F` (3.56:1) is used only for the logo's `nf-` prefix. On the landing page, a 5-line `is:inline` script applies the `starlight-theme` value from localStorage, so it follows the theme chosen in the docs. |
| D8 | Fonts | static weights / **variable `@fontsource-variable/*`** | Three CSS imports. Each subset has a `unicode-range`, so browsers fetch only Latin. Nothing is loaded from a CDN. |
| D9 | Footer | landing only / **`SiteFooter.astro` + Starlight `components.Footer` override** | The override renders Starlight's default footer, then `SiteFooter`. The text is README §Status (no clinical claims) and the README trademark line, inside `<footer data-site-footer>`. There is no location. |
| D10 | Hero animation | SMIL `animateMotion` / CSS `offset-path` on SVG / **CSS `stroke-dashoffset` on a particle overlay** | Each edge `<path pathLength="100">` gets a twin with `stroke-dasharray: 0.1 49.9`, a round cap and an animated `stroke-dashoffset`, so dots run along the edge. Delays are staggered per edge. Agent diamonds glow through the animated `opacity` of a pre-blurred twin (compositor-only, no animated filter). SMIL ignores CSS media queries. `offset-path` on SVG children is less portable. Under `prefers-reduced-motion: reduce`: `animation: none`, particle twins hidden, glow fixed at 0.5. The diagram stays complete. No JS. `role="img"` + `<title>`. |
| D11 | OG image | build-time (satori + resvg) / sharp (system fonts) / **committed `public/og.png` from `og/og.svg` via `npm run og`** | `npm run og` = `npm install --no-save @resvg/resvg-js@2.6.2 && node scripts/og.mjs`. It adds nothing to the lockfile or to CI. resvg cannot read WOFF, so `og.mjs` decodes the fontsource `.woff` files (zlib per table, about 25 lines, `node:zlib`) into temporary TTFs and passes them as `fontFiles` with `loadSystemFonts: false`. The output is reproducible. |
| D12 | Checks | Playwright / vitest / **`node:test` + one dist script** | No test framework is added. Lighthouse and screenshots run in verify. |
| D13 | PR path filter | `site/**` only / **`site/**`, the five sources, `brand/logo/**`, `site.yml`** | Editing a source can break the sync (for example by removing the H1). ⚠ The spec scenario "Unrelated PR" names only `site/**` and `site.yml`. The spec should add the source paths. |

## Data Flow

```
npm run build ─ prebuild: sync-docs.mjs
   ├─ docs/demo.md …ROADMAP.md ─transform()─▶ src/content/docs/docs/*.md   (gitignored)
   └─ brand/logo/*.svg ─copy─▶ src/generated/brand/, public/favicon.svg     (gitignored)
 astro build ─▶ dist/ (landing + Starlight + pagefind + sitemap)
npm test ─ node --test scripts/*.test.mjs ─▶ check-dist.mjs dist/ (exit 1 + list)
site.yml: check (PR filtered | push main) ─upload-pages-artifact─▶ deploy (main) ─▶ Pages
```

## File Changes

| File | Action | PR | Est. |
|---|---|---|---|
| `site/package.json`, `.nvmrc`, `package-lock.json` (generated) | Create | A | 30 |
| `site/astro.config.mjs`, `src/content.config.ts` | Create | A | 55 |
| `site/src/styles/tokens.css`, `starlight.css` | Create | A | 100 |
| `site/scripts/sync-docs.mjs` + `sync-docs.test.mjs` | Create | A | 190 |
| `site/scripts/check-dist.mjs` + `check-dist.test.mjs` | Create | A | 170 |
| `site/src/layouts/Landing.astro`, `components/SiteFooter.astro`, `StarlightFooter.astro`, placeholder `pages/index.astro` | Create | A | 110 |
| `.github/workflows/site.yml` | Create | A | 60 |
| `.gitignore` (3 generated paths), `ROADMAP.md` (in progress), `CONTRIBUTING.md` (site section) | Modify | A | 10 |
| `site/src/components/` SiteHeader, Hero (static flow SVG), Terminal, BuiltFor, QuickStart, HowItWorks, Clouds, Principles, Modules, Bench, Audience, Explorer | Create | B | 560 |
| `site/src/data/milestones.ts` (versions/status: one source for `planned`/`pending`) | Create | B | 40 |
| `site/src/pages/index.astro`, `src/styles/landing.css` | Modify/Create | B | 110 |
| `site/scripts/tokens.test.mjs` (contrast ≥ 4.5:1 per text pair) | Create | B | 50 |
| `site/src/components/PipelineFlow.astro` (animation + particle twins) | Create | C | 200 |
| `site/og/og.svg`, `scripts/og.mjs`, `public/og.png` (binary) | Create | C | 130 |
| `Landing.astro` OG/Twitter meta, Starlight `head` og:image, OG check in `check-dist` (+ test) | Modify | C | 50 |

Totals ≈ A 725, B 760, C 380 hand-written lines. A is larger than the proposal's 450 because both test suites ship in A. The terminal diff and the commands come from the `docs/demo.md` transcript and the README quickstart.

## Interfaces / Contracts

```js
// scripts/sync-docs.mjs (pure parts exported for node:test)
export const SOURCES = [{ src: 'docs/demo.md', slug: '' }, { src: 'docs/architecture.md', slug: 'architecture' }, …];
export function transform(md, { src, base = '/nf-agents' }) // → string; throws Error(`${src}: …`)
export function rewriteLink(target, src, base)             // → string
export async function sync({ repoRoot, siteRoot })        // throws `missing source: <src>`
// scripts/check-dist.mjs
export function checkHtml(relPath, html, files /* Set of dist paths */) // → string[] errors
export function pngSize(buf) // → { width, height } from IHDR
```

`check-dist` assertions, run on every `dist/**/*.html`:
1. Exactly one `<h1`.
2. `data-site-footer` is present and contains "Not affiliated with or endorsed by Seqera".
3. Every root-relative `href`/`src`/`srcset` starts with `/nf-agents/` and resolves to a file in `dist/`. Only internal links are checked; external ones are not followed.
4. No `script`/`link`/`img`/`source`/CSS `url()` points to another host. A denylist (`fonts.googleapis`, `fonts.gstatic`, `googletagmanager`, `google-analytics`, `plausible`) is also grepped across html/css/js.
5. No asset name or `alt` matches `nextflow|seqera|aws|azure|gcp|google-cloud` logos.
6. The five docs routes exist.
7. PR C adds: `og.png` is 1200x630 and `og:image` is `https://pingedbrain.github.io/nf-agents/og.png`.

The check prints all failures, then exits 1.

`site.yml`:
- Triggers: `pull_request` (paths per D13), `push: branches [main]` (no path filter, per spec) and `workflow_dispatch`.
- Top-level `permissions: contents: read`. Concurrency `site-${{ github.ref }}` cancels in progress on PRs only.
- `check` job (`defaults.run.working-directory: site`): `actions/checkout@v7` (`persist-credentials: false`), `actions/setup-node@v7` (`node-version-file: site/.nvmrc`, `cache: npm`, `cache-dependency-path: site/package-lock.json`), then `npm ci`, `npm run build`, `npm test`. When the ref is `main` and the event is not a PR, it also runs `actions/upload-pages-artifact@v5` with `path: site/dist`.
- `deploy` job: `needs: check`, `if: github.ref == 'refs/heads/main' && github.event_name != 'pull_request'`. It sets `permissions: { pages: write, id-token: write }`, `environment: github-pages`, and `concurrency: { group: pages, cancel-in-progress: false }`. Steps: `actions/configure-pages@v6`, which fails with a "Pages not enabled" error before deploy, then `actions/deploy-pages@v5`. No `secrets.*`.

## Testing Strategy

| Layer | What | Approach |
|---|---|---|
| Unit | H1 → title, missing H1, link rewrite (doc, repo file, dir, `../../issues`, anchor, code fence untouched, relative image rejected), missing source | `sync-docs.test.mjs` with string fixtures; `sync()` against a temp repo root |
| Unit | each dist assertion, positive and negative; PNG IHDR | `check-dist.test.mjs` with inline HTML strings |
| Unit | token contrast, light and dark | `tokens.test.mjs` parses `tokens.css` |
| Build | whole site | `npm run build` + `check-dist.mjs` (CI `check`) |
| Verify | copy table, Lighthouse ≥ 95, screenshots at desktop/375 px, reduced motion | manual / sdd-verify |

## Migration / Rollout

No migration. After PR A merges, enable Pages: `gh api -X POST repos/pingedbrain/nf-agents/pages -f build_type=workflow`. Until then `deploy` fails at `configure-pages`. Then merge B and C.

## Open Questions

- [ ] D13: align the "Unrelated PR" spec scenario with the source path filter (spec owner).
- [ ] `npm ci` runs install scripts by default. Whether `--ignore-scripts` still builds (esbuild, pagefind, sharp binaries) is untested. Try it in PR A, and keep plain `npm ci` if it fails. Non-blocking.
