---
name: verify-before-done
description: Prove a change works before saying it is done — run the checks the repository names on what you touched, show a fix red before green, re-read your own diff as a reviewer would, and report what ran and what it read. Load before reporting any change as finished.
---

# Verify before done

**Done means you watched it work, not that you expect it to.** The
repository's `CLAUDE.md` or `AGENTS.md` names its checks and how to read them;
its rules beat this file.

## Run the checks on what you touched

- Run the checks the repository names — tests, lint, typecheck, build — on the
  code you changed and its module. Whole suites are CI's unless the brief says
  otherwise.
- Run them in the foreground and wait for them to finish. A check still running
  is not a result.
- A check that was red before your change and is red the same way after is not
  yours, but say so. Compare against the default branch when you cannot tell.

## A fix is shown red, then green

- Before the fix: run the test or the steps that show the failure, and see it
  fail.
- After the fix: run the same thing, and see it pass.
- A test that has never failed proves nothing. If you broke the behaviour on
  purpose to see red, revert the break and check `git status` is clean.

## Read your own diff as its reviewer

Before the last commit, read `git diff` against the base as someone who wants
to find the bug:

- Anything left over: debug output, commented-out code, a stray file.
- A caller, test or doc that names what you changed and still says the old thing.
- An edge the change does not handle: empty, missing, twice, concurrent.
- A change wider than the brief asked for.

Fix what you find, then run the checks again.

## Report what ran

For each check: the command, and what it read — the count it printed, or each
failure with `file:line`. Say what you did not run, and why. "Should work" is
not a result.
