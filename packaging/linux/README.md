# Linux packages

`scripts/build-linux-packages` builds, with [nfpm](https://nfpm.goreleaser.com/):

| Package | Formats | Architectures | Contents |
|---|---|---|---|
| `sinden-driver` | deb, rpm | amd64, arm64, armhf/armv7hl | `/usr/bin/sinden-driver`, the systemd unit, the udev rule, the sysusers.d entry, `/etc/sinden-driver/LightgunMono.exe.config` |
| `sinden-driver-ui` | deb, rpm | the build host's | `/usr/bin/sinden-driver-ui`, its `.desktop` file and hicolor icons |

## Service files

The systemd unit, the udev rule, the sysusers.d and tmpfiles.d entries are
the driver's: `build-linux-packages` runs

```sh
dist/driver/<host target>/sinden-driver service files --bin /usr/bin/sinden-driver --dir dist/stage/service-files
```

with the binary of the build host's architecture (the files are the same
for every architecture, so the x86_64 binary serves the arm64 and armhf
packages too) and adds every file it writes to the package. The user
`sinden` is a member of group `input` (to clear the joystick axes' kernel
filter); the udev rule gives it the gun's serial port and camera.

`files/` holds hand-written **fallbacks** of those four files, used only
with stub binaries (`fetch-driver --stub`), when no binary runs on the
build host, or with `--fallback-service-files`.

| File | Installed as |
|---|---|
| (driver) or `files/sinden-driver.service` | `/usr/lib/systemd/system/sinden-driver.service` |
| (driver) or `files/60-sinden-driver.rules` | `/usr/lib/udev/rules.d/…` |
| (driver) or `files/sinden-driver.sysusers` | `/usr/lib/sysusers.d/…` |
| (driver) or `files/sinden-driver.tmpfiles` | `/usr/lib/tmpfiles.d/…` |
| `files/sinden-driver-ui.desktop` | `/usr/share/applications/sinden-driver-ui.desktop` |
| `icons/hicolor/` (from `scripts/make-icons`) | `/usr/share/icons/hicolor/` |
| `../common/LightgunMono.exe.config` | `/etc/sinden-driver/LightgunMono.exe.config` (conffile) |

`nfpm/sinden-driver.yaml` is a template: the script inserts the service
files at its `@SERVICE_FILES@` marker and writes `dist/stage/sinden-driver.yaml`.

## Maintainer scripts

`scripts/postinstall.sh` creates the user `sinden` (the package's
sysusers.d entries, or `useradd` and group `input` without systemd),
applies the tmpfiles.d entries and gives it the configuration directory,
reloads udev for the guns already plugged, then enables and (re)starts
`sinden-driver.service`. `preremove.sh` stops and disables it on removal
(not on upgrade); `postremove.sh` reloads systemd and udev, and a purge
removes `/etc/sinden-driver`. The user is kept.

## Checking a package

```sh
dpkg-deb -c dist/packages/sinden-driver_*_amd64.deb
docker run --rm -v "$PWD/dist/packages:/p:ro" debian:trixie \
  sh -c 'apt-get update -qq && apt-get install -y /p/sinden-driver_*_amd64.deb && getent passwd sinden'
```

rpm 6 (Fedora 45 and later) refuses unsigned packages by default: sign
them (CI does when the `RPM_SIGNING_KEY` secret is set) or install a local
build with `rpm -i --nosignature`.
