#!/usr/bin/env python3
#
# `scripts/native-app` against a scratch folder, never an installed app.
#
#   python3 scripts/test_native_app.py
#
# By hand, on a Mac: it needs codesign, so a machine without one skips it and CI does not run it.

import os
import plistlib
import shutil
import subprocess
import tempfile
import unittest

HERE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
NATIVE_APP = os.path.join(HERE, "scripts", "native-app")
BRAND_ICON = os.path.join(HERE, "packages", "brand", "macos", "AppIcon.icns")


@unittest.skipUnless(shutil.which("codesign"), "the bundle is signed, and only a Mac can")
class NativeApp(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, self.tmp)
        self.out = os.path.join(self.tmp, "out dir")
        # A real, signed Mach-O to stand in for the cargo binary.
        self.binary = os.path.join(self.tmp, "native")
        shutil.copyfile("/usr/bin/true", self.binary)
        os.chmod(self.binary, 0o755)

    def run_script(self, *args):
        return subprocess.run([NATIVE_APP, *args], capture_output=True, text=True)

    def make(self, *extra):
        done = self.run_script(self.binary, self.out, *extra)
        self.assertEqual(done.returncode, 0, done.stderr)
        return done.stdout.strip()

    def test_it_makes_armada_app_in_the_folder_and_prints_where(self):
        app = self.make()
        self.assertEqual(app, os.path.join(self.out, "Armada.app"))
        contents = os.path.join(app, "Contents")
        self.assertEqual(sorted(os.listdir(contents)), ["Info.plist", "MacOS", "Resources", "_CodeSignature"])
        self.assertEqual(os.listdir(os.path.join(contents, "MacOS")), ["Armada"])
        self.assertTrue(os.access(os.path.join(contents, "MacOS", "Armada"), os.X_OK))

    def test_the_icon_is_the_brands_unless_one_is_given(self):
        app = self.make()
        with open(BRAND_ICON, "rb") as brand, open(os.path.join(app, "Contents", "Resources", "AppIcon.icns"), "rb") as kept:
            self.assertEqual(kept.read(), brand.read())
        other = os.path.join(self.tmp, "Other.icns")
        with open(other, "w") as f:
            f.write("other\n")
        app = self.make(other)
        with open(os.path.join(app, "Contents", "Resources", "AppIcon.icns")) as f:
            self.assertEqual(f.read(), "other\n")

    def test_the_plist_names_it_apart_from_electrons(self):
        app = self.make()
        with open(os.path.join(app, "Contents", "Info.plist"), "rb") as f:
            info = plistlib.load(f)
        self.assertEqual(info["CFBundleName"], "Armada")
        self.assertEqual(info["CFBundleExecutable"], "Armada")
        self.assertEqual(info["CFBundleIconFile"], "AppIcon")
        self.assertEqual(info["CFBundleIdentifier"], "dev.armada.bridge.native")
        self.assertNotIn(info["CFBundleIdentifier"], ("com.github.Electron", "dev.armada.bridge"))

    def test_it_is_signed_ad_hoc_and_the_signature_holds(self):
        app = self.make()
        verified = subprocess.run(["codesign", "--verify", "--deep", "--strict", app], capture_output=True, text=True)
        self.assertEqual(verified.returncode, 0, verified.stderr)
        shown = subprocess.run(["codesign", "-dv", app], capture_output=True, text=True)
        self.assertIn("Signature=adhoc", shown.stderr)

    def test_making_it_again_replaces_it(self):
        app = self.make()
        with open(os.path.join(app, "stray"), "w") as f:
            f.write("left\n")
        self.make()
        self.assertFalse(os.path.exists(os.path.join(app, "stray")))

    def test_a_binary_or_icon_that_is_not_there_makes_nothing(self):
        done = self.run_script(os.path.join(self.tmp, "nope"), self.out)
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("is not a file", done.stderr)
        done = self.run_script(self.binary, self.out, os.path.join(self.tmp, "nope.icns"))
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("nope.icns is missing", done.stderr)
        self.assertFalse(os.path.exists(os.path.join(self.out, "Armada.app")))

    def test_the_wrong_number_of_arguments_says_how_it_is_used(self):
        done = self.run_script(self.binary)
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("usage: native-app", done.stderr)


if __name__ == "__main__":
    unittest.main()
