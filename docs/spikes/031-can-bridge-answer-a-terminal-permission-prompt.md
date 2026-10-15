# Spike 31 — Can Bridge answer a terminal Session's own permission prompt?

**Yes: a mod can allow the call from outside while the terminal's dialog is open.** The mod's
`tool.call` hook races the call beneath it against an outside answer. When the answer is allow,
the hook marks the call in a `tool.check` hook and calls `next(e)` a second time. That run is
allowed without a second dialog, the tool runs, and returning from the hook closes the
terminal's open dialog.

Measured on 14 Oct 2026 against Claude Code 2.1.300, `--model haiku`, `--permission-mode
default`, in a scratch directory, with a probe mod loaded by `--plugin-dir`. A pty driver
typed the prompt and nobody answered the dialog. The probe stood in for Bridge with an eight
second timer. Nothing reported into a Fleet.

| Question | Probe | Result |
|---|---|---|
| Does `classic.PermissionRequest` fire when the dialog opens? | A note from the hook, after `next(e)` returns with no decision | Fired, naming `Bash`, as the dialog drew |
| Can the call be allowed while the dialog is open? | After eight seconds, a `tool.check` hook answers `allow` for that command and `tool.call` calls `next(e)` again | `check allowed by bridge`, then the second run ended with no error, and `touched` was written |
| Is there a second dialog? | The terminal's screen after the second run | None. The first dialog closed when the hook returned |
| What does a refusal return? | In `claude plugin test`, not the terminal: a plugin's own `$.tool.call` on a call the hook denies | `{ deny }`, the message the hook gave |

## Things Fleet depends on

- **The terminal and Bridge race.** Whichever answers first wins. The terminal answering first
  settles the item in Fleet as answered there.
- **The second `next(e)` is a new permission check**, which is why `tool.check` must allow it.
  The mark is keyed by the tool and its detail and dropped when the run returns, so nothing else
  is allowed by it.
- **Allow and remember is not offered.** Remembering is the harness's own rule store, which the
  mod does not write.

The probe mod is `031-probe-mod-register.ts`, the driver `031-drive-terminal.py`, and the
probe's log `031-log.txt`.
