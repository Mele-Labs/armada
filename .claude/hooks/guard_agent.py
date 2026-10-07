#!/usr/bin/env python3
"""PreToolUse: an agent works in a leased slot, never in a tree the harness cut.

`scripts/preview` merges the Armada slots and the merge line, and nothing
else. An Agent call with `isolation: "worktree"` works in a tree under
`.claude/worktrees/` that it cannot see, so the owner's preview silently lacks
that branch. `isolation: "remote"` and no isolation are not refused.

Reads the hook payload on stdin and answers `deny` or nothing at all.
`.claude/skills/agent-worktrees/` is the rule this enforces.
"""
import json
import sys

SAY = (
    "An agent works in a leased slot, because `scripts/preview` merges only "
    "slots and the merge line, and a tree the harness cuts is invisible to it:\n"
    "  slot=$(armada worktree lease <branch>)   # or --existing <branch>\n"
    "Dispatch without `isolation`, and put the slot's absolute path in the "
    "brief. `.claude/skills/agent-worktrees/SKILL.md` has the lease and the "
    "release."
)


def answer(reason: str) -> None:
    """Emit a refusal and exit. Silence is this hook's every other answer."""
    print(json.dumps({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "deny",
            "permissionDecisionReason": reason,
        }
    }))
    sys.exit(0)


def main() -> None:
    try:
        payload = json.load(sys.stdin)
    except Exception:
        sys.exit(0)

    tool_input = payload.get("tool_input") or {}
    if tool_input.get("isolation") == "worktree":
        answer(f"This dispatches an agent with `isolation: \"worktree\"`.\n{SAY}")

    sys.exit(0)


if __name__ == "__main__":
    main()
