#!/usr/bin/env python3
"""What `guard_merge.py` refuses, and what it must let through.

`python3 .claude/hooks/test_guard_merge.py`. Nothing here runs git or the real
`gh`: the hook reads a payload and answers, and where it asks about a pull
request's checks it asks a stand-in, so a test is one string in and one decision
out. The false-positive half is the half that matters — a hook that refuses an
ordinary branch push stops every agent in the repository.
"""
import json
import os
import pathlib
import subprocess
import sys
import tempfile
import unittest

HOOK = pathlib.Path(__file__).with_name("guard_merge.py")

# What `gh pr view --json …` answers, as the stand-in prints it. Unset, the
# stand-in fails, which is a pull request whose checks cannot be read.
STAND_IN = """#!/usr/bin/env python3
import os, sys
view = os.environ.get("FAKE_VIEW")
if view is None:
    sys.exit(1)
print(view)
"""
_tmp = tempfile.TemporaryDirectory()
GH = pathlib.Path(_tmp.name) / "gh"
GH.write_text(STAND_IN)
GH.chmod(0o755)


def view(ci: str | None, state: str = "OPEN", base: str = "main") -> str:
    """The stand-in's answer for a pull request whose `ci` came to `ci`."""
    rollup = [{"name": "needs", "conclusion": "SUCCESS"}]
    if ci:
        rollup.append({"name": "ci", "conclusion": ci})
    return json.dumps({"state": state, "baseRefName": base, "statusCheckRollup": rollup})


def env_with(fake_view: str | None) -> dict[str, str]:
    env = {**os.environ, "GUARD_MERGE_GH": str(GH)}
    env.pop("FAKE_VIEW", None)
    if fake_view is not None:
        env["FAKE_VIEW"] = fake_view
    return env


def decide(command: str, cwd: str | None = None, fake_view: str | None = None) -> str | None:
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
        env=env_with(fake_view),
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

    def test_a_merge_that_is_not_a_merge_commit_of_one_named_pull_request(self) -> None:
        green = view("SUCCESS")
        for command in (
            "gh pr merge --squash 1327",
            "gh pr merge 1327 --rebase",
            "gh pr merge 1327 --merge --admin",
            "gh pr merge 1327",
            "gh pr merge --merge",
            "gh pr merge 1327 1328 --merge",
        ):
            with self.subTest(command=command):
                self.assertEqual(decide(command, fake_view=green), "deny")

    def test_a_merge_whose_ci_has_not_passed(self) -> None:
        for answer in (None, view(None), view("FAILURE"), view("CANCELLED"), view("SUCCESS", state="MERGED"), view("SUCCESS", base="dev")):
            with self.subTest(answer=answer):
                self.assertEqual(decide("gh pr merge 1327 --merge", fake_view=answer), "deny")

    def test_the_shapes_that_used_to_walk_past_it(self) -> None:
        # Each of these was allowed until an adversarial review typed it.
        for command in (
            "GIT_AUTHOR_NAME=x git push origin main",
            'sh -c "git push origin main"',
            "gh api -X PUT repos/NickMele/armada/pulls/1327/merge",
        ):
            with self.subTest(command=command):
                self.assertEqual(decide(command), "deny")

    def test_the_wrapped_shapes_still_ask_the_checks(self) -> None:
        # Wrapping a merge in env, -R or a shell does not skip the question.
        for command in (
            "gh -R NickMele/armada pr merge 1327 --merge",
            "GH_TOKEN=x gh pr merge 1327 --merge",
            'bash -c "gh pr merge 1327 --merge"',
        ):
            with self.subTest(command=command):
                self.assertEqual(decide(command, fake_view=view("FAILURE")), "deny")

    def test_a_separator_stuck_to_a_word(self) -> None:
        # The shell splits `main;` into `main` and `;`; the hook must too, or
        # the destination reads `main;` and the push walks past it.
        for command in (
            "git push origin main;",
            "git push origin main;echo done",
            "git push origin main&&echo done",
            "git push origin main|cat",
            "git push origin main 2>&1",
            "git push origin main >/dev/null",
            "git status\ngit push origin main",
            "echo ok;git push origin main",
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
            capture_output=True, text=True, check=True, env=env_with(None),
        )
        reason = json.loads(run.stdout)["hookSpecificOutput"]["permissionDecisionReason"]
        self.assertIn("gh pr create --base main", reason)
        self.assertIn("Create a merge commit", reason)
        self.assertIn("gh pr merge <n> --merge", reason)
        self.assertIn("docs/practices/ci.md", reason)
        self.assertNotIn("scripts/land", reason)


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
    def test_a_merge_commit_of_a_pull_request_whose_ci_passed(self) -> None:
        green = view("SUCCESS")
        for command in (
            "gh pr merge 1327 --merge",
            "gh pr merge 1327 --merge --delete-branch",
            "gh pr merge --merge 1327",
            "gh -R Mele-Labs/armada pr merge 1327 --merge",
            "GH_TOKEN=x gh pr merge 1327 --merge",
            'bash -c "gh pr merge 1327 --merge"',
        ):
            with self.subTest(command=command):
                self.assertIsNone(decide(command, fake_view=green))

    def test_a_merge_with_a_redirect_or_a_glued_separator(self) -> None:
        green = view("SUCCESS")
        for command in (
            "gh pr merge 1327 --merge 2>&1 | tail -3",
            "gh pr merge 1327 --merge;",
            "gh pr merge 1327 --merge;echo done",
            "gh pr merge 1327 --merge >/dev/null",
            "gh pr merge 1327 --merge &>/dev/null",
        ):
            with self.subTest(command=command):
                self.assertIsNone(decide(command, fake_view=green))

    def test_auto_merge_leaves_the_wait_to_github(self) -> None:
        # No checks are read: GitHub merges it when `ci` passes, and not before.
        for command in (
            "gh pr merge 1327 --merge --auto",
            "gh pr merge --auto --merge 1327",
        ):
            with self.subTest(command=command):
                self.assertIsNone(decide(command, fake_view=None))

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
        # Still allowed while the line drains; the refusal no longer teaches them.
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
