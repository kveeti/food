# Food

A food diary with a Rust backend and React frontend. Track meals, water,
nutrient goals, and weight.

## Local development

Nix provides Rust, Node.js, pnpm, PostgreSQL, and Playwright. PostgreSQL data
lives in `.pg/`.

```sh
nix develop
make devi
```

Open <http://127.0.0.1:3000>. `make devi` starts the app and a local sign-in
provider. Use `make dev` to run the app with a real OIDC provider instead.

Useful commands:

- `make check` runs Rust and frontend checks
- `make e2e` runs browser tests
- `make build` builds the backend and frontend
- `make fmt` formats the code

Use `fin` to stop PostgreSQL and leave the Nix shell.

Without Nix, install Rust with `rustfmt` and `clippy`, Node.js 24, pnpm 10,
PostgreSQL, and Chromium for Playwright. Set `DATABASE_URL`, then run the
commands above. Install the browser with `pnpm exec playwright install chromium`.

## Browser tests

- Run all: `make e2e` from the repo root
- Run one file: `pnpm run e2e food.test.ts` from `frontend`
- Test the built frontend: `pnpm run e2e:preview` from `frontend`

Tests in `prerender.test.ts` only run in preview mode.

## Food catalogs

The app uses Fineli and Open Food Facts. Dev starts without a food catalog.
Import one below if needed. Fineli is very fast to import, OFF is bigger and
takes longer.

Run imports from the repo root.

### Fineli

Download and unpack a Fineli open-data release. The importer expects the CSV
files in the unpacked directory, including `food.csv` and `component.csv`.

```sh
cargo run --manifest-path backend/Cargo.toml \
  --features import-tools --bin import-foods -- \
  fineli "path/to/fineli"
```

### Open Food Facts

Download the full Open Food Facts JSONL gzip export, then filter it to Finland:

```sh
scripts/extract-off-finland.sh \
  "path/to/openfoodfacts-products.jsonl.gz" \
  "path/to/openfoodfacts-finland.jsonl.gz"
```

Use the export's `Last-Modified` HTTP date as the snapshot end time.

```sh
cargo run --manifest-path backend/Cargo.toml \
  --features import-tools --bin import-foods -- \
  off-full --snapshot-end 'LAST-MODIFIED DATE' \
  "path/to/openfoodfacts-finland.jsonl.gz"
```

Download OFF deltas manually, filter each to Finland with the script above,
then import them in order:

```sh
cargo run --manifest-path backend/Cargo.toml \
  --features import-tools --bin import-foods -- \
  off-delta --start START --end END \
  "path/to/openfoodfacts-finland-delta.jsonl.gz"
```

Use the delta's Unix start and end times. The importer checks for gaps and
overlap.
