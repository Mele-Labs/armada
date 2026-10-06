#!/usr/bin/env python3
"""What `guard_merge.py` refuses, and what it must let through.

`python3 .claude/hooks/test_guard_merge.py`. Nothing here runs git or `gh`: the
hook reads a payload and answers, so a test is one string in and one decision
out. The false-positive half is the half that matters — a hook that refuses an
ordinary branch push stops every agent in the repository.
"""
import json
import pathlib
import subprocess
import sys
import tempfile
import unittest

HOOK = pathlib.Path(__file__).with_name("guard_merge.py")


def decide(command: str, cwd: str | None = None) -> str | None:
    """The hook's decision on one Bash command, or None where it stayed silent."""
    payload = {"tool_name": "Bash", "tool_input": {"command": command}}
    if cwd:
        payload["cwd"] = cwd
    run = subprocess.run(
        [sys.executable, str(HOOK)],
        input=json.dumps(payload),
        capture_output=True,
        text=True,
        check=True,
    )
    if not run.stdout.strip():
        return None
    answer = json.loads(run.stdout)["hookSpecificOutput"]
    return answer["permissionDecision"]


class Refuses(unittest.TestCase):
    def test_a_push_naming_the_base(self) -> None:
        for command in (
            "git push origin main",
            "git push origin HEAD:main",
            "git push origin HEAD:refs/heads/main",
            "git -C /Users/x/armada push origin main",
            "git push --force origin my-branch:main",
            "git push origin --delete main",
            "cd /tmp/x && git push origin main",
        ):
            with self.subTest(command=command):
                self.assertEqual(decide(command), "deny")

    def test_a_merge_pressed_by_hand(self) -> None:
        for command in (
            "gh pr merge 1327 --merge",
            "gh pr merge --squash 1327",
            "git fetch && gh pr merge 1327 --merge --delete-branch",
        ):
            with self.subTest(command=command):
                self.assertEqual(decide(command), "deny")

    def test_the_shapes_that_used_to_walk_past_it(self) -> None:
        # Each of these was allowed until an adversarial review typed it.
        for command in (
            "GIT_AUTHOR_NAME=x git push origin main",
            "gh -R NickMele/armada pr merge 1327 --merge",
            "GH_TOKEN=x gh pr merge 1327 --merge",
            'sh -c "git push origin main"',
            'bash -c "gh pr merge 1327 --merge"',
            "gh api -X PUT repos/NickMele/armada/pulls/1327/merge",
        ):
            with self.subTest(command=command):
                self.assertEqual(decide(command), "deny")

    def test_a_command_claiming_to_be_the_runner(self) -> None:
        # The runner's own push never reaches this hook, so nothing typed
        # here can be it, however it is dressed.
        for command in (
            "ARMADA_LAND_RUNNER=1 git push origin main",
            "env ARMADA_LAND_RUNNER=1 git push origin HEAD:main",
        ):
            with self.subTest(command=command):
                self.assertEqual(decide(command), "deny")

    def test_the_refusal_says_what_to_run_instead(self) -> None:
        run = subprocess.run(
            [sys.executable, str(HOOK)],
            input=json.dumps({"tool_input": {"command": "gh pr merge 1"}}),
            capture_output=True, text=True, check=True,
        )
        reason = json.loads(run.stdout)["hookSpecificOutput"]["permissionDecisionReason"]
        self.assertIn("scripts/land preflight", reason)
        self.assertIn("docs/capabilities/merge-line.md", reason)
        self.assertIn("until it stops exiting 3", reason)
        self.assertIn("Exit 10 is not the end", reason)


def checkout(root: pathlib.Path, name: str, branch: str) -> str:
    """A directory whose `.git/HEAD` says `branch` is checked out."""
    git = root / name / ".git"
    git.mkdir(parents=True)
    (git / "HEAD").write_text(f"ref: refs/heads/{branch}\n")
    (root / name / "crates").mkdir()
    return str(root / name)


class MergesByHand(unittest.TestCase):
    """A `git merge` is the line's business only where `main` is checked out."""

    def setUp(self) -> None:
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        root = pathlib.Path(tmp.name)
        self.main = checkout(root, "main-checkout", "main")
        self.feature = checkout(root, "feature-checkout", "feature")
        # A linked worktree keeps a `.git` file pointing at its own git directory.
        gitdir = root / "gitdir"
        gitdir.mkdir()
        (gitdir / "HEAD").write_text("ref: refs/heads/main\n")
        (root / "linked").mkdir()
        (root / "linked" / ".git").write_text(f"gitdir: {gitdir}\n")
        self.linked = str(root / "linked")

    def test_refuses_a_merge_in_the_checkout_at_main(self) -> None:
        for command in (
            "git merge --no-ff worktree-agent-a3a1662de3faa7bc2",
            "git merge feature",
            "git merge origin/main",
            'sh -c "git merge feature"',
            "GIT_AUTHOR_NAME=x git merge feature",
        ):
            with self.subTest(command=command):
                self.assertEqual(decide(command, self.main), "deny")

    def test_refuses_it_from_below_the_root_and_in_a_linked_worktree(self) -> None:
        self.assertEqual(decide("git merge feature", self.main + "/crates"), "deny")
        self.assertEqual(decide("git merge feature", self.linked), "deny")

    def test_refuses_it_where_the_command_points_at_main(self) -> None:
        self.assertEqual(decide(f"git -C {self.main} merge feature", self.feature), "deny")
        self.assertEqual(decide(f"cd {self.main} && git merge feature", self.feature), "deny")

    def test_lets_a_feature_branch_catch_up_with_main(self) -> None:
        for command in (
            "git merge origin/main",
            "git merge --no-ff main",
            "git fetch && git merge origin/main",
        ):
            with self.subTest(command=command):
                self.assertIsNone(decide(command, self.feature))

    def test_lets_a_checkout_at_main_follow_the_remote(self) -> None:
        for command in (
            "git merge --ff-only origin/main",
            "git merge --abort",
            "git merge --continue",
            "git log --oneline merge",
            "git branch merge",
        ):
            with self.subTest(command=command):
                self.assertIsNone(decide(command, self.main))

    def test_stays_silent_where_it_cannot_tell(self) -> None:
        self.assertIsNone(decide("git merge feature", "/nonexistent/place"))


class Allows(unittest.TestCase):
    def test_an_ordinary_branch_push(self) -> None:
        for command in (
            "git push -u origin tools/merge-line",
            "git push --force-with-lease origin fix/main-bar",
            "git push",
            "git push origin main:refs/heads/spike",
            "git -C /Users/x/armada push origin HEAD:refs/heads/tools/merge-line",
        ):
            with self.subTest(command=command):
                self.assertIsNone(decide(command))

    def test_the_line_itself_and_reading_a_pull_request(self) -> None:
        # The runner pushes main from a process this starts, not from here.
        for command in (
            "scripts/land",
            "scripts/land --status",
            "armada land preflight",
            "armada land",
            "gh pr view 1327 --json state",
            "gh pr list --state merged",
            "gh pr create --fill",
        ):
            with self.subTest(command=command):
                self.assertIsNone(decide(command))

    def test_prose_naming_the_commands(self) -> None:
        # A doc edit or a grep that carries the words is not a merge.
        for command in (
            "grep -rn 'gh pr merge' docs",
            "echo 'run git push origin main'",
        ):
            with self.subTest(command=command):
                self.assertIsNone(decide(command))

    def test_a_command_the_shell_could_not_parse(self) -> None:
        self.assertIsNone(decide("git push 'origin main"))


if __name__ == "__main__":
    unittest.main(verbosity=2)
