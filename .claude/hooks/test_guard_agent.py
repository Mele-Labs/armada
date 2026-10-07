#!/usr/bin/env python3
"""What `guard_agent.py` refuses, and what it must let through.

`python3 .claude/hooks/test_guard_agent.py`. The hook reads a payload and
answers, so a test is one payload in and one decision out. The half that
matters is the allowed one: a hook that refuses an ordinary dispatch stops
every agent in the repository.
"""
import json
import pathlib
import subprocess
import sys
import unittest

HOOK = pathlib.Path(__file__).with_name("guard_agent.py")


def decide(tool_input: dict) -> str | None:
    """The hook's decision on one Agent call, or None where it stayed silent."""
    payload = {"tool_name": "Agent", "tool_input": tool_input}
    run = subprocess.run(
        [sys.executable, str(HOOK)],
        input=json.dumps(payload),
        capture_output=True,
        text=True,
    )
    assert run.returncode == 0, run.stderr
    if not run.stdout.strip():
        return None
    return json.loads(run.stdout)["hookSpecificOutput"]["permissionDecision"]


class Dispatch(unittest.TestCase):
    def test_refuses_worktree_isolation(self) -> None:
        self.assertEqual(
            decide({"prompt": "go", "subagent_type": "bridge-engineer", "isolation": "worktree"}),
            "deny",
        )

    def test_the_refusal_says_what_to_do(self) -> None:
        run = subprocess.run(
            [sys.executable, str(HOOK)],
            input=json.dumps({"tool_input": {"isolation": "worktree"}}),
            capture_output=True,
            text=True,
        )
        reason = json.loads(run.stdout)["hookSpecificOutput"]["permissionDecisionReason"]
        for words in ("armada worktree lease", "--existing", "without `isolation`", "preview"):
            self.assertIn(words, reason)

    def test_allows_no_isolation(self) -> None:
        self.assertIsNone(decide({"prompt": "go", "subagent_type": "rust-engineer"}))

    def test_allows_remote_isolation(self) -> None:
        self.assertIsNone(decide({"prompt": "go", "isolation": "remote"}))

    def test_stays_silent_on_an_unreadable_payload(self) -> None:
        run = subprocess.run(
            [sys.executable, str(HOOK)], input="not json", capture_output=True, text=True
        )
        self.assertEqual((run.returncode, run.stdout), (0, ""))


if __name__ == "__main__":
    unittest.main()
