#!/usr/bin/env python3
#
# `scripts/preview` against a throwaway repository and a stub `armada` that
# answers `worktree --status` and `land --status` from two files. Git only:
# nothing here builds, runs a Check or touches Fleet.
#
#   python3 scripts/test_preview.py

import http.server
import json
import os
import shutil
import subprocess
import sys
import tempfile
import threading
import unittest

HERE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PREVIEW = os.path.join(HERE, "scripts", "preview")

STUB_ARMADA = """#!/bin/sh
case "$1 $2" in
  "worktree --status") cat "$STUB_DIR/slots" ;;
  "land --status") cat "$STUB_DIR/line"; exit 8 ;;
  *) echo "stub: not a form" >&2; exit 2 ;;
esac
"""

MIGRATIONS = (
    "pub const MIGRATIONS: &[&str] = &[\n"
    "    V1,\n"
    "    // a spacer so two appends do not touch the same lines\n"
    "    crate::x::V2,\n"
    "    // another\n"
    "    // another\n"
    "];\n"
)


class Preview(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.mkdtemp()
        self.repo = os.path.join(self.dir, "repo")
        self.stub = os.path.join(self.dir, "stub")
        os.makedirs(self.stub)
        for name in ("slots", "line"):
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
        self.write("crates/store/src/migrations.rs", MIGRATIONS)
        self.commit("main: start")
        self.slot_lines = []

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

    def hold(self, *branches, line=()):
        self.slot_lines = [
            f"slot-{i + 1}  held     {self.repo}/slots/s{i}  {b}  by claude (pid 1) for 1m"
            for i, b in enumerate(branches)
        ]
        with open(os.path.join(self.stub, "slots"), "w") as f:
            f.write("\n".join(self.slot_lines) + "\n")
        with open(os.path.join(self.stub, "line"), "w") as f:
            f.write("merge line: in line\n" + "".join(
                f"  {n + 1}. {b}  waiting: behind others\n" for n, b in enumerate(line)))

    def run_preview(self, *args):
        env = {**os.environ,
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

    def test_a_migration_number_taken_twice_is_skipped_though_git_merges_it(self):
        base = MIGRATIONS
        one = base.replace("    V1,\n", "    V1,\n    crate::p::V3,\n")
        two = base.replace("    // another\n    // another\n",
                           "    // another\n    // another\n    crate::q::V3,\n")
        self.branch("feat/m-first", {"crates/store/src/migrations.rs": one})
        self.branch("feat/m-second", {"crates/store/src/migrations.rs": two})
        self.hold("feat/m-first", "feat/m-second")
        said = self.run_preview()
        self.assertRegex(said, r"feat/m-second\s+skipped \(migration V3 is already taken by feat/m-first\)")
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

    def test_only_the_held_and_queued_and_ahead_are_in_flight(self):
        self.branch("feat/held", {"h.txt": "h\n"})
        self.branch("feat/queued", {"q.txt": "q\n"})
        self.branch("feat/idle", {"i.txt": "i\n"})
        self.git("branch", "feat/no-commits")
        self.hold("feat/held", "feat/no-commits", "main", line=("feat/queued",))
        self.run_preview()
        self.assertEqual(self.merged_in_order(), ["feat/held", "feat/queued"])

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


RESTART = os.path.join(HERE, "scripts", "restart")


def roster_server(jobs):
    """A Fleet's two reads, `/drones` and `/jobs/<id>`, from {job_id: status}."""
    class Handler(http.server.BaseHTTPRequestHandler):
        def do_GET(self):
            if self.path == "/drones":
                body = {"drones": [{"handle": f"drone-{j}", "job_id": j} for j in jobs]}
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
        with open(os.path.join(self.tree, "crates", "store", "src", "migrations.rs"), "w") as f:
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
        done = subprocess.run([RESTART, "--from", self.tree, *args], env=env, capture_output=True, text=True)
        return done.returncode, done.stdout + done.stderr

    def test_a_working_drone_refuses_without_adopt(self):
        self.fleet({"j1": "running", "j2": "escalated"})
        code, said = self.restart()
        self.assertNotEqual(code, 0)
        self.assertIn("refusing — drone-j1's Drone is working", said)
        self.assertNotIn("unheard", said)

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
        self.assertIn("a working Drone would be adopted and the refusal skipped", said)
        for cost in ("cannot be redirected, poked or handed a verdict", "undercount", "`unheard`",
                     "servers stop when Fleet stops", "gate re-runs from scratch",
                     "a Drone that cannot be adopted is ended"):
            self.assertEqual(said.count(cost), 1, cost)

    def test_adopt_with_no_drone_working_prints_no_costs(self):
        self.fleet({"j1": "escalated"})
        code, said = self.restart("--adopt", "--dry-run")
        self.assertEqual(code, 0)
        self.assertNotIn("undercount", said)

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

    def test_an_armada_that_does_not_answer_is_refused_whatever_it_exits(self):
        self.branch("feat/a", {"a.txt": "a\n"})
        with open(os.path.join(self.stub, "line"), "w") as f:
            f.write("`--status` is not a flag this verb takes\n")
        env = {**os.environ,
               "ARMADA_LAND_ARMADA": os.path.join(self.stub, "armada"),
               "STUB_DIR": self.stub}
        done = subprocess.run([sys.executable, PREVIEW], cwd=self.repo,
                              capture_output=True, text=True, env=env)
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("said:", done.stderr)


if __name__ == "__main__":
    unittest.main()
