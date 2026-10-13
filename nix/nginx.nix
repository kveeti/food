{ self }:
{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.services.food;
  packages = self.packages.${pkgs.stdenv.hostPlatform.system};
  backendUrl = "http://127.0.0.1:${toString cfg.port}";
  proxy = {
    proxyPass = backendUrl;
    recommendedProxySettings = true;
  };
in
{
  options.services.food = {
    enable = lib.mkEnableOption "Food";

    environment = lib.mkOption {
      type = lib.types.attrsOf lib.types.str;
      default = { };
      description = "Environment variables passed to Food. Do not use this for secrets.";
    };

    environmentFile = lib.mkOption {
      type = lib.types.nullOr lib.types.path;
      default = null;
      description = "Systemd environment file containing Food's config and secrets.";
      example = "/run/secrets/food-env";
    };

    port = lib.mkOption {
      type = lib.types.port;
      default = 8000;
      description = "Local port used by the Food backend.";
    };

    nginx = {
      enable = lib.mkEnableOption "the Food nginx virtual host";

      virtualHost = lib.mkOption {
        type = lib.types.str;
        description = "Name of the nginx virtual host used by Food.";
        example = "food.example.com";
      };
    };
  };

  config = lib.mkIf cfg.enable (
    lib.mkMerge [
      {
        systemd.services.food = {
          description = "Food backend";
          wantedBy = [ "multi-user.target" ];
          wants = [ "network-online.target" ];
          after = [ "network-online.target" ];

          environment = cfg.environment // {
            HOST = "127.0.0.1";
            PORT = toString cfg.port;
          };

          serviceConfig = {
            Type = "exec";
            ExecStart = "${packages.backend}/bin/food-backend";
            EnvironmentFile = lib.mkIf (cfg.environmentFile != null) cfg.environmentFile;
            DynamicUser = true;
            Restart = "always";
            RestartSec = 5;
            TimeoutStopSec = 45;

            CapabilityBoundingSet = [ "" ];
            DeviceAllow = [ "" ];
            DevicePolicy = "strict";
            LockPersonality = true;
            MemoryDenyWriteExecute = true;
            NoNewPrivileges = true;
            PrivateDevices = true;
            PrivateTmp = true;
            PrivateUsers = true;
            ProcSubset = "pid";
            ProtectClock = true;
            ProtectControlGroups = true;
            ProtectHome = true;
            ProtectHostname = true;
            ProtectKernelLogs = true;
            ProtectKernelModules = true;
            ProtectKernelTunables = true;
            ProtectProc = "invisible";
            ProtectSystem = "strict";
            RemoveIPC = true;
            RestrictAddressFamilies = [
              "AF_INET"
              "AF_INET6"
              "AF_UNIX"
            ];
            RestrictNamespaces = true;
            RestrictRealtime = true;
            RestrictSUIDSGID = true;
            SystemCallArchitectures = "native";
            SystemCallFilter = [
              "@system-service"
              "~@privileged"
              "~@resources"
            ];
            UMask = "0077";
          };
        };
      }

      (lib.mkIf cfg.nginx.enable {
        services.nginx = {
          enable = true;

          virtualHosts.${cfg.nginx.virtualHost} = {
            root = packages.frontend;

            locations = {
              "/api/" = proxy;
              "/auth/" = proxy;
              "= /logout" = proxy;
              "= /sign-in".tryFiles = "/sign-in/index.html =404";

              "/assets/".extraConfig = ''
                expires 1y;
                add_header Cache-Control "public, immutable";
              '';

              "/".tryFiles = "$uri $uri/ /index.html";
            };
          };
        };
      })
    ]
  );
}
