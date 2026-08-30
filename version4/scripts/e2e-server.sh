#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
admin_url="${DATABASE_URL:?DATABASE_URL must point at the local Postgres database}"
database="food_e2e_$$"
database_url="${admin_url%/*}/$database"
app_pid=""
idp_pid=""
replica_pid=""

cleanup() {
  trap - EXIT INT TERM
  if [[ -n "$replica_pid" ]]; then
    kill "$replica_pid" 2>/dev/null || true
    wait "$replica_pid" 2>/dev/null || true
  fi
  if [[ -n "$app_pid" ]]; then
    kill "$app_pid" 2>/dev/null || true
    wait "$app_pid" 2>/dev/null || true
  fi
  if [[ -n "$idp_pid" ]]; then
    kill "$idp_pid" 2>/dev/null || true
    wait "$idp_pid" 2>/dev/null || true
  fi
  psql "$admin_url" -c "DROP DATABASE IF EXISTS \"$database\" WITH (FORCE)" >/dev/null
}

trap cleanup EXIT
trap 'exit 0' INT TERM

psql "$admin_url" -v ON_ERROR_STOP=1 -c "CREATE DATABASE \"$database\"" >/dev/null

export DATABASE_URL="$database_url"
export APP_URL=http://127.0.0.1:8200
export OIDC_ISSUER=http://127.0.0.1:8201
export OIDC_CLIENT_ID=food-dev
export OIDC_CLIENT_SECRET=food-dev-secret
export SESSION_ENCRYPTION_KEY=f/HGP+VLA41YaUwW1ufnIjL2dUkt8gcgmUJLuVyY7Zg=
export ALLOW_INSECURE_OIDC=1
export IDP_ACCESS_TOKEN_LIFETIME=2
export IDP_REFRESH_DELAY=500

cd "$root"
PORT=8201 deno run --allow-env --allow-net dev-idp/main.ts &
idp_pid=$!

for _ in {1..100}; do
  (echo >/dev/tcp/127.0.0.1/8201) 2>/dev/null && break
  kill -0 "$idp_pid" 2>/dev/null || wait "$idp_pid"
  sleep 0.1
done
(echo >/dev/tcp/127.0.0.1/8201) 2>/dev/null || {
  echo "dev IdP failed to start on port 8201" >&2
  exit 1
}

PORT=8200 deno task start &
app_pid=$!

for _ in {1..100}; do
  (echo >/dev/tcp/127.0.0.1/8200) 2>/dev/null && break
  kill -0 "$app_pid" 2>/dev/null || wait "$app_pid"
  sleep 0.1
done
(echo >/dev/tcp/127.0.0.1/8200) 2>/dev/null || {
  echo "app failed to start on port 8200" >&2
  exit 1
}

PORT=8202 deno task start &
replica_pid=$!

for _ in {1..100}; do
  (echo >/dev/tcp/127.0.0.1/8202) 2>/dev/null && break
  kill -0 "$replica_pid" 2>/dev/null || wait "$replica_pid"
  sleep 0.1
done
(echo >/dev/tcp/127.0.0.1/8202) 2>/dev/null || {
  echo "app replica failed to start on port 8202" >&2
  exit 1
}

wait "$app_pid"
