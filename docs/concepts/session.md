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

A session is not a [Drone](drone.md), which Fleet starts, places in a worktree and gates. A session started by a person was none of those, and until the ledger Fleet could not see it. [Helm](helm.md) is one of these now: Armada's own conversation with Fleet is a hosted session per repository, marked `helm`.

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
| `started` | `cwd`, `title?`, `origin?`, `mod_version?` | Creates the row or starts a resumed one again; resolves the directory to a repository and a pool slot |
| `titled` | `title`, `named?` | `named` is a person's `/rename` in the terminal and replaces the title; without it, the first prompt's line, kept only where there is no title |
| `moved` | `cwd` | Re-resolves; a slot the session left is given back. A slot whose lease is another Session's is not taken |
| `attached` | `attachment`: `kind`, `target`, `detail?` | Takes it; `branch` and `slot` replace the one held. A `branch` or `pr` read from a slot whose lease is another Session's is dropped (*Whose slot it is*) |
| `settled` | `attachment`: `kind`, `target`, `state` | Moves it to `spent` or `given_back` |
| `measured` | `usage`: `context_tokens?`, `context_window?`, `cost_micros?`; `plan?`: the plan's windows; `context_breakdown?`: what fills the window | Keeps what was reported. The breakdown is kept whole in `session_context`, one row a session, and replaced by the next; a report without one leaves it. The windows are the account's, so Fleet keeps the newest from any session as `get_plan_usage` (`machine.md`, *Quota*) and keeps nothing of them on the session |
| `tuned` | `model?`, `effort?`, `mode?`, `commands?`, `mod_version?` | Held in memory as the terminal's `terminal` facts: model, effort, mode and the commands it lists |
| `turn_started` | nothing | A prompt went in (the mod, never for a slash command, which most of the time ends without a turn): the session is working, and the reply the turn before left is over (*What a Session is doing*) |
| `turn_completed` | `said?`: the first line of what the turn ended on | Stamps the last turn, lets go a hold for the plan's limit, and keeps `said` as the reply (*What a Session is doing*). The mod does not tell a turn that died on an API error |
| `limited` | `resets_at` | Holds the session until the plan's limit resets (*A session that hit the plan's limit*) |
| `turn_completed` | nothing | Stamps the last turn, and marks every window the session showed ready |
| `wrote` | nothing | The session itself wrote a file (the mod, never for a subagent): marks every window it showed as still changing |
| `permission_asked` | `tool`, `detail?` | A terminal's harness opened its own permission prompt (the mod, from Claude Code's `PermissionRequest` hook): a `permission` call until it closes (*A terminal's permission prompt*) |
| `permission_settled` | nothing | The tool call that raised the prompt ended: the call goes |
| `touched` | `paths`: files written, from the session's directory | Checks them against who else writes them (*Write clashes*); a terminal Session's are counted for the timeline as `session.edited` |
| `read` | `path`: a file read, from the session's directory | Counted for the timeline as `session.read`, for a terminal Session; nothing on the ledger changes |
| `ran` | `command`: the first line of a shell command, 300 characters at most | Counted for the timeline as `session.ran`, cut and with what looks like a secret in it hidden; nothing on the ledger changes |
| `tasks` | `tasks`: the agent's whole list, each with `id`, `text` and `status` (`pending`, `in_progress` or `completed`) | Keeps the list and when each item was first seen completed (*The build record*). An empty list clears it |
| `plan_left` | nothing | The agent left plan mode (*The build record*) |
| `committed` | `branch?` | A `git commit` printed a commit made. Entered on the timeline as `session.committed` |
| `pushed` | `branch?` | A `git push` printed a ref updated. Entered on the timeline as `session.pushed` |
| `ended` | `reason` | Ends it and gives back everything it still holds. A terminal's slot is parked unless the reason is `clear` or `resume` (*Ending a terminal session*) |

**No instant is reported**: Fleet stamps each fact on arrival. **A repeated fact publishes nothing**, so a harness may report on every turn.

The mod for Claude Code is `plugins/armada/`, and `crates/adapters` is the only Rust that names the harness. It sends no message text and no prompt apart from the first line of real text in the first, as a title, the first line of what a turn ended on, as the reply (*What a Session is doing*), and the first line of each shell command, for the timeline (`docs/practices/protocol.md`, *The Fleet's timeline*): the harness's wrapper tags (`<agent-message …>`, `<command-…>`, `<system-reminder>`) are taken off first, and a prompt with nothing left has no title. A `/rename` in the terminal is reported as the name typed, or, for a bare `/rename`, the name Claude Code wrote to the transcript. **A Session's name can also be changed from Bridge**, by pressing its title in the Session view: `rename_session`, hosted or terminal, which stands until the next terminal `/rename`. A hosted Session's harness is not told, so its own name for itself stays what it was. **The mod also sends the text of each item on the task list the agent wrote**, read from the task tools it already calls (*The build record*). A need is reported from the Bash command `armada need <path> "<what>"`, `--took` or `--release`, as the path, the words it was declared in and what it took, and from nothing else.

## What is kept

Two tables, in `crates/store`. The second is not the first's child.

| Table | One row is | Key |
|---|---|---|
| `sessions` | A session: harness, origin, repository, directory, title, `live` or `ended`, when it started, was last seen and last finished a turn, the figures it last reported | `id`, the harness's own |
| `ledger_attachments` | One thing a holder took or did | `holder_kind`, `holder_id`, `kind`, `manifest_id`, `target` |

A holder is `{ kind: session | job, id }` and points at neither table, so a Job or a need can hold rows without a table of its own. **A Job's own slot and branch are rows held by the Job**, written where Fleet leases the slot and records the branch, so `who_owns` names a Job that holds a slot. A `kind` is open text:

| `kind` | `target` | Held until |
|---|---|---|
| `slot` | the slot's number | the session leaves it, the pool shows it held by another or free, or the session ends |
| `branch` | the branch's name | the session checks out another, its slot is given back, or it ends |
| `pr` | the pull request's number | it merges (`spent`) or closes (`given_back`) |
| `job` | the Job's id | nothing settles it yet |
| `need` | the path, repository-relative | its holder's work merges (`spent`), or the holder is gone or releases it (`given_back`). Not exclusive: `docs/capabilities/needs.md` |
| `subagent` | the agent's id | it stops (`spent`) or its process is cut off (`given_back`); more work after either holds it again, *A terminal session's thread* |
| `message` | `to:<who>` or `from:<who>`, with a count in `detail` | |
| `studio` | the Studio's id | not reported by anything yet |
| `forked_to`, `forked_from` | the other session's id | `spent` when written, so the old session ending does not give the link back |
| `started_to`, `started_by` | the other session's id | `spent` when written, as a fork's links are. The caller holds `started_to` and the Session its agent started holds `started_by` |
| `unsent` | the words of the first message another session's agent wrote | `standing` until the person's next send, which spends it; ending the session gives it back |
| `artifact` | a page's or document's address, or a file's absolute path; `detail.form` is `page`, `file`, `image`, `doc` or `window`, `detail.title` its name | nothing: it stays on the ledger after the session ends |

**An artifact is something a person would open, and a code edit is never one.** Edits are Branches and Pull requests. Five forms:

| Form | What counts | The press opens |
|---|---|---|
| `page` | A page published with Claude Code's `Artifact` tool (a publish, not a list, read or asset upload) | the address, in the browser |
| `file` | A file the `Write` tool **created** with a document extension: `md`, `mdx`, `txt`, `pdf`, `png`, `jpg`, `jpeg`, `gif`, `webp`, `svg`, `csv`, `doc`, `docx`, `xls`, `xlsx`, `ppt`, `pptx`. Not under `node_modules`, `.git`, `.claude`, `.armada`, `target`, `dist`, `build` or a temporary directory. `html` is left out, because a page's source appears as the page once published; configuration (`json`, `yaml`, `toml`) is not a document | the file, on this machine |
| `image` | A picture the session **looked at**: a `Read` of a `png`, `jpg`, `jpeg`, `gif` or `webp`, in any directory (a screenshot is usually in `/tmp`). The same path twice is one row, and a picture it wrote stays `file` | the file, on this machine |
| `doc` | A Claude Docs document made or edited (`mcp__claude_ai_Claude_Docs__create`, `batch`, `update`), by the address its answer carries or the document's id | the address, in the browser |
| `window` | A page the session showed with `show_window` | the address, in Bridge's own window |
| `sketch` | A diagram the session drew with `show_sketch`; `target` is `sketch:<title>` and `detail.by` is `agent` | the sketch pad |

The terminal mod tells the first five (`plugins/armada/hooks/facts.ts`, `artifactOf`). **A hosted session tells files and pictures looked at**, because its stream carries a `Write` call but not its result, so a published page's address and a Docs link are not seen; and it cannot tell a new file from a rewrite, so a rewritten document counts. The extension lists are the same rule written twice, in `facts.ts` and `adapters::written_document` / `adapters::viewed_image`. Files made by other tools, such as a screenshot a command saved, are not seen.

**A piloted Job's slot and branch are the Session's while it pilots.** `take_over` names the Session, writes a `slot` and a `branch` row held by it with `detail.handed` reading `job <id>`, and gives the Job's own rows back; an exit gives the Session's back and the Job holds them again. `who_owns` names one holder throughout. The Session is the piloted session of [Pilot](pilot.md), and a pilot's three exits are on its row for the Job as well as on Job detail.

### Whose slot it is

**The lease in the pool says which Session works a slot. A ledger row is a claim, believed while the pool agrees.** A harness reports the directory a Session is in, and a slot it left stays in that directory for whoever leases it next, so the directory says where a Session was and the lease says who works there. `armada worktree lease` records the agent session it ran under beside the holder; a hosted Session holds as a Job of its own id; a lease taken by hand, or before leases named a session, names none.

| What the pool says of a slot, for a Session | Fleet |
|---|---|
| The lease is written as the Session's | Credits the `slot`, its checked-out `branch`, and what is read from it |
| The lease names no live Session: a person's own terminal, an older lease, a Session that ended (`/clear` starts a new id in the same terminal) | Credits it, as before the lease named anyone |
| Held for another live Session or a Job, or free | Credits no `slot` and drops a `branch` or `pr` read from it |
| Any of these, for a `pr` the mod reports with `created` in its `detail` | Credits it. The mod read its address off the session's own `gh pr create`, so no directory is being trusted, and a session that leased a second slot and left its first still owns what it opened |

**A release is settled by the pull sweep, for hosted and terminal Sessions alike.** Each interval Fleet gives back the `slot` and `branch` rows of a live Session whose lease is no longer its own, by whatever road the slot went: its own `armada worktree release`, Bridge's release, a lease taken back from a gone holder. A hosted Session is also followed at its next tool call. **A `pr` row stays**: the Session that opened a pull request is still the one to tell after it leaves the slot, and the row settles when the pull request merges or closes.

**The ledger draws every slot the Session holds.** It keeps one `slot` row, the slot the Session works in, so a slot its subagent leased has no row of its own. Its record is drawn with a `slot` for each other slot whose lease is the Session's, by Fleet's lease or a subagent's `armada worktree lease` naming it (`crate::slot_holders::slots_of`). A terminal Session that moved out of a slot gave it up though its shell still holds the lease, so a slot the ledger gave back for one is not drawn again. The owner, 15 Oct 2026.

**One reading answers who holds a branch or a pull request.** A Session is confirmed on a branch when it is live, its `slot` row stands, the lease there is its own or names nobody live, the worktree exists, its `branch` row stands and that branch is checked out. The owner of a pull request, the holder of a need, the Sessions told which branches a build holds, the write-clash table and the attaching of a pull request nobody reported all ask it (`crate::slot_holders`); the write-clash table adds every other slot the Session holds. Where no Session is confirmed, the one whose standing row is oldest stands, and a branch a Session gave back is nobody's. A merge is attributed to whoever held the pull request or its branch in any state, since the question there is who made it.

**State is one vocabulary for every kind.** `standing` is in force, `spent` is used and done with, and `given_back` was let go before it was used. A released slot and a spent need read the same way.

### Which machine owned a pull request

**A registry names the Session or Job that claimed a pull request, so a second machine under the same GitHub account can tell who had it.** It is a small central store, set up by [signing in](account.md), and Fleet writes a row when a Session or Job claims a pull request. Nothing is written to the pull request or the repository, so a label, a comment and a body trailer are all refused.

| Field | Holds |
|---|---|
| key | The sha256, in hex, of `owner/name#number`, the name lowercased so every machine makes the same one |
| kind | `session` or `job` |
| id | The holder's short id |
| machine | The machine's name |
| since | When the claim was made |

**A row never holds a title, a repository name or code.** The machine that owns the pull request draws its title from its own ledger, and any other machine draws only the stored facts. A repository registers nothing until its Manifest turns it on, and the switch is off by default. With it off, the repository registers none and shows no owners at all, those another machine registered included.

| When | Fleet |
|---|---|
| A Session claims a pull request (`claim_pull_request`) | Writes the row |
| A Job opens its pull request, or finds its own already open | Writes the row |
| The reading of a repository's open pull requests | Asks for the rows of all of them in one call, and puts each owner on the merge line's pull request and on the panel's page as `owner` (kind, id, machine, `this_machine`) |
| A pull request is in the last reading and not in this one | Deletes its row |

**None of it can fail the claim or the delivery.** A store that cannot be reached, a repository bound to no profile (or to one signed out) and a switch that is off leave the row unwritten and the field absent, and a claim is made all the same. The panel's page carries what the last reading found and makes no call of its own.

**Bridge draws a row as a chip.** The kind's glyph (`square-terminal` for a Session, `box` for a Job) and the short id in mono lead, and the machine's mark follows: `laptop` for this machine, `monitor` for another, its tooltip the machine's name. A pull request with no row draws no chip, in the merge line's open pull requests and in the Pull request panel. The chip is drawn from the wire's `owner`. The switch is *Pull request owners* in the Policy section of the Manifest form.

## What Fleet answers

| Operation | Answers |
|---|---|
| `list_sessions` | Every session, most recently seen first, narrowed by repository, `live` or `ended`, and `q`: a title, a branch, a pull request number, a Job id or a slot |
| `who_owns` | Every holder of a branch, a pull request, a Job or a slot, the ones still holding it first. Empty is an answer |
| `session.changed` | One session whole, whenever a fact changed something |

Both reads are on the agent's door, so a session can ask who else holds the branch it is about to take.

## A session Fleet hosts

A person can start a session from Bridge. Fleet runs it, keeps its thread, and
puts it on the same ledger with `origin` `bridge`. It is not a [Drone](drone.md):
it is the person at work, in their own configuration. [Helm](helm.md) is the same
engine with a note and a few more tools, one per repository.

> **Rule.** A hosted session holds what the person's terminal holds: their
> servers, plugins, skills and settings, with no `--strict-mcp-config` and no
> `--setting-sources`.
> Why: it is the person's own conversation, and the `armada` mod must load in it
> so it reports through the ledger like any other. A hosted session is one row,
> because Fleet keeps its `origin` when the mod reports its start and ignores the
> mod reporting its end, which is a process stopping.

> **Rule.** A hosted session starts with no slot and leases one on its first
> write, and one resumed with no slot is leased one before its process starts.
> Why: most conversations never write, and a slot held by a conversation that
> only reads is one a Job cannot have. A resumed session has already worked, and
> a resume that waited for its first call to move it lost that call and a turn
> (s-34c7f2c2, three times on 13 Oct 2026).

```
 start ─▶ a row: no process, no slot, no branch
 message ─▶ a live process, `--session-id`, the message on its input
 first write ─▶ a hook asks Fleet; Fleet leases a slot and cuts a branch, writes
                both on the ledger, refuses the write and the turn ends; the
                process is ended and resumed with the slot as its directory
 first write, pool full ─▶ the session is queued, told its place, and stops its
                turn; Fleet leases it a slot when one frees and resumes it there
 later writes ─▶ held to the slot
 quiet for a while ─▶ the process is ended; the next message resumes it, leased a
                slot first if it holds none and the line ahead of it is empty
 close ─▶ the slot is parked on its own branch, the row ends
```

**One live process while it is open**, as Helm's now is (it was one per message), because
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
does not recognise as a read leases first. `|`, `;` and `&` inside quotes do not cut
a line, and a redirect to `/dev/null` or onto another descriptor (`2>&1`) writes nothing;
any other `>`, a `$(` or a backtick, or a quote left open is a write. `sed -n` with an
address and `p` (`sed -n 10,40p file`) reads; every other `sed` is a write.
[Spike 24](../spikes/024-how-does-a-session-move-into-a-slot-at-its-first-write.md).

**A full pool queues the session rather than telling it to try again.** Every
slot held, the write is refused with the session's place in its repository's
line and the instruction to stop for the turn. The line is in Fleet's memory,
first come first served, and a session is in it once; a session behind others
is not leased a slot a moment before the one ahead of it. When a slot is given
back, Fleet leases one for the head of the line, writes the lease as it would
for a first write and moves the session into it, whether or not its refused turn
has ended: an idle session is resumed at once, one still finishing the turn is
moved as that turn ends. A slot is given back by a session closing or its
terminal ending, a Job ending, being paused or cleared, a release or rescue from
Cleanup (a release of a bay a hosted Session holds is refused while its turn runs, and otherwise gives back its slot and branch rows), and the pool growing or a slot reopening; a slot freed where Fleet is
not told, as `armada worktree release` frees one, is found by a look every 15
seconds while anyone waits. A session that ends, or is leased a slot another way,
leaves the line, and a Fleet that restarts forgets it: the session's next write
joins again. A terminal session is not queued, because `armada worktree lease`
waits for a slot itself.

**What the write gate covers.** A slot lives under the main checkout's path, so
the gate tells three places apart: the main checkout, a slot, and anywhere
else. **A session writes in a slot it leased, or one something it started leased
(its subagents), and in no other** (the owner, 13 Oct 2026). A write by any other
session, a fork included, into a slot it never leased is refused with a sentence
saying so; a fork owns nothing of the session it came from, whatever its copy of the
conversation says. Who holds a slot is read from the lease record beside it, at every
call. A slot leased with `armada worktree lease` from a hosted session, the
agent's or a subagent's, is held for the session by its id (Fleet sets
`ARMADA_SESSION` in the process it starts and a subagent inherits it), as a Job's is,
so it survives Fleet ending and resuming the process. One leased with no session id
in the environment is held by the first process above the command that is not a
shell, and is the session's while that process is in its tree. A slot held by
anything else, or by a holder that has gone, is refused. The main checkout stays
refused, and a `..` is resolved by reading before the path is placed.

**Setup's session is the one exception** (the owner, 13 Oct 2026). Started by `root` in a
repository with no `armada.yml`, it has no pool to lease from, so until that root's
Manifest is served it writes only its setup files and is refused anything else: a file
named `armada.yml` or `.gitignore` at any depth under the root, one for each workspace of
a monorepo, and `<root>/.git/info/exclude` (the owner, 14 Oct 2026). A path is placed
after its `..` is resolved, so one that climbs out of the root is refused, and nothing
else under `.git` is a setup file. Fleet loads the Manifest when one of its turns ends,
and from its next write the session leases a slot like any other, **save its setup
files, which still land in the checkout**: an ignored file written on a slot branch
would never reach it. The hosting row's `setup` marks the session as Setup's, so the
window outlasts the `manifest_id` it is bound to.

The same slots decide for **every write path alike**:

| Write | Placed by |
|---|---|
| `Write`, `Edit`, `MultiEdit`, `NotebookEdit` | The path; a relative one against the call's working directory |
| A shell redirect (`>`, `>>`, `2>`), `tee`, `cp`, `mv`, `install`, `ln`, `rsync`, `touch`, `mkdir`, `rm`, `chmod`, `truncate`, `dd`, `sed -i`, `perl -i` | The paths it names; one that begins with `..` is read against the working directory |
| `git` with a subcommand that is not a read | `-C`, `--work-tree` or `--git-dir`, and otherwise where the call stands |
| `python`, `node`, `ruby`, `perl`, `php`, `sh -c` and other interpreters | Every slot path in the line, since what the code writes cannot be read from it. A read of a slot's file through an interpreter is held too |
| `armada worktree release` | The slot it gives back (the directory it runs in, where it names none) |
| Anything else that is not a read | The directory the call stands in |

**A `cd` or `pushd` moves what follows it**, and **the working directory persists between
a session's calls**: a line that goes into the main checkout or a slot the session does
not hold, and a line that goes nowhere while the call already stands in a slot it does not
hold, is held if it then runs anything that is not a read. `cd <slot> && git status` is a
read and runs. **A relative `cd` after another is read from where the first left the line**,
and so is a `..` path after it, never from where the call began. Every part of a line is told
read from write before the slot rule applies, and `git worktree list`, `git stash list`,
`git config --get` and the other listing forms of a subcommand are reads. **Not covered**: a path or a `cd` target built from a variable or a
substitution, a file a script opens by a name the line does not spell, and anything a build
writes by itself. [Spike 29](../spikes/029-why-a-sessions-auto-asked-and-how-a-question-is-answered.md).

**The permission door reads the same ledger.** In `auto` it refuses a call that removes or
overwrites something outside the slots the session holds and `/tmp`: **every** slot the
session holds, its own and each one a subagent leased, and not only the one it runs in. The
door decides whether a call is one of the owner's three classes; whether a slot may be
written at all is the gate's. **What the session started or made is its own to remove**
(the owner, retro of session 9c74d82e): `kill` of pids in its own process tree and of
nothing else (`kill -9 -1` and `pkill` stay refused), `git branch -d`/`-D` of its leased
branch and the branches of slots it holds, once the base holds every commit on them, and
`rm` under `$TMPDIR` as under `/tmp`.

**In `auto` a session never waits on the person** (the owner, 8 Oct 2026).
Inside its slots and `/tmp` it runs, rewrites its own branch, pushes and reaches
off the machine unasked. A call that reaches into the owner's own files, or
removes or overwrites something outside its slots and `/tmp`, is refused with a
reason the agent reads and works around. Only a question (`AskUserQuestion`)
is put to the person; `ask` and `acceptEdits` still put calls to him.

**The slot is held by the session's id as a Job's is held by its id**, so a
restart of the process or of Fleet is not the session ending. **So is every
slot the session leases itself**: `armada worktree lease` run from a hosted
session reads `ARMADA_SESSION` and holds as a Job of that id, and takes a slot
of its own each call. Before this a lease was held for the agent's process,
which Fleet ends when it moves the session into a slot, so the lease read as
gone and the next caller took it. It is given back on close by parking every
slot held under the id: what is in each is committed and nothing is pushed.

**It is committed to the branch the slot is on, not the one it was leased on.** A
session rewrites its own branch inside its slot, so it may have moved to another.
Only the base and a detached head are refused, as for a Job. A Job's park still
refuses a slot on another branch than its lease names. **What the session left
running in the slot ends first**: every process whose working directory is the
slot or inside it (a backgrounded `pnpm mock`, a Check's process, a subagent's
agent), found by working directory and never outside the slot. This process and
its ancestors are spared. Fleet holds no pid for a Check run its CLI makes, so
that is how a run's process ends; the run itself is marked failed. A server
Fleet holds for the slot's checkout is stopped, and the session's open walk is
ended, which Bridge reads as its window closing. **A park the pool refuses
leaves the ledger's slot and branch rows standing**, so `who_owns` still names the
session as holding the slot, since a row given back over a held slot is how one is
stranded with nothing saying so. Fleet parks it again at boot, and again when a
repository is added, for any slot the pool holds under the id of a session that
has ended.

**The person sets the mode, the model and the effort** for each session. `auto`
is the default and means what it meant for Helm: everything runs except what is
destructive, pushes code to a shared space or writes off this machine, which
Fleet puts to the person on the session's own thread. The CLI's own auto mode is
not reachable for a spawned session, so `auto` is `default` with that door.
Inside the session's own slot, what it writes is its work and does not ask, and a
redirect into `/dev/null` is not a truncation; `rm`, `git reset --hard`, `git push` and the like
still do. `ask` and `acceptEdits` put every call the person's settings do not cover.
[Spike 25](../spikes/025-is-auto-mode-reachable-in-a-spawned-session-now.md).
A change applies from the next process: an idle one is ended at once, a running
one when its turn is over.

**A question is an ask too.** The agent's `AskUserQuestion` reaches the same door as any call, and
in a hosted session nobody could answer it. Fleet now holds it whatever the mode and the thread
draws it as a form: each question with its options, a multi-select where the agent said so, and
*Other* for the person's own words. *Answer* hands back one entry per question, *Skip* tells the
agent so. Fleet returns the tool's input with its `answers` map filled in, keyed by question text,
a multi-select's labels joined with `, `. [Spike 29](../spikes/029-why-a-sessions-auto-asked-and-how-a-question-is-answered.md).

| Operation | Does |
|---|---|
| `start_session` | A row for one repository, with optional title, model, effort and mode |
| `rename_session` | A person's name for a Session, hosted or in a terminal; stands until the next terminal `/rename` |
| `start_session_run` | A shell block run from a Session's thread, in its slot worktree or directory; output is held in memory only |
| `stop_session_run` | Ends a run's process group |
| `observe_session_run` | A run's output, then how it ended, on a socket of its own |
| `send_session_message` | Text, pictures and files stored by Fleet, and the sessions, Jobs, pull requests and branches it names; takes a turn |
| `answer_session_ask` | One of the offers on the ask the agent is held on, with an answer for each question where it asked some |
| `tune_session` | Model, effort and mode |
| `close_session` | Ends the process, parks every slot held for the session and ends what ran in it, ends the row and its walk. For a terminal session, the end is held for its mod (*Ending a terminal session*) |
| `stop_session` | Ends the turn that is running and leaves the row live (*Stopping a turn*) |
| `get_session`, `get_session_file` | The row with its thread, and what a message carried |
| `session.row` | One row of the thread, appended or replaced by its id |

**What a message names is one line Fleet adds for the agent** and the thread
keeps the person's words. A thread's rows are the person's messages, the agent's,
another session's, one line for each tool call, the lease, and each ask with
where it stands. A Helm Session's thread also has the `sent` row, the person's
words routed on to another Session or a Job, with where they went. **An ask is
the session's own**: it never appears in the old Helm dock's holds and
`answer_helm_call` does not find it. Helm's own questions are asks of its
Session, answered in its thread.

### What a restart does to a session

**A hosted session survives a Fleet restart, and so a Bridge restart.** Its agent
runs under a *keeper*, `armada session-keep`, which Fleet starts detached and
without Fleet's mark, so `crate::orphans` leaves it. The keeper owns the agent's
pipes, appends every line the agent writes to a spool file beside its socket
(`<support dir>/sessions/<id>.sock`), and serves that socket. Fleet holds a
connection, not the pipes, so the agent and the background subagents it started
keep running when Fleet stops.

| | Before | Now |
|---|---|---|
| Fleet stops | pipes close, agent exits, subagents die | agent and subagents run on |
| Fleet boots | next message resumes the conversation, in-flight work lost | Fleet reattaches, the thread continues from what the agent said while it was down |
| The next message | `--resume` | a line on the same process's stdin |
| A keeper that is gone | | a dead socket is removed; the next message resumes, as before |

**The keeper holds the offset.** Fleet acknowledges each line it has taken; on
attach the keeper replays from the last acknowledgement. A line taken but not
acknowledged when Fleet died is sent again, so one row may repeat; none is lost.
A Fleet that attaches mid-turn is told so, and the session shows as working.

**The wire** is lines of text over the socket. Fleet sends `ATTACH`,
`IN <line>`, `ACK <offset>` and `END`. The keeper sends `READY <pid> <busy>`,
`OUT <offset> <line>` and `GONE`. One client at a time; the newest attach wins.

**A keeper ends** when the agent exits and Fleet has been told (`GONE`), on
`END` (closing a session, the quiet timeout, a tune), or when no Fleet has been
attached for 30 minutes, which also ends the agent: a keeper no Fleet will
return to must not run one for ever. The permission door (`armada mcp`) waits up
to 90 seconds for a Fleet that is restarting before it fails a call. It reads
Fleet's port once, so a Fleet that comes back on another port is not found; the
port is claimed from the store and is normally the same.

**`END` takes the keeper's socket and spool away at once**, from Fleet's side,
so the next message starts a new keeper instead of reattaching to an agent that
is going. The keeper's own stderr, the agent's and how the agent exited go to
`<id>.log` beside the spool; Fleet puts its last lines in the thread's
"process ended" row.

**A process Fleet replaces takes its subagents with it.** A subagent runs inside
its agent's process, so the keeper only carries it across a Fleet restart. A tune,
a quiet spell, the move into a slot at the first write and a lost keeper each end
the process, and the one that resumes has never heard of the subagents that were
working. When Fleet starts a process for a session that has run, every `subagent`
row still `standing` whose transcript shows no finished turn was cut off. Fleet
gives those rows back, keeps their type, description and id in the restart's
note (so the retro lists them), and puts the same list ahead of the agent's next
turn, once, so it can start again what it still needs.

**Not covered**: a keeper's own death (the agent loses its pipes and exits), and
an agent whose hooks name a Fleet port that changed.

## Showing a window

**A session shows the person a web page with `show_window`** (`url`, optional `title`, optional `variations`), a tool on the agent's door, and Bridge opens it in a capture window by itself. Any `http` or `https` address; any other scheme is refused with its reason. Fleet places a hosted session by its connection, as for its asks, so the call names no session; a terminal session names its own id, which the mod puts in the model's first-message context. It writes one `window` artifact on the ledger however often the address is shown, and a quiet `window` row on the thread each time, which is what makes Bridge open (or raise) the window. Pressing the ledger row or the thread row opens it again, only for an address the ledger shows as a window. **A note taken in the window goes to the session as a message from the person**, with the element it pointed at, and wakes it. The window is pinned to the origin it opened on, as a server's is. **`variations`, a list of `{name, url}`, offers several takes of one change in the one window**, `url` the first's and each held to the same `http`/`https` rule. The bar draws a letter and name per take, a press swaps the page, and the pin moves to the origin of the take chosen and to no address the session did not send. A note names the take it was taken on (`On B, Needs you first: …`), Approve names the one on screen (`Approved B, Needs you first: <url>`) and is per take, and the window row and the ledger's `window` artifact keep the takes so a reopened window brings them back. **The bar says whether the page is ready to walk**, a Session mark at its start (*Ready to walk*, or *Still changing*) and the bar's hue, and Approve reads *Not ready* while it changes. **It is detected, not declared:** `show_window` says ready, the mod's report of a file the session itself wrote (`wrote`; a subagent's does not count) or a note from the window marks every window the session showed as working (`detail.working` on its artifact), and showing the page again or the end of the turn (`turn_completed`) marks it ready. A Job's window reads the Job: ready while it waits on the person, working while it runs. **The window draws the session beside the page**, its thread and the box to answer it, so the walk is read and answered without going back to Bridge's window. `docs/practices/capture-window.md`, *The panel beside the page*.

**A question asked while the person walks a page is refused, not held.** A message starts a turn only once the current one ends, and an open `AskUserQuestion` keeps it open, so every note and the Approve would queue behind the question until he answered it. [Spike 27](../spikes/027-how-does-fleet-deliver-text-to-a-live-terminal-session.md).

| | |
|---|---|
| A walk opens | At `show_window`: the newest `window` row on the thread |
| A walk ends | The person approves the page (`Approved: <url>`, or of several takes `Approved B, Needs you first: <url>`), Bridge says its window closed (`window_closed`), or a newer `show_window` replaces it. The row gets `ended_at` and is published again |
| A note in the window | Ends nothing, though it settles the `walk:` waiting item |
| The window cannot show the page | Ends nothing. Bridge says so (`window_failed`) and Fleet tells the session in its own voice, with the address and what went wrong: a failed load, a status of 400 or more, a crashed page, Vite's error overlay or a page with nothing drawn. Not while the page is still changing, and once until it is seen working |
| A question meanwhile | Refused with a reason the agent reads: put it under `waiting_for` and end the turn, or, for a question about the page, ask it in the window with `ask_in_window` (*Asking in the window*). A hosted session's is refused at its permission door; a terminal session's at `ask_from_terminal`, which the mod asks before the terminal's own prompt opens |

The mod's first-message note and `show_window`'s description tell the agent the same. Closing the window ends the walk and does not settle its `walk:` item, which still waits. A Bridge that quits with a window open may not reach Fleet to say so. The walk then reads open until it is approved or replaced, and a question meanwhile goes under `waiting_for`, which blocks nothing.

## Drawing a sketch

**A session draws the person a diagram with `show_sketch`** (`title`, `scene`, optional `against`), a tool on the agent's door, in place of ASCII art. The scene is the format of `docs/concepts/sketch-pane.md`: `nodes`, `edges` and optional `strokes`. Fleet places a hosted session by its connection, as for `show_window`; a terminal session names its own id.

**A scene that `parseScene` would draw as an empty frame is refused first.** `ipc` (`sketch.rs`) checks it against the same rules and answers 422 with up to twelve problems, each naming the part (`nodes[1] `b``), what it holds and what is allowed. It is stricter in one way: a field the format has no name for is refused, where `parseScene` drops it. A scene with no nodes is refused. The kinds, wire parts and icons are listed in the tool's description, and `crates/ipc/src/tests/sketch.rs` fails when they differ from `scene.ts`.

| Cap | Holds |
|---|---|
| Nodes, edges, strokes | 80, 120, 20 |
| Scene, written, with `against` | 32768 bytes |
| Title | 120 characters |

**It is kept as an artifact of form `sketch`**, `target` `sketch:<title>`, with `detail` of `by` `agent`, `title` and `scene`, the scene as JSON text, and `against` where the call named one. A second call with the same title replaces the first where it stands; another title adds one. It stays on the ledger after the session ends and survives a restart. The call is a tool row on the thread whose text is `<tool> <arguments as JSON>`, the arguments unwrapped from the door's `body` and carried whole (`title`, `scene`, `against`; not `session_id`), which Bridge draws the sketch's card from. No window opens. Through the door the answer is a receipt of `session_id` and `sketch`.

**`step` orders what plays.** It is a finite number on a node or an edge, with one numbering for both: everything with the same step plays together, in ascending order, the dot travels an edge from `from` to `to`, and an edge with no step never pulses. A sequence diagram is groups as lifelines, label nodes as points and stepped edges between them; a workflow steps its nodes and its arrows.

**`against` names an earlier sketch of the session by title**, which the owner reads this one as a change to. Fleet resolves it at the call and keeps that scene whole in `detail.against`, a JSON object `{"title", "scene"}` written as text (`detail` holds strings only), `scene` being the earlier scene's text as it was kept. A later replacement of the earlier sketch does not move the diff, and a sketch may be drawn against the title it replaces. A title the session holds no sketch of is refused with the titles it does hold, and a blank one is refused. The earlier scene counts in the byte cap with the new one.

**The scene rides on the record**, which is published whole on every change, so each sketch is carried by every event after it. That is why the byte cap is 32 KiB and not more.

## Asking in the window

**A question about the walk opens in the walk.** A session that wants the person's answer about something on a page it showed calls `ask_in_window`, a tool on the agent's door, with the `question`, its `options` (each a `label` and an optional `description`), the page's `url` and a CSS `selector` for the place, with an optional `text` the element shows that picks one where the selector finds several. The owner, 11 Oct 2026: "a question about the walk should open in the walk. In fact it should take me where the agent is asking me about. Highlight it like an annotation and show a card with the question and my options to answer. If the question is not related to the walk it should wait."

| | |
|---|---|
| What it may ask about | An address on the origin of a window the session showed, or of any of its takes (the window swaps to that take), whether or not its walk is still open: a closed window is opened again. Anything else is refused with a reason that names `waiting_for`, where a question about something else waits |
| What the call does | Keeps a `window_ask` row on the thread and answers at once. **It does not wait**: the session ends its turn, and the answer arrives as a message from the person |
| What Bridge does | Brings the session's walk window forward, opening it if it is closed, moves the page to the address, scrolls the place into view, outlines it, and draws the question on a card of the window's own bar. `docs/practices/capture-window.md` |
| How it waits | `window_ask:<row>` under Waiting on you, with its options and its `place`. Its act is a walk of the window it was asked in, so a press there opens the window with the card |
| How it is answered | An option, words of the person's own, or both, through `answer_waiting`. The session gets `Re: <question>` with the option and the words beneath it, and the row keeps the answer. Being a message, it also settles the walk it was asked in. `dismiss_waiting` drops it with nothing sent |
| While sleep is on | Recorded as a walk is and never decided for him: it is about what is on the page |

Capture and Approve keep working while the card is open. The card folds into the bar so the page takes presses again, and the window keeps showing it until it is answered or dismissed.

## What a Session is doing

**Nothing the person sees depends on the agent making a tool call.** Fleet reads what a Session is doing from what it observed, and `waiting_for` and `all_done` only add detail to it. `SessionRecord.standing` says it, for a live Session; an ended one has none.

| `standing` | Read from | Item in `waiting_for` |
|---|---|---|
| `working` | a turn is running and something was heard from it within the bound, or, between turns, a `subagent` row of it stands: a parent that ended its turn to wait on its subagents read as idle while they ran (14 Oct 2026) | none |
| `stuck` | a turn is running, nothing has been heard from it for **ten minutes**, and it is not held on a question or a permission | none |
| `done` | `done_since` stands (the agent said nothing is left) | `done:<done_since>`, text `Done` |
| `replied` | the last turn ended on a message to the person, and he has not written since | none: a reply asks nothing of him |
| `idle` | none of the above | none |

**The reply is read off the turn.** For a hosted Session the turn is on its stream: the last thing the agent said before the turn ended. For a terminal Session the mod reports `turn_started` when a prompt goes in and `turn_completed` with `said`, the first line of the last assistant message, when the turn ends; the agent is never asked to. Fleet keeps the turn's end and that line in `session_turn_end` (one row a Session), so a reply survives a restart. It stops standing when the person writes to the Session, when another turn begins by any route and when the Session ends. A turn that ended on tools alone, with nothing said, leaves no reply, and a Session whose turn ended before Fleet observed this has none: nothing is read back from old turns. A Helm Session has no reply, since its replies are the dock's. A `done` outranks a reply.

**A reply is not something the person owes, so it is no item and raises no call.** The owner, 14 Oct 2026: "When a session replies to me, it shouldn't show up in needs me. Needs me is when it needs something from me. Replied should be idle or unread." `replied` is an idle Session with an unread mark, which Bridge draws and clears once he has been in the Session (`cockpit/viewed.ts`); Fleet does not yet say that he has. `WaitingSource::Replied` stays in the enum, since Bridge's mirror names it, and Fleet no longer serves it. A `done` is still an item: it asks whether to end the Session.

**Stuck is its own bound**, `STUCK_AFTER` in `session_host/standing.rs`, ten minutes of nothing heard from a running turn. It is not `settings.session-quiet-timeout`, which is how long an idle process is kept and says nothing about a turn that went quiet. A Session goes stuck by time passing, which no event says, so the notice loop's tick looks at every running turn and publishes the Sessions whose stuck state moved, in both directions (`standing_checked`). A terminal turn's running state is kept in memory only: a Fleet restart forgets it, and the Session reads as idle until its next turn starts. A mod from before `0.3.20` reports neither `turn_started` nor `said`, so its Session is never working or stuck by this reading and has no reply; a `done` still stands.

**A terminal's permission prompt.** Claude Code in a terminal asks its own permission for a tool call, and nobody may be at the terminal. The mod forwards what the harness announced: its `PermissionRequest` hook reports `permission_asked` with the tool and its command or path, where no hook beneath decided it, and the tool call ending reports `permission_settled`. The agent is never asked. The mod also puts the prompt to Fleet as it puts a question, and races the terminal against the answer: **the call is answered from Bridge**, Allow once or Refuse, as a hosted Session's permission is, or at the terminal, whichever comes first. An allow from Bridge runs the call again past its own check, and the terminal's dialog closes (`docs/spikes/031-can-bridge-answer-a-terminal-permission-prompt.md`). In the Session the command is a block to run, and its ▶ is Allow once. Allow and remember is not offered, because remembering is the harness's own. Until the ask arrives Fleet raises the call from the fact alone, with no options and the `message` act naming the Session. While it is open the turn is waiting on the person, never stuck. A turn starting or ending, and the Session ending, closes it too. A mod from before `0.3.22` reports neither, so its prompt raises no call and the turn reads stuck after ten minutes.

**Bridge reads the state from `standing` and from nothing else**: not from an event's `live` flag or its age, and not from whether a tool was called. `Timeline::session_turn` marks no entry live, because a turn that finished is not moving. An answer sent from a card counts as him being in the Session, so the items he has answered stop being calls (*Waiting on the person*).

## Waiting on the person

**A session says what it is waiting on the person for with `waiting_for`**, a tool on the agent's door that replaces the whole list each call (`items`: `id`, `text`, an optional `act` of `walk`, `answer`, `approve_pr` or `run` with a `target`, optional `options`, and optional `steps`, a list of short lines); an empty list clears it. Fleet keeps the agent's items in `session_waiting`, stamps `since` and keeps it for an unchanged `id`, and **adds its own on every read** into `SessionRecord.waiting_for`: `ask:<call>` for an open question, `perm:<call>` for an open permission, `walk:<url>` for a window the session showed that no `Approved: <url>` message (of any of its takes) has followed, `window_ask:<row>` for a question asked in the window and not yet answered (*Asking in the window*). Where an agent item names the same act and target as one of Fleet's, Fleet's stands, and an agent `walk` item for any address the session has shown in a window (trailing slash aside, approved or not) is left off: the window is the one record of that page. An ended session holds nothing. **Bridge shows the list at the foot of the thread**, over the composer, as a **Needs you** band folded to one glyph an item, named on hover; a press opens it to the rows, each a press that does its act. The owner, 12 Oct 2026: "when the session is not completed, the 'needs you' items in the right panel move to a panel in the thread like this that is locked to the bottom. Maybe it's collapsed by default?" A question or permission is left out of the band, since its own card shows it. Once the agent says it is done, the done callout takes that place and the items go back to the ledger's first section, **Waiting on you**. Bridge puts a Session with items under Needs you with a bell-ring mark whose tooltip lists them. **The Cockpit raises a call for an item only while he has not been in the Session since it began waiting** (*Job Board*, the call deck): Bridge keeps when he last viewed each Session (`localStorage`, per id), his latest message counts as being in it, and an item whose `since` is older than the later of the two is no call, though it stays in the list. **`answer_waiting`** (`session_id`, `item_id`, and a `choice` index into the item's `options`, `text`, or `mode` of `best` or `quick`) settles one: a card item goes through `answer_session_ask`, a walk item sends the approval as the person's message, an agent item becomes a message to the session, and once that is delivered the item is dismissed as `dismiss_waiting` does, so it leaves the list without the agent restating it. A `choice` with `text` says both: the words go with the pick, to the agent as a second line and to an ask card as its note. `best` tells the agent to decide for itself and think it through, `quick` to just get it done: finish the work and land it, skipping the formalities. **`dismiss_waiting`** (`session_id`, `item_id`, and an optional `reason`) drops an item for good: the id is kept in `session_waiting_dismissed`, a derived item with it never comes back, and an agent item with it is removed and left off if the agent states it again. **An item the agent stated tells it so**: the Session is sent `Owner removed "<item text>"`, with `: <reason>` after it where the person gave a reason that is not blank, so the agent does not take the silence for an answer. The owner, 14 Oct 2026: "when there is a 'needs me' item on a session, I should be able to delete it, which lets the agent know that I removed it. Maybe I can give a reason when I do." An item Fleet derived (a card, a permission, a walk, a window question, the first message, a done) is only dropped, and the agent is told nothing, since it never stated it. The message goes before the item is dropped, so a send that fails leaves the item standing to try again. A nothing-held id is refused as for `answer_waiting`. **Only the newest walk window of a session waits**, and it stops waiting once the person has sent the session a message after it opened; closing the window does not settle it. The mod's first-message note tells the agent to keep the list current, to give each item an act, and never to list a page it showed with `show_window`, which waits by itself, and puts these tools ahead of the person's own rules for reporting actions and questions in a reply. It also says a decision is asked one way, in the prompt or in `waiting_for` and never both, that a shell command for the owner goes in a block tagged `bash` so Bridge draws its run button, and that a page is shown as a link and never a screenshot. **An agent item that carries `steps` is a procedure**: its glyph is Steps, and its row reads its `text`, and its press opens the steps beside the thread in place of its `act`. A `walk` stays as the page they are about, one key at the top and a second that opens it in the browser; a `run` stays as its command, drawn whole at the top with a copy key in the block's corner, and left off where a step names it. A step's `backticked` command is set as code in its line and drawn again under it, whole, with its own copy key. The owner, 15 Oct 2026, on a run item whose steps hid its command: "The copy button should be embedded in the code block instead of outside of it." Each step has a tick that is the person's own place-keeping, a step the agent got wrong is edited in place, dropped (struck through, kept) or added at the end, and the key under the reply box reads Done while the box is empty and Send once something is typed. Done answers the item with `Done`, ticked or not; either key sends the corrections as plain lines after the reply (`Step 2 changed: "was" → "now"`, `Step 4 dropped: "was"`, `Added after step 6: "now"`). Fleet trims each step and drops a blank one; an empty list is no steps. The agent keeps `text` to one short line. An agent item with none, or with `answer` and no `options` while no question is open in the thread, gets a Reply glyph, and its press opens a box under the row whose text goes to `answer_waiting` as `text`.

