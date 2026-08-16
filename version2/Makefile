ifneq (,$(wildcard ./.env))
	include .env
	export
endif

.PHONY: dev build start typecheck db-generate e2e fmt fmt-check

PORT ?= 3000

dev:
	@pnpm run dev --host 0.0.0.0 --port $(PORT)

build:
	@pnpm run build

start:
	@pnpm run start

typecheck:
	@pnpm run typecheck

db-generate:
	@pnpm run db:generate

e2e:
	@pnpm run e2e

fmt:
	@pnpm run fmt

fmt-check:
	@pnpm run fmt:check
