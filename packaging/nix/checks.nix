# Evaluates the NixOS module with a stand-in driver and checks the service,
# the user, the udev rule and the initial configuration. Pure evaluation:
# no driver release and no VM needed.
{
  self,
  nixpkgs,
  system,
}:
let
  pkgs = nixpkgs.legacyPackages.${system};
  inherit (nixpkgs) lib;
  standIn = pkgs.writeShellScriptBin "sinden-driver" "exit 0";
  eval = lib.nixosSystem {
    inherit system;
    modules = [
      self.nixosModules.default
      {
        boot.isContainer = true;
        system.stateVersion = "26.05";
        services.sinden-driver = {
          enable = true;
          package = standIn;
          settings = {
            Joystick = 1;
            LeftHanded = "1";
            Odd = "a<b&\"c\"";
          };
        };
      }
    ];
  };
  inherit (eval) config;
  service = config.systemd.services.sinden-driver;
  udevText = builtins.readFile ../linux/files/60-sinden-driver.rules;
  expect = name: cond: if cond then null else throw "sinden-driver module check failed: ${name}";
  results = [
    (expect "ExecStart" (
      lib.hasSuffix "/bin/sinden-driver service run" service.serviceConfig.ExecStart
    ))
    (expect "User" (service.serviceConfig.User == "sinden"))
    (expect "RuntimeDirectory" (service.serviceConfig.RuntimeDirectory == "sinden-driver"))
    (expect "wantedBy" (builtins.elem "multi-user.target" service.wantedBy))
    (expect "system user" config.users.users.sinden.isSystemUser)
    (expect "group" (config.users.groups ? sinden))
    (expect "udev rule" (lib.hasInfix ''GROUP="sinden"'' udevText))
    (expect "udev package" (builtins.length config.services.udev.packages >= 1))
    (expect "config dir" (
      config.systemd.tmpfiles.settings."10-sinden-driver"."/etc/sinden-driver".d.group == "sinden"
    ))
    (expect "settings seeded" (lib.hasInfix "LightgunMono.exe.config" service.preStart))
  ];
in
{
  module = builtins.deepSeq results (pkgs.writeText "sinden-driver-module-check" "ok\n");
}
