#!/bin/sh
# sinden-driver: after removal (deb: remove|purge; rpm: %postun 0).
# The user `sinden` is kept (its files may outlive the package).
set -e

case "$1" in
remove | purge | 0)
	if [ -d /run/systemd/system ]; then
		systemctl daemon-reload || true
	fi
	if command -v udevadm >/dev/null 2>&1; then
		udevadm control --reload-rules || true
	fi
	;;
esac
if [ "$1" = "purge" ]; then
	rm -rf /etc/sinden-driver
fi
exit 0
