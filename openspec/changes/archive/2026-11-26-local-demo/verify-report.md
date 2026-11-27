# Verification report: local-demo (PR A + PR B)

Mode: Strict TDD. Date: 2026-11-26.

## Completeness

PR A: 12/13 done; A.13 (C-demo on a CI run) is a post-merge follow-up. PR B: 4/4 done.

## Execution

| Command | Result |
|---------|--------|
| `cargo fmt --all -- --check` | ok |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | ok |
| `cargo test --workspace --locked` | ok, 0 failed (incl. `secrets::committed_cassettes_contain_no_credentials`) |
| `cargo deny check` | advisories, bans, licenses, sources ok |
| `cargo machete` | no unused dependencies |
| `python3 -I -m unittest discover -s demo -p 'test_*.py'` (DM-unit) | 9/9 ok on Python 3.12 and 3.9.25 |
| `shellcheck demo/run.sh` (DM-sh, `koalaman/shellcheck:stable`) | clean |
| `actionlint` (V-yaml) | exit 0; `docker` job still has no push/login/`secrets.`/`packages: write` |
| DM-run, DM-miss, DM-record | see apply-progress.md |

## Spec compliance

| Requirement | Scenario | Evidence | Result |
|-------------|----------|----------|--------|
| Entry point | No Docker | `RunScriptTests.test_fails_fast_without_docker` | COMPLIANT |
| Entry point | Record without upstream | `RunScriptTests.test_record_requires_upstream` | COMPLIANT |
| Entry point | CI images | DM-run locally with `:ci` overrides (no build) | COMPLIANT (CI run pending) |
| Workdir safety | Foreign directory | `RunScriptTests.test_refuses_foreign_workdir` | COMPLIANT |
| Pipeline and replay | Offline answers | DM-run: both runs print `QC verdict: [answer:no, confidence:0.9]`, 0 misses | COMPLIANT |
| Pipeline and replay | Cassette drift | DM-miss | COMPLIANT |
| Cassette | Secret-free | `committed_cassettes_contain_no_credentials` | COMPLIANT |
| Re-recording | Deterministic regeneration | DM-record | COMPLIANT |
| MCP tour | Happy path / unexpected diff / tool error / answer shape / edit guard / server gone | 6 `TourTests` + DM-run | COMPLIANT |
| Claude Code hand-off | Printed | DM-run stdout; CI step greps it | COMPLIANT |
| Honesty and docs | Docs (DM-doc, DM-time) | DM-doc grep; timings table in `docs/demo.md` | COMPLIANT |
| ci-pipeline | C-demo / C-demo-unit / V-yaml | steps added; actionlint; local runs | COMPLIANT (CI run pending) |

## Verification: PR B scope (2026-11-26)

| Command | Result |
|---------|--------|
| `cargo fmt --all -- --check`, `cargo clippy … -D warnings`, `cargo test --workspace --locked` | ok (0 failed) |
| `cargo deny check`, `cargo machete` | ok |
| DM-unit (`unittest`), DM-sh (`shellcheck demo/run.sh`) | ok |
| DM-run (fresh, default images) | exit 0, 24 s; transcript in `docs/demo.md` |
| DM-doc (README and `docs/demo.md`: `demo/run.sh`, `claude mcp add`, `research use`, `synthetic`, `MCP_IMAGE`, `DEMO_RECORD=1`, warm 24 s) | all present in both files |
| DM-time | warm 24 s, `:ci` overrides 21 s, first build 180 s; fully cold, published images, CI step, Apple Silicon, WSL2 stated as not measured or pending |
| Public-content grep over the tree | empty; `.atl/` not tracked |

Honesty and docs: COMPLIANT. `docs/demo.md` states the committed cassette was recorded from the deterministic fake upstream (resolves the PR A suggestion).

## Issues

- CRITICAL: none.
- WARNING: C-demo is only proven locally until a CI run of the `Local demo` step (A.13); the CI step time is still "pending" in `docs/demo.md`. Fully cold timing not measured (would need a host-wide build cache prune).
- SUGGESTION: fill the CI step time into `docs/demo.md` after the first CI run.

## Verdict

PASS WITH WARNINGS (PR A + PR B).
