#!/usr/bin/env python3
#
# A restart outliving the terminal that started it, the native Bridge it reopens
# when its running protocol is not the new Fleet's, and the Armada Classic job and app
# it removes. Nothing here builds or touches launchd: the restart scripts run under a
# scratch HOME against stubs, and `scripts/running-bridge` reads stub `launchctl`, `ps` and `lsof`.
#
#   python3 scripts/test_restart_detach.py
#
# `ARMADA_TEST_SCRIPTS` names another `scripts/` folder to run against, which is
# how a test is shown red: the folder as it was before the fix.

import importlib.machinery
import importlib.util
import os
import pty
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import unittest

HERE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SCRIPTS = os.environ.get("ARMADA_TEST_SCRIPTS") or os.path.join(HERE, "scripts")
WAIT = 20

_loader = importlib.machinery.SourceFileLoader("running_bridge", os.path.join(HERE, "scripts", "running-bridge"))
BRIDGE = importlib.util.module_from_spec(importlib.util.spec_from_loader("running_bridge", _loader))
_loader.exec_module(BRIDGE)


def write_executable(path, text):
    with open(path, "w") as f:
        f.write(text)
    os.chmod(path, 0o755)


def says(path, text):
    try:
        with open(path) as f:
            return text in f.read()
    except OSError:
        return False


def until(condition, seconds=WAIT):
    end = time.time() + seconds
    while time.time() < end:
        if condition():
            return True
        time.sleep(0.05)
    return condition()


