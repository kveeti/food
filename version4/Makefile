ifneq (,$(wildcard ./.env))
	include .env
	export
endif

include dev-idp.env

PORT ?= 8000
IDP_PORT ?= $(shell expr $(PORT) + 1)
LOCAL_APP_URL = http://127.0.0.1:$(PORT)
LOCAL_OIDC_ISSUER = http://127.0.0.1:$(IDP_PORT)

.PHONY: dev devi check e2e watch-local-app watch-idp

dev:
	@test -f .env || { echo "Copy .env.example to .env and fill it in."; exit 1; }
	@deno run --watch --allow-env --allow-net --allow-read=drizzle main.ts

devi:
	@test -f .env || { echo "Copy .env.example to .env and fill it in."; exit 1; }
	@$(MAKE) --no-print-directory -j2 watch-local-app watch-idp

watch-local-app:
	@PORT="$(PORT)" APP_URL="$(LOCAL_APP_URL)" \
	OIDC_ISSUER="$(LOCAL_OIDC_ISSUER)" \
	OIDC_CLIENT_ID="$(DEV_OIDC_CLIENT_ID)" \
	OIDC_CLIENT_SECRET="$(DEV_OIDC_CLIENT_SECRET)" \
	ALLOW_INSECURE_OIDC=1 \
	deno run --watch --allow-env --allow-net --allow-read=drizzle main.ts

watch-idp:
	@PORT="$(IDP_PORT)" APP_URL="$(LOCAL_APP_URL)" \
	OIDC_ISSUER="$(LOCAL_OIDC_ISSUER)" \
	OIDC_CLIENT_ID="$(DEV_OIDC_CLIENT_ID)" \
	OIDC_CLIENT_SECRET="$(DEV_OIDC_CLIENT_SECRET)" \
	deno run --watch --allow-env --allow-net dev-idp/main.ts

check:
	@deno task check

e2e:
	@deno task e2e
