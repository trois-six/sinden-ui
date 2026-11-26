#!/bin/sh
# Removes sinden-driver and sinden-driver-ui. Run with sudo:
#   sudo "/Library/Application Support/SindenDriver/uninstall.sh"
# Removes the logged-in user's agent; other users who ran `sinden-driver
# service install` run `sinden-driver service uninstall` first. The users'
# configuration (~/Library/Application Support/SindenDriver) stays.
set -u
if [ "$(id -u)" -ne 0 ]; then
	echo "run with sudo" >&2
	exit 1
fi
user=$(stat -f %Su /dev/console)
if [ -n "$user" ] && [ "$user" != "root" ] && [ "$user" != "loginwindow" ]; then
	uid=$(id -u "$user")
	bin="/Library/Application Support/SindenDriver/SindenDriver.app/Contents/MacOS/sinden-driver"
	if [ -x "$bin" ]; then
		launchctl asuser "$uid" sudo -u "$user" "$bin" service stop || true
		launchctl asuser "$uid" sudo -u "$user" "$bin" service uninstall || true
	fi
	if [ -x "/Applications/Sinden Driver.app/Contents/MacOS/sinden-driver-ui" ]; then
		launchctl asuser "$uid" sudo -u "$user" \
			"/Applications/Sinden Driver.app/Contents/MacOS/sinden-driver-ui" --autostart=off || true
	fi
	pkill -x sinden-driver-ui 2>/dev/null || true
fi
rm -f /usr/local/bin/sinden-driver
rm -rf "/Applications/Sinden Driver.app" "/Library/Application Support/SindenDriver"
pkgutil --forget com.sinden-driver.pkg.driver >/dev/null 2>&1 || true
pkgutil --forget com.sinden-driver.pkg.ui >/dev/null 2>&1 || true
echo "sinden-driver removed"
