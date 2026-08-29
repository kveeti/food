#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
admin_url="${DATABASE_URL:?DATABASE_URL must point at the local Postgres database}"
database="food_v5_e2e_$$"
database_url="${admin_url%/*}/$database"
tmp="$(mktemp -d)"
app_pid=""
idp_pid=""
replica_pid=""

cleanup() {
  trap - EXIT INT TERM
  for pid in "$replica_pid" "$app_pid" "$idp_pid"; do
    if [[ -n "$pid" ]]; then
      kill "$pid" 2>/dev/null || true
      wait "$pid" 2>/dev/null || true
    fi
  done
  psql "$admin_url" -c "DROP DATABASE IF EXISTS \"$database\" WITH (FORCE)" >/dev/null
  rm -rf "$tmp"
}

trap cleanup EXIT
trap 'exit 0' INT TERM

psql "$admin_url" -v ON_ERROR_STOP=1 -c "CREATE DATABASE \"$database\"" >/dev/null

cd "$root"
cargo build --bins >"$tmp/build.log" 2>&1 || {
  cat "$tmp/build.log" >&2
  exit 1
}

export DATABASE_URL="$database_url"
export APP_URL=http://127.0.0.1:8200
export OIDC_ISSUER=http://127.0.0.1:8201
export OIDC_CLIENT_ID=food-dev
export OIDC_CLIENT_SECRET=food-dev-secret
export SESSION_ENCRYPTION_KEY=f/HGP+VLA41YaUwW1ufnIjL2dUkt8gcgmUJLuVyY7Zg=
export ALLOW_INSECURE_OIDC=1
export IDP_ACCESS_TOKEN_LIFETIME=2
export IDP_REFRESH_DELAY=500

PORT=8201 "$root/target/debug/dev_idp" >"$tmp/idp.log" 2>&1 &
idp_pid=$!
for _ in {1..200}; do
  (echo >/dev/tcp/127.0.0.1/8201) 2>/dev/null && break
  sleep 0.1
done
(echo >/dev/tcp/127.0.0.1/8201) 2>/dev/null || {
  cat "$tmp/idp.log" >&2
  exit 1
}

PORT=8200 "$root/target/debug/food" >"$tmp/app.log" 2>&1 &
app_pid=$!
for _ in {1..200}; do
  (echo >/dev/tcp/127.0.0.1/8200) 2>/dev/null && break
  sleep 0.1
done
(echo >/dev/tcp/127.0.0.1/8200) 2>/dev/null || {
  cat "$tmp/app.log" >&2
  exit 1
}

PORT=8202 "$root/target/debug/food" >"$tmp/replica.log" 2>&1 &
replica_pid=$!
for _ in {1..200}; do
  (echo >/dev/tcp/127.0.0.1/8202) 2>/dev/null && break
  sleep 0.1
done
(echo >/dev/tcp/127.0.0.1/8202) 2>/dev/null || {
  cat "$tmp/replica.log" >&2
  exit 1
}

wait "$app_pid"
