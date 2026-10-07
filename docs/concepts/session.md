# Session

**What it is:** An agent session a person runs, in a terminal or in Bridge, kept as one row on a ledger so Fleet can say which sessions are open and which one holds a branch, a pull request, a Job or a slot.

---

**Kind:** Record.

You have five terminal sessions open across three repositories. One is in slot 3, one opened pull request 1853, and one dispatched a Job an hour ago. You ask who has `fleet/session-ledger`, and the answer is a session by its title, still running, with the slot it stands in.

## What it is

> **Rule.** A session from a terminal is a first-class row, the same shape a session Bridge hosts will have.
> Why: the question is who holds a thing, and it should not depend on where the session was started.

> **Rule.** A session is reported as it happens and is never read back from.
> Why: the third attempt in [scope](../scope.md) was a conversation read as a claim. The ledger keeps what a session took and did, as structure, and nothing downstream reads a transcript.

> **Rule.** Fleet never waits on a session and a terminal session never waits on Fleet.
> Why: a terminal session is the person's own, and Fleet being down, slow or gone must cost it nothing. A session Fleet hosts is Fleet's to run, so its write waits on Fleet's hook, and what the CLI does with a hook Fleet does not answer was not measured.

A session is not [Helm](helm.md), which is Armada's own conversation with Fleet, and not a [Drone](drone.md), which Fleet starts, places in a worktree and gates. A session started by a person was none of those, and until the ledger Fleet could not see it.

## How a session gets in

The harness sends Fleet facts in Armada's own shape and nothing past the adapter that sends them knows what the session runs in.

```
 harness (mod in a terminal session)          Fleet
 ───────────────────────────────────          ─────────────────────────────
 session starts ── started ───────────────▶  POST /sessions/report
 git, gh, dispatch, subagent, message ────▶   stamp, resolve cwd → repository, slot
 turn ends, usage measured ───────────────▶   keep in sessions, ledger_attachments
 session ends ─── ended ──────────────────▶   publish session.changed, whole
```

| `fact` | Carries | Fleet does |
|---|---|---|
| `started` | `cwd`, `title?`, `origin?` | Creates the row or starts a resumed one again; resolves the directory to a repository and a pool slot |
| `titled` | `title` | Sets it |
| `moved` | `cwd` | Re-resolves; a slot the session left is given back |
| `attached` | `attachment`: `kind`, `target`, `detail?` | Takes it; `branch` and `slot` replace the one held |
| `settled` | `attachment`: `kind`, `target`, `state` | Moves it to `spent` or `given_back` |
| `measured` | `usage`: `context_tokens?`, `context_window?`, `cost_micros?` | Keeps what was reported |
| `tuned` | `model?`, `effort?`, `mode?`, `commands?` | Held in memory as the terminal's `terminal` facts: model, effort, mode and the commands it lists |
| `turn_completed` | nothing | Stamps the last turn |
| `ended` | `reason` | Ends it and gives back everything it still holds |

**No instant is reported**: Fleet stamps each fact on arrival. **A repeated fact publishes nothing**, so a harness may report on every turn.

The mod for Claude Code is `plugins/armada/`, and `crates/adapters` is the only Rust that names the harness. It sends no message text and no prompt apart from the first line of the first, as a title. A need is reported from the Bash command `armada need <path> "<what>"`, `--took` or `--release`, as the path, the words it was declared in and what it took, and from nothing else.

## What is kept

Two tables, in `crates/store`. The second is not the first's child.

| Table | One row is | Key |
|---|---|---|
| `sessions` | A session: harness, origin, repository, directory, title, `live` or `ended`, when it started, was last seen and last finished a turn, the figures it last reported | `id`, the harness's own |
| `ledger_attachments` | One thing a holder took or did | `holder_kind`, `holder_id`, `kind`, `manifest_id`, `target` |

A holder is `{ kind: session | job, id }` and points at neither table, so a Job or a need can hold rows without a table of its own. **A Job's own slot and branch are rows held by the Job**, written where Fleet leases the slot and records the branch, so `who_owns` names a Job that holds a slot. A `kind` is open text:

