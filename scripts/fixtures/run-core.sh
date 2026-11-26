#!/usr/bin/env bash
# Runs inside nextflow/nextflow:<version>, cwd = the recorded root (staged pipeline + config).
# Usage: run-core.sh <version> <image-ref>. Writes the version dir to /out/<version>.
set -euo pipefail
VER=$1 IMAGE=$2 TOOLS=/nf-agents-tools
OUT=/out/$VER
ORACLE=$OUT/oracle
# shellcheck source=lib.sh
source "$TOOLS/lib.sh"
trap 'chown -R "${HOST_UID:-0}:${HOST_GID:-0}" /out "$PWD"' EXIT
command -v python3 > /dev/null || die "python3 not found in the image"
mkdir -p "$ORACLE"

for g in hello bonjour; do
  nextflow run pipeline/main.nf --greeting "$g" -output-dir "results-$g" > "/tmp/run-$g.log" 2>&1 ||
    { tail -n 40 "/tmp/run-$g.log" >&2; die "$VER: nextflow run ($g) failed"; }
done
python3 "$TOOLS/ids.py" .lineage "$ORACLE/ids.json" > /tmp/ids.env
while IFS='=' read -r k v; do printf -v "$k" %s "$v"; done < /tmp/ids.env

cap version -version
cap help lineage -h
cap list lineage list
cap view-workflowrun lineage view "$RUN1"
cap view-workflowrun-output lineage view "$RUN1#output"
cap view-published-file lineage view "$RUN1/summary.txt"
cap view-published-file-subdir lineage view "$RUN1/upper/upper.txt"
cap view-taskrun lineage view "$TASK_GENERATE_RUN1"
cap view-taskrun-summarize lineage view "$TASK_SUMMARIZE_RUN1"
cap view-taskoutput lineage view "$TASK_SUMMARIZE_RUN1#output"
cap view-task-fileoutput lineage view "$TASK_SUMMARIZE_RUN1/summary.txt"
cap view-field lineage view "$RUN1#params"
cap view-not-lid lineage view foo
cap view-missing lineage view lid://00000000000000000000000000000000
cap find-taskrun lineage find type=TaskRun "workflowRun=$RUN1"
cap find-workflowrun lineage find type=WorkflowRun
cap find-fileoutput lineage find type=FileOutput
cap find-fileoutput-label lineage find type=FileOutput label=summary
cap find-taskoutput lineage find type=TaskOutput
cap find-workflowoutput lineage find type=WorkflowOutput
cap find-bad-field lineage find foo=bar
cap find-noeq lineage find type
cap diff-runs lineage diff "$RUN1" "$RUN2"
cap diff-tasks lineage diff "$TASK_GENERATE_RUN1" "$TASK_GENERATE_RUN2"
cap check-published lineage check "$RUN1/summary.txt"
cap check-taskfile lineage check "$TASK_SUMMARIZE_RUN1/summary.txt"
cap check-taskrun lineage check "$TASK_GENERATE_RUN1"
cap render-file lineage render "$RUN1/summary.txt" /tmp/render-file.html
cap render-run lineage render "$RUN1" /tmp/render-run.html

python3 "$TOOLS/export.py" --kind core --version "$VER" --image "$IMAGE" --recorded-root "$PWD" . "$OUT"

# Tamper cases run on the live tree, which is disposable once exported.
replace() { # replace a (possibly symlinked) file by a fresh regular copy: same bytes, new mtime
  local t
  t=$(mktemp)
  cp "$1" "$t"
  rm "$1"
  mv "$t" "$1"
}
replace results-hello/summary.txt
echo tampered >> results-hello/summary.txt
cap check-tampered lineage check "$RUN1/summary.txt"
replace results-bonjour/summary.txt
cap check-mtime-only lineage check "$RUN2/summary.txt"
