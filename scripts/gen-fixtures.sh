#!/usr/bin/env bash
# Generate the committed lineage fixtures (fixtures/README.md). Host needs only bash and Docker.
# Usage: scripts/gen-fixtures.sh [--verify] [--out DIR] [VERSION...]
#   VERSION   Nextflow image tag (default: the core matrix below); 26.10.7 also builds 26.10.7-agent
#   --out     where version dirs are written (default: fixtures/lineage)
#   --verify  only re-run the round trip on existing version dirs, no regeneration
set -euo pipefail
REPO=$(cd "$(dirname "$0")/.." && pwd)
TOOLS=$REPO/scripts/fixtures
PIPES=$REPO/fixtures/pipelines
MOUNT=/nf-agents-fixture
CORE=(25.04.8 25.10.9 26.04.9 26.10.7)
AGENT=26.10.7 # the AgentRun store (fixtures/lineage/26.10.7-agent) is built with this version
# shellcheck source=fixtures/lib.sh
source "$TOOLS/lib.sh"

verify=0 out=$REPO/fixtures/lineage versions=()
while (($#)); do
  case $1 in
    --verify) verify=1 ;;
    --out) (($# >= 2)) || die "--out needs a directory"; out=$2; shift ;;
    -h | --help) sed -n '2,6p' "$0"; exit 0 ;;
    -*) die "unknown option: $1" ;;
    *) [[ $1 =~ ^[0-9]+\.[0-9]+\.[0-9]+(-edge)?$ ]] || die "invalid version tag: $1"; versions+=("$1") ;;
  esac
  shift
done
((${#versions[@]})) || versions=("${CORE[@]}")
command -v docker > /dev/null || die "docker not found"
docker info > /dev/null 2>&1 || die "docker daemon not reachable"

mkdir -p "$out"
out=$(cd "$out" && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT # containers chown what they write back to the host user
export HOST_UID HOST_GID
HOST_UID=$(id -u) HOST_GID=$(id -g)
# check.py already rejects /home/, /Users/ and CI workspace paths; also reject the literal $HOME,
# except /root, which is the container's own home and legitimately recorded (metadata.homeDir).
forbid=()
[[ ${HOME:-/} == / || $HOME == /root ]] || forbid=(--forbid "$HOME")

in_image() { # in_image <version> <docker opts...> -- <command...>: one step in nextflow/nextflow:<version>
  local v=$1 opts=()
  shift
  while [[ $1 != -- ]]; do opts+=("$1"); shift; done
  shift
  docker run --rm --cpus 2 -e NXF_ANSI_LOG=false -e HOST_UID -e HOST_GID \
    -v "$TOOLS:/baqueano-tools:ro" "${opts[@]}" "nextflow/nextflow:$v" "$@"
}

roundtrip() { # roundtrip <version> <version-dir>
  local dst=$tmp/verify-${2##*/}
  cp -r "$2" "$dst" # plain copy: mtimes are lost, as after a git checkout
  in_image "$1" -v "$dst:$MOUNT" -w "$MOUNT" -- bash /baqueano-tools/verify.sh ||
    die "$1: round trip failed"
}

generate() { # generate <version> core|agent
  local v=$1 kind=$2 img=nextflow/nextflow:$1 digest name=$1 pipe=$PIPES/minimal
  [[ $kind == core ]] || { name=$v-agent pipe=$PIPES/agent; }
  local root=$tmp/root-$name stage=$tmp/out-$name
  docker pull -q "$img" > /dev/null || die "$v: cannot pull $img"
  digest=$(docker image inspect --format '{{index .RepoDigests 0}}' "$img") || die "$v: no digest for $img"
  mkdir -p "$root/pipeline" "$stage"
  { [[ $v != 25.04.* ]] || echo 'nextflow.preview.output = true'; cat "$pipe/main.nf"; } > "$root/pipeline/main.nf"
  [[ ! -d $pipe/data ]] || cp -r "$pipe/data" "$root/pipeline/"
  cp "$pipe/nextflow.config" "$root/"
  in_image "$v" -v "$root:$MOUNT" -v "$stage:/out" -w "$MOUNT" -- \
    bash "/baqueano-tools/run-$kind.sh" "$v" "$img@${digest#*@}" || die "$name: generation failed"
  roundtrip "$v" "$stage/$name"
  in_image "$v" -v "$stage:/check:ro" -- python3 -I /baqueano-tools/check.py /check "${forbid[@]}" ||
    die "$name: fixture check failed"
  rm -rf "${out:?}/$name"
  mv "$stage/$name" "$out/$name"
  echo "[$name] written to $out/$name"
}

for v in "${versions[@]}"; do
  names=("$v")
  [[ $v != "$AGENT" ]] || names+=("$v-agent")
  for name in "${names[@]}"; do
    if ((verify)); then
      [[ -d $out/$name ]] || die "$name: no fixtures at $out/$name"
      roundtrip "$v" "$out/$name"
    else
      generate "$v" "$([[ $name == *-agent ]] && echo agent || echo core)"
    fi
  done
done
