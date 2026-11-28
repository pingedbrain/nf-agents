# Design: release-pipeline

## Technical Approach

A `v*` tag runs `release.yml`: `check` → `crates` → `images` (matrix of 2) → `release`. Every rule a tag must satisfy lives in one script, `scripts/release/check.sh`, which CI also runs on every PR so `main` is always releasable. No new dependencies; actions pinned by major tag like `ci.yml` (`actions/checkout@v7`).

## Architecture Decisions

| Topic | Choice | Rejected | Reason |
|---|---|---|---|
| Auth action | `rust-lang/crates-io-auth-action@v1` (latest v1.0.6) | SHA pin | matches `ci.yml` style |
| Provenance | `actions/attest-build-provenance@v4` (latest v4.2.2) | `actions/attest@v4` | named in proposal; v4 wraps `actions/attest` with the same inputs, swap is one line |
| Token fallback | step reads `secrets.CARGO_REGISTRY_TOKEN` into env, writes only `bootstrap=true` to `$GITHUB_OUTPUT` when non-empty; auth action runs `if: steps.token.outputs.bootstrap != 'true'`; publish uses `steps.auth.outputs.token \|\| secrets.CARGO_REGISTRY_TOKEN` | `if: secrets.X != ''` | `secrets` is not allowed in step `if`; token never echoed, only a boolean |
| Metadata read | `cargo metadata --format-version 1 --no-deps` + `jq` | grep on `Cargo.toml` | resolves `*.workspace = true`; `publish = false` is `[]`; `--no-deps` needs no lockfile; `jq` ships on ubuntu runners |
| Tag rule | tag must equal `v<version>` exactly | separate semver regex | versions are already semver, so equality implies `vX.Y.Z[-pre]` |
| On-main check | in `release.yml` (`git merge-base --is-ancestor "$GITHUB_SHA" origin/main`, `fetch-depth: 0`) | in `check.sh` | needs git history, not a manifest property; keeps the script offline and testable |
| Image digest | `docker buildx imagetools inspect "$IMAGE:$version" --format '{{json .Manifest}}' \| jq -r .digest` | `RepoDigests` | correct for both classic and containerd image stores |
| Images | plain `docker build/push`, `--label` for `org.opencontainers.image.version` and `.revision`; source/licenses/description already in Dockerfiles | `docker/build-push-action` | matches CI `docker` job |
| Shellcheck | `shellcheck demo/run.sh scripts/release/check.sh` | none | one word on an existing line |
| `include` | `["/src", "/README.md"]` for `nf-llm-replay` (other two already have it); no LICENSE file | ship `LICENSE` | `license` SPDX field suffices on crates.io; same as the two published-ready crates |

`cargo package --list -p nf-llm-replay` today ships `Dockerfile`, `examples/`, `tests/`; after: `src/*`, `README.md`, cargo-generated files.

## `scripts/release/check.sh [tag]`

Runs against the workspace in the current directory (CI and tests set the cwd).

- Exit 2: more than one argument.
- Exit 1, one stderr line per problem: any member with `publish = false`; empty `description`, `license`, `repository`, `readme`; empty `keywords`; more than one distinct version; tag given and not `v<version>`.
- Exit 0: prints `release check ok: <version> (<tag>)`.

A 20-line prototype was verified locally: today it exits 1 on `nf-llm-replay` (publish, readme, keywords).

## release.yml

```
on: push: tags: ['v*']        permissions: {}  (per job below)
concurrency: { group: release, cancel-in-progress: false }   # never cancel mid-publish

check   contents:read           checkout fetch-depth 0 → is-ancestor origin/main
                                → rustup toolchain install → check.sh "$GITHUB_REF_NAME"
crates  contents:read           environment: release; token step → auth action (if no token)
        id-token:write          → cargo publish --workspace --locked
images  contents:read           matrix crate [nf-lineage-mcp, nf-llm-replay]
        packages:write          version=${GITHUB_REF_NAME#v}; tags :version, +:latest if no '-'
        id-token:write          build → docker login ghcr.io (GITHUB_TOKEN via stdin)
        attestations:write      → push --all-tags → digest → attest (subject-name
        artifact-metadata:write   ghcr.io/pingedbrain/<crate>, subject-digest, push-to-registry: true)
release contents:write          gh release create "$GITHUB_REF_NAME" --repo "$GITHUB_REPOSITORY"
                                --verify-tag --generate-notes [--prerelease if '-']
```

Tag/ref values reach scripts only through env (`$GITHUB_REF_NAME`, `env: CRATE: ${{ matrix.crate }}`), never `${{ }}` inside `run`. All checkouts use `persist-credentials: false`; `release` needs no checkout.

## File Changes

| File | Action | ~Lines |
|---|---|---|
| `scripts/release/check.sh` | Create | 20 |
| `scripts/release/test_check.py` | Create | 60 |
| `.github/workflows/release.yml` | Create | 100 |
| `.github/workflows/ci.yml` | Modify (`test` job) | +4 |
| `crates/nf-llm-replay/Cargo.toml` | Modify | +5 / -1 |
| `crates/nf-llm-replay/README.md` | Create | 25 |
| `ROADMAP.md` | Modify | 1 |

CI `test` additions: `python3 -I -m unittest discover -s scripts/release -p 'test_*.py'`, `scripts/release/check.sh`, `cargo package --workspace --locked`, `scripts/release/check.sh` added to the shellcheck line.

## Testing Strategy (strict TDD)

`scripts/release/test_check.py` (unittest, same pattern as `demo/test_*.py`), written first:

| Case | Expect |
|---|---|
| repo root, no tag | exit 0 (red today, green after the manifest change) |
| temp 2-crate workspace, `1.2.3-rc.1`, tag `v1.2.3-rc.1` | 0 |
| same, tag `v1.2.3` / `1.2.3-rc.1` | 1 |
| versions differ | 1, stderr has `versions differ` |
| one crate `publish = false` / no `readme` | 1, stderr names the crate |
| two args | 2 |

Temp workspaces are tiny manifests plus empty `src/lib.rs`, built with `tempfile`. Workflow YAML is not unit-tested; `cargo package --workspace --locked` in CI covers packaging, the first tag covers publishing.

## Migration / Rollout

Maintainer steps from the proposal. Rerunning `crates` after a partial publish fails on the already-published crate: fix forward with a patch version.

## Open Questions

- [ ] None blocking. Optional: switch to `actions/attest@v4` per upstream note.
