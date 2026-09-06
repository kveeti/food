ifneq (,$(wildcard ./.env))
	include .env
	export
endif

include dev-idp.env

BACKEND_PORT ?= 8000
FRONTEND_PORT ?= 3000
IDP_PORT ?= 8001
BACKEND_URL = http://127.0.0.1:$(BACKEND_PORT)
LOCAL_APP_URL = http://127.0.0.1:$(FRONTEND_PORT)
LOCAL_OIDC_ISSUER = http://127.0.0.1:$(IDP_PORT)

.PHONY: frontend backend dev-idp dev devi build check e2e fmt
MAKEFLAGS += -j

frontend/node_modules/.modules.yaml: frontend/package.json frontend/pnpm-lock.yaml
	@cd frontend && pnpm install --frozen-lockfile

frontend: frontend/node_modules/.modules.yaml
	@cd frontend && VITE_PORT="$(FRONTEND_PORT)" BACKEND_URL="$(BACKEND_URL)" pnpm run dev

backend:
	@PORT="$(BACKEND_PORT)" cargo run --manifest-path backend/Cargo.toml --bin food-backend

dev-idp:
	@PORT="$(IDP_PORT)" cargo run --manifest-path backend/Cargo.toml --bin dev_idp

dev: frontend backend

devi: export APP_URL=$(LOCAL_APP_URL)
devi: export OIDC_ISSUER=$(LOCAL_OIDC_ISSUER)
devi: export OIDC_CLIENT_ID=$(DEV_OIDC_CLIENT_ID)
devi: export OIDC_CLIENT_SECRET=$(DEV_OIDC_CLIENT_SECRET)
devi: export SESSION_ENCRYPTION_KEY=$(DEV_SESSION_ENCRYPTION_KEY)
devi: export ALLOW_INSECURE_OIDC=1
devi: frontend backend dev-idp

build: frontend/node_modules/.modules.yaml
	@cd frontend && pnpm run build
	@cargo build --manifest-path backend/Cargo.toml --bins

check: frontend/node_modules/.modules.yaml
	@cargo fmt --manifest-path backend/Cargo.toml --check
	@cargo test --manifest-path backend/Cargo.toml --bins
	@cargo clippy --manifest-path backend/Cargo.toml --bins -- -D warnings
	@cd frontend && pnpm run fmt:check && pnpm run lint && pnpm run typecheck

# Each Playwright test gets a new database on the dev-shell PostgreSQL server.
e2e: frontend/node_modules/.modules.yaml
	@cd frontend && pnpm run e2e

fmt: frontend/node_modules/.modules.yaml
	@cargo fmt --manifest-path backend/Cargo.toml
	@cd frontend && pnpm run fmt
