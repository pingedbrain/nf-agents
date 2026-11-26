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
