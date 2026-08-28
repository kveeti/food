#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
admin_url="${DATABASE_URL:?DATABASE_URL must point at the local Postgres database}"
database="food_e2e_$$"
database_url="${admin_url%/*}/$database"
app_pid=""

cleanup() {
  trap - EXIT INT TERM
  if [[ -n "$app_pid" ]]; then
    kill "$app_pid" 2>/dev/null || true
    wait "$app_pid" 2>/dev/null || true
  fi
  psql "$admin_url" -c "DROP DATABASE IF EXISTS \"$database\" WITH (FORCE)" >/dev/null
}

trap cleanup EXIT
trap 'exit 0' INT TERM

psql "$admin_url" -v ON_ERROR_STOP=1 -c "CREATE DATABASE \"$database\"" >/dev/null

cd "$root"
PORT=8200 DATABASE_URL="$database_url" deno task start &
app_pid=$!
wait "$app_pid"
