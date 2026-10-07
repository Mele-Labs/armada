#!/usr/bin/env python3
#
# `scripts/sync-mod` against a scratch HOME, never the real one.
#
#   python3 scripts/test_sync_mod.py

import os
import shutil
import subprocess
import tempfile
import unittest

HERE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SYNC = os.path.join(HERE, "scripts", "sync-mod")


class SyncMod(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, self.tmp)
        self.home = os.path.join(self.tmp, "home")
        self.tree = os.path.join(self.tmp, "tree")
        os.makedirs(self.home)
        self.support = os.path.join(self.home, "Library", "Application Support", "Armada")
        self.mod = os.path.join(self.support, "mod")

    def write(self, path, text):
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w") as f:
            f.write(text)

    def source(self, version):
        self.write(os.path.join(self.tree, "plugins", ".claude-plugin", "marketplace.json"), "{}")
        self.write(os.path.join(self.tree, "plugins", "armada", ".claude-plugin", "plugin.json"), version)

    def sync(self):
        return subprocess.run([SYNC, self.tree], env={**os.environ, "HOME": self.home},
                              capture_output=True, text=True)

    def read(self, *parts):
        with open(os.path.join(self.mod, *parts)) as f:
            return f.read()

    def test_the_copy_is_made(self):
        self.source("v1")
        self.assertEqual(self.sync().returncode, 0)
        self.assertEqual(self.read("armada", ".claude-plugin", "plugin.json"), "v1")
        self.assertTrue(os.path.isfile(os.path.join(self.mod, ".claude-plugin", "marketplace.json")))

    def test_a_second_copy_replaces_the_first_and_leaves_nothing_behind(self):
        self.source("v1")
        self.write(os.path.join(self.support, "fleet.json"), "keep")
        self.sync()
        self.source("v2")
        self.write(os.path.join(self.tree, "plugins", "armada", "new.txt"), "x")
        self.assertEqual(self.sync().returncode, 0)
        self.assertEqual(self.read("armada", ".claude-plugin", "plugin.json"), "v2")
        self.assertEqual(sorted(os.listdir(self.support)), ["fleet.json", "mod"])

    def test_a_file_dropped_from_the_source_is_dropped_from_the_copy(self):
        self.source("v1")
        self.write(os.path.join(self.tree, "plugins", "armada", "old.txt"), "x")
        self.sync()
        os.remove(os.path.join(self.tree, "plugins", "armada", "old.txt"))
        self.sync()
        self.assertFalse(os.path.exists(os.path.join(self.mod, "armada", "old.txt")))

    def test_a_missing_source_warns_and_keeps_the_old_copy(self):
        self.source("v1")
        self.sync()
        shutil.rmtree(os.path.join(self.tree, "plugins"))
        done = self.sync()
        self.assertEqual(done.returncode, 0)
        self.assertIn("warning", done.stderr)
        self.assertEqual(self.read("armada", ".claude-plugin", "plugin.json"), "v1")

    def test_a_missing_source_with_no_copy_makes_nothing(self):
        done = self.sync()
        self.assertEqual(done.returncode, 0)
        self.assertIn("warning", done.stderr)
        self.assertFalse(os.path.exists(self.mod))


if __name__ == "__main__":
    unittest.main()
