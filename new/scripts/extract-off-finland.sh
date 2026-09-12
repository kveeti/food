#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 || $# -gt 2 ]]; then
    echo "usage: $0 OFF-JSONL[.GZ] [OUTPUT.GZ|-]" >&2
    exit 2
fi

input=$1
output=${2:--}
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)

if [[ $output == - ]]; then
    temporary="${TMPDIR:-/tmp}/food-off-feed.$$.jsonl.gz"
else
    temporary="$output.tmp.$$"
fi
rm -f -- "$temporary"
trap 'rm -f -- "$temporary"' EXIT

OFF_INPUT="$input" OFF_OUTPUT="$temporary" \
    nix shell nixpkgs#duckdb -c duckdb -init /dev/null \
    < "$script_dir/extract-off-finland.sql" >/dev/null

if [[ $output == - ]]; then
    cat -- "$temporary"
else
    mv -- "$temporary" "$output"
fi
trap - EXIT
