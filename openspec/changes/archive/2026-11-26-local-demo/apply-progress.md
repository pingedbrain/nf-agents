# Apply progress: local-demo

Mode: Strict TDD. Batch 1 = PR A (2026-11-26). Batch 2 = PR B (2026-11-26): README + `docs/demo.md`.

## Tasks

- [x] A.1–A.12 (see tasks.md)
- [ ] A.13 C-demo green on the PR's CI run (needs the PR)
- [x] B.1–B.4 (batch 2)

## TDD Cycle Evidence

| Task | RED | GREEN | REFACTOR / triangulation |
|------|-----|-------|--------------------------|
| A.1–A.3 tour | `demo/test_tour.py` written first; `unittest` failed (no `tour` module) | `demo/tour.py`; 6/6 `TourTests` pass | 6 cases: happy path, extra diff change, `isError`, EOF, wrong answer shape, edit guard |
| A.4–A.5 run.sh early exits | 3 `RunScriptTests` failed (exit 127, no script) | `demo/run.sh`; 9/9 pass (Python 3.12 host, 3.9 in the Nextflow image) | foreign workdir, Docker down, record without `UPSTREAM` |
| A.6–A.8 pipeline, cassette | n/a (pipeline/config; checked end to end) | probe runs, then `run.sh` record + replay green | recorded twice: byte-identical (sha256 `dd4eed43…`) |
| A.9 secrets | injected `/demo/` into the demo cassette → test failed naming the file and needle | restored → 2/2 pass | also bans `/demo/` and `/src/` |
| A.10–A.11 wiring | n/a (config) | actionlint exit 0, shellcheck clean | — |
| B.1–B.4 docs | n/a (docs only; `docs/demo.md` did not exist) | README `## Try it in 5 minutes` + `docs/demo.md`; every DM-doc literal present | transcript pasted from a fresh `demo/run.sh` run (exit 0, 24 s), host path replaced |

## Manual checks (Docker)

| Check | Result |
|-------|--------|
| DM-run, `:ci` overrides | exit 0, 21 s; tour 6/6; all workdir files owned by the invoking user; no containers/networks left |
| DM-run, images built by `run.sh` | exit 0, 180 s on first build (base and Nextflow images cached), 24 s warm |
| DM-miss (one cassette line removed) | exit 1, replay `outcome="miss"` lines + the `DEMO_RECORD=1 … demo/run.sh` hint; cleanup done |
| DM-record (fake upstream, twice) | cassette byte-identical |

| DM-run, fresh for PR B docs | exit 0, 24 s warm (25 s on a first repeat); output pasted into `docs/demo.md` |

Measured on a 12-thread desktop (Linux, Docker Engine). Machine: 16 GB RAM, Docker Engine 29.7. Not measured: cold with every image removed (would need a global `docker builder prune`), published images (pending release), Apple Silicon.

## Deviations

See design.md "Deviations (2026-11-26)" and the dated notes in spec.md and tasks.md.

PR B also refreshed the status markers: README "Status" and ROADMAP v0.1 list `nf-lineage-mcp` and the local demo as **in review**; "Release pipeline" stays planned.
