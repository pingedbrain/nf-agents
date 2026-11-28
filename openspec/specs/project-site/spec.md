# project-site Specification

## Purpose

Landing page and generated docs for baqueano, built from `site/` (Astro + Starlight) at `https://pingedbrain.github.io/baqueano/` (`base: /baqueano`). Public text is English and matches README.md and ROADMAP.md.

## Verification checks

| ID | Kind | Command / name |
|----|------|----------------|
| S-build | test | `npm run build` in `site/` |
| S-test | test | `npm test` in `site/` (sync transform + `dist/` checks); CI then fails if `site/` has any git change |
| S-copy | test | `checkCopy` in `scripts/check-dist.mjs` (banned and required copy) |
| S-lh | verify | Lighthouse a11y and performance ≥ 95, desktop and mobile |
| S-shot | verify | screenshots at desktop and 375 px |

## Requirements

### Requirement: Docs generated from repository sources

The site MUST generate `/docs/` pages from `docs/demo.md`, `docs/architecture.md`, `crates/baqueano-mcp/README.md`, `docs/examples/baqueano-replay/README.md` and `ROADMAP.md`. Generated output MUST be gitignored.

#### Scenario: Source edit and links (S-test)
- GIVEN a source change and a source with a mapped-doc link and a repo-file link
- WHEN synced and built
- THEN the change appears, the doc link resolves under `/baqueano/docs/`, the repo link becomes a GitHub URL, and no tracked file under `site/` changes

#### Scenario: H1 and missing source (S-test)
- GIVEN a source whose first line is an H1
- THEN the H1 becomes the frontmatter title and the page has exactly one `h1`
- GIVEN a mapped source file that is missing
- THEN the build fails naming the file

### Requirement: Copy matches the repository

Public copy MUST state only what README.md and ROADMAP.md support. These six corrections MUST hold:

| Reference text | Required site text |
|----------------|--------------------|
| `docker run ghcr.io/[ORG]/baqueano-demo` | `git clone … && demo/run.sh` |
| sarek terminal example | real demo diff (`params.greeting`) |
| `nf-evals`, modules "In progress" | planned for v0.2; scores `pending` |
| leaderboard "v0.1", explorer "v0.2" | leaderboard v0.3, explorer v0.6 |
| compute / Bedrock / NIST as current | roadmap items v0.4 and v0.5 |
| location in footer | removed |

#### Scenario: Corrections in dist (S-copy)
- GIVEN the built `dist/`
- THEN each required text appears and no reference text (left column) appears
- AND every unmeasured score shows `pending`
- AND the old name `nf-agents` does not appear, except inside the fixture label `nf-agents-fixture`

### Requirement: Brand, footer and claims

`baqueano` MUST be lowercase in public text. The site MUST NOT show Nextflow, Seqera or cloud-provider logos, MUST NOT make clinical claims, and every page MUST render the mandatory footer disclaimer. Brand tokens MUST define light and dark sets; under `prefers-color-scheme: dark` body text contrast MUST be at least 4.5:1.

#### Scenario: Footer and logos (S-test)
- GIVEN every generated HTML page
- THEN each contains the footer disclaimer and no Nextflow, Seqera or cloud-provider logo is referenced

#### Scenario: Dark contrast (S-lh)
- GIVEN the page under a dark colour scheme
- THEN dark tokens apply and body text contrast is at least 4.5:1

### Requirement: No third-party analytics or external assets

The site MUST NOT load analytics, font CDNs or other external resources. Fonts MUST be bundled from `@fontsource` at build time.

#### Scenario: No external hosts (S-test)
- GIVEN `dist/`
- WHEN script, stylesheet, image and font references are inspected
- THEN none points to a host other than the site origin

### Requirement: Hero, OG image and reduced motion

The hero MUST be an inline SVG with a CSS pipeline flow. Under `prefers-reduced-motion: reduce` all hero animation MUST stop and the static diagram MUST remain. The site MUST provide a 1200x630 Open Graph image (wordmark and tagline) referenced under the `/baqueano/` base.

#### Scenario: Reduced motion (S-shot)
- GIVEN reduced motion requested
- THEN no hero animation runs and the diagram is fully visible

#### Scenario: OG image (S-test)
- GIVEN `dist/`
- THEN the OG image is 1200x630 and `og:image` carries the `/baqueano/` base

### Requirement: Responsive layout and accessibility

Every page MUST work at 375 px with no horizontal scroll and a 16 px side gutter, and MUST have exactly one `h1`. Lighthouse accessibility and performance MUST each be ≥ 95 on desktop and mobile.

#### Scenario: Phone width (S-shot, S-lh)
- GIVEN a 375 px viewport and the deployed site
- THEN no horizontal scroll occurs, a 16 px gutter is kept, and Lighthouse scores are each ≥ 95
