{
  description = "sinden-driver, the Sinden light gun driver service: package and NixOS module";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";

  outputs =
    { self, nixpkgs }:
    let
      inherit (nixpkgs) lib;
      lock = lib.importJSON ./driver.lock;
      targets = import ./packaging/nix/targets.nix;
      systems = builtins.attrNames targets;
      forAllSystems = lib.genAttrs systems;
      hasDriver = system: lock.assets ? ${targets.${system}};

      pkgsFor = system: import nixpkgs { inherit system; };
    in
    {
      # Only the systems driver.lock has a binary for (none while it is the
      # placeholder).
      packages = forAllSystems (
        system:
        lib.optionalAttrs (hasDriver system) rec {
          sinden-driver = (pkgsFor system).callPackage ./packaging/nix/package.nix { inherit lock; };
          default = sinden-driver;
        }
      );

      nixosModules = rec {
        sinden-driver = import ./packaging/nix/module.nix { inherit self; };
        default = sinden-driver;
      };

      # The module evaluated with a stand-in binary: runs without a driver
      # release.
      checks = lib.genAttrs [ "x86_64-linux" "aarch64-linux" ] (
        system: import ./packaging/nix/checks.nix { inherit self nixpkgs system; }
      );

      formatter = forAllSystems (system: nixpkgs.legacyPackages.${system}.nixfmt);
    };
}
