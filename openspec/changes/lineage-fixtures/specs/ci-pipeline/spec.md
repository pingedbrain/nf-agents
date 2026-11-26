# Delta for ci-pipeline

## Verification checks (added)

| ID | Kind | Command / name |
|----|------|----------------|
| C-fixtures | CI `test` | `python3 -I -m unittest discover -s scripts/fixtures -p 'test_*.py'` and `python3 -I scripts/fixtures/check.py fixtures/lineage` |
| V-fx-yaml | verify | inspect `.github/workflows/fixtures.yml` (grep/yq; `actionlint` if available) |
| C-fx-run | CI (manual) | a `workflow_dispatch` run of `fixtures.yml` on the PR branch or after merge |

## ADDED Requirements

### Requirement: Fixtures regeneration workflow

`.github/workflows/fixtures.yml` MUST trigger only on `workflow_dispatch` (inputs `force`:
boolean, default `false`; `edge`: string, default `26.11.0-edge`) and on a weekly `schedule`;
it MUST NOT trigger on `pull_request` or `push`. Top-level `permissions` MUST be
`contents: read`; only the regeneration job MAY raise them, to exactly `contents: write` and
`pull-requests: write`. It MUST NOT reference any secret other than the built-in `GITHUB_TOKEN`
and MUST NOT use third-party actions (only `actions/*` plus the preinstalled `gh` CLI). Runs MUST
NOT overlap (`concurrency` group without cancelling a run in progress).

#### Scenario: Triggers and permissions (V-fx-yaml)
- GIVEN `fixtures.yml`
- THEN `on` has exactly `workflow_dispatch` and `schedule`, top-level permissions are `contents: read`, and `grep -E 'secrets\.' | grep -v GITHUB_TOKEN` is empty

#### Scenario: Action allowlist (V-fx-yaml)
- GIVEN every `uses:` line
- THEN each starts with `actions/` and has a `@vN` or 40-hex ref

### Requirement: Regeneration opens a PR only on drift

The regeneration job MUST run `scripts/gen-fixtures.sh` (which includes the round trip), then
C-fixtures. If any `fixtures/lineage/*/shape.txt` differs from the checked-out ref, or `force`
is true, it MUST commit `fixtures/` to branch `automation/lineage-fixtures`, force-push it and
open (or update the existing) PR against the triggering ref, with a body listing the changed
versions, the image digests and a note that CI must be re-triggered by a maintainer because
`GITHUB_TOKEN` pushes do not start workflows. Otherwise it MUST NOT push or open a PR and MUST
write "no format drift" to the job summary. Any generation or check failure MUST fail the job
without pushing.

#### Scenario: No drift (C-fx-run)
- GIVEN fixtures generated with the same images
- WHEN the weekly run executes
- THEN the job succeeds, nothing is pushed and the summary says "no format drift"

#### Scenario: Drift (C-fx-run, V-fx-yaml)
- GIVEN a `shape.txt` change (or `force: true`)
- WHEN the run executes
- THEN exactly one open PR from `automation/lineage-fixtures` exists afterwards, containing only `fixtures/` changes

#### Scenario: Generation failure (V-fx-yaml)
- GIVEN `scripts/gen-fixtures.sh` exits non-zero
- THEN the job fails and no push step runs (steps gated on success)

### Requirement: Edge early warning

A separate job MUST, on `schedule` and on `workflow_dispatch` with a non-empty `edge` input,
generate fixtures for that edge tag into a temporary directory, and write the diff between its
`shape.txt` and `fixtures/lineage/26.10.7/shape.txt` to the job summary. It MUST be non-blocking
(`continue-on-error: true`), MUST have `contents: read` only and MUST NOT commit, push or open PRs.

#### Scenario: Edge drift reported (C-fx-run, V-fx-yaml)
- GIVEN the edge image emits a new checksum `mode`
- WHEN the edge job runs
- THEN the summary shows the shape diff and the workflow result is not failed by this job

## MODIFIED Requirements

### Requirement: Least privilege, no secrets

`ci.yml` MUST declare `permissions: contents: read` at top level and MUST NOT reference `secrets.*`. It MUST NOT need API keys or cloud credentials. (`fixtures.yml` is governed by "Fixtures regeneration workflow".)
(Previously: stated for "the workflow" when `ci.yml` was the only workflow.)

#### Scenario: Fork PR (V-yaml, C-run)
- GIVEN a PR from a fork (no secrets available)
- WHEN CI runs
- THEN both jobs can pass; `grep -c 'secrets\.' ci.yml` is 0

### Requirement: Test job

Job `test` MUST run `cargo test --workspace --locked` and the offline fixture checks (C-fixtures), and MUST fail if any of them fails. The fixture checks MUST NOT need Docker or network.
(Previously: only `cargo test --workspace --locked`.)

#### Scenario: Failing test (C-run, V-yaml)
- GIVEN a failing test in any member
- WHEN `test` runs
- THEN the job fails

#### Scenario: Corrupted fixture (C-fixtures)
- GIVEN a PR that changes one byte of a data file under `fixtures/lineage/`
- WHEN `test` runs
- THEN the job fails naming the file
