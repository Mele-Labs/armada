# A job Fleet hosts

**Part of:** [Job](../session.md).

A person can start a job from Bridge. Fleet runs it, keeps its thread, and
puts it on the same ledger with `origin` `bridge`. It is the person at
work, in their own configuration. [Helm](../helm.md) is the same
engine with a note and a few more tools, one per repository.

> **Rule.** A hosted job holds what the person's terminal holds: their
> servers, plugins, skills and settings, with no `--strict-mcp-config` and no
> `--setting-sources`.
> Why: it is the person's own conversation, and the `armada` mod must load in it
> so it reports through the ledger like any other. A hosted job is one row,
> because Fleet keeps its `origin` when the mod reports its start and ignores the
> mod reporting its end, which is a process stopping.

> **Rule.** Every process of a hosted job carries Armada's job rules in
> its system prompt (`--append-system-prompt`), appended to whatever output style
> the person has. They lead the argument; Helm's note and a fork's or move's note
> follow, and the Logbook note comes last ([Logbook](../logbook.md), *Recall*). Where the person's style and the rules conflict on how a need or
> progress is surfaced, the rules win, and the style keeps tone, length and format.
> Why: a style that asks for every action in a table leaves `waiting_for`, the task
> list and `all_done` uncalled, and Bridge reads needs and progress from those
> alone. The wording is `docs/contracts/agent-prompt.md`, *5f. The job rules*.
> The `Armada` output style in the mod carries the same rules for a person with
> none (*Installing the Claude Code mod*).

> **Rule.** A hosted job starts with no slot and leases one on its first
> write, and one resumed with no slot is leased one before its process starts.
> Why: most conversations never write, and a slot held by a conversation that
> only reads is one a writing Job cannot have. A resumed job has already worked, and
> a resume that waited for its first call to move it lost that call and a turn
> (s-34c7f2c2, three times on 13 Oct 2026).

```
 start ─▶ a row: no process, no slot, no branch (a start with a `request` is read first;
          its process starts when the reading lands, see *A Session's proposal*)
 message ─▶ a live process, `--session-id`, the message on its input
 first write ─▶ a hook asks Fleet; Fleet leases a slot and cuts a branch, writes
                both on the ledger, refuses the write and the turn ends; the
                process is ended and resumed with the slot as its directory
 first write, pool full ─▶ the session is queued, told its place, and stops its
                turn; the owner is asked: wait for a slot, or add one
                wait ─▶ Fleet leases it the next slot to free and resumes it there
                add  ─▶ Fleet makes a slot, leases it to this session ahead of the
                        line and resumes it there
 later writes ─▶ held to the slot
 quiet for a while ─▶ the process is ended; the next message resumes it, leased a
                slot first if it holds none and the line ahead of it is empty
 close ─▶ the slot is parked on its own branch, the row ends
```

**One live process while it is open**, as Helm's now is (it was one per message), because
another job's message wakes an idle one: `SendMessage` to a hosted job's
address, `s-` and the first eight characters of its id, starts a turn in it. A
turn that opens with no message of the person's outstanding is a wake, and the
thread says whose. **A job whose process was ended is resumed by the
message**, written for the sender, which is a hosted job Fleet can read the
send of. **A paused job is not woken by it**: the message is kept and read after the person resumes it (*Pausing a Job*). `settings.session-quiet-timeout` is how long a process may sit idle.
[Spike 26](../../spikes/026-does-a-message-from-another-session-wake-a-live-process.md).

**The lease is taken by a hook, and nothing writes the main checkout before it.**
A write tool, or a shell line that is not a read, is refused until the job
holds a slot; after it, a file tool naming a path in the main checkout is
refused with the slot's path. A shell line is read by its words, so one Fleet
does not recognise as a read leases first. `|`, `;` and `&` inside quotes do not cut
a line, and a redirect to `/dev/null` or onto another descriptor (`2>&1`) writes nothing;
any other `>`, a `$(` or a backtick, or a quote left open is a write. `sed -n` with an
address and `p` (`sed -n 10,40p file`) reads; every other `sed` is a write.
[Spike 24](../../spikes/024-how-does-a-session-move-into-a-slot-at-its-first-write.md).

