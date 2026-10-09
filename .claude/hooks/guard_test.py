#!/usr/bin/env python3
"""PreToolUse: tests run through `armada check test`, never `cargo test`.

A bare `cargo test -p fleet -p store …` ran the fleet test binary on 14 of 18
cores for minutes, 77% of it system time, and left the owner's Mac unusable.
`armada check test` runs at half the machine's width, waits its turn among the
other Checks on the machine, and `--changed` limits it to what the diff reaches.
So this refuses `cargo test` and `cargo nextest` wherever they sit in a command.

Reads the hook payload on stdin and answers `deny` or nothing at all. It reads
the program a command runs, so `armada check test` and a commit message that
says "cargo test" are not its business. `docs/practices/rust.md` section 8.
"""
import json
import re
import sys

from guard_merge import answer, segments

SAY = (
    "`cargo test` and bare `nextest` ignore the machine's width and the queue of "
    "other Checks, and one run made the owner's Mac unusable. Run:\n"
    "  armada check test --changed   # what this diff touches\n"
    "  armada check test <name>      # one test by name\n"
    "docs/practices/rust.md section 8."
)

REFUSED = ("test", "t", "nextest")

# `cargo` options that take a value, which is not the subcommand.
TAKES_VALUE = ("-C", "-Z", "--config", "--manifest-path", "--color")

# Words that run the command after them with nothing changed.
WRAPPERS = ("env", "time", "nice", "nohup", "exec", "command")

HEREDOC = re.compile(r"<<-?\s*(['\"]?)(\w+)\1")


def without_heredocs(command: str) -> str:
    """The command with heredoc bodies removed.

    A commit message or PR body may carry a line that starts `cargo test`, and
    split on newlines it would read as a command. The body is text for another
    program, so it is dropped; the line that opens it stays.
    """
    out: list[str] = []
    end: str | None = None
    for line in command.split("\n"):
        if end is not None:
            if line.strip() == end:
                end = None
            continue
        out.append(line)
        opened = HEREDOC.search(line)
        if opened:
            end = opened.group(2)
    return "\n".join(out)


def inner(words: list[str]) -> list[list[str]]:
    """The commands inside `sh -c '…'`, `bash -lc '…'` and the like."""
    if not words or words[0].rsplit("/", 1)[-1] not in ("sh", "bash", "zsh"):
        return []
    for i, word in enumerate(words[1:-1], start=1):
        if re.fullmatch(r"-[a-z]*c[a-z]*", word):
            return segments(without_heredocs(words[i + 1]))
    return []


def refused(words: list[str]) -> bool:
    """Whether this command runs `cargo test` or `cargo nextest` for real."""
    while words and (
        words[0].rsplit("/", 1)[-1] in WRAPPERS
        or ("=" in words[0].split("/")[0] and not words[0].startswith("-"))
    ):
        wrapper = words[0].rsplit("/", 1)[-1] in WRAPPERS
        words = words[1:]
        # `nice -n 10 cargo test`: the wrapper's own options and their numbers.
        while wrapper and words and (words[0].startswith("-") or words[0].isdigit()):
            words = words[1:]
    if not words:
        return False
    program = words[0].rsplit("/", 1)[-1]
    if program == "cargo-nextest":
        return True
    if program != "cargo":
        return False
    args = words[1:]
    i = 0
    while i < len(args):
        if args[i] in TAKES_VALUE:
            i += 2
        elif args[i].startswith(("-", "+")):
            i += 1
        else:
            break
    if i >= len(args) or args[i] not in REFUSED:
        return False
    # Building the test binaries runs nothing.
    return "--no-run" not in args


def main() -> None:
    try:
        payload = json.load(sys.stdin)
    except Exception:
        sys.exit(0)

    command = (payload.get("tool_input") or {}).get("command") or ""
    if not command:
        sys.exit(0)

    commands = segments(without_heredocs(command))
    for words in commands:
        commands.extend(inner(words))
    if any(refused(words) for words in commands):
        answer(SAY)
    sys.exit(0)


if __name__ == "__main__":
    main()
