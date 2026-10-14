# Is Helm its own agent, or a Session?

**Decided 2026-10-13.** His words: *"Whatever makes helm as good as sessions."*

Helm and a Session did the same job in two engines, and Helm's was the poorer
one. Measured on `origin/main`:

| | Helm | Session |
|---|---|---|
| Process | a new CLI process per message, stdin closed at once, no keeper, no interrupt, no background work | one live process under `armada session-keep`, turns on stdin, ten minutes idle before it ends |
| Model | pinned to `sonnet` | his pick, else the CLI's default |
| Permissions | `default` plus Fleet's three-class cards; editing an existing file, git and `rm` all became cards | Auto: it runs, and refuses only what is destructive or off this machine |
| Writes | the main checkout, no slot, no branch; Armada's own `guard_write.py` refused them | the first write leases a slot and cuts a branch |
| Brief | about 3,000 words prepended to the first turn: a workflow schema, a 467-word sample, and rules that held it to one observation and one poll a turn | none, and the mod's short note |

**Chosen: Helm is a hosted Session.** One per repository, marked `helm` on its
record, created by the first message the dock sends and ended by Start fresh.
It has everything a Session has: the live process and keeper, Auto, a slot and
a branch on its first write, `tune_session`, interrupt, subagents, questions,
attachments and mentions, the `/` list. What makes it Helm is a short note
appended to its system prompt, the line Fleet puts before each message saying
where the person is in Bridge, and the door's `Helm only` rows (`tell_session`,
`ask_person_to_approve` among them).

**What it reverses, with his yes:**

- **Helm edits the main checkout with no branch.** (17 Sep 2026, `#1373`.) Its
  first write now leases a slot and cuts a branch like any Session's, and
  `helm.changed_checkout` is retired, since a write in a slot is the Session's
  own ledger.
- **The three asks for Helm.** (18 Sep 2026: destructive, pushes to a shared
  space, writes off this machine, each put to him on a card.) Helm's
  permission is the Session's Auto gate, the same as any Session's. `crate::helm::deciding` no longer decides for the dock.
- **"Keep Helm and Sessions both, decide later on evidence."** (7 Oct 2026.) The
  table above is the evidence. The later decision is this one.

**What it keeps.** A Job's Commander still runs on the old Helm host, with its
own launch, its own thread files and its own reach. Only the dock's use of that
host is retired, and the code the Commander shares stays. Routing a thought,
`tell_session` and `record_thought_sent` stay, and their "Sent to" rows are now
rows of the Helm Session's thread. The old Helm thread files under the records
root stay on disk, unread: Armada is pre-alpha and nothing migrates.

**Cost he took:**

- Helm can now push, as any Session can. Auto refuses only the destructive and
  the off-machine, so a `git push` runs. The ask that used to stop it is gone.
- The workflow schema and sample left the prompt. A definition is checked at
  `save_workflow`, whose refusal carries what Helm needs to correct it.
- Helm is a Session, so a person finds it in the Sessions list as well as in the
  dock, marked.
- A Helm Session that writes holds a slot until it ends. Start fresh ends it.

**Where it landed:** branch `helm/is-a-session`. `docs/concepts/helm.md` is
rewritten, `docs/contracts/agent-prompt.md` section 5a is the note, and
`docs/practices/protocol.md` has the wire. Bridge's dock still has to move onto
it.
