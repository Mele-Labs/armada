#!/usr/bin/env python3
#
# `scripts/land` against a throwaway repository with a local bare remote, a
# stub `gh` that reads a pull request as merged once its head is on `main` as
# GitHub would, and a stub `armada` whose Checks are shell files in the tree
# being gated. Nothing here
# reaches GitHub or runs a real Check.
#
#   python3 scripts/test_land.py
#
# Beside the script because no Python test in this repository has a home yet.
#
# `Line` runs every test against the real `armada land` (Rust) — the stub
# `armada`'s own `land` case execs into it. It ran twice for one release,
# the second pass forced through a Python fallback, while every installed
# `armada` still predated the verb; that fallback is gone and so is the
# second pass.

import json
import os
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import textwrap
import unittest
from hashlib import sha256

HERE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
LAND = os.path.join(HERE, "scripts", "land")
REAL_ARMADA = os.path.join(HERE, "target", "debug", "armada")


def setUpModule():
    # `armada.yml`'s `run:` gets no shell to chain a build onto, so the one
    # command this Check declares has to be the one thing that both builds
    # and runs. A `Line` test that finds no binary here skips with a clearer
    # reason than a build failure buried inside it would give.
    subprocess.run(
        ["cargo", "build", "--quiet", "-p", "armada"], cwd=HERE, check=False
    )

# `gh pr view` and `gh pr close`, over a JSON file of pull requests. The view
# answers MERGED once the branch's head is in the remote's `main`, which is how
# GitHub reads a pull request whose commits were pushed to its base; set
# `STUB_GH_NO_DETECT` and it never does. `gh pr merge` is refused outright: the
# line lands by pushing `main` itself.
STUB_GH = r'''#!/usr/bin/env python3
import json, os, subprocess, sys
state_file = os.environ["STUB_GH_STATE"]
remote = os.environ["STUB_REMOTE"]
prs = json.load(open(state_file))
args = sys.argv[1:]

def git(*a, check=True):
    return subprocess.run(["git", *a], capture_output=True, text=True, check=check)

def head(branch):
    said = git("ls-remote", remote, f"refs/heads/{branch}").stdout.strip()
    return said.split()[0] if said else None

def find(sel):
    for number, pr in prs.items():
        if sel == number or sel == pr["branch"]:
            return number, pr
    sys.exit("no pull requests found")

if args[:2] == ["pr", "view"]:
    number, pr = find(args[2])
    at = pr.get("merged_head") or head(pr["branch"])
    if pr["state"] == "OPEN" and at and not os.environ.get("STUB_GH_NO_DETECT"):
        if git("--git-dir", remote, "merge-base", "--is-ancestor", at, "main", check=False).returncode == 0:
            pr.update(state="MERGED", merged_head=at)
            json.dump(prs, open(state_file, "w"))
    fields = args[args.index("--json") + 1].split(",")
    full = {
        "number": int(number),
        "state": pr["state"],
        "baseRefName": os.environ.get("STUB_GH_BASE", "main"),
        "headRefOid": at,
    }
    print(json.dumps({f: full[f] for f in fields}))
elif args[:2] == ["pr", "close"]:
    number, pr = find(args[2])
    pr.update(state="CLOSED", comment=args[args.index("--comment") + 1] if "--comment" in args else None)
    json.dump(prs, open(state_file, "w"))
elif args[:2] == ["pr", "merge"]:
    sys.exit("stub gh: the line lands by pushing main, never through the forge")
else:
    sys.exit(f"stub gh: {args} is not stubbed")
'''

# `covers` reads `checks.json` in the working directory: a Check name to a list
# of path prefixes, or null for always. `check <name>` runs `checks/<name>.sh`.
#
# `land` is answered two ways, chosen by whether `STUB_ARMADA_LAND` is set in
# the environment — never by anything on the command line, so this is a
# fixture decision, not a value `scripts/land` itself could be tricked into
# passing through. Set, it execs into the real binary named there, exactly
# as an installed `armada` that already knows the verb would answer it
# itself. Unset, it answers the way an `armada` built before this verb
# existed actually answers an unknown one — `crates/armada/src/cli.rs`'s own
# wording — so `scripts/land`'s fallback detection is exercised against the
# real sentence, not a fixture's paraphrase of it.
STUB_ARMADA = r'''#!/usr/bin/env python3
import json, os, subprocess, sys
args = sys.argv[1:]
if args == ["covers"]:
    paths = [p for p in sys.stdin.read().splitlines() if p]
    for name, prefixes in json.load(open("checks.json")).items():
        if prefixes is None or any(p.startswith(x) for p in paths for x in prefixes):
            print(name)
elif args[:1] == ["check"]:
    script = os.path.join("checks", f"{args[1]}.sh")
    sys.exit(subprocess.run(["sh", script]).returncode if os.path.exists(script) else 0)
elif args[:1] == ["run"]:
    open(f"{args[1]}.stamp", "w").write("prepared\n")
    sys.exit(0)
elif args[:1] == ["land"]:
    real = os.environ.get("STUB_ARMADA_LAND")
    if not real:
        sys.exit("stub armada: STUB_ARMADA_LAND is unset, so there is no binary to answer `land`")
    os.execv(real, [real, *args])
else:
    sys.exit(f"stub armada: {args}")
'''


def key(branch):
    return sha256(branch.encode()).hexdigest()[:16]


def load(path):
    with open(path) as held:
        return json.load(held)


def dump(path, value):
    with open(path, "w") as out:
        json.dump(value, out)


def sh(*argv, cwd=None, env=None, check=True):
    done = subprocess.run(argv, cwd=cwd, env=env, capture_output=True, text=True)
    if check and done.returncode != 0:
        raise AssertionError(f"{argv} exited {done.returncode}:\n{done.stdout}{done.stderr}")
    return done


