#!/usr/bin/env python3
#
# `scripts/preview` against a throwaway repository and a stub `armada` that
# answers `worktree --status` from a file. Git only:
# nothing here builds, runs a Check or touches Fleet.
#
#   python3 scripts/test_preview.py

import http.server
import json
import os
import re
import shutil
import sqlite3
import subprocess
import sys
import tempfile
import threading
import unittest

HERE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PREVIEW = os.path.join(HERE, "scripts", "preview")

STUB_ARMADA = """#!/bin/sh
case "$1 $2" in
  "worktree --status") [ -f "$STUB_DIR/refuse" ] && { cat "$STUB_DIR/refuse" >&2; exit 2; }; cat "$STUB_DIR/slots" ;;
  *) echo "stub: not a form" >&2; exit 2 ;;
esac
"""

LEGACY = "crates/store/src/legacy_migrations.rs"
DIR = "crates/store/migrations"
# The frozen numbered entries a build lists, which is what the guard reads for them.
MIGRATIONS = (
    "pub(crate) const LEGACY: &[Migration] = &[\n"
    '    Migration::additive("schema.v1", crate::schema::V1),\n'
    '    Migration::additive("x.v2", crate::x::V2),\n'
    "];\n"
)

class Preview(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.mkdtemp()
        self.repo = os.path.join(self.dir, "repo")
        self.stub = os.path.join(self.dir, "stub")
        os.makedirs(self.stub)
        for name in ("slots",):
            open(os.path.join(self.stub, name), "w").close()
        armada = os.path.join(self.stub, "armada")
        with open(armada, "w") as f:
            f.write(STUB_ARMADA)
        os.chmod(armada, 0o755)
        self.clock = 1_700_000_000
        os.makedirs(self.repo)
        self.git("init", "-q", "-b", "main")
        self.git("config", "user.name", "t")
        self.git("config", "user.email", "t@example.com")
        self.write("a.txt", "one\ntwo\nthree\n")
        self.write(f"{DIR}/.gitkeep", "")
        self.commit("main: start")
        self.slot_lines = []
        self.home = os.path.join(self.dir, "home")
        self.support = os.path.join(self.home, "Library", "Application Support", "Armada")
        os.makedirs(self.support)

    def tearDown(self):
        shutil.rmtree(self.dir, ignore_errors=True)

    def git(self, *args):
        return subprocess.run(
            ["git", *args], cwd=self.repo, check=True, capture_output=True, text=True
        ).stdout.strip()

    def write(self, path, text):
        full = os.path.join(self.repo, path)
        os.makedirs(os.path.dirname(full), exist_ok=True)
        with open(full, "w") as f:
            f.write(text)

    def commit(self, message):
        self.clock += 100
        stamp = f"{self.clock} +0000"
        subprocess.run(
            ["git", "add", "-A"], cwd=self.repo, check=True, capture_output=True
        )
        subprocess.run(
            ["git", "commit", "-q", "-m", message],
            cwd=self.repo, check=True, capture_output=True,
            env={**os.environ, "GIT_AUTHOR_DATE": stamp, "GIT_COMMITTER_DATE": stamp},
        )

    def branch(self, name, edits, base="main"):
        """Cut `name` from `base`, commit `edits` on it, return to main."""
        self.git("checkout", "-q", "-b", name, base)
        for path, text in edits.items():
            self.write(path, text)
        self.commit(f"{name}: work")
        self.git("checkout", "-q", "main")

    def hold(self, *branches):
        self.slot_lines = [
            f"slot-{i + 1}  held     {self.repo}/slots/s{i}  {b}  by claude (pid 1) for 1m"
            for i, b in enumerate(branches)
        ]
        with open(os.path.join(self.stub, "slots"), "w") as f:
            f.write("\n".join(self.slot_lines) + "\n")

    def hold_rows(self, *rows):
        """Slot lines as `armada worktree --status` prints them: (state, branch, holder)."""
        lines = [f"slot-{i + 1}  {state}     {self.repo}/slots/s{i}  {b}  {holder}"
                 for i, (state, b, holder) in enumerate(rows)]
        with open(os.path.join(self.stub, "slots"), "w") as f:
            f.write("\n".join(lines) + "\n")

    def fleet(self, jobs, drones):
        """A Fleet answering `{job_id: status}`, with Drones for `drones`; HOME points at it."""
        server = roster_server(jobs, drones)
        self.addCleanup(server.server_close)
        self.addCleanup(server.shutdown)
        with open(os.path.join(self.support, "fleet.json"), "w") as f:
            json.dump({"pid": os.getpid(), "port": server.server_address[1]}, f)

    def run_preview(self, *args):
        env = {**os.environ,
               "HOME": self.home,
               "ARMADA_LAND_ARMADA": os.path.join(self.stub, "armada"),
               "STUB_DIR": self.stub}
        done = subprocess.run([sys.executable, PREVIEW, *args], cwd=self.repo,
                              capture_output=True, text=True, env=env)
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        return done.stdout

    @property
    def wt(self):
        return os.path.join(self.repo, ".armada", "preview")

    def merged_in_order(self):
        subjects = subprocess.run(
            ["git", "log", "--first-parent", "--reverse", "--format=%s", "main..preview"],
            cwd=self.wt, capture_output=True, text=True, check=True).stdout.splitlines()
        return [s.split("'")[1] for s in subjects]

    def land_on_origin_main(self, name):
        """A commit only `origin/main` has, as a pull request landing after the checkout last pulled."""
        self.git("checkout", "-q", "-b", f"landed-{name}", "main")
        self.write(f"{name}.txt", f"{name}\n")
        self.commit(f"landed: {name}")
        self.git("update-ref", "refs/remotes/origin/main", f"refs/heads/landed-{name}")
        self.git("checkout", "-q", "main")

    def test_a_checkout_behind_origin_main_previews_what_has_landed(self):
        self.land_on_origin_main("landed")
        self.hold()
        self.run_preview()
        self.assertTrue(os.path.exists(os.path.join(self.wt, "landed.txt")))

    def test_a_local_main_ahead_of_origin_main_is_the_base(self):
        self.git("update-ref", "refs/remotes/origin/main", "HEAD")
        self.write("ahead.txt", "ahead\n")
        self.commit("main: ahead of the remote")
        self.hold()
        self.run_preview()
        self.assertTrue(os.path.exists(os.path.join(self.wt, "ahead.txt")))

    def test_merges_oldest_commit_first_not_by_name(self):
        self.branch("feat/z-oldest", {"z.txt": "z\n"})
        self.branch("feat/a-newest", {"n.txt": "n\n"})
        self.hold("feat/a-newest", "feat/z-oldest")
        said = self.run_preview()
        self.assertEqual(self.merged_in_order(), ["feat/z-oldest", "feat/a-newest"])
        self.assertIn("Only committed work is included", said)
        self.assertTrue(os.path.exists(os.path.join(self.wt, "PREVIEW.txt")))
        self.assertIn("preview head: " + self.git("rev-parse", "--short", "preview"), said)

    def test_a_conflict_is_skipped_and_never_resolved(self):
        self.branch("feat/one", {"a.txt": "one\nTWO-from-one\nthree\n"})
        self.branch("feat/two", {"a.txt": "one\nTWO-from-two\nthree\n"})
        self.branch("feat/other", {"o.txt": "o\n"})
        self.hold("feat/one", "feat/two", "feat/other")
        said = self.run_preview()
        self.assertRegex(said, r"feat/two\s+skipped \(conflict: a.txt\)")
        self.assertEqual(self.merged_in_order(), ["feat/one", "feat/other"])
        with open(os.path.join(self.wt, "a.txt")) as f:
            self.assertNotIn("<<<<<<<", f.read())
        status = subprocess.run(["git", "status", "--porcelain"], cwd=self.wt,
                                capture_output=True, text=True).stdout
        self.assertNotIn("UU", status)

    def test_a_migration_name_taken_twice_is_skipped_though_git_merges_it(self):
        self.branch("feat/m-first", {f"{DIR}/20261007T0100Z-p.same.sql": "CREATE TABLE a (x TEXT);"})
        self.branch("feat/m-second", {f"{DIR}/20261007T0200Z-p.same.sql": "CREATE TABLE b (x TEXT);"})
        self.hold("feat/m-first", "feat/m-second")
        said = self.run_preview()
        self.assertRegex(said, r"feat/m-second\s+skipped \(migration p.same is already taken by feat/m-first\)")
        self.assertEqual(self.merged_in_order(), ["feat/m-first"])
        # Git alone would have merged both.
        probe = os.path.join(self.dir, "probe")
        self.git("worktree", "add", "-q", "--detach", probe, "feat/m-first")
        self.assertEqual(subprocess.run(["git", "merge", "-q", "--no-edit", "feat/m-second"],
                                        cwd=probe, capture_output=True).returncode, 0)

    def test_only_and_skip(self):
        for name in ("feat/a", "feat/b", "feat/c"):
            self.branch(name, {name.split("/")[1] + ".txt": "x\n"})
        self.hold("feat/a", "feat/b", "feat/c")
        self.run_preview("--only", "feat/a,feat/c")
        self.assertEqual(self.merged_in_order(), ["feat/a", "feat/c"])
        said = self.run_preview("--skip", "feat/b", "--skip", "feat/c")
        self.assertEqual(self.merged_in_order(), ["feat/a"])
        self.assertIn("feat/b: skipped by --skip", said)

    def test_only_the_held_and_ahead_are_in_flight(self):
        self.branch("feat/held", {"h.txt": "h\n"})
        self.branch("feat/idle", {"i.txt": "i\n"})
        self.git("branch", "feat/no-commits")
        self.hold("feat/held", "feat/no-commits", "main")
        self.run_preview()
        self.assertEqual(self.merged_in_order(), ["feat/held"])

    def test_a_branch_already_inside_another_is_nothing_new(self):
        self.branch("feat/base", {"b.txt": "b\n"})
        self.git("branch", "feat/same", "feat/base")
        self.hold("feat/base", "feat/same")
        said = self.run_preview()
        self.assertIn("nothing new", said)
        self.assertEqual(self.merged_in_order(), ["feat/base"])

    def test_uncommitted_work_is_warned_about_and_left_alone(self):
        self.branch("feat/dirty", {"d.txt": "d\n"})
        slot = os.path.join(self.repo, "slots", "s0")
        self.git("worktree", "add", "-q", slot, "feat/dirty")
        with open(os.path.join(slot, "wip.txt"), "w") as f:
            f.write("not committed\n")
        self.hold("feat/dirty")
        said = self.run_preview()
        self.assertIn("has 1 uncommitted files, not included", said)
        self.assertTrue(os.path.exists(os.path.join(slot, "wip.txt")))
        self.assertFalse(os.path.exists(os.path.join(self.wt, "wip.txt")))

    def test_each_run_starts_over_from_main_and_keeps_the_build_directories(self):
        self.branch("feat/gone", {"g.txt": "g\n"})
        self.hold("feat/gone")
        self.run_preview()
        self.assertTrue(os.path.exists(os.path.join(self.wt, "g.txt")))
        os.makedirs(os.path.join(self.wt, "target"))
        os.makedirs(os.path.join(self.wt, "node_modules"))
        for stray in ("target/warm", "node_modules/warm", "stray.txt"):
            with open(os.path.join(self.wt, stray), "w") as f:
                f.write("x")

        self.write("m.txt", "moved\n")
        self.commit("main: moved")
        self.hold()  # the branch left the slots
        self.run_preview()
        self.assertFalse(os.path.exists(os.path.join(self.wt, "g.txt")))
        self.assertTrue(os.path.exists(os.path.join(self.wt, "m.txt")))
        self.assertEqual(self.git("rev-parse", "preview"), self.git("rev-parse", "main"))
        self.assertTrue(os.path.exists(os.path.join(self.wt, "target", "warm")))
        self.assertTrue(os.path.exists(os.path.join(self.wt, "node_modules", "warm")))
        self.assertFalse(os.path.exists(os.path.join(self.wt, "stray.txt")))

    def test_a_note_pinned_on_the_preview_survives_a_rebuild(self):
        self.hold()
        self.run_preview()
        note = os.path.join(self.wt, ".armada", "annotations", "n1.json")
        os.makedirs(os.path.dirname(note))
        other = os.path.join(self.wt, ".armada", "other.log")
        for path in (note, other):
            with open(path, "w") as f:
                f.write("x")
        self.write("m.txt", "moved\n")
        self.commit("main: moved")
        self.run_preview()
        self.assertTrue(os.path.exists(note))
        self.assertFalse(os.path.exists(other))

    def test_restart_runs_the_restart_script_from_the_preview(self):
        self.branch("feat/r", {"r.txt": "r\n"})
        self.hold("feat/r")
        record = os.path.join(self.dir, "restart-args")
        script = os.path.join(self.dir, "restart")
        with open(script, "w") as f:
            f.write(f'#!/bin/sh\necho "$@" > "{record}"\n')
        os.chmod(script, 0o755)
        os.environ["ARMADA_PREVIEW_RESTART"] = script
        try:
            self.run_preview("--restart", "--dry-run")
            with open(record) as f:
                self.assertEqual(f.read().split(), ["--from", os.path.realpath(self.wt), "--dry-run"])
            self.run_preview("--restart", "--adopt")
            with open(record) as f:
                self.assertEqual(f.read().split(), ["--from", os.path.realpath(self.wt), "--adopt"])
        finally:
            del os.environ["ARMADA_PREVIEW_RESTART"]

    def test_a_kept_slot_and_a_stopped_jobs_slot_are_left_out_and_named(self):
        for name in ("feat/live", "feat/kept", "feat/dead", "feat/orphan", "feat/review"):
            self.branch(name, {name.split("/")[1] + ".txt": "x\n"})
        self.fleet({"JLIVE": "running", "JKEPT": "completed_failed", "JDEAD": "killed",
                    "JORPHAN": "running", "JREVIEW": "awaiting_review"}, drones=["JLIVE"])
        self.hold_rows(
            ("held", "feat/live", "by /usr/bin/claude (pid 1) for 1m"),
            ("kept", "feat/kept", "by job JKEPT for 2h, which ended and could not give it back: 2 uncommitted"),
            ("held", "feat/dead", "by job JDEAD for 2h"),
            ("held", "feat/orphan", "by job JORPHAN for 7h"),
            ("held", "feat/review", "by job JREVIEW for 1h"))
        said = self.run_preview()
        self.assertEqual(self.merged_in_order(), ["feat/live", "feat/review"])
        for name in ("kept", "dead", "orphan"):
            self.assertIn(f"left out: feat/{name}: held by a stopped Job", said)

    def test_a_stranded_branch_stays_when_a_live_slot_holds_it(self):
        self.branch("feat/both", {"b.txt": "b\n"})
        self.hold_rows(
            ("kept", "feat/both", "by job J2 for 2h, which ended and could not give it back: x"),
            ("held", "feat/both", "by /usr/bin/claude (pid 1) for 1m"))
        said = self.run_preview()
        self.assertEqual(self.merged_in_order(), ["feat/both"])
        self.assertNotIn("stopped Job", said)

    def test_a_job_slot_is_kept_when_fleet_does_not_answer(self):
        self.branch("feat/job", {"j.txt": "j\n"})
        self.hold_rows(("held", "feat/job", "by job J1 for 7h"))
        self.run_preview()
        self.assertEqual(self.merged_in_order(), ["feat/job"])

    def test_an_armada_that_does_not_answer_is_refused_whatever_it_exits(self):
        self.branch("feat/a", {"a.txt": "a\n"})
        with open(os.path.join(self.stub, "refuse"), "w") as f:
            f.write("`--status` is not a flag this verb takes\n")
        env = {**os.environ,
               "ARMADA_LAND_ARMADA": os.path.join(self.stub, "armada"),
               "STUB_DIR": self.stub}
        done = subprocess.run([sys.executable, PREVIEW], cwd=self.repo,
                              capture_output=True, text=True, env=env)
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("said:", done.stderr)


RESTART = os.path.join(HERE, "scripts", "restart")


def roster_server(jobs, drones=None):
    """A Fleet's two reads, `/drones` and `/jobs/<id>`, from {job_id: status}.
    `drones` names the Jobs that have a Drone; every Job has one by default."""
    drones = list(jobs) if drones is None else drones
    class Handler(http.server.BaseHTTPRequestHandler):
        def do_GET(self):
            if self.path == "/drones":
                body = {"drones": [{"handle": f"drone-{j}", "job_id": j} for j in drones]}
            elif self.path.startswith("/jobs/") and self.path[6:] in jobs:
                body = {"job": {"status": jobs[self.path[6:]]}}
            else:
                self.send_error(404)
                return
            data = json.dumps(body).encode()
            self.send_response(200)
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)

        def log_message(self, *args):
            pass

    server = http.server.HTTPServer(("127.0.0.1", 0), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return server


def run(cwd, *args):
    return subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@t", *args], cwd=cwd,
                          check=True, capture_output=True, text=True).stdout.strip()