| `kind` | `target` | Held until |
|---|---|---|
| `slot` | the slot's number | the session leaves it or ends |
| `branch` | the branch's name | the session checks out another or ends |
| `pr` | the pull request's number | it merges (`spent`) or closes (`given_back`) |
| `job` | the Job's id | nothing settles it yet |
| `need` | the path, repository-relative | its holder's work merges (`spent`), or the holder is gone or releases it (`given_back`). Not exclusive: `docs/capabilities/needs.md` |
| `subagent` | the agent's id | |
| `message` | `to:<who>` or `from:<who>`, with a count in `detail` | |
| `studio` | the Studio's id | not reported by anything yet |

**A piloted Job's slot and branch are the Session's while it pilots.** `take_over` names the Session, writes a `slot` and a `branch` row held by it with `detail.handed` reading `job <id>`, and gives the Job's own rows back; an exit gives the Session's back and the Job holds them again. `who_owns` names one holder throughout. The Session is the piloted session of [Pilot](pilot.md), and a pilot's three exits are on its row for the Job as well as on Job detail.

**State is one vocabulary for every kind.** `standing` is in force, `spent` is used and done with, and `given_back` was let go before it was used. A released slot and a spent need read the same way.

## What Fleet answers

| Operation | Answers |
|---|---|
| `list_sessions` | Every session, most recently seen first, narrowed by repository, `live` or `ended`, and `q`: a title, a branch, a pull request number, a Job id or a slot |
| `who_owns` | Every holder of a branch, a pull request, a Job or a slot, the ones still holding it first. Empty is an answer |
| `session.changed` | One session whole, whenever a fact changed something |

Both reads are on the agent's door, so a session can ask who else holds the branch it is about to take.

## A session Fleet hosts

A person can start a session from Bridge. Fleet runs it, keeps its thread, and
puts it on the same ledger with `origin` `bridge`. It is not [Helm](helm.md),
which is unchanged, and it is not a [Drone](drone.md): it is the person at work,
in their own configuration.

> **Rule.** A hosted session holds what the person's terminal holds: their
> servers, plugins, skills and settings, with no `--strict-mcp-config` and no
> `--setting-sources`.
> Why: it is the person's own conversation, and the `armada` mod must load in it
> so it reports through the ledger like any other. A hosted session is one row,
> because Fleet keeps its `origin` when the mod reports its start and ignores the
> mod reporting its end, which is a process stopping.

> **Rule.** A hosted session starts with no slot and leases one on its first
> write.
> Why: most conversations never write, and a slot held by a conversation that
> only reads is one a Job cannot have.

```
 start ─▶ a row: no process, no slot, no branch
 message ─▶ a live process, `--session-id`, the message on its input
 first write ─▶ a hook asks Fleet; Fleet leases a slot and cuts a branch, writes
                both on the ledger, refuses the write and the turn ends; the
                process is ended and resumed with the slot as its directory
 later writes ─▶ held to the slot
 quiet for a while ─▶ the process is ended; the next message resumes it
 close ─▶ the slot is parked on its own branch, the row ends
```

**One live process while it is open**, unlike Helm's one per message, because
another session's message wakes an idle one: `SendMessage` to a hosted session's
address, `s-` and the first eight characters of its id, starts a turn in it. A
turn that opens with no message of the person's outstanding is a wake, and the
thread says whose. **A session whose process was ended is resumed by the
message**, written for the sender, which is a hosted session Fleet can read the
send of. `settings.session-quiet-timeout` is how long a process may sit idle.
[Spike 26](../spikes/026-does-a-message-from-another-session-wake-a-live-process.md).

**The lease is taken by a hook, and nothing writes the main checkout before it.**
A write tool, or a shell line that is not a read, is refused until the session
holds a slot; after it, a file tool naming a path in the main checkout is
refused with the slot's path. A shell line is read by its words, so one Fleet
does not recognise as a read leases first.
[Spike 24](../spikes/024-how-does-a-session-move-into-a-slot-at-its-first-write.md).

**The slot is held by the session's id as a Job's is held by its id**, so a
restart of the process or of Fleet is not the session ending. It is given back
on close by parking: what is in it is committed to the session's own branch and
nothing is pushed.

**The person sets the mode, the model and the effort** for each session. `auto`
is the default and means what it means for Helm: everything runs except what is
destructive, pushes code to a shared space or writes off this machine, which
Fleet puts to the person on the session's own thread. The CLI's own auto mode is
not reachable for a spawned session, so `auto` is `default` with that door.
`ask` and `acceptEdits` put every call the person's settings do not cover.
[Spike 25](../spikes/025-is-auto-mode-reachable-in-a-spawned-session-now.md).
A change applies from the next process: an idle one is ended at once, a running
one when its turn is over.

