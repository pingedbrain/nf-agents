# contributor-guide Specification

## Purpose

`CONTRIBUTING.md` tells a new contributor how to set up, what to run before a PR, and the project rules, in English.

## Verification checks

| ID | Kind | Command / name |
|----|------|----------------|
| V-doc | verify | `grep` for each required item in `CONTRIBUTING.md` |
| V-links | verify | every relative link in `CONTRIBUTING.md` resolves to an existing file |

## Requirements

### Requirement: Required content

`CONTRIBUTING.md` MUST exist at the repo root, be written in English, and MUST cover:

| # | Item | V-doc grep (literal) |
|---|------|----------------------|
| 1 | Dev setup: rustup (toolchain from `rust-toolchain.toml`), `cargo install cargo-deny cargo-machete` | `rust-toolchain.toml`, `cargo-deny`, `cargo-machete` |
| 2 | Pre-PR commands, identical to CI | `cargo fmt --all`, `cargo clippy --workspace --all-targets`, `cargo test --workspace`, `cargo deny check`, `cargo machete` |
| 3 | Small PRs, one task each, tests in the same PR | `small`, `same PR` |
| 4 | AI-usage declaration in every PR | `AI` |
| 5 | No secrets, `.env`, or real patient/genomic data | `secrets`, `patient` |
| 6 | Read-only by default principle | `read-only` |
| 7 | Spec-driven flow in `openspec/`: proposal → specs → design → tasks → apply → verify → archive; every acceptance criterion maps to a test | `openspec/`, `proposal`, `archive`, `acceptance criterion` |
| 8 | Links to code of conduct and security policy | `CODE_OF_CONDUCT.md`, `SECURITY.md` |

#### Scenario: Newcomer finds the pre-PR commands (V-doc)
- GIVEN a contributor reading `CONTRIBUTING.md`
- WHEN they look for what to run before a PR
- THEN all five commands of item 2 appear verbatim and match CI

#### Scenario: Every item present (V-doc)
- GIVEN `CONTRIBUTING.md`
- WHEN each literal in the table is grepped
- THEN every grep matches

#### Scenario: Links resolve (V-links)
- GIVEN the relative links in `CONTRIBUTING.md`
- WHEN each target path is checked
- THEN every target exists in the repo

### Requirement: Commands stay in sync with CI

The pre-PR commands in `CONTRIBUTING.md` MUST be the same commands (same flags) as the `lint` and `test` jobs run.

#### Scenario: CI flag changes (V-doc, V-yaml)
- GIVEN a change to a command in `ci.yml`
- WHEN the change is verified
- THEN `CONTRIBUTING.md` shows the same command or verify reports a mismatch
