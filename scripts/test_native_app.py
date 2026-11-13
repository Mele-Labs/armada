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
PLIST = (
    '<?xml version="1.0" encoding="UTF-8"?><plist version="1.0"><dict>'
    "<key>CFBundleIdentifier</key><string>dev.armada.voice.test</string>"
    "<key>CFBundleExecutable</key><string>armada-voice</string>"
    "<key>CFBundlePackageType</key><string>APPL</string></dict></plist>"
)
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

    def run_script(self, *args, env=None):
        return subprocess.run([NATIVE_APP, *args], capture_output=True, text=True, env=env)

    def make(self, *extra):
        done = self.run_script(self.binary, self.out, *extra)
        self.assertEqual(done.returncode, 0, done.stderr)
        return done.stdout.strip()

    def test_it_makes_armada_app_in_the_folder_and_prints_where(self):
        app = self.make()
        self.assertEqual(app, os.path.join(self.out, "Armada.app"))
        contents = os.path.join(app, "Contents")
        # Helpers is there when Armada Voice is, and the script builds it where it can.
        helper = os.path.join(HERE, "apps", "desktop", "native", "voice", "build", "Armada Voice.app", "Contents", "MacOS", "armada-voice")
        want = ["Info.plist", "MacOS", "Resources", "_CodeSignature"]
        self.assertEqual(sorted(os.listdir(contents)), sorted(want + (["Helpers"] if os.path.isfile(helper) else [])))
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

    def test_the_notification_tones_are_in_resources_as_the_desktop_app_ships_them(self):
        app = self.make()
        for tone in ("armada-blocked.wav", "armada-waiting.wav"):
            with open(os.path.join(HERE, "apps", "desktop", "sounds", tone), "rb") as shipped, open(os.path.join(app, "Contents", "Resources", tone), "rb") as kept:
                self.assertEqual(kept.read(), shipped.read())

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


@unittest.skipUnless(shutil.which("codesign") and shutil.which("node"), "the bundle is signed, and the helper is built with node")
class NativeAppVoice(unittest.TestCase):
    """Armada Voice is built into the app, from a scratch tree whose build.mjs only counts how often it ran."""

    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, self.tmp)
        self.out = os.path.join(self.tmp, "out")
        self.binary = os.path.join(self.tmp, "native")
        shutil.copyfile("/usr/bin/true", self.binary)
        self.voice = os.path.join(self.tmp, "tree", "apps", "desktop", "native", "voice")
        os.makedirs(self.voice)
        self.calls = os.path.join(self.tmp, "calls")
        self.helper = os.path.join(self.voice, "build", "Armada Voice.app", "Contents", "MacOS", "armada-voice")
        self.source = os.path.join(self.voice, "main.swift")
        with open(self.source, "w") as f:
            f.write("source\n")
        self.script(
            'import { appendFileSync, copyFileSync, mkdirSync, writeFileSync } from "node:fs";\n'
            f'appendFileSync({self.calls!r}, "x");\n'
            f'mkdirSync({os.path.dirname(self.helper)!r}, {{ recursive: true }});\n'
            # A real Mach-O in a bundle with an Info.plist, since the app is signed with it inside.
            f'copyFileSync("/usr/bin/true", {self.helper!r});\n'
            f'writeFileSync({os.path.join(self.voice, "build", "Armada Voice.app", "Contents", "Info.plist")!r}, {PLIST!r});\n'
            'console.log("voice: built");\n'
        )

    def script(self, text):
        with open(os.path.join(self.voice, "build.mjs"), "w") as f:
            f.write(text)

    def built(self):
        if not os.path.exists(self.calls):
            return 0
        with open(self.calls) as f:
            return len(f.read())

    def make(self):
        env = {**os.environ, "NATIVE_APP_TREE": os.path.join(self.tmp, "tree")}
        return subprocess.run([NATIVE_APP, self.binary, self.out], capture_output=True, text=True, env=env)

    def carried(self):
        return os.path.isfile(os.path.join(self.out, "Armada.app", "Contents", "Helpers", "Armada Voice.app", "Contents", "MacOS", "armada-voice"))

    def test_a_missing_helper_is_built_and_carried_and_stdout_stays_the_apps_path(self):
        done = self.make()
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(done.stdout.strip(), os.path.join(self.out, "Armada.app"))
        self.assertEqual(self.built(), 1)
        self.assertTrue(self.carried())

    def test_a_helper_newer_than_every_source_is_not_built_again(self):
        self.make()
        past = os.path.getmtime(self.helper) - 100
        for name in os.listdir(self.voice):
            if os.path.isfile(os.path.join(self.voice, name)):
                os.utime(os.path.join(self.voice, name), (past, past))
        self.make()
        self.assertEqual(self.built(), 1)
        self.assertTrue(self.carried())

    def test_a_source_newer_than_the_helper_builds_it_again(self):
        self.make()
        later = os.path.getmtime(self.helper) + 100
        os.utime(self.source, (later, later))
        self.make()
        self.assertEqual(self.built(), 2)

    def test_a_failed_build_says_so_in_one_line_and_the_app_is_made_without_it(self):
        self.script("process.exit(1);\n")
        done = self.make()
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(done.stdout.strip(), os.path.join(self.out, "Armada.app"))
        self.assertEqual(len([line for line in done.stderr.splitlines() if "Armada Voice" in line]), 1)
        self.assertFalse(self.carried())


if __name__ == "__main__":
    unittest.main()
