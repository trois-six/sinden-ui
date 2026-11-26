#!/bin/sh
# sinden-driver: after install or upgrade (deb: configure; rpm: %post 1|2).
set -e

if [ "$1" = "configure" ] || [ "$1" = "1" ] || [ "$1" = "2" ] || [ -z "$1" ]; then
	# The service's user (member of group input), then the configuration it
	# may rewrite. The file names are the driver's: take every sinden one.
	if command -v systemd-sysusers >/dev/null 2>&1; then
		for f in /usr/lib/sysusers.d/*sinden*.conf; do
			if [ -e "$f" ]; then systemd-sysusers "$f"; fi
		done
	fi
	if ! getent passwd sinden >/dev/null; then
		useradd --system --user-group --no-create-home --home-dir /var/empty \
			--shell /usr/sbin/nologin --comment "Sinden light gun driver" sinden
		if getent group input >/dev/null; then usermod -a -G input sinden; fi
	fi
	if command -v systemd-tmpfiles >/dev/null 2>&1; then
		for f in /usr/lib/tmpfiles.d/*sinden*.conf; do
			if [ -e "$f" ]; then systemd-tmpfiles --create "$f" || true; fi
		done
	fi
	chown root:sinden /etc/sinden-driver /etc/sinden-driver/LightgunMono.exe.config
	chmod 0775 /etc/sinden-driver
	chmod 0664 /etc/sinden-driver/LightgunMono.exe.config

	# The guns already plugged get the new permissions.
	if command -v udevadm >/dev/null 2>&1; then
		udevadm control --reload-rules || true
		udevadm trigger --action=change --subsystem-match=tty \
			--subsystem-match=video4linux || true
	fi

	if [ -d /run/systemd/system ]; then
		systemctl daemon-reload
		systemctl enable sinden-driver.service
		systemctl restart sinden-driver.service || true
	fi
fi
exit 0
