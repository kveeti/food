#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 || "$1" != *:* ]]; then
  echo "usage: $0 HOST:REMOTE_FILE.csv.gz" >&2
  exit 2
fi

host=${1%%:*}
remote_path=${1#*:}
printf -v quoted_path '%q' "$remote_path"
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

cargo build --bin import-foods
ssh "$host" "cat -- $quoted_path" |
  gzip -dc |
  target/debug/import-foods open-food-facts -