**A session says it has nothing left to work on with `all_done`** (`session_id` only; the hosted session holding the connection wins, as for `waiting_for`). Fleet stamps `SessionRecord.done_since`, keeps it in `session_done`, and publishes `SessionChanged`; a second call keeps the first instant, and an ended session is refused. It is a statement and ends nothing: Bridge draws a callout at the foot of the thread, **End session** (the ordinary close) or **Keep going** (local to Bridge, it hides that `done_since` only). `done_since` stops standing when the person sends the session a message (`send_session_message` or an `answer_waiting` that becomes one), when the agent calls `waiting_for` with a non-empty list, and when a hosted session's turn starts again, except a turn that Fleet's own news starts (`told_news_by_fleet`: a pull request merged, a build moved), which asks the agent for nothing. A word that asks for work, such as a failed check or a stranded slot, clears it. A terminal session's mod reports a prompt going in as `turn_started`, which is the same turn-start signal; a mod from before it has none, and for that one only the first two apply. The mod's first-message note tells the agent to clear `waiting_for`, then call `all_done` only when nothing is owed and none of its background work is running.

**A session gives back what it is finished with using `finish_up`** (`session_id` only; the hosted session holding the connection wins). It releases each slot the pool holds for the session as `armada worktree release` does, which refuses a tree with uncommitted files or commits that neither its branch's remote nor the base has; stops the servers running in each slot it released; and deletes the local branches those slots were on, the one the session was leased and the head branch of each pull request it holds or held, where the base (the local branch or `origin`'s) already holds every commit. Answered with `released` (slot numbers), `deleted_branches` and `kept`, a sentence for each thing that stayed and why. It commits nothing for the agent. **It is a call on Fleet's door because a command cannot do the same:** once a slot is released the next write in the session takes a new lease, and the git guard refuses `git branch -D` and `kill`. A terminal session is refused; its slot is parked when its process ends. The mod's first-message note tells the agent to call it before `all_done`, and to start servers with `start_server` so it can stop them.

