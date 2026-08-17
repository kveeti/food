#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
admin_url="${DATABASE_URL:?DATABASE_URL must point at the local postgres database}"
database="food_e2e_$$"
database_url="${admin_url%/*}/$database"
port=8200
tmp="$(mktemp -d)"
app_pid=""

cleanup() {
  trap - EXIT INT TERM
  if [[ -n "$app_pid" ]]; then
    kill "$app_pid" 2>/dev/null || true
    wait "$app_pid" 2>/dev/null || true
  fi
  psql "$admin_url" -c "DROP DATABASE IF EXISTS \"$database\" WITH (FORCE)" >/dev/null
  if [[ -s "$tmp/app.log" ]]; then
    cat "$tmp/app.log" >&2
  fi
  rm -rf "$tmp"
}
trap cleanup EXIT
trap 'cleanup; exit 0' INT TERM

psql "$admin_url" -v ON_ERROR_STOP=1 -c "CREATE DATABASE \"$database\"" >/dev/null

cd "$root"
"$HOME/.cargo/bin/topcoat" asset bundle --bin version3 >"$tmp/build.log" 2>&1 || {
  cat "$tmp/build.log" >&2
  exit 1
}

PORT="$port" \
APP_URL="http://127.0.0.1:$port" \
DATABASE_URL="$database_url" \
IS_PROD=0 \
"$root/target/debug/version3" >"$tmp/app.log" 2>&1 &
app_pid=$!

set +e
wait "$app_pid"
status=$?
set -e
cat "$tmp/app.log" >&2
exit "$status"