| Operation | Does |
|---|---|
| `start_session` | A row for one repository, with optional title, model, effort and mode |
| `send_session_message` | Text, pictures and files stored by Fleet, and the sessions, Jobs, pull requests and branches it names; takes a turn |
| `answer_session_ask` | One of the offers on the ask the agent is held on |
| `tune_session` | Model, effort and mode |
| `close_session` | Ends the process, parks the slot, ends the row |
| `get_session`, `get_session_file` | The row with its thread, and what a message carried |
| `session.row` | One row of the thread, appended or replaced by its id |

**What a message names is one line Fleet adds for the agent** and the thread
keeps the person's words. A thread's rows are the person's messages, the agent's,
another session's, one line for each tool call, the lease, and each ask with
where it stands. **An ask is the session's own**: it never appears in Helm's
dock and `answer_helm_call` does not find it.

## Acts on a pull request

A pull request a session holds is a `pr` row, and a person can act on it without leaving the session. The pull request is named by its repository and number, and is nobody's Job.

| Operation | Does | Refuses |
|---|---|---|
| `get_pull_request` | Reads it: draft, open, merged or closed, auto-merge, the forge's checks (`pending`, `passed`, or `failed` with the failing names), title, branch, address | Where the forge would not answer |
| `ready_pull_request` | Takes a draft out of draft | Anything that is not a draft |
| `merge_pull_request_by_number` | Merges it | While the checks are running or failed, a draft, a closed one, and the forge's own refusals |
| `enable_auto_merge` | Asks the forge to merge when the checks pass | Once they have passed or failed, a draft, a closed one |
| `review_pull_request` | Drafts a Code Review Job against it, by number or address | A name that is neither, a Session the ledger never heard of |

```
 a person presses       Fleet                                  the forge
 ───────────────        ──────────────────────────────────     ─────────
 merge ───────────────▶ read state + checks ────────────────▶ gh pr view, gh pr checks
                        checks not passed ─▶ refuse, no write
                        passed ────────────────────────────▶ gh pr merge --merge
                        read again, refresh every `pr` row   ◀─
 ◀─ the new state       merged ─▶ row `spent`, closed ─▶ `given_back`
```

> **Rule.** Every write is read first, and a refusal Fleet can name is named before the forge is touched.
> Why: a merge pressed while the checks run must reach nobody, and a count of what the forge was asked to write is the only proof it did not.

> **Rule.** The ledger follows the forge through its own attach.
> Why: a Session's `pr` row is what the harness reported, and `detail` (`state`, `auto_merge`, `checks`, `failing`, `title`, `branch`, `address`) is Fleet's reading of the same pull request, so one writer is not two.

**The three writes are a person's ask, `pushes to shared`.** Helm asks first, as for `merge_pull_request`, and the operations are `Helm only`. Bridge's own buttons are the person's press and reach the routes directly. `get_pull_request` is open to any agent.

**A review is at the approval gate**, like every dispatch, with `origin` `session_dispatched` (*From a Session, by you*) and the pull request as the Job's subject. The workflow is `code_review`, named by Fleet and never read off a link by the proposer. Where a Session pressed it, the Session holds a `job` row whose `detail.origin` says *dispatched from Session <id>*, which is how the Board row and `who_owns` name it.

## Started on a piloted Job

`start_session` takes an optional `pilot` (`job_id`, `outcome`), and that one call is the whole of taking a Job over from a Session: Fleet runs `take_over` first, which is the only thing that can refuse, then writes the Session already holding the Job's slot and branch (`detail.handed`), with the Job as its own `job` row. **It runs in the Job's worktree from its first message and takes no lease on a first write**; its thread opens with a `handoff` row, the bundle's structured fields. The bundle as the agent reads it goes ahead of the person's first message and starts no turn of its own, so a piloted Session nobody has spoken in has spent nothing.

**Ending the pilot, or closing the Session while the Job is still piloted, hands the worktree back to the Job.** The Session's lease is cleared, so its next process starts in the repository and its next write leases a slot of its own. A close parks nothing in that case: the checkout is the Job's, not the Session's to commit.

