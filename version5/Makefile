ifneq (,$(wildcard ./.env))
	include .env
	export
endif

include dev-idp.env

PORT ?= 8000
IDP_PORT ?= $(shell expr $(PORT) + 1)
LOCAL_APP_URL = http://127.0.0.1:$(PORT)
LOCAL_OIDC_ISSUER = http://127.0.0.1:$(IDP_PORT)

.PHONY: dev devi otel build check e2e fmt watch-local-app watch-idp

dev:
	@test -f .env || { echo "Copy .env.example to .env and fill it in."; exit 1; }
	@cargo run --bin food

devi:
	@$(MAKE) --no-print-directory -j2 watch-local-app watch-idp

otel:
	@otel-tui

watch-local-app:
	@PORT="$(PORT)" APP_URL="$(LOCAL_APP_URL)" \
	OIDC_ISSUER="$(LOCAL_OIDC_ISSUER)" \
	OIDC_CLIENT_ID="$(DEV_OIDC_CLIENT_ID)" \
	OIDC_CLIENT_SECRET="$(DEV_OIDC_CLIENT_SECRET)" \
	SESSION_ENCRYPTION_KEY="$(DEV_SESSION_ENCRYPTION_KEY)" \
	ALLOW_INSECURE_OIDC=1 \
	cargo run --bin food

watch-idp:
	@PORT="$(IDP_PORT)" APP_URL="$(LOCAL_APP_URL)" \
	OIDC_ISSUER="$(LOCAL_OIDC_ISSUER)" \
	OIDC_CLIENT_ID="$(DEV_OIDC_CLIENT_ID)" \
	OIDC_CLIENT_SECRET="$(DEV_OIDC_CLIENT_SECRET)" \
	cargo run --bin dev_idp

build:
	@cargo build --bins

check:
	@cargo fmt --check
	@cargo test --bins
	@cargo clippy --bins -- -D warnings
	@deno task check

e2e:
	@deno task e2e

fmt:
	@cargo fmt
	@deno fmt
