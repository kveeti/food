#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 || "$1" != *:* ]]; then
  echo "usage: $0 HOST:REMOTE_FILE.jsonl.gz" >&2
  exit 2
fi

host=${1%%:*}
remote_path=${1#*:}
if [[ "$remote_path" =~ [^A-Za-z0-9._/-] ]]; then
  echo "remote file contains unsupported characters" >&2
  exit 2
fi

remote_output="/tmp/open-food-facts-finland-$$.jsonl"
sql=$(cat <<SQL
COPY (
  SELECT *
  FROM read_json(
    '$remote_path',
    format = 'newline_delimited',
    columns = {
      code: 'JSON',
      product_name: 'JSON',
      brands: 'JSON',
      generic_name: 'JSON',
      countries_tags: 'VARCHAR[]',
      nutrition_data_per: 'JSON',
      nutrition: 'JSON',
      nutriments: 'JSON',
      quantity: 'JSON',
      product_quantity: 'JSON',
      product_quantity_unit: 'JSON',
      serving_quantity: 'JSON',
      serving_quantity_unit: 'JSON'
    }
  )
  WHERE list_contains(countries_tags, 'en:finland')
) TO '$remote_output' (FORMAT JSON, ARRAY false);
SQL
)
printf -v quoted_sql '%q' "$sql"
printf -v quoted_output '%q' "$remote_output"

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

cargo build --features import-tools --bin import-foods
ssh "$host" \
  "set -e; trap 'rm -f -- $remote_output' EXIT; nix shell nixpkgs#duckdb -c duckdb -c $quoted_sql >&2; gzip -c -- $quoted_output" |
  target/debug/import-foods open-food-facts-gzip -
