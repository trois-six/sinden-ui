"""Tests of scripts/fetch-driver against a fake local release.

Run: python3 -m unittest discover -s scripts/tests -v
"""

from __future__ import annotations

import contextlib
import functools
import hashlib
import http.server
import importlib.machinery
import importlib.util
import io
import json
import tempfile
import threading
import unittest
import unittest.mock
from pathlib import Path

SCRIPT = Path(__file__).resolve().parent.parent / "fetch-driver"


def load_module():
    loader = importlib.machinery.SourceFileLoader("fetch_driver", str(SCRIPT))
    spec = importlib.util.spec_from_loader("fetch_driver", loader)
    module = importlib.util.module_from_spec(spec)
    loader.exec_module(module)
    return module


fetch_driver = load_module()

TAG = "driver-v1.2.3"
CONTENTS = {
    "x86_64-unknown-linux-musl": b"\x7fELF fake linux driver",
    "x86_64-pc-windows-msvc": b"MZ fake windows driver",
}


def file_name(target: str) -> str:
    suffix = ".exe" if "windows" in target else ""
    return f"sinden-driver-1.2.3-{target}{suffix}"


class FakeRelease:
    """A release directory <root>/<tag>/<file> and its driver.lock."""

    def __init__(self, root: Path):
        self.root = root
        (root / TAG).mkdir(parents=True)
        assets = {}
        for target, data in CONTENTS.items():
            (root / TAG / file_name(target)).write_bytes(data)
            assets[target] = {
                "file": file_name(target),
                "sha256": hashlib.sha256(data).hexdigest(),
            }
        self.lock = {"version": "1.2.3", "tag": TAG, "assets": assets}
        self.lock_path = root / "driver.lock"
        self.write_lock()

    def write_lock(self) -> None:
        self.lock_path.write_text(json.dumps(self.lock), encoding="utf-8")


class QuietHandler(http.server.SimpleHTTPRequestHandler):
    def log_message(self, *args):  # keep the test output clean
        pass


@contextlib.contextmanager
def http_server(directory: Path):
    handler = functools.partial(QuietHandler, directory=str(directory))
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield f"http://127.0.0.1:{server.server_address[1]}"
    finally:
        server.shutdown()
        server.server_close()


