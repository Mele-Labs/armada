#!/usr/bin/env python3
"""PreToolUse: nothing reaches `main` around a pull request and its `ci`.

Merges to `main` go through pull requests and the `ci` check. An agent may
merge a pull request with "Create a merge commit" once `ci` has passed, or ask
GitHub to do it when `ci` does (`--auto`). So this refuses the commands that
reach `main` around that: a push to it, a forge merge that is not that one, and
a `git merge` run in the checkout that has it. `scripts/land` and `armada land` drain what is already
queued on the merge line and are not refused here.

Reads the hook payload on stdin and answers `deny` or nothing at all.
`docs/capabilities/merge-line.md` is the design of the line being retired.

The line's own push of `main` is the one allowed, and it never reaches this
hook: the detached runner makes it, not the Bash tool. So no command typed
here is the runner's, and nothing in one, an environment variable included,
lets it through.
"""
import json
import os
import shlex
import subprocess
import sys

# What `land` puts on the base. Both spellings of the same ref, plus the bare
# name a refspec may use.
BASE = "main"
BASE_REFS = (BASE, f"refs/heads/{BASE}")

SAY = (
    "A merge to `main` is made on a pull request with \"Create a merge "
    "commit\", once `ci` has passed. When the branch's self-check passes:\n"
    "  git push -u origin <branch>\n"
    "  gh pr create --base main   # say what the diff cannot; end with "
    "\"Merge with Create a merge commit\"\n"
    "  gh pr merge <n> --merge    # once `gh pr checks <n>` shows `ci` passed, "
    "or add --auto to let GitHub merge it then\n"
    "GitHub runs the `checks` workflow: `ci` is the gate and `desktop_test` "
    "reports beside it. If `ci` is red, read `gh pr checks <n>` and "
    "`gh run view --log-failed`, fix on the same branch and push again; the "
    "pull request updates.\n"
    "docs/practices/ci.md says what the workflow runs."
)

# The `gh` this hook asks about a pull request's checks. Only the hook's own
# environment sets it, never a command typed into the tool, so a test can point
# it at a stand-in and nothing an agent runs can.
GH = os.environ.get("GUARD_MERGE_GH", "gh")

# Merge methods and bypasses a pull request is not to be merged with.
OTHER_METHODS = ("--squash", "-s", "--rebase", "-r", "--admin")


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


def segments(command: str) -> list[list[str]]:
    """The command split into the separate commands a shell would run.

    `cd x && git push origin main` is one string to the tool and two commands
    to the shell, and only the second one is this hook's business.
    """
    try:
        words = shlex.split(command, comments=True)
    except ValueError:
        # Unbalanced quotes: the shell would refuse it too.
        return []
    out: list[list[str]] = [[]]
    for word in words:
        if word in ("&&", "||", ";", "|", "&"):
            out.append([])
        else:
            out[-1].append(word)
    return [s for s in out if s]


def lands_on_base(words: list[str]) -> bool:
    """Whether this `git push` would write the base branch.

    A push names its destination in the refspec's right-hand side, and with no
    colon the whole word is both sides. `--delete main` is the same write by
    another route.
    """
    after_flags = [w for w in words[1:] if not w.startswith("-")]
    if not after_flags:
        # `git push` with no arguments follows the branch's upstream, which is
        # never `main` here: the checkout at `main` is never worked in.
        return False
    deleting = any(w in ("--delete", "-d") for w in words)
    # The first bare word is the remote; the rest are refspecs.
    for spec in after_flags[1:]:
        destination = spec.split(":")[-1] if ":" in spec else spec
        if destination in BASE_REFS:
            return True
        if deleting and spec in BASE_REFS:
            return True
    return False


def checkout_has_base(path: str) -> bool:
    """Whether the checkout holding `path` has `main` checked out.

    Read from `.git` rather than asked of git, as `guard_write.py` does. A
    checkout that cannot be read answers `False`: a hook that cannot tell where
    it is must not refuse a merge.
    """
    here = os.path.abspath(path)
    while not os.path.exists(os.path.join(here, ".git")):
        parent = os.path.dirname(here)
        if parent == here:
            return False
        here = parent
    git = os.path.join(here, ".git")
    try:
        if os.path.isfile(git):
            with open(git, encoding="utf-8") as f:
                pointer = f.read().strip()
            if not pointer.startswith("gitdir:"):
                return False
            git = pointer.split(":", 1)[1].strip()
        with open(os.path.join(git, "HEAD"), encoding="utf-8") as f:
            return f.read().strip() == f"ref: refs/heads/{BASE}"
    except OSError:
        return False


def git_verb(words: list[str]) -> str:
    """The subcommand of a `git` command, past the options that take a value."""
    i = 1
    while i < len(words):
        if words[i] in ("-C", "-c"):
            i += 2
        elif words[i].startswith("-"):
            i += 1
        else:
            return words[i]
    return ""