**A full pool queues the job, and asks the owner what to do about it.** Every
slot held, the write is refused with the job's place in its repository's
line, the news that the owner is being asked, and the instruction to stop for the
turn. The line is in Fleet's memory, first come first served, and a job is in
it once; a job behind others is not leased a slot a moment before the one
ahead of it. **The owner's call is `slot_choice`**, a derived waiting item with
id `slot-choice`, text *Every slot is in use* and two options, *Wait for a slot*
and *Add a slot*, raised when the job joins the line and gone when he answers
or the job leaves. A job resumed onto a full pool raises it at once, before
any write. **Wait** leaves the job where it is in the line: when a slot is given
back, Fleet leases one for the head of the line, writes the lease as it would for a
first write and moves the job into it, whether or not its refused turn has ended:
an idle job is resumed at once, one still finishing the turn is moved as that turn
ends. **Add** is `Fleet::change_slot_pool(Add)` for the repository and a lease of the
new slot to **that** job, not to the head of the line; the grant is held back
while it does, so the head cannot take the new slot first. A slot freed before he
answers goes to the head as it always did, and the call goes with it. A slot is given
back by a job closing or its
terminal ending, a release or rescue from
Cleanup (a release of a bay a hosted Job holds is refused while its turn runs, and otherwise gives back its slot and branch rows), and the pool growing or a slot reopening; a slot freed where Fleet is
not told, as `armada worktree release` frees one, is found by a look every 15
seconds while anyone waits. A job that ends, or is leased a slot another way,
leaves the line, and a Fleet that restarts forgets it: the job's next write
joins again and he is asked again. A terminal job is not queued, because
`armada worktree lease` waits for a slot itself.

**`armada worktree lease` run by a hosted Job asks the owner the same question.**
Finding every slot held, the command tells Fleet (`ask_for_slot`, naming the Job by the
`ARMADA_SESSION` Fleet set in its process) and again about once a second, and says on stderr
what the owner chose. It raises the Job's one `slot_choice` call, or shares the one a
queued write already raised: never two. **Wait** leaves the command looking for a free slot, as it
always did. **Add** makes a slot and holds it for the command until it leases it or about
thirty seconds pass; the grant to the line leaves that repository alone meanwhile, and a write
that would take the slot straight off the pool queues instead. Where the Job is also in
the line, the new slot goes to the Job and the command keeps waiting. Fleet forgets a command
it has not heard from for ten seconds, taking the call with it, so one that died leaves nothing
on the owner's list. A command with no `ARMADA_SESSION`, or no Fleet to ask, waits as it always did.
Nothing moves the Job: the command takes the slot itself.

**What the write gate covers.** A slot lives under the main checkout's path, so
the gate tells three places apart: the main checkout, a slot, and anywhere
else. **A job writes in a slot it leased, or one something it started leased
(its subagents), and in no other** (the owner, 13 Oct 2026). A write by any other
job, a fork included, into a slot it never leased is refused with a sentence
saying so; a fork owns nothing of the job it came from, whatever its copy of the
conversation says. Who holds a slot is read from the lease record beside it, at every
call. A slot leased with `armada worktree lease` from a hosted job, the
agent's or a subagent's, is held for the job by its id (Fleet sets
`ARMADA_SESSION` in the process it starts and a subagent inherits it),
so it survives Fleet ending and resuming the process. One leased with no job id
in the environment is held by the first process above the command that is not a
shell, and is the job's while that process is in its tree. A slot held by
anything else, or by a holder that has gone, is refused. The main checkout stays
refused, and a `..` is resolved by reading before the path is placed.

