#!/bin/sh
# sinden-driver: after removal (deb: remove|purge; rpm: %postun 0).
# The user `sinden` is kept (its files may outlive the package).
set -e

case "$1" in
remove | purge | 0)
	if [ -d /run/systemd/system ]; then
		systemctl daemon-reload || true
	fi
	# Without the hwdb entry, the guns' axes get the kernel's filter back.
	if command -v systemd-hwdb >/dev/null 2>&1; then
		systemd-hwdb update || true
	fi
	if command -v udevadm >/dev/null 2>&1; then
		udevadm control --reload || true
		udevadm trigger --action=change --subsystem-match=input || true
	fi
	;;
esac
if [ "$1" = "purge" ]; then
	rm -rf /etc/sinden-driver
fi
exit 0
