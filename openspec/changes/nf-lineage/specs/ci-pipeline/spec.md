# Delta for ci-pipeline

Change `nf-lineage` adds two PR jobs and extends the supply-chain policy for `object_store`'s
dependency tree. (`lineage-fixtures` also edits this capability; archive whichever lands second as a merge.)

## ADDED Requirements

### Requirement: Contract job

Job `contract` MUST run on every PR and push to `main`, install the pinned toolchain, and run
`cargo test -p nf-lineage --test contract --locked` against the committed `fixtures/lineage/*` oracles.
It MUST NOT need Docker, network beyond crates.io, or secrets. (The `test` job also runs these
tests; `contract` exists as a named status check.)

#### Scenario: Oracle mismatch (V-yaml, C-contract)
- GIVEN a change that makes `find` return a different LID set for one fixture
- WHEN CI runs
- THEN `contract` fails naming the version and case

### Requirement: Integration job

Job `integration` MUST run on every PR and push to `main` with a service container
`ministackorg/ministack:1.5.45` on port 4566, set `NF_LINEAGE_MINISTACK_URL=http://localhost:4566`,
and run `cargo test -p nf-lineage --test s3_ministack --locked`. It MUST use only dummy credentials
(`test`/`test`) defined in the workflow, never `secrets.*`. The tests themselves MUST wait for
MiniStack readiness (bounded, ≤ 60 s) and fail, not skip, when the variable is set but MiniStack is unreachable.

#### Scenario: Service up (C-it, V-yaml)
- GIVEN the PR run
- WHEN `integration` runs
- THEN both `s3_ministack` tests execute (not skipped) and pass

#### Scenario: No secrets (V-yaml)
- GIVEN `ci.yml`
- THEN `grep -c 'secrets\.'` is 0 and the service image is pinned to `1.5.45`

## MODIFIED Requirements

### Requirement: Supply-chain policy

`deny.toml` MUST: allow only permissive licenses compatible with Apache-2.0 (no GPL/LGPL/AGPL/SSPL); deny crates with known vulnerabilities and yanked versions; deny wildcard version requirements; allow only the crates.io registry and no git sources. Licenses outside the allowlist MAY be accepted only as crate-scoped `exceptions` with a comment naming the crate and why. Advisories MAY be ignored only by explicit ID with a `reason` (initially RUSTSEC-2026-0542 `humantime` and RUSTSEC-2026-0543 `chrono`, unmaintained, pulled by `object_store`).
(Previously: no license exceptions and no advisory ignores were allowed.)

#### Scenario: Clean tree (C-deny)
- GIVEN the workspace as shipped
- WHEN `cargo deny check` runs
- THEN it exits 0

#### Scenario: Policy content (V-deny)
- GIVEN `deny.toml`
- WHEN inspected
- THEN the license allowlist has no copyleft ids, `unknown-registry` and `unknown-git` are `deny`, `allow-registry` is crates.io only

#### Scenario: Scoped exceptions (V-deny)
- GIVEN `deny.toml`
- WHEN inspected
- THEN every `exceptions` entry names one crate and has a comment, and every advisory ignore has an ID and `reason`
