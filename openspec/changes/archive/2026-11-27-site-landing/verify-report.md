# Verification report: site-landing

Mode: Strict TDD (test runner `cd site && npm ci && npm run build && npm test`). Date: 2026-11-27. Branch: `chore/archive-site-landing`.

## Completeness

Tasks: 29 total, 29 complete (`tasks.md` phases 1-7, all `[x]`). Post-merge item (enable GitHub Pages with source GitHub Actions) is a rollout step outside `tasks.md`, not verified here.

## Execution

| Command | Result |
|---------|--------|
| `cd site && npm ci` | ok |
| `npm run build` | ok, 7 pages, Pagefind index, sitemap |
| `npm test` (node:test 29/29, then `check-dist` on `dist/`) | ok, 29 pass / 0 fail, `dist OK: 7 html pages checked` |
| `cargo test --workspace` | ok, every suite 0 failed (exit 0) |
| Lighthouse 12 on built site (`astro preview`, headless) | see table below |

### Lighthouse (built site, preview server)

| Page | Preset | Performance | Accessibility |
|------|--------|-------------|---------------|
| `/nf-agents/` | desktop, run 1 | 82 | 100 |
| `/nf-agents/` | desktop, reruns 2 and 3 | 100, 100 | 100, 100 |
| `/nf-agents/` | mobile | 99 | 100 |
| `/nf-agents/docs/` | desktop | 100 | 100 |
| `/nf-agents/docs/` | mobile | 99 | 100 |

Run 1 on the landing page had total-blocking-time 400 ms (LCP 0.4 s, CLS 0.006). Both reruns had TBT 0 ms. Treated as cold-start noise, recorded as WARNING W1.

## Spec compliance

Each row is backed by a test that passed in this run, a static check, or a grep on `dist/`.

| Requirement | Scenario | Evidence | Result |
|-------------|----------|----------|--------|
| Docs generated from repository sources | Source edit and links (S-test) | `sync-docs.test.mjs` "links to mapped docs...", "...other repo files...", "sync writes docs pages and copies brand assets" | COMPLIANT (tracked-file clause: W6) |
| Docs generated from repository sources | H1 and missing source | "first H1 becomes the frontmatter title", "missing H1 throws", "sync fails naming a missing source" | COMPLIANT |
| Copy matches the repository | Corrections in dist (S-copy) | grep on `dist/` (see Copy check) | COMPLIANT (manual grep, not automated: S1) |
| Brand, footer and claims | Footer and logos (S-test) | `check-dist.test.mjs` footer and vendor-logo cases; dist check on 7 pages | COMPLIANT |
| Brand, footer and claims | Dark contrast (S-lh) | `tokens.test.mjs` darkMedia and darkAttr sets >= 4.5:1; Lighthouse a11y 100 | COMPLIANT |
| No third-party analytics or external assets | No external hosts (S-test) | `check-dist.test.mjs` external-host cases; dist scan: all `src`/`href` to external hosts are `<a>` links to GitHub only | COMPLIANT |
| Hero, OG image and reduced motion | Reduced motion (S-shot) | CSS `@media (prefers-reduced-motion: reduce)` in `PipelineFlow.astro` (`animation: none`, twins hidden, glow 0.5) present in build | PARTIAL (no screenshot run: W2) |
| Hero, OG image and reduced motion | OG image (S-test) | `check-dist.test.mjs` "OG image must exist and be 1200x630", "og:image ... base passes"; `dist/og.png` checked | COMPLIANT |
| Responsive layout and accessibility | Phone width (S-shot, S-lh) | Lighthouse mobile a11y 100 and perf 99; 16 px gutter in `landing.css`; overflow measured in apply phase | PARTIAL (screenshot and overflow not re-run: W2) |
| Responsive layout and accessibility | Exactly one h1 | `check-dist` "more than one h1 is reported"; dist check passes on 7 pages | COMPLIANT |
| site-pipeline | Site change (P-check) | Workflow `check` runs `npm ci`, `npm run build`, `npm test` in order | PARTIAL (static; no GitHub run: W3) |
| site-pipeline | Broken prefix (P-check) | `check-dist.test.mjs` "root-relative links without the /nf-agents/ base are reported" | COMPLIANT |
| site-pipeline | Unrelated PR (P-yaml) | `pull_request.paths` lists `site/**`, `site.yml`, five sources, `brand/logo/**` | COMPLIANT (static) |
| site-pipeline | Push without site changes (P-deploy) | `push: branches: [main]`, no path filter | COMPLIANT (static) |
| site-pipeline | PR does not deploy (P-yaml) | `deploy.if` requires `refs/heads/main` and not `pull_request` | COMPLIANT (static) |
| site-pipeline | Failed check blocks deploy (P-deploy) | `deploy.needs: check` | COMPLIANT (static) |
| site-pipeline | Pages not enabled (P-deploy) | `deploy` uses `actions/configure-pages@v6`; error text comes from the action | PARTIAL (needs a GitHub run: W3) |
| site-pipeline | Permissions (P-yaml) | top-level `contents: read`; `pages: write`, `id-token: write` only in `deploy`; `grep -c 'secrets\.'` = 0 | COMPLIANT |
| site-pipeline | Action refs (P-yaml) | `@v7`, `@v5`, `@v6` only (checkout, setup-node, upload-pages-artifact, configure-pages, deploy-pages) | COMPLIANT |
| site-pipeline | Overlapping pushes (P-yaml) | deploy `concurrency: group: pages`, `cancel-in-progress: false` | COMPLIANT (static) |

