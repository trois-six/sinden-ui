# sinden-ui

Packaging and an optional tray application for `sinden-driver`, a driver
service for the Sinden light gun on Linux, Windows and macOS.

This repository builds two packages:

| Package | What it contains | Required |
|---|---|---|
| `sinden-driver` | the driver binary, registered as a system service (systemd unit, Windows service, macOS LaunchAgent), its udev rule and default configuration | yes |
| `sinden-driver-ui` | a tray application that shows the guns' state and edits every configuration key through the driver's control API | no |

The driver runs on its own: the UI is a convenience. Without it, the
configuration is the file `LightgunMono.exe.config` (Sinden's format):

| System | Configuration file |
|---|---|
| Linux | `/etc/sinden-driver/LightgunMono.exe.config` |
| Windows | `%ProgramData%\SindenDriver\LightgunMono.exe.config` |
| macOS | `~/Library/Application Support/SindenDriver/LightgunMono.exe.config` |

## Where the driver comes from

The driver's source is not in this repository and no binary is ever
committed here. [`driver.lock`](driver.lock) pins one driver release:

```json
{
  "version": "X.Y.Z",
  "tag": "driver-vX.Y.Z",
  "assets": {
    "x86_64-unknown-linux-musl": { "file": "sinden-driver-X.Y.Z-x86_64-unknown-linux-musl", "sha256": "..." }
  }
}
```

The driver's release workflow publishes the binaries as the assets of the
release `driver-vX.Y.Z` of this repository and commits the new
`driver.lock`. [`scripts/fetch-driver`](scripts/fetch-driver) downloads
them from
`https://github.com/trois-six/sinden-ui/releases/download/<tag>/<file>`
and checks their SHA-256; a mismatch stops the build.

The driver binaries are MIT-licensed too, but their source code is not
published: see [NOTICE](NOTICE).

## Layout

| Path | Contents |
|---|---|
| `driver.lock` | the pinned driver release |
| `scripts/` | `fetch-driver`, the package build scripts, `make-icons` |
| `assets/` | the application icon (SVG sources) |
| `packaging/common/` | the default configuration file |
| `packaging/linux/` | nfpm configs (deb, rpm), maintainer scripts, fallback service files, desktop entry, icons |
| `packaging/aur/` | Arch Linux PKGBUILDs |
| `packaging/nix/` | Nix package and NixOS module (exposed by `flake.nix`) |
| `packaging/windows/` | Inno Setup installer |
| `packaging/macos/` | installer package (`pkgbuild`/`productbuild`) |
| `crates/sinden-control/` | Rust client of the driver's control API (+ `sinden-ctl` example) |
| `ui/` | the Tauri v2 tray application |

## Building

Every build starts by fetching the driver:

```sh
scripts/fetch-driver                       # every target of driver.lock
scripts/fetch-driver --target x86_64-unknown-linux-musl
scripts/fetch-driver --stub                # placeholder binaries, see below
```

The binaries land in `dist/driver/<target>/sinden-driver[.exe]`. The
packages use the generic targets; extra entries of the lock (such as the
MiSTer-tuned `armv7-unknown-linux-musleabihf-cortex-a9`) are ignored by
the packaging.

### The UI

```sh
cd ui
npm ci
npm run tauri build -- --no-bundle         # target/release/sinden-driver-ui
```

Linux needs the WebKitGTK development files (Debian/Ubuntu:
`libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev`).

### Linux: deb and rpm

```sh
scripts/build-linux-packages               # sinden-driver for every Linux target
scripts/build-linux-packages --ui target/release/sinden-driver-ui
```

Packages land in `dist/packages/`. The systemd unit, udev rule,
sysusers.d and tmpfiles.d entries come from the driver itself
(`sinden-driver service files`, run with the build host's binary); stub
builds use the fallbacks of `packaging/linux/files/`. The script uses
[nfpm](https://nfpm.goreleaser.com/); if it is not on the `PATH` it is
installed with `go install` into `.tools/` (pinned version).

### Arch Linux (AUR)

```sh
scripts/build-aur                          # dist/aur/<package>/ ready to push to the AUR
```

See [packaging/aur/README.md](packaging/aur/README.md).

### Nix

```sh
nix build .#sinden-driver
```

and, in a NixOS configuration:

```nix
{
  inputs.sinden-ui.url = "github:trois-six/sinden-ui";
  # ...
  modules = [
    sinden-ui.nixosModules.default
    { services.sinden-driver.enable = true; }
  ];
}
```

See [packaging/nix/README.md](packaging/nix/README.md).

### Windows

[Inno Setup 6](https://jrsoftware.org/isinfo.php), on Windows:

```powershell
iscc /DDriverVersion=X.Y.Z /DUiVersion=A.B.C /DUiExe=..\..\target\release\sinden-driver-ui.exe packaging\windows\sinden-driver.iss
```

See [packaging/windows/README.md](packaging/windows/README.md).

### macOS

```sh
packaging/macos/build-pkg.sh --ui "target/universal-apple-darwin/release/bundle/macos/Sinden Driver.app"
```

See [packaging/macos/README.md](packaging/macos/README.md).

### Continuous integration

[`.github/workflows/build.yml`](.github/workflows/build.yml) builds every
package on each push and pull request (Linux on Ubuntu, the installer on
Windows, the pkg on macOS). A tag `v*` attaches them to a GitHub release.
Signing runs only when its secrets are set, so forks build unsigned
packages without any secret.

## Contributing without the driver's source

You do not need the driver's source: `scripts/fetch-driver` downloads the
published binaries. Before the first driver release (or offline),
`scripts/fetch-driver --stub` writes placeholder binaries so that every
package can still be built and inspected; CI does the same while
`driver.lock` is the placeholder (`"version": "0.0.0"`), and refuses to
release stub packages. The UI can be developed against a fake control
server: see [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT for this repository, see [LICENSE](LICENSE). The `sinden-driver`
binaries are MIT too, distributed without their source code, see
[NOTICE](NOTICE).
