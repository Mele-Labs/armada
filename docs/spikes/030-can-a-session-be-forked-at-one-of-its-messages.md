# Spike 30 — Can a session be forked at one of its messages?

**Yes: `--resume-session-at <uuid>` combines with `--fork-session --session-id`.** The new
session's conversation is the old one's up to and including the transcript line with that
`uuid`, written under the id Fleet chose. The flag is hidden from `--help` (Claude Code
2.1.296, the Agent SDK's `resumeSessionAt`) and it was measured, not documented.

Measured on 12 Oct 2026 against Claude Code 2.1.296, `--model haiku`, `--setting-sources
project,local`, in scratch directories. Each session was told to remember a word (APPLE, then
BANANA) and the fork was asked which words it remembered. Nothing reported into a Fleet.

| Question | Probe | Result |
|---|---|---|
| What does it take: a transcript line's `uuid`, or the `message.id` (`msg_…`)? | Cut at the `uuid` of an `assistant` stream event; cut at its `msg_…` id | The `uuid` works. The `msg_…` id exits 1 with `No message found with message.uuid of: msg_…` |
| Does it combine with `--fork-session --session-id <new>`? | Two turns, forked at the first turn's reply | Accepted. The result's `session_id` is `<new>`. The fork answered `APPLE` and not `BANANA` |
| Does it work with `-p` and the stream-json input Fleet uses? | The same fork, the prompt written to stdin as a `{"type":"user",…}` line | The same answer, so a hosted process takes it |
| What does an unknown `uuid` do? | `00000000-0000-4000-8000-000000000000` | Exits 1 with `No message found with message.uuid of: …`, and writes no fork |
| Is the cut kept by the second process? | `--resume <new>` alone after the fork | Answered `APPLE` only. The fork's own file holds the cut, so only the first process is passed the flag, as `--fork-session` is |
| Can the cut be a person's message? | A `user` line's `uuid`, sent in on stdin with `--replay-user-messages` | Works. The `uuid` a stream-json input line carries is the line's `uuid` in the transcript, so Fleet could have named it |
| A cut at an assistant line whose tool call has no result? | Cut at the `tool_use` line of a turn that ran `Bash` | Accepted. The CLI repaired the pair: the fork said the call returned "Tool result missing" |
| A cut after the tool result? | Cut at the `done` line after it | Accepted. The fork knew it had run the command |

## Things Fleet depends on

- **The cut is a line's `uuid`**, which is a terminal Session's row id (`adapters::terminal_thread`
  draws a row's id from the line's `uuid`, `<uuid>.<n>` for a second row of one line). A cut at a
  line is a cut after its last block.
- **One message can be several lines.** A reply with a thinking block and a text block is two
  lines sharing a `message.id` and each with its own `uuid`, and a stream event carries the line's.
  Thread rows are drawn from the text line, so the cut is at the text line and the thinking before
  it is kept.
- **A hosted Session's rows carry no `uuid`.** Fleet mints their ids (`<sid8>-<ts>-<n>`) and the
  `Said` events the stream is read into drop the line's `uuid`. The cut is found by reading the
  transcript: see `docs/concepts/session.md`, *A fork at a message*.
- **An unknown `uuid` is a failed start, not a copy of everything.** Fleet finds the line before
  it writes anything, so the refusal is Fleet's (422) and the CLI is never given one that is missing.
- **The old transcript is read, not changed**, as for `028`.

## Not measured

- A cut inside a conversation the CLI had compacted. The summary line is a `user` line with its own
  `uuid`, and a cut before it was not tried.
- A cut at a line in a subagent's sidechain. Thread rows never draw those (`isSidechain`), so no
  row id names one.

## Provenance

Scratch directories under `/tmp`, removed afterwards. The sessions were made with `claude -p`
(`--session-id`, then `--resume`) and `--output-format stream-json --verbose`, which prints each
line's `uuid`. The transcript files themselves were not opened: the cut points are the `uuid`s the
stream printed, and every result is the fork's own answer. The tests that hold the findings are
`fleet::tests::session_fork` and `adapters::tests::hosted_session`.
