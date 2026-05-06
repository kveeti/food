# Export variables from ./.env if it exists
ifneq (,$(wildcard ./.env))
	include .env
	export
endif

.PHONY: all
MAKEFLAGS += -j

frontdev:
	@cd front && pnpm run dev
backdev:
	@cd back && cargo watch -x run
dev: backdev frontdev

frontbuild:
	@cd front && pnpm run build
backbuild:
	@cd back && cargo build --release
build: backbuild frontbuild

frontpreview:
	@cd front && pnpm run build && pnpm run preview
backpreview:
	@cd back && FRONTEND_DIR=../front/dist cargo run --release
preview: backpreview frontpreview

