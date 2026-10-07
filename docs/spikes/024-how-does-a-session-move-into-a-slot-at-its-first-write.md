# Spike 24 — How does a hosted session move into a slot at its first write?

**By ending its turn and resuming it with the slot as the directory.** A process's
working directory is fixed when it starts, and the built-in worktree tool cannot
be used from a hosted session at all: it demands a consent disclosure that a
permission tool cannot show. `--resume` from another directory finds the same
conversation under the same id and runs its tools in the new one. A `PreToolUse`
hook that Fleet answers is what holds a write until then, and it cannot be
skipped by the person's own `allow` rules.

Measured against Claude Code 2.1.292 on 7 Oct 2026, with `--model haiku`, over
`--input-format stream-json --output-format stream-json --verbose`, in a scratch
repository with one extra worktree standing in for a slot. For the Sessions host,
`crates/fleet/src/session_host/gate.rs`.

## What was tried

| Option | Result |
|---|---|
| **(a) The built-in `EnterWorktree` with `path` set to an existing worktree** | **Refused.** Under `--permission-prompt-tool` it answers *"requires the user to read a consent disclosure before approving, and the configured --permission-prompt-tool (a tool_name+input wire) cannot display it"*. Under `acceptEdits` with no prompt tool it waits for an approval nobody can give. The session stayed in its original directory (`pwd` unchanged). It is not available to a session Fleet hosts. |
| **(b) End the process, `--resume <id>` with the slot as cwd** | **Works.** The resumed `init` reports `cwd` as the slot and the same `session_id`; the session recalled a word given before the move (`MARIGOLD`); `pwd` was the slot, `git branch --show-current` was the slot's branch, and a relative `Write` landed in the slot and not the original checkout. Spike 016 had measured resume across directories; this adds that the tools follow the new directory. |
| **(c) `cd` inside Bash, or `--add-dir`** | Not used. A `cd` is outside what the harness lets persist across calls once the directory is not one of its working directories, and the slot is not known when the process starts, so there is nothing to pass to `--add-dir`. |

`--session-id <uuid>` is accepted on a new session, so Fleet names the session
before the process starts and the ledger's row and the CLI's are one. `--name`,
`--effort` and `--add-dir` were accepted in the same launch.

## Holding the write until then

A permission tool is only asked about what the person's own rules do not cover,
so a rule that allows `Edit` would skip it. **A hook runs first and always.**
`--settings` takes an `http` hook, which Claude Code POSTs the call to before it
runs (`024-hook-stub.py` records the POST and answers). The body names the
`session_id`, the `cwd`, the `tool_name` and `tool_input`; an answer of
`{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny",
"permissionDecisionReason":"…"}}` stops the call and the reason reaches the model
as the tool's result (`is_error`), in its words, in place of what the call would
have returned. Settings given by `--settings` are added to the person's, not in
place of them.

## What Fleet does with it

```
 first Write / Edit / NotebookEdit into the checkout, or a shell line that is
 not a read ── hook ──▶ Fleet leases a slot, cuts the branch, records slot and
                        branch on the ledger, answers `deny`:
                        "leased slot N at <path>; stop, you will be moved"
 the turn ends ──────▶ Fleet ends the process, resumes the session with the
                        slot as its directory, and tells it where it is
 every later write ──▶ held to the slot: a path in the main checkout is refused
                        with the slot's path
```

## What this does not cover

**A shell line is read by its words and not by what it does.** The hook cannot
see what `make` writes, so a line Fleet does not recognise as a read leases a
slot first. A line that runs in the slot but names the main checkout by absolute
path cannot be refused; it is the person's own agent, in their own configuration,
and the file tools are the ones held to the slot.

**Two parallel writes in one turn** both reach the hook. The first leases and the
second is held with the same answer, and one slot is taken.

## Provenance

`024-hook-stub.py` is the stub. The cases are the shell lines in this file run
from a scratch directory under the session's scratchpad: the worktree tool with
and without a permission tool, the hook in `deny` mode with a `Write`, and a
two-process run, `--session-id` then `--resume` from the second directory.
