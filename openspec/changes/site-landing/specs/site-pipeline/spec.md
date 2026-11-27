# site-pipeline Specification

## Purpose

`.github/workflows/site.yml` checks the site on pull requests and deploys it to GitHub Pages on push to `main`. It does not modify `ci.yml`.

## Verification checks

| ID | Kind | Command / name |
|----|------|----------------|
| P-check | CI | a PR touching `site/**` shows the `check` job green |
| P-deploy | CI | a push to `main` runs `deploy` after `check` |
| P-yaml | verify | inspect `.github/workflows/site.yml` (grep/yq; `actionlint` if available) |

## Requirements

### Requirement: Check on pull requests

The workflow MUST run `check` on pull requests that change `site/**` or `.github/workflows/site.yml`. `check` MUST run `npm ci`, `npm run build` and `npm test` in `site/` and MUST fail if any step fails.

#### Scenario: Site change (P-check)
- GIVEN a PR that edits a file under `site/`
- WHEN the workflow runs
- THEN `check` runs the three steps and passes when all succeed

#### Scenario: Broken prefix (P-check)
- GIVEN a PR that adds a link missing the `/nf-agents` base
- WHEN `npm test` runs the `dist/` check
- THEN `check` fails

#### Scenario: Unrelated PR (P-yaml)
- GIVEN a PR that touches none of `site/**`, `.github/workflows/site.yml`, the five doc sources, or `brand/logo/**`
- THEN the path filter prevents `check` from running on that PR

### Requirement: Check on push to main

`check` MUST also run on every push to `main`, so it can be required without a path filter blocking it.

#### Scenario: Push without site changes (P-deploy)
- GIVEN a push to `main` that touches no site file
- WHEN the workflow runs
- THEN `check` runs and reports its result

### Requirement: Deploy on push to main

`deploy` MUST run only on push to `main`, MUST depend on a successful `check`, and MUST publish the built `site/dist` with GitHub Pages (source: GitHub Actions).

#### Scenario: PR does not deploy (P-yaml)
- GIVEN a pull request event
- THEN the `deploy` job is skipped

#### Scenario: Failed check blocks deploy (P-deploy)
- GIVEN `check` fails on a push to `main`
- THEN `deploy` does not run

#### Scenario: Pages not enabled (P-deploy)
- GIVEN GitHub Pages is not enabled with source GitHub Actions
- WHEN `deploy` runs
- THEN the job fails and the error names the missing Pages setup

### Requirement: Least privilege and no secrets

Top-level `permissions` MUST be `contents: read`. Only `deploy` MAY raise permissions, to `pages: write` and `id-token: write`. The workflow MUST NOT reference `secrets.*`.

#### Scenario: Permissions (P-yaml)
- GIVEN `site.yml`
- THEN the top-level block is `contents: read`, `deploy` is the only job with `pages: write`, and `grep -c 'secrets\.'` is 0

### Requirement: Pinned actions and deploy concurrency

Every `uses:` line MUST reference a version tag (`@vN` or more specific) or a 40-hex commit SHA. The deploy concurrency group MUST be `pages` and MUST NOT cancel a deployment in progress.

#### Scenario: Action refs (P-yaml)
- GIVEN every `uses:` line
- THEN each ref matches `@v[0-9]…` or a 40-hex SHA

#### Scenario: Overlapping pushes (P-yaml)
- GIVEN two pushes to `main` in quick succession
- WHEN the second deploy is queued
- THEN the first deployment is not cancelled
