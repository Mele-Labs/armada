# Spike 27 — How does Fleet deliver text into a live terminal session?

**Through the `armada` mod already loaded in that session.** A mod can call
`$.prompt.submit({ text, asUser: true })`, which starts a turn as if the person
had typed the text. Fleet cannot push to a mod, so the mod asks Fleet for held
messages on a timer (`$.clock.every` and `$.http.fetch`, both of which the mod
already uses to report) and submits what it gets.

Measured on 7 Oct 2026 against Claude Code 2.1.292, `--model haiku`, an
interactive `claude` on a pty the probe owned, `--setting-sources project,local`
so the installed mod did not load, and a scratch mod loaded with `--plugin-dir`
polling a local stub server. No session of the owner's was touched.

| Question | Result |
|---|---|
| Idle session, stub holds a message | Picked up within the 1 s poll, `submit` resolved 40 ms later, a turn ran, the model answered `PINEAPPLE` |
| Session busy with a long reply | `submit` waited for the turn to end, then started its own turn: `MANGO`. Nothing was lost or interleaved |
| What the transcript records | A `user` row with `origin: {kind: "plugin", name, asUser: true}`; a person's own typing is `origin: {kind: "human"}` |
| Does the mod API have a way in | Yes: `$.prompt.submit`. `claude plugin validate` lists it among the mod's calls |
| Cross-session inbox (`SendMessage`, `$.session.send`) | Read from the docs, not measured. The sender has to be a Claude session, and Fleet is not one. The receiver must accept inbound messages. A mod would still be the way to reach it, so it adds nothing over `$.prompt.submit` |
| `-p` sender process | Not measured. It spends a model turn to relay text and still needs the receiver to accept inbound messages |

## Things Fleet depends on

- **Reachable means the mod is polling.** A terminal session whose mod is not
  loaded, or whose `claude` has ended, picks nothing up. Fleet holds the message
  and answers the send with a refusal when the session has not asked for held
  messages within a few poll periods.
- **A held message is delivered once.** The mod's poll takes it off Fleet's queue.
- **The transcript is the thread.** The user row is in the file as soon as the
  turn starts, so Fleet draws the sent text from the transcript and not from its
  own record of the send.
- **Transcript path** is `~/.claude/projects/<cwd, with slashes and dots as dashes>/<session id>.jsonl`.
  One test turn wrote about 320 KB of mostly `attachment` and bookkeeping rows,
  so the reader skips every `type` it does not draw.
- A permission dialog open in the terminal holds the submitted turn the way it
  holds typing; the mod cannot answer it.