class LineFixture(unittest.TestCase):
    """The bare-remote-and-stubs harness, with no test methods of its own.
    `Line` adds them; the split is what let a second suite run the same
    scenarios against a second implementation while one existed."""

    # Which binary the stub's own `land` case forwards to — the real
    # `armada`, built by this crate's own `cargo build -p armada`.
    def stub_armada_land(self):
        if not os.path.exists(REAL_ARMADA):
            raise unittest.SkipTest(
                f"{REAL_ARMADA} does not exist — `cargo build -p armada` first"
            )
        return REAL_ARMADA

    def setUp(self):
        self.root = os.path.realpath(tempfile.mkdtemp(prefix="land-"))
        self.remote = os.path.join(self.root, "remote.git")
        self.repo = os.path.join(self.root, "repo")
        self.prs = os.path.join(self.root, "prs.json")
        stubs = os.path.join(self.root, "bin")
        os.makedirs(stubs)
        for name, body in (("gh", STUB_GH), ("armada", STUB_ARMADA)):
            path = os.path.join(stubs, name)
            with open(path, "w") as out:
                out.write(body)
            os.chmod(path, 0o755)
        with open(self.prs, "w") as out:
            out.write("{}")
        stub_land = self.stub_armada_land()
        self.env = dict(
            os.environ,
            ARMADA_LAND_GH=os.path.join(stubs, "gh"),
            ARMADA_LAND_ARMADA=os.path.join(stubs, "armada"),
            ARMADA_LAND_FOUNDATIONS="sh foundations.sh",
            ARMADA_LAND_SETUP="marker",
            ARMADA_LAND_SEED="seeded",
            ARMADA_LAND_KEEP="node_modules",
            ARMADA_LAND_REGENERATE="",
            LAND_TEST_EVIDENCE=os.path.join(self.root, "evidence.txt"),
            ARMADA_LAND_PR_WAIT="10",
            # One branch a turn, unless a test is about batching: most of
            # these queue a branch while another gates and read its turn alone.
            ARMADA_LAND_BATCH="1",
            STUB_ARMADA_LAND=stub_land,
            STUB_GH_STATE=self.prs,
            STUB_REMOTE=self.remote,
            GIT_CONFIG_GLOBAL="/dev/null",
            GIT_AUTHOR_NAME="test", GIT_AUTHOR_EMAIL="test@example.com",
            GIT_COMMITTER_NAME="test", GIT_COMMITTER_EMAIL="test@example.com",
        )
        sh("git", "init", "--quiet", "--bare", "-b", "main", self.remote)
        sh("git", "init", "--quiet", "-b", "main", self.repo)
        self.write(self.repo, {
            "checks.json": json.dumps({"test": None, "ui": ["ui/"]}),
            # Red only in combination: each branch alone passes.
            "checks/test.sh": (
                'printf "%s seed=%s setup=%s at=%s stray=%s node=%s\\n" check '
                '"$([ -f seeded/mark.txt ] && echo yes || echo no)" '
                '"$([ -f marker.stamp ] && echo yes || echo no)" "$PWD" '
                '"$([ -f stray.stamp ] && echo yes || echo no)" '
                '"$([ -f node_modules/mark ] && echo yes || echo no)" >> "$LAND_TEST_EVIDENCE"\n'
                "touch stray.stamp\n"
                "mkdir -p node_modules && touch node_modules/mark\n"
                "! { [ -f one.txt ] && [ -f two.txt ]; }\n"
            ),
            "foundations.sh": (
                'printf "%s seed=%s setup=%s at=%s\\n" foundations '
                '"$([ -f seeded/mark.txt ] && echo yes || echo no)" '
                '"$([ -f marker.stamp ] && echo yes || echo no)" "$PWD" >> "$LAND_TEST_EVIDENCE"\n'
                "cat foundations.txt 2>/dev/null; true\n"
            ),
            "foundations.txt": "FAIL  a rule main already fails\n        missing: its subject\n\nverify-foundations: RED — 1 failing, 0 warning\n",
            "shared.txt": "base\n",
            ".gitignore": ".armada/\n*.stamp\n",
        })
        self.git(self.repo, "add", "-A")
        self.git(self.repo, "commit", "--quiet", "-m", "base")
        self.git(self.repo, "remote", "add", "origin", self.remote)
        self.git(self.repo, "push", "--quiet", "-u", "origin", "main")
        # A seed path git neither tracks nor ignores, which is what the gate
        # worktree's own build directory is not.
        self.write(self.repo, {"seeded/mark.txt": "cloned into every gate worktree\n"})

    def tearDown(self):
        for line in (self.state_file("runner.log"),):
            if os.path.exists(line) and os.environ.get("LAND_TEST_VERBOSE"):
                print(open(line).read())
        # Only this test's runner, named by its own git directory.
        subprocess.run(["pkill", "-f", "--", f"--runner {self.repo}/.git"], capture_output=True)
        shutil.rmtree(self.root, ignore_errors=True)

    # ------------------------------------------------------------ helpers

    def git(self, cwd, *args):
        return sh("git", *args, cwd=cwd, env=self.env).stdout.strip()

    def write(self, cwd, files):
        for path, body in files.items():
            os.makedirs(os.path.dirname(os.path.join(cwd, path)) or cwd, exist_ok=True)
            with open(os.path.join(cwd, path), "w") as out:
                out.write(body)

    def state_file(self, *parts):
        return os.path.join(self.repo, ".git", "armada-land", *parts)

    def branch(self, name, files, pr=True):
        """An agent's worktree on `name`, cut from main and committed — pushed,
        with a PR, unless `pr` is false, since the line needs neither."""
        where = os.path.join(self.root, "wt-" + name.replace("/", "-"))
        self.git(self.repo, "fetch", "--quiet", "origin")
        self.git(self.repo, "worktree", "add", "--quiet", "-b", name, where, "origin/main")
        self.commit(where, files, f"work on {name}", push=pr)
        if pr:
            prs = load(self.prs)
            prs[str(len(prs) + 1)] = {"branch": name, "state": "OPEN"}
            dump(self.prs, prs)
        return where

    def commit(self, where, files, message, push=True):
        self.write(where, files)
        self.git(where, "add", "-A")
        self.git(where, "commit", "--quiet", "-m", message)
        if push:
            self.git(where, "push", "--quiet", "-u", "origin", "HEAD")

    def land(self, where, *args, check=True):
        done = sh(sys.executable, LAND, *args, cwd=where, env=self.env, check=False)
        if check and done.returncode not in (0,):
            raise AssertionError(f"land {args} exited {done.returncode}:\n{done.stdout}{done.stderr}")
        return done

    def settle(self, where, branch, timeout=60):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            done = self.land(where, "--status", branch, check=False)
            if done.returncode != 3:
                return done
            time.sleep(0.2)
        raise AssertionError(f"{branch} still in line:\n{done.stdout}")

    def main_clone(self):
        work = os.path.join(self.root, "check-" + str(time.monotonic_ns()))
        sh("git", "clone", "--quiet", self.remote, work, env=self.env)
        return work

    def main_files(self):
        return set(os.listdir(self.main_clone()))

    def main_head(self):
        return sh("git", "ls-remote", self.remote, "refs/heads/main", env=self.env).stdout.split()[0]

    def onto_main(self, files, message):
        """Commit straight onto main, as a hand merge would, with no turn to refuse it."""
        self.git(self.repo, "pull", "--quiet", "--ff-only", "origin", "main")
        self.write(self.repo, files)
        self.git(self.repo, "add", "--", *files)
        self.git(self.repo, "commit", "--quiet", "-m", message)
        self.git(self.repo, "push", "--quiet", "origin", "HEAD:main")

    def mover(self):
        """A script that pushes one commit onto main, for a base that moves mid-gate."""
        path = os.path.join(self.root, "move-main.sh")
        self.write(self.root, {"move-main.sh": textwrap.dedent("""\
            set -e
            [ -n "$LAND_TEST_MOVE_ONCE" ] && [ -f "$LAND_TEST_MOVE_ONCE" ] && exit 0
            [ -n "$LAND_TEST_MOVE_ONCE" ] && touch "$LAND_TEST_MOVE_ONCE"
            d=$(mktemp -d)
            git clone -q "$STUB_REMOTE" "$d"
            cd "$d"
            n=$(date +%s)$$
            echo "$n" > "moved-$n.txt"
            git add -A
            git commit -q -m "main moved under a gate"
            git push -q origin HEAD:main
        """)})
        self.env["LAND_TEST_MOVER"] = path
        return path

    def blocked(self):
        """A branch holding the turn on a Check that waits for a file, so the
        next few can queue behind it and be taken as one batch. The same Check
        holds that batch too, on a second file, where a member adds
        `hold.txt`. Returns both files."""
        gate = os.path.join(self.root, "gate-open")
        held = os.path.join(self.root, "batch-open")
        self.env["ARMADA_LAND_BATCH"] = "4"
        blocker = self.branch("fix/blocker", {
            "ui/block.ts": "1\n",
            "checks/ui.sh": (
                f"while [ ! -f {gate} ]; do sleep 0.1; done\n"
                f"if [ -f hold.txt ]; then while [ ! -f {held} ]; do sleep 0.1; done; fi\n"
            ),
        })
        self.land(blocker, "preflight")
        self.land(blocker)
        deadline = time.monotonic() + 30
        while "running ui" not in self.outcome("fix/blocker").get("detail", ""):
            self.assertLess(time.monotonic(), deadline, "the blocker never reached its Check")
            time.sleep(0.1)
        return gate, held

    def queue(self, names_and_files):
        """Branches cut, preflighted and queued in this order."""
        made = []
        for name, files in names_and_files:
            where = self.branch(name, files)
            self.land(where, "preflight")
            self.land(where)
            made.append(where)
        return made

    def candidate_runs(self):
        """The `test` Check's runs in the candidate worktree, one line each."""
        return [line for line in open(self.env["LAND_TEST_EVIDENCE"]).read().splitlines()
                if line.startswith("check ") and "/land/candidate" in line]

    def read(self, cwd, path):
        with open(os.path.join(cwd, path)) as held:
            return held.read()

    def outcome(self, branch):
        return load(self.state_file("outcomes", key(branch) + ".json"))

    def turns(self, branch):
        """Each turn's own log directory for `branch`, oldest first."""
        return sorted(os.listdir(self.state_file("logs", key(branch))))

    def logged(self, branch):
        """What the latest turn wrote a log for, apart from the merge itself."""
        latest = self.state_file("logs", key(branch), self.turns(branch)[-1])
        return sorted(set(os.listdir(latest)) - {"merge.log"})


# ---------------------------------------------------------------- the claims


