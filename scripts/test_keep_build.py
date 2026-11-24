#!/usr/bin/env python3
#
# `scripts/keep-build` against a scratch HOME and a fake tree, never the real ones.
#
#   python3 scripts/test_keep_build.py

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
# A native Bridge and Fleet's binary are signed as they are kept, and only a Mac has codesign: CI runs this on Linux.
SIGNS = unittest.skipUnless(shutil.which("codesign"), "the native bundle is signed, and only a Mac can")


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
        self.src = os.path.join(self.tmp, "tree")
        os.makedirs(self.home)
        # keep-build finds `native-app` beside itself, so a copy sits beside a copy of it.
        scripts = os.path.join(self.tmp, "scripts")
        os.makedirs(scripts)
        self.keep = os.path.join(scripts, "keep-build")
        shutil.copy2(KEEP, self.keep)
        shutil.copy2(NATIVE_APP, os.path.join(scripts, "native-app"))
        shutil.copy2(os.path.join(HERE, "scripts", "signing-identity"), os.path.join(scripts, "signing-identity"))
        # `native-app` takes the notification tones from beside its own folder.
        shutil.copytree(os.path.join(HERE, "apps", "desktop", "sounds"), os.path.join(self.tmp, "apps", "desktop", "sounds"))
        self.armada = os.path.join(self.tmp, "armada")
        self.write(self.armada, "#!/bin/sh\necho armada\n", mode=0o755)
        # Launch Services is the Mac's own, so every run gets a stand-in: it prints `lsregister.dump` and logs the rest.
        self.lsregister_calls = os.path.join(self.tmp, "lsregister.calls")
        self.lsregister_dump = os.path.join(self.tmp, "lsregister.dump")
        self.lsregister = os.path.join(self.tmp, "lsregister")
        self.write(self.lsregister, (
            "#!/bin/sh\n"
            f'if [ "$1" = -dump ]; then cat "{self.lsregister_dump}" 2>/dev/null; exit 0; fi\n'
            f'echo "$1 $2" >> "{self.lsregister_calls}"\n'), mode=0o755)
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
        """The parts of a checkout that a kept build reads."""
        s = self.src
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
        env = {**os.environ, "HOME": self.home, "ARMADA_LSREGISTER": self.lsregister}
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

    def fake_build(self, name, record=""):
        """A kept build of an earlier day: only what `point` reads. `record` is its `build.toml`, as an earlier keep wrote it."""
        self.write(os.path.join(self.builds, name, "build.toml"), record)
        self.write(os.path.join(self.builds, name, "Armada Fleet.app", "Contents", "MacOS", "armada"), f"fleet of {name}\n")

    def set_link(self, name, to):
        os.makedirs(self.builds, exist_ok=True)
        os.symlink(to, os.path.join(self.builds, name))

    def folders(self):
        return sorted(f for f in os.listdir(self.builds) if f not in ("current", "previous"))

    # make

    def test_the_snapshot_has_the_layout(self):
        snapshot = self.make()
        self.assertEqual(os.path.dirname(snapshot), self.builds)
        self.assertRegex(os.path.basename(snapshot), r"^\d{4}-\d{2}-\d{2}T\d{6}Z-" + self.git("rev-parse", "HEAD")[:7] + "$")
        self.assertEqual(sorted(os.listdir(snapshot)),
                         ["Armada Fleet.app", "armada", "bridge-launch", "build.toml"])
        self.assertEqual(os.readlink(os.path.join(snapshot, "armada")), "Armada Fleet.app/Contents/MacOS/armada")
        self.assertEqual(self.read(os.path.join(snapshot, "armada")), "#!/bin/sh\necho armada\n")
        self.assertTrue(os.access(os.path.join(snapshot, "armada"), os.X_OK))
        self.assertTrue(os.access(os.path.join(snapshot, "bridge-launch"), os.X_OK))

    def native_binary(self, name="native", source="/usr/bin/true"):
        """A real, signed Mach-O to stand in for the cargo binary, since the bundle is signed."""
        path = os.path.join(self.tmp, name)
        shutil.copyfile(source, path)
        os.chmod(path, 0o755)
        return path

    @SIGNS
    def test_the_native_bridge_is_kept_as_armada_app(self):
        snapshot = self.make(native=self.native_binary())
        self.assertEqual(sorted(os.listdir(snapshot)),
                         ["Armada Fleet.app", "Armada.app", "armada", "bridge-launch", "build.toml"])
        app = os.path.join(snapshot, "Armada.app", "Contents")
        self.assertEqual(sorted(os.listdir(app)), ["Info.plist", "MacOS", "Resources", "_CodeSignature"])
        self.assertEqual(os.listdir(os.path.join(app, "MacOS")), ["Armada"])
        self.assertTrue(os.access(os.path.join(app, "MacOS", "Armada"), os.X_OK))
        self.assertEqual(self.read(app, "Resources", "AppIcon.icns"), "icon one\n")

    @SIGNS
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

    @SIGNS
    def test_the_native_bridge_is_signed_and_the_signature_holds_after_the_rename(self):
        snapshot = self.make(native=self.native_binary())
        app = os.path.join(snapshot, "Armada.app")
        verified = subprocess.run(["codesign", "--verify", "--strict", app], capture_output=True, text=True)
        self.assertEqual(verified.returncode, 0, verified.stderr)
        shown = subprocess.run(["codesign", "-dv", app], capture_output=True, text=True)
        # Ad hoc, or with this Mac's certificate where `scripts/signing-identity` made one.
        self.assertTrue("Signature=adhoc" in shown.stderr or "Authority=Armada Local Signing" in shown.stderr, shown.stderr)

    def test_fleet_is_kept_in_a_bundle_named_dev_armada_fleet_with_no_dock_icon(self):
        snapshot = self.make()
        with open(os.path.join(snapshot, "Armada Fleet.app", "Contents", "Info.plist"), "rb") as f:
            info = plistlib.load(f)
        self.assertEqual(info["CFBundleIdentifier"], "dev.armada.fleet")
        self.assertEqual(info["CFBundleExecutable"], "armada")
        self.assertIs(info["LSUIElement"], True)
        self.assertTrue(os.access(os.path.join(snapshot, "Armada Fleet.app", "Contents", "MacOS", "armada"), os.X_OK))

    @SIGNS
    def test_fleets_bundle_is_signed_as_dev_armada_fleet_and_its_source_is_left_alone(self):
        self.armada = self.native_binary("armada-built")
        with open(self.armada, "rb") as f:
            built = f.read()
        snapshot = self.make()
        kept = os.path.join(snapshot, "Armada Fleet.app")
        verified = subprocess.run(["codesign", "--verify", "--strict", kept], capture_output=True, text=True)
        self.assertEqual(verified.returncode, 0, verified.stderr)
        shown = subprocess.run(["codesign", "-dv", kept], capture_output=True, text=True)
        self.assertIn("Identifier=dev.armada.fleet", shown.stderr)
        self.assertTrue("Signature=adhoc" in shown.stderr or "Authority=Armada Local Signing" in shown.stderr, shown.stderr)
        with open(self.armada, "rb") as f:
            self.assertEqual(f.read(), built)

    def test_a_build_without_a_native_binary_has_no_app_and_no_native_key(self):
        snapshot = self.make()
        self.assertFalse(os.path.exists(os.path.join(snapshot, "Armada.app")))
        self.assertNotIn("native", self.toml(snapshot)[0])

    @SIGNS
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
        self.assertEqual(sorted(flat), ["built_at", "commit", "main", "protocol_id", "source", "tree"])
        self.assertEqual(jobs, [])
        self.assertRegex(flat["built_at"], r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$")
        self.assertEqual(flat["commit"], commit)
        self.assertEqual(flat["source"], "main")
        self.assertEqual(flat["tree"], os.path.abspath(self.src))
        self.assertEqual(flat["main"], commit)
        self.assertEqual(flat["protocol_id"], "abc123")

    @unittest.skipIf(os.geteuid() == 0, "root reads what mode 000 hides")
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

    def test_no_built_bridge_or_node_modules_is_needed_and_none_is_kept(self):
        # The tree here has no `apps/desktop/out`, no `node_modules` and no lockfile: Armada Classic is gone.
        snapshot = self.make()
        self.assertFalse(os.path.exists(os.path.join(snapshot, "bridge")))
        self.assertFalse(os.path.exists(os.path.join(snapshot, "launch-bridge")))
        self.assertFalse(os.path.exists(os.path.join(self.support, "node_modules")))
        self.assertFalse(os.path.exists(os.path.join(self.support, "electron")))

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

    def test_point_places_fleet_at_one_path_and_names_its_build(self):
        names = ["2026-01-01T000000Z-abcdef1", "2026-01-02T000000Z-abcdef2"]
        for name in names:
            self.fake_build(name)
        fleet = os.path.join(self.support, "fleet")
        program = os.path.join(fleet, "Armada Fleet.app", "Contents", "MacOS", "armada")
        self.point(os.path.join(self.builds, names[0]))
        self.assertEqual(self.read(program), f"fleet of {names[0]}\n")
        stale = os.path.join(fleet, "Armada Fleet.app", "Contents", "stale")
        self.write(stale, "only the first had it\n")
        with open(program) as running:
            self.point(os.path.join(self.builds, names[1]))
            self.assertEqual(running.read(), f"fleet of {names[0]}\n", "a running Fleet keeps its inode")
        self.assertEqual(self.read(program), f"fleet of {names[1]}\n")
        self.assertFalse(os.path.islink(program))
        self.assertFalse(os.path.exists(stale))
        self.assertEqual(self.read(os.path.join(fleet, "build")), f"{names[1]}\n")
        self.assertEqual(sorted(os.listdir(fleet)), ["Armada Fleet.app", "build"])

    @SIGNS
    def test_the_placed_fleet_keeps_its_signature_as_dev_armada_fleet(self):
        self.armada = self.native_binary("armada-built")
        self.point(self.make())
        placed = os.path.join(self.support, "fleet", "Armada Fleet.app")
        verified = subprocess.run(["codesign", "--verify", "--strict", placed], capture_output=True, text=True)
        self.assertEqual(verified.returncode, 0, verified.stderr)
        shown = subprocess.run(["codesign", "-dv", placed], capture_output=True, text=True)
        self.assertIn("Identifier=dev.armada.fleet", shown.stderr)

    def test_point_refuses_a_build_with_no_fleet_bundle_and_moves_no_link(self):
        name = "2026-01-01T000000Z-abcdef1"
        self.fake_build(name)
        shutil.rmtree(os.path.join(self.builds, name, "Armada Fleet.app"))
        done = self.run_keep("point", os.path.join(self.builds, name))
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("keeps no Armada Fleet.app", done.stderr)
        self.assertIsNone(self.link("current"))

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

    def test_a_build_kept_with_electron_rows_is_pruned_like_any_other(self):
        record = 'bridge = "b"\nbridge_shell = "s"\nelectron = "40.1.2"\nnode_modules = "aaaaaaaaaaaaaaaa"\npage_reload = true\n'
        names = [f"2026-01-0{day}T000000Z-abcdef{day}" for day in range(1, 8)]
        for name in names:
            self.fake_build(name, record)
        self.set_link("current", names[6])
        self.point(os.path.join(self.builds, names[6]))
        self.assertEqual(self.folders(), names[2:])

    # Launch Services

    def registered(self, *paths):
        """What `lsregister -dump` lists, in its own line shape."""
        self.write(self.lsregister_dump, "".join(f"path:                       {p} (0x{n:x})\n" for n, p in enumerate(paths, 1)))

    def lsregister_ran(self):
        try:
            return self.read(self.lsregister_calls).splitlines()
        except OSError:
            return []

    def test_point_unregisters_every_bundle_under_builds_but_currents_app_and_registers_current(self):
        names = [f"2026-01-0{day}T000000Z-abcdef{day}" for day in range(1, 3)]
        for name in names:
            self.fake_build(name)
            self.write(os.path.join(self.builds, name, "Armada.app", "Contents", "MacOS", "Armada"), "app\n")
        builds = os.path.realpath(self.builds)
        current = f"{builds}/{names[1]}/Armada.app"
        gone = f"{builds}/2025-12-01T000000Z-0000000/Armada.app"
        elsewhere = "/private/tmp/armada-red/e2e/out/Armada.app"
        self.registered(
            gone,
            f"{builds}/2025-12-02T000000Z-0000000/Armada.app/Contents/Helpers/Armada Voice.app",
            f"{builds}/{names[0]}/Armada.app",
            f"{builds}/{names[1]}/Armada Fleet.app",
            current,
            f"{current}/Contents/Helpers/Armada Voice.app",
            elsewhere,
            f"{self.support}/fleet/Armada Fleet.app",
        )
        self.set_link("current", names[0])
        self.point(os.path.join(self.builds, names[1]))
        ran = self.lsregister_ran()
        self.assertEqual(sorted(line for line in ran if line.startswith("-u ")), [
            f"-u {gone}",
            f"-u {builds}/2025-12-02T000000Z-0000000/Armada.app/Contents/Helpers/Armada Voice.app",
            f"-u {builds}/{names[0]}/Armada.app",
            f"-u {builds}/{names[1]}/Armada Fleet.app",
        ])
        self.assertEqual([line for line in ran if line.startswith("-f ")],
                         [f"-f {current}", f"-f {self.support}/fleet/Armada Fleet.app"])
        self.assertTrue(all(elsewhere not in line for line in ran), "a bundle outside builds is never touched")

    def test_the_bundle_the_running_native_bridge_holds_its_lock_from_stays_registered(self):
        names = [f"2026-01-0{day}T000000Z-abcdef{day}" for day in range(1, 3)]
        for name in names:
            self.fake_build(name)
        builds = os.path.realpath(self.builds)
        running = f"{builds}/{names[0]}/Armada.app"
        self.write(os.path.join(running, "Contents", "MacOS", "Armada"), "app\n")
        self.write(os.path.join(self.support, "bridge-native.lock"), f"{os.getpid()}\n{running}/Contents/MacOS/Armada\ntrue\n")
        self.registered(running, f"{running}/Contents/Helpers/Armada Voice.app")
        self.set_link("current", names[0])
        self.point(os.path.join(self.builds, names[1]))
        self.assertEqual([line for line in self.lsregister_ran() if line.startswith("-u ")], [])

    def test_a_lock_record_whose_pid_is_gone_keeps_nothing_registered(self):
        names = [f"2026-01-0{day}T000000Z-abcdef{day}" for day in range(1, 3)]
        for name in names:
            self.fake_build(name)
        builds = os.path.realpath(self.builds)
        old = f"{builds}/{names[0]}/Armada.app"
        dead = subprocess.Popen(["true"])
        dead.wait()
        self.write(os.path.join(self.support, "bridge-native.lock"), f"{dead.pid}\n{old}/Contents/MacOS/Armada\ntrue\n")
        self.registered(old)
        self.set_link("current", names[0])
        self.point(os.path.join(self.builds, names[1]))
        self.assertIn(f"-u {old}", self.lsregister_ran())


if __name__ == "__main__":
    unittest.main()
