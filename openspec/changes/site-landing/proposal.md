# Proposal: site-landing

## Intent

No public site exists. Add a landing page and docs at https://pingedbrain.github.io/nf-agents/, deployed by CI, with copy that matches the repo.

## Scope

### In Scope
- `site/`: Astro + Starlight; `site: https://pingedbrain.github.io`, `base: /nf-agents`; docs at `/docs/`, landing at `/`.
- Brand tokens (light/dark), `@fontsource` fonts, `brand/logo/` assets, mandatory footer.
- Docs generated from repo sources (single source of truth, output gitignored):

  | Page | Source |
  |---|---|
  | Getting started | `docs/demo.md` |
  | Architecture | `docs/architecture.md` |
  | nf-lineage-mcp | `crates/nf-lineage-mcp/README.md` |
  | nf-llm-replay | `docs/examples/nf-llm-replay/README.md` |
  | Roadmap | `ROADMAP.md` |

- Landing sections from the reference, copy checked against the repo (table below).
- Hero: inline SVG + CSS pipeline flow (particles on edges, glowing agent diamonds), static under `prefers-reduced-motion`.
- 1200x630 Open Graph image (wordmark + tagline).
- `.github/workflows/site.yml`: `check` on PRs touching site sources; `deploy` to Pages on push to `main`.

### Out of Scope
- Leaderboard, lineage explorer, module catalog, link checker, custom domain, analytics.

### Copy corrections vs reference

| Reference | Site |
|---|---|
| `docker run ghcr.io/[ORG]/nf-agents-demo` | `git clone … && demo/run.sh` (README) |
| sarek terminal example | real demo diff (`params.greeting`) |
| `nf-evals`, modules "In progress" | planned, v0.2; scores `pending` |
| leaderboard "v0.1", explorer "v0.2" | v0.3, v0.6 |
| compute/Bedrock/NIST | roadmap (v0.4, v0.5) |
| location in footer | removed |

## Capabilities

### New Capabilities
- `project-site`: landing, docs generation, brand, a11y, performance, OG image, copy rules.
- `site-pipeline`: PR check and Pages deployment workflow.

### Modified Capabilities
- None (`ci.yml` untouched).

## Approach

- Prebuild `scripts/sync-docs.mjs`: copy sources, H1 → frontmatter title, rewrite relative links (docs → site routes, others → GitHub URLs).
- `npm test` = `node --test` (sync transform) + post-build check of `dist/` (single `h1`, footer, no external font/analytics hosts, `/nf-agents/` prefixes, OG size). No new test framework.
- Lighthouse and screenshots measured in verify, not CI.

| PR | Content | Est. lines |
|---|---|---|
| A | scaffold, tokens, fonts, docs sync + test, placeholder landing, `site.yml` | ~450 |
| B | landing sections, responsive, a11y | ~700 |
| C | hero animation, OG image | ~400 |

Each PR builds and deploys on its own.

## Affected Areas

| Area | Impact |
|---|---|
| `site/` | New |
| `.github/workflows/site.yml` | New |
| `ROADMAP.md` | docs-site item → **in progress** |

No crates affected.

## Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| Broken links under `base` | Med | post-build prefix check |
| Rewritten doc links break | Med | transform unit test |
| Path-filtered check can't be required | Low | `check` also runs on push to `main` |
| Unsupported claims in copy | Low | copy table above; review in verify |

## Rollout

1. Merge PR A.
2. Enable Pages, source "GitHub Actions" (`gh api -X POST repos/pingedbrain/nf-agents/pages -f build_type=workflow`).
3. Merge B, then C.

## Rollback Plan

Revert the PRs or disable Pages; nothing depends on `site/`.

## Dependencies

- GitHub Pages enabled (rollout step 2).

## Success Criteria

- [ ] `npm run build` and `npm test` pass in `site/` locally and in CI.
- [ ] Site live; docs pages generated from repo sources.
- [ ] Lighthouse ≥ 95 accessibility and performance, desktop and mobile.
- [ ] Screenshots at desktop and 375 px.
- [ ] No claim beyond README/ROADMAP; unbuilt scores show `pending`.
- [ ] Each PR ≤ ~800 hand-written lines.
