# shellcheck shell=bash
# Shared helpers for the fixture generator (sourced, host and container side).

die() {
  echo "gen-fixtures: $*" >&2
  exit 1
}

# cap <case> <argv...>: run `nextflow <argv...>` and write $ORACLE/<case>.{args,out,err,exit}.
# Every argv element must match SAFE_ARG, so the .args JSON needs no escaping and no shell
# metacharacter ever reaches Nextflow. Unsafe input aborts before anything runs or is written.
SAFE_ARG='^[A-Za-z0-9:/#=._-]+$'
cap() {
  local name=$1 a json='' rc=0
  shift
  [[ $name =~ ^[a-z0-9-]+$ ]] || die "unsafe case name: $name"
  for a in "$@"; do
    [[ $a =~ $SAFE_ARG ]] || die "unsafe argument in case $name: $a"
    json+="${json:+, }\"$a\""
  done
  printf '[%s]\n' "$json" > "$ORACLE/$name.args"
  nextflow "$@" > "$ORACLE/$name.out" 2> "$ORACLE/$name.err" || rc=$?
  echo "$rc" > "$ORACLE/$name.exit"
}
