#!/usr/bin/env bash
set -euo pipefail

: "${PGPORT:?Run the end-to-end tests from nix develop}"

database="food_e2e_${BASHPID}_${RANDOM}"
port=3200
server_pid=
database_created=

cleanup() {
  trap - EXIT INT TERM
  if [ -n "$server_pid" ]; then
    kill "$server_pid" 2>/dev/null || true
    wait "$server_pid" 2>/dev/null || true
  fi
  if [ -n "$database_created" ]; then
    dropdb --host 127.0.0.1 --port "$PGPORT" --username postgres --if-exists --force "$database" >/dev/null
  fi
}
trap cleanup EXIT INT TERM

createdb --host 127.0.0.1 --port "$PGPORT" --username postgres "$database"
database_created=1

PORT="$port" \
APP_URL="http://127.0.0.1:$port" \
DATABASE_URL="postgres://postgres@127.0.0.1:$PGPORT/$database" \
OIDC_ISSUER="http://127.0.0.1:$port/dev/oidc" \
OIDC_CLIENT_ID=dev \
OIDC_CLIENT_SECRET=dev \
OIDC_REDIRECT_URL="http://127.0.0.1:$port/auth/callback" \
SESSION_SECRET=food-e2e-secret-at-least-32-characters \
pnpm run dev --host 127.0.0.1 --port "$port" --strictPort &
server_pid=$!

set +e
wait "$server_pid"
status=$?
set -e
server_pid=
exit "$status"
