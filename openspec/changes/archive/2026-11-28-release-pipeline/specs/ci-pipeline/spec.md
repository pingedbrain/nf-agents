# Delta for ci-pipeline

## MODIFIED Requirements

### Requirement: Test job

Job `test` MUST run `cargo test --workspace --locked`, the offline fixture checks (C-fixtures), the demo checks (C-demo-unit: tour unittest and `shellcheck demo/run.sh`), the release metadata check `scripts/release/check.sh` (R-check) and `cargo package --workspace --locked` (R-pkg). It MUST fail if any of them fails. The fixture and demo checks MUST NOT need Docker or network. The packaging step needs only the crates.io index, as the `contract` job does. Neither the release check nor the packaging step publishes anything.
(Previously: `test` ran only `cargo test`, the fixture checks and the demo checks.)

#### Scenario: Failing test (C-run, V-yaml)
- GIVEN a failing test in any member
- WHEN `test` runs
- THEN the job fails

#### Scenario: Corrupted fixture (C-fixtures)
- GIVEN a PR that changes one byte of a data file under `fixtures/lineage/`
- WHEN `test` runs
- THEN the job fails naming the file

#### Scenario: Broken tour logic (C-demo-unit)
- GIVEN a PR that makes `tour.py` accept a diff with extra changes
- WHEN `test` runs
- THEN `TourTests.test_unexpected_diff_fails` fails the job

#### Scenario: Missing crate keyword (R-check)
- GIVEN a PR that removes `keywords` from a publishable crate
- WHEN `test` runs
- THEN `scripts/release/check.sh` fails the job naming the crate

#### Scenario: Unpackageable crate (R-pkg)
- GIVEN a PR that makes a crate depend on an unpublished path-only crate without a version
- WHEN `test` runs
- THEN `cargo package --workspace --locked` fails the job

#### Scenario: No publish from CI (V-yaml)
- GIVEN `ci.yml`
- THEN it contains no `cargo publish`, no `CARGO_REGISTRY_TOKEN` and no `secrets.` reference
