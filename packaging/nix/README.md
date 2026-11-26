# Nix

`flake.nix` (at the repository root) exposes:

| Output | What |
|---|---|
| `packages.<system>.sinden-driver` (and `default`) | the driver binary pinned in `driver.lock`, fetched with `fetchurl` and its SHA-256; static, nothing patched. Only for the systems the lock has a binary for: none while it is the placeholder |
| `nixosModules.default` (`sinden-driver`) | `services.sinden-driver`: user `sinden`, systemd service, udev rule, configuration |
| `checks.<system>.module` | evaluates the module with a stand-in binary |

The driver binary's license is not free: the flake's own package set allows
`sinden-driver` (and only it) through `allowUnfreePredicate`. Using the
module's default `package` therefore needs no `allowUnfree` in your
configuration.

```nix
# flake.nix of a NixOS configuration
{
  inputs.sinden-ui.url = "github:trois-six/sinden-ui";
  outputs = { nixpkgs, sinden-ui, ... }: {
    nixosConfigurations.arcade = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        sinden-ui.nixosModules.default
        {
          services.sinden-driver = {
            enable = true;
            settings = { Joystick = 1; LeftHanded = 1; };
          };
        }
      ];
    };
  };
}
```

`settings` seeds `/etc/sinden-driver/LightgunMono.exe.config` when it does
not exist, so that the tray application can edit it later;
`overwriteConfig = true` rewrites it from `settings` at every start
instead.

The tray application is not packaged for Nix yet.

Checks: `nix flake check` (CI runs it).
