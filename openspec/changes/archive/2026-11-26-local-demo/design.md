# Design: local-demo

## Technical Approach

`demo/run.sh` (bash 3.2-compatible, Docker only) orchestrates four short-lived containers:
1. **replay**: `nf-llm-replay`, replay mode, cassette mounted read-only.
2. **nextflow**: `nextflow/nextflow:26.10.7`, two runs, as root, `chown` back in an in-container `trap`.
3. **mcp**: `nf-lineage-mcp`, `--network none`, workdir read-only.
4. **tour**: `demo/tour.py`, run in the Nextflow image as the invoking user.

The tour and mcp containers are wired stdin↔stdout through two host FIFOs. Specs: `specs/local-demo/spec.md`, `specs/ci-pipeline/spec.md`. Evidence comes from `exploration.md` (verified locally: pipeline runs, FIFO wiring between two containers in 3.7 s, cold build timing).

## Architecture Decisions

| # | Topic | Options | Decision / rationale |
|---|---|---|---|
| D1 | Entry point | Makefile / just / compose / **bash script** | One file, no extra tool. Compose cannot wire an interactive stdio client. A Makefile would be a one-line wrapper (YAGNI). |
| D2 | MCP client runtime | host python / binary copied into the Nextflow image / docker-socket mount / **tour container ↔ mcp container via host FIFOs** | Copying the binary breaks on Apple Silicon (the Nextflow image is amd64-only). A socket mount is root-equivalent and its path varies. FIFOs keep each container on its native arch and run the *same* `docker run -i …` users paste into Claude Code. Verified locally. |
| D3 | FIFO open order | any / **server `>out <in`, client `<out >in`** | Redirections open left to right; this order cannot deadlock (comment in the script). |
| D4 | Client language | bash + grep / Rust example / **python 3.9 stdlib** | Python 3.9 is already in the Nextflow image. Same precedent as `lineage-fixtures` D4. Unit-testable with host python in CI. |
| D5 | Agent output | free text / **structured `QcVerdict`** | The fake upstream's fixed `{"answer":"no","confidence":0.9}` is the correct answer to "any lowercase letters?". A real model answers the same, so the cassette is honest either way. Also exercises `json_schema` and `AgentRun.outputSchema`. |
| D6 | `enable.types` | on / **off** | On rejects the legacy `params.*` of the minimal pipeline (verified locally). Agents and `record` work without it. |
| D7 | Pipeline source | `include` the fixture / **copy + agent** | The fixture has an entry workflow + `output` block; including it is fragile. The fixture must stay byte-stable for shape checks. A ~50-line copy is cheaper. |
| D8 | Output dirs | shared / **`results-<greeting>`** | A shared dir would overwrite run A, and `check` on A would mismatch for the wrong reason. |
| D9 | Store paths | `--path-map` in the tour / **same mount `/demo` everywhere** | Recorded paths resolve as-is in the mcp container. Only the native Claude Code variant needs `--path-map /demo=<workdir>`. |
| D10 | Images | always build / pull / **build unless `MCP_IMAGE`/`REPLAY_IMAGE` set** | Works today. CI reuses the `:ci` images. GHCR is just an override after the release change (no version logic). |
| D11 | Network | `--internal` + plugin prefetch / **private bridge** | The `nf-agent` plugin download needs egress. Replay mode has no HTTP client (nf-llm-replay design), and mcp runs `--network none`. |
| D12 | Replay readiness | sleep / **poll `docker logs` for `listening on` (≤ 10 s), fail if the container exits** | Deterministic, and surfaces cassette load errors. |
| D13 | Workdir | `/tmp` / **`demo/.run` + `.nf-agents-demo` marker** | `/tmp` is not shared by default on some Docker Desktop setups and gets wiped. The marker prevents `rm -rf` of a user directory. |
| D14 | `check` demo | Valid only / **Valid, then Mismatch on run A after a labelled append** | Shows why `check` matters. The tour only touches its own synthetic output under the workdir, and the tools stay read-only. |
| D15 | CI placement | nightly / new job / **last step of `docker`** | The images are already built and the Nextflow image already pulled by the replay e2e, so it adds ≈ 40–60 s. PR-blocking keeps the README honest. |
| D16 | PR split | one / **two** | ≈ 790 lines total. Docs need timings measured on the merged demo. |

### Deviations (2026-11-26, from the implementation)

- **D11 → internal network.** Replay mode uses `docker network create --internal` plus a plugin prefetch into `<workdir>/plugins` and `NXF_OFFLINE=true` (the `nf-llm-replay` e2e precedent): no model provider is reachable, so answers can only come from the cassette. `demo/nextflow.config` pins `plugins { id 'nf-agent@0.2.0' }` for the offline run.
- **Record mode → host network.** `examples/fake_upstream` binds `127.0.0.1` only, so `host.docker.internal` cannot reach it. Record mode runs the proxy and Nextflow with `--network host` (proxy on `127.0.0.1:18080`, `UPSTREAM=http://127.0.0.1:<port>`), records into a fresh `<workdir>/cassette.jsonl` (record mode only fills missing keys) and copies it to `demo/cassettes/demo.jsonl` on success. Verified: two recordings against the fake upstream are byte-identical.
- **Miss detection** greps the replay log for `outcome="miss"` (the actual log field); `nf_llm_replay_miss` is the 599 body type, which Nextflow prints itself.
- **D14:** `check` Valid then Mismatch both on Run A's `summary.txt`; the first `check` answer gives the path to edit, and the tour refuses paths outside the workdir. Run B's `summary.txt` is used for provenance.
- **D16:** PR B (README + `docs/demo.md`) runs as a separate apply batch to keep each PR within the line budget.
- The CI step uses `shell: bash` (pipefail) and `tee`, so the demo output stays visible when it fails.

