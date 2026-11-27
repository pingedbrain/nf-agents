# Verification report: local-demo (PR A scope)

Mode: Strict TDD. Date: 2026-11-26.

## Completeness

PR A: 12/13 done; A.13 (C-demo on the PR's CI run) needs the PR. PR B (B.1–B.4) not in scope.

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
| Honesty and docs | Docs (DM-doc, DM-time) | PR B | OUT OF SCOPE |
| ci-pipeline | C-demo / C-demo-unit / V-yaml | steps added; actionlint; local runs | COMPLIANT (CI run pending) |

## Issues

- CRITICAL: none.
- WARNING: C-demo is only proven locally until the PR's CI run; DM-doc/DM-time belong to PR B.
- SUGGESTION: `docs/demo.md` (PR B) must state the committed cassette was recorded from the deterministic fake upstream.

## Verdict

PASS WITH WARNINGS (PR A scope).
