"""Tests of scripts/build-aur's PKGBUILD rendering (no network, no makepkg)."""

from __future__ import annotations

import importlib.machinery
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parent.parent / "build-aur"


def load_module():
    loader = importlib.machinery.SourceFileLoader("build_aur", str(SCRIPT))
    spec = importlib.util.spec_from_loader("build_aur", loader)
    module = importlib.util.module_from_spec(spec)
    loader.exec_module(module)
    return module


build_aur = load_module()


class SetVarTest(unittest.TestCase):
    def test_scalar_and_multiline_array(self):
        text = "pkgver=0.0.0\nsha256sums=('SKIP'\n            'SKIP')\nsha256sums_x86_64=('SKIP')\n"
        text = build_aur.set_var(text, "pkgver", "1.2.3")
        text = build_aur.set_var(text, "sha256sums", build_aur.array(["a", "b"]))
        text = build_aur.set_var(text, "sha256sums_x86_64", "('c')")
        self.assertEqual(
            text,
            "pkgver=1.2.3\nsha256sums=('a'\n            'b')\nsha256sums_x86_64=('c')\n",
        )

    def test_missing_variable_fails(self):
        with self.assertRaises(SystemExit):
            build_aur.set_var("pkgver=1\n", "pkgrel", "2")


class RenderDriverTest(unittest.TestCase):
    def test_lock_values_land_in_the_pkgbuild(self):
        digest = "ab" * 32
        lock = {
            "version": "1.2.3",
            "tag": "driver-v1.2.3",
            "assets": {
                "x86_64-unknown-linux-musl": {"file": "sinden-driver-1.2.3-x86_64-unknown-linux-musl", "sha256": digest}
            },
        }
        real_root = build_aur.ROOT
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "driver.lock").write_text(json.dumps(lock), encoding="utf-8")
            build_aur.ROOT = root
            try:
                dest = build_aur.render_driver(root / "out")
            finally:
                build_aur.ROOT = real_root
            pkgbuild = (dest / "PKGBUILD").read_text(encoding="utf-8")
            self.assertIn("pkgver=1.2.3\n", pkgbuild)
            self.assertIn(f"sha256sums_x86_64=('{digest}')", pkgbuild)
            self.assertIn("sha256sums_aarch64=('SKIP')", pkgbuild)
            self.assertNotIn("sha256sums=('SKIP'", pkgbuild)
            for name in build_aur.DRIVER_FILES:
                self.assertTrue((dest / name).is_file(), name)
            self.assertTrue((dest / "sinden-driver.install").is_file())


if __name__ == "__main__":
    unittest.main()
