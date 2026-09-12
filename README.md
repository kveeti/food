## Local development

Rust, Topcoat, PostgreSQL, Deno, and Playwright

Both the app and the e2e tests need `DATABASE_URL` pointed to a running
PostgreSQL database

### Useful commands

- `make devi` runs the app at `http://127.0.0.1:8000` and the dev OIDC server at
  `http://127.0.0.1:8001`
- `make dev` runs only the app. Use it to test against a real OIDC provider
- `make check` runs the Rust and Deno checks
- `make e2e` runs the Playwright tests

## With Nix

```sh
nix develop
make devi
```

Nix shell installs Rust, Topcoat CLI, Deno, PostgreSQL and Playwright browsers.
It starts PostgreSQL in `.pg` and points `DATABASE_URL` to that

Use `fin` to stop PostgreSQL and leave the shell

## Without Nix

Install Rust with `cargo`, `rustfmt`, and `clippy`. Install Topcoat CLI, Deno
and PostgreSQL, then set `DATABASE_URL`.

```sh
cargo install topcoat-cli
```

Install the Chromium browser used by the end-to-end tests:

```sh
deno task e2e:browsers
```

Run the app with `topcoat dev --bin food` or use the Make commands above

## Food catalog imports

Import Fineli from an extracted release directory:

```sh
cargo run --features import-tools --bin import-foods -- \
  fineli "$HOME/Downloads/Fineli_rel20_74"
```

Extract Finnish products from a full OFF export or delta on the machine that
stores it:

```sh
scripts/extract-off-finland.sh openfoodfacts-products.jsonl.gz off-finland.jsonl.gz
```

Import a full feed with the Unix time or HTTP date published for that OFF
export. Do not use a timestamp changed by copying the file:

```sh
cargo run --features import-tools --bin import-foods -- \
  off-full --snapshot-end 'Sun, 23 Aug 2026 13:20:00 GMT' off-finland.jsonl.gz
```

Apply extracted deltas in order:

```sh
cargo run --features import-tools --bin import-foods -- \
  off-delta --start 1787480400 --end 1787566800 off-finland-delta.jsonl.gz
```

The importer reads gzip files and also accepts `-` for standard input. Full and
delta merges use staged batches and leave the live catalog unchanged if parsing
or validation fails.

## Testing

With or without Nix:

```sh
make check
make e2e
```

`make e2e` requires the local Postgres server and `DATABASE_URL`
