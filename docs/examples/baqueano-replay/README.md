# Offline Nextflow agents with baqueano-replay

A Nextflow pipeline with three `agent`s (plain text, structured output, a process tool) that runs
with **no model provider and no API key**: every LLM call is answered by `baqueano-replay` from the
committed cassette `cassettes/example.jsonl`.

The cassette was recorded against a deterministic fake provider, so its answers are synthetic
research material, not model output. `baqueano-replay` is pre-alpha, for research and testing only.

| File | What |
|---|---|
| `main.nf` | `qa` (text), `qa_structured` (record output, strict `json_schema`), `shouty` (calls the `uppercase` process as a tool) |
| `nextflow.config` | only `agent.baseUrl`, from `REPLAY_BASE_URL` (default `http://localhost:8080/v1`); no `apiKey` |
| `cassettes/example.jsonl` | one JSON line per recorded request/response |
| `e2e.sh` | the offline check CI runs (`E2E_REQUIRE_DOCKER=1`, `IMAGE=...`) |
| `record.sh` | regenerates the cassette |

## Replay with Docker

```bash
docker build -f crates/baqueano-replay/Dockerfile -t baqueano-replay .   # from the repo root
docker run --rm -p 8080:8080 -v "$PWD/docs/examples/baqueano-replay/cassettes:/cassettes:ro" \
  baqueano-replay --mode replay --cassette /cassettes/example.jsonl
# in another shell, from docs/examples/baqueano-replay:
nextflow run main.nf          # agent.baseUrl -> http://localhost:8080/v1
```

A request that is not in the cassette gets HTTP 599; Nextflow prints its body, which names the
request and the exact `--mode record` command to add it. `./e2e.sh` does all of the above on an
`--internal` Docker network (only the `nf-agent@0.2.0` plugin download needs the network).

Do **not** set `OPENAI_API_KEY` (or put an `apiKey` in the config) for Nextflow when `agent.baseUrl`
is custom: the proxy never forwards client credentials, and keeping the real key out of the
Nextflow environment keeps it out of task logs.

## Record

`./record.sh` runs the same pipeline against `baqueano-replay --mode record` and rewrites
`cassettes/example.jsonl`. With no `UPSTREAM` it starts the fake provider
(`cargo run -p baqueano-replay --example fake_upstream -- --port 18181`), so no key is needed and the
output is byte-identical every time. Against a real provider:

```bash
BAQUEANO_REPLAY_API_KEY=sk-... UPSTREAM=https://api.openai.com ./record.sh
```

The key is read only from `BAQUEANO_REPLAY_API_KEY`, injected upstream, and never written to the
cassette or printed. Record and replay must use the same Nextflow version and the same flags
(`-with-agent-trace`, for example, changes the system prompt and therefore every key).

Recording with the image instead of the host binary: mount the cassette's **directory** read-write
(the cassette is replaced by an atomic rename, which fails on a single bind-mounted file) and run
as your user so the file stays yours:

```bash
docker run --rm -p 127.0.0.1:8080:8080 --user "$(id -u):$(id -g)" -e BAQUEANO_REPLAY_API_KEY \
  -v "$PWD/cassettes:/cassettes" baqueano-replay \
  --mode record --cassette /cassettes/example.jsonl --upstream https://api.openai.com
```

Record and passthrough modes hold the real API key and have no auth, so never expose them beyond localhost.

## The `pi` runner

With `agent.runner = 'pi'` the agent runs in its own container, so:

- `agent.baseUrl` must use a host name that container can reach (not `localhost` unless it shares
  the host network, e.g. `docker.runOptions = '--network host'`);
- set a dummy `agent.apiKey = 'dummy-not-a-secret'`: pi needs a non-empty key, and the proxy
  strips it anyway.
