## What

<!-- One task per PR. What does this change do? Link the issue: Closes #... -->

## Why

## How it was tested

<!-- Which tests cover each acceptance criterion? -->

## Checklist

- [ ] The PR is small and does one thing.
- [ ] Tests are included in this PR, and every acceptance criterion maps to a test.
- [ ] Rust: `cargo fmt --all`, `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo test --workspace --locked` pass.
- [ ] OpenTofu: `tofu fmt -recursive`, `tofu validate` and `tofu test` pass.
- [ ] No secrets, `.env` files, or real patient/genomic data are committed.
- [ ] PR CI does not require secrets or API keys.
- [ ] Tools and agents stay read-only by default.
- [ ] No clinical claims in code, docs or outputs.

## AI usage

- [ ] **No AI assistance** was used in this PR.
- [ ] **AI assistance was used.** Tool(s) and what for: <!-- e.g. "Claude Code: drafted the parser and tests; reviewed and edited by me" -->
