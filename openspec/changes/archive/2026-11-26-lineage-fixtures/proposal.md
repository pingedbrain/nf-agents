# Proposal: Lineage fixtures (stores + CLI oracle per Nextflow version)

## Intent

`nf-lineage` (parser, `check`, contract tests) and `nf-lineage-mcp` need real Nextflow lineage
stores and the `nextflow lineage` CLI output for every supported format generation, available
offline in a git checkout. Today there are none; the lab copies leak host paths, use absolute
symlinks and were tampered in place (see `exploration.md`).

## Scope

### In Scope
- `fixtures/pipelines/minimal/`: 3-process pipeline (GENERATE → TRANSFORM → SUMMARIZE), synthetic
  seed, `output` block; 25.04 gets `nextflow.preview.output = true` prepended by the generator.
- `scripts/gen-fixtures.sh` (host: bash + Docker only) + python 3.9 stdlib helpers run **inside**
  the Nextflow image: per version 25.04.8 / 25.10.9 / 26.04.9 / 26.10.7, two runs
  (`--greeting hello|bonjour`) at the fixed path `/nf-agents-fixture`, `--cpus 2`; every
  `nextflow lineage` subcommand captured (`args`/`out`/`err`/`exit`); tamper cases on a throw-away copy.
- `fixtures/lineage/<ver>/`: `.lineage/`, FileOutput-referenced files only (dereferenced, no
  symlinks), `oracle/`, `manifest.json` (path → size, mtime_ms, sha256), `shape.txt`.
- Generation-time round trip: restore mtimes from the committed layout, `nextflow lineage check`
  every `FileOutput`, `list` equals the oracle.
- `fixtures/lineage/26.10.7-agent/`: `AgentRun` store from 3 agents against an in-container
  stdlib LLM stub (no network to any LLM, no keys).
- `fixtures/README.md`: provenance, Apache-2.0, synthetic data, regeneration, mtime contract.
- `.github/workflows/fixtures.yml`: `workflow_dispatch` + weekly; PR only if `shape.txt` changes
  (or `force`); non-blocking `26.11.0-edge` shape report.
- `ci.yml`: offline well-formedness check of `fixtures/` (JSON, manifest sizes/sha256, no host paths).
- `.gitignore` negation for `fixtures/lineage/*/work/`; `.gitattributes` `-text` for fixtures.

### Out of Scope
- Any Rust code (parser, mtime-restore helper, contract tests) → change `nf-lineage`; this change
  fixes the **contract** they consume.
- Committing `render` HTML, run logs, `.nextflow/`, task scaffolding files.
- nf-core/sarek store; S3-backed store (open item in `docs/verification.md`); `nf-llm-replay` cassettes.

## Capabilities

### New Capabilities
- `lineage-fixtures`: minimal pipelines, generator, store/oracle/manifest/shape layout, agent
  store, README, round-trip verification, offline well-formedness check.

### Modified Capabilities
- `ci-pipeline`: `ci.yml` gains the fixtures check step; new `fixtures.yml` workflow with its own
  (write) permissions, triggers and PR policy.

## Approach

Path-stable, machine-independent records (fixed mount, CPUs, image), drift detected on a
normalized `shape.txt` because task hashes embed the session UUID (verified: not pinnable).

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `fixtures/pipelines/{minimal,agent}/` | New | pipelines, seed, stub |
| `fixtures/lineage/*` | New (generated) | 5 stores, ≈ 60 KB each incl. oracle |
| `fixtures/README.md` | New | provenance/regeneration |
| `scripts/gen-fixtures.sh`, `scripts/fixtures/*` | New | generator + helpers |
| `.github/workflows/{fixtures,ci}.yml` | New / Modified | regeneration; check step |
| `.gitignore`, `.gitattributes` | Modified / New | keep `work/`, no EOL conversion |
| Rust crates | None | — |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Bot PRs (GITHUB_TOKEN) don't trigger `ci.yml` | High | documented; maintainer re-runs/pushes; PR body says so |
| `python3` removed from a future image | Low | guard with clear error |
| `nf-agent` plugin download / preview drift | Med | pin `nf-agent@0.2.0`; agent store optional per run |
| EOL/whitespace normalization breaks sizes | Med | `.gitattributes -text` + sha256 in manifest + CI check |

## Rollback Plan

Revert the PR(s): delete `fixtures/`, `scripts/gen-fixtures.sh`, `scripts/fixtures/`,
`fixtures.yml`, the `ci.yml` step, `.gitattributes` and the `.gitignore` negation. No crate
depends on them yet; disable `fixtures.yml` from the Actions UI if a bad PR loop appears.

## Dependencies

Docker; images `nextflow/nextflow:{25.04.8,25.10.9,26.04.9,26.10.7}`; plugin `nf-agent@0.2.0`.

## Success Criteria

- [ ] `scripts/gen-fixtures.sh` regenerates all stores from a clean checkout with only bash + Docker.
- [ ] Round-trip `check` succeeds for every `FileOutput` of every committed store.
- [ ] CI fixtures check passes; no `/home/`, `/Users/` or user names in `fixtures/`.
- [ ] Second consecutive regeneration yields an identical `shape.txt` (no PR).
- [ ] Each PR ≤ 800 hand-written lines; workspace stays green.