## In Bridge

Bridge reads every live session from `list_sessions` once per connection and keeps it by `session.changed`, whole, so a chip anywhere asks the same list who owns its branch, slot, pull request or Job, across every repository. A thread is read with `get_session` when a window first opens it, then followed by `session.row`. The renderer reaches both through the draft seam (`SessionsDraft`): the mock fills it from fixtures and a real window from what main publishes, so a screen cannot tell which.

> **Rule.** A session started in Bridge starts in the repository the window has picked, or the one repository Fleet serves.
> Why: Dispatch names its repository the same way, and a session started somewhere the person is not looking is one they cannot find.

| Bridge draws | From |
|---|---|
| The ledger's slot and branch as one pair | The `slot` and `branch` rows of one repository |
| A pull request, its Checks and its acts | The `pr` row's `detail`; each press is one of the pull request operations above, and `read` brings the row current when a session is opened |
| A Job | The `job` row, read against the Board for its title, number, state and branch. A Job the Board has forgotten is left off. A Job a person attested reads `piloted.exit`, and is marked apart from one that passed |
| A sketch the person drew | The picture it was sent as, and the drawing Bridge kept for the ledger. The wire holds only the picture |

## A terminal session's thread

Open a session a person runs in a terminal and Bridge draws its conversation and a message box. `get_session` reads the transcript file the agent CLI keeps for it (`adapters::terminal_thread`, found by session id under the person's home) and answers rows in the shape a hosted thread has: what the person typed, what the agent said, and one line per tool call. Fleet then watches the file for as long as the session is live and publishes each new line as `session.row`, so the thread follows the terminal. A row's id is the transcript line's own, so a line read twice replaces itself.

> **Rule.** The thread is drawn and never read as a claim. Nothing Fleet knows about a terminal session, its title, its Job or its state, comes from its rows.

What is not drawn: the agent's reasoning, a tool's answer, a subagent's own turns, the CLI's bookkeeping and a message from another session.

**A message reaches a terminal by its own mod.** Fleet cannot push into a terminal. `send_session_message` to a terminal session holds the text, and the `armada` mod in that session asks `take_held_messages` every two seconds and submits each text as the person's own prompt, which starts a turn whether the session is idle or busy (spike 27). The ask is also how Fleet knows the session is listening: a send to one that has not asked within ten seconds is refused as `fleet.terminal_session_unreachable`, so the person is told at once. A file or picture is saved by Fleet as for a hosted session and sent as `Attached file: <path>` in the text. The mod reports what the terminal runs on (`tuned`: its model, effort, permission mode and the commands it lists) and Bridge draws those in the composer. A model or effort chosen in Bridge is held as a command, and the mod runs it as `/model <x>` or `/effort <x>`; the engine refuses a slash command submitted as text, so it goes through the mods API's command call (spike 27). **The permission mode is shown and not set**: nothing in the mods API switches a live session's, and it is read from the settings-hook inputs at each turn, so it is stale between a change in the terminal and the next turn. There is no Close in Bridge for a terminal session. The sent text shows in the thread when the transcript has it, which is when the turn starts.

## Not built

**The `/` list is empty until an agent has started.** `hosted.commands` is the names in the stream's `init` line, slash commands then skills, and a session whose own process has not started is given the last one any session read. It is held in memory, so after a Fleet restart it is empty until a process starts, and the harness says nothing of what each command does, so the list draws names only.

**A session that dies without ending stays `live`.** Its `last_seen_at` is what says it has gone quiet; nothing yet checks the process.

**A message another session sent a terminal session is not drawn.** A hosted
session's thread carries what another hosted session wrote to it; a terminal
thread skips the line, and the mod reports the message as a count and never
its text.

**Hosted sessions in different permission modes may not wake each other.** The
tool says a session in another mode holds a cross-session message for approval.
Not measured.

**A message names where it went and not which session**, because the harness does not say: a delivery says whether it came from a peer or a teammate, and never a session id.

## Installing the Claude Code mod

```
claude plugin marketplace add <this repository>/plugins
claude plugin install armada@armada-local
```

It loads in the person's own sessions only. A Drone, a Judge call and a scout are started with `--setting-sources project,local`, so a mod installed in a person's user settings never loads in one ([spike 23](../spikes/023-does-a-user-installed-mod-load-in-a-drone.md)).