class Hangup(unittest.TestCase):
    """A script started on a terminal that is closed under it, as Fleet's Run does by stopping Fleet."""

    def setUp(self):
        self.dir = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, self.dir, True)
        self.home = os.path.join(self.dir, "home")
        self.support = os.path.join(self.home, "Library", "Application Support", "Armada")
        os.makedirs(self.support)
        self.log = os.path.join(self.home, "Library", "Logs", "Armada", "restart.log")
        self.marker = os.path.join(self.dir, "marker")
        self.bin = os.path.join(self.dir, "bin")
        os.makedirs(self.bin)
        # A launchctl that touches nothing: a real one would reach the owner's Fleet.
        write_executable(os.path.join(self.bin, "launchctl"), "#!/bin/sh\nexit 1\n")
        self.env = {**os.environ, "HOME": self.home, "PATH": self.bin + os.pathsep + os.environ["PATH"],
                    "ARMADA_FLEET_LABEL": "com.armada.test.detach"}
        self.env.pop("ARMADA_RESTART_DETACHED", None)
        self.env.pop("ARMADA_SPAWNED_BY", None)

    def on_a_terminal(self, argv, cwd, env):
        """Run `argv` with a terminal for stdin, stdout and stderr. Returns (pid, master)."""
        pid, master = pty.fork()
        if pid == 0:
            os.chdir(cwd)
            os.execvpe(argv[0], argv, env)
        return pid, master

    def read_until(self, master, text):
        seen = b""
        end = time.time() + WAIT
        while text.encode() not in seen and time.time() < end:
            try:
                chunk = os.read(master, 4096)
            except OSError:
                break
            if not chunk:
                break
            seen += chunk
        return seen.decode("utf-8", "replace")

    def hang_up(self, pid, master):
        """Close the terminal the run was started on, which hangs up its foreground process group."""
        os.close(master)
        done = []

        def reaped():
            try:
                done.append(os.waitpid(pid, os.WNOHANG))
            except ChildProcessError:
                return True
            return done[-1][0] == pid

        self.assertTrue(until(reaped), "the foreground did not go when its terminal closed")

    def slow_stub(self, name, after):
        """A stub that says `before`, waits for the hangup, then writes the marker and `after` and exits 1."""
        path = os.path.join(self.dir, name)
        write_executable(path, f'#!/bin/sh\necho before\nsleep 2\necho done > "{self.marker}"\necho {after}\nexit 1\n')
        return path

    def test_restart_outlives_the_terminal_that_started_it(self):
        repo = os.path.join(self.dir, "repo")
        os.makedirs(repo)
        for args in (["init", "-q", "-b", "main"], ["config", "user.name", "t"], ["config", "user.email", "t@example.com"]):
            subprocess.run(["git", *args], cwd=repo, check=True, capture_output=True)
        with open(os.path.join(repo, "a.txt"), "w") as f:
            f.write("one\n")
        subprocess.run(["git", "add", "-A"], cwd=repo, check=True, capture_output=True)
        subprocess.run(["git", "commit", "-q", "-m", "start"], cwd=repo, check=True, capture_output=True)
        write_executable(os.path.join(self.dir, "armada"), "#!/bin/sh\nexit 0\n")
        stub = self.slow_stub("restart-stub", "stub-line-after-the-hangup")
        env = {**self.env, "ARMADA_PREVIEW_RESTART": stub, "ARMADA_LAND_ARMADA": os.path.join(self.dir, "armada")}
        pid, master = self.on_a_terminal([sys.executable, os.path.join(SCRIPTS, "preview"), "--restart"], repo, env)
        seen = self.read_until(master, "before")
        self.assertIn("before", seen)
        self.hang_up(pid, master)
        self.assertTrue(until(lambda: os.path.exists(self.marker)),
                        "the restart died with its terminal before it finished: " + seen)
        self.assertTrue(until(lambda: says(self.log, "stub-line-after-the-hangup")))

    def scratch_restart(self):
        """A copy of `scripts/restart` in a repository whose `origin/main` lists no migration, and a tree to build from."""
        root = os.path.join(self.dir, "root")
        os.makedirs(os.path.join(root, "scripts"))
        for name in ("restart", "detach", "running-bridge", "retire-classic"):
            # The folder as it was before the fix has no helpers.
            if os.path.exists(os.path.join(SCRIPTS, name)):
                shutil.copy(os.path.join(SCRIPTS, name), os.path.join(root, "scripts", name))
        legacy = os.path.join(root, "crates", "store", "src", "legacy_migrations.rs")
        os.makedirs(os.path.dirname(legacy))
        with open(legacy, "w") as f:
            f.write('pub(crate) const LEGACY: &[Migration] = &[\n    Migration::additive("schema.v1", crate::schema::V1),\n];\n')
        for args in (["init", "-q", "-b", "main"], ["add", "."],
                     ["-c", "user.name=t", "-c", "user.email=t@example.com", "commit", "-q", "-m", "main"],
                     ["update-ref", "refs/remotes/origin/main", "HEAD"]):
            subprocess.run(["git", *args], cwd=root, check=True, capture_output=True)
        tree = os.path.join(self.dir, "tree")
        os.makedirs(os.path.join(tree, "crates", "armada"))
        os.makedirs(os.path.join(tree, "crates", "store", "src"))
        os.makedirs(os.path.join(tree, "crates", "bridge-native"))
        open(os.path.join(tree, "crates", "armada", "Cargo.toml"), "w").close()
        shutil.copy(legacy, os.path.join(tree, "crates", "store", "src", "legacy_migrations.rs"))
        return os.path.join(root, "scripts", "restart"), tree

    def test_scripts_restart_outlives_its_terminal_too(self):
        script, tree = self.scratch_restart()
        # The build step the hangup lands in, and the step after it, which the restart reaches only if it lived.
        write_executable(os.path.join(self.bin, "cargo"), "#!/bin/sh\necho before\nsleep 2\n")
        write_executable(os.path.join(os.path.dirname(script), "sync-mod"),
                         f'#!/bin/sh\necho done > "{self.marker}"\necho next-step-after-the-hangup\nexit 1\n')
        pid, master = self.on_a_terminal([script, "--from", tree], self.dir, self.env)
        seen = self.read_until(master, "before")
        self.assertIn("before", seen)
        self.hang_up(pid, master)
        self.assertTrue(until(lambda: os.path.exists(self.marker)),
                        "the restart died with its terminal before it finished: " + seen)
        self.assertTrue(until(lambda: says(self.log, "next-step-after-the-hangup")))

    def run_restart(self, argv, cwd, env=None):
        return subprocess.run(argv, cwd=cwd, env=env or self.env, capture_output=True, text=True, timeout=60)

    def test_the_foreground_names_the_log_first_follows_it_and_exits_with_its_code(self):
        script, tree = self.scratch_restart()
        write_executable(os.path.join(self.bin, "cargo"), "#!/bin/sh\necho from-cargo\nexit 7\n")
        done = self.run_restart([script, "--from", tree], self.dir)
        self.assertEqual(done.returncode, 7, done.stdout + done.stderr)
        self.assertEqual(done.stdout.splitlines()[0], f"restart: log {self.log}")
        self.assertIn("from-cargo", done.stdout)
        with open(self.log) as f:
            self.assertIn("from-cargo", f.read())

    def test_the_foreground_follows_from_where_this_run_began(self):
        script, tree = self.scratch_restart()
        os.makedirs(os.path.dirname(self.log))
        with open(self.log, "w") as f:
            f.write("an earlier run said this\n")
        write_executable(os.path.join(self.bin, "cargo"), "#!/bin/sh\necho from-cargo\nexit 1\n")
        done = self.run_restart([script, "--from", tree], self.dir)
        self.assertNotIn("an earlier run", done.stdout)
        with open(self.log) as f:
            self.assertTrue(f.read().startswith("an earlier run said this\n"))

    def test_the_inner_script_does_not_detach_again(self):
        script, tree = self.scratch_restart()
        write_executable(os.path.join(self.bin, "cargo"), "#!/bin/sh\necho from-cargo\nexit 1\n")
        done = self.run_restart([script, "--from", tree], self.dir, {**self.env, "ARMADA_RESTART_DETACHED": "1"})
        self.assertNotIn("restart: log", done.stdout)
        self.assertIn("from-cargo", done.stdout)
        self.assertFalse(os.path.exists(self.log))

    def test_the_detached_run_has_the_marker_and_not_a_fleets_mark(self):
        script, tree = self.scratch_restart()
        record = os.path.join(self.dir, "env")
        write_executable(os.path.join(self.bin, "cargo"),
                         f'#!/bin/sh\necho "${{ARMADA_RESTART_DETACHED:-unset}} ${{ARMADA_SPAWNED_BY:-unset}}" > "{record}"\nexit 1\n')
        self.run_restart([script, "--from", tree], self.dir, {**self.env, "ARMADA_SPAWNED_BY": "123:Mon_Nov_12"})
        with open(record) as f:
            self.assertEqual(f.read().strip(), "1 unset")

    def test_a_relative_from_still_finds_its_tree(self):
        script, tree = self.scratch_restart()
        write_executable(os.path.join(self.bin, "cargo"), "#!/bin/sh\nexit 7\n")
        done = self.run_restart([script, "--from", "tree"], self.dir)
        self.assertEqual(done.returncode, 7, done.stdout + done.stderr)

    def test_a_dry_run_runs_where_it_is_and_says_it_would_detach(self):
        script, tree = self.scratch_restart()
        write_executable(os.path.join(self.bin, "cargo"), "#!/bin/sh\nexit 1\n")
        done = self.run_restart([script, "--from", tree, "--dry-run"], self.dir)
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertNotIn("restart: log", done.stdout)
        self.assertIn("would run detached", done.stdout)
        self.assertIn(self.log, done.stdout)
        self.assertIn("the new Fleet would speak protocol", done.stdout)
        self.assertNotIn("apps/desktop", done.stdout)
        self.assertNotIn("Electron", done.stdout)
        self.assertIn("Armada Classic has left nothing here to remove", done.stdout)

    def test_a_dry_run_says_it_would_remove_classics_job_and_app_where_they_exist(self):
        script, tree = self.scratch_restart()
        write_executable(os.path.join(self.bin, "cargo"), "#!/bin/sh\nexit 1\n")
        plist = os.path.join(self.support, "com.armada.test.detach.bridge.plist")
        app = os.path.join(self.home, "Applications", "Armada Classic.app")
        open(plist, "w").close()
        os.makedirs(app)
        done = self.run_restart([script, "--from", tree, "--dry-run"], self.dir)
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertIn(f"would boot gui/{os.getuid()}/com.armada.test.detach.bridge out if it is loaded and delete {plist}", done.stdout)
        self.assertIn(f"would quit Armada Classic and delete {app}", done.stdout)
        self.assertNotIn("has left nothing", done.stdout)
        self.assertTrue(os.path.exists(plist) and os.path.isdir(app), "a dry run removes nothing")

    def test_a_restart_runs_the_classic_cleanup_after_fleet_is_up_and_before_the_native_bridge(self):
        # The run itself needs a build, a Fleet and launchd, so what is read is where the script calls it.
        with open(os.path.join(SCRIPTS, "restart")) as f:
            text = f.read()
        self.assertEqual(text.count('"$root/scripts/retire-classic"'), 2, "once for the dry run, once for the real one")
        self.assertLess(text.rindex("settle ${old_pid"), text.rindex('"$root/scripts/retire-classic"'))
        self.assertLess(text.rindex('"$root/scripts/retire-classic"'), text.index("native_reopened=false"))
        self.assertFalse(os.path.exists(self.log))


