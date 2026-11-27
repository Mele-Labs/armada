#!/usr/bin/env python3
#
# `scripts/app-entry` against a scratch HOME: the opener it keeps in `~/Applications`, what it
# leaves alone, and which of launchd and `open` the opener hands over to. The opener runs
# against stub `launchctl` and `open`, so nothing here starts an app or touches launchd.
#
#   python3 scripts/test_app_entry.py
#
# `ARMADA_TEST_SCRIPTS` names another `scripts/` folder to run against, which is
# how a test is shown red: the folder as it was before the change.

import os
import shutil
import subprocess
import tempfile
import unittest

HERE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SCRIPTS = os.environ.get("ARMADA_TEST_SCRIPTS") or os.path.join(HERE, "scripts")
OPENER_ID = "dev.armada.bridge.opener"


def write(path, text, mode=0o644):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w") as f:
        f.write(text)
    os.chmod(path, mode)


def read(path):
    with open(path) as f:
        return f.read()


class AppEntry(unittest.TestCase):
    def setUp(self):
        self.home = os.path.realpath(tempfile.mkdtemp(prefix="armada-app-entry-"))
        self.addCleanup(shutil.rmtree, self.home, ignore_errors=True)
        builds = os.path.join(self.home, "Library/Application Support/Armada/builds")
        write(os.path.join(builds, "stamp-a/Armada.app/Contents/Resources/AppIcon.icns"), "icon a")
        os.symlink("stamp-a", os.path.join(builds, "current"))
        self.current = os.path.join(builds, "current/Armada.app")
        self.entry = os.path.join(self.home, "Applications/Armada.app")
        self.bin = os.path.join(self.home, "bin")
        self.calls = os.path.join(self.home, "calls")

    def run_entry(self, *args):
        env = {"HOME": self.home, "PATH": os.environ["PATH"], "ARMADA_FLEET_LABEL": "test.fleet"}
        return subprocess.run(
            [os.path.join(SCRIPTS, "app-entry"), *args], env=env, capture_output=True, text=True, check=True
        )

    def test_writes_an_opener_with_the_current_builds_icon_and_no_dock_icon(self):
        self.run_entry()
        plist = read(os.path.join(self.entry, "Contents/Info.plist"))
        self.assertIn(f"<string>{OPENER_ID}</string>", plist)
        self.assertIn("<key>LSUIElement</key><true/>", plist)
        self.assertEqual(read(os.path.join(self.entry, "Contents/Resources/AppIcon.icns")), "icon a")
        opener = os.path.join(self.entry, "Contents/MacOS/armada-open")
        self.assertTrue(os.access(opener, os.X_OK))
        self.assertIn("test.fleet.native", read(opener), "the opener names the native Bridge's job")
        self.assertEqual(os.stat(self.entry).st_mode & 0o777, 0o755)

    def test_a_second_run_changes_nothing_and_a_new_icon_is_taken_up(self):
        self.run_entry()
        self.assertIn("is current", self.run_entry("--dry-run").stdout)
        write(os.path.join(self.current, "Contents/Resources/AppIcon.icns"), "icon b")
        self.assertIn("would write", self.run_entry("--dry-run").stdout)
        self.assertEqual(read(os.path.join(self.entry, "Contents/Resources/AppIcon.icns")), "icon a", "a dry run writes nothing")
        self.run_entry()
        self.assertEqual(read(os.path.join(self.entry, "Contents/Resources/AppIcon.icns")), "icon b")

    def test_a_link_left_there_is_replaced(self):
        os.makedirs(os.path.dirname(self.entry))
        os.symlink(self.current, self.entry)
        self.run_entry()
        self.assertFalse(os.path.islink(self.entry))
        self.assertIn(OPENER_ID, read(os.path.join(self.entry, "Contents/Info.plist")))

    def test_an_app_it_did_not_write_is_left_alone(self):
        write(os.path.join(self.entry, "Contents/Info.plist"), "<string>dev.armada.bridge.native</string>")
        out = self.run_entry()
        self.assertIn("left alone", out.stderr)
        self.assertEqual(read(os.path.join(self.entry, "Contents/Info.plist")), "<string>dev.armada.bridge.native</string>")

    def test_no_build_to_take_an_icon_from_writes_nothing(self):
        os.remove(os.path.join(self.current, "Contents/Resources/AppIcon.icns"))
        self.run_entry()
        self.assertFalse(os.path.exists(self.entry))

    def open_with_job(self, state):
        """Run the opener with launchd reporting `state` for the job, or no job when None."""
        self.run_entry()
        printed = f"\tstate = {state}\n" if state else ""
        write(
            os.path.join(self.bin, "launchctl"),
            f'#!/bin/bash\necho "launchctl $*" >> "{self.calls}"\n'
            f'if [[ "$1" == print ]]; then [[ -n "{printed}" ]] || exit 113; printf "{printed}"; fi\n',
            0o755,
        )
        write(os.path.join(self.bin, "open"), f'#!/bin/bash\necho "open $*" >> "{self.calls}"\n', 0o755)
        env = {"HOME": self.home, "PATH": f"{self.bin}:{os.environ['PATH']}"}
        subprocess.run([os.path.join(self.entry, "Contents/MacOS/armada-open")], env=env, check=True)
        return [line for line in read(self.calls).splitlines() if not line.startswith("launchctl print")]

    def test_a_running_bridge_is_opened_which_brings_it_forward(self):
        self.assertEqual(self.open_with_job("running"), [f"open {self.current}"])

    def test_a_bridge_he_quit_is_started_by_launchd(self):
        uid = os.getuid()
        self.assertEqual(self.open_with_job("not running"), [f"launchctl kickstart gui/{uid}/test.fleet.native"])

    def test_with_no_job_the_current_build_is_opened(self):
        self.assertEqual(self.open_with_job(None), [f"open {self.current}"])


if __name__ == "__main__":
    unittest.main()