def scratch_root(base, main_list):
    """A repository of its own holding a copy of `scripts/restart`, with
    `origin/main` at `main_list`, so the guard never reads the real one."""
    root = os.path.join(base, "root")
    os.makedirs(os.path.join(root, "scripts"))
    shutil.copy(RESTART, os.path.join(root, "scripts", "restart"))
    run(root, "init", "-q", "-b", "main")
    os.makedirs(os.path.join(root, os.path.dirname(LEGACY)))
    with open(os.path.join(root, LEGACY), "w") as f:
        f.write(main_list)
    run(root, "add", ".")
    run(root, "commit", "-q", "-m", "main")
    run(root, "update-ref", "refs/remotes/origin/main", "HEAD")
    return os.path.join(root, "scripts", "restart")


class RestartAdopt(unittest.TestCase):
    """`scripts/restart` under a scratch HOME against a stubbed roster. Never the real one."""

    def setUp(self):
        self.dir = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, self.dir, True)
        self.home = os.path.join(self.dir, "home")
        support = os.path.join(self.home, "Library", "Application Support", "Armada")
        os.makedirs(support)
        self.support = support
        self.tree = os.path.join(self.dir, "tree")
        os.makedirs(os.path.join(self.tree, "crates", "armada"))
        os.makedirs(os.path.join(self.tree, "crates", "store", "src"))
        os.makedirs(os.path.join(self.tree, "apps", "desktop"))
        open(os.path.join(self.tree, "crates", "armada", "Cargo.toml"), "w").close()
        with open(os.path.join(self.tree, LEGACY), "w") as f:
            f.write(MIGRATIONS)
        # The build steps run before the late roster check; stubs keep them off.
        self.bin = os.path.join(self.dir, "bin")
        os.makedirs(self.bin)
        for name in ("cargo", "pnpm"):
            path = os.path.join(self.bin, name)
            with open(path, "w") as f:
                f.write("#!/bin/sh\nexit 0\n")
            os.chmod(path, 0o755)

    def fleet(self, jobs):
        server = roster_server(jobs)
        self.addCleanup(server.server_close)
        self.addCleanup(server.shutdown)
        with open(os.path.join(self.support, "fleet.json"), "w") as f:
            json.dump({"pid": os.getpid(), "port": server.server_address[1]}, f)

    def restart(self, *args):
        env = dict(os.environ, HOME=self.home, PATH=self.bin + os.pathsep + os.environ["PATH"])
        script = scratch_root(self.dir, MIGRATIONS) if not hasattr(self, "script") else self.script
        self.script = script
        done = subprocess.run([script, "--from", self.tree, *args], env=env, capture_output=True, text=True)
        return done.returncode, done.stdout + done.stderr

    def plain_restart(self, *args):
        """`scripts/restart` with no `--from`, with a stub standing in for `scripts/preview`."""
        script = scratch_root(self.dir, MIGRATIONS) if not hasattr(self, "script") else self.script
        self.script = script
        record = os.path.join(self.dir, "preview-args")
        stub = os.path.join(self.dir, "preview")
        with open(stub, "w") as f:
            f.write(f'#!/bin/sh\necho "$@" > "{record}"\n')
        os.chmod(stub, 0o755)
        env = dict(os.environ, HOME=self.home, PATH=self.bin + os.pathsep + os.environ["PATH"],
                   ARMADA_RESTART_PREVIEW=stub)
        done = subprocess.run([script, *args], env=env, capture_output=True, text=True)
        passed = None
        if os.path.exists(record):
            with open(record) as f:
                passed = f.read().strip()
        return done.returncode, passed, done.stdout + done.stderr

    def test_a_plain_restart_while_the_preview_runs_refreshes_the_preview(self):
        with open(os.path.join(self.support, "restart-source"), "w") as f:
            f.write("/somewhere/.armada/preview\n")
        code, passed, said = self.plain_restart("--dry-run", "--adopt")
        self.assertEqual(code, 0, said)
        self.assertEqual(passed, "--restart --dry-run --adopt")

    def test_main_leaves_the_preview(self):
        with open(os.path.join(self.support, "restart-source"), "w") as f:
            f.write("/somewhere/.armada/preview\n")
        code, passed, said = self.plain_restart("--main", "--dry-run")
        self.assertIsNone(passed, said)
        self.assertIn("--dry-run is for --from", said)

    def test_a_plain_restart_with_no_preview_running_does_not_run_the_preview(self):
        code, passed, said = self.plain_restart("--dry-run")
        self.assertIsNone(passed, said)

    def test_the_gateway_is_written_only_when_the_owner_opted_in(self):
        self.fleet({})
        code, said = self.restart("--dry-run")
        self.assertEqual(code, 0, said)
        self.assertIn("would not run the Phone Gateway", said)
        self.assertNotIn("com.armada.pocket", said)
        open(os.path.join(self.support, "pocket"), "w").close()
        code, said = self.restart("--dry-run")
        self.assertEqual(code, 0, said)
        self.assertIn("com.armada.pocket.plist", said)
        self.assertIn("<string>pocket</string>", said)
        self.assertIn("<string>%s</string>" % os.path.join(self.support, "pocket-app"), said)
        self.assertIn("would copy %s to %s" % (os.path.join(self.tree, "apps", "pocket", "dist"),
                                               os.path.join(self.support, "pocket-app")), said)
        self.assertIn("pnpm -C apps/pocket", said)

    def test_a_label_override_gives_the_gateway_its_own_label(self):
        self.fleet({})
        open(os.path.join(self.support, "pocket"), "w").close()
        env = dict(os.environ, HOME=self.home, ARMADA_FLEET_LABEL="com.armada.test",
                   PATH=self.bin + os.pathsep + os.environ["PATH"])
        script = scratch_root(self.dir, MIGRATIONS)
        done = subprocess.run([script, "--from", self.tree, "--dry-run"], env=env, capture_output=True, text=True)
        self.assertIn("com.armada.test.pocket.plist", done.stdout + done.stderr)
        self.assertNotIn("com.armada.pocket", done.stdout + done.stderr)

    def test_a_working_drone_refuses_without_adopt(self):
        self.fleet({"j1": "running", "j2": "escalated"})
        code, said = self.restart()
        self.assertNotEqual(code, 0)
        self.assertIn("refusing — drone-j1's Drone is working", said)
        self.assertNotIn("unheard", said)

    def test_the_refusal_comes_before_the_build(self):
        self.fleet({"j1": "running"})
        built = os.path.join(self.dir, "built")
        with open(os.path.join(self.bin, "cargo"), "w") as f:
            f.write(f'#!/bin/sh\ntouch "{built}"\n')
        code, said = self.restart()
        self.assertNotEqual(code, 0)
        self.assertFalse(os.path.exists(built), said)

    def test_dry_run_without_adopt_says_it_would_refuse(self):
        self.fleet({"j1": "running"})
        code, said = self.restart("--dry-run")
        self.assertEqual(code, 0)
        self.assertIn("the restart would refuse", said)

    def test_adopt_skips_the_refusal_and_prints_jobs_and_costs_once(self):
        self.fleet({"j1": "running", "j2": "running", "j3": "escalated"})
        # Past the check the script touches launchd, so the dry run stands for "proceeds".
        code, said = self.restart("--adopt", "--dry-run")
        self.assertEqual(code, 0)
        self.assertIn("drone-j1 (j1)", said)
        self.assertIn("drone-j2 (j2)", said)
        self.assertNotIn("j3", said)
        self.assertIn("the working Drones above would be adopted and the refusal skipped", said)
        for cost in ("cannot be redirected, poked or handed a verdict", "undercount", "`unheard`",
                     "servers stop when Fleet stops", "gate re-runs from scratch",
                     "a Drone that cannot be adopted is ended"):
            self.assertEqual(said.count(cost), 1, cost)

    def test_dry_run_says_fleet_is_booted_out_and_in_not_kickstarted(self):
        self.fleet({"j1": "escalated"})
        code, said = self.restart("--dry-run")
        self.assertEqual(code, 0, said)
        self.assertIn("out if it is loaded, then bootstrap it from", said)

    def test_adopt_with_no_drone_working_prints_no_costs(self):
        self.fleet({"j1": "escalated"})
        code, said = self.restart("--adopt", "--dry-run")
        self.assertEqual(code, 0)
        self.assertNotIn("undercount", said)
        self.assertNotIn("would be adopted", said)
        self.assertIn("no Drone is working, so there is nothing to skip", said)

    def test_adopt_still_refuses_when_the_roster_does_not_answer(self):
        self.fleet({})
        server = roster_server({})
        port = server.server_address[1]
        server.shutdown()
        server.server_close()
        with open(os.path.join(self.support, "fleet.json"), "w") as f:
            json.dump({"pid": os.getpid(), "port": port}, f)
        code, said = self.restart("--adopt")
        self.assertNotEqual(code, 0)
        self.assertIn("did not answer", said)
        code, said = self.restart("--adopt", "--dry-run")
        self.assertIn("would still refuse", said)


