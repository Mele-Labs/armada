#!/usr/bin/env python3
#
# `scripts/armada-electron` against a fake Electron.app in a scratch tree.
#
#   python3 scripts/test_armada_electron.py

import os
import plistlib
import shutil
import subprocess
import tempfile
import unittest

HERE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ARMADA_ELECTRON = os.path.join(HERE, "scripts", "armada-electron")
DEV = os.path.join(HERE, "scripts", "dev")


class ArmadaElectron(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, self.tmp)
        dist = os.path.join(self.tmp, "apps", "desktop", "node_modules", "electron", "dist")
        macos = os.path.join(dist, "Electron.app", "Contents", "MacOS")
        os.makedirs(macos)
        with open(os.path.join(dist, "version"), "w") as f:
            f.write("1.2.3")
        binary = os.path.join(macos, "Electron")
        with open(binary, "w") as f:
            f.write("#!/bin/sh\n")
        os.chmod(binary, 0o755)
        with open(os.path.join(dist, "Electron.app", "Contents", "Info.plist"), "wb") as f:
            plistlib.dump({"CFBundleName": "Electron", "CFBundleExecutable": "Electron"}, f)

    def test_the_copy_is_named_armada(self):
        out = subprocess.run([ARMADA_ELECTRON, self.tmp], capture_output=True, text=True)
        self.assertEqual(out.returncode, 0, out.stderr)
        binary = out.stdout.strip()
        self.assertTrue(os.access(binary, os.X_OK), binary)
        plist = os.path.join(os.path.dirname(os.path.dirname(binary)), "Info.plist")
        with open(plist, "rb") as f:
            info = plistlib.load(f)
        self.assertEqual(info["CFBundleName"], "Armada")
        self.assertEqual(info["CFBundleDisplayName"], "Armada")

    def test_dev_runs_bridge_on_the_copy(self):
        with open(DEV) as f:
            text = f.read()
        self.assertIn("scripts/armada-electron", text)
        self.assertIn("ELECTRON_EXEC_PATH", text)


if __name__ == "__main__":
    unittest.main()
