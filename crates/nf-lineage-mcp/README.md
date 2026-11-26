# nf-lineage-mcp

A read-only [MCP](https://modelcontextprotocol.io) server that lets AI agents (Claude Code and
other MCP clients) query a [Nextflow](https://www.nextflow.io) data lineage store: which runs
exist, what a run or task recorded, what changed between two runs, whether an output file still
matches its checksum, and where a file came from.

> **Status: pre-alpha.** Tool names and result shapes may change before 1.0.

It speaks MCP over stdio and is built on the [`nf-lineage`](https://crates.io/crates/nf-lineage) library, so it reads
local, `s3://`, `gs://` and `az://` stores without a JVM.

## Install

```sh
cargo install --path crates/nf-lineage-mcp           # from a checkout
docker build -f crates/nf-lineage-mcp/Dockerfile -t nf-lineage-mcp:local .   # or as an image
```

The image is static, runs as a non-root user (65532) and has no shell.

## Use with Claude Code

Options go before the server name; everything after `--` is the command that starts the server.

```sh
# A local store
claude mcp add --transport stdio nf-lineage -- nf-lineage-mcp --store /path/to/.lineage

# An S3 store: credentials come from the standard environment. `--env` takes several values,
# so put another option (here `--transport`) between it and the server name.
claude mcp add --env NF_LINEAGE_STORE=s3://my-bucket/.lineage --env AWS_PROFILE=research \
  --transport stdio nf-lineage -- nf-lineage-mcp

# The Docker image: `-i` keeps stdin open; mount the store read-only (absolute path)
claude mcp add --transport stdio nf-lineage -- \
  docker run -i --rm -v /abs/path/.lineage:/store:ro nf-lineage-mcp:local --store /store
```

The same entry as JSON:

```sh
claude mcp add-json nf-lineage \
  '{"type":"stdio","command":"nf-lineage-mcp","args":["--store","/path/to/.lineage"]}'
```

Then ask, for example, "what changed between my last two runs?" or "is results/summary.txt still
the file the pipeline wrote?".

## Configuration

| Flag | Environment | Default | Meaning |
|------|-------------|---------|---------|
| `--store` | `NF_LINEAGE_STORE` | required | Local path, `file://`, `s3://`, `gs://` or `az://` URL |
| `--max-response-bytes` | `NF_LINEAGE_MCP_MAX_RESPONSE_BYTES` | 32768 | Upper bound of one tool result (≥ 1024) |
| `--path-map FROM=TO` | — | none | For `check`: read files recorded under `FROM` below the local directory `TO` (repeatable) |
| — | `NF_LINEAGE_MCP_LOG` | `info` | Log filter (`tracing` syntax); logs go to stderr, never stdout |

Cloud credentials use the usual variables (`AWS_*`, `GOOGLE_*`, `AZURE_*`). The server opens the
store at startup and exits with status 1 and a message on stderr when it cannot.

`--path-map` helps when the files were written somewhere else, e.g. a run on a cluster checked
on a laptop: `--path-map /scratch/run1=/data/run1`. Only the operator sets it; tool calls cannot.

## Tools

All tools are read-only (`readOnlyHint: true`, `destructiveHint: false`): they never write,
delete or create anything in the store or on disk.

| Tool | Use it to |
|------|-----------|
| `list_runs` | List runs, newest first, with their lineage IDs (start here); filter by date or pipeline |
| `view` | Show one record (`lid://<hash>`) or one field of it (`lid://<hash>#params`) |
| `find` | Find records by type, labels, name or `key=value` terms |
| `diff` | Show what changed between two runs or tasks (semantic JSON, or `mode=raw` text diff) |
| `check` | Verify a data file against its recorded checksum |
| `provenance` | Trace the tasks and inputs a file was produced from |

Failures are tool results with `isError: true` and a message of the form
`<class>: <reason> Hint: <what to try>`, so the agent can correct itself. Classes:
`invalid_argument`, `invalid_lid`, `invalid_query`, `not_found`, `unexpected_kind`,
`invalid_record`, `unsupported`, `storage`, `error`.

## Large results

A result longer than `--max-response-bytes` is cut and ends with a `[truncated] … Narrow the
request: …` note. Claude Code applies its own limit to MCP output (`MAX_MCP_OUTPUT_TOKENS`,
in tokens); keep the byte cap well below it (the default 32 KiB is roughly 8 000 tokens).

## Security

- Read-only by design: no tool modifies the store, and none accepts a path to read outside it
  (`check` only reads the file a record points to, mapped by the operator's `--path-map`).
- stdio only: the server opens no network port.
- In Docker, mount the store `:ro`.

Lineage records can contain sample names, paths and parameters. Only connect stores you are
allowed to share with the agent and its model provider. This tool is for research use; it is
not a medical device.

## License

Apache-2.0
