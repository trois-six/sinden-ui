# Shell snippet installing the hand-written fallbacks of the driver's
# service files under $out/lib, for $out/bin/sinden-driver. Used when the
# driver cannot run at build time (cross builds) and by the module checks.
''
  install -Dm644 ${../linux/files/sinden-driver.service} "$out/lib/systemd/system/sinden-driver.service"
  substituteInPlace "$out/lib/systemd/system/sinden-driver.service" \
    --replace-fail /usr/bin/sinden-driver "$out/bin/sinden-driver"
  install -Dm644 ${../linux/files/60-sinden-driver.rules} "$out/lib/udev/rules.d/60-sinden-driver.rules"
  install -Dm644 ${../linux/files/sinden-driver.sysusers} "$out/lib/sysusers.d/sinden-driver.conf"
  install -Dm644 ${../linux/files/sinden-driver.tmpfiles} "$out/lib/tmpfiles.d/sinden-driver.conf"
  install -Dm644 ${../linux/files/sinden-driver.hwdb} "$out/lib/udev/hwdb.d/60-sinden-driver.hwdb"
''
