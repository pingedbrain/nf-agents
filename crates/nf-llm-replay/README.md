# nf-llm-replay

A record/replay proxy for LLM HTTP APIs. Point an agent at it instead of the provider, and the
recorded answers replay offline, so agent tests run without network access or API keys.

> **Status: pre-alpha.** Research and testing use only. Options may change before 1.0.

Requests are normalized into a stable key (canonical JSON body, SHA-256) and answered from a
reviewable JSONL cassette: one recorded request/response pair per line.

## Install

```sh
cargo install nf-llm-replay
```

## Modes

| Mode | Behavior |
|------|----------|
| `replay` (default) | Answer only from the cassette; never touch the network. |
| `record` | Answer missing keys from the upstream and add them to the cassette. |
| `passthrough` | Forward everything upstream; no cassette. |

## Usage

```sh
nf-llm-replay --mode replay --cassette calls.jsonl --port 8080
nf-llm-replay --mode record --cassette calls.jsonl --upstream https://api.example.com
```

Every flag has an `NF_LLM_REPLAY_*` environment variable (for example `NF_LLM_REPLAY_PORT`).
The upstream API key is read from `NF_LLM_REPLAY_API_KEY` and is never printed.
Run `nf-llm-replay --help` for the full list.

## License

Apache-2.0. See the repository for details.