**Compliance summary**: 16 of 20 scenarios COMPLIANT by test or dist evidence; 4 PARTIAL (screenshot or GitHub run not executed, see W2 and W3); 0 FAILING; 0 UNTESTED.

Copy check (S-copy), grep on `dist/` text:

| Required text | Found | Reference text | Found |
|---------------|-------|----------------|-------|
| `git clone` + `demo/run.sh` | yes | `ghcr.io` | 0 |
| `params.greeting` diff | yes | `NF-Agents`, `Nf-agents` | 0 |
| `planned for v0.2` / `pending` | yes / yes | `In progress` on landing | 0 (only in architecture doc, source text) |
| leaderboard `v0.3`, explorer `v0.6` | yes | leaderboard `v0.1` on landing | 0 |
| `v0.4` Bedrock/compute, `v0.5` NIST | yes | location in footer | 0 |

## Coherence (design)

| Decision | Followed? | Notes |
|----------|-----------|-------|
| D1 pinned versions | Yes | exact pins in `package.json`, lockfile committed |
| D2 Node 26.13.1 | Yes | `site/.nvmrc` = 26.13.1; `engines >=22.23.4` |
| D3 routes | Yes | 7 pages built, docs routes present |
| D7 tokens and theme | Yes | light `:root`, dark via media and `data-theme` |
| D8 bundled fonts | Yes | Fontsource variable imports, no CDN |
| D10 hero animation | Yes | CSS `stroke-dashoffset` overlay, no JS |
| D11 OG image | Deviated | design says zlib-only WOFF decode; `scripts/og.mjs` also uses `wawoff2` (installed `--no-save` by `npm run og`). See W4 |
| D12 no test framework | Yes | `node:test` only |
| D13 path filter | Yes | spec scenario "Unrelated PR" already widened |

## Strict TDD compliance

- RED test files exist: `sync-docs.test.mjs`, `check-dist.test.mjs`, `tokens.test.mjs`. All GREEN in this run (29/29).
- Assertion quality audit: no tautologies (`assert(true)`, `toBe(true)`), no ghost loops. Tests assert values (paths, error messages, WCAG ratios, PNG dimensions).
- Batch A deviation: scaffold files (phases 1 and 3) were written before the phase 2 RED tests. Disclosed in `apply-progress.md` (W5).
- Coverage: no coverage tool configured for `site/`. Reported as not available.
- Lint and type check: none configured for `site/` (`node --check` only, recorded in apply-progress). Not available.
- Test layers: 29 unit tests (node:test over pure functions and temp fixtures) plus the `dist/` check. No integration or E2E layer. Informational (S2).

## Issues

**CRITICAL** (must fix before archive): None.

**WARNING** (should fix):
- W1: Landing desktop performance scored 82 on run 1 (TBT 400 ms). Reruns scored 100. The spec threshold is >= 95. Re-measure on the CI runner or a quiet machine before the next release.
- W2: S-shot not re-run in this verify: no 375 px screenshot, no reduced-motion screenshot, no horizontal-overflow measurement. Code evidence only (CSS present; apply-progress overflow measurement).
- W3: `site-pipeline` checks P-check, P-deploy and the Pages-not-enabled error need a real GitHub Actions run. Verified statically only.
- W4: Design D11 says the OG script decodes WOFF with `node:zlib` only. `scripts/og.mjs` depends on `wawoff2@2.0.1` (installed with `--no-save`). Either update design D11 or drop the dependency.
- W5: Strict TDD evidence table in `apply-progress.md` has no TRIANGULATE or SAFETY NET columns. Batch A scaffold was written before its RED tests (disclosed).
- W6: Spec scenario "no tracked file under site/ changes" (S-test) has no assertion.

**SUGGESTION** (nice to have):
- S1: Automate the S-copy grep as a `check-dist` assertion so copy regressions fail `npm test`.
- S2: Add a coverage or lint step for `site/` if the site grows (`node --test --experimental-test-coverage` is available with the current runner).
- S3: Enable GitHub Pages (source: GitHub Actions) before the first `main` deploy. The post-merge step is still open.

---

**Verdict**: PASS WITH WARNINGS

0 CRITICAL. All 29 tasks complete. All build, test and Rust checks pass. Lighthouse: every score >= 95 except one desktop landing run (82), which reran at 100. Eight scenarios need a screenshot or a GitHub run to close fully (W2, W3).
