{
  description = "Food";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { nixpkgs, ... }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          packages = [
            pkgs.playwright-driver.browsers
            pkgs.postgresql_18
            pkgs.deno
            pkgs.rustc
            pkgs.cargo
            pkgs.rustfmt
            pkgs.clippy
            pkgs.otel-tui
            pkgs.rust-analyzer
            pkgs.topcoat-cli
          ];
          RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";

          shellHook = ''
            export PLAYWRIGHT_BROWSERS_PATH="${pkgs.playwright-driver.browsers}"
            for chromium in "${pkgs.playwright-driver.browsers}"/chromium-*/chrome-linux64/chrome; do
              if [ -x "$chromium" ]; then
                export PLAYWRIGHT_CHROMIUM_EXECUTABLE="$chromium"
                break
              fi
            done

            free_port() {
              local port="$1"
              while (echo >/dev/tcp/127.0.0.1/"$port") 2>/dev/null; do
                port=$((port + 1))
              done
              echo "$port"
            }

            export PGDATA="$PWD/.pg"
            mkdir -p "$PGDATA"
            chmod 700 "$PGDATA"

            if [ ! -f "$PGDATA/PG_VERSION" ]; then
              echo "Initializing PostgreSQL..."
              initdb -D "$PGDATA" -U postgres >/dev/null || exit 1
            fi

            if ! pg_ctl -D "$PGDATA" status >/dev/null 2>&1; then
              port="$(free_port "''${PGPORT:-5556}")"
              echo "Starting PostgreSQL on port $port..."
              pg_ctl -D "$PGDATA" -o "-p $port" -l "$PGDATA/server.log" -w start >/dev/null || exit 1
            fi

            unset -f free_port
            export PGPORT="$(awk 'NR == 4 { print; exit }' "$PGDATA/postmaster.pid")"
            export PGHOST=127.0.0.1
            export PGUSER=postgres
            export DATABASE_URL="postgres://postgres@127.0.0.1:$PGPORT/postgres"

            echo "PostgreSQL: $DATABASE_URL"
            alias pg='psql'
            alias fin='pg_ctl -D "$PGDATA" stop && exit'
          '';
        };
      });
    };
}