**A session marks done a note Bridge sent it using `mark_annotation_done`** (`note_id`, and `session_id` placed as `finish_up`'s is). A note id is letters, digits and `-`, so it names no path outside an annotations directory. Fleet looks for `<note_id>.json` under the served root's `.armada/annotations/`, then the preview's `.armada/preview/.armada/annotations/`, then each slot's and each `.claude/worktrees/` checkout's; none found is refused, naming where it looked. Only a note whose `sent.sessionId` is the calling session is marked: `status` becomes `done` and `updatedAt` Fleet's now, every other field stays in Bridge's order, and the file is written through a rename. Answered with the `path` written. **It is a call on Fleet's door because the note may sit in a checkout the session holds no slot for**, such as the preview's, and the slot guard refuses that overwrite; the guard is not widened for it.

**An agent that calls `show_window`, `ask_in_window`, `waiting_for` or `all_done` through the door is answered a receipt, not the row.** The row grows with every attachment (36 to 62 KB for a long session, 10 KB at the median, measured 7 to 13 Oct), and an agent keeps each answer in its context for good. The receipt is `session_id` and what the act set: `window` (with `title` and the `takes` names), `asked` (the question's `window_ask:<row>`), `waiting_for` (the agent's items as Fleet kept them, `id` and `text`), or `done` and `done_since`. It weighs the same on the first call and the thousandth. Bridge's routes and the mod's hook, which are not the door, are answered the row. Nothing reads these four answers today.

## Working on

**A hosted Session shows what it is doing, and Fleet detects it.** `SessionRecord.activity` (`SessionActivity`: `now?`, `todos[]`) is built from the tool calls Fleet's PreToolUse gate sees, which is every tool call of a Session Fleet hosts: the hook's matcher is `*`. Until 14 Oct 2026 the matcher named only the five tools a lease is taken on, so a read, a search, a subagent or a `TodoWrite` never reached Fleet and `now` and `todos` stayed empty for a Session that was not writing. A call to any other tool is noted and let through before the pool or the Manifest is read. No tool asks the agent to say it, because Fleet does not rely on an agent calling a tool to know its state (the owner, 14 Oct 2026). The one thing the agent does say is its own to-do list, and Fleet reads that off the call too.

| Field | Where it comes from | What it is |
| --- | --- | --- |
| `now` | Detected, from the call's tool and input | One short fact for the latest call Fleet can name. A call it cannot name leaves it as it was |
| `todos` | Reported, as the agent's own `TodoWrite` list | Each `TodoWrite` replaces the whole list. `text` is the item's `content`, and `completed`, `in_progress` and `pending` are `done`, `doing` and `todo`. An empty list clears it. A `TodoWrite` never changes `now` |

`now` reads in one of these forms, with no counts and no narration:

| Tool | `now` |
| --- | --- |
| `Edit`, `Write`, `MultiEdit`, `NotebookEdit` | `Editing gate.rs`, the file's name without its directory |
| `Read` | `Reading gate.rs` |
| `Bash` | `Running cargo check`: the command's name, and its subcommand where it is a tool that has them (`git`, `cargo`, `pnpm`, `armada` and the like) and the next word is a plain one. A leading `cd x &&`, an environment assignment or a wrapper such as `time` is skipped, and a line with nothing in it that runs a command leaves `now` alone |
| `Grep`, `Glob` | `Searching` |
| `WebFetch`, `WebSearch` | `Fetching` |
| `Agent`, `Task` | `Running a subagent` |

**It is kept with the rest of a Session's runtime state, in memory.** It is not in the store, a Fleet restart starts it empty, and it is dropped when the Session ends. The gate notes a call only for a Session that is hosted and open, after the check that already passes an unknown or ended one, and does so whatever the gate then decides about the call.

**A call does not publish a row.** The row is large and goes out whole with every `session.changed`, and a busy Session makes many calls. A change to `now` or `todos` publishes at once when the Session's row has not gone out for what it is doing within the last two seconds. Otherwise one publish follows when those two seconds are up, carrying the latest, so the last change is never lost and a burst costs two rows. A call that changes neither publishes nothing, so a long run of identical calls costs no rows. The gate takes a lock for a moment and awaits nothing, and the publish is a task of its own, so the answer to the call is not held for it.

**A turn starting clears `now` and keeps `todos`.** The line of the last turn is not the new turn's, and the agent's list is still true across turns. It is the same moment `done_since` stands down, and a turn Fleet's own notice begins counts: a new turn is a new turn. The row that announces the turn carries the clear. A turn ending clears nothing.

**Bridge draws nothing for a Session whose turn is not running.** Whether a line or a band is drawn is Bridge's rule, read off `hosted.turn` and these fields.

**Silence is not here.** A running turn nothing has been heard from is the `stuck` state, from the activity of the process Fleet hears, whatever it calls. `activity` says what a Session is doing, and carries no time to read a silence from.

**Only a hosted Session is covered.** A Session run in a terminal makes its tool calls in a process Fleet does not host. Its mod reports a few tools through `report_session` and does not call the gate, so such a row carries no `activity`.

## The build record

**`SessionRecord.build` says how far a Session has got, read from what Fleet observed.** It asks the model for nothing: there is no tool for the agent to call and no line in any prompt. The agent states a plan with the task tools it already uses, the mod hears those calls, and Fleet checks the plan against the edits it saw. Bridge draws the record as a ring under the Session's drone (the decision of 14 Oct 2026, *A Session builds a ring*). The field is absent for a Session that has done nothing Fleet observed.

**What the mod tells.** `TodoWrite`, `TaskCreate` and `TaskUpdate` become one fact, `tasks`, holding the whole list after the call. `ExitPlanMode` becomes `plan_left`. A successful `git commit` and `git push` become `committed` and `pushed`, read from what git printed and not from the command alone, so a commit with nothing staged, a push with nothing to send, a rejected push and a `--dry-run` tell nothing. A subagent's own list is not the Session's and is not told, but **its reads, edits, commands, commits and pushes are the Session's work**: in a terminal Session the mod's hooks fire inside subagents and report under the parent, and a hosted Session's process stream carries a subagent's tool calls, which Fleet hears without drawing its words. A hosted Session feeds the same path from its gate (`TodoWrite`, `ExitPlanMode`) and from its process's tool events. **A hosted Session is started with `CLAUDE_CODE_ENABLE_TODO_TOOLS=1`**: a headless process has no task tools without it, so until 15 Oct 2026 its agent could write no list and its build record never had pieces.

**What Fleet keeps.** Two tables (`session_build`, `session_build_tasks`): the instants of the first edit, the last edit, the last tool activity, plan mode left and the list's first write, and the list with the instant each item was first seen completed and whether an edit was seen while it was worked. Items are matched by id and text, else by text, so a reordered list keeps what was seen of each. An item that goes back to not completed loses its instant. Everything else is derived on read. A walk shown, a commit and a push are timeline entries (`session.walked`, `session.committed`, `session.pushed`), and a pull request's stops are the entries it already has (`pr.opened`, `pr.checks`, `pr.queued`, `pr.merged`); the row is published again when one is written.

**A row with no first edit is filled from the timeline.** When a Session's row is first made, and on each fact heard while the row still has no first edit, Fleet reads that Session's `session.edited`, `session.committed` and `session.pushed` entries in one query, from the later of the Session's start and the last merge of a pull request it held. The earliest of the three is the first edit, the last `session.edited` entry is the last edit, and the build's start moves back to the earliest. This covers work heard before this Fleet started or before the row existed, which the timeline keeps for 30 days. A row that has a first edit is not read again, and no row is made from nothing kept. An entry is dated by its burst's first call, so a seeded last edit is when the last burst began.

**A commit or a push counts as the first edit** where none was heard, since it proves code changed even when a script made the edits through a shell. A read or a command does not. A commit or a push does not start a new build after a merge, as a read does not.

**A build ends where its pull request merged.** The first edit, plan or task list heard after the merge starts another build with no list and no road. A read or a command after the merge does not.

| Field | Read from |
|---|---|
| `phase` | `landed` when the pull request merged. `road` when a stop was reached at or after the first edit, or any stop where no edit, commit or push was ever observed. `plan` when a task list was written or plan mode left and no edit followed. `implement` when edits, a commit or a push were seen. Otherwise `scope` |
| `pieces` | One per task, in list order. `solid`: completed, and an edit was seen between the previous completion (or the list's first write) and this one. `hollow`: completed with no edit seen in that window. `scaffold`: the first task not completed, while an edit has landed since the last completion. `ghost`: every other. With no list there are none: progress is never invented |
| `stops` | The road as drawn: the stops reached in the order they were, then the forecast ones not yet reached, each with no `at` |
| `forecast` | The stops the repository's recent landed Sessions usually reached |
| `landed_at` | When the pull request merged, as the forge dated it |
| `quiet_since` | The last read, edit, command or task list. It is as of the last time the row went out, which a call after a minute of quiet does |
| `stalled` | The Session's `standing` is `stuck` |
| `workers` | The subagents still standing, each as `Type: description` |

**The stops.** `walk` is the first walk shown after the first edit. `pushed` is a push that updated a ref. `pr` is the pull request opened, which is never later than its first check, queue entry or merge. `checks` is its first CI state. `auto_merge` is its first queue state. Merging is not a stop: it closes the road and is `landed_at`. The pull request is the newest one the Session holds that is not closed and did not merge before this build began.

**The forecast rule.** For the Session's repository, read the pull requests that merged most recently and that a Session held, up to ten. For each, take the stops reached between that Session's previous landing and this one, in the order they were reached. The forecast is the most common whole sequence among them, and a tie goes to the more recent. A landing with no stop observed takes no part, and a repository with no history has no forecast. A stop already reached is not drawn faint twice.

**What is not observed, and left absent.** A Session resumed in a terminal after its mod restarted does not know the ids of tasks it made earlier, so a `TaskUpdate` for one tells nothing until the list is written again. Edits made through a shell command (`sed`, a redirect) are not edits, though the commit that follows them is the first edit if none was heard, and nor are files outside the Session's directory. A task completed before its edits were filled in from the timeline stays hollow, and a seeded last edit is only as exact as the timeline's bursts. A Session with no new fact since a Fleet restart keeps the row it had until its next tool call. A task the agent never marks completed is never solid or hollow. Auto-merge set through the forge's page, and checks Fleet's merge line read only after the merge, may have no stop.

## What fills the context window

**A Session's context tank opens onto what fills it, the way `/context` shows it.** The mod asks the engine for a `summary` breakdown, estimated locally and never `full`, which would call the token-count API for every tool and file. It asks when the throttle lets a measurement through and the context moved, and sends `SessionRecord.context_breakdown`: `categories` (`name`, `tokens`, and a `kind` of `used`, `free`, `buffer` or `deferred` that Bridge branches on, never the name), `memory_files` (`path`, `tokens`), the ten `mcp_tools` that weigh most (`name`, `server`, `tokens`), `total_tokens`, `max_tokens` and `autocompact_threshold?`. **Names, paths and counts only**: no message text, prompt or file contents leave the mod. The live token count is the `context_tokens` already on `usage`.

## A session that hit the plan's limit

**A session the plan's limit stopped is woken when the limit resets, and nobody has to poke it.** A turn that ends on `error: rate_limit` is reported by the mod as `limited`, with the instant the full window opens again: the latest reset among the windows at or near full, so a weekly window spent beside a five-hour one is waited out. A limit no window shows full leaves nothing to wait for, and is not reported. Fleet keeps the instant in `session_limit`, one row a session, and serves it as `SessionRecord.held_until` for Bridge to draw as `battery-low` reading "Limit reached · resumes 6:40 PM" (`hourglass` is banned; `packages/icons/icons/battery-low.toml`).

A minute after `held_until`, a tick of the notice loop sends the session `Continue` through `told_by_fleet`, the word Fleet already has for its own messages, and lets the hold go. The row is persisted, so a Fleet that restarts meanwhile still wakes it.

| Session | The wake |
|---|---|
| Hosted | A turn on its process, started again if it was ended for being quiet. The thread draws `Continue` as Fleet's |
| Terminal | Held for its mod as `Fleet: Continue` and submitted when the mod next asks. A mod that is not asking is tried again each tick, and given up on 30 minutes past the wake |
| Ended | Nothing is sent, and the hold goes |

**A session that took a turn in the meantime is not woken.** The hold goes with a finished turn, with a message from the person, with a hosted turn starting and with the session ending. The turn the limit ended is not a finished one: the mod does not report a turn that died on an API error as `turn_completed`.

**The mod keeps no timer of its own.** A terminal session is woken by Fleet through the path it already asks on; a mod timer would wake it again after a Fleet restart that had already done so.

## What Fleet tells a Session

**A Session never has to be told to watch its pull request.** When a pull request a Session holds fails its checks, conflicts with the base, leaves the merge queue, is marked unmergeable, or merges, Fleet sends that Session one message, once. It names the pull request, what happened and the branch. For failed checks it lists each failing check with its log address, and the last lines of the first two logs. A merge tells the Session to call `finish_up`, which releases its slot and deletes the merged branch, and never to delete the branch by hand or ask the owner to: ending the Session parks its slot and keeps the branch. `docs/concepts/fleet.md`, *Telling the owner of a pull request*, has the triggers.

| Session | What happens |
|---|---|
| Hosted | The message is a turn: it wakes the Session and starts its process if it was ended for being quiet. The thread draws it as a message from **Fleet**, the voice another Session's message has, named `Fleet`, so it is never the person's |
| Terminal | Fleet cannot push into one. The message is held for its mod, prefixed `Fleet:`, and handed over when the mod next asks. A mod that is not asking leaves it unsent and Fleet tries again at the next reading |
| Ended | Nothing is sent |

**A Session is told when what it wrote would conflict.** When Fleet finds that the files a Session wrote conflict with another Job's or Session's, or with the base after it moved, it sends that Session one message: the files, who the other is and how to reach it. The Session is not paused and the message is sent once per set of files. `docs/concepts/fleet.md`, *Write clashes*. Bridge marks the Session's row, and its open thread names the other holder and the files, from the standing clashes Fleet lists.

| The other is | The message names |
|---|---|
| A Job | Its handle and title, and says to send its Drone a message with `redirect_drone` and the Job's id |
| A Session | Its address, `s-` and the first eight characters of its id, and its branch, and says to use `SendMessage` to that address |
| The base | The commit on it that changed the first file, and says to merge it into the branch |

A terminal Session reports the files it wrote through the mod's `touched` fact, as paths from its directory. A Job's Drone can answer a Session with `leave_note` on the Session's address, which arrives as a message naming that Job.

**A Session that held a slot is told when the slot is stranded.** When a slot a Session held last has work in it and nothing holds it now, Fleet sends that Session one message, once. It names the slot and its branch, what the slot holds, and asks the Session to commit, push and release it. `docs/concepts/fleet.md`, *Rescuing a stranded slot*.

## Acts on a pull request

A pull request a session holds is a `pr` row, and a person can act on it without leaving the session. The pull request is named by its repository and number, and is nobody's Job.

| Operation | Does | Refuses |
|---|---|---|
| `get_pull_request` | Reads it: draft, open, merged or closed, auto-merge, the forge's checks (`pending`, `passed`, or `failed` with the failing names), title, branch, address | Where the forge would not answer |
| `ready_pull_request` | Takes a draft out of draft | Anything that is not a draft |
| `merge_pull_request_by_number` | Merges it | While the checks are running or failed, a draft, a closed one, and the forge's own refusals |
| `enable_auto_merge` | Asks the forge to merge when the checks pass | Once they have passed or failed, a draft, a closed one |
| `review_pull_request` | Drafts a Code Review Job against it, by number or address | A name that is neither, a Session the ledger never heard of |
| `get_pull_request_detail` | Reads the page the panel draws: base, author, mergeability, review state, the conversation, the forge's checks with their run time, files and commits | Where the forge would not answer |
| `get_pull_request_file` | One file's patch, read when a person presses a row of the Files tab. Bridge draws it in a panel to the left of the pull request's | Where the forge would not answer, and for a file the pull request does not list. A file the forge sent no patch for answers with Fleet's sentence |
| `write_pull_request` | Comments, approves or requests changes, then reads the page again | A comment or a request with no body, a Session the ledger never heard of, and the forge's own refusals |

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
> Why: a Session's `pr` row is what the harness reported, and `detail` (`state`, `auto_merge`, `checks`, `failing`, `title`, `branch`, `address`, and `body` where the author wrote one) is Fleet's reading of the same pull request, so one writer is not two.

**Only a failed required check is `checks: failed`.** Where a repository names a gate (`ci`), a check outside it that is red while the gate runs reads `pending`, and one red after the gate passed reads `passed`, with its name still in `failing`. Bridge draws those two as a tinted chip and leaves the Session's row alone; only `failed` puts a Session under Idle with a red mark, and a Session that waits on the person is under Needs you whatever its checks say. Fleet refuses a merge on all three.

**The three writes are a person's ask, `pushes to shared`.** The operations are `Helm only`, and Helm calls them on a person's ask. Bridge's own buttons are the person's press and reach the routes directly. `get_pull_request` is open to any agent.

**A review is at the approval gate**, like every dispatch, with `origin` `session_dispatched` (*From a Session, by you*) and the pull request as the Job's subject. The workflow is `code_review`, named by Fleet and never read off a link by the proposer. Where a Session pressed it, the Session holds a `job` row whose `detail.origin` says *dispatched from Session <id>*, which is how the Board row and `who_owns` name it.

**A press is drawn as underway until Fleet has answered and the pull request has been read again.** The pressed button takes Button's pending rendering with a turning ring before its label, the other acts are disabled so a second cannot start, and a refusal is said above them and the button returns. A Review Fleet accepted reads "Review dispatched", disabled, for as long as Bridge holds that Session's pull request open, since a second one would review it twice. Merge, Enable auto-merge and Ready for review show the state the re-read brings.

## Started on a piloted Job

`start_session` takes an optional `pilot` (`job_id`, `outcome`), and that one call is the whole of taking a Job over from a Session: Fleet runs `take_over` first, which is the only thing that can refuse, then writes the Session already holding the Job's slot and branch (`detail.handed`), with the Job as its own `job` row. **It runs in the Job's worktree from its first message and takes no lease on a first write**; its thread opens with a `handoff` row, the bundle's structured fields. The bundle as the agent reads it goes ahead of the person's first message and starts no turn of its own, so a piloted Session nobody has spoken in has spent nothing.

**Ending the pilot, or closing the Session while the Job is still piloted, hands the worktree back to the Job.** The Session's lease is cleared, so its next process starts in the repository and its next write leases a slot of its own. A close parks nothing in that case: the checkout is the Job's, not the Session's to commit.

## A forked session

A session that is **ended or dead** offers Fork, in place of its message box. `start_session` takes `fork { session_id }` (23.69) and starts a new session hosted by Bridge as a copy of that conversation. Spike 28 measured the agent side.

| | |
|---|---|
| Dead | The ledger says `ended`, or it is a terminal session whose mod has not asked for ten seconds (`terminal.listening` is absent). A hosted session is dead only once it is closed |
| Refused | A live session, 409 `fleet.session_fork_live`. An unknown id, 422 `fleet.no_such_session`. Nothing is written |
| The fork | Its own id, ledger and slot. It takes the old title unless one is given, starts in the repository's main checkout, and leases a slot at its first write like any other session. It holds none of the old session's slot, branch or pull requests |
| First process | Resumes the old id as a fork under the new one, so the conversation is copied and the new id is Fleet's. Once it has run, the session resumes by its own id |
| Ledger | A `forked_from` row on the fork and a `forked_to` row on the old session, both `spent` |
| Told | **Every process of a fork has a note in its system prompt** (`--append-system-prompt`): forked from <title> (<id>) at <time>, that session is live or has ended, and nothing it started is the fork's. Its subagents, background tasks and shells, slots, branches, windows, pull requests and peer sessions stay its own, and a task-notification about its background work is not acted on, relaunched or reported. A fork launch cannot be built without the note (`HostedLaunch::forking` takes it) |

> **Rule.** The whole of a session is forked only once it is ended or dead. A live one is forked at a message, below.
> Why: a copy of the whole of a conversation still being written is two sessions racing to the same next message, and a session that can still be spoken to needs no copy.

### A fork at a message

**Words selected inside one message offer Fork from here**, on a live session as well as a dead one. `fork { session_id, at }` (23.79) copies the conversation up to and including the message whose thread row id is `at`, and nothing after it. The old session is not touched: it keeps its process, its slot and its next turn, and gains only the `forked_to` row. The fork is hosted by Bridge as any fork is.

| | |
|---|---|
| Refused | An `at` no message of the thread has, or one the agent's conversation does not hold, 422 `fleet.fork_point_unwritten`. Nothing is written, and the line is looked for before the fork is made |
| Ledger | The `forked_from` row's detail carries `at`, the row Bridge cuts the folded old rows at, and `cut`, the transcript line Fleet gives the agent. The detail is the ledger's, so a Fleet that restarts before the fork's first process still has the cut |
| First process | As above, with `--resume-session-at <cut>`. Only that process is told it; the fork's own file holds the cut after |
| The line | A terminal session's row ids are its transcript lines' `uuid`s (`<uuid>.<n>` for a later row of one line), so the cut is the row id without its suffix. **A hosted session's rows carry no `uuid`**: Fleet minted their ids and the `Said` events the stream is read into drop the line's. Fleet lays the thread's messages over the transcript's from the end, matching the agent's by its words and a person's by being inside the line Fleet wrote (it adds a mention line and attachment paths), and the cut is where `at` lands. A message with no words, or none that lands, is refused: it is not guessed |

The agent side is spike 30. A cut at a message the agent is part way through a tool call on is repaired by the CLI.

**Why a note, and not a cleaner copy.** The CLI copies the conversation, and the conversation says the old session launched subagents, shells and a slot; the CLI then replays a notification for each of them that had not finished ("didn't finish before the previous session ended", "failed", "stopped") into the fork. Stripping the notifications from a copy of the transcript would leave the launches that caused them in the words the fork reads, and the copy would be a file Fleet writes into the CLI's own folder in a format Fleet does not own. The fork is told instead, in the one place it cannot read past. Found 13 Oct 2026: a fork of a live session relaunched the old session's builder into the old session's slot, and showed the owner the old session's windows as its own.

**Bridge** keeps every session that ended in the last seven days (by when it was last seen) beside the live ones, under an Ended heading (a terminal session that has not ended but whose mod is not asking is Quiet, its own heading above it, and still offers Fork), and finds an older one by search, and draws a dead session without a message box and with Fork in its head. It owns nothing: its slots, branches and pull requests are not offered to a chip, and it is not offered to `@`. A press on Fork asks Fleet, and Bridge opens the new session once it holds it. The ledger draws a Forks section only where a session has a row for it.

**A terminal that comes back is live again.** The mod asking again flips `listening`, which Fleet publishes as `session.changed` while a window has the thread open. Fork on a session whose terminal woke between the press and Fleet is refused as live. After a Fleet restart every terminal session reads dead until its mod's next ask, about two seconds.

## A Session an agent started

A Session's agent may start a Session, or fork its own conversation into one, through two tools on its door (23.80). The owner approved it on 12 Oct 2026 on the condition that **the new Session waits for him**.

| | |
|---|---|
| `start_another_session` | `manifest_id`, `title`, `message`. A new Session in any repository Fleet serves |
| `fork_this_session` | `title`, `message`, optional `at`. A copy of the caller's own conversation, to the row `at` or to its latest message of its own that the transcript holds. Allowed while the caller is live, since it is a fork at a message |
| The caller | Placed by the connection of its hosted process, as `show_window` places it. **No call names a Session**, so an agent cannot fork another's conversation, and a call from no hosted Session is refused 422 `fleet.agent_start_unplaced` |
| Not offered | Tune, close, send and every other hosted-session command stay `agent_access` No |

> **Rule.** An agent's Session is created and listed at once, and nothing runs until the owner sends its first message.
> Why: a Session that started itself is a model spending tokens in a repository nobody chose to open. Holding the message makes the start a proposal he reads and may edit.

**The first message is kept as an `unsent` attachment** on the new Session, and read back on every read as a waiting item: id `unsent:<session id>`, text *First message*, source `first_message`, act `message` on the Session. It has its own source and act so the Cockpit draws no reply box for it and a press opens the Session. Bridge puts the words in the new Session's message box once, where the box is empty. His ordinary send (`send_session_message`) sends what the box holds, starts the first turn, and spends the attachment and so the item; editing first is the same call. `answer_waiting` refuses the item, since sending is the answer. Ending the Session discards both. While sleep is on, the item is left on the list and not decided for him.

**Both ledgers record it.** A start writes `started_by` on the new Session and `started_to` on the caller; a fork writes the `forked_from` and `forked_to` of *A forked session*, with `at` and `cut`. The caller's thread gets a `started` row naming the Session and whether it is a fork, which Bridge draws as the press that opens it. The new Session's thread opens under a line naming the Session whose agent started it.

**Nothing bounds how many an agent starts.** Each waits for him, so none spends anything, but a loop would fill the list. A limit waits for one being seen.

## In Bridge

Bridge reads every live session from `list_sessions` once per connection and keeps it by `session.changed`, whole, so a chip anywhere asks the same list who owns its branch, slot, pull request or Job, across every repository. A thread is read with `get_session` when a window first opens it, then followed by `session.row`. The renderer reaches both through the draft seam (`SessionsDraft`): the mock fills it from fixtures and a real window from what main publishes, so a screen cannot tell which.

> **Rule.** A session started in Bridge starts in the repository the window has picked, or the one repository Fleet serves.
> Why: Dispatch names its repository the same way, and a session started somewhere the person is not looking is one they cannot find.

| Bridge draws | From |
|---|---|
| The ledger's slot and branch as one pair | The `slot` and `branch` rows of one repository |
| A pull request, its Checks and its acts | The `pr` row's `detail`; each press is one of the pull request operations above, and `read` brings the row current when a session is opened |
| A Job | The `job` row, read against the Board for its title, number, state and branch. A Job the Board has forgotten is left off. A Job a person attested reads `piloted.exit`, and is marked apart from one that passed |
| The ledger's sections | Only a kind that holds a row is drawn. A ledger with nothing on it draws one small picture and no words |
| Artifacts | The `artifact` rows, one section with a glyph per form (`globe`, `files`, `image`, `notebook-text`) and a tooltip naming it (Published page, File written, Looked at, Doc). A file opens through main, which opens only a path the session's own ledger names as a file it wrote or a picture it looked at |
| The ledger beside the thread | The ledger is its own panel beside the conversation, headed "Ledger" with a button at its trailing edge that hides it. While hidden, the conversation's header holds the button that shows it again; the choice is the window's, kept in its storage. Below the breakpoint the header's one button opens the ledger as a sheet instead |
| A sketch the person drew | The picture it was sent as, and the drawing Bridge kept for the ledger. The wire holds only the picture |
| The context tank | The head reads `context_tokens` and `context_window` from `usage` as a pill of live tokens and the percent, `53.8k · 5%`, filled to that percent and coloured normal below 80, warning from 80, danger from 95, with a faint dotted line at `autocompact_threshold` where the breakdown holds one. Where `context_breakdown` is held it is a button: hover or focus opens a card, a press keeps it open, Esc or a press outside closes it. The card is the Bays view's stop card: a pill `5% used`, the tokens over the window, a slim bar of the categories in the window, one fact a category, then the three heaviest memory files and MCP servers (a server's tools summed). Where none is held it is a figure with a tooltip and no button |
| A running Session's line in the list | `activity.now`, one line under the title. Nothing is drawn for a Session whose turn is not running, however old its last `now` is |
| The thread's Working on band | `activity.todos`, over the message box, in the order doing, to do, done. Folded, a task mark a todo, each named on hover with its words (`TaskMark`'s); a press opens the rows. Left off when the Session says it is done, and when the list is empty |
| A Cockpit tile's line | The todo being done, else `activity.now` |
| A session held for the plan's limit | `held_until` draws `battery-low` in place of the live state mark, on its list row, its Cockpit tile and its header, with the tooltip `Limit reached · resumes 6:40 PM`. Nothing animates. A Job Fleet holds the same way carries it as `paused.until` and draws the same mark beside its status, in place of the pause mark |
| An ask from the phone | A hosted session's held ask is answered through Pocket, with the same `answer_session_ask`. A terminal session shows there as waiting, and while Fleet holds its question the phone answers it the same way; after the hold it is answered in its terminal. [Pocket](pocket.md) |

