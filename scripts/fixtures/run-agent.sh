#!/usr/bin/env bash
# Runs inside nextflow/nextflow:<version>, cwd = the recorded root (staged agent pipeline + config).
# Usage: run-agent.sh <version> <image-ref>. Writes the version dir to /out/<version>-agent.
# The LLM is llm_stub.py on 127.0.0.1: no credentials are passed in, nothing leaves the container
# except the nf-agent plugin download.
set -euo pipefail
VER=$1 IMAGE=$2 TOOLS=/nf-agents-tools
OUT=/out/$VER-agent
ORACLE=$OUT/oracle
# shellcheck source=lib.sh
source "$TOOLS/lib.sh"
trap 'chown -R "${HOST_UID:-0}:${HOST_GID:-0}" /out "$PWD"' EXIT
command -v python3 > /dev/null || die "python3 not found in the image"
mkdir -p "$ORACLE"

python3 "$TOOLS/llm_stub.py" 18080 > /tmp/stub.ready 2> /tmp/stub.err &
until [[ -s /tmp/stub.ready ]]; do
  kill -0 $! 2> /dev/null || die "llm stub did not start: $(cat /tmp/stub.err)"
  sleep 0.1
done
nextflow run pipeline/main.nf > /tmp/run.log 2>&1 || { tail -n 40 /tmp/run.log >&2; die "$VER: agent run failed"; }
python3 "$TOOLS/ids.py" --agent .lineage "$ORACLE/ids.json" > /tmp/ids.env
while IFS='=' read -r k v; do printf -v "$k" %s "$v"; done < /tmp/ids.env

cap version -version
cap list lineage list
cap find-agentrun lineage find type=AgentRun
for a in qa qa_structured shouty; do
  lid=AGENT_${a^^}
  cap "view-agentrun-${a//_/-}" lineage view "${!lid}"
  cap "view-agentrun-output-${a//_/-}" lineage view "${!lid}#output"
done
cap render-agentrun lineage render "$AGENT_SHOUTY" /tmp/render-agentrun.html

python3 "$TOOLS/export.py" --kind agent --version "$VER" --image "$IMAGE" --recorded-root "$PWD" . "$OUT"
python3 "$TOOLS/shape.py" "$OUT" > "$OUT/shape.txt"