def merges_into_base(words: list[str], cwd: str) -> bool:
    """Whether this `git merge` could make a merge commit on `main` by hand.

    `--ff-only` moves `main` to what the remote already holds, which is how the
    owner's checkout is brought up to date, and `--abort` and the like end a
    merge that is already there. Neither adds a commit nobody gated.
    """
    if git_verb(words) != "merge":
        return False
    if any(w in ("--ff-only", "--abort", "--continue", "--quit") for w in words):
        return False
    if "-C" in words[:-1]:
        cwd = os.path.join(cwd, words[words.index("-C") + 1])
    return checkout_has_base(cwd)


def inner(words: list[str]) -> list[list[str]]:
    """The commands inside a `sh -c '…'`, which are commands like any other.

    A shell started by hand is the obvious way past a guard that reads the
    command it is given, and it costs one line to follow it in.
    """
    if not words or not words[0].rsplit("/", 1)[-1] in ("sh", "bash", "zsh"):
        return []
    try:
        script = words[words.index("-c") + 1]
    except (ValueError, IndexError):
        return []
    return segments(script)


def pr_merge_refusal(words: list[str]) -> str | None:
    """Why this `gh pr merge` may not run, or None where it may.

    It may when it makes a merge commit of one named pull request into `main`,
    and `ci` has passed on it or `--auto` leaves the wait to GitHub. A
    squash, a rebase or `--admin` is refused: main's history is one merge per
    branch, and `--admin` walks past the check that is the gate. A pull request
    whose `ci` cannot be read is refused, never assumed green.
    """
    if any(w in OTHER_METHODS for w in words):
        return "This merges a pull request another way than \"Create a merge commit\"."
    if not any(w in ("--merge", "-m") for w in words):
        return "This merges a pull request without saying how; pass --merge."
    after = words[words.index("merge") + 1:] if "merge" in words else []
    selectors = []
    skip = False
    for w in after:
        if skip:
            skip = False
        elif w in ("-R", "--repo", "--body", "-b", "--subject", "-t", "--match-head-commit"):
            skip = True
        elif not w.startswith("-"):
            selectors.append(w)
    if len(selectors) != 1:
        return "This merges a pull request without naming exactly one."
    if "--auto" in words:
        return None
    command = [GH]
    for flag in ("-R", "--repo"):
        if flag in words[:-1]:
            command += [flag, words[words.index(flag) + 1]]
    command += ["pr", "view", selectors[0], "--json", "state,baseRefName,statusCheckRollup"]
    try:
        run = subprocess.run(command, capture_output=True, text=True, timeout=30, check=True)
        view = json.loads(run.stdout)
    except Exception:
        return "The pull request's checks could not be read, so `ci` is not known to have passed."
    if view.get("state") != "OPEN" or view.get("baseRefName") != BASE:
        return f"That pull request is not open against `{BASE}`."
    for check in view.get("statusCheckRollup") or []:
        if check.get("name") == "ci":
            if check.get("conclusion") == "SUCCESS":
                return None
            return "`ci` has not passed on that pull request."
    return "That pull request has no `ci` check yet."


def main() -> None:
    try:
        payload = json.load(sys.stdin)
    except Exception:
        sys.exit(0)

    command = (payload.get("tool_input") or {}).get("command") or ""
    if not command:
        sys.exit(0)

    commands = segments(command)
    for words in list(commands):
        commands.extend(inner(words))

    cwd = payload.get("cwd") or os.getcwd()
    for words in commands:
        # `NAME=value git push …` and `env NAME=value git push …` run git with
        # an environment, and the assignments sit where the program name would be.
        while words and (
            words[0].rsplit("/", 1)[-1] == "env"
            or ("=" in words[0].split("/")[0] and not words[0].startswith("-"))
        ):
            words = words[1:]
        bare = [w for w in words if not w.startswith("-")]
        if bare[:1] == ["cd"] and len(bare) > 1:
            # `cd <checkout> && git merge …` runs the merge there.
            cwd = os.path.join(cwd, os.path.expanduser(bare[1]))
        if len(bare) >= 2 and bare[0].endswith("git"):
            if merges_into_base(words, cwd):
                answer(f"This merges into `{BASE}` by hand, in the checkout that has it.\n{SAY}")
            # `-C <path>` puts a path where a verb would be, so the verb is
            # whichever of the first few words git actually knows.
            if "push" in bare[1:4] and lands_on_base(words):
                answer(f"This pushes `{BASE}`.\n{SAY}")
        # `gh -R owner/repo pr merge` puts the repository between the two, so
        # the pair is looked for wherever it sits rather than at fixed places.
        if bare and bare[0].endswith("gh"):
            pairs = zip(bare, bare[1:])
            if any(verb == "pr" and act == "merge" for verb, act in pairs):
                why = pr_merge_refusal(words)
                if why:
                    answer(f"{why}\n{SAY}")
            # The same write as a request: `gh api -X PUT repos/…/pulls/1/merge`.
            if any(w.endswith("/merge") and "/pulls/" in w for w in bare):
                answer(f"This merges a pull request through the forge's API.\n{SAY}")

    sys.exit(0)


if __name__ == "__main__":
    main()