## Which model, and a fresh start

**A Session's composer names the model it runs on.** Fleet launches `claude` with `--model` only where a person picked one, and the CLI's own default applies otherwise, so the pick alone says nothing. Fleet reads the model off each `init` line of the stream (`DroneEvent::Started`, one per turn) and carries it as `HostedFacts.resolved_model` (23.85): `claude-opus-5-5`, which Bridge names `Opus 5.5`. **It is cleared when the process is let go** and absent before the first turn, because the next process has not said. The composer's Model select reads it: the selected option, the picked alias or the default, is labelled `Opus 5.5` once the resolved model is of that option's family. Until it has, or after another model is picked, the select reads the alias or the default as it did, **never a guess**. The head draws no model. A terminal Session is named only where its mod reported a model.

**Past 300k tokens of context or a day since it started, a quiet mark (`move-right`) sits after the context tank.** Hover or focus opens its card and a press keeps it open, as the context tank's does: `612k context`, `Started 3h ago`, and two acts. **Fork** forks the conversation at its latest row (`start_session` with `fork.at`, 23.79). **New Session** asks the old Session for its handoff, which is a message of the person's sent with `send_session_message`: the goal, where it stands, open pull requests and branches, and what is next. Its next reply, once the turn is over, is the handoff, and the new Session opens under the same title with `Handoff from <title>` and that reply as its first message, sent for the person. While the old Session writes it the mark pulses and New Session waits. **If it ended, the send was refused, or no reply came within 90 seconds**, the first message is the ledger's facts instead: the title, its slot, its branches and its open pull requests. Nothing is typed. Both are Bridge's alone (`session-handoff.ts`), over operations that already exist; the cost is one turn of the old Session, which re-reads its whole context once.

