# Proposal: nf-lineage 0.1

## Intent

Nothing in the workspace reads Nextflow lineage. `nf-lineage-mcp` (next) and stage-2 crates need a typed, tolerant, cloud-agnostic reader with CLI-parity `list/view/find`, a semantic `diff`, a real `check` and `provenance`.

## Scope

### In Scope
- `Lid`, `Record` (6 kinds + `Unknown`), both envelopes, `extra` preserved, `UnsupportedVersion`, `LineageError`.
- `FindQuery` with `nextflow lineage find` semantics.
- `#[async_trait] LineageStore` (`list_runs`, `get`, `find`); `ObjectLineageStore` over `object_store` 0.14 (local/S3/GCS/Azure/InMemory), `open(url)` + options/env.
- `check` (`nextflow`/`standard` murmur3) for local and `s3://` data; others → `Unsupported`.
- `diff` (semantic) + `diff_raw` (CLI-comparable); `view` (fragments); `provenance`.
- Tests: unit, insta over all fixtures, CLI contract, MiniStack S3, doctests. CI `contract`, `integration`.
- Crate `0.1.0`, publishable (metadata, README); not published.

### Out of Scope
- Publishing, `release` job, MCP server, `render`, writes of any kind.
- Checksum modes other than `standard`; `check` on `gs://`/`az://`.
- GCS/Azure emulator tests; sarek fixtures; generating fixtures (`lineage-fixtures`).

## Capabilities

### New Capabilities
- `nf-lineage`: identifiers, record model/parsing, store contract + object_store backend, find/view/diff/check/provenance, error model, packaging.

### Modified Capabilities
- `ci-pipeline`: jobs `contract`, `integration`; deny advisory ignores + crate-scoped license exceptions.
- `rust-workspace`: "nf-lineage stub" requirement retired.
- `contributor-guide`: how to run the MiniStack test.

## Approach

Exploration approach 1: typed all-`Option` structs + hand-written envelope step; Groovy `find`/`view` navigation reimplemented on the JSON projection; one store impl on `object_store` (`Path::parse`, ignore S3 directory markers); `check` hashes `utf16le(path) ‖ size ‖ mtime_ms` (S3: `/<bucket>/<key>`, mtime from a parent listing — verified against Nextflow 26.10.7 on MiniStack). PRs: **A** types+parser+FindQuery, **B** store+check+diff, **C** view+provenance+contract+MiniStack+CI+release metadata.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/nf-lineage/` | Modified | real crate |
| `Cargo.lock` | Modified | deps |
| `deny.toml` | Modified | advisories/licenses |
| `.github/workflows/ci.yml` | Modified | 2 jobs |
| `CONTRIBUTING.md` | Modified | MiniStack note |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| `lineage-fixtures` late/different | Med | hard ordering; layout pinned in spec |
| PR > 800 lines | Med | escape hatches in tasks |
| aws-lc/licenses break `lint` | Med | crate-scoped exceptions |
| New checksum modes (#8018) | Med | typed `UnsupportedChecksum` |
| Find edge fidelity | Low | documented; table tests |

## Rollback Plan

Revert the PR(s) in reverse order; restore stub `lib.rs`/`Cargo.toml`, `deny.toml` and `ci.yml`. Read-only library, no data migrations; nothing depends on it yet.

## Dependencies

- `lineage-fixtures` merged first. Crates: object_store 0.14, async-trait, serde/serde_json, thiserror, futures, url, murmur3, similar; dev: insta, tokio, tempfile. MiniStack 1.5.45.

## Success Criteria

- [ ] All fixtures parse; snapshots stable.
- [ ] Contract tests match the CLI oracle on every version.
- [ ] MiniStack test green in `integration`; skips without MiniStack.
- [ ] Semantic diff of the two minimal runs = `params.greeting` only.
- [ ] fmt, clippy, tests, deny, machete, doctests green; each PR ≤ ~800 lines.
