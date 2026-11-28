#!/usr/bin/env bash
# Release gate for the workspace in the current directory: every member must be publishable,
# carry complete crates.io metadata, and share one version; a given tag must equal v<version>.
# Usage: scripts/release/check.sh [tag]
set -euo pipefail

if [ "$#" -gt 1 ]; then
  echo "usage: check.sh [tag]" >&2
  exit 2
fi
tag="${1:-}"

meta=$(cargo metadata --format-version 1 --no-deps)

problems=$(jq -r --arg tag "$tag" '
  (.packages[] | select(.publish == []) | "\(.name): publish = false"),
  ([.packages[] | select(.publish != [])] as $pub
    | if ($pub | length) == 0 then "no publishable crates" else empty end,
      ($pub[] | . as $p
        | (["description", "license", "repository", "readme"][]
            | select(($p[.] // "") == "") | "\($p.name): empty \(.)"),
          (["keywords", "categories"][]
            | select(($p[.] // []) == []) | "\($p.name): empty \(.)"),
          (select($tag != "" and $tag != "v" + $p.version)
            | "\($p.name): tag \($tag) is not v\($p.version)")),
      ($pub | map(.version) | unique | select(length > 1)
        | "versions differ: \(join(", "))"))
' <<<"$meta")

if [ -n "$problems" ]; then
  printf '%s\n' "$problems" >&2
  exit 1
fi

version=$(jq -r '[.packages[] | select(.publish != []) | .version] | unique[0]' <<<"$meta")
echo "release check ok: $version${tag:+ ($tag)}"