## Stopping a turn

**A Stop sits beside the working mark at the end of a hosted Session's thread**, and the same press is available wherever that mark is drawn with a turn running. It sends `stop_session` (`POST /sessions/stop`, 23.84). Fleet ends the process as it does after a quiet spell, withdraws a question the agent was held on, sets the turn to `idle` and publishes the row. **The row stays live and the slot stays held**: nothing is parked or committed, which is what separates it from `close_session`. The next message resumes the conversation. The thread keeps every row the agent had written, and Stop draws no row of its own.

**A terminal Session has no Stop.** Fleet does not host its process and refuses the act.

**Debug Bridge starts a Session with no turn.** `start_session` writes the row and runs nothing, so Bridge starts one for the note and leaves the note and its picture in the message box as a draft (`annotate/debug-draft.ts`). The first turn is the person's Enter, with whatever he added. `docs/practices/running-locally.md`, *Annotating Bridge*.

## What a message names

A message to a hosted Session can name things with `@` and `#`, sent as `mentions` and kept on its `message` row as `tags`. **Fleet writes the line that tells the agent** (`The person named: ...`), ahead of the words, and the thread keeps the words as typed.

| Tag `kind` | `id` | The agent is told |
|---|---|---|
| `session` | The Session's id | Its title, and its address to write to with `SendMessage` where Fleet hosts it |
| `job` | The Job's id | Its number, title, branch, slot and state, where the tag knows them |
| `pull_request` | The number | `pull request #219 "title"`, and its address where the tag has one |
| `issue` | The number | `issue #214 "title"` and its address. A tag's title begins with `#` and the number, and the line drops that so the number is said once |
| `branch` | The name | The branch |

