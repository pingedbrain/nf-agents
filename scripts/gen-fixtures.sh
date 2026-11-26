#!/usr/bin/env bash
# Generate the committed lineage fixtures (fixtures/README.md). Host needs only bash and Docker.
# Usage: scripts/gen-fixtures.sh [--verify] [--out DIR] [VERSION...]
#   VERSION   Nextflow image tag (default: the core matrix below)
#   --out     where version dirs are written (default: fixtures/lineage)
#   --verify  only re-run the round trip on existing version dirs, no regeneration
set -euo pipefail
REPO=$(cd "$(dirname "$0")/.." && pwd)
TOOLS=$REPO/scripts/fixtures
PIPE=$REPO/fixtures/pipelines/minimal
MOUNT=/nf-agents-fixture
CORE=(25.04.8 25.10.9 26.04.9 26.10.7)
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
forbid=()
[[ $(id -un) == root ]] || forbid=(--forbid "$(id -un)")

in_image() { # in_image <version> <docker opts...> -- <command...>: one step in nextflow/nextflow:<version>
  local v=$1 opts=()
  shift
  while [[ $1 != -- ]]; do opts+=("$1"); shift; done
  shift
  docker run --rm --cpus 2 -e NXF_ANSI_LOG=false -e HOST_UID -e HOST_GID \
    -v "$TOOLS:/nf-agents-tools:ro" "${opts[@]}" "nextflow/nextflow:$v" "$@"
}

roundtrip() { # roundtrip <version> <version-dir>
  local dst=$tmp/verify-$1
  cp -r "$2" "$dst" # plain copy: mtimes are lost, as after a git checkout
  in_image "$1" -v "$dst:$MOUNT" -w "$MOUNT" -- bash /nf-agents-tools/verify.sh ||
    die "$1: round trip failed"
}

generate() { # generate <version>
  local v=$1 img=nextflow/nextflow:$1 digest root=$tmp/root-$1 stage=$tmp/out-$1
  docker pull -q "$img" > /dev/null || die "$v: cannot pull $img"
  digest=$(docker image inspect --format '{{index .RepoDigests 0}}' "$img") || die "$v: no digest for $img"
  mkdir -p "$root/pipeline/data" "$stage"
  { [[ $v != 25.04.* ]] || echo 'nextflow.preview.output = true'; cat "$PIPE/main.nf"; } > "$root/pipeline/main.nf"
  cp "$PIPE/data/seed.txt" "$root/pipeline/data/"
  cp "$PIPE/nextflow.config" "$root/"
  in_image "$v" -v "$root:$MOUNT" -v "$stage:/out" -w "$MOUNT" -- \
    bash /nf-agents-tools/run-core.sh "$v" "$img@${digest#*@}" || die "$v: generation failed"
  roundtrip "$v" "$stage/$v"
  in_image "$v" -v "$stage:/check:ro" -- python3 -I /nf-agents-tools/check.py /check "${forbid[@]}" ||
    die "$v: fixture check failed"
  rm -rf "${out:?}/$v"
  mv "$stage/$v" "$out/$v"
  echo "[$v] written to $out/$v"
}

for v in "${versions[@]}"; do
  if ((verify)); then
    [[ -d $out/$v ]] || die "$v: no fixtures at $out/$v"
    roundtrip "$v" "$out/$v"
  else
    generate "$v"
  fi
done
