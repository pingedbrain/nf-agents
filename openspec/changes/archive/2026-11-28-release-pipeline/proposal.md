# Proposal: release-pipeline

## Intent

Last v0.1 item: a `v*` tag publishes crates (`0.1.0`) and GHCR images.

## Scope

### In Scope
- `nf-llm-replay` publishable: drop `publish = false`; add `readme`, `keywords`, `categories`, `include = ["/src", "/README.md"]`; short crate `README.md`.
- `scripts/release/check.sh [tag]`: every crate publishable with `readme` and `keywords`; with a tag, `v<version>` equals every crate version.
- CI `test` job: `check.sh` + `cargo package --workspace --locked`.
- `.github/workflows/release.yml` on `v*` tags: `check` → `crates` → `images` → `release`.
- ROADMAP: release item checked.

### Out of Scope
- arm64 images (needs cross-compile or ARM runners).
- CHANGELOG; README switch to GHCR images (after the tag).

## Capabilities

### New Capabilities
- `release-pipeline`: crate metadata, tag check, publishing, images, Release.

### Modified Capabilities
- `ci-pipeline`: `test` also runs `check.sh` and `cargo package --workspace --locked`. `ci.yml` still never publishes.

## Approach

| Decision | Choice | Reason |
|---|---|---|
| Packaging | `cargo package --workspace` | cargo 1.100 resolves the unpublished `nf-lineage`; verified locally |
| Publish | `cargo publish --workspace` | dependency order and index wait built in |
| Auth | Trusted Publishing (`rust-lang/crates-io-auth-action`), environment `release` | no long-lived token |
| First publish | token fallback if `secrets.CARGO_REGISTRY_TOKEN` set | crates.io has no pending publishers |
| Images | `ghcr.io/pingedbrain/{nf-lineage-mcp,nf-llm-replay}`, plain `docker build/push` | matches CI `docker` |
| Tags | `X.Y.Z`; `latest` only without `-` | prereleases stay off `latest` |
| Provenance | `actions/attest-build-provenance` | one step |
| Safety | tag commit on `main`; per-job permissions | tags not ruleset-gated |
| Notes | `gh release create --generate-notes`, `--prerelease` on `-` | no CHANGELOG |

Today: names free on crates.io; packages are 41/31/54 KiB (`nf-llm-replay` also ships tests, fixed by `include`).

## Maintainer-only steps
1. Create environment `release` restricted to `v*` tags.
2. crates.io: token scoped `publish-new` + `publish-update` to the three crates, short expiry, saved as `release` secret `CARGO_REGISTRY_TOKEN`.
3. Tag ruleset for `v*` (creation limited to admins).
4. Push `v0.1.0` from `main`.
5. After 0.1.0: add trusted publishers (repo, `release.yml`, env `release`), delete secret, revoke token.
6. Make both GHCR packages public if private.

## Affected Areas

| Area | Impact |
|---|---|
| `crates/nf-llm-replay/{Cargo.toml,README.md}` | Modified/New |
| `scripts/release/check.sh` | New |
| `.github/workflows/{ci,release}.yml` | Modified/New |
| `ROADMAP.md` | Modified |

## Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| Partial publish | Low | publish is permanent; fix forward with `0.1.1` |
| Bootstrap token leaks | Low | scoped, short-lived, env-only, revoked |
| Images public before crates | Low | `images` needs `crates` |

## Rollback Plan

Revert the PR. A bad release: `cargo yank`, delete GHCR tag and GitHub Release, ship a patch. Versions cannot be republished.

## Success Criteria

- [ ] `check.sh` fails on today's `main`, passes after.
- [ ] `cargo package --workspace --locked` green in CI.
- [ ] `v0.1.0` publishes three crates, two images with attestations, and a Release.
