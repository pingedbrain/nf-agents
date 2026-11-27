# Contributing to nf-agents

Thanks for your interest. nf-agents is pre-alpha, so expect things to move. This guide covers setup, what to run before a PR, and the project rules.

## Dev setup

1. Install [rustup](https://rustup.rs). The toolchain is pinned to an exact stable release in [`rust-toolchain.toml`](rust-toolchain.toml); rustup installs it (with `rustfmt` and `clippy`) the first time you run `cargo` in the repo. Your global default toolchain is not changed.
2. Install the supply-chain tools used by CI:

   ```sh
   cargo install --locked cargo-deny cargo-machete
   ```

Toolchain bumps are normal PRs that edit `rust-toolchain.toml`, so local and CI lints always match.

## Before opening a PR

Run the same commands CI runs (`.github/workflows/ci.yml`):

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo deny check
cargo machete
cargo test --workspace --locked
```

Use `cargo fmt --all` to fix formatting. The test suite must pass without API keys, cloud credentials or GPUs.

### S3 integration tests (MiniStack)

`nf-lineage` reads lineage stores from S3. Its S3 tests run against [MiniStack](https://github.com/ministackorg/ministack), a local AWS emulator, and need only Docker:

```sh
docker run --rm -d --name ministack -p 4566:4566 ministackorg/ministack:1.5.45
NF_LINEAGE_MINISTACK_URL=http://localhost:4566 cargo test -p nf-lineage --test s3_ministack
docker stop ministack
```

The tests create their own buckets with dummy `test`/`test` credentials. Without `NF_LINEAGE_MINISTACK_URL` they print a skip notice and pass, so `cargo test --workspace` works without Docker. With the variable set they wait up to 60 seconds for MiniStack and fail if it does not answer. CI runs them in the `integration` job.

### Docs site (`site/`)

The site is Astro + Starlight. The docs pages are generated from `docs/demo.md`, `docs/architecture.md`, `crates/nf-lineage-mcp/README.md`, `docs/examples/nf-llm-replay/README.md` and `ROADMAP.md`, so edit those sources, not the generated pages. Run the same checks as CI (`.github/workflows/site.yml`):

```sh
cd site
npm ci
npm run build
npm test
```

Use Node.js 26 (see `site/.nvmrc`). The site does not use analytics or external font or script hosts.

## Pull requests

- Keep PRs small: one task per PR.
- Tests go in the same PR as the code they cover.
- Declare AI usage in every PR (the PR template has an "AI usage" section). Saying "none" is fine.

## Project rules

- Never commit secrets, `.env` files, or real patient or genomic data. Use synthetic fixtures.
- Tools and agents are read-only by default: they never modify data or infrastructure unless a user explicitly opts in.
- Library code must not panic on external input: `unwrap`, `expect`, `panic!` and unchecked indexing are denied by clippy outside tests, and `unsafe` is forbidden.
- All public text (code, comments, commits, docs) is in English.

## Spec-driven flow

Substantial changes follow a spec-driven flow with artifacts in `openspec/`:

proposal → specs → design → tasks → apply → verify → archive

Each change lives in `openspec/changes/<change-name>/` and is moved to `openspec/changes/archive/` when done. Every acceptance criterion in a spec maps to a test.

## Conduct and security

- Read the [Code of Conduct](CODE_OF_CONDUCT.md).
- Report vulnerabilities privately as described in the [security policy](SECURITY.md), not in public issues.
