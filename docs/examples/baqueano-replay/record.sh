#!/usr/bin/env bash
# Re-records cassettes/example.jsonl: Nextflow (Docker, host network) -> baqueano-replay --mode record
# (host binary) -> UPSTREAM. Default UPSTREAM is the deterministic fake provider (no key needed):
#   ./record.sh
# Against a real provider (the key is read from the environment and never printed):
#   BAQUEANO_REPLAY_API_KEY=... UPSTREAM=https://api.openai.com ./record.sh
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../../.." && pwd)
nxf_image=nextflow/nextflow:26.10.7
proxy_port=${PROXY_PORT:-18080}
fake_port=${FAKE_PORT:-18181}

# Waits until the background process logging to $1 prints its `listening on` line.
wait_listening() {
  for _ in $(seq 100); do
    grep -q 'listening on' "$1" && return 0
    sleep 0.2
  done
  echo "FAIL: no 'listening on' in $1" >&2
  cat "$1" >&2
  return 1
}

tmp=$(mktemp -d)
pids=()
cleanup() {
  for pid in "${pids[@]}"; do kill "$pid" 2>/dev/null || true; done
  # Nextflow runs as root in its container: hand the files back before removing them.
  docker run --rm -v "$tmp:/t" "$nxf_image" chown -R "$(id -u):$(id -g)" /t >/dev/null 2>&1 || true
  rm -rf "$tmp"
}
trap cleanup EXIT

cargo build -q --manifest-path "$root/Cargo.toml" -p baqueano-replay --bin baqueano-replay --example fake_upstream
bin=${CARGO_TARGET_DIR:-$root/target}/debug

if [ -z "${UPSTREAM:-}" ]; then
  "$bin/examples/fake_upstream" --port "$fake_port" >"$tmp/fake.log" 2>&1 &
  pids+=($!)
  wait_listening "$tmp/fake.log"
  UPSTREAM=http://127.0.0.1:$fake_port
  # The fake ignores credentials; record mode still requires a key.
  export BAQUEANO_REPLAY_API_KEY=${BAQUEANO_REPLAY_API_KEY:-fake-upstream-key}
fi

mkdir -p "$tmp/run"
cp "$here/main.nf" "$here/nextflow.config" "$tmp/run/"
# Record into a fresh file (record mode only fills missing keys); replace the cassette on success.
"$bin/baqueano-replay" --mode record --cassette "$tmp/example.jsonl" --upstream "$UPSTREAM" \
  --port "$proxy_port" >"$tmp/proxy.log" 2>&1 &
pids+=($!)
wait_listening "$tmp/proxy.log"

# Same Nextflow version and flags as e2e.sh: other flags change the prompts and thus the keys.
if ! docker run --rm --network host -v "$tmp:$tmp" -w "$tmp/run" -e NXF_ANSI_LOG=false \
  -e REPLAY_BASE_URL="http://127.0.0.1:$proxy_port/v1" "$nxf_image" nextflow run main.nf; then
  echo "FAIL: nextflow run failed; proxy log:" >&2
  cat "$tmp/proxy.log" >&2
  exit 1
fi
cp "$tmp/example.jsonl" "$here/cassettes/example.jsonl"
echo "recorded $(wc -l <"$here/cassettes/example.jsonl") entries into cassettes/example.jsonl"
