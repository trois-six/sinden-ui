#!/bin/sh
# sinden-driver: before removal (deb: remove; rpm: %preun 0). Upgrades keep
# the service enabled.
set -e

if [ "$1" = "remove" ] || [ "$1" = "0" ]; then
	if [ -d /run/systemd/system ]; then
		systemctl disable --now sinden-driver.service || true
	fi
fi
exit 0
