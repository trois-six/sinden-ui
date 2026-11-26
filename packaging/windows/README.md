# Windows installer

`sinden-driver.iss` is an [Inno Setup 6](https://jrsoftware.org/isinfo.php)
script producing `sinden-driver-<version>-setup-x86_64.exe`. It:

1. installs `sinden-driver.exe` into `C:\Program Files\SindenDriver`, and
   the default `%ProgramData%\SindenDriver\LightgunMono.exe.config` when
   none exists (kept on upgrade and uninstall);
2. runs `sinden-driver service install` then `service start` (the Windows
   service `SindenDriver`, roadmap § 3.2), and reports a failure; an
   upgrade stops the service first;
3. offers the tray application as an optional component (type "Driver
   only" leaves it out), with a task "Start the tray application at
   logon", applied as the installing user through the UI's own
   `--autostart=on|off` (the login item is per user);
4. uninstalls cleanly: login item off, UI closed, `service stop`,
   `service uninstall`. The configuration stays.

The wizard is in English and French.

## Why Inno Setup rather than WiX v4

* The driver registers itself (`sinden-driver service install`). In an MSI
  that means a deferred custom action running our executable, which WiX
  discourages and makes fragile (rollback, repair); in Inno Setup it is a
  plain, checked `Exec`. WiX's native `ServiceInstall` would duplicate what
  the driver does and drift from it.
* An optional component plus a checkbox task is the standard Inno Setup
  wizard; WiX needs a custom UI (or the feature tree dialog) for the same.
* One readable script, a single small compiler (`choco install innosetup`),
  no .NET toolchain; signing is the `SignTool` directive.
* MSI's advantage, deployment by group policy, does not matter for a
  light-gun driver installed by its owner.

## Building

On Windows, after `scripts/fetch-driver --target x86_64-pc-windows-msvc`
(or `--stub`) and, for the UI, `cd ui && npm ci && npm run tauri build --
--no-bundle`:

```powershell
packaging\windows\build-installer.ps1 -UiExe target\release\sinden-driver-ui.exe
```

The installer lands in `dist\packages\`.

## Signing

Optional. When `SIGNTOOL_CMD` is set (CI sets it when the
`WINDOWS_CERTIFICATE` secret exists), the installer and the uninstaller are
signed; CI also signs `sinden-driver.exe` and `sinden-driver-ui.exe` before
packaging them. Unsigned installers work but SmartScreen warns about them.

## To check on Windows

* the service runs as LocalSystem in session 0: whether it can capture the
  camera there is the roadmap's open point (§ 3.2); if it cannot, the
  driver's `service install` registers a logon task instead and this
  installer needs no change;
* the UI needs the WebView2 runtime, present on Windows 10 (21H2 and
  later) and 11.
