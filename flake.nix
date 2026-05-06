{
  description = "Food";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
        "x86_64-darwin"
      ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
    in
    {
      packages = forAllSystems (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};

          frontend = pkgs.stdenv.mkDerivation (finalAttrs: {
            pname = "food-frontend";
            version = "0.0.1";
            src = ./front;

            pnpmDeps = pkgs.fetchPnpmDeps {
              inherit (finalAttrs) pname version src;
              fetcherVersion = 3;
              hash = "sha256-JFkVDQ+aVcnTCpxnFN1eeGbDDB+dClRboHfESPf2sy8=";
            };

            nativeBuildInputs = with pkgs; [
              nodejs_24
              pnpm_10
              pnpmConfigHook
            ];

            buildPhase = ''
              runHook preBuild
              pnpm build
              runHook postBuild
            '';

            installPhase = ''
              runHook preInstall
              cp -r dist $out
              runHook postInstall
            '';
          });

          backend = pkgs.rustPlatform.buildRustPackage {
            pname = "food-backend";
            version = "0.0.1";
            src = ./back;

            cargoLock = {
              lockFile = ./back/Cargo.lock;
            };
          };
        in {
          inherit frontend backend;

          default = pkgs.runCommand "food" {
            nativeBuildInputs = [ pkgs.makeWrapper ];
          } ''
            mkdir -p $out/bin
            makeWrapper ${backend}/bin/back $out/bin/food \
              --set FRONTEND_DIR ${frontend}
          '';
        }
      );

      devShells = forAllSystems (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in {
          default = pkgs.mkShell {
            nativeBuildInputs = with pkgs; [
              nodejs_24
              pnpm_10
              rustup
              cargo-watch
              sqlite
            ];
          };
        }
      );

      nixosModules.default = { pkgs, ... }@args:
        let
          foodPkg = self.packages.${pkgs.system}.default;
        in
        import ./module.nix { inherit foodPkg; } args;
    };
}
