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
topcoat asset bundle --bin food >"$tmp/build.log" 2>&1 || {
  cat "$tmp/build.log" >&2
  exit 1
}
cargo build --bin dev_idp >>"$tmp/build.log" 2>&1 || {
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

psql "$DATABASE_URL" -v ON_ERROR_STOP=1 >/dev/null <<'SQL'
INSERT INTO foods (id, source, source_id, display_name, brand, basis_unit, source_data)
VALUES
  ('00000000-0000-7000-8000-000000000101', 'fineli', 'test-milk',
   'Maito, rasvaton', NULL, 'g', '{"food_type":"FOOD","process":"IND"}'),
  ('00000000-0000-7000-8000-000000000102', 'open_food_facts', '6411401015098',
   'Karl Fazer Maitosuklaa', 'Fazer', 'g', '{}'),
  ('00000000-0000-7000-8000-000000000103', 'fineli', 'test-apple',
   'Omena, keskiarvo', NULL, 'g', '{"food_type":"FOOD","process":"RAW"}');

INSERT INTO food_names (food_id, name, locale)
VALUES
  ('00000000-0000-7000-8000-000000000101', 'Maito, rasvaton', 'fi'),
  ('00000000-0000-7000-8000-000000000101', 'Skim milk', 'en'),
  ('00000000-0000-7000-8000-000000000101', 'Skummjölk', 'sv'),
  ('00000000-0000-7000-8000-000000000102', 'Milk chocolate', 'en'),
  ('00000000-0000-7000-8000-000000000103', 'Omena', 'fi'),
  ('00000000-0000-7000-8000-000000000103', 'Apple', 'en'),
  ('00000000-0000-7000-8000-000000000103', 'Äpple', 'sv');

INSERT INTO food_nutrients (food_id, nutrient_id, value)
SELECT food_id, nutrients.id, value
FROM (VALUES
  ('00000000-0000-7000-8000-000000000101'::uuid, 'energy', 146.0),
  ('00000000-0000-7000-8000-000000000101'::uuid, 'protein', 3.5),
  ('00000000-0000-7000-8000-000000000101'::uuid, 'carbohydrate', 4.8),
  ('00000000-0000-7000-8000-000000000101'::uuid, 'fat', 0.1),
  ('00000000-0000-7000-8000-000000000101'::uuid, 'sodium', 0.04),
  ('00000000-0000-7000-8000-000000000102'::uuid, 'energy', 2250.0),
  ('00000000-0000-7000-8000-000000000102'::uuid, 'protein', 8.1),
  ('00000000-0000-7000-8000-000000000102'::uuid, 'carbohydrate', 49.0),
  ('00000000-0000-7000-8000-000000000102'::uuid, 'fat', 31.0),
  ('00000000-0000-7000-8000-000000000103'::uuid, 'energy', 185.0),
  ('00000000-0000-7000-8000-000000000103'::uuid, 'fibre', 2.4)
) AS values(food_id, nutrient_code, value)
JOIN nutrients ON nutrients.code = values.nutrient_code;

SELECT refresh_food_search_vector(id) FROM foods;
SQL

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
