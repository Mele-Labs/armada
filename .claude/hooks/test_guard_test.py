#!/usr/bin/env python3
"""What `guard_test.py` refuses, and what it must let through.

`python3 .claude/hooks/test_guard_test.py`. Nothing here runs cargo: the hook
reads a payload and answers, so a test is one string in and one decision out.
The allowed half matters as much as the refused: a hook that refuses
`armada check test` or `cargo build` stops every agent in the repository.
"""
import json
import pathlib
import subprocess
import sys
import unittest

HOOK = pathlib.Path(__file__).with_name("guard_test.py")


def run(command: str) -> subprocess.CompletedProcess:
    payload = {"tool_name": "Bash", "tool_input": {"command": command}}
    return subprocess.run(
        [sys.executable, str(HOOK)],
        input=json.dumps(payload),
        capture_output=True,
        text=True,
        check=True,
    )


def decide(command: str) -> str | None:
    """The hook's decision on one Bash command, or None where it stayed silent."""
    out = run(command).stdout
    if not out.strip():
        return None
    return json.loads(out)["hookSpecificOutput"]["permissionDecision"]


class Refuses(unittest.TestCase):
    def test_cargo_test_and_nextest(self) -> None:
        for command in (
            "cargo test",
            "cargo test -p fleet -p store -p ipc -p api -p acceptance -p core-model",
            "cargo test --workspace",
            "cargo nextest run",
            "cargo nextest run --workspace",
            "cargo nextest list",
            "cargo t",
            "cargo-nextest nextest run",
        ):
            with self.subTest(command=command):
                self.assertEqual(decide(command), "deny")

    def test_options_before_the_subcommand(self) -> None:
        for command in (
            "cargo +nightly test",
            "cargo -q test",
            "cargo --locked nextest run",
            "cargo -C crates/fleet test",
            "cargo --config build.jobs=4 test",
            "cargo -Z unstable-options test",
            "/Users/x/.cargo/bin/cargo test",
        ):
            with self.subTest(command=command):
                self.assertEqual(decide(command), "deny")

    def test_wherever_it_sits_in_a_compound_command(self) -> None:
        for command in (
            "cd crates/fleet && cargo test",
            "cargo build && cargo test",
            "cargo build; cargo nextest run",
            "true || cargo test",
            "cargo build\ncargo test",
            "cargo test 2>&1 | tail -20",
            "(cd x && cargo test)",
        ):
            with self.subTest(command=command):
                self.assertEqual(decide(command), "deny")

    def test_prefixes_and_shells(self) -> None:
        for command in (
            "RUST_LOG=debug cargo test",
            "env RUST_BACKTRACE=1 cargo test",
            "time cargo nextest run",
            "nice -n 10 cargo test",
            'sh -c "cargo test"',
            "bash -lc 'cd x && cargo nextest run'",
            "bash -c \"sh -c 'cargo test'\"",
        ):
            with self.subTest(command=command):
                self.assertEqual(decide(command), "deny")

    def test_the_message_names_the_command_to_use(self) -> None:
        reason = json.loads(run("cargo test").stdout)["hookSpecificOutput"]["permissionDecisionReason"]
        self.assertIn("armada check test --changed", reason)
        self.assertIn("armada check test <name>", reason)


class Allows(unittest.TestCase):
    def test_armada_check_in_every_shape(self) -> None:
        for command in (
            "armada check test",
            "armada check test --changed",
            "armada check test some_test_name",
            "armada check test cargo",
            "cd crates/fleet && armada check test --changed",
            "armada check build --changed",
            "armada check acceptance",
        ):
            with self.subTest(command=command):
                self.assertIsNone(decide(command))

    def test_other_cargo_commands(self) -> None:
        for command in (
            "cargo build",
            "cargo build -p fleet",
            "cargo check --workspace",
            "cargo clippy",
            "cargo xtask verify-foundations",
            "cargo install --locked cargo-nextest",
            "cargo test --no-run",
            "cargo nextest run --no-run",
            "cargo +nightly test -p fleet --no-run",
        ):
            with self.subTest(command=command):
                self.assertIsNone(decide(command))

    def test_the_words_as_text_for_another_program(self) -> None:
        for command in (
            "grep -rn 'cargo test' docs",
            'echo "cargo test is refused"',
            'git commit -m "Stop cargo test running bare"',
            "git log --grep='cargo test'",
            "gh pr create --body 'Never cargo test; use armada check test'",
            "rg cargo\\ nextest docs",
            "cat docs/practices/rust.md",
        ):
            with self.subTest(command=command):
                self.assertIsNone(decide(command))

    def test_a_heredoc_body_is_text(self) -> None:
        command = (
            "git commit -F - <<'EOF'\n"
            "Route tests through the queue\n"
            "\n"
            "cargo test -p fleet took the machine.\n"
            "cargo nextest run did too.\n"
            "EOF"
        )
        self.assertIsNone(decide(command))

    def test_a_command_after_a_heredoc_is_still_read(self) -> None:
        command = "cat <<EOF\nnotes\nEOF\ncargo test"
        self.assertEqual(decide(command), "deny")

    def test_what_it_cannot_read_stays_silent(self) -> None:
        for command in ("", "echo 'unbalanced", "   "):
            with self.subTest(command=command):
                self.assertIsNone(decide(command))
        out = subprocess.run(
            [sys.executable, str(HOOK)], input="not json", capture_output=True, text=True, check=True
        )
        self.assertEqual(out.stdout.strip(), "")


if __name__ == "__main__":
    unittest.main()
