"""Tests of scripts/build-linux-packages' service files (no nfpm needed)."""

from __future__ import annotations

import contextlib
import importlib.machinery
import importlib.util
import io
import platform
import shutil
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parent.parent / "build-linux-packages"
REPO = SCRIPT.parent.parent


def load_module():
    loader = importlib.machinery.SourceFileLoader("build_linux_packages", str(SCRIPT))
    spec = importlib.util.spec_from_loader("build_linux_packages", loader)
    module = importlib.util.module_from_spec(spec)
    loader.exec_module(module)
    return module


blp = load_module()

# A stand-in for `sinden-driver service files --bin BIN --dir DIR`.
FAKE_DRIVER = """#!/bin/sh
[ "$1 $2 $3" = "service files --bin" ] && [ "$5" = "--dir" ] || exit 64
mkdir -p "$6/usr/lib/systemd/system" "$6/usr/lib/udev/rules.d"
printf '[Service]\\nExecStart=%s service run\\n' "$4" > "$6/usr/lib/systemd/system/sinden-driver.service"
echo 'SUBSYSTEM=="tty", GROUP="sinden"' > "$6/usr/lib/udev/rules.d/70-sinden-driver.rules"
"""


@unittest.skipUnless(platform.system() == "Linux" and platform.machine().lower() in blp.HOST_TARGET, "Linux host")
class ServiceFilesTest(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)
        for rel in ["packaging/linux/nfpm/sinden-driver.yaml", *blp.FALLBACK_SERVICE_FILES]:
            (self.root / rel).parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(REPO / rel, self.root / rel)
        self.driver_dir = self.root / "dist" / "driver"
        self.real_root, blp.ROOT = blp.ROOT, self.root

    def tearDown(self):
        blp.ROOT = self.real_root
        self._tmp.cleanup()

    def host_binary(self) -> Path:
        path = self.driver_dir / blp.HOST_TARGET[platform.machine().lower()] / "sinden-driver"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(FAKE_DRIVER, encoding="utf-8")
        path.chmod(0o644)  # as after an artifact download
        return path

    def render(self, force_fallback: bool = False) -> str:
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            tree = blp.service_files(self.driver_dir, force_fallback)
            return (self.root / blp.render_driver_config(tree)).read_text(encoding="utf-8")

    def test_the_driver_generates_the_files(self):
        self.host_binary()
        config = self.render()
        self.assertIn("dst: /usr/lib/systemd/system/sinden-driver.service", config)
        self.assertIn("dst: /usr/lib/udev/rules.d/70-sinden-driver.rules", config)
        self.assertNotIn("dst: /usr/lib/sysusers.d/", config, "no fallback mixed in")
        unit = self.root / "dist/stage/service-files/usr/lib/systemd/system/sinden-driver.service"
        self.assertIn("ExecStart=/usr/bin/sinden-driver service run", unit.read_text())
        self.assertNotIn("@SERVICE_FILES@", config)

    def test_stub_binaries_use_the_fallbacks(self):
        self.host_binary()
        (self.driver_dir / "STUB").write_text("", encoding="utf-8")
        config = self.render()
        for installed in blp.FALLBACK_SERVICE_FILES.values():
            self.assertIn(f"dst: /{installed}", config)

    def test_no_host_binary_uses_the_fallbacks(self):
        self.driver_dir.mkdir(parents=True)
        config = self.render()
        self.assertIn("dst: /usr/lib/tmpfiles.d/sinden-driver.conf", config)

    def test_forced_fallback(self):
        self.host_binary()
        config = self.render(force_fallback=True)
        self.assertIn("dst: /usr/lib/sysusers.d/sinden-driver.conf", config)

    def test_a_failing_driver_stops_the_build(self):
        self.host_binary().write_text("#!/bin/sh\nexit 3\n", encoding="utf-8")
        with self.assertRaises(SystemExit):
            self.render()


if __name__ == "__main__":
    unittest.main()
