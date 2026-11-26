#!/bin/sh
# sinden-driver: after install or upgrade (deb: configure; rpm: %post 1|2).
set -e

if [ "$1" = "configure" ] || [ "$1" = "1" ] || [ "$1" = "2" ] || [ -z "$1" ]; then
	# The service's user, then the configuration it may rewrite.
	if command -v systemd-sysusers >/dev/null 2>&1; then
		systemd-sysusers /usr/lib/sysusers.d/sinden-driver.conf
	elif ! getent passwd sinden >/dev/null; then
		useradd --system --user-group --no-create-home --home-dir /var/empty \
			--shell /usr/sbin/nologin --comment "Sinden light gun driver" sinden
	fi
	chown root:sinden /etc/sinden-driver /etc/sinden-driver/LightgunMono.exe.config
	chmod 0775 /etc/sinden-driver
	chmod 0664 /etc/sinden-driver/LightgunMono.exe.config

	# The guns already plugged get the new permissions.
	if command -v udevadm >/dev/null 2>&1; then
		udevadm control --reload-rules || true
		udevadm trigger --action=change --property-match=ID_VENDOR_ID=16c0 \
			--subsystem-match=tty --subsystem-match=video4linux --subsystem-match=input || true
	fi

	if [ -d /run/systemd/system ]; then
		systemctl daemon-reload
		systemctl enable sinden-driver.service
		systemctl restart sinden-driver.service || true
	fi
fi
exit 0