class FetchDriverTest(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = Path(self._tmp.name)
        self.release = FakeRelease(self.tmp / "release")
        self.out = self.tmp / "out"

    def tearDown(self):
        self._tmp.cleanup()

    def run_main(self, *args: str) -> tuple[int, str, str]:
        stdout, stderr = io.StringIO(), io.StringIO()
        argv = ["--lock", str(self.release.lock_path), "--out", str(self.out), *args]
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            code = fetch_driver.main(argv)
        return code, stdout.getvalue(), stderr.getvalue()

    def file_url(self) -> str:
        return self.release.root.as_uri()

    def assert_installed(self, target: str) -> None:
        installed = self.out / target / fetch_driver.binary_name(target)
        self.assertEqual(installed.read_bytes(), CONTENTS[target])
        self.assertTrue(installed.stat().st_mode & 0o111, "binary must be executable")

    def test_fetch_all_over_file_url(self):
        code, out, err = self.run_main("--base-url", self.file_url())
        self.assertEqual(code, 0, err)
        for target in CONTENTS:
            self.assert_installed(target)
            self.assertIn(f"{target}: sha256 ok", out)
        self.assertEqual((self.out / "VERSION").read_text().strip(), "1.2.3")
        self.assertFalse((self.out / "STUB").exists())

    def test_fetch_one_target_over_http(self):
        with http_server(self.release.root) as base:
            code, _, err = self.run_main("--base-url", base, "--target", "x86_64-pc-windows-msvc")
        self.assertEqual(code, 0, err)
        self.assert_installed("x86_64-pc-windows-msvc")
        self.assertFalse((self.out / "x86_64-unknown-linux-musl").exists())

    def test_base_url_from_environment(self):
        env_name = fetch_driver.BASE_URL_ENV
        with unittest.mock.patch.dict("os.environ", {env_name: self.file_url()}):
            code, _, err = self.run_main()
        self.assertEqual(code, 0, err)

    def test_mismatch_fails_loudly_and_deletes_the_download(self):
        target = "x86_64-unknown-linux-musl"
        self.release.lock["assets"][target]["sha256"] = "0" * 64
        self.release.write_lock()
        code, _, err = self.run_main("--base-url", self.file_url(), "--target", target)
        self.assertEqual(code, 1)
        self.assertIn("SHA-256 MISMATCH", err)
        self.assertFalse((self.out / file_name(target)).exists())
        self.assertFalse((self.out / target).exists())
        self.assertEqual([p.name for p in self.out.iterdir() if p.name.startswith(".part-")], [])

    def test_tampered_cached_file_is_downloaded_again(self):
        target = "x86_64-unknown-linux-musl"
        self.out.mkdir()
        (self.out / file_name(target)).write_bytes(b"tampered")
        code, out, err = self.run_main("--base-url", self.file_url(), "--target", target)
        self.assertEqual(code, 0, err)
        self.assertIn("downloading", out)
        self.assert_installed(target)

    def test_verified_cached_file_is_not_downloaded_again(self):
        target = "x86_64-unknown-linux-musl"
        self.assertEqual(self.run_main("--base-url", self.file_url(), "--target", target)[0], 0)
        # The release disappears: the cached, verified file must be enough.
        (self.release.root / TAG / file_name(target)).unlink()
        code, out, err = self.run_main("--base-url", self.file_url(), "--target", target)
        self.assertEqual(code, 0, err)
        self.assertIn("already downloaded and verified", out)

    def test_missing_asset_on_server_fails(self):
        with http_server(self.release.root) as base:
            (self.release.root / TAG / file_name("x86_64-pc-windows-msvc")).unlink()
            code, _, err = self.run_main("--base-url", base)
        self.assertEqual(code, 1)
        self.assertIn("download failed", err)

    def test_unknown_target_fails(self):
        code, _, err = self.run_main("--base-url", self.file_url(), "--target", "riscv64gc-unknown-linux-musl")
        self.assertEqual(code, 1)
        self.assertIn("not in driver.lock", err)

    def test_placeholder_lock_asks_for_stub(self):
        self.release.lock = {"version": "0.0.0", "tag": "", "assets": {}}
        self.release.write_lock()
        code, _, err = self.run_main("--base-url", self.file_url())
        self.assertEqual(code, 1)
        self.assertIn("--stub", err)

    def test_stub_writes_every_known_target(self):
        self.release.lock = {"version": "0.0.0", "tag": "", "assets": {}}
        self.release.write_lock()
        code, _, err = self.run_main("--stub")
        self.assertEqual(code, 0, err)
        for target in fetch_driver.KNOWN_TARGETS:
            stub = self.out / target / fetch_driver.binary_name(target)
            self.assertIn("stub", stub.read_text())
        self.assertTrue((self.out / "STUB").is_file())

    def test_real_fetch_clears_the_stub_marker(self):
        self.assertEqual(self.run_main("--stub")[0], 0)
        self.assertEqual(self.run_main("--base-url", self.file_url())[0], 0)
        self.assertFalse((self.out / "STUB").exists())

    def test_invalid_lock_entries_are_rejected(self):
        cases = {
            "path traversal": {"file": "../evil", "sha256": "a" * 64},
            "slash": {"file": "dir/evil", "sha256": "a" * 64},
            "short digest": {"file": "ok", "sha256": "abc"},
            "uppercase digest": {"file": "ok", "sha256": "A" * 64},
        }
        for name, asset in cases.items():
            with self.subTest(name):
                self.release.lock["assets"] = {"x86_64-unknown-linux-musl": asset}
                self.release.write_lock()
                code, _, err = self.run_main("--base-url", self.file_url())
                self.assertEqual(code, 1)
                self.assertIn("invalid", err)

    def test_list(self):
        code, out, _ = self.run_main("--list")
        self.assertEqual(code, 0)
        self.assertIn("version 1.2.3", out)
        self.assertIn(f"tag {TAG}", out)

    def test_repository_lock_is_valid(self):
        lock = fetch_driver.load_lock(SCRIPT.parent.parent / "driver.lock")
        self.assertIsInstance(lock["assets"], dict)


if __name__ == "__main__":
    unittest.main()