class Line(LineFixture):
    def test_two_back_to_back_the_second_reruns_red_and_does_not_merge(self):
        one = self.branch("fix/one", {"one.txt": "1\n"})
        two = self.branch("fix/two", {"two.txt": "2\n"})
        self.land(one, "preflight")
        self.land(two, "preflight")
        self.land(one)
        self.land(two)

        first = self.settle(one, "fix/one")
        self.assertEqual(first.returncode, 0, first.stdout)
        self.assertIn("test.log", self.logged("fix/one"), "main had not moved, and the Check the branch hits still ran")
        second = self.settle(two, "fix/two")
        self.assertEqual(second.returncode, 4, second.stdout)
        self.assertIn("test failed", second.stdout)
        self.assertNotIn("already fails", second.stdout, "this one is the branch's own")
        self.assertIn("test.log", self.logged("fix/two"))
        ran = {check["name"]: check["state"] for check in self.outcome("fix/two")["checks"]}
        self.assertEqual(ran.get("test"), "failed", "each Check's own state is kept with the outcome")

        on_main = self.main_files()
        self.assertIn("one.txt", on_main)
        self.assertNotIn("two.txt", on_main)
        self.assertEqual(self.git(two, "rev-parse", "HEAD"), self.git(two, "ls-remote", "origin", "refs/heads/fix/two").split()[0],
                         "a red gate pushes nothing onto the branch")
        self.assertEqual(load(self.prs)["2"]["state"], "OPEN")
        self.assertEqual(os.listdir(self.state_file("queue")), [], "every exit leaves the line")

    def test_main_unmoved_still_runs_the_checks_the_branch_hits(self):
        where = self.branch("fix/red-alone", {"checks/test.sh": "exit 1\n"})
        self.land(where, "preflight")
        self.land(where)
        done = self.settle(where, "fix/red-alone")
        self.assertEqual(done.returncode, 4, done.stdout)
        self.assertIn("red against main: test failed", done.stdout)
        self.assertIn("test.log", self.logged("fix/red-alone"))
        self.assertEqual(load(self.prs)["1"]["state"], "OPEN")

    def test_main_unmoved_lands_on_the_gate_and_the_branchs_checks(self):
        where = self.branch("fix/alone", {"alone.txt": "1\n"})
        self.land(where, "preflight")
        self.land(where)
        done = self.settle(where, "fix/alone")
        self.assertEqual(done.returncode, 0, done.stdout)
        self.assertIn("merged as", done.stdout)
        logged = self.logged("fix/alone")
        self.assertIn("test.log", logged, "the Check the branch hits ran, though main had not moved")
        self.assertIn("foundations.log", logged, "the gate reads the tree on every turn")
        merge = self.outcome("fix/alone")["merge_commit"]
        self.git(self.repo, "fetch", "--quiet", "origin")
        self.assertEqual(len(self.git(self.repo, "rev-list", "--parents", "-n", "1", merge).split()), 3, "a merge commit, not a rebase")
        self.assertEqual(merge, self.main_head(), "the line pushed main itself")
        message = self.git(self.repo, "log", "-1", "--format=%B", merge)
        self.assertIn("#1", message, "the pull request is named where there is one")
        self.assertIn("Landed-from: fix/alone", message)
        self.assertEqual(load(self.prs)["1"]["state"], "MERGED", "the forge read the push as the merge")
        self.assertEqual(self.git(self.repo, "ls-remote", "origin", "refs/heads/fix/alone"), "", "the remote branch is deleted")
        self.assertIn("git worktree remove", done.stdout)
        self.assertTrue(os.path.isdir(where), "the agent's worktree is never removed")

    def test_a_branch_never_pushed_and_with_no_pull_request_lands(self):
        where = self.branch("fix/local-only", {"local.txt": "1\n"}, pr=False)
        mover = self.branch("fix/moves-local", {"moved.txt": "1\n"})
        self.land(mover, "preflight")
        self.land(mover)
        self.assertEqual(self.settle(mover, "fix/moves-local").returncode, 0)
        self.land(where, "preflight")
        self.land(where)
        done = self.settle(where, "fix/local-only")
        self.assertEqual(done.returncode, 0, done.stdout)
        merge = self.outcome("fix/local-only")["merge_commit"]
        self.assertEqual(merge, self.main_head())
        self.git(self.repo, "fetch", "--quiet", "origin")
        self.assertEqual(self.git(self.repo, "rev-parse", merge + "^1"), self.outcome("fix/local-only")["gated_base"],
                         "what was pushed is the gated candidate on the base it was gated against")
        self.assertEqual(self.git(self.repo, "rev-parse", merge + "^{tree}"),
                         self.git(self.repo, "rev-parse", self.outcome("fix/local-only")["candidate"] + "^{tree}"),
                         "nothing was pushed that was not gated")
        message = self.git(self.repo, "log", "-1", "--format=%B", merge)
        self.assertIn("Landed-from: fix/local-only", message)
        self.assertNotIn("#", message.splitlines()[0], "no pull request to name")
        self.assertEqual(self.git(self.repo, "ls-remote", "origin", "refs/heads/fix/local-only"), "")

    def test_a_branch_with_nothing_ahead_of_main_is_refused(self):
        where = os.path.join(self.root, "wt-empty")
        self.git(self.repo, "worktree", "add", "--quiet", "-b", "fix/empty", where, "origin/main")
        done = self.land(where, "preflight", check=False)
        self.assertEqual(done.returncode, 1)
        self.assertIn("nothing ahead of", done.stderr)

    def test_a_pull_request_the_forge_does_not_close_is_closed_naming_the_merge(self):
        self.env["STUB_GH_NO_DETECT"] = "1"
        self.env["ARMADA_LAND_PR_WAIT"] = "1"
        where = self.branch("fix/not-detected", {"x.txt": "1\n"})
        self.land(where, "preflight")
        self.land(where)
        done = self.settle(where, "fix/not-detected")
        self.assertEqual(done.returncode, 0, done.stdout)
        merge = self.outcome("fix/not-detected")["merge_commit"]
        pr = load(self.prs)["1"]
        self.assertEqual(pr["state"], "CLOSED")
        self.assertIn(merge, pr["comment"])

    def test_a_remote_branch_holding_more_than_landed_is_kept(self):
        where = self.branch("fix/more-on-remote", {"x.txt": "1\n"})
        self.land(where, "preflight")
        other = os.path.join(self.root, "elsewhere")
        sh("git", "clone", "--quiet", "-b", "fix/more-on-remote", self.remote, other, env=self.env)
        self.commit(other, {"later.txt": "pushed from elsewhere\n"}, "more work")
        self.land(where)
        done = self.settle(where, "fix/more-on-remote")
        self.assertEqual(done.returncode, 0, done.stdout)
        self.assertNotIn("later.txt", self.main_files())
        self.assertNotEqual(self.git(self.repo, "ls-remote", "origin", "refs/heads/fix/more-on-remote"), "",
                            "a commit that did not land is not deleted with the branch")
        self.assertEqual(load(self.prs)["1"]["state"], "OPEN", "nor is its pull request closed")
        self.assertIn("did not land", done.stdout)

    def test_a_conflict_stops_and_keeps_its_place(self):
        where = self.branch("fix/clash", {"shared.txt": "branch\n"})
        mover = self.branch("fix/mover", {"shared.txt": "moved\n"})
        self.land(mover, "preflight")
        self.land(mover)
        self.assertEqual(self.settle(mover, "fix/mover").returncode, 0)

        self.land(where, "preflight")
        self.land(where)
        done = self.settle(where, "fix/clash")
        self.assertEqual(done.returncode, 5, done.stdout)
        self.assertIn("shared.txt", done.stdout)
        place = self.outcome("fix/clash")["place"]

        self.git(where, "fetch", "--quiet", "origin")
        sh("git", "merge", "origin/main", cwd=where, env=self.env, check=False)
        self.commit(where, {"shared.txt": "resolved\n"}, "resolve")
        self.land(where, "preflight")
        self.land(where)
        requeued = self.outcome("fix/clash")["place"]
        self.assertEqual(requeued, place, "resubmitted after a conflict, it keeps its place")
        self.assertEqual(self.settle(where, "fix/clash").returncode, 0)

    def test_a_conflicted_generated_file_is_regenerated_not_refused(self):
        header = "GENERATED by `sh regen.sh`. Do not hand-edit.\n"
        self.write(self.repo, {"regen.sh": "cat parts/* > gen.txt\n", "gen.txt": header, "parts/0": header})
        self.git(self.repo, "add", "-A")
        self.git(self.repo, "commit", "--quiet", "-m", "generated")
        self.git(self.repo, "push", "--quiet", "origin", "main")
        where = self.branch("fix/gen-a", {"parts/a": "a\n", "gen.txt": header + "a\n"})
        mover = self.branch("fix/gen-b", {"parts/b": "b\n", "gen.txt": header + "b\n"})
        self.land(mover, "preflight")
        self.land(mover)
        self.assertEqual(self.settle(mover, "fix/gen-b").returncode, 0)
        self.land(where, "preflight")
        self.land(where)
        done = self.settle(where, "fix/gen-a")
        self.assertEqual(done.returncode, 0, done.stdout)
        work = os.path.join(self.root, "gen-check")
        sh("git", "clone", "--quiet", self.remote, work, env=self.env)
        self.assertEqual(open(os.path.join(work, "gen.txt")).read(), header + "a\nb\n")

    def stale_open(self):
        """Main with an `OPEN.md` its gate checks is current, regenerated by `sh open.sh`."""
        header = "GENERATED by `sh open.sh`. Do not hand-edit.\n"
        self.write(self.repo, {
            "open.sh": "cat questions/* > OPEN.md\n",
            "OPEN.md": header, "questions/0": header,
            "foundations.sh": self.read(self.repo, "foundations.sh") + (
                "cat questions/* | cmp -s - OPEN.md || "
                "printf 'FAIL  every open question is collected\\n        missing: OPEN.md\\n'\n"
            ),
        })
        self.git(self.repo, "add", "-A")
        self.git(self.repo, "commit", "--quiet", "-m", "a generated OPEN.md")
        self.git(self.repo, "push", "--quiet", "origin", "main")
        self.env["ARMADA_LAND_REGENERATE"] = "sh open.sh"
        return header

    def test_a_stale_generated_file_is_regenerated_and_lands(self):
        header = self.stale_open()
        where = self.branch("fix/stale-open", {"questions/a": "a\n"})
        self.land(where, "preflight")
        self.land(where)
        done = self.settle(where, "fix/stale-open")
        self.assertEqual(done.returncode, 0, done.stdout)
        landed = self.main_clone()
        self.assertEqual(self.read(landed, "OPEN.md"), header + "a\n")
        self.assertIn("Regenerate", self.git(landed, "log", "--format=%s", "-n", "3"),
                      "the regeneration is a commit of its own, and it is what landed")

    def test_a_stale_generated_file_does_not_carry_another_failure_through(self):
        self.stale_open()
        known = self.read(self.repo, "foundations.txt")
        where = self.branch("fix/stale-and-worse", {
            "questions/a": "a\n",
            "foundations.txt": "FAIL  a new rule\n        missing: a new subject\n" + known,
        })
        pushed = self.git(where, "rev-parse", "HEAD")
        self.land(where, "preflight")
        self.land(where)
        done = self.settle(where, "fix/stale-and-worse")
        self.assertEqual(done.returncode, 4, done.stdout)
        self.assertIn("a new subject", done.stdout)
        self.assertNotIn("missing: OPEN.md", done.stdout, "the regenerated file is not what refused it")
        self.assertEqual(self.git(where, "ls-remote", "origin", "refs/heads/fix/stale-and-worse").split()[0], pushed,
                         "a red turn pushes no regeneration onto the branch")

    def test_a_regeneration_that_fails_is_red_and_says_so(self):
        self.stale_open()
        self.env["ARMADA_LAND_REGENERATE"] = "sh open.sh; sh no-such-generator.sh"
        where = self.branch("fix/regen-fails", {"questions/a": "a\n"})
        self.land(where, "preflight")
        self.land(where)
        done = self.settle(where, "fix/regen-fails")
        self.assertEqual(done.returncode, 4, done.stdout)
        self.assertIn("sh no-such-generator.sh", done.stdout)
        self.assertFalse(os.path.exists(os.path.join(self.main_clone(), "questions", "a")))

    def test_a_killed_runner_gives_the_turn_up(self):
        marker = os.path.join(self.root, "slow-once")
        mover = self.branch("fix/first", {"first.txt": "1\n"})
        where = self.branch("fix/slow", {"checks/test.sh": f"[ -f {marker} ] && exit 0\ntouch {marker}\nsleep 60\n"})
        self.land(mover, "preflight")
        self.land(mover)
        self.assertEqual(self.settle(mover, "fix/first").returncode, 0)
        self.land(where, "preflight")
        self.land(where)

        deadline = time.monotonic() + 30
        while not os.path.exists(marker):
            self.assertLess(time.monotonic(), deadline, "the slow Check never started")
            time.sleep(0.1)
        os.killpg(os.getpgid(self.outcome("fix/slow")["runner"]), signal.SIGKILL)

        done = self.settle(where, "fix/slow")  # --status starts a runner for the turn left behind
        self.assertEqual(done.returncode, 0, done.stdout)
        self.assertIn("first.txt", self.main_files())

    def test_status_while_waiting_answers_at_once(self):
        gate = os.path.join(self.root, "gate-open")
        mover = self.branch("fix/ahead", {"ahead.txt": "1\n"})
        slow = self.branch("fix/held", {"checks/test.sh": f"while [ ! -f {gate} ]; do sleep 0.1; done\n"})
        behind = self.branch("feature/behind/deep", {"behind.txt": "1\n"})
        self.land(mover, "preflight")
        self.land(mover)
        self.assertEqual(self.settle(mover, "fix/ahead").returncode, 0)
        self.land(slow, "preflight")
        self.land(slow)
        self.land(behind, "preflight")
        self.land(behind)

        started = time.monotonic()
        done = self.land(behind, "--status", check=False)
        self.assertLess(time.monotonic() - started, 1.0)
        self.assertEqual(done.returncode, 3, done.stdout)
        self.assertIn("feature/behind/deep: waiting", done.stdout)
        self.assertIn("1. fix/held", done.stdout)
        self.assertIn("2. feature/behind/deep", done.stdout)

        open(gate, "w").close()
        self.assertEqual(self.settle(slow, "fix/held").returncode, 0)
        self.assertEqual(self.settle(behind, "feature/behind/deep").returncode, 0)

    def test_only_a_new_failing_foundations_line_is_red(self):
        known = "FAIL  a rule main already fails\n        missing: its subject\n"
        mover = self.branch("fix/move", {"moved.txt": "1\n"})
        warned = self.branch("fix/warned", {"foundations.txt": known + "        warn:    a new warning\n\nverify-foundations: RED — 1 failing, 1 warning\n"})
        worse = self.branch("fix/worse", {"foundations.txt": "FAIL  a new rule\n        missing: a new subject\n" + known + "\nverify-foundations: RED — 1 failing, 0 warning\n"})
        for where, name in ((mover, "fix/move"), (warned, "fix/warned")):
            self.land(where, "preflight")
            self.land(where)
            done = self.settle(where, name)
            self.assertEqual(done.returncode, 0, done.stdout)
        self.land(worse, "preflight")
        self.land(worse)
        done = self.settle(worse, "fix/worse")
        self.assertEqual(done.returncode, 4, done.stdout)
        self.assertIn("missing: a new subject", done.stdout)
        self.assertNotIn("its subject", done.stdout.replace("a new subject", ""))

    def test_a_new_foundations_line_is_red_without_running_a_check(self):
        known = self.read(self.repo, "foundations.txt")
        where = self.branch("fix/gate-only", {
            "foundations.txt": "FAIL  a new rule\n        missing: a new subject\n" + known,
        })
        self.land(where, "preflight")
        self.land(where)
        done = self.settle(where, "fix/gate-only")
        self.assertEqual(done.returncode, 4, done.stdout)
        self.assertIn("missing: a new subject", done.stdout)
        self.assertIn("no Check was run", done.stdout)
        self.assertIn("foundations.log", self.logged("fix/gate-only"))
        self.assertNotIn("test.log", self.logged("fix/gate-only"), "no Check log")
        self.assertEqual(self.candidate_runs(), [], "no Check ran in the candidate")
        self.assertFalse(os.path.exists(os.path.join(self.repo, ".armada", "land", "candidate", "marker.stamp")),
                         "nor was it set up for one")

    def test_withdraw_takes_a_waiting_entry_out_of_the_line(self):
        gate, _ = self.blocked()
        (where,) = self.queue([("fix/changed-my-mind", {"mind.txt": "1\n"})])
        done = self.land(where, "--withdraw")
        self.assertIn("withdrew fix/changed-my-mind", done.stdout)
        self.assertNotIn(key("fix/changed-my-mind") + ".json", os.listdir(self.state_file("queue")))
        status = self.land(where, "--status", check=False)
        self.assertEqual(status.returncode, 7, status.stdout)
        self.assertIn("withdrawn", status.stdout)
        open(gate, "w").close()
        self.assertEqual(self.settle(self.repo, "fix/blocker").returncode, 0)
        self.assertNotIn("mind.txt", self.main_files())
        again = self.land(where, "--withdraw", "fix/changed-my-mind")
        self.assertIn("not in line", again.stdout)

    def test_a_check_only_one_side_hits_still_reruns(self):
        """The union: main lands under `ui/`, the branch never touches it."""
        mover = self.branch("fix/ui-landed", {"ui/button.ts": "1\n"})
        where = self.branch("fix/no-ui", {"elsewhere.txt": "1\n"})
        self.land(mover, "preflight")
        self.land(mover)
        self.assertEqual(self.settle(mover, "fix/ui-landed").returncode, 0)
        self.land(where, "preflight")
        self.land(where)
        self.assertEqual(self.settle(where, "fix/no-ui").returncode, 0)
        self.assertIn("ui.log", self.logged("fix/no-ui"), "a Check only the base hits still reruns")

    def test_what_a_regeneration_writes_besides_the_conflict_is_merged(self):
        header = "GENERATED by `sh regen.sh`. Do not hand-edit.\n"
        self.write(self.repo, {
            "regen.sh": "cat parts/* > gen.txt\nls parts > index.txt\n",
            "gen.txt": header, "index.txt": "0\n", "parts/0": header,
        })
        self.git(self.repo, "add", "-A")
        self.git(self.repo, "commit", "--quiet", "-m", "generated, with a second output")
        self.git(self.repo, "push", "--quiet", "origin", "main")
        where = self.branch("fix/gen-a", {"parts/a": "a\n", "gen.txt": header + "a\n"})
        mover = self.branch("fix/gen-b", {"parts/b": "b\n", "gen.txt": header + "b\n"})
        for wt, name in ((mover, "fix/gen-b"), (where, "fix/gen-a")):
            self.land(wt, "preflight")
            self.land(wt)
            self.assertEqual(self.settle(wt, name).returncode, 0)
        landed = self.main_clone()
        self.assertEqual(open(os.path.join(landed, "gen.txt")).read(), header + "a\nb\n")
        self.assertEqual(open(os.path.join(landed, "index.txt")).read(), "0\na\nb\n",
                         "the regeneration's other output is in the commit, not only in the gate")

    def test_a_foundations_run_that_names_no_rule_is_red(self):
        mover = self.branch("fix/moves-1", {"moved.txt": "1\n"})
        broken = self.branch("fix/breaks-the-gate", {"foundations.sh": "echo 'error[E0433]: cannot find `covers`' >&2\nexit 101\n"})
        self.land(mover, "preflight")
        self.land(mover)
        self.assertEqual(self.settle(mover, "fix/moves-1").returncode, 0)
        self.land(broken, "preflight")
        self.land(broken)
        done = self.settle(broken, "fix/breaks-the-gate")
        self.assertEqual(done.returncode, 4, done.stdout)
        self.assertIn("naming no failing rule", done.stdout)
        self.assertIn("test.log", self.logged("fix/breaks-the-gate"),
                      "a gate that could not run does not hide the Checks beside it")
        self.assertNotIn("moved.txt", self.main_files() - {"moved.txt"} or set())

    def test_a_branch_that_breaks_the_gate_is_refused_on_an_unmoved_turn(self):
        where = self.branch("fix/breaks-the-gate-alone", {"foundations.sh": "exit 101\n"})
        self.land(where, "preflight")
        self.land(where)
        done = self.settle(where, "fix/breaks-the-gate-alone")
        self.assertEqual(done.returncode, 4, done.stdout)
        self.assertNotIn("foundations.sh", self.main_files() - {"foundations.sh"} or set())
        self.assertIn("test.log", self.logged("fix/breaks-the-gate-alone"), "and the Check the branch hits ran beside it")

    def test_a_base_whose_own_gate_cannot_run_stops_rather_than_reds(self):
        # Broken on main by a hand merge, which is what the guard hook refuses.
        hand = os.path.join(self.root, "hand-merge")
        sh("git", "clone", "--quiet", self.remote, hand, env=self.env)
        self.write(hand, {"foundations.sh": "exit 101\n"})
        self.git(hand, "add", "-A")
        self.git(hand, "commit", "--quiet", "-m", "break the gate on main")
        self.git(hand, "push", "--quiet", "origin", "HEAD:main")
        after = self.branch("fix/after-base", {"after.txt": "1\n"})
        self.land(after, "preflight")
        self.land(after)
        done = self.settle(after, "fix/after-base")
        self.assertEqual(done.returncode, 7, done.stdout)
        self.assertIn("main itself is broken", done.stdout)
        self.assertIn("not at fault", done.stdout, "the branch behind a broken main is not the one to fix")

    def test_a_cached_base_run_that_is_not_a_report_is_taken_again(self):
        mover = self.branch("fix/moves-2", {"moved.txt": "1\n"})
        where = self.branch("fix/after-empty-cache", {"after.txt": "1\n"})
        self.land(mover, "preflight")
        self.land(mover)
        self.assertEqual(self.settle(mover, "fix/moves-2").returncode, 0)
        os.makedirs(self.state_file("foundations"), exist_ok=True)
        with open(self.state_file("foundations", self.main_head() + ".txt"), "w") as out:
            out.write("error: the runner was killed half-way through\n")  # never a report
        self.land(where, "preflight")
        self.land(where)
        done = self.settle(where, "fix/after-empty-cache")
        self.assertEqual(done.returncode, 0, done.stdout)

    def test_a_renumbered_finding_is_not_a_new_one(self):
        known = "FAIL  a rule main already fails\n        missing: a/b.rs:10 — over 500\n"
        self.write(self.repo, {"foundations.txt": known})
        self.git(self.repo, "add", "-A")
        self.git(self.repo, "commit", "--quiet", "-m", "a numbered finding")
        self.git(self.repo, "push", "--quiet", "origin", "main")
        mover = self.branch("fix/moves-3", {"moved.txt": "1\n"})
        shifted = self.branch("fix/shifts-a-line", {"foundations.txt": known.replace(":10", ":12")})
        self.land(mover, "preflight")
        self.land(mover)
        self.assertEqual(self.settle(mover, "fix/moves-3").returncode, 0)
        self.land(shifted, "preflight")
        self.land(shifted)
        done = self.settle(shifted, "fix/shifts-a-line")
        self.assertEqual(done.returncode, 0, done.stdout)

    def test_a_runner_killed_after_the_push_finds_it_landed_on_the_next_turn(self):
        # The remote's own hook kills the pushing runner's group once main has moved.
        where = self.branch("fix/killed-after-push", {"x.txt": "1\n"})
        hook = os.path.join(self.remote, "hooks", "post-receive")
        self.write(self.root, {"post-receive": (
            "#!/bin/sh\n"
            "grep -q ' refs/heads/main$' || exit 0\n"
            '[ -n "$LAND_TEST_KILL_AFTER_PUSH" ] && [ ! -f "$LAND_TEST_KILL_AFTER_PUSH" ] || exit 0\n'
            'touch "$LAND_TEST_KILL_AFTER_PUSH"\n'
            "kill -9 -$(ps -o pgid= $$ | tr -d ' ')\n"
        )})
        shutil.copy(os.path.join(self.root, "post-receive"), hook)
        os.chmod(hook, 0o755)
        self.env["LAND_TEST_KILL_AFTER_PUSH"] = os.path.join(self.root, "killed-once")
        self.land(where, "preflight")
        self.land(where)
        done = self.settle(where, "fix/killed-after-push")
        self.assertTrue(os.path.exists(self.env["LAND_TEST_KILL_AFTER_PUSH"]), "the runner was killed after its push")
        self.assertEqual(done.returncode, 0, done.stdout)
        self.assertIn("x.txt", self.main_files())
        self.git(self.repo, "fetch", "--quiet", "origin")
        self.assertEqual(self.git(self.repo, "rev-list", "--count", "--merges", "origin/main"), "1",
                         "the next turn found it landed rather than merging it twice")
        self.assertEqual(self.git(where, "ls-remote", "origin", "refs/heads/fix/killed-after-push"), "")
    def test_main_moving_mid_gate_gates_again_against_it(self):
        once = os.path.join(self.root, "moved-once")
        self.mover()
        mover = self.branch("fix/moves-4", {"moved.txt": "1\n"})
        where = self.branch("fix/races-main", {"checks/test.sh": 'sh "$LAND_TEST_MOVER"\n'})
        self.env["LAND_TEST_MOVE_ONCE"] = once
        self.land(mover, "preflight")
        self.land(mover)
        self.assertEqual(self.settle(mover, "fix/moves-4").returncode, 0)
        self.land(where, "preflight")
        self.land(where)
        done = self.settle(where, "fix/races-main", timeout=90)
        self.assertEqual(done.returncode, 0, done.stdout)
        landed = self.outcome("fix/races-main")["merge_commit"]
        self.git(self.repo, "fetch", "--quiet", "origin")
        self.assertEqual(self.git(self.repo, "rev-parse", landed + "^1"),
                         self.outcome("fix/races-main")["gated_base"],
                         "the second gate's base is what it merged onto")

    def test_main_moving_every_round_gives_the_turn_up(self):
        self.mover()
        mover = self.branch("fix/moves-5", {"moved.txt": "1\n"})
        where = self.branch("fix/never-quiet", {"checks/test.sh": 'sh "$LAND_TEST_MOVER"\n'})
        self.land(mover, "preflight")
        self.land(mover)
        self.assertEqual(self.settle(mover, "fix/moves-5").returncode, 0)
        self.land(where, "preflight")
        self.land(where)
        done = self.settle(where, "fix/never-quiet", timeout=120)
        self.assertEqual(done.returncode, 7, done.stdout)
        self.assertIn("moved during each of", done.stdout)

    def test_a_commit_to_the_branch_mid_gate_stops_before_merging(self):
        gate = os.path.join(self.root, "gate-open")
        mover = self.branch("fix/moves-7", {"moved.txt": "1\n"})
        where = self.branch("fix/pushed-under", {"checks/test.sh": f'while [ ! -f {gate} ]; do sleep 0.1; done\n'})
        self.land(mover, "preflight")
        self.land(mover)
        self.assertEqual(self.settle(mover, "fix/moves-7").returncode, 0)
        self.land(where, "preflight")
        self.land(where)
        deadline = time.monotonic() + 30
        while self.outcome("fix/pushed-under").get("state") != "gating" or "running test" not in self.outcome("fix/pushed-under")["detail"]:
            self.assertLess(time.monotonic(), deadline, "the Check never started")
            time.sleep(0.1)
        self.commit(where, {"late.txt": "written while it was gated\n"}, "more work", push=False)
        open(gate, "w").close()
        done = self.settle(where, "fix/pushed-under")
        self.assertEqual(done.returncode, 7, done.stdout)
        self.assertIn("while it was gated", done.stdout)
        self.assertNotIn("late.txt", self.main_files())

    def test_the_manifest_gates_both_of_the_line_s_suites(self):
        """Asserted here because this file may name the agent harness's own
        directory, and the Rust tests under `crates/` may not."""
        here = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
        manifest = open(os.path.join(here, "armada.yml")).read()
        scripts = manifest.split("scripts_test:")[1].split("hooks_test:")[0]
        hooks = manifest.split("hooks_test:")[1].split("format:")[0]
        self.assertIn("run: python3 scripts/test_land.py", scripts)
        self.assertIn('- "scripts/**"', scripts)
        self.assertIn("run: python3 .claude/hooks/test_guard_merge.py", hooks)
        self.assertIn('- ".claude/hooks/**"', hooks)
        self.assertTrue(os.path.exists(os.path.join(here, ".claude/hooks/test_guard_merge.py")))

    def test_a_check_whose_command_is_missing_stops_rather_than_reds(self):
        mover = self.branch("fix/moves-11", {"moved.txt": "1\n"})
        where = self.branch("fix/no-such-tool", {
            "checks/test.sh": 'echo "error: no such command: nextest" >&2\nexit 101\n',
            "wanted.txt": "this must not land\n",
        })
        self.land(mover, "preflight")
        self.land(mover)
        self.assertEqual(self.settle(mover, "fix/moves-11").returncode, 0)
        self.land(where, "preflight")
        self.land(where)
        done = self.settle(where, "fix/no-such-tool")
        self.assertEqual(done.returncode, 7, done.stdout)
        self.assertIn("nextest", done.stdout)
        self.assertIn("not on this", done.stdout)
        self.assertNotIn("wanted.txt", self.main_files(), "a tool nobody installed merges nothing")
        self.assertNotEqual(self.git(where, "ls-remote", "origin", "refs/heads/fix/no-such-tool"), "",
                            "nothing was merged, so the branch is still there")

    def test_a_worktree_an_older_script_left_is_swept(self):
        left = os.path.join(self.repo, ".armada", "gates", "left-behind")
        self.git(self.repo, "worktree", "add", "--quiet", "--detach", left, "HEAD")
        where = self.branch("fix/after-a-death", {"x.txt": "1\n"})
        self.land(where, "preflight")
        self.land(where)
        self.assertEqual(self.settle(where, "fix/after-a-death").returncode, 0)
        self.assertFalse(os.path.exists(left), "the first turn to hold the lock takes it back")

    def test_the_worktree_is_reused_with_its_build_and_none_of_the_last_turn(self):
        # One unmoved turn to move `main`, then two gated turns with a red one
        # between them: what the red turn checked out may not survive either.
        moves = self.branch("fix/turn-moves", {"a.txt": "1\n"})
        first = self.branch("fix/turn-one", {"b.txt": "2\n"})
        red = self.branch("fix/turn-red", {"red-only.txt": "3\n", "checks/test.sh": "exit 1\n"})
        second = self.branch("fix/turn-two", {"d.txt": "4\n"})
        for wt, name, code in ((moves, "fix/turn-moves", 0), (first, "fix/turn-one", 0),
                               (red, "fix/turn-red", 4), (second, "fix/turn-two", 0)):
            self.land(wt, "preflight")
            self.land(wt)
            self.assertEqual(self.settle(wt, name).returncode, code, name)

        runs = [dict(part.split("=", 1) for part in line.split()[1:])
                for line in open(self.env["LAND_TEST_EVIDENCE"]).read().splitlines()
                if line.startswith("check ")]
        # The candidate worktree's own runs: `main`'s own tree is the other one.
        runs = [run for run in runs if run["at"].endswith("/land/candidate")]
        self.assertGreaterEqual(len(runs), 2, "two gated turns ran the Check")
        self.assertEqual(runs[-1]["stray"], "no", "the last turn's files are gone")
        self.assertEqual(runs[-1]["node"], "yes", "its build directories are not")
        self.assertEqual(runs[-1]["seed"], "yes", "nor the seed")
        self.assertFalse(os.path.exists(os.path.join(self.repo, ".armada", "land", "candidate", "red-only.txt")),
                         "and nothing a turn that went red checked out")

    def test_a_killed_turn_leaves_nothing_in_the_worktree_for_the_next(self):
        once = os.path.join(self.root, "slept-once")
        mover = self.branch("fix/moves-12", {"moved.txt": "1\n"})
        where = self.branch("fix/killed-mid-turn", {
            "checks/test.sh": (
                f'printf "killed stray=%s\\n" "$([ -f half.stamp ] && echo yes || echo no)" '
                '>> "$LAND_TEST_EVIDENCE"\n'
                f'[ -f {once} ] && exit 0\n'
                f"touch {once}\n"
                "touch half.stamp\n"
                "sleep 60\n"
            ),
        })
        self.land(mover, "preflight")
        self.land(mover)
        self.assertEqual(self.settle(mover, "fix/moves-12").returncode, 0)
        self.land(where, "preflight")
        self.land(where)

        deadline = time.monotonic() + 30
        while not os.path.exists(once):
            self.assertLess(time.monotonic(), deadline, "the slow Check never started")
            time.sleep(0.1)
        os.killpg(os.getpgid(self.outcome("fix/killed-mid-turn")["runner"]), signal.SIGKILL)

        done = self.settle(where, "fix/killed-mid-turn")
        self.assertEqual(done.returncode, 0, done.stdout)
        killed = [line for line in open(self.env["LAND_TEST_EVIDENCE"]).read().splitlines()
                  if line.startswith("killed ")]
        self.assertEqual(killed[-1], "killed stray=no", "the dead turn's files went with it")

    def test_a_worktree_that_is_gone_or_broken_is_remade(self):
        one = self.branch("fix/remade-one", {"a.txt": "1\n"})
        two = self.branch("fix/remade-two", {"b.txt": "2\n"})
        three = self.branch("fix/remade-three", {"c.txt": "3\n"})
        self.land(one, "preflight")
        self.land(one)
        self.assertEqual(self.settle(one, "fix/remade-one").returncode, 0)

        # Registered, and no longer on disk.
        shutil.rmtree(os.path.join(self.repo, ".armada", "land", "candidate"))
        self.land(two, "preflight")
        self.land(two)
        self.assertEqual(self.settle(two, "fix/remade-two").returncode, 0)

        # There, and not a worktree any more.
        with open(os.path.join(self.repo, ".armada", "land", "candidate", ".git"), "w") as out:
            out.write("gitdir: /nowhere\n")
        self.land(three, "preflight")
        self.land(three)
        self.assertEqual(self.settle(three, "fix/remade-three").returncode, 0)

    def test_a_check_already_red_on_main_is_not_the_branchs_fault(self):
        after = self.branch("fix/behind-a-red-main", {"after.txt": "1\n"})
        self.onto_main({"checks/test.sh": "exit 1\n"}, "break test on main")

        self.land(after, "preflight")
        self.land(after)
        done = self.settle(after, "fix/behind-a-red-main")
        self.assertEqual(done.returncode, 7, done.stdout)
        self.assertIn("already fails on main", done.stdout)
        self.assertIn("not this branch's", done.stdout)
        self.assertNotIn("after.txt", self.main_files(), "nothing merges either way")

    def test_a_check_past_its_limit_is_red_and_gives_the_turn_up(self):
        # The grandchild leads a group of its own, as `armada check` puts a
        # Check's command in one, so killing only the runner's child would miss it.
        pid_file = os.path.join(self.root, "hung.pid")
        self.env["ARMADA_LAND_CHECK_LIMIT"] = "2"
        mover = self.branch("fix/moves-hung", {"moved.txt": "1\n"})
        hung = self.branch("fix/hung", {"checks/test.sh": (
            "python3 -c 'import os, time; os.setpgid(0, 0); "
            f"open(\"{pid_file}\", \"w\").write(str(os.getpid())); time.sleep(600)' &\n"
            "sleep 600\n"
        )})
        self.land(mover, "preflight")
        self.land(mover)
        self.assertEqual(self.settle(mover, "fix/moves-hung").returncode, 0)
        behind = self.branch("fix/behind-hung", {"behind.txt": "1\n"})
        for where in (hung, behind):
            self.land(where, "preflight")
            self.land(where)

        done = self.settle(hung, "fix/hung")
        self.assertEqual(done.returncode, 4, done.stdout)
        self.assertIn("test timed out after 2 seconds", done.stdout)
        self.assertNotIn("already fails", done.stdout, "main's own run of it passed")
        ran = {check["name"]: check["state"] for check in self.outcome("fix/hung")["checks"]}
        self.assertEqual(ran.get("test"), "timed_out")
        with open(pid_file) as held:
            pid = int(held.read())
        deadline = time.monotonic() + 5
        while True:
            try:
                os.kill(pid, 0)
            except ProcessLookupError:
                break
            self.assertLess(time.monotonic(), deadline, "the Check's own process group outlived its limit")
            time.sleep(0.1)
        self.assertEqual(self.settle(behind, "fix/behind-hung").returncode, 0,
                         "the next entry gets its turn")

    def test_a_check_past_its_limit_on_main_too_is_mains(self):
        self.env["ARMADA_LAND_CHECK_LIMIT"] = "2"
        after = self.branch("fix/behind-a-hung-main", {"after.txt": "1\n"})
        self.onto_main({"checks/test.sh": "sleep 600\n"}, "hang test on main")

        self.land(after, "preflight")
        self.land(after)
        done = self.settle(after, "fix/behind-a-hung-main")
        self.assertEqual(done.returncode, 7, done.stdout)
        self.assertIn("test timed out after 2 seconds", done.stdout)
        self.assertIn("test timed out on main itself, past its limit of 2 seconds", done.stdout)
        self.assertNotIn("already fails", done.stdout, "a timeout is not a failure")

    def test_a_timeout_on_main_is_worded_as_one(self):
        # The branch's own run fails at once; main's hangs. What the turn says
        # is that main timed out, not that main fails the Check.
        self.env["ARMADA_LAND_CHECK_LIMIT"] = "2"
        after = self.branch("fix/fails-beside-a-hung-main", {"after.txt": "1\n"})
        self.onto_main({"checks/test.sh": "[ -f after.txt ] && exit 1\nsleep 600\n"}, "hang test on main")

        self.land(after, "preflight")
        self.land(after)
        done = self.settle(after, "fix/fails-beside-a-hung-main")
        self.assertEqual(done.returncode, 7, done.stdout)
        self.assertIn("test timed out on main itself, past its limit of 2 seconds", done.stdout)
        self.assertNotIn("already fails", done.stdout)
        on_main = next(path for path in self.outcome("fix/fails-beside-a-hung-main")["logs"]
                       if path.endswith("/test-on-main.log"))
        turn = self.turns("fix/fails-beside-a-hung-main")[-1]
        self.assertIn(os.path.join(key("fix/fails-beside-a-hung-main"), turn), on_main,
                      "main's rerun logs into the turn's own directory")
        self.assertTrue(os.path.exists(on_main))

    def test_a_check_past_its_limit_on_main_is_not_remembered_against_it(self):
        # Main's run hangs once and passes after, as a slow machine would; the
        # second branch on the same main commit is asked about again, not told
        # main is broken.
        self.env["ARMADA_LAND_CHECK_LIMIT"] = "2"
        once = os.path.join(self.root, "hung-once")
        first = self.branch("fix/first-behind-a-slow-main", {"first.txt": "1\n"})
        second = self.branch("fix/second-behind-a-slow-main", {"second.txt": "1\n"})
        self.onto_main({"checks/test.sh": (
            "{ [ -f first.txt ] || [ -f second.txt ]; } && exit 1\n"
            f"[ -f {once} ] && exit 0\n"
            f"touch {once}\n"
            "sleep 600\n"
        )}, "a test that hangs on main once")

        self.land(first, "preflight")
        self.land(first)
        done = self.settle(first, "fix/first-behind-a-slow-main")
        self.assertEqual(done.returncode, 7, done.stdout)
        self.assertIn("test timed out on main itself", done.stdout, "this turn still reads it as main's")

        self.land(second, "preflight")
        self.land(second)
        done = self.settle(second, "fix/second-behind-a-slow-main")
        self.assertEqual(done.returncode, 4, done.stdout)
        self.assertIn("test failed", done.stdout)
        self.assertNotIn("already fails", done.stdout, "main's timeout was not cached as red")

    def test_one_turn_says_the_branchs_red_and_mains_together(self):
        self.write(self.repo, {"checks/ui.sh": "! { [ -f ui/a ] && [ -f ui/b ]; }\n"})
        self.git(self.repo, "add", "-A")
        self.git(self.repo, "commit", "--quiet", "-m", "a scoped Check")
        self.git(self.repo, "push", "--quiet", "origin", "main")
        mine = self.branch("fix/red-mine", {"ui/b": "1\n"})
        self.onto_main({"checks/test.sh": "exit 1\n", "ui/a": "1\n"}, "break test and half of ui on main")

        self.land(mine, "preflight")
        self.land(mine)
        done = self.settle(mine, "fix/red-mine")
        self.assertEqual(done.returncode, 4, done.stdout)
        self.assertIn("ui failed", done.stdout)
        self.assertIn("test already fails on main", done.stdout)
        self.assertNotIn("ui/b", self.main_files())

    def test_a_relative_binary_is_refused_rather_than_traced(self):
        where = self.branch("fix/relative", {"x.txt": "1\n"})
        self.env["ARMADA_LAND_ARMADA"] = "target/debug/armada"
        done = self.land(where, "preflight", check=False)
        self.assertEqual(done.returncode, 1)
        self.assertIn("absolute path", done.stderr)
        self.assertNotIn("Traceback", done.stderr)

    def test_an_entry_missing_its_place_does_not_stop_the_line(self):
        where = self.branch("fix/beside-a-bad-entry", {"x.txt": "1\n"})
        self.land(where, "preflight")
        os.makedirs(self.state_file("queue"), exist_ok=True)
        with open(self.state_file("queue", "half-written.json"), "w") as out:
            out.write('{"branch": "fix/half"}')
        self.land(where)
        self.assertEqual(self.settle(where, "fix/beside-a-bad-entry").returncode, 0)

    def test_a_red_branch_keeps_its_place(self):
        one = self.branch("fix/one", {"one.txt": "1\n"})
        two = self.branch("fix/two", {"two.txt": "2\n"})
        for wt, name in ((one, "fix/one"), (two, "fix/two")):
            self.land(wt, "preflight")
            self.land(wt)
        self.assertEqual(self.settle(one, "fix/one").returncode, 0)
        self.assertEqual(self.settle(two, "fix/two").returncode, 4)
        place = self.outcome("fix/two")["place"]
        self.git(two, "fetch", "--quiet", "origin")
        self.git(two, "merge", "--no-edit", "origin/main")
        self.commit(two, {"checks/test.sh": "exit 0\n"}, "make the combination pass")
        self.land(two, "preflight")
        self.land(two)
        self.assertEqual(self.outcome("fix/two")["place"], place)
        self.assertEqual(self.settle(two, "fix/two").returncode, 0)

    def test_a_red_turn_keeps_its_logs_after_the_rerun_lands(self):
        one = self.branch("fix/one", {"one.txt": "1\n"})
        two = self.branch("fix/two", {"two.txt": "2\n"})
        for wt in (one, two):
            self.land(wt, "preflight")
            self.land(wt)
        self.assertEqual(self.settle(one, "fix/one").returncode, 0)
        self.assertEqual(self.settle(two, "fix/two").returncode, 4)
        red_logs = self.outcome("fix/two")["logs"]
        red_test_log = next(path for path in red_logs if path.endswith("/test.log"))
        red_said = self.read(self.root, red_test_log)

        self.git(two, "fetch", "--quiet", "origin")
        self.git(two, "merge", "--no-edit", "origin/main")
        self.commit(two, {"checks/test.sh": "exit 0\n"}, "make the combination pass")
        self.land(two, "preflight")
        self.land(two)
        self.assertEqual(self.settle(two, "fix/two").returncode, 0)

        self.assertEqual(len(self.turns("fix/two")), 2, "one directory a turn")
        for path in red_logs:
            self.assertTrue(os.path.exists(path), f"the red turn's {path} outlived the rerun")
        self.assertEqual(self.read(self.root, red_test_log), red_said, "and was not overwritten")
        green_logs = self.outcome("fix/two")["logs"]
        self.assertTrue(green_logs)
        self.assertFalse(set(green_logs) & set(red_logs), "the outcome points at its own turn")

    def test_both_sides_of_the_comparison_are_prepared_the_same_way(self):
        mover = self.branch("fix/moves-8", {"moved.txt": "1\n"})
        where = self.branch("fix/prepared", {"x.txt": "1\n"})
        self.land(mover, "preflight")
        self.land(mover)
        self.assertEqual(self.settle(mover, "fix/moves-8").returncode, 0)
        self.land(where, "preflight")
        self.land(where)
        self.assertEqual(self.settle(where, "fix/prepared").returncode, 0)

        ran = [line.split() for line in open(self.env["LAND_TEST_EVIDENCE"]).read().splitlines()]
        foundations = [line for line in ran if line[0] == "foundations"]
        checks = [line for line in ran if line[0] == "check"]
        self.assertGreaterEqual(len(foundations), 2, "each turn reads the base's own run and its own tree's")
        for line in foundations:
            self.assertEqual(line[1:3], ["seed=yes", "setup=no"],
                             "both sides are seeded, and neither is installed into")
        self.assertTrue(checks and all(line[1:3] == ["seed=yes", "setup=yes"] for line in checks),
                        "a Check runs after setup")
        inside = os.path.join(os.path.realpath(self.repo), ".armada", "land")
        for line in foundations + checks:
            self.assertTrue(line[3].startswith("at=" + inside), line)

    def test_a_check_that_writes_a_tracked_file_stops_the_turn(self):
        mover = self.branch("fix/moves-9", {"moved.txt": "1\n"})
        where = self.branch("fix/writes-in-the-gate", {"checks/test.sh": "echo written-by-a-check >> shared.txt\n"})
        self.land(mover, "preflight")
        self.land(mover)
        self.assertEqual(self.settle(mover, "fix/moves-9").returncode, 0)
        self.land(where, "preflight")
        self.land(where)
        done = self.settle(where, "fix/writes-in-the-gate")
        self.assertEqual(done.returncode, 7, done.stdout)
        self.assertIn("the Checks left files", done.stdout)
        self.assertNotIn("checks", self.main_files() - {"checks"} or set())

    def test_a_second_instance_of_a_known_finding_is_new(self):
        known = "FAIL  no vendor literal outside adapters\n        missing: crates/a.rs:10 — `openai`\n"
        self.write(self.repo, {"foundations.txt": known})
        self.git(self.repo, "add", "-A")
        self.git(self.repo, "commit", "--quiet", "-m", "one violation on main")
        self.git(self.repo, "push", "--quiet", "origin", "main")
        mover = self.branch("fix/moves-10", {"moved.txt": "1\n"})
        second = self.branch("fix/one-more", {"foundations.txt": known + "        missing: crates/a.rs:80 — `openai`\n"})
        self.land(mover, "preflight")
        self.land(mover)
        self.assertEqual(self.settle(mover, "fix/moves-10").returncode, 0)
        self.land(second, "preflight")
        self.land(second)
        done = self.settle(second, "fix/one-more")
        self.assertEqual(done.returncode, 4, done.stdout)
        self.assertIn("crates/a.rs:80", done.stdout)

    def test_a_dirty_tree_or_a_missing_stamp_is_refused(self):
        where = self.branch("fix/dirty", {"x.txt": "1\n"})
        self.assertEqual(self.land(where, check=False).returncode, 1)
        self.write(where, {"stray.txt": "untracked\n"})
        done = self.land(where, "preflight", check=False)
        self.assertEqual(done.returncode, 1)
        self.assertIn("stray.txt", done.stderr)


