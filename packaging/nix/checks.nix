# Checks without a driver release:
#  * package: package.nix built from a stand-in binary that implements
#    `service files`, so its installPhase runs for real;
#  * module: the NixOS module evaluated with a stand-in package carrying the
#    fallback service files.
{
  self,
  nixpkgs,
  system,
}:
let
  pkgs = nixpkgs.legacyPackages.${system};
  inherit (nixpkgs) lib;

  fakeDriver = pkgs.writeScript "fake-sinden-driver" ''
    #!/bin/sh
    [ "$1 $2 $3 $5" = "service files --bin --dir" ] || exit 64
    mkdir -p "$6/usr/lib/systemd/system" "$6/usr/lib/udev/rules.d"
    printf '[Service]\nExecStart=%s service run\n' "$4" > "$6/usr/lib/systemd/system/sinden-driver.service"
    echo 'SUBSYSTEM=="tty", GROUP="sinden"' > "$6/usr/lib/udev/rules.d/70-sinden-driver.rules"
  '';
  fakeLock = {
    version = "0.0.0-check";
    tag = "driver-v0.0.0-check";
    assets.${(import ./targets.nix).${system}} = {
      file = "unused";
      sha256 = lib.fakeSha256;
    };
  };
  package = (pkgs.callPackage ./package.nix { lock = fakeLock; }).overrideAttrs {
    src = fakeDriver;
  };

  standIn = pkgs.runCommand "sinden-driver-stand-in" { } ''
    mkdir -p "$out/bin"
    printf '#!/bin/sh\nexit 0\n' > "$out/bin/sinden-driver"
    chmod +x "$out/bin/sinden-driver"
    ${import ./fallback-service-files.nix}
  '';
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
  expect = name: cond: if cond then null else throw "sinden-driver module check failed: ${name}";
  results = [
    (expect "wantedBy" (builtins.elem "multi-user.target" service.wantedBy))
    (expect "seeding runs as root" (
      lib.hasPrefix "+" (builtins.head service.serviceConfig.ExecStartPre)
    ))
    (expect "unit from the package" (builtins.elem standIn config.systemd.packages))
    (expect "udev rule from the package" (builtins.elem standIn config.services.udev.packages))
    (expect "tmpfiles from the package" (builtins.elem standIn config.systemd.tmpfiles.packages))
    (expect "system user" config.users.users.sinden.isSystemUser)
    (expect "group input" (builtins.elem "input" config.users.users.sinden.extraGroups))
    (expect "group sinden" (config.users.groups ? sinden))
  ];
in
{
  module = builtins.deepSeq results (pkgs.writeText "sinden-driver-module-check" "ok\n");

  package = pkgs.runCommand "sinden-driver-package-check" { } ''
    unit=${package}/lib/systemd/system/sinden-driver.service
    grep -qx "ExecStart=${package}/bin/sinden-driver service run" "$unit"
    test -f ${package}/lib/udev/rules.d/70-sinden-driver.rules
    test -x ${package}/bin/sinden-driver
    touch "$out"
  '';

  fallback = pkgs.runCommand "sinden-driver-fallback-check" { } ''
    grep -q "ExecStart=${standIn}/bin/sinden-driver service run" ${standIn}/lib/systemd/system/sinden-driver.service
    grep -q "m sinden input" ${standIn}/lib/sysusers.d/sinden-driver.conf
    touch "$out"
  '';
}
