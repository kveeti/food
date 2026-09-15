{
  description = "Food";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs, ... }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
      mkPackages =
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};

          backend = pkgs.rustPlatform.buildRustPackage {
            pname = "food-backend";
            version = "0.1.0";
            src = ./backend;
            cargoLock.lockFile = ./backend/Cargo.lock;
            cargoBuildFlags = [
              "--bin"
              "food-backend"
            ];
          };

          frontend = pkgs.stdenvNoCC.mkDerivation (finalAttrs: {
            pname = "food-frontend";
            version = "0.1.0";
            src = ./frontend;

            pnpmDeps = pkgs.fetchPnpmDeps {
              inherit (finalAttrs) pname version src;
              pnpm = pkgs.pnpm_10;
              fetcherVersion = 4;
              hash = "sha256-Tue4GNlm7/1Ekf8dnpkWZqDeOGXXeHt3IfazmPeyn9Q=";
            };

            nativeBuildInputs = [
              pkgs.nodejs_24
              pkgs.pnpm_10
              pkgs.pnpmConfigHook
            ];

            buildPhase = ''
              runHook preBuild
              pnpm run build
              runHook postBuild
            '';

            installPhase = ''
              runHook preInstall
              mkdir -p "$out"
              cp -r dist/. "$out/"
              runHook postInstall
            '';
          });
        in
        {
          inherit backend frontend;
          default = backend;
        };
    in
    {
      packages = nixpkgs.lib.genAttrs systems mkPackages;
      nixosModules.nginx = import ./nix/nginx.nix { inherit self; };

      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          packages = [
            pkgs.playwright-driver.browsers
            pkgs.postgresql_18
            pkgs.nodejs_24
            pkgs.pnpm_10
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
