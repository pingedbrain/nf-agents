#!/usr/bin/env bash
# Offline end-to-end run: Nextflow 26.10.7 + the baqueano-replay image in replay mode on an
# --internal Docker network (no model provider reachable). The plugin prefetch is the only
# networked step. Env: IMAGE (default: build baqueano-replay:e2e), E2E_REQUIRE_DOCKER=1 (fail
# instead of skipping when Docker is unavailable).
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../../.." && pwd)
nxf_image=nextflow/nextflow:26.10.7

if ! command -v docker >/dev/null 2>&1 || ! docker info >/dev/null 2>&1; then
  if [ "${E2E_REQUIRE_DOCKER:-}" = 1 ]; then
    echo "FAIL: docker not available" >&2
    exit 1
  fi
  echo "SKIP: docker not available"
  exit 0
fi

tmp=$(mktemp -d)
net=nfllm-e2e-$$
replay=nfllm-replay-$$
cleanup() {
  docker rm -f "$replay" >/dev/null 2>&1 || true
  docker network rm "$net" >/dev/null 2>&1 || true
  # Nextflow runs as root in its container: hand the files back before removing them.
  docker run --rm -v "$tmp:/t" "$nxf_image" chown -R "$(id -u):$(id -g)" /t >/dev/null 2>&1 || true
  rm -rf "$tmp"
}
trap cleanup EXIT
fail() {
  echo "FAIL: $1" >&2
  echo "--- nextflow" >&2 && cat "$tmp/nextflow.log" >&2
  echo "--- baqueano-replay" >&2 && docker logs "$replay" >&2 2>&1
  exit 1
}

if [ -z "${IMAGE:-}" ]; then
  IMAGE=baqueano-replay:e2e
  docker build -q -f "$root/crates/baqueano-replay/Dockerfile" -t "$IMAGE" "$root" >/dev/null
fi

mkdir -p "$tmp/plugins" "$tmp/run"
cp "$here/main.nf" "$here/nextflow.config" "$tmp/run/"
docker run --rm -v "$tmp/plugins:/plugins" -e NXF_PLUGINS_DIR=/plugins "$nxf_image" \
  nextflow plugin install nf-agent@0.2.0 >/dev/null

docker network create --internal "$net" >/dev/null
docker run -d --name "$replay" --network "$net" --network-alias replay \
  -v "$here/cassettes:/cassettes:ro" "$IMAGE" \
  --mode replay --cassette /cassettes/example.jsonl >/dev/null
for _ in $(seq 50); do
  docker logs "$replay" 2>/dev/null | grep -q 'listening on' && break
  sleep 0.2
done

docker run --rm --network "$net" -v "$tmp:$tmp" -v "$tmp/plugins:/plugins:ro" -w "$tmp/run" \
  -e NXF_OFFLINE=true -e NXF_PLUGINS_DIR=/plugins -e NXF_ANSI_LOG=false \
  -e REPLAY_BASE_URL=http://replay:8080/v1 "$nxf_image" nextflow run main.nf \
  >"$tmp/nextflow.log" 2>&1 || fail "nextflow exited $?"
for want in 'FASTQ is a text format.' '{"result":"HELLO"}' '[answer:no, confidence:0.9]'; do
  grep -qF "$want" "$tmp/nextflow.log" || fail "missing output: $want"
done
# Each request is logged with outcome="hit" or outcome="miss".
if docker logs "$replay" 2>&1 | grep -q 'outcome="miss"'; then fail "cassette misses"; fi
echo "PASS: offline pipeline run answered from cassettes/example.jsonl"
