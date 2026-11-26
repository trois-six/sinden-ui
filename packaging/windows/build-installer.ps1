# Builds the Windows installer with Inno Setup 6 (ISCC.exe).
#
#   packaging\windows\build-installer.ps1 [-UiExe target\release\sinden-driver-ui.exe]
#
# Run from anywhere; reads the versions from driver.lock and ui\package.json.
# Signing: set SIGNTOOL_CMD to a full sign command ending with $f (Inno
# Setup's placeholder for the file), e.g.
#   signtool.exe sign /f cert.pfx /p **** /fd sha256 /tr http://timestamp.digicert.com /td sha256 $f
param(
    [string]$UiExe = "",
    [string]$Iscc = ""
)
$ErrorActionPreference = "Stop"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path

if (-not $Iscc) {
    $found = Get-Command iscc.exe -ErrorAction SilentlyContinue
    if ($found) { $Iscc = $found.Source }
    else { $Iscc = Join-Path ${env:ProgramFiles(x86)} "Inno Setup 6\ISCC.exe" }
}
if (-not (Test-Path $Iscc)) { throw "ISCC.exe not found: install Inno Setup 6 (choco install innosetup)" }

$lock = Get-Content (Join-Path $root "driver.lock") -Raw | ConvertFrom-Json
$driverVersion = $lock.version
if (Test-Path (Join-Path $root "dist\driver\STUB")) { $driverVersion = "$driverVersion-stub" }

$defines = @("/DDriverVersion=$driverVersion")
if ($UiExe) {
    $ui = Get-Content (Join-Path $root "ui\package.json") -Raw | ConvertFrom-Json
    $defines += "/DUiExe=$((Resolve-Path $UiExe).Path)"
    $defines += "/DUiVersion=$($ui.version)"
}
if ($env:SIGNTOOL_CMD) {
    $defines += "/DSign"
    $defines += "/Ssigntool=$($env:SIGNTOOL_CMD)"
}

& $Iscc @defines (Join-Path $PSScriptRoot "sinden-driver.iss")
if ($LASTEXITCODE -ne 0) { throw "ISCC failed ($LASTEXITCODE)" }
