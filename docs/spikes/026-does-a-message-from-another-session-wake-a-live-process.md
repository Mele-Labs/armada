# Spike 26 — Does another session's message wake a live headless process?

**Yes.** A `-p --input-format stream-json` process that has ended its turn and is
waiting on its input starts a new turn when another session sends it a message
with the built-in `SendMessage`, with no input on its own stdin. Its stream shows
a second `init`, the model's work and a second `result`, under the same session
id. Fleet knows it was woken because it counted no message of its own
outstanding.

Measured on 7 Oct 2026 against Claude Code 2.1.292, `--model haiku`.

| Step | What happened |
|---|---|
| Session A, `--name alpha-spike`, `--settings '{"crossSessionInbound":"accept"}'`, input held open, one message in | `READY`, then a `result`. The process stayed up |
| Session B, a separate `-p` process, asked to call `ListAgents` and then `SendMessage` with `to: "alpha-spike"` | `ListAgents` lists A as a peer, `interactive`, and the send answered *"in that session's inbox, not yet read by its Claude"* |
| A's stream | a `command_lifecycle` `started`, an `init`, `PINEAPPLE`, a `result`, a `command_lifecycle` `completed` |

## Things Fleet depends on

- **A session is addressed by name**, set at launch with `--name`. Fleet names a
  hosted session `s-<first eight of its id>`, which is the same after a restart.
- **The delivered text is not in A's stream.** Fleet draws it on the receiver's
  thread from the *sender's* `SendMessage` call (`to`, `message`) when the sender
  is a session it hosts. A message from a terminal session wakes the session and
  its text is not shown; the mod reports only a count.
- **A session whose process Fleet ended is not a peer**, so a send to it fails
  for the sender. Fleet reads the sender's call and, where the target is a hosted
  session with no process, resumes it and writes the message as a turn.
- **A mismatch of permission modes holds the message** for approval at the
  receiver, by the tool's own account. Hosted sessions in different modes may
  therefore not wake each other; this was read and not measured.