**Setup's job is the one exception** (the owner, 13 Oct 2026). Started by `root` in a
repository with no Manifest (no `armada.yml` and no Kit copy), it has no pool to lease from, so until that root's
Manifest is served it writes only its setup files and is refused anything else: a file
named `armada.yml` or `.gitignore` at any depth under the root, one for each workspace of
a monorepo, and `<root>/.git/info/exclude` (the owner, 14 Oct 2026). A path is placed
after its `..` is resolved, so one that climbs out of the root is refused, and nothing
else under `.git` is a setup file. Fleet loads the Manifest when one of its turns ends,
and from its next write the job leases a slot like any other, **save its setup
files, which still land in the checkout**: an ignored file written on a slot branch
would never reach it. The hosting row's `setup` marks the job as Setup's, so the
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
a job's calls**: a line that goes into the main checkout or a slot the job does
not hold, and a line that goes nowhere while the call already stands in a slot it does not
hold, is held if it then runs anything that is not a read. `cd <slot> && git status` is a
read and runs. **A relative `cd` after another is read from where the first left the line**,
and so is a `..` path after it, never from where the call began. Every part of a line is told
read from write before the slot rule applies, and `git worktree list`, `git stash list`,
`git config --get` and the other listing forms of a subcommand are reads. **Not covered**: a path or a `cd` target built from a substitution or a variable the line did
not set (one it set, `out=/tmp/x; : > $out`, is read as the path it holds), a file a script opens by a name the line does not spell, and anything a build
writes by itself. [Spike 29](../../spikes/029-why-a-sessions-auto-asked-and-how-a-question-is-answered.md).

**The permission door reads the same ledger.** In `auto` it refuses a call that removes or
overwrites something outside the slots the job holds and `/tmp`: **every** slot the
job holds, its own and each one a subagent leased, and not only the one it runs in. The
door decides whether a call is one of the owner's three classes; whether a slot may be
written at all is the gate's. **What the job started or made is its own to remove**
(the owner, retro of job 9c74d82e): `kill` of pids in its own process tree, or running a file from a slot it holds (a dev Fleet
its subagent started leaves the tree by `setsid`), and of
nothing else (`kill -9 -1` and `pkill` stay refused), `git branch -d`/`-D` of its leased
branch and the branches of slots it holds, once the base holds every commit on them, and
`rm` under `$TMPDIR` as under `/tmp`.

