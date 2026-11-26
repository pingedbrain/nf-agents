#!/usr/bin/env bash
# Runs inside nextflow/nextflow:<version> on a plain copy of a version dir (mtimes lost, as after
# a git checkout) mounted at the recorded root. Proves the mtime manifest is necessary and enough.
set -euo pipefail
TOOLS=/nf-agents-tools
# shellcheck source=lib.sh
source "$TOOLS/lib.sh"
trap 'chown -R "${HOST_UID:-0}:${HOST_GID:-0}" "$PWD"' EXIT
RUN1=$(python3 -c 'import json, sys; print(json.load(open(sys.argv[1]))["RUN1"])' oracle/ids.json)

rc=0
nextflow lineage check "$RUN1/summary.txt" > /dev/null 2>&1 || rc=$?
[[ $rc == 1 ]] || die "check without restored mtimes exited $rc, expected 1"

python3 "$TOOLS/restore_mtimes.py" .
lids=$(python3 -c 'import sys; sys.path.insert(0, sys.argv[1]); import export
print("\n".join(sorted(export.data_paths(".lineage", sys.argv[2]))))' "$TOOLS" "$PWD")
for lid in $lids; do
  out=$(nextflow lineage check "$lid" 2>&1) || die "check $lid failed: $out"
  [[ $out == *"Checksum validation succeed"* ]] || die "check $lid: $out"
done
nextflow lineage list > /tmp/list.out 2> /tmp/list.err || die "lineage list failed: $(cat /tmp/list.err)"
[[ $(sha256sum < /tmp/list.out) == $(sha256sum < oracle/list.out) ]] || die "lineage list differs from oracle/list.out"
echo "round trip ok: $(wc -w <<< "$lids") FileOutputs"