class RunningProtocol(unittest.TestCase):
    """`scripts/running-bridge`: what each Bridge runs, read off its process, against what the new Fleet speaks."""

    OLD, NEW = "1111111111111111", "2222222222222222"

    def setUp(self):
        self.dir = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, self.dir, True)
        self.support = os.path.join(self.dir, "Application Support", "Armada")
        self.builds = os.path.join(self.support, "builds")
        # The build before the last one is the one running; `current` was moved on to the new build
        # by a restart that died, so the two carry the same native key.
        self.old = self.keep("2026-11-12T100000Z-aaaaaaa", self.OLD)
        self.new = self.keep("2026-11-12T110000Z-bbbbbbb", self.NEW)
        os.symlink(os.path.basename(self.new), os.path.join(self.builds, "current"))
        self.service = "gui/501/com.armada.test.native"

    def keep(self, name, protocol):
        path = os.path.join(self.builds, name)
        os.makedirs(os.path.join(path, "Armada.app", "Contents", "MacOS"))
        os.makedirs(os.path.join(path, "bridge"))
        with open(os.path.join(path, "build.toml"), "w") as f:
            f.write(f'built_at = "2026-11-12T10:00:00Z"\nprotocol_id = "{protocol}"\nnative = "same-key"\n')
        return path

    def native_machine(self, binary, loaded=True, child=True):
        """A runner that answers like launchd, `ps` and `lsof` for a native Bridge whose app is `binary`."""
        def runner(argv):
            if argv[0] == "launchctl":
                return "\tstate = running\n\tpid = 100\n" if loaded else None
            if argv[0] == "ps":
                rows = "  100     1 bash bridge-launch /x/Armada.app/Contents/MacOS/Armada\n"
                if child:
                    rows += f"  101   100 {binary}\n"
                return rows
            if argv[0] == "lsof":
                return f"p101\nftxt\nn{binary}\n" if binary else None
            return None
        return runner

    def binary_in(self, build):
        return os.path.join(build, "Armada.app", "Contents", "MacOS", "Armada")

    def test_native_reopens_when_its_running_protocol_differs(self):
        running = BRIDGE.native_protocols(self.support, self.service, self.native_machine(self.binary_in(self.old)))
        self.assertEqual(running, [self.OLD])
        reopen, why = BRIDGE.decide(running, self.NEW)
        self.assertTrue(reopen, why)
        self.assertIn(self.OLD, why)

    def test_native_is_left_alone_when_its_running_protocol_matches(self):
        running = BRIDGE.native_protocols(self.support, self.service, self.native_machine(self.binary_in(self.new)))
        self.assertEqual(running, [self.NEW])
        reopen, why = BRIDGE.decide(running, self.NEW)
        self.assertFalse(reopen, why)

    def test_a_native_job_loaded_and_not_running_stays_closed(self):
        runner = lambda argv: "\tstate = not running\n" if argv[0] == "launchctl" else None
        running = BRIDGE.native_protocols(self.support, self.service, runner)
        self.assertEqual(running, [])
        self.assertFalse(BRIDGE.decide(running, self.NEW)[0])

    def test_a_native_app_that_cannot_be_read_is_reopened(self):
        for runner in (self.native_machine(None), self.native_machine("/elsewhere/Armada.app/Contents/MacOS/Armada"),
                       self.native_machine("x", child=False)):
            running = BRIDGE.native_protocols(self.support, self.service, runner)
            self.assertEqual(running, [BRIDGE.UNREADABLE])
            self.assertTrue(BRIDGE.decide(running, self.NEW)[0])

    def test_the_running_build_is_read_where_the_process_resolved_it_not_through_current(self):
        # `current` now names the new build, but the process runs the old one.
        through_current = os.path.join(self.builds, "current", "Armada.app", "Contents", "MacOS", "Armada")
        self.assertEqual(BRIDGE.build_protocol(self.support, through_current), self.NEW)
        self.assertEqual(BRIDGE.build_protocol(self.support, self.binary_in(self.old)), self.OLD)

    def test_the_command_reads_the_live_tools(self):
        """End to end through the command, with `launchctl`, `ps` and `lsof` on the PATH."""
        bin = os.path.join(self.dir, "bin")
        os.makedirs(bin)
        binary = self.binary_in(self.old)
        write_executable(os.path.join(bin, "launchctl"), "#!/bin/sh\nprintf '\\tpid = 100\\n'\n")
        write_executable(os.path.join(bin, "ps"), f"#!/bin/sh\necho '  100     1 bash bridge-launch'\necho '  101   100 {binary}'\n")
        write_executable(os.path.join(bin, "lsof"), f"#!/bin/sh\nprintf 'p101\\nftxt\\nn{binary}\\n'\n")
        env = {**os.environ, "PATH": bin + os.pathsep + os.environ["PATH"]}
        said = []
        for want in (self.NEW, self.OLD):
            done = subprocess.run([os.path.join(HERE, "scripts", "running-bridge"), "native", "--support", self.support,
                                   "--service", self.service, "--want", want], env=env, capture_output=True, text=True)
            said.append(done.stdout.split("\t")[0])
        self.assertEqual(said, ["reopen", "leave"])