**In `auto` a job waits on the person only for a call past its slots** (the owner, 16 Oct
2026, reversing 8 Oct's "never waits" for these calls). Inside its slots and `/tmp` it runs,
rewrites its own branch, pushes and reaches off the machine unasked. **A read anywhere runs**
(16 Oct 2026), except of a folder macOS puts a prompt in front of (Desktop, Documents, Downloads,
Pictures, Movies, Music, iCloud, Mail and the like), his credentials (`~/.ssh`, `~/.aws`,
`~/.config/gh`), another volume, or a search from `/`, `/Users` or his home
(`fleet::helm::deciding::GUARDED`). Such a read, a write into his own files, and a `Write` or
`Edit` anywhere outside its slots and `/tmp` are put to him on a card in the job's thread.
**The card says whether the call reads or writes** (the owner, 16 Oct 2026): "Permission to read"
or "Permission to write", and "Always applies to reads of" or "writes to" over the path. It offers
Allow once, Reject once, Always allow, Always allow for this job and Always block, the last
three over the path the guard named first, drawn as its folders so a press on `Documents` chooses
everything under it. **The folder list answers before a card is drawn** (`features.reachFolders`
in settings.json, shown and edited in Settings, Features): a path in a blocked folder is refused
with no card, one in an allowed folder runs, and the deeper entry wins, a block winning a tie.
**Always allow for this job lets only that job in**, until it is closed: Fleet holds the
folder in memory beside the job, writes it nowhere, and Settings does not list it. It is read
after his list, so a folder he blocked stays blocked, and a Fleet restart drops it. Nobody answering inside the hold is
a refusal, so a job never hangs while he is away. **A removal is not a reach**: `rm`, `mv`,
`chmod`, `chown` or a truncating redirect outside the slots and `/tmp` is refused with a reason the
agent reads and works around, whatever folder he allowed, and `sudo` and `kill -9 -1` stay refused.
`launchctl print`, `diskutil list`, an awk script's `~` and `$5`, jq's `//`, and a read of another
job's slot all run. `chmod` and `chown` in a held slot are the job's work as `rm` is, and a
redirect into `/tmp` runs however `$$` or a variable the line set spells it. Writing another
job's slot stays refused. A question (`AskUserQuestion`) is put to him in every mode; `ask` and
`acceptEdits` still put every call to him.

**The slot is held by the job's id**, so a
restart of the process or of Fleet is not the job ending. **So is every
slot the job leases itself**: `armada worktree lease` run from a hosted
job reads `ARMADA_SESSION` and holds by that id, and takes a slot
of its own each call. Before this a lease was held for the agent's process,
which Fleet ends when it moves the job into a slot, so the lease read as
gone and the next caller took it. It is given back on close by parking every
slot held under the id: what is in each is committed and nothing is pushed.

**It is committed to the branch the slot is on, not the one it was leased on.** A
job rewrites its own branch inside its slot, so it may have moved to another.
On a detached head or on the base it is committed to a new `kept/slot-<n>-<id>`
branch cut at `HEAD`, and a clean slot holding nothing a detach would lose is
only released. **What the job left
running in the slot ends first**: every process whose working directory is the
slot or inside it (a backgrounded `pnpm mock`, a Check's process, a subagent's
agent), found by working directory and never outside the slot. This process and
its ancestors are spared. Fleet holds no pid for a Check run its CLI makes, so
that is how a run's process ends; the run itself is marked failed. A server
Fleet holds for the slot's checkout is stopped, and the job's open walk is
ended, which Bridge reads as its window closing. **A park the pool refuses
leaves the ledger's slot and branch rows standing**, so `who_owns` still names the
job as holding the slot, since a row given back over a held slot is how one is
stranded with nothing saying so. Fleet parks it again at boot, and again when a
repository is added, for any slot the pool holds under the id of a job that
has ended.

**The person sets the mode, the model and the effort** for each job. `auto`
is the default and means what it meant for Helm: everything runs except what is
destructive, pushes code to a shared space or writes off this machine, which
Fleet puts to the person on the job's own thread. The CLI's own auto mode is
not reachable for a spawned job, so `auto` is `default` with that door.
Inside the job's own slot, what it writes is its work and does not ask, and a
redirect into `/dev/null` is not a truncation; `rm`, `git reset --hard`, `git push` and the like
still do. `ask` and `acceptEdits` put every call the person's settings do not cover.
[Spike 25](../../spikes/025-is-auto-mode-reachable-in-a-spawned-session-now.md).
A change applies from the next process: an idle one is ended at once, a running
one when its turn is over.

**A question is an ask too.** The agent's `AskUserQuestion` reaches the same door as any call, and
in a hosted job nobody could answer it. Fleet now holds it whatever the mode and the thread
draws it as a form: each question with its options, a multi-select where the agent said so, and
*Other* for the person's own words. *Answer* hands back one entry per question, *Skip* tells the
agent so. Fleet returns the tool's input with its `answers` map filled in, keyed by question text,
a multi-select's labels joined with `, `. [Spike 29](../../spikes/029-why-a-sessions-auto-asked-and-how-a-question-is-answered.md).

| Operation | Does |
|---|---|
| `start_session` | A row for one repository, with optional title, model, effort and mode; `held` keeps it held (*A held Job*) |
| `begin_session` | Start pressed on a held Job: keeps how the work lands, lifts the hold, and sends the first message as he left it in the box, with the turns that waited |
| `rename_session` | A person's name for a Job, hosted or in a terminal; stands until the next terminal `/rename` |
| `name_session` | Armada's name for a hosted Job from its first message, asked of the proposer on request; not kept as his rename |
| `start_session_run` | A shell block run from a Job's thread, on a terminal of its own, in its slot worktree or directory; output is held in memory only |
| `stop_session_run` | Ends a run's process group |
| `write_session_run` | Keys typed into a run's terminal |
| `resize_session_run` | The run's terminal changed size |
| `observe_session_run` | A run's terminal output as it arrives, then how it ended, on a socket of its own |
| `send_session_message` | Text, pictures and files stored by Fleet, and the jobs, pull requests and branches it names; takes a turn |
| `answer_session_ask` | One of the offers on the ask the agent is held on, with an answer for each question where it asked some |
| `tune_session` | Model, effort and mode |
| `close_session` | Ends the process, parks every slot held for the job and ends what ran in it, ends the row and its walk. For a terminal job, the end is held for its mod (*Ending a terminal job*) |
| `stop_session` | Ends the turn that is running and leaves the row live (*Stopping a turn*) |
| `pause_session`, `resume_session` | Parks the Job's work on its branch and gives its slot back, and takes a slot again (*Pausing a Job*) |
| `set_session_auto_land` | Turns Auto land on or off for the Job, and the forge's auto-merge follows it (`crates/ipc/operations/set_session_auto_land.toml`) |
| `get_session`, `get_session_file` | The row with its thread, and what a message carried |
| `session.row` | One row of the thread, appended or replaced by its id |

**What a message names is one line Fleet adds for the agent** and the thread
keeps the person's words. A thread's rows are the person's messages, the agent's,
another job's, one line for each tool call, the lease, and each ask with
where it stands. A Helm Job's thread also has the `sent` row, the person's
words routed on to another Job, with where they went. **An ask is
the job's own**: it never appears in the old Helm dock's holds and
`answer_helm_call` does not find it. Helm's own questions are asks of its
Job, answered in its thread.

## What a restart does to a job

**A hosted job survives a Fleet restart, and so a Bridge restart.** Its agent
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
A Fleet that attaches mid-turn is told so, and the job shows as working.

**The wire** is lines of text over the socket. Fleet sends `ATTACH`,
`IN <line>`, `ACK <offset>` and `END`. The keeper sends `READY <pid> <busy>`,
`OUT <offset> <line>` and `GONE`. One client at a time; the newest attach wins.

**A keeper ends** when the agent exits and Fleet has been told (`GONE`), on
`END` (closing a job, the quiet timeout, a tune), or when no Fleet has been
attached for 30 minutes, which also ends the agent: a keeper no Fleet will
return to must not run one for ever. The permission door (`armada mcp`) waits up
to 90 seconds for a Fleet that is restarting before it fails a call. It reads
Fleet's port once, so a Fleet that comes back on another port is not found; the
port is claimed from the store and is normally the same.

**`END` takes the keeper's socket and spool away at once**, from Fleet's side,
so the next message starts a new keeper instead of reattaching to an agent that
is going. The keeper's own stderr, the agent's and how the agent exited go to
`<id>.log` beside the spool; Fleet puts its last lines in the thread's
"process ended" note, a message in Fleet's voice.

**A process Fleet replaces takes its subagents with it.** A subagent runs inside
its agent's process, so the keeper only carries it across a Fleet restart. A tune,
a quiet spell, the move into a slot at the first write and a lost keeper each end
the process, and the one that resumes has never heard of the subagents that were
working. When Fleet starts a process for a job that has run, every `subagent`
row still `standing` whose transcript shows no finished turn was cut off. Fleet
gives those rows back, keeps their type, description and id in the restart's
note (so the retro lists them), and puts the same list ahead of the agent's next
turn, once, so it can start again what it still needs. **Background tasks go the same way**: a `background` row still `standing` when the new process starts is given back, and is in the same list.

**Not covered**: a keeper's own death (the agent loses its pipes and exits), and
an agent whose hooks name a Fleet port that changed.

## Background tasks

**A hosted Job's ledger shows what its agent runs beside its turn, and Fleet detects it.** A `Bash` call run in the background and a `Monitor` watch each become one `background` row, read off the Job's own stream, so no tool call of the agent's is involved (the owner, 14 Oct 2026: Fleet detects state and does not rely on a tool call).
The call is remembered when it is heard; the harness's `task_started` line names the task, the call that asked for it and whether it runs beside the turn, which is true for a call a hook moved to the background as it is for one that asked.
A task whose call was not a shell command or a watch (a subagent) has no row, a command that ran in the foreground has none, and an end for an id that never started here settles nothing.
A subagent's backgrounded command is the Job's row, heard the same way.

| Line | Gives the row |
|---|---|
| `task_started` | `target` (the task id), `detail.task` (its description), and from the call, `detail.form` and `detail.command` (the whole command) |
| `task_updated` with a final status | `spent`, and `detail.status`: `completed`, `failed` or `killed` |
| `task_notification` | `detail.output` (the file its output went to), `detail.exit`, and the same `status` (`stopped` is `killed`) |

The two ending lines each fill in what they have, in either order.
**The exit code is the one fact read out of prose:** no key carries it, so only the end of the notice's `summary` is read, in the two forms the harness writes (`completed (exit code 0)`, `failed with exit code 3`).
A summary in neither form has no `exit`.
`detail.output` is only known once the task ends.
A terminal Job's `background` rows are not kept: its mod does not report them.
