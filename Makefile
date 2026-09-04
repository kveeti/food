ifneq (,$(wildcard ./.env))
	include .env
	export
endif

include dev-idp.env

PORT ?= 8000
IDP_PORT ?= $(shell expr $(PORT) + 1)
LOCAL_APP_URL = http://127.0.0.1:$(PORT)
LOCAL_OIDC_ISSUER = http://127.0.0.1:$(IDP_PORT)

.PHONY: all
MAKEFLAGS += -j

otel:
	@otel-tui

watch-local-app:
	@set -o pipefail; \
	PORT="$(PORT)" APP_URL="$(LOCAL_APP_URL)" \
	OIDC_ISSUER="$(LOCAL_OIDC_ISSUER)" \
	OIDC_CLIENT_ID="$(DEV_OIDC_CLIENT_ID)" \
	OIDC_CLIENT_SECRET="$(DEV_OIDC_CLIENT_SECRET)" \
	SESSION_ENCRYPTION_KEY="$(DEV_SESSION_ENCRYPTION_KEY)" \
	ALLOW_INSECURE_OIDC=1 \
	topcoat dev --bin food | cat

watch-idp:
	@PORT="$(IDP_PORT)" APP_URL="$(LOCAL_APP_URL)" \
	OIDC_ISSUER="$(LOCAL_OIDC_ISSUER)" \
	OIDC_CLIENT_ID="$(DEV_OIDC_CLIENT_ID)" \
	OIDC_CLIENT_SECRET="$(DEV_OIDC_CLIENT_SECRET)" \
	cargo run --bin dev_idp

devi: watch-local-app watch-idp
dev: watch-local-app

build:
	@topcoat asset bundle --bin food
	@cargo build --bin dev_idp

check:
	@topcoat fmt
	@cargo fmt --check
	@cargo test --bins
	@cargo clippy --bins -- -D warnings
	@deno task check

e2e:
	@deno task e2e

fmt:
	@topcoat fmt
	@cargo fmt
	@deno fmt
