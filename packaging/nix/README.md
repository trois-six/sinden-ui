# Nix

`flake.nix` (at the repository root) exposes:

| Output | What |
|---|---|
| `packages.<system>.sinden-driver` (and `default`) | the driver binary pinned in `driver.lock`, fetched with `fetchurl` and its SHA-256; static, nothing patched. On Linux, `lib/` holds the systemd unit, udev rule, sysusers.d and tmpfiles.d entries generated at build time by `sinden-driver service files` (hand-written fallbacks for cross builds). Only for the systems the lock has a binary for: none while it is the placeholder |
| `nixosModules.default` (`sinden-driver`) | `services.sinden-driver`: the package's unit (enabled), udev rule and tmpfiles.d entries, user `sinden` in group `input`, configuration |
| `checks.<system>.package` | builds `package.nix` from a stand-in binary implementing `service files` |
| `checks.<system>.module`, `fallback` | evaluate the module with a stand-in package carrying the fallback files |

The driver binary is MIT-licensed but distributed without its source
(`sourceProvenance = binaryNativeCode`): no `allowUnfree` is needed.

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
