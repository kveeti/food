# Food

A personal nutrition and activity diary. See [`docs/`](docs/) for the product rules.

## Develop

```sh
nix develop
pnpm install
make dev
```

The dev app uses a mock OIDC provider at `/dev/oidc`. It is not available in production.

Run checks with:

```sh
make typecheck
make e2e
make fmt-check
```

Database changes live in `src/lib/data/schema.ts`. Generate a migration after changing it:

```sh
make db-generate
```

The app applies pending migrations before its first database query.

## Run on Node

```sh
make build
APP_URL=https://food.example \
DATABASE_URL=postgres://food:...@db.example/food \
MIGRATIONS_PATH=./drizzle \
OIDC_ISSUER=https://id.example \
OIDC_CLIENT_ID=food \
OIDC_CLIENT_SECRET=... \
OIDC_REDIRECT_URL=https://food.example/auth/callback \
SESSION_SECRET=... \
make start
```

`SESSION_SECRET` must contain at least 32 characters.
