#!/bin/sh
# sinden-driver: after install or upgrade (deb: configure; rpm: %post 1|2).
set -e

if [ "$1" = "configure" ] || [ "$1" = "1" ] || [ "$1" = "2" ] || [ -z "$1" ]; then
	# The service's user (in its own group only), then the configuration it
	# rewrites. The file names are the driver's: take every sinden one.
	if command -v systemd-sysusers >/dev/null 2>&1; then
		for f in /usr/lib/sysusers.d/*sinden*.conf; do
			if [ -e "$f" ]; then systemd-sysusers "$f"; fi
		done
	fi
	if ! getent passwd sinden >/dev/null; then
		useradd --system --user-group --no-create-home --home-dir /var/empty \
			--shell /usr/sbin/nologin --comment "Sinden light gun driver" sinden
	fi
	if command -v systemd-tmpfiles >/dev/null 2>&1; then
		for f in /usr/lib/tmpfiles.d/*sinden*.conf; do
			if [ -e "$f" ]; then systemd-tmpfiles --create "$f" || true; fi
		done
	fi
	chown sinden:sinden /etc/sinden-driver /etc/sinden-driver/LightgunMono.exe.config
	chmod 0755 /etc/sinden-driver
	chmod 0644 /etc/sinden-driver/LightgunMono.exe.config

	# The guns already plugged get the new permissions (udev rule) and their
	# axes' filter cleared (hwdb).
	if command -v systemd-hwdb >/dev/null 2>&1; then
		systemd-hwdb update || true
	fi
	if command -v udevadm >/dev/null 2>&1; then
		udevadm control --reload || true
		udevadm trigger --action=change --subsystem-match=tty \
			--subsystem-match=video4linux --subsystem-match=input || true
	fi

	if [ -d /run/systemd/system ]; then
		systemctl daemon-reload
		systemctl enable sinden-driver.service
		systemctl restart sinden-driver.service || true
	fi
fi
exit 0
