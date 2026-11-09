#!/usr/bin/env python3
#
# `scripts/keep-build` against a scratch HOME and a fake tree, never the real ones.
#
#   python3 scripts/test_keep_build.py

import hashlib
import os
import plistlib
import re
import shutil
import subprocess
import tempfile
import unittest

HERE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
KEEP = os.path.join(HERE, "scripts", "keep-build")
NATIVE_APP = os.path.join(HERE, "scripts", "native-app")
LOCK = "aaaaaaaaaaaaaaaa"


def parse(text):
    """build.toml as `(flat keys, [[job]] tables)`. Flat `key = "value"` lines only."""
    flat, jobs, into = {}, [], None
    for line in text.splitlines():
        if line == "[[job]]":
            into = {}
            jobs.append(into)
        elif m := re.fullmatch(r'(\w+) = "(.*)"', line):
            (flat if into is None else into)[m.group(1)] = m.group(2)
        elif m := re.fullmatch(r'(\w+) = (true|false)', line):
            (flat if into is None else into)[m.group(1)] = m.group(2) == "true"
        else:
            assert line == "", f"unreadable build.toml line: {line!r}"
    return flat, jobs


class KeepBuild(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, self.tmp)
        self.home = os.path.join(self.tmp, "home")
        self.support = os.path.join(self.home, "Library", "Application Support", "Armada")
        self.builds = os.path.join(self.support, "builds")
        self.modules = os.path.join(self.support, "node_modules")
        self.src = os.path.join(self.tmp, "tree")
        os.makedirs(self.home)
        # keep-build finds `armada-electron` and `native-app` beside itself, so a stub and a copy sit beside a copy of it.
        scripts = os.path.join(self.tmp, "scripts")
        os.makedirs(scripts)
        self.keep = os.path.join(scripts, "keep-build")
        shutil.copy2(KEEP, self.keep)
        shutil.copy2(NATIVE_APP, os.path.join(scripts, "native-app"))
        self.electron_calls = os.path.join(scripts, "armada-electron.calls")
        self.write(os.path.join(scripts, "armada-electron"), (
            "#!/bin/sh\n"
            f'echo "$1" >> "{self.electron_calls}"\n'
            # What the real one does for a tree with Electron: a copy named Armada Classic in the tree.
            'if [ -f "$1/.stub-makes-the-copy" ]; then\n'
            '  rm -rf "$1/node_modules/.cache/armada-bridge"\n'
            '  mkdir -p "$1/node_modules/.cache/armada-bridge/Armada Classic.app/Contents/MacOS"\n'
            '  echo made > "$1/node_modules/.cache/armada-bridge/Armada Classic.app/Contents/MacOS/Armada Classic"\n'
            "  exit 0\n"
            "fi\n"
            "exit 1\n"), mode=0o755)
        self.armada = os.path.join(self.tmp, "armada")
        self.write(self.armada, "#!/bin/sh\necho armada\n", mode=0o755)
        self.tree()

    def read(self, *parts):
        with open(os.path.join(*parts)) as f:
            return f.read()

    def write(self, path, text, mode=None):
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w") as f:
            f.write(text)
        if mode is not None:
            os.chmod(path, mode)

    def tree(self):
        """The parts of a built checkout that a kept build reads, with pnpm's relative link."""
        s = self.src
        desktop = os.path.join(s, "apps", "desktop")
        self.write(os.path.join(desktop, "package.json"), '{"main": "./out/main/index.js"}\n')
        self.write(os.path.join(desktop, "out", "main", "index.js"), "main\n")
        self.write(os.path.join(desktop, "out", "preload", "index.js"), "preload\n")
        self.write(os.path.join(desktop, "out", "renderer", "index.html"), "<html>\n")
        self.write(os.path.join(desktop, "out", ".armada-restart-stamp"), "stamp\n")
        self.write(os.path.join(desktop, "out", ".armada-restart-stamp.pending"), "pending\n")
        self.write(os.path.join(desktop, "sounds", "a.wav"), "wav\n")
        self.write(os.path.join(desktop, "node_modules", "electron", "dist", "version"), "40.1.2\n")
        os.symlink("../../../node_modules/.pnpm/x", os.path.join(desktop, "node_modules", "x"))
        self.write(os.path.join(s, "node_modules", ".pnpm", "x", "index.js"), "x\n")
        self.write(os.path.join(s, "node_modules", ".cache", "armada-bridge", "Armada Classic.app",
                                "Contents", "MacOS", "Armada Classic"), "armada electron\n", mode=0o755)
        self.write(os.path.join(s, "pnpm-lock.yaml"), "lock one\n")
        self.write(os.path.join(s, "packages", "brand", "macos", "AppIcon.icns"), "icon one\n")
        self.write(os.path.join(s, "scripts", "bridge-launch"), "#!/bin/sh\nexec \"$@\"\n", mode=0o755)
        self.git("init", "-q")
        self.git("add", "-A")
        self.commit()

    def git(self, *args):
        return subprocess.run(["git", "-C", self.src, "-c", "user.name=t", "-c", "user.email=t@t", *args],
                              check=True, capture_output=True, text=True).stdout.strip()

    def commit(self):
        """A new HEAD, so two builds in one second still have two names."""
        self.git("commit", "-q", "--allow-empty", "-m", "c")
        return self.git("rev-parse", "HEAD")

    def run_keep(self, *args, path=None):
        env = {**os.environ, "HOME": self.home}
        if path is not None:
            env["PATH"] = f"{path}:{env['PATH']}"
        return subprocess.run([self.keep, *args], env=env, capture_output=True, text=True)

    def make(self, source="main", protocol="proto1", binary=None, native=None):
        done = self.run_keep("make", self.src, binary or self.armada, source, protocol, *([native] if native else []))
        self.assertEqual(done.returncode, 0, done.stderr)
        return done.stdout.strip()

    def point(self, snapshot):
        done = self.run_keep("point", snapshot)
        self.assertEqual(done.returncode, 0, done.stderr)

    def toml(self, snapshot):
        with open(os.path.join(snapshot, "build.toml")) as f:
            return parse(f.read())

    def link(self, name):
        try:
            return os.readlink(os.path.join(self.builds, name))
        except OSError:
            return None

    def fake_build(self, name, lock=LOCK):
        """A kept build of an earlier day: only what `point` reads."""
        self.write(os.path.join(self.builds, name, "build.toml"), f'node_modules = "{lock}"\n')
        os.makedirs(os.path.join(self.modules, lock), exist_ok=True)

    def set_link(self, name, to):
        os.makedirs(self.builds, exist_ok=True)
        os.symlink(to, os.path.join(self.builds, name))

    def folders(self):
        return sorted(f for f in os.listdir(self.builds) if f not in ("current", "previous"))

    def lock_of(self, text):
        return hashlib.sha256(text.encode()).hexdigest()[:16]

    # make

    def test_the_snapshot_has_the_layout(self):
        snapshot = self.make()
        self.assertEqual(os.path.dirname(snapshot), self.builds)
        self.assertRegex(os.path.basename(snapshot), r"^\d{4}-\d{2}-\d{2}T\d{6}Z-" + self.git("rev-parse", "HEAD")[:7] + "$")
        self.assertEqual(sorted(os.listdir(snapshot)), ["armada", "bridge", "bridge-launch", "build.toml", "launch-bridge"])
        self.assertEqual(self.read(os.path.join(snapshot, "armada")), "#!/bin/sh\necho armada\n")
        self.assertTrue(os.access(os.path.join(snapshot, "armada"), os.X_OK))
        self.assertTrue(os.access(os.path.join(snapshot, "bridge-launch"), os.X_OK))
        self.assertTrue(os.access(os.path.join(snapshot, "launch-bridge"), os.X_OK))
        bridge = os.path.join(snapshot, "bridge")
        self.assertEqual(sorted(os.listdir(bridge)), ["node_modules", "out", "package.json", "sounds"])
        self.assertEqual(self.read(os.path.join(bridge, "package.json")), '{"main": "./out/main/index.js"}\n')
        self.assertEqual(self.read(os.path.join(bridge, "sounds", "a.wav")), "wav\n")
        self.assertEqual(sorted(os.listdir(os.path.join(bridge, "out"))), ["main", "preload", "renderer"])
        self.assertEqual(os.readlink(os.path.join(bridge, "node_modules")),
                         os.path.join(self.modules, self.lock_of("lock one\n"), "apps", "desktop", "node_modules"))
        self.assertTrue(os.path.isfile(os.path.join(self.modules, self.lock_of("lock one\n"),
                                                    "node_modules", ".pnpm", "x", "index.js")))

    def native_binary(self, name="native", source="/usr/bin/true"):
        """A real, signed Mach-O to stand in for the cargo binary, since the bundle is signed."""
        path = os.path.join(self.tmp, name)
        shutil.copyfile(source, path)
        os.chmod(path, 0o755)
        return path

    def test_the_native_bridge_is_kept_as_armada_app(self):
        snapshot = self.make(native=self.native_binary())
        self.assertEqual(sorted(os.listdir(snapshot)),
                         ["Armada.app", "armada", "bridge", "bridge-launch", "build.toml", "launch-bridge"])
        app = os.path.join(snapshot, "Armada.app", "Contents")
        self.assertEqual(sorted(os.listdir(app)), ["Info.plist", "MacOS", "Resources", "_CodeSignature"])
        self.assertEqual(os.listdir(os.path.join(app, "MacOS")), ["Armada"])
        self.assertTrue(os.access(os.path.join(app, "MacOS", "Armada"), os.X_OK))
        self.assertEqual(self.read(app, "Resources", "AppIcon.icns"), "icon one\n")

    def test_the_native_bridges_plist_names_it_apart_from_electrons(self):
        snapshot = self.make(native=self.native_binary())
        with open(os.path.join(snapshot, "Armada.app", "Contents", "Info.plist"), "rb") as f:
            info = plistlib.load(f)
        self.assertEqual(info["CFBundleName"], "Armada")
        self.assertEqual(info["CFBundleDisplayName"], "Armada")
        self.assertEqual(info["CFBundleExecutable"], "Armada")
        self.assertEqual(info["CFBundleIconFile"], "AppIcon")
        self.assertEqual(info["LSMinimumSystemVersion"], "12.0")
        self.assertNotIn(info["CFBundleIdentifier"], ("com.github.Electron", "dev.armada.bridge"))

    def test_the_native_bridge_is_signed_ad_hoc_and_the_signature_holds_after_the_rename(self):
        snapshot = self.make(native=self.native_binary())
        app = os.path.join(snapshot, "Armada.app")
        verified = subprocess.run(["codesign", "--verify", "--strict", app], capture_output=True, text=True)
        self.assertEqual(verified.returncode, 0, verified.stderr)
        shown = subprocess.run(["codesign", "-dv", app], capture_output=True, text=True)
        self.assertIn("Signature=adhoc", shown.stderr)

    def test_a_build_without_a_native_binary_has_no_app_and_no_native_key(self):
        snapshot = self.make()
        self.assertFalse(os.path.exists(os.path.join(snapshot, "Armada.app")))
        self.assertNotIn("native", self.toml(snapshot)[0])

    def test_the_native_key_follows_the_binary_and_the_icon(self):
        first = self.toml(self.make(native=self.native_binary()))[0]["native"]
        self.assertRegex(first, r"^[0-9a-f]{16}$")
        self.commit()
        self.assertEqual(self.toml(self.make(native=self.native_binary()))[0]["native"], first)
        other = self.native_binary("other", "/usr/bin/false")
        self.commit()
        changed = self.toml(self.make(native=other))[0]["native"]
        self.assertNotEqual(changed, first)
        self.write(os.path.join(self.src, "packages", "brand", "macos", "AppIcon.icns"), "icon two\n")
        self.commit()
        self.assertNotIn(self.toml(self.make(native=other))[0]["native"], (first, changed))

    def test_a_native_binary_that_is_not_there_stops_the_keep(self):
        done = self.run_keep("make", self.src, self.armada, "main", "proto1", os.path.join(self.tmp, "nope"))
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("is not a file", done.stderr)
        self.assertFalse(os.path.exists(self.builds) and [n for n in os.listdir(self.builds) if not n.startswith(".")])

    def test_the_brand_icon_missing_stops_the_keep_of_a_native_bridge(self):
        os.remove(os.path.join(self.src, "packages", "brand", "macos", "AppIcon.icns"))
        done = self.run_keep("make", self.src, self.armada, "main", "proto1", self.native_binary())
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("AppIcon.icns is missing", done.stderr)

    def test_make_touches_no_link(self):
        self.make()
        self.assertEqual(self.link("current"), None)
        self.assertEqual(self.link("previous"), None)

    def test_build_toml_for_a_main_build(self):
        commit = self.git("rev-parse", "HEAD")
        snapshot = self.make(protocol="abc123")
        flat, jobs = self.toml(snapshot)
        self.assertEqual(sorted(flat), ["bridge", "bridge_shell", "built_at", "commit", "electron", "main",
                                        "node_modules", "protocol_id", "source", "tree"])
        self.assertEqual(jobs, [])
        self.assertRegex(flat["built_at"], r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$")
        self.assertEqual(flat["commit"], commit)
        self.assertEqual(flat["source"], "main")
        self.assertEqual(flat["tree"], os.path.abspath(self.src))
        self.assertEqual(flat["main"], commit)
        self.assertEqual(flat["protocol_id"], "abc123")
        self.assertEqual(flat["electron"], "40.1.2")
        self.assertEqual(flat["node_modules"], self.lock_of("lock one\n"))
        self.assertRegex(flat["bridge"], r"^[0-9a-f]{16}$")
        self.assertRegex(flat["bridge_shell"], r"^[0-9a-f]{16}$")
        self.assertNotEqual(flat["bridge"], flat["bridge_shell"])

    def test_electron_is_left_out_when_the_tree_has_no_version(self):
        os.remove(os.path.join(self.src, "apps", "desktop", "node_modules", "electron", "dist", "version"))
        self.assertNotIn("electron", self.toml(self.make())[0])

    def test_the_bridge_keys_follow_what_is_in_out(self):
        out = os.path.join(self.src, "apps", "desktop", "out")
        first = self.toml(self.make())[0]
        self.commit()
        self.write(os.path.join(out, "renderer", "index.html"), "<html>changed\n")
        renderer = self.toml(self.make())[0]
        self.assertNotEqual(renderer["bridge"], first["bridge"])
        self.assertEqual(renderer["bridge_shell"], first["bridge_shell"], "the renderer is not the shell")
        self.commit()
        self.write(os.path.join(out, "main", "index.js"), "main changed\n")
        shell = self.toml(self.make())[0]
        self.assertNotEqual(shell["bridge_shell"], renderer["bridge_shell"])

    def test_the_restart_stamps_are_not_in_the_build_or_its_key(self):
        out = os.path.join(self.src, "apps", "desktop", "out")
        first = self.make()
        self.assertEqual(sorted(os.listdir(os.path.join(first, "bridge", "out"))), ["main", "preload", "renderer"])
        self.commit()
        self.write(os.path.join(out, ".armada-restart-stamp"), "a later stamp\n")
        self.assertEqual(self.toml(self.make())[0]["bridge"], self.toml(first)[0]["bridge"])

    def test_the_voice_helper_is_kept_when_it_was_built(self):
        self.write(os.path.join(self.src, "apps", "desktop", "native", "voice", "build", "Armada Voice.app", "v"), "voice\n")
        snapshot = self.make()
        self.assertEqual(self.read(os.path.join(snapshot, "bridge", "native", "voice", "build", "Armada Voice.app", "v")), "voice\n")

    def test_launch_bridge_names_the_clones_armada_binary_and_this_snapshot(self):
        snapshot = self.make()
        with open(os.path.join(snapshot, "launch-bridge")) as f:
            lines = f.read().splitlines()
        electron = os.path.join(self.support, "electron", "40.1.2", "Armada Classic.app", "Contents", "MacOS", "Armada Classic")
        self.assertEqual(lines, ["#!/bin/sh", f'exec "{electron}" "{snapshot}/bridge"'])
        self.assertTrue(os.path.isfile(electron))
        self.assertNotIn(self.src, "\n".join(lines))

    def test_electron_is_kept_once_per_version_whatever_the_lockfile(self):
        first = self.make()
        self.write(os.path.join(self.src, "pnpm-lock.yaml"), "lock two\n")
        self.commit()
        second = self.make()
        self.assertEqual(os.listdir(os.path.join(self.support, "electron")), ["40.1.2"])
        self.assertEqual(os.listdir(os.path.join(self.support, "electron", "40.1.2")), ["Armada Classic.app"])
        for snapshot in (first, second):
            self.assertIn(os.path.join(self.support, "electron", "40.1.2", "Armada Classic.app"),
                          self.read(snapshot, "launch-bridge"))

    def test_page_reload_is_written_only_where_bridges_main_holds_the_mark(self):
        self.assertNotIn("page_reload", self.toml(self.make())[0])
        self.write(os.path.join(self.src, "apps", "desktop", "out", "main", "index.js"),
                   'log("armada:page-reload")\n')
        self.commit()
        self.assertIs(self.toml(self.make())[0]["page_reload"], True)

    def test_launch_bridge_falls_back_to_the_real_electron_in_the_clone(self):
        shutil.rmtree(os.path.join(self.src, "node_modules", ".cache"))
        real = os.path.join(self.src, "node_modules", ".pnpm", "electron-real")
        self.write(os.path.join(real, "dist", "version"), "40.1.2\n")
        self.write(os.path.join(real, "dist", "Electron.app", "Contents", "MacOS", "Electron"), "electron\n", mode=0o755)
        shutil.rmtree(os.path.join(self.src, "apps", "desktop", "node_modules", "electron"))
        os.symlink("../../../node_modules/.pnpm/electron-real", os.path.join(self.src, "apps", "desktop", "node_modules", "electron"))
        snapshot = self.make()
        with open(os.path.join(snapshot, "launch-bridge")) as f:
            exec_line = f.read().splitlines()[1]
        # Kept by its version, like the Armada Classic copy.
        electron = os.path.join(self.support, "electron", "40.1.2", "Electron.app", "Contents", "MacOS", "Electron")
        self.assertEqual(exec_line, f'exec "{electron}" "{snapshot}/bridge"')
        self.assertTrue(os.path.isfile(electron))

    def test_the_armada_classic_electron_is_made_in_the_tree_before_it_is_cloned(self):
        shutil.rmtree(os.path.join(self.src, "node_modules", ".cache"))
        self.write(os.path.join(self.src, ".stub-makes-the-copy"), "")
        snapshot = self.make()
        with open(self.electron_calls) as f:
            self.assertEqual(f.read().split(), [self.src])
        self.assertIn("/Armada Classic.app/Contents/MacOS/Armada Classic", self.read(os.path.join(snapshot, "launch-bridge")))

    def test_a_node_modules_kept_with_the_old_copy_gets_the_classic_one(self):
        self.make()
        cache = os.path.join(self.modules, self.lock_of("lock one\n"), "node_modules", ".cache", "armada-bridge")
        shutil.rmtree(cache)
        self.write(os.path.join(cache, "Armada.app", "Contents", "MacOS", "Armada"), "old\n", mode=0o755)
        self.write(os.path.join(self.src, ".stub-makes-the-copy"), "")
        self.commit()
        snapshot = self.make()
        self.assertEqual(os.listdir(cache), ["Armada Classic.app"])
        self.assertIn("/Armada Classic.app/Contents/MacOS/Armada Classic", self.read(os.path.join(snapshot, "launch-bridge")))

    def test_the_relative_pnpm_link_resolves_inside_the_clone(self):
        snapshot = self.make()
        shutil.rmtree(self.src)
        reached = os.path.realpath(os.path.join(snapshot, "bridge", "node_modules", "x", "index.js"))
        self.assertEqual(self.read(reached), "x\n")
        self.assertTrue(reached.startswith(os.path.realpath(os.path.join(self.modules, self.lock_of("lock one\n"))) + os.sep))

    def test_a_second_build_with_the_same_lockfile_reuses_the_node_modules_folder(self):
        first = self.make()
        lock = os.path.join(self.modules, self.lock_of("lock one\n"))
        self.write(os.path.join(lock, "marker"), "kept\n")
        before = os.stat(lock).st_ino
        self.commit()
        second = self.make()
        self.assertNotEqual(first, second)
        self.assertEqual(os.stat(lock).st_ino, before)
        self.assertEqual(self.read(os.path.join(lock, "marker")), "kept\n")
        self.assertEqual(os.listdir(self.modules), [self.lock_of("lock one\n")])
        self.assertEqual(os.readlink(os.path.join(second, "bridge", "node_modules")), os.readlink(os.path.join(first, "bridge", "node_modules")))

    def test_a_changed_lockfile_makes_another_node_modules_folder(self):
        first = self.make()
        self.write(os.path.join(self.src, "pnpm-lock.yaml"), "lock two\n")
        self.commit()
        second = self.make()
        self.assertEqual(sorted(os.listdir(self.modules)), sorted([self.lock_of("lock one\n"), self.lock_of("lock two\n")]))
        self.assertEqual(self.toml(first)[0]["node_modules"], self.lock_of("lock one\n"))
        self.assertEqual(self.toml(second)[0]["node_modules"], self.lock_of("lock two\n"))

    @unittest.skipIf(os.geteuid() == 0, "root reads what mode 000 hides")
    def test_a_clone_that_fails_leaves_no_node_modules_folder(self):
        hidden = os.path.join(self.src, "apps", "desktop", "node_modules", "electron")
        os.chmod(hidden, 0)
        self.addCleanup(os.chmod, hidden, 0o755)
        done = self.run_keep("make", self.src, self.armada, "main", "p")
        self.assertNotEqual(done.returncode, 0)
        self.assertEqual(os.listdir(self.modules), [])
        self.assertEqual(os.listdir(self.builds), [])

    def test_a_preview_build_records_main_and_its_jobs(self):
        commit = self.git("rev-parse", "HEAD")
        self.write(os.path.join(self.support, "preview-held"), f"{commit}\nmain-commit\nfeat/a\ttipa\nfeat/b\ttipb\n")
        flat, jobs = self.toml(self.make(source="preview"))
        self.assertEqual(flat["source"], "preview")
        self.assertEqual(flat["commit"], commit)
        self.assertEqual(flat["main"], "main-commit")
        self.assertEqual(jobs, [{"branch": "feat/a", "tip": "tipa"}, {"branch": "feat/b", "tip": "tipb"}])

    def test_a_preview_whose_held_file_is_for_another_commit_records_neither(self):
        self.write(os.path.join(self.support, "preview-held"), "someone-elses-commit\nmain-commit\nfeat/a\ttipa\n")
        flat, jobs = self.toml(self.make(source="preview"))
        self.assertNotIn("main", flat)
        self.assertEqual(jobs, [])

    def test_a_main_build_records_no_jobs_whatever_preview_held(self):
        commit = self.git("rev-parse", "HEAD")
        self.write(os.path.join(self.support, "preview-held"), f"{commit}\nmain-commit\nfeat/a\ttipa\n")
        flat, jobs = self.toml(self.make(source="main"))
        self.assertEqual(flat["main"], commit)
        self.assertEqual(jobs, [])

    # point

    def test_the_first_point_sets_current_and_no_previous(self):
        snapshot = self.make()
        self.point(snapshot)
        self.assertEqual(self.link("current"), os.path.basename(snapshot))
        self.assertIsNone(self.link("previous"))

    def test_point_makes_the_old_current_the_previous(self):
        first = self.make()
        self.point(first)
        self.commit()
        second = self.make()
        self.point(second)
        self.assertEqual(self.link("current"), os.path.basename(second))
        self.assertEqual(self.link("previous"), os.path.basename(first))
        self.assertEqual(os.path.realpath(os.path.join(self.builds, "current")), os.path.realpath(second))
        self.assertEqual(sorted(f for f in os.listdir(self.builds) if f.startswith(".")), [])

    def test_pointing_at_current_again_leaves_previous_alone(self):
        first = self.make()
        self.point(first)
        self.commit()
        second = self.make()
        self.point(second)
        self.point(second)
        self.assertEqual(self.link("current"), os.path.basename(second))
        self.assertEqual(self.link("previous"), os.path.basename(first))

    def test_point_refuses_a_folder_outside_the_builds(self):
        os.makedirs(self.builds)
        done = self.run_keep("point", self.tmp)
        self.assertNotEqual(done.returncode, 0)
        self.assertIsNone(self.link("current"))

    def test_seven_builds_prune_to_the_newest_five(self):
        names = [f"2026-01-0{day}T000000Z-abcdef{day}" for day in range(1, 8)]
        for name in names:
            self.fake_build(name)
        self.set_link("current", names[6])
        self.set_link("previous", names[5])
        self.point(os.path.join(self.builds, names[6]))
        self.assertEqual(self.folders(), names[2:])

    def test_an_old_previous_is_kept_past_the_five(self):
        names = [f"2026-01-0{day}T000000Z-abcdef{day}" for day in range(1, 8)]
        for name in names:
            self.fake_build(name)
        self.set_link("current", names[6])
        self.set_link("previous", names[0])
        self.point(os.path.join(self.builds, names[6]))
        self.assertEqual(self.folders(), [names[0]] + names[2:])
        self.assertEqual(self.link("previous"), names[0])

    def test_an_old_current_is_kept_past_the_five_when_pointing_at_it(self):
        names = [f"2026-01-0{day}T000000Z-abcdef{day}" for day in range(1, 8)]
        for name in names:
            self.fake_build(name)
        self.set_link("current", names[6])
        self.set_link("previous", names[5])
        self.point(os.path.join(self.builds, names[0]))
        self.assertEqual(self.link("current"), names[0])
        self.assertEqual(self.link("previous"), names[6])
        self.assertEqual(self.folders(), [names[0]] + names[2:])

    def test_a_node_modules_folder_no_build_names_is_deleted(self):
        names = [f"2026-01-0{day}T000000Z-abcdef{day}" for day in range(1, 4)]
        for name in names:
            self.fake_build(name)
        os.makedirs(os.path.join(self.modules, "bbbbbbbbbbbbbbbb", "node_modules"))
        self.set_link("current", names[2])
        self.point(os.path.join(self.builds, names[2]))
        self.assertEqual(os.listdir(self.modules), [LOCK])

    def test_an_electron_version_no_build_names_is_deleted(self):
        names = [f"2026-01-0{day}T000000Z-abcdef{day}" for day in range(1, 3)]
        for name in names:
            self.fake_build(name)
        with open(os.path.join(self.builds, names[1], "build.toml"), "a") as f:
            f.write('electron = "40.1.2"\n')
        for version in ("40.1.2", "39.0.0"):
            os.makedirs(os.path.join(self.support, "electron", version, "Armada Classic.app"))
        self.set_link("current", names[1])
        self.point(os.path.join(self.builds, names[1]))
        self.assertEqual(os.listdir(os.path.join(self.support, "electron")), ["40.1.2"])

    def test_a_build_a_running_bridge_was_opened_from_is_kept_past_the_five(self):
        names = [f"2026-01-0{day}T000000Z-abcdef{day}" for day in range(1, 8)]
        for name in names:
            self.fake_build(name)
        stub = os.path.join(self.tmp, "stub-bin")
        self.write(os.path.join(stub, "ps"),
                   f'#!/bin/sh\necho "/x/Armada Classic.app/Contents/MacOS/Armada Classic {self.builds}/{names[0]}/bridge"\n', mode=0o755)
        self.set_link("current", names[-1])
        done = self.run_keep("point", os.path.join(self.builds, names[-1]), path=stub)
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(self.folders(), [names[0], *names[2:]])

    def test_a_node_modules_folder_only_a_pruned_build_named_is_deleted(self):
        old = [f"2026-01-0{day}T000000Z-abcdef{day}" for day in range(1, 3)]
        recent = [f"2026-01-0{day}T000000Z-abcdef{day}" for day in range(3, 8)]
        for name in old:
            self.fake_build(name, "cccccccccccccccc")
        for name in recent:
            self.fake_build(name, LOCK)
        self.set_link("current", recent[-1])
        self.point(os.path.join(self.builds, recent[-1]))
        self.assertEqual(self.folders(), recent)
        self.assertEqual(os.listdir(self.modules), [LOCK])


if __name__ == "__main__":
    unittest.main()