**`@` offers the other Sessions, then the Jobs, and once letters follow it the repository's files that match them.** A file is written into the words as `@path` and is not a tag: the agent reads the path itself. Pull requests are `#`'s, and no trigger offers a branch, though the kind is still accepted.

**`#` at the start of a word offers the repository's open issues and open pull requests**, matched on the number, or on the key where the tracker names issues by one (`#ops` offers `OPS-7` and `OPS-12`, whatever the case), and reads one number that is neither (a merged pull request, a closed issue) when it is typed whole. The repository is the Session's own. Helm's box offers the same, for the repository its conversation answers for (`helm.md`, *The message box*). `url` on a tag is optional and only the two forge kinds carry it.

## A Session's Checks

**An `armada check` run in a slot a Session holds is the Session's run.** The CLI keeps no record of its own, so it tells Fleet when the run begins and when it ends with the log (`report_session_check`, `POST /sessions/checks`, 23.75). Fleet places the Session by the slot the directory is in, never by what the call says. A directory no Session holds is a person's own run and is not recorded, and neither is a Fleet that is not running.

| Where it shows | What |
|---|---|
| A hosted Session's thread | One `check` row per run: the Check's key, its run id, `running`, `passed` or `failed`, and what it was narrowed to (`target`: the test filter, or `--changed` and the packages the change reached; absent where it ran whole). The row is replaced by id as the run ends, and the Session is published whole each time |
| The Checks page | A row with the `session` requester (`session_id`, `slot`), no Job, drawn with the slot's owner chip. Pressing the thread row opens the run's log over the Session |
| The log | `get_session_check_output`, `GET /sessions/checks/:run/output`, answered as `get_check_output` answers a Job's. Kept up to 256 KiB, the newest end when cut |

