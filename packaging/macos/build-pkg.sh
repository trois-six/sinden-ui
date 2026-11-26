#!/bin/sh
# Builds the macOS installer package (pkgbuild + productbuild).
#
#   packaging/macos/build-pkg.sh [--driver BINARY] [--ui "Sinden Driver.app"] [--out DIR]
#
# Components:
#   com.sinden-driver.pkg.driver  the driver, wrapped in SindenDriver.app (so
#                                 that the camera permission has a name and a
#                                 usage string), the LaunchAgent, a
#                                 /usr/local/bin/sinden-driver link and an
#                                 uninstall script; always installed
#   com.sinden-driver.pkg.ui      the tray application in /Applications;
#                                 optional (a choice of the installer), only
#                                 when --ui is given
#
# Signing and notarisation run only when their variables are set:
#   MACOS_APP_IDENTITY        "Developer ID Application: ..." (codesign)
#   MACOS_INSTALLER_IDENTITY  "Developer ID Installer: ..." (productbuild)
#   MACOS_NOTARY_APPLE_ID, MACOS_NOTARY_TEAM_ID, MACOS_NOTARY_PASSWORD
#                             notarytool credentials (app-specific password)
set -eu

here=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
root=$(CDPATH='' cd -- "$here/../.." && pwd)

driver="$root/dist/driver/universal-apple-darwin/sinden-driver"
ui=""
out="$root/dist/packages"

while [ $# -gt 0 ]; do
	case "$1" in
	--driver) driver=$2; shift 2 ;;
	--ui) ui=$2; shift 2 ;;
	--out) out=$2; shift 2 ;;
	-h | --help) sed -n '2,22p' "$0"; exit 0 ;;
	*) echo "build-pkg: unknown argument $1" >&2; exit 2 ;;
	esac
done

json_version() { plutil -extract version raw -o - "$1" 2>/dev/null || python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["version"])' "$1"; }

[ -f "$driver" ] || { echo "build-pkg: $driver not found (scripts/fetch-driver --target universal-apple-darwin)" >&2; exit 1; }
version=$(json_version "$root/driver.lock")
[ -f "$root/dist/driver/STUB" ] && version="$version-stub"

work="$root/target/macos-pkg"
rm -rf "$work"
mkdir -p "$work/driver-root" "$work/ui-root" "$work/components" "$out"

sign() {
	# sign <path> [entitlements]
	[ -n "${MACOS_APP_IDENTITY:-}" ] || return 0
	if [ -n "${2:-}" ]; then
		codesign --force --timestamp --options runtime --entitlements "$2" --sign "$MACOS_APP_IDENTITY" "$1"
	else
		codesign --force --timestamp --options runtime --sign "$MACOS_APP_IDENTITY" "$1"
	fi
}

# --- Driver component ---------------------------------------------------------
support="$work/driver-root/Library/Application Support/SindenDriver"
bundle="$support/SindenDriver.app"
mkdir -p "$bundle/Contents/MacOS" "$work/driver-root/Library/LaunchAgents"
cp "$driver" "$bundle/Contents/MacOS/sinden-driver"
chmod 0755 "$bundle/Contents/MacOS/sinden-driver"
sed "s/@VERSION@/$version/g" "$here/driver-Info.plist" >"$bundle/Contents/Info.plist"
cp "$root/NOTICE" "$support/NOTICE.txt"
cp "$root/packaging/common/LightgunMono.exe.config" "$support/LightgunMono.exe.config.default"
cp "$here/uninstall.sh" "$support/uninstall.sh"
chmod 0755 "$support/uninstall.sh"
cp "$here/com.sinden-driver.agent.plist" "$work/driver-root/Library/LaunchAgents/"
sign "$bundle" "$here/driver.entitlements"

pkgbuild \
	--root "$work/driver-root" \
	--identifier com.sinden-driver.pkg.driver \
	--version "$version" \
	--scripts "$here/scripts/driver" \
	--install-location / \
	"$work/components/driver.pkg"

# --- UI component (optional) ---------------------------------------------------
ui_choice=""
ui_line=""
if [ -n "$ui" ]; then
	[ -d "$ui" ] || { echo "build-pkg: $ui is not an app bundle" >&2; exit 1; }
	ui_version=$(json_version "$root/ui/package.json")
	mkdir -p "$work/ui-root/Applications"
	cp -R "$ui" "$work/ui-root/Applications/Sinden Driver.app"
	sign "$work/ui-root/Applications/Sinden Driver.app"
	cat >"$work/ui-components.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<array>
  <dict>
    <key>RootRelativeBundlePath</key>
    <string>Applications/Sinden Driver.app</string>
    <key>BundleIsRelocatable</key>
    <false/>
    <key>BundleIsVersionChecked</key>
    <true/>
    <key>BundleHasStrictIdentifier</key>
    <true/>
    <key>BundleOverwriteAction</key>
    <string>upgrade</string>
  </dict>
</array>
</plist>
PLIST
	pkgbuild \
		--root "$work/ui-root" \
		--component-plist "$work/ui-components.plist" \
		--identifier com.sinden-driver.pkg.ui \
		--version "$ui_version" \
		--scripts "$here/scripts/ui" \
		--install-location / \
		"$work/components/ui.pkg"
	ui_choice='    <line choice="ui"/>'
	ui_line="  <choice id=\"ui\" title=\"Tray application\" description=\"Settings and status of the guns in the menu bar (optional).\" start_selected=\"true\">
    <pkg-ref id=\"com.sinden-driver.pkg.ui\"/>
  </choice>
  <pkg-ref id=\"com.sinden-driver.pkg.ui\" version=\"$ui_version\">ui.pkg</pkg-ref>"
fi

# --- Product ---------------------------------------------------------------------
cat >"$work/distribution.xml" <<EOF
<?xml version="1.0" encoding="utf-8"?>
<installer-gui-script minSpecVersion="2">
  <title>Sinden Driver $version</title>
  <options customize="allow" require-scripts="false" hostArchitectures="x86_64,arm64"/>
  <os-version min="11.0"/>
  <domains enable_localSystem="true"/>
  <choices-outline>
    <line choice="driver"/>
$ui_choice
  </choices-outline>
  <choice id="driver" title="Driver" description="The sinden-driver agent, started at login (required)." enabled="false" start_selected="true">
    <pkg-ref id="com.sinden-driver.pkg.driver"/>
  </choice>
  <pkg-ref id="com.sinden-driver.pkg.driver" version="$version">driver.pkg</pkg-ref>
$ui_line
</installer-gui-script>
EOF

product="$out/sinden-driver-$version-macos-universal.pkg"
if [ -n "${MACOS_INSTALLER_IDENTITY:-}" ]; then
	productbuild --distribution "$work/distribution.xml" --package-path "$work/components" \
		--sign "$MACOS_INSTALLER_IDENTITY" --timestamp "$product"
else
	productbuild --distribution "$work/distribution.xml" --package-path "$work/components" "$product"
fi

if [ -n "${MACOS_NOTARY_APPLE_ID:-}" ] && [ -n "${MACOS_NOTARY_TEAM_ID:-}" ] && [ -n "${MACOS_NOTARY_PASSWORD:-}" ]; then
	xcrun notarytool submit "$product" --wait \
		--apple-id "$MACOS_NOTARY_APPLE_ID" --team-id "$MACOS_NOTARY_TEAM_ID" --password "$MACOS_NOTARY_PASSWORD"
	xcrun stapler staple "$product"
fi

echo "build-pkg: $product"