## Data Flow

```
run.sh ─ guard workdir ─ docker? ─ build|use images ─ network nf-agents-demo-$$
  ├─ nextflow: plugin install nf-agent@0.2.0 → /demo/plugins (default network)
  ├─ replay  (-d, alias replay, /cassettes:ro, --internal network)  ◀── HTTP ──┐
  ├─ nextflow (/src:ro=demo/, /demo=workdir, NXF_OFFLINE) run hello, run bonjour ┘ → /demo/.lineage, results-*/
  ├─ mkfifo T/in T/out
  │   mcp  docker run -i --network none -v W:/demo:ro MCP --store /demo/.lineage  >T/out <T/in &
  │   tour docker run -i --network none --user U -v W:/demo -v demo:/src:ro NXF python3 -I /src/tour.py <T/out >T/in
  │        (human output → stderr; transcript → /demo/transcript.jsonl)
  └─ print claude mcp add (docker + native) ; trap: rm containers, network, T
```

## File Changes

| File | Action | PR | Est. |
|---|---|---|---|
| `demo/run.sh` | Create | A | 125 |
| `demo/main.nf`, `demo/nextflow.config`, `demo/data/seed.txt` | Create | A | 100 |
| `demo/cassettes/demo.jsonl` | Generated (2 lines) | A | — |
| `demo/tour.py` | Create | A | 170 |
| `demo/test_tour.py` | Create | A | 150 |
| `crates/nf-llm-replay/tests/secrets.rs` | Modify: S-cas → `committed_cassettes_contain_no_credentials` over both cassette dirs | A | 10 |
| `.github/workflows/ci.yml` | Modify: `docker` step `Local demo`; `test` steps unittest + shellcheck | A | 14 |
| `.gitignore` | `demo/.run/` | A | 1 |
| `README.md` | "Try it in 5 minutes" | B | 50 |
| `docs/demo.md` | Create | B | 170 |

## Interfaces / Contracts

`run.sh` env: `DEMO_WORKDIR`, `MCP_IMAGE`, `REPLAY_IMAGE`, `DEMO_RECORD=1` + `UPSTREAM` (+ `NF_LLM_REPLAY_API_KEY`, passed with `-e NAME` only; required by record mode, any value for the fake upstream). Exit codes: 0 ok, 1 usage/guard/docker/pipeline/tour failure.

```python
# demo/tour.py — python 3.9, stdlib only
class TourError(Exception): ...                      # message names the step; main() → exit 1, no traceback
class Session:
    def __init__(self, rx: TextIO, tx: TextIO, transcript: Optional[TextIO] = None): ...
    def request(self, method: str, params: dict) -> dict  # skips lines without a matching id; EOF → TourError("server closed")
    def notify(self, method: str) -> None
    def tool(self, name: str, args: dict) -> Any          # isError → TourError(f"{name}: {text}"); json.loads(text) or text
def run_tour(s: Session, workdir: str, say: Callable[[str], None]) -> None   # steps + assertions (spec "MCP tour")
def main(argv=None, rx=sys.stdin, tx=sys.stdout, err=sys.stderr) -> int     # --workdir /demo
```

Tool argument shapes are taken from the `lineage-mcp` spec, e.g. `find {terms: ["type=AgentRun", "workflowRun=<B>"]}`, `view {lid: "<agent>#output"}`, `provenance {lid: "<B>/summary.txt"}`. Run order: `list_runs` is newest first, so B = `rows[0]`. The two runs are ≥ 4 s apart, so their timestamps are distinct.

Printed hand-off (absolute `W`):
```
claude mcp add --transport stdio nf-lineage -- docker run -i --rm --network none -v "W:/demo:ro" MCP --store /demo/.lineage
cargo install --path crates/nf-lineage-mcp   # or crates.io after 0.1.0
claude mcp add --transport stdio nf-lineage -- nf-lineage-mcp --store "W/.lineage" --path-map /demo="W"
```

## Testing Strategy

| Layer | What | Approach |
|---|---|---|
| Unit | tour steps, assertions, error paths, EOF | `TourTests`: an in-memory `FakeServer` (`write` parses the request and queues a canned response with the same `id`, `readline` pops it); a temp workdir for the append |
| Script | workdir guard, no Docker | `RunScriptTests`: `subprocess` with a stub `docker` (exit 1) first on `PATH` (the real one lives in `/usr/bin` on runners) and a temp `DEMO_WORKDIR` |
| Lint | bash pitfalls | `shellcheck demo/run.sh` |
| Secrets | demo cassette | extended `secrets.rs` (cargo test) |
| E2E | whole demo | CI `docker` step with the `:ci` images |
| Manual | DM-miss, DM-record, DM-time (warm on a dev box and the CI step; cold: `docker builder prune -af` + `rmi`) | numbers go into `docs/demo.md` |

## Migration / Rollout

No migration. After the release change publishes images, the README fast path becomes `MCP_IMAGE=ghcr.io/pingedbrain/nf-lineage-mcp:0.1.0 REPLAY_IMAGE=ghcr.io/pingedbrain/nf-llm-replay:0.1.0 demo/run.sh`.

## Open Questions

- [ ] Record `demo.jsonl` from real `gpt-5-mini` once a provider key is available. Until then, use the fake upstream. Non-blocking: the answers are identical.
- [ ] Apple Silicon timing (emulated Nextflow image) is not measured in CI. Ask a contributor to report it.