class MigrationGuard(unittest.TestCase):
    """The names guard of `scripts/restart --from`, as `--dry-run` runs it, under a
    scratch HOME and a scratch root whose `origin/main` is the base. Never the real database."""

    def setUp(self):
        self.dir = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, self.dir, True)
        self.home = os.path.join(self.dir, "home")
        self.support = os.path.join(self.home, "Library", "Application Support", "Armada")
        os.makedirs(self.support)
        self.script = scratch_root(self.dir, MIGRATIONS)
        self.tree = os.path.join(self.dir, "tree")
        os.makedirs(os.path.join(self.tree, "crates", "armada"))
        os.makedirs(os.path.join(self.tree, "apps", "desktop"))
        os.makedirs(os.path.join(self.tree, DIR))
        os.makedirs(os.path.join(self.tree, os.path.dirname(LEGACY)))
        open(os.path.join(self.tree, "crates", "armada", "Cargo.toml"), "w").close()
        with open(os.path.join(self.tree, LEGACY), "w") as f:
            f.write(MIGRATIONS)
        open(os.path.join(self.tree, DIR, ".gitkeep"), "w").close()
        run(self.tree, "init", "-q", "-b", "main")
        run(self.tree, "add", ".")
        run(self.tree, "commit", "-q", "-m", "base")

    def on_branch(self, branch, name, breaking=False):
        """The build being previewed: a branch that adds one migration file."""
        run(self.tree, "checkout", "-q", "-b", branch)
        with open(os.path.join(self.tree, DIR, f"20261007T0130Z-{name}.sql"), "w") as f:
            f.write(("-- breaking\n" if breaking else "") + "CREATE TABLE t (x TEXT);\n")
        run(self.tree, "add", ".")
        run(self.tree, "commit", "-q", "-m", branch)

    def database(self, rows=None, numbered=None):
        db = os.path.join(self.support, "armada.db")
        con = sqlite3.connect(db)
        con.execute("CREATE TABLE armada_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL)")
        if numbered is not None:
            con.execute("INSERT INTO armada_meta VALUES ('schema_version', ?)", (str(numbered),))
        else:
            con.execute("CREATE TABLE armada_migrations (name TEXT PRIMARY KEY, additive INTEGER NOT NULL)")
            con.executemany("INSERT INTO armada_migrations VALUES (?, ?)", rows or [])
        con.commit()
        con.close()

    def guard(self):
        env = dict(os.environ, HOME=self.home)
        done = subprocess.run([self.script, "--from", self.tree, "--dry-run"], env=env,
                              capture_output=True, text=True)
        return done.returncode, done.stdout + done.stderr

    def test_a_build_with_nothing_unlanded_passes(self):
        self.database([("schema.v1", 1), ("x.v2", 1)])
        code, said = self.guard()
        self.assertEqual(code, 0, said)
        self.assertIn("Migration guard: 2 migrations listed, 2 applied", said)

    def test_an_unlanded_breaking_migration_is_refused_naming_it_and_its_branch(self):
        self.database([("schema.v1", 1), ("x.v2", 1)])
        self.on_branch("fleet/rebuilds-a-table", "y.rebuild", breaking=True)
        code, said = self.guard()
        self.assertNotEqual(code, 0)
        self.assertIn("y.rebuild from fleet/rebuilds-a-table", said)
        self.assertIn("unlanded work", said)

    def test_a_migration_file_already_on_origin_main_is_not_unlanded(self):
        self.database([("schema.v1", 1), ("x.v2", 1), ("y.rebuild", 1)])
        name = f"{DIR}/20261007T0130Z-y.rebuild.sql"
        root = os.path.dirname(os.path.dirname(self.script))
        os.makedirs(os.path.join(root, DIR))
        for base in (root, self.tree):
            with open(os.path.join(base, name), "w") as f:
                f.write("-- breaking\nDROP TABLE t;\n")
            run(base, "add", ".")
            run(base, "commit", "-q", "-m", "landed")
        run(root, "update-ref", "refs/remotes/origin/main", "HEAD")
        code, said = self.guard()
        self.assertEqual(code, 0, said)
        self.assertNotIn("unlanded", said)

    def test_an_unlanded_additive_migration_is_allowed_and_said_to_be_safe(self):
        self.database([("schema.v1", 1), ("x.v2", 1)])
        self.on_branch("fleet/step-baseline", "step_baseline.survives_restart")
        code, said = self.guard()
        self.assertEqual(code, 0, said)
        self.assertIn("unlanded and additive, so safe to go back from: step_baseline.survives_restart", said)

    def test_an_unlanded_breaking_migration_the_database_already_has_applies_nothing_new(self):
        self.database([("schema.v1", 1), ("x.v2", 1), ("y.rebuild", 0)])
        self.on_branch("fleet/rebuilds-a-table", "y.rebuild", breaking=True)
        code, said = self.guard()
        self.assertEqual(code, 0, said)

    def test_a_database_with_extra_additive_names_is_not_a_refusal(self):
        self.database([("schema.v1", 1), ("x.v2", 1), ("step_baseline.survives_restart", 1)])
        code, said = self.guard()
        self.assertEqual(code, 0, said)
        self.assertIn("applied and not listed here, all additive: step_baseline.survives_restart", said)

    def test_a_database_with_an_extra_breaking_name_is_refused(self):
        self.database([("schema.v1", 1), ("x.v2", 1), ("z.rewrote", 0)])
        code, said = self.guard()
        self.assertNotEqual(code, 0)
        self.assertIn("breaking migrations", said)
        self.assertIn("z.rewrote", said)

    def test_a_numbered_database_converts_in_the_read(self):
        self.database(numbered=2)
        code, said = self.guard()
        self.assertEqual(code, 0, said)
        self.assertIn("2 applied", said)

    def test_a_numbered_database_past_the_old_list_is_refused(self):
        self.database(numbered=114)
        code, said = self.guard()
        self.assertNotEqual(code, 0)
        self.assertIn("114", said)

    def test_no_database_is_not_a_refusal(self):
        code, said = self.guard()
        self.assertEqual(code, 0, said)

    def test_a_database_with_a_live_wal_reads(self):
        self.database([("schema.v1", 1), ("x.v2", 1)])
        con = sqlite3.connect(os.path.join(self.support, "armada.db"))
        con.execute("PRAGMA journal_mode=WAL")
        con.execute("INSERT INTO armada_migrations VALUES ('w.late', 1)")
        con.commit()
        self.addCleanup(con.close)
        code, said = self.guard()
        self.assertEqual(code, 0, said)
        self.assertIn("w.late", said)

    def test_a_wal_with_no_shm_in_a_directory_that_cannot_be_written_reads_from_a_copy(self):
        # The 6 Oct failure: `unable to open database file (14)` read-only, because a WAL
        # with no usable -shm cannot be opened without creating one.
        self.database([("schema.v1", 1), ("x.v2", 1)])
        db = os.path.join(self.support, "armada.db")
        con = sqlite3.connect(db)
        con.execute("PRAGMA journal_mode=WAL")
        con.execute("INSERT INTO armada_migrations VALUES ('w.late', 1)")
        con.commit()
        shutil.copy(db, db + ".keep")
        shutil.copy(db + "-wal", db + "-wal.keep")
        con.close()
        for suffix in ("", "-wal", "-shm"):
            if os.path.exists(db + suffix):
                os.remove(db + suffix)
        shutil.move(db + ".keep", db)
        shutil.move(db + "-wal.keep", db + "-wal")
        os.chmod(self.support, 0o555)
        self.addCleanup(os.chmod, self.support, 0o755)
        code, said = self.guard()
        self.assertEqual(code, 0, said)
        self.assertIn("w.late", said)

    def test_an_origin_main_that_cannot_be_read_is_refused_not_guessed(self):
        run(os.path.dirname(os.path.dirname(self.script)), "update-ref", "-d", "refs/remotes/origin/main")
        code, said = self.guard()
        self.assertNotEqual(code, 0)
        self.assertIn("origin/main", said)