class Batch(LineFixture):
    def test_three_greens_land_in_one_turn(self):
        gate, held = self.blocked()
        before = len(self.candidate_runs())
        names = ["fix/batch-a", "fix/batch-b", "fix/batch-c"]
        wts = self.queue([
            ("fix/batch-a", {"a.txt": "1\n", "hold.txt": "1\n"}),
            ("fix/batch-b", {"b.txt": "1\n"}),
            ("fix/batch-c", {"c.txt": "1\n"}),
        ])
        open(gate, "w").close()
        self.assertEqual(self.settle(self.repo, "fix/blocker").returncode, 0)

        deadline = time.monotonic() + 30
        while "running ui" not in self.outcome("fix/batch-c").get("detail", ""):
            self.assertLess(time.monotonic(), deadline, "the batch never reached its Check")
            time.sleep(0.1)
        status = self.land(wts[2], "--status", check=False).stdout
        for name in names:
            self.assertIn(f"{name}  #", status)
        self.assertIn("together with fix/batch-a, fix/batch-b", status, "--status says who gates together")
        open(held, "w").close()

        for where, name in zip(wts, names):
            done = self.settle(where, name)
            self.assertEqual(done.returncode, 0, done.stdout)
        self.assertEqual(len(self.candidate_runs()) - before, 1, "one run for the three")
        merges = [self.outcome(name)["merge_commit"] for name in names]
        self.assertEqual(merges[-1], self.main_head(), "one push, of the last member's merge")
        self.git(self.repo, "fetch", "--quiet", "origin")
        first_parents = self.git(self.repo, "log", "--first-parent", "--format=%H", "-n", "3", "origin/main").split()
        self.assertEqual(first_parents, list(reversed(merges)), "a merge commit each, in place order")
        for merge, name in zip(merges, names):
            self.assertIn(f"Landed-from: {name}", self.git(self.repo, "log", "-1", "--format=%B", merge))

    def test_one_red_among_three_lands_the_other_two(self):
        gate, held = self.blocked()
        names = ["fix/green-a", "fix/red-b", "fix/green-c"]
        wts = self.queue([
            ("fix/green-a", {"a.txt": "1\n"}),
            ("fix/red-b", {"red.txt": "1\n", "checks/test.sh": "! [ -f red.txt ]\n"}),
            ("fix/green-c", {"c.txt": "1\n"}),
        ])
        open(gate, "w").close()
        open(held, "w").close()
        codes = [self.settle(where, name).returncode for where, name in zip(wts, names)]
        self.assertEqual(codes, [0, 4, 0])
        self.assertIn("test failed", self.settle(wts[1], "fix/red-b").stdout, "reported red to its own agent")
        on_main = self.main_files()
        self.assertIn("a.txt", on_main)
        self.assertIn("c.txt", on_main)
        self.assertNotIn("red.txt", on_main)

    def test_two_members_that_conflict_with_each_other_both_land_in_order(self):
        gate, held = self.blocked()
        a, b = self.queue([
            ("fix/clash-a", {"shared.txt": "a\n"}),
            ("fix/clash-b", {"shared.txt": "b\n"}),
        ])
        open(gate, "w").close()
        open(held, "w").close()
        self.assertEqual(self.settle(a, "fix/clash-a").returncode, 0)
        done = self.settle(b, "fix/clash-b")
        self.assertEqual(done.returncode, 5, done.stdout)
        self.assertIn("shared.txt", done.stdout)

        self.git(b, "fetch", "--quiet", "origin")
        sh("git", "merge", "origin/main", cwd=b, env=self.env, check=False)
        self.commit(b, {"shared.txt": "a and b\n"}, "resolve", push=False)
        self.land(b, "preflight")
        self.land(b)
        self.assertEqual(self.settle(b, "fix/clash-b").returncode, 0)
        first, second = self.outcome("fix/clash-a")["merge_commit"], self.outcome("fix/clash-b")["merge_commit"]
        self.git(self.repo, "fetch", "--quiet", "origin")
        sh("git", "merge-base", "--is-ancestor", first, second, cwd=self.repo, env=self.env)

    def capped(self, lines):
        """Main with a gate rule that fails a `.ts` file over `lines` lines,
        naming the file, as `no_file_too_long` does."""
        self.write(self.repo, {
            "foundations.sh": self.read(self.repo, "foundations.sh") + (
                f"for f in *.ts; do [ -f \"$f\" ] || continue; n=$(wc -l < \"$f\" | tr -d ' '); "
                f"[ \"$n\" -gt {lines} ] && printf 'FAIL  no file too long\\n        "
                f"missing: %s is %s lines, over {lines}\\n' \"$f\" \"$n\"; done; true\n"
            ),
            "long.ts": "".join(f"{n}\n" for n in range(lines - 1)),
        })
        self.git(self.repo, "add", "-A")
        self.git(self.repo, "commit", "--quiet", "-m", "a capped file")
        self.git(self.repo, "push", "--quiet", "origin", "main")

    def foundations_runs(self):
        return [line for line in open(self.env["LAND_TEST_EVIDENCE"]).read().splitlines()
                if line.startswith("foundations ") and "/land/candidate" in line]

    def test_a_gate_line_goes_to_the_one_member_that_touched_its_file(self):
        self.capped(10)
        gate, held = self.blocked()
        names = ["fix/beside-a", "fix/grows-it", "fix/beside-c"]
        long = "".join(f"{n}\n" for n in range(12))
        wts = self.queue([
            ("fix/beside-a", {"a.txt": "1\n"}),
            ("fix/grows-it", {"long.ts": long, "grown.txt": "1\n"}),
            ("fix/beside-c", {"c.txt": "1\n"}),
        ])
        before = len(self.foundations_runs())
        open(gate, "w").close()
        open(held, "w").close()
        codes = [self.settle(where, name).returncode for where, name in zip(wts, names)]
        self.assertEqual(codes, [0, 4, 0])
        red = self.settle(wts[1], "fix/grows-it").stdout
        self.assertIn("missing: long.ts is 12 lines, over 10", red)
        self.assertIn("no Check was run", red)
        merges = [self.outcome(name)["merge_commit"] for name in ("fix/beside-a", "fix/beside-c")]
        self.assertEqual(merges[-1], self.main_head(), "one push for the two")
        self.git(self.repo, "fetch", "--quiet", "origin")
        self.assertEqual(self.git(self.repo, "rev-parse", merges[-1] + "^1"), merges[0],
                         "the two landed together, not one turn each")
        self.assertEqual(len(self.foundations_runs()) - before, 2, "gated twice, with no split")
        on_main = self.main_files()
        self.assertNotIn("grown.txt", on_main)
        self.assertIn("a.txt", on_main)
        self.assertIn("c.txt", on_main)

    def test_a_gate_line_two_members_touched_splits_the_batch(self):
        self.capped(11)
        gate, held = self.blocked()
        names = ["fix/top", "fix/bottom", "fix/beside"]
        base = "".join(f"{n}\n" for n in range(10))
        wts = self.queue([
            ("fix/top", {"long.ts": "top\n" + base}),
            ("fix/bottom", {"long.ts": base + "bottom\n"}),
            ("fix/beside", {"beside.txt": "1\n"}),
        ])
        before = len(self.foundations_runs())
        open(gate, "w").close()
        open(held, "w").close()
        codes = [self.settle(where, name).returncode for where, name in zip(wts, names)]
        self.assertEqual(codes, [0, 4, 0])
        self.assertIn("missing: long.ts is 12 lines, over 11", self.settle(wts[1], "fix/bottom").stdout)
        self.assertGreaterEqual(len(self.foundations_runs()) - before, 4,
                                "split to find whose: all three, the first two, then each")

    def test_a_withdrawn_entry_in_a_pending_half_is_not_gated(self):
        gate, held = self.blocked()
        known = self.read(self.repo, "foundations.txt")
        names = ["fix/half-a", "fix/half-b", "fix/half-c"]
        wts = self.queue([
            ("fix/half-a", {"a.txt": "1\n", "hold.txt": "1\n"}),
            ("fix/half-b", {"b.txt": "1\n"}),
            # A line naming no path, so the batch splits rather than blames.
            ("fix/half-c", {"c.txt": "1\n", "foundations.txt": "FAIL  a new rule\n        missing: a new subject\n" + known}),
        ])
        open(gate, "w").close()
        deadline = time.monotonic() + 30
        while "running ui" not in self.outcome("fix/half-a").get("detail", ""):
            self.assertLess(time.monotonic(), deadline, "the first half never reached its Check")
            time.sleep(0.1)
        self.assertIn("together with fix/half-b", self.outcome("fix/half-a")["detail"])
        self.assertNotIn("fix/half-c", self.outcome("fix/half-a")["detail"], "split before the Checks")
        before = len(self.foundations_runs())
        done = self.land(wts[2], "--withdraw")
        self.assertIn("withdrew fix/half-c", done.stdout)
        open(held, "w").close()
        for where, name in zip(wts[:2], names[:2]):
            self.assertEqual(self.settle(where, name).returncode, 0)
        status = self.settle(wts[2], "fix/half-c")
        self.assertEqual(status.returncode, 7, status.stdout)
        self.assertIn("withdrawn", status.stdout)
        self.assertEqual(len(self.foundations_runs()) - before, 0, "the withdrawn half was never gated")
        self.assertNotIn("c.txt", self.main_files())

    def test_withdraw_while_gating_says_the_turn_will_finish(self):
        gate, _ = self.blocked()
        done = self.land(self.repo, "--withdraw", "fix/blocker")
        self.assertIn("withdrew fix/blocker", done.stdout)
        self.assertIn("will still finish", done.stdout)
        open(gate, "w").close()
        self.assertEqual(self.settle(self.repo, "fix/blocker").returncode, 0)

    def test_a_member_that_conflicts_with_main_goes_back_and_the_rest_land(self):
        gate, held = self.blocked()
        clash, green = self.queue([
            ("fix/clash-main", {"ui/block.ts": "mine\n"}),
            ("fix/beside-it", {"beside.txt": "1\n"}),
        ])
        open(gate, "w").close()
        open(held, "w").close()
        done = self.settle(clash, "fix/clash-main")
        self.assertEqual(done.returncode, 5, done.stdout)
        self.assertIn("ui/block.ts", done.stdout)
        self.assertEqual(self.settle(green, "fix/beside-it").returncode, 0)
        self.assertIn("beside.txt", self.main_files())


if __name__ == "__main__":
    unittest.main()