A terminal Session's thread is its transcript and carries no `check` row; its runs are on the Checks page. A run whose `armada check` process is killed stays `running` until its Session ends, when Fleet ends it as failed with "the Session ended" as its log.

## A Session's retro in its thread

**A retro being written is a `retro` row on a hosted Session's thread** (23.78). Fleet adds it when it accepts a press, replaces it by id as the writing goes (`stage`, `stage_since`, `characters`, the `items` that have settled) and leaves it when the retro is kept (`retro`, the number `get_session_retro` takes) or has failed (`why`). `docs/concepts/retro.md`, *A Session's retro*, has the stages.

| Where it shows | What |
|---|---|
| A hosted Session's thread | One `retro` row per retro, in the order the presses came. It is a progress report: it holds no retro, and `has_news` and the retro's own record never read it |
| `get_session` | The row as it last stood. One left going by a Fleet that has restarted is returned and kept as `failed`, "Fleet restarted while it was written" |
| A terminal Session | No row. Its thread is its transcript |

## Running a block

A fenced shell block in a Session's thread shows Run when the block is hovered or focused. Pressing it runs the command in the Session's working directory, its slot if it has one, and the output prints under the block as it arrives. A finished run carries a mark with the exit code on its tooltip, and Run again sits where Run was. A run that goes on shows Stop in Run's place, and a stopped run ends with no exit code.

**It is the person's own act.** Nothing is sent to the agent, no row is added to the thread, and the Session's record does not change, with one exception: **a run that exits 0 settles the waiting item whose `run` act names that command**, from the thread or from the Needs you band, where the item is the command's own block with Run. Bridge answers it with `answer_waiting` and `text` `Ran: exit 0`, so the agent is told. Bridge starts the run with `start_session_run` (`POST /sessions/run/start`) and reads what it prints on `observe_session_run`, a socket of its own, until `exited`. A Session that has ended, or a run Fleet does not know, is refused in the same words as any other act on a Session.

The output is not kept past the run. Closing the window, or the Session, ends the reading; a reopened thread shows the block without it.

**A block with no language is a shell block when it reads as one** (`readsAsShell`): every line opens like a command, after an optional `$ `, and none reads as code, JSON, prose or a diagram. A doubt is no Run. In a **terminal** Session, inline code that is exactly a command the terminal lists (`` `/update-armada` ``, `` `/model opus` ``) is pressable instead: it is run in the terminal as typed with `run_session_command` (`POST /sessions/command`), no output comes back, and a check says Sent. A Fleet-hosted Session has no terminal, so the code stays plain. A waiting item whose `run` target is such a command takes the same path: Run in the Needs-you list sends it to the terminal instead of a shell, and a Sent settles the item.

