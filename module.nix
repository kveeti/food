{ foodPkg }:
{ config, lib, pkgs, ... }:

let
  cfg = config.services.food;
  dataDir = "/var/lib/food";
  defaultEnvironment = {
    DATABASE_URL = "sqlite://food.db?mode=rwc";
    HOST = "127.0.0.1:8000";
  };
in
{
  options.services.food = {
    enable = lib.mkEnableOption "Food reminder service";

    environment = lib.mkOption {
      type = lib.types.attrsOf lib.types.str;
      default = {};
      description = "Environment variables to pass to the service";
    };

    environmentFile = lib.mkOption {
      type = lib.types.nullOr lib.types.path;
      default = null;
      description = "File containing environment variables to pass to the service";
    };
  };

  config = lib.mkIf cfg.enable {
    users.groups.food = { };
    users.users.food = {
      isSystemUser = true;
      group = "food";
    };

    systemd.services.food = {
      description = "Food reminder service";
      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" ];
      environment = defaultEnvironment // cfg.environment;

      serviceConfig = {
        ExecStart = "${foodPkg}/bin/food";
        Restart = "always";
        User = "food";
        Group = "food";

        StateDirectory = "food";
        WorkingDirectory = dataDir;

        EnvironmentFile = lib.mkIf (cfg.environmentFile != null) cfg.environmentFile;

        # Hardening
        CapabilityBoundingSet = [ "" ];
        DeviceAllow = [ "/dev/stdin" "/dev/urandom" ];
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
        RestrictAddressFamilies = [ "AF_INET" "AF_INET6" "AF_UNIX" ];
        RestrictNamespaces = true;
        RestrictRealtime = true;
        RestrictSUIDSGID = true;
        SystemCallArchitectures = "native";
        SystemCallFilter = [ "@system-service" "~@privileged" "~@resources" ];
        UMask = "0027";
      };
    };
  };
}
