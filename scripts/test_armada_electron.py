#!/usr/bin/env python3
#
# `scripts/armada-electron` against a fake Electron.app in a scratch tree.
#
#   python3 scripts/test_armada_electron.py
#
# By hand, on a Mac: it needs PlistBuddy and codesign, so CI does not run it.

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
            plistlib.dump({"CFBundleName": "Electron", "CFBundleExecutable": "Electron",
                           "CFBundleIconFile": "electron.icns"}, f)
        resources = os.path.join(dist, "Electron.app", "Contents", "Resources")
        os.makedirs(resources)
        with open(os.path.join(resources, "electron.icns"), "w") as f:
            f.write("electron icon\n")
        icons = os.path.join(self.tmp, "packages", "brand", "macos")
        os.makedirs(icons)
        with open(os.path.join(icons, "ClassicIcon.icns"), "w") as f:
            f.write("classic icon\n")

    def test_the_copy_is_named_armada_classic(self):
        out = subprocess.run([ARMADA_ELECTRON, self.tmp], capture_output=True, text=True)
        self.assertEqual(out.returncode, 0, out.stderr)
        binary = out.stdout.strip()
        self.assertTrue(os.access(binary, os.X_OK), binary)
        plist = os.path.join(os.path.dirname(os.path.dirname(binary)), "Info.plist")
        with open(plist, "rb") as f:
            info = plistlib.load(f)
        self.assertEqual(info["CFBundleName"], "Armada Classic")
        self.assertEqual(info["CFBundleDisplayName"], "Armada Classic")
        self.assertEqual(os.path.basename(os.path.dirname(os.path.dirname(os.path.dirname(binary)))), "Armada Classic.app")

    def test_the_copy_carries_the_classic_icon_and_electrons_own_is_untouched(self):
        out = subprocess.run([ARMADA_ELECTRON, self.tmp], capture_output=True, text=True)
        self.assertEqual(out.returncode, 0, out.stderr)
        resources = os.path.join(os.path.dirname(os.path.dirname(out.stdout.strip())), "Resources")
        with open(os.path.join(resources, "electron.icns")) as f:
            self.assertEqual(f.read(), "classic icon\n")
        real = os.path.join(self.tmp, "apps", "desktop", "node_modules", "electron", "dist",
                            "Electron.app", "Contents", "Resources", "electron.icns")
        with open(real) as f:
            self.assertEqual(f.read(), "electron icon\n")

    def test_a_copy_made_before_the_rename_is_made_again(self):
        cache = os.path.join(self.tmp, "node_modules", ".cache", "armada-bridge")
        os.makedirs(os.path.join(cache, "Armada.app"))
        with open(os.path.join(cache, ".stamp"), "w") as f:
            f.write("1.2.3-armada-executable")
        out = subprocess.run([ARMADA_ELECTRON, self.tmp], capture_output=True, text=True)
        self.assertEqual(out.returncode, 0, out.stderr)
        self.assertEqual(os.listdir(cache).count("Armada.app"), 0)

    def test_the_process_is_named_armada_classic(self):
        # The Dock and the app switcher read the executable's file name.
        out = subprocess.run([ARMADA_ELECTRON, self.tmp], capture_output=True, text=True)
        self.assertEqual(out.returncode, 0, out.stderr)
        binary = out.stdout.strip()
        self.assertEqual(os.path.basename(binary), "Armada Classic")
        plist = os.path.join(os.path.dirname(os.path.dirname(binary)), "Info.plist")
        with open(plist, "rb") as f:
            self.assertEqual(plistlib.load(f)["CFBundleExecutable"], "Armada Classic")

    def test_dev_runs_bridge_on_the_copy(self):
        with open(DEV) as f:
            text = f.read()
        self.assertIn("scripts/armada-electron", text)
        self.assertIn("ELECTRON_EXEC_PATH", text)


if __name__ == "__main__":
    unittest.main()