class RetireClassic(unittest.TestCase):
    """`scripts/retire-classic`: Armada Classic's launchd job, plist and installed app go, once, and a second run changes nothing."""

    LABEL = "com.armada.test.retire"

    def setUp(self):
        self.dir = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, self.dir, True)
        self.home = os.path.join(self.dir, "home")
        self.support = os.path.join(self.home, "Library", "Application Support", "Armada")
        os.makedirs(self.support)
        self.plist = os.path.join(self.support, f"{self.LABEL}.bridge.plist")
        self.native_plist = os.path.join(self.support, f"{self.LABEL}.native.plist")
        self.apps = os.path.join(self.home, "Applications")
        self.loaded = os.path.join(self.dir, "loaded")
        self.calls = os.path.join(self.dir, "calls")
        bin = os.path.join(self.dir, "bin")
        os.makedirs(bin)
        # A launchd that holds the one job while `loaded` exists, and a Finder that quits nothing; real ones would reach the owner's.
        write_executable(os.path.join(bin, "launchctl"),
                         f'#!/bin/sh\necho "launchctl $@" >> "{self.calls}"\n'
                         f'case "$1" in print) [ -f "{self.loaded}" ] ;; bootout) rm -f "{self.loaded}" ;; esac\n')
        write_executable(os.path.join(bin, "osascript"), f'#!/bin/sh\necho "osascript $@" >> "{self.calls}"\n')
        self.env = {**os.environ, "HOME": self.home, "PATH": bin + os.pathsep + os.environ["PATH"], "ARMADA_FLEET_LABEL": self.LABEL}

    def classic_left_behind(self):
        open(self.plist, "w").close()
        open(self.loaded, "w").close()
        os.makedirs(os.path.join(self.apps, "Armada Classic.app", "Contents"))

    def run_retire(self, *args):
        return subprocess.run([os.path.join(SCRIPTS, "retire-classic"), *args], env=self.env, capture_output=True, text=True, timeout=60)

    def said(self):
        try:
            with open(self.calls) as f:
                return f.read().splitlines()
        except OSError:
            return []

    def test_the_job_the_plist_and_the_app_are_removed(self):
        self.classic_left_behind()
        done = self.run_retire()
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertIn(f"launchctl bootout gui/{os.getuid()}/{self.LABEL}.bridge", self.said())
        self.assertFalse(os.path.exists(self.loaded), "the job is out of launchd")
        self.assertFalse(os.path.exists(self.plist))
        self.assertFalse(os.path.exists(os.path.join(self.apps, "Armada Classic.app")))
        self.assertIn('osascript -e quit app "Armada Classic"', self.said())

    def test_the_electron_runtime_and_node_modules_kept_for_classic_are_removed(self):
        for kept in ("electron", "node_modules"):
            os.makedirs(os.path.join(self.support, kept, "inner"))
        open(self.native_plist, "w").close()
        done = self.run_retire()
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertFalse(os.path.exists(os.path.join(self.support, "electron")))
        self.assertFalse(os.path.exists(os.path.join(self.support, "node_modules")))
        self.assertTrue(os.path.exists(self.native_plist), "the native app's job is not Classic's")

    def test_a_second_run_finds_nothing_and_touches_nothing(self):
        self.classic_left_behind()
        self.run_retire()
        before = self.said()
        done = self.run_retire()
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertEqual(done.stdout + done.stderr, "")
        self.assertEqual([c for c in self.said()[len(before):] if "bootout" in c or "osascript" in c], [])

    def test_a_plist_left_with_no_job_loaded_is_still_removed(self):
        open(self.plist, "w").close()
        self.assertEqual(self.run_retire().returncode, 0)
        self.assertFalse(os.path.exists(self.plist))

    def test_a_job_loaded_with_no_plist_is_still_booted_out(self):
        open(self.loaded, "w").close()
        self.assertEqual(self.run_retire().returncode, 0)
        self.assertFalse(os.path.exists(self.loaded))

    def test_the_native_bridge_and_its_app_are_left_alone(self):
        self.classic_left_behind()
        open(self.native_plist, "w").close()
        os.makedirs(os.path.join(self.apps, "Armada.app"))
        self.run_retire()
        self.assertTrue(os.path.exists(self.native_plist))
        self.assertTrue(os.path.isdir(os.path.join(self.apps, "Armada.app")))
        self.assertEqual([c for c in self.said() if "bootout" in c], [f"launchctl bootout gui/{os.getuid()}/{self.LABEL}.bridge"])

    def test_a_dry_run_says_what_it_would_remove_and_removes_nothing(self):
        self.classic_left_behind()
        done = self.run_retire("--dry-run")
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertIn(f"would boot gui/{os.getuid()}/{self.LABEL}.bridge out", done.stdout)
        self.assertIn(f"would quit Armada Classic and delete {self.apps}/Armada Classic.app", done.stdout)
        self.assertTrue(os.path.exists(self.plist) and os.path.exists(self.loaded))
        self.assertTrue(os.path.isdir(os.path.join(self.apps, "Armada Classic.app")))
        self.assertEqual([c for c in self.said() if "bootout" in c or "osascript" in c], [])

    def test_a_dry_run_with_nothing_there_says_so(self):
        done = self.run_retire("--dry-run")
        self.assertEqual(done.stdout.strip(), "Armada Classic has left nothing here to remove")

    def test_an_argument_it_does_not_know_is_refused(self):
        self.assertNotEqual(self.run_retire("--now").returncode, 0)


if __name__ == "__main__":
    unittest.main()
