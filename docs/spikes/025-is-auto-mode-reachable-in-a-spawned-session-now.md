# Spike 25 — Is auto mode reachable in a spawned session now?

**No, as in spike 019: `--permission-mode auto` is accepted and not in force.** On
Claude Code 2.1.292 the `init` line of a session started with it, or with
`permissions.defaultMode: "auto"` in `--settings`, still reports `default`, and a
shell line that writes outside the working directory is not classified: it waits
for a host to answer and, with none, is denied. So a hosted session's **auto** is
Fleet's own rule: everything runs except what is destructive, pushes code to a
shared space or writes off this machine, which `fleet::helm::because` already
reads for Helm, and the door asks the person about those three.

Measured on 7 Oct 2026 against 2.1.292 with `--model haiku`.

| Launch | `init` reports | A write outside the working directory |
|---|---|---|
| `--permission-mode auto` | `default` | `permission_denied` after about two minutes with nobody answering; no classifier call (`modelUsage` names the session's own model only) |
| `--settings '{"permissions":{"defaultMode":"auto"}}'` | `default` | waited for a host until the run was cut at 100 seconds |
| `--permission-mode acceptEdits`, `plan`, `default` | itself | as spike 018 |

`claude auto-mode` is still a terminal command that inspects the classifier's
rules and not a launch flag.

## What a session's modes are, then

| Mode the person sets | Launched as | Put to the person |
|---|---|---|
| `auto` (the default) | `default` + the door | the three classes |
| `ask` | `default` + the door | every call the person's own settings do not cover |
| `acceptEdits` | `acceptEdits` + the door | the same, but edits never reach the door |
| `plan` | `plan` + the door | the three classes; the CLI itself blocks writes |

**The person's own `allow` and `deny` rules still decide first**, in every mode,
as for Helm. Spike 019's reason for the door stands.

## A second thing found

A hosted session is **a live process**, and a stream-json session with its input
held open runs a turn per message with the same session id (spike 004, 016); the
`result` line is a turn boundary and not an exit.