RESTART_BUILD = os.path.join(HERE, "scripts", "restart-build")


class RestartBuild(unittest.TestCase):
    """`scripts/restart-build` under a scratch HOME, with stubs for the two scripts it runs and for `cargo`."""

    def setUp(self):
        self.dir = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, self.dir, True)
        self.home = os.path.join(self.dir, "home")
        self.support = os.path.join(self.home, "Library", "Application Support", "Armada")
        os.makedirs(self.support)
        self.root = os.path.join(self.dir, "repo")
        os.makedirs(os.path.join(self.root, "scripts"))
        shutil.copy(RESTART_BUILD, os.path.join(self.root, "scripts", "restart-build"))
        for args in (["init", "-q", "-b", "main"], ["config", "user.name", "t"], ["config", "user.email", "t@example.com"],
                     ["commit", "-q", "--allow-empty", "-m", "start"]):
            subprocess.run(["git", "-C", self.root, *args], check=True, capture_output=True)
        self.bin = os.path.join(self.dir, "bin")
        os.makedirs(self.bin)
        self.ran = os.path.join(self.dir, "ran")
        self.cleaned = os.path.join(self.dir, "cleaned")
        self.stub("cargo", f'echo "$PWD: $@" >> "{self.cleaned}"')
        self.status_file = os.path.join(self.support, "restart-build.status")

    def stub(self, name, body):
        path = os.path.join(self.bin, name)
        with open(path, "w") as f:
            f.write(f"#!/bin/sh\n{body}\n")
        os.chmod(path, 0o755)
        return path

    def build(self, which, *args, script="exit 0"):
        """Run the wrapper with `script` standing in for what it wraps."""
        stub = self.stub("wrapped", f'echo "$@" >> "{self.ran}"\n{script}')
        env = dict(os.environ, HOME=self.home, PATH=self.bin + os.pathsep + os.environ["PATH"],
                   ARMADA_RESTART_SCRIPT=stub, ARMADA_PREVIEW_SCRIPT=stub, ARMADA_RESTART_POLL="0.02")
        done = subprocess.run([os.path.join(self.root, "scripts", "restart-build"), which, *args],
                              env=env, capture_output=True, text=True)
        return done.returncode

    def runs(self):
        if not os.path.exists(self.ran):
            return []
        with open(self.ran) as f:
            return f.read().split("\n")[:-1]

    def status(self):
        if not os.path.exists(self.status_file):
            return None
        with open(self.status_file) as f:
            return f.read()

    def test_main_runs_a_plain_restart_with_main_and_leaves_no_status(self):
        self.assertEqual(self.build("main"), 0)
        self.assertEqual(self.runs(), ["--main"])
        self.assertIsNone(self.status())

    def test_adopt_is_passed_through(self):
        self.assertEqual(self.build("main", "--adopt"), 0)
        self.assertEqual(self.build("preview", "--adopt"), 0)
        self.assertEqual(self.runs(), ["--main --adopt", "--restart --adopt"])

    def test_preview_runs_the_previews_restart(self):
        self.assertEqual(self.build("preview"), 0)
        self.assertEqual(self.runs(), ["--restart"])

    def test_a_refusal_is_one_plain_line_and_names_the_build_it_failed_on(self):
        with open(os.path.join(self.support, "restart-commit"), "w") as f:
            f.write("a" * 40 + "\n")
        code = self.build("main", script='echo "restart: refusing \u2014 drone-3\'s Drone is working" >&2; exit 1')
        self.assertEqual(code, 1)
        self.assertEqual(self.status(), "failed\nmain\n" + "a" * 40 + "\nrefusing \u2014 drone-3's Drone is working\n")

    def test_a_failure_with_no_line_of_its_own_says_its_last_line(self):
        self.assertEqual(self.build("preview", script='printf "one\\n\\033[31mtwo\\033[0m\\n"; exit 3'), 1)
        self.assertEqual(self.status(), "failed\npreview\n\ntwo\n")

    def test_main_is_refused_from_a_checkout_on_another_branch_and_nothing_runs(self):
        subprocess.run(["git", "-C", self.root, "checkout", "-q", "-b", "feature"], check=True)
        self.assertEqual(self.build("main"), 1)
        self.assertEqual(self.runs(), [])
        self.assertEqual(self.status(), "failed\nmain\n\nthe checkout is on feature, not main\n")

    def test_a_stale_sqlite_build_script_is_cleaned_in_the_tree_and_tried_once_more(self):
        os.makedirs(os.path.join(self.root, ".armada", "preview"))
        marker = os.path.join(self.dir, "first")
        script = (f'if [ ! -f "{marker}" ]; then touch "{marker}"; '
                  'echo "error: failed to run custom build command for libsqlite3-sys" >&2; exit 101; fi')
        self.assertEqual(self.build("preview", script=script), 0)
        self.assertEqual(self.runs(), ["--restart", "--restart"])
        with open(self.cleaned) as f:
            cleaned = f.read().strip()
        self.assertTrue(cleaned.endswith(f"{os.sep}repo{os.sep}.armada{os.sep}preview: clean -p libsqlite3-sys"), cleaned)
        self.assertIsNone(self.status())

    def test_the_retry_is_once_and_a_second_failure_is_reported(self):
        os.makedirs(os.path.join(self.root, ".armada", "preview"))
        script = 'echo "error: failed to run custom build command for libsqlite3-sys" >&2; exit 101'
        self.assertEqual(self.build("preview", script=script), 1)
        self.assertEqual(self.runs(), ["--restart", "--restart"])
        self.assertEqual(self.status(), "failed\npreview\n\nerror: failed to run custom build command for libsqlite3-sys\n")

    def stages_seen(self, which, headings, plain=(), **kwargs):
        """The stages the status file shows, in order, as a stub prints `headings` bold and `plain` unbold.
        The stub waits after each line, long enough for the wrapper's poll to have read it."""
        seen = os.path.join(self.dir, "seen")
        if os.path.exists(seen):
            os.remove(seen)
        save = f'sleep 0.3; cat "{self.status_file}" >> "{seen}"; echo --- >> "{seen}"'
        lines = [f"printf '\\n\\033[1m%s\\033[0m\\n' '{h}' >&2; {save}" for h in headings]
        lines += [f"printf '%s\\n' '{p}' >&2; {save}" for p in plain]
        code = self.build(which, script="\n".join(lines), **kwargs)
        self.assertEqual(code, 0)
        with open(seen) as f:
            records = [r.split("\n") for r in f.read().split("---\n") if r]
        return [r[2] if len(r) > 2 else None for r in records]

    PREVIEW_RUN = ["Installing armada from /x", "Copying the Claude Code mod from /x", "ipc was last built at protocol a",
                   "Building Bridge \u2014 apps/ moved", "Snapshot of the database: x", "Booting org.armada out and back in",
                   "Quitting the running Bridge (pid 1) for this repository", "Reopening Bridge from /x", "Done"]

    def test_a_preview_restart_starts_merging_and_follows_the_headings(self):
        self.assertEqual(self.stages_seen("preview", self.PREVIEW_RUN),
                         ["building_fleet", "building_fleet", "building_fleet", "building_bridge", "restarting_fleet",
                          "restarting_fleet", "reopening_bridge", "reopening_bridge", "reopening_bridge"])

    def test_the_first_stage_is_merging_for_preview_and_fetching_main_for_main(self):
        for which, first in (("preview", "merging"), ("main", "fetching_main")):
            seen = self.stages_seen(which, [], plain=["something that is no heading"])
            self.assertEqual(seen, [first], which)

    def test_a_main_restart_goes_from_the_fast_forward_through_the_same_stages(self):
        stages = self.stages_seen("main", ["Fast-forwarding main to origin/main", "Fleet is running (pid 1) \u2014 checking for a working Drone",
                                           *self.PREVIEW_RUN])
        self.assertEqual(stages[:2], ["fetching_main", "fetching_main"])
        self.assertEqual(stages[2], "building_fleet")
        self.assertEqual(stages[-1], "reopening_bridge")

    def test_a_line_that_is_not_bold_never_sets_a_stage(self):
        seen = self.stages_seen("main", [], plain=["Building Bridge", "Installing armada from /x", "Booting x",
                                                   "  Installing armada v0.0.0", "\x1b[1m\x1b[32m   Installing\x1b[0m armada"])
        self.assertEqual(seen, ["fetching_main"] * 5)

    def test_cargos_own_bold_output_is_not_a_heading(self):
        stub = ("printf '\\033[1m\\033[32m   Installing\\033[0m armada v1\\n' >&2\n"
                f"sleep 0.3; cat \"{self.status_file}\" > \"{self.dir}/seen\"")
        self.assertEqual(self.build("main", script=stub), 0)
        with open(os.path.join(self.dir, "seen")) as f:
            self.assertEqual(f.read(), "running\nmain\nfetching_main\n")

    def test_retrying_holds_through_the_second_installing_heading_and_gives_way_to_bridge(self):
        os.makedirs(os.path.join(self.root, ".armada", "preview"))
        marker = os.path.join(self.dir, "first")
        seen = os.path.join(self.dir, "seen")
        save = f'sleep 0.3; cat "{self.status_file}" >> "{seen}"; echo --- >> "{seen}"'
        say = lambda h: f"printf '\\n\\033[1m%s\\033[0m\\n' '{h}' >&2; {save}"
        script = (f'if [ ! -f "{marker}" ]; then touch "{marker}"; {say("Installing armada from /x")}\n'
                  'echo "error: failed to run custom build command for libsqlite3-sys" >&2; exit 101; fi\n'
                  f'{say("Installing armada from /x")}\n{say("Fast-forwarding main to origin/main")}\n{say("Building Bridge")}')
        self.assertEqual(self.build("preview", script=script), 0)
        with open(seen) as f:
            records = [r.split("\n") for r in f.read().split("---\n") if r]
        self.assertEqual([r[2] for r in records], ["building_fleet", "retrying", "retrying", "building_bridge"])

    def test_a_failure_after_a_stage_is_still_one_failed_record(self):
        code = self.build("main", script='printf "\\n\\033[1mInstalling armada from /x\\033[0m\\n" >&2; sleep 0.3; echo "restart: !!! nope" >&2; exit 1')
        self.assertEqual(code, 1)
        self.assertEqual(self.status(), "failed\nmain\n\nnope\n")

    def test_an_unknown_build_is_refused(self):
        self.assertNotEqual(self.build("feature"), 0)
        self.assertEqual(self.runs(), [])


class PreviewAdopt(unittest.TestCase):
    def run_preview(self, *args):
        return subprocess.run([PREVIEW, *args], capture_output=True, text=True)

    def test_adopt_is_only_for_restart(self):
        done = self.run_preview("--adopt")
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("--adopt is for --restart", done.stderr)

    def test_adopt_is_refused_with_watch(self):
        done = self.run_preview("--watch", "--restart", "--adopt")
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("--watch never restarts Fleet", done.stderr)


if __name__ == "__main__":
    unittest.main()
