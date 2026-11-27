#!/usr/bin/env bash
# Local demo: Docker + bash only, no API keys. Runs demo/main.nf twice (greeting hello, then
# bonjour) with lineage on; its AI step is answered by nf-llm-replay from demo/cassettes/demo.jsonl
# on an internal Docker network (no model provider reachable). Then demo/tour.py asks
# nf-lineage-mcp what changed, where a file came from, what the AI step did and whether a file is
# intact, and the script prints how to ask Claude Code the same. Research use only; synthetic data.
#
# Env: DEMO_WORKDIR (default demo/.run; recreated each run), MCP_IMAGE / REPLAY_IMAGE (use these
# images instead of building nf-lineage-mcp:demo / nf-llm-replay:demo), DEMO_RECORD=1 with
# UPSTREAM=<base URL> and NF_LLM_REPLAY_API_KEY (re-record the cassette; see docs/demo.md).
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/.." && pwd)
nxf_image=nextflow/nextflow:26.10.7
owner="$(id -u):$(id -g)"
marker=.nf-agents-demo
die() {
  echo "demo: $*" >&2
  exit 1
}
step() { echo "==> $*"; }

work=${DEMO_WORKDIR:-$here/.run}
if [ -d "$work" ] && [ -n "$(ls -A "$work")" ] && [ ! -e "$work/$marker" ]; then
  die "refusing to delete $work: it is not empty and has no $marker file (set DEMO_WORKDIR)"
fi
record=${DEMO_RECORD:-}
if [ "$record" = 1 ] && [ -z "${UPSTREAM:-}" ]; then
  die "DEMO_RECORD=1 needs UPSTREAM=<provider base URL>, e.g. http://127.0.0.1:18181"
fi
docker info >/dev/null 2>&1 || die "docker is required (install it and start the daemon)"

rm -rf "$work"
mkdir -p "$work"
touch "$work/$marker"
work=$(cd "$work" && pwd)

net=nf-agents-demo-$$
replay=nf-agents-demo-replay-$$
fifos=$(mktemp -d)
cleanup() {
  docker rm -f "$replay" >/dev/null 2>&1 || true
  docker network rm "$net" >/dev/null 2>&1 || true
  rm -rf "$fifos"
}
trap cleanup EXIT

build() {
  step "building $1 from source (first time: a few minutes)"
  docker build -q -f "$root/crates/$2/Dockerfile" -t "$1" "$root" >/dev/null
}
if [ -z "${MCP_IMAGE:-}" ]; then
  MCP_IMAGE=nf-lineage-mcp:demo
  build "$MCP_IMAGE" nf-lineage-mcp
fi
if [ -z "${REPLAY_IMAGE:-}" ]; then
  REPLAY_IMAGE=nf-llm-replay:demo
  build "$REPLAY_IMAGE" nf-llm-replay
fi

# Nextflow runs as root in its image: hand every file back to the invoking user, on failure too.
nxf() {
  docker run --rm -v "$work:/demo" -v "$here:/src:ro" -w /demo -e NXF_PLUGINS_DIR=/demo/plugins \
    -e NXF_ANSI_LOG=false "$@"
}
own="trap 'chown -R $owner /demo' EXIT;"
step "fetching the nf-agent plugin (the only networked step)"
nxf "$nxf_image" sh -c "$own nextflow plugin install nf-agent@0.2.0 >/dev/null"

if [ "$record" = 1 ]; then
  # The proxy needs the upstream: host network, key passed by name only, cassette written fresh.
  step "starting nf-llm-replay in record mode against $UPSTREAM"
  docker run -d --name "$replay" --network host --user "$owner" -v "$work:/rec" \
    -e NF_LLM_REPLAY_API_KEY "$REPLAY_IMAGE" --mode record --cassette /rec/cassette.jsonl \
    --upstream "$UPSTREAM" --port 18080 >/dev/null
  netargs=(--network host -e REPLAY_BASE_URL=http://127.0.0.1:18080/v1)
else
  step "starting nf-llm-replay in replay mode (internal network, no internet)"
  docker network create --internal "$net" >/dev/null
  docker run -d --name "$replay" --network "$net" --network-alias replay \
    -v "$here/cassettes:/cassettes:ro" "$REPLAY_IMAGE" --mode replay \
    --cassette /cassettes/demo.jsonl >/dev/null
  netargs=(--network "$net" -e NXF_OFFLINE=true)
fi
for _ in $(seq 50); do
  docker logs "$replay" 2>&1 | grep -q 'listening on' && break
  if [ "$(docker inspect -f '{{.State.Running}}' "$replay")" != true ]; then
    docker logs "$replay" >&2
    die "nf-llm-replay did not start (log above)"
  fi
  sleep 0.2
done

step "running demo/main.nf twice: --greeting hello, then --greeting bonjour"
status=0
nxf "${netargs[@]}" "$nxf_image" sh -c "$own
  nextflow run /src/main.nf --greeting hello -output-dir results-hello &&
  nextflow run /src/main.nf --greeting bonjour -output-dir results-bonjour" || status=$?
if docker logs "$replay" 2>&1 | grep 'outcome="miss"' >&2; then
  die "the cassette has no answer for a request (HTTP 599 above): the pipeline or the agent plugin
  changed. Re-record it: NF_LLM_REPLAY_API_KEY=... DEMO_RECORD=1 UPSTREAM=<url> demo/run.sh"
fi
[ "$status" = 0 ] || die "the pipeline failed (output above)"
answers="the AI answers were replayed from a recorded cassette (demo/cassettes/demo.jsonl),
no model was called"
if [ "$record" = 1 ]; then
  answers="the AI answers were recorded from $UPSTREAM"
  cp "$work/cassette.jsonl" "$here/cassettes/demo.jsonl"
  step "recorded $(wc -l <"$here/cassettes/demo.jsonl" | tr -d ' ') entries into demo/cassettes/demo.jsonl"
fi

step "asking nf-lineage-mcp about the two runs (scripted MCP session, transcript in $work/transcript.jsonl)"
mkfifo "$fifos/in" "$fifos/out"
# Both sides open `out` before `in` (redirections open left to right), so neither blocks forever.
docker run -i --rm --network none -v "$work:/demo:ro" "$MCP_IMAGE" --store /demo/.lineage \
  >"$fifos/out" <"$fifos/in" 2>"$fifos/mcp.log" &
mcp=$!
status=0
docker run -i --rm --network none --user "$owner" -v "$work:/demo" -v "$here:/src:ro" \
  "$nxf_image" python3 -I /src/tour.py --workdir /demo <"$fifos/out" >"$fifos/in" || status=$?
wait "$mcp" || true
if [ "$status" != 0 ]; then
  cat "$fifos/mcp.log" >&2
  die "the MCP tour failed (server log above)"
fi

cat <<EOF

Demo finished in ${SECONDS}s. Research use only, no clinical claims: synthetic data, and $answers.

Ask Claude Code the same questions (the server is read-only and has no network):
  claude mcp add --transport stdio nf-lineage -- docker run -i --rm --network none -v "$work:/demo:ro" $MCP_IMAGE --store /demo/.lineage
or with the native binary (cargo install --path "$root/crates/nf-lineage-mcp"):
  claude mcp add --transport stdio nf-lineage -- nf-lineage-mcp --store "$work/.lineage" --path-map /demo="$work"

Then ask, for example:
  What changed between my last two Nextflow runs?
  Where did summary.txt of the latest run come from?
  What did the AI step answer, and which model was it?
  Is results-hello/summary.txt still intact?
Remove it with: claude mcp remove nf-lineage
EOF
