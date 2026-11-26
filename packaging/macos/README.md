# macOS installer package

`build-pkg.sh` builds `sinden-driver-<version>-macos-universal.pkg` with
`pkgbuild` and `productbuild` (on macOS; Xcode command line tools):

```sh
scripts/fetch-driver --target universal-apple-darwin      # or --stub
(cd ui && npm ci && npm run tauri build -- --target universal-apple-darwin --bundles app)
packaging/macos/build-pkg.sh --ui "target/universal-apple-darwin/release/bundle/macos/Sinden Driver.app"
```

## What it installs

| Choice | Contents |
|---|---|
| Driver (required) | `/Library/Application Support/SindenDriver/SindenDriver.app` (the driver binary in a minimal bundle), `/Library/LaunchAgents/com.sinden-driver.agent.plist`, the link `/usr/local/bin/sinden-driver`, `uninstall.sh` |
| Tray application (optional, selected by default) | `/Applications/Sinden Driver.app` |

* The LaunchAgent `com.sinden-driver.agent` (roadmap § 3.2) runs
  `sinden-driver service run` as the logged-in user: a LaunchDaemon cannot
  get the camera permission. In `/Library/LaunchAgents` it starts for
  every user at login; the postinstall script starts it at once for the
  user logged in.
* The driver is wrapped in `SindenDriver.app` with an `Info.plist`
  (`NSCameraUsageDescription`, `LSUIElement`): macOS then asks for the
  camera under the name "Sinden Driver" and remembers the answer for the
  bundle. A bare binary started by launchd has no usage string to show.
* The tray application's postinstall enables its login item for the
  logged-in user (`--autostart=on`) and starts it in the menu bar.
* Configuration: `~/Library/Application Support/SindenDriver/LightgunMono.exe.config`
  per user (created by the driver; a default copy sits next to the bundle).
* Uninstall: `sudo "/Library/Application Support/SindenDriver/uninstall.sh"`.

The driver's own `sinden-driver service install` registers a per-user
agent with the same label; with this package it is not needed.

## Signing and notarisation

Optional: without the variables below the package is unsigned (fine for
testing; Gatekeeper blocks it on other Macs unless opened with
right-click > Open).

| Variable | Use |
|---|---|
| `MACOS_APP_IDENTITY` | `Developer ID Application: …`: `codesign` of `SindenDriver.app` (hardened runtime, entitlement `com.apple.security.device.camera`) and of the UI app |
| `MACOS_INSTALLER_IDENTITY` | `Developer ID Installer: …`: `productbuild --sign` |
| `MACOS_NOTARY_APPLE_ID`, `MACOS_NOTARY_TEAM_ID`, `MACOS_NOTARY_PASSWORD` | `xcrun notarytool submit --wait`, then `stapler staple` |

CI imports the certificates from the `MACOS_CERTIFICATE_P12` and
`MACOS_CERTIFICATE_PASSWORD` secrets into a temporary keychain and sets
these variables only when the secrets exist. Both identities come from a
paid Apple Developer account (Developer ID certificates).

## To check on a Mac

* the camera permission prompt for the agent (TCC) and that it survives
  upgrades (it does when the bundle identifier and signature stay the
  same);
* AVFoundation capture from a LaunchAgent in the Aqua session.
