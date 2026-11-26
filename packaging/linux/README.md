# Linux packages

`scripts/build-linux-packages` builds, with [nfpm](https://nfpm.goreleaser.com/):

| Package | Formats | Architectures | Contents |
|---|---|---|---|
| `sinden-driver` | deb, rpm | amd64, arm64, armhf/armv7hl | `/usr/bin/sinden-driver`, the systemd unit, the udev rule, the sysusers.d entry, `/etc/sinden-driver/LightgunMono.exe.config` |
| `sinden-driver-ui` | deb, rpm | the build host's | `/usr/bin/sinden-driver-ui`, its `.desktop` file and hicolor icons |

## Files

| File | Installed as |
|---|---|
| `files/sinden-driver.service` | `/usr/lib/systemd/system/sinden-driver.service` |
| `files/60-sinden-driver.rules` | `/usr/lib/udev/rules.d/60-sinden-driver.rules` |
| `files/sinden-driver.sysusers` | `/usr/lib/sysusers.d/sinden-driver.conf` |
| `files/sinden-driver-ui.desktop` | `/usr/share/applications/sinden-driver-ui.desktop` |
| `icons/hicolor/` (from `scripts/make-icons`) | `/usr/share/icons/hicolor/` |
| `../common/LightgunMono.exe.config` | `/etc/sinden-driver/LightgunMono.exe.config` (conffile) |

The unit, the udev rule and the sysusers.d entry are written by hand for
now. Once the driver provides `sinden-driver service print-files`, they
are **regenerated** from its output (the Nix module reads the same files).

## Maintainer scripts

`scripts/postinstall.sh` creates the user `sinden` (sysusers.d, or
`useradd` without systemd), gives it the configuration directory, reloads
udev for the guns already plugged, then enables and (re)starts
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
