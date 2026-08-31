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

## Testing

With or without Nix:

```sh
make check
make e2e
```

`make e2e` requires the local Postgres server and `DATABASE_URL`
