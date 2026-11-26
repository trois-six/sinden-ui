# NixOS module: services.sinden-driver.
#
# The systemd unit mirrors packaging/linux/files/sinden-driver.service and
# the udev rule is read from packaging/linux/files: both are to be
# regenerated from `sinden-driver service print-files`.
{ self }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.services.sinden-driver;
  inherit (pkgs.stdenv.hostPlatform) system;
  configDir = "/etc/sinden-driver";
  configFile = "${configDir}/LightgunMono.exe.config";

  settingLine =
    key: value: ''<add key="${lib.escapeXML key}" value="${lib.escapeXML (toString value)}" />'';
  initialConfig = pkgs.writeText "LightgunMono.exe.config" ''
    <?xml version="1.0" encoding="utf-8"?>
    <!-- Written by the NixOS module services.sinden-driver. -->
    <configuration>
      <appSettings>
    ${lib.concatStringsSep "\n" (lib.mapAttrsToList settingLine cfg.settings)}
      </appSettings>
    </configuration>
  '';

  udevRules = pkgs.writeTextFile {
    name = "sinden-driver-udev-rules";
    text = builtins.readFile ../linux/files/60-sinden-driver.rules;
    destination = "/etc/udev/rules.d/60-sinden-driver.rules";
  };
in
{
  options.services.sinden-driver = {
    enable = lib.mkEnableOption "the Sinden light gun driver service";

    package = lib.mkOption {
      type = lib.types.package;
      default =
        self.packages.${system}.sinden-driver
          or (throw "services.sinden-driver: driver.lock has no binary for ${system}; set services.sinden-driver.package");
      defaultText = lib.literalExpression "sinden-ui.packages.\${system}.sinden-driver";
      description = "The sinden-driver package.";
    };

    settings = lib.mkOption {
      type = lib.types.attrsOf (
        lib.types.oneOf [
          lib.types.str
          lib.types.int
        ]
      );
      default = { };
      example = {
        Joystick = 1;
        LeftHanded = 1;
        ResourceMode = "low";
      };
      description = ''
        Keys of `${configFile}` (see the driver's configuration reference).
        They are written when the file does not exist yet, so that the tray
        application can edit it afterwards; with
        {option}`services.sinden-driver.overwriteConfig` they replace it at
        every start.
      '';
    };

    overwriteConfig = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "Rewrite the configuration file from `settings` at every start (edits made through the control API are lost).";
    };
  };

  config = lib.mkIf cfg.enable {
    users.users.sinden = {
      isSystemUser = true;
      group = "sinden";
      description = "Sinden light gun driver";
    };
    users.groups.sinden = { };

    services.udev.packages = [ udevRules ];
    environment.systemPackages = [ cfg.package ];

    # The driver rewrites its configuration (set_config): a real directory,
    # not a read-only environment.etc link.
    systemd.tmpfiles.settings."10-sinden-driver".${configDir}.d = {
      user = "root";
      group = "sinden";
      mode = "0775";
    };

    systemd.services.sinden-driver = {
      description = "Sinden light gun driver";
      documentation = [ "https://github.com/trois-six/sinden-ui" ];
      wantedBy = [ "multi-user.target" ];
      after = [ "systemd-udevd.service" ];
      restartTriggers = lib.optional cfg.overwriteConfig initialConfig;

      preStart = ''
        if ${lib.boolToString cfg.overwriteConfig} || [ ! -e ${configFile} ]; then
          install -m 0664 ${initialConfig} ${configFile}
        fi
      '';

      serviceConfig = {
        Type = "simple";
        User = "sinden";
        Group = "sinden";
        # /run/sinden-driver/control.sock (mode 0666).
        RuntimeDirectory = "sinden-driver";
        RuntimeDirectoryMode = "0755";
        ExecStart = "${lib.getExe cfg.package} service run";
        Restart = "on-failure";
        RestartSec = 2;

        NoNewPrivileges = true;
        ProtectSystem = "strict";
        ReadWritePaths = [ configDir ];
        ProtectHome = true;
        PrivateTmp = true;
        ProtectKernelModules = true;
        ProtectKernelTunables = true;
        ProtectControlGroups = true;
        RestrictSUIDSGID = true;
        LockPersonality = true;
      };
    };
  };
}