## A terminal session's thread

Open a session a person runs in a terminal and Bridge draws its conversation and a message box. `get_session` reads the transcript file the agent CLI keeps for it (`adapters::terminal_thread`, found by session id under the person's home) and answers rows in the shape a hosted thread has: what the person typed, what the agent said, and one line per tool call. Fleet then watches the file for as long as the session is live and publishes each new line as `session.row`, so the thread follows the terminal. A row's id is the transcript line's own, so a line read twice replaces itself.

A subagent has a thread of its own. `get_session_subagent` reads `<project>/<session id>/subagents/agent-<subagent id>.jsonl`, the file the CLI keeps beside the session's, and answers it in the same rows with whether the subagent `finished` (a turn ended with nothing more to run, a hand-back, or the CLI's interrupt line where its parent's `TaskStop` or the person cut it off) and its `report`, the last thing it said. It reads that one path and no other, for hosted and terminal sessions alike. Bridge reads it again while the subagent runs. **The engine's stop settles it**: the mod's `SubagentStop` hook makes the row `spent` and stamps when, even on a row already spent, because a transcript can end on a line that reads as still working (13 Oct 2026). Where no stop came, a `standing` row is shown `spent` once its transcript shows a turn that ended, and the first `get_session_subagent` that sees it finished settles it. **A parent can send a settled subagent more work**, which is no new spawn and no hook, so Fleet looks every two seconds (`subagents_followed`) at the transcripts of the settled subagents of live sessions, a `stat` each until one grows: one whose last line is work stamped after the row settled is `standing` again, and the timeline enters `Subagent resumed`. Work stamped before the stop opens nothing (14 Oct 2026).

> **Rule.** The thread is drawn and never read as a claim. Nothing Fleet knows about a terminal session, its title, its Job or its state, comes from its rows.

A slash command the person ran is one `command` row, as typed, and the summary the CLI writes where it compacted a long conversation is one `compaction` row, so neither reads as the person's words. What is not drawn: the agent's reasoning, a tool's answer, a command's output, a subagent's own turns, the CLI's bookkeeping (a caveat, a reminder, a task notification) and a message from another session. No wrapper tag the CLI writes for itself reaches a row.

**A message reaches a terminal by its own mod.** Fleet cannot push into a terminal. `send_session_message` to a terminal session holds the text, and the `armada` mod in that session holds an ask for `take_held_messages` open (`wait_ms`, up to 25 seconds, one at a time), which Fleet answers the moment something is held, and submits each text as the person's own prompt, which starts a turn whether the session is idle or busy (spike 27). The ask is also how Fleet knows the session is listening: a send to one with no ask open that has not asked within ten seconds is refused as `fleet.terminal_session_unreachable`, so the person is told at once. A file or picture is saved by Fleet as for a hosted session and sent as `Attached file: <path>` in the text. The mod reports what the terminal runs on (`tuned`: its model, effort, permission mode and the commands it lists) and Bridge draws those in the composer. A model or effort chosen in Bridge is held as a command, and the mod runs it as `/model <x>` or `/effort <x>`; the engine refuses a slash command submitted as text, so it goes through the mods API's command call (spike 27). **The permission mode is shown and not set**: nothing in the mods API switches a live session's, and it is read from the settings-hook inputs at each turn, so it is stale between a change in the terminal and the next turn. End session works on a terminal session too: *Ending a terminal session*, below. The sent text shows in the thread when the transcript has it, which is when the turn starts.

### Ending a terminal session

**End session ends a terminal session everywhere**: the agent's process in the
person's terminal ends, the slot it stood in is parked, and the row ends as a
hosted close does, `end_reason` `closed`. Fleet cannot end a process it does not
host, so `close_session` on a terminal session holds an `end` for its mod, in
the answer to the ask it already holds open (`take_held_messages`). The mod runs
it as `/exit` through the mods API's command call, behind any message held with
it, and the engine runs a command only once the session is idle, so a turn
under way, or one a message just started, finishes first. Measured on 12 Oct
2026 against the harness at 2.1.296: `$.command.run({ command: "exit" })` from a
mod ended an interactive `claude` (exit status 0) and fired `session.end` with
`reason` `prompt_input_exit`.

| Moment | What happens |
|---|---|
| The press, mod listening | The end is held and the press is answered with the row, still live |
| The press, mod not listening | Refused as `fleet.terminal_session_unreachable`. Nothing is held, ended or parked |
| The press, mod older than 0.3.10 | Refused the same way: an older mod would drop the end unread. `/reload-plugins` in the terminal fixes it |
| The mod's next answer | The messages, then `end`. The mod runs `/exit` after them |
| The terminal ends | The mod reports `ended`; the row ends with `end_reason` `closed` |
| The slot | Parked once the process holding its lease has gone, as a hosted close parks (the branch it is on, what still runs in it ended first). The mod reports from inside the exit, so Fleet asks the pool again for up to ten seconds. A holder still running then (a person's own terminal, another session's lease) is left alone; a park the pool refuses for another reason is said on Fleet's log. The ledger's rows were given back when the terminal reported its end, so unlike a hosted close they do not wait on the park |

**A terminal that ends on its own is parked too**, the same way and with the
same wait for its process, and its row keeps the terminal's reason:

| Reason the mod reports | Slot |
|---|---|
| `prompt_input_exit`: `/exit`, ctrl+c, ctrl+d | Parked |
| `other`: the window closed (a hangup), another signal, a `-p` run done | Parked |
| `logout` | Parked, once the process has gone |
| `clear`, `resume` | Kept. The same process goes on under another session id, in the same directory |
| None: a crash or a kill no hook outlives | Kept. The session reads Quiet and its slot is held for a resume |

What Fleet remembers about a press is held in memory only, so a Fleet restarted
between the press and the end ends the row with the terminal's reason, and parks
by the table above.

**A terminal's question is an ask in its thread too.** The `armada` mod hooks `AskUserQuestion` (a Drone skips it) and posts the question once with `ask_from_terminal`, which Fleet keeps in its store as the same `ask` row a hosted session writes, so Bridge draws the same card, and answers with the call's id. The terminal's own prompt opens once Fleet has answered that post, or after a second and a half without an answer, and not at all where Fleet refused the question because a walk is open (*Showing a window*). The mod then polls the same route with `wait`, each poll held about 25 seconds, so no one request has to outlast the question: a poll that fails or times out is asked again, and a Fleet that restarts finds the question where it was. A person answering in Bridge has the answer kept until the mod's next poll collects it, and the mod returns it as the tool's result, which drops the terminal's prompt. A person answering in the terminal first makes the mod send `settled`, which closes the card. A card whose session ended, or whose mod has not polled for three minutes, is closed too, so Bridge never offers one that cannot be answered. Fleet out of reach leaves the terminal's prompt alone.

## Not built

**The Manifest switch is drawn on mock data only.** The chip is read off the wire, but the form's *Pull request owners* switch still runs on `draft.prOwners`, so a real Bridge draws none and the key is set in `armada.yml`. A row is deleted by a Fleet that saw the pull request open and then gone: one that merged while no Fleet was reading keeps its row, and so does a claim that was given back.

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

## Sleep mode

The owner turns on Sleep from the menu in Bridge's title row before bed (`set_sleep`), and the menu button carries the moon while it is on. Sessions and Jobs keep going until only he can unblock them. `get_sleep` reads the switch and the night's rows; turning it on starts a new night and clears the last one's rows.

While it is on, a pass over every live session's waiting items (`crates/fleet/src/sleeping.rs`, riding the pull-request notice tick) works like this:

| Item | What the night does |
|---|---|
| `ask:` or agent item with a "(Recommended)" option, waiting under two minutes | Nothing. The agent carries on by itself first. |
| `ask:` or agent item with an option labelled "(Recommended)", past two minutes | Answers with that option. Recorded under decided, with the option as what it chose and the session it came from. |
| `ask:` or agent item with no such option: a question with other options, an open question, an errand with an act | Nothing, unless it reads as destructive (next row). It stays in the session's Waiting on you list for the owner, and is not recorded under decided or blocked. |
| Destructive or irreversible (delete, drop, remove, force, reset, rm, overwrite, discard, close PR and the like, read from the text and the options; unsure holds) | Never answered. Recorded under blocked. |
| `perm:` permission | Never allowed. Recorded under blocked. |
| `walk:` | Recorded under walks. |
| A pull request the pull watch sees merge | Recorded under landed. |
| A Job that escalates on a stall, silence, loop, failed gate, unanswered ask, or a refused hatch, scope or evidence, with a title that is not destructive | Told to decide for itself, once, with the owner's own act: a redirect to its Drone, or a restart of the step with the same words where the Drone is gone. Recorded under decided. |
| A Job that escalates because its Drone or run ended, was not heard, would not start, was not prepared, or its gate or proposer gave no answer, with a title that is not destructive | Its step restarted once, with no words (an undecided gate is run again instead). Recorded under decided; one with no stopped step to restart is recorded under blocked with the reason; if it escalates again the same night it stays as it is. |
| Any other escalation, or a destructive title | Recorded under blocked. |
| A card a Job's Commander has out to the owner (`commander.md`, *The card*), whether it went out before the night or during it | Never answered, and the Commander is not woken to answer it. Recorded under blocked, once, as *Commander asks, waiting on you*. |

**The review is cards in the Cockpit's Incoming calls**, not a panel of its own: each decided row is a call, headed by who decided it. The card says what Sleep chose and keeps its acts on the rail: **Agree** keeps it (`review_sleep`), **Change** sends the owner's words to the agent (`override_sleep`) and reviews the row, **Open the Session** or **Open the Job** goes to where it was decided, **Later** puts it at the back, and **Dismiss**, under a cover, reviews it for good. A decision made in a Session ends in that Session's own box, which is how it is changed, and `session_id` on the wire row is the Session it opens. A decision made in a Job has no Session, so Change reveals a plain box. A correction Fleet refused leaves the row where it is. Either act marks the row reviewed, and `get_sleep` leaves a reviewed row out of `decided`. **The first use of Bridge after Sleep was turned on turns it off** (`set_sleep` off). Blocked and walk rows are already calls: escalated Jobs and waiting items.

`override_sleep` sends the session `Re: <asked>` and the owner's words, marks the row corrected and reviewed. Every change publishes `sleep.changed` with the night whole.

## A mod that is out of date

A mod loaded before the plugin was updated keeps running the old code until the person runs `/reload-plugins` in that session. **The mod reports its version** in `started` and in the first `tuned` (`MOD_VERSION` in `plugins/armada/hooks/facts.ts`, held equal to `version` in `plugins/armada/.claude-plugin/plugin.json` by a Fleet test, so bump both together), and Fleet keeps it on the session's row. `SessionRecord.mod_out_of_date` is true for a terminal session whose reported version is older than the `version` in the same file in the repository it stands in, or that reported none. Bridge marks it on the row and in the header with "Mod out of date: run /reload-plugins". Fleet reads the repository's file when it builds the record, so the mark moves with the next fact the session reports and not when the file changes.

## Installing the Claude Code mod

```
claude plugin marketplace add "$HOME/Library/Application Support/Armada/mod"
claude plugin install armada@armada-local
```

**Fleet keeps its own copy of the mod** in that folder. `scripts/restart` copies
`plugins/` into it from the tree it builds, so the mod is as new as the Fleet
and a pull on the checkout changes nothing a session loads. After a restart,
`/reload-plugins` in an open session picks the new copy up. The copy is made
beside the folder and renamed over it, so a session never reads half of one.
A tree with no `plugins/` leaves the old copy and warns. Whatever compares a
session's reported mod version with the installed one reads
`Armada/mod/armada/.claude-plugin/plugin.json`.

It loads in a Drone, a Judge call and a scout too, since they read the person's user settings. Fleet starts each with `ARMADA_DRONE=1`; the mod reads it and reports nothing, so only the person's own sessions reach the ledger ([spike 23](../spikes/023-does-a-user-installed-mod-load-in-a-drone.md)).
