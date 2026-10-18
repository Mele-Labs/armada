# Job

**What it is:** A Job is an agent a person runs, in a terminal or in Bridge, kept as one row on a ledger so Fleet can say which Jobs are open and which one holds a branch, a pull request or a slot.

---

**Kind:** Record. This concept was called Session until 17 Oct 2026; the code, the wire, the store and this file's name keep `session`.

You have five terminal jobs open across three repositories. One is in slot 3, one opened pull request 1853, and one started a second Job an hour ago. You ask who has `fleet/session-ledger`, and the answer is a job by its title, still running, with the slot it stands in.

## What it is

> **Rule.** A job from a terminal is a first-class row, the same shape a job Bridge hosts will have.
> Why: the question is who holds a thing, and it should not depend on where the job was started.

> **Rule.** A job is reported as it happens and is never read back from.
> Why: the third attempt in [scope](../scope.md) was a conversation read as a claim. The ledger keeps what a job took and did, as structure, and nothing downstream reads a transcript.

> **Rule.** Fleet never waits on a job and a terminal job never waits on Fleet.
> Why: a terminal job is the person's own, and Fleet being down, slow or gone must cost it nothing. A job Fleet hosts is Fleet's to run, so its write waits on Fleet's hook, and what the CLI does with a hook Fleet does not answer was not measured.

Until the ledger Fleet could not see a job a person started. [Helm](helm.md) is one of these now: Armada's own conversation with Fleet is a hosted job per repository, marked `helm`.

## How a job gets in

The harness sends Fleet facts in Armada's own shape and nothing past the adapter that sends them knows what the job runs in.

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
| `moved` | `cwd` | Re-resolves; a slot the job left is given back. A slot whose lease is another Job's is not taken |
| `attached` | `attachment`: `kind`, `target`, `detail?` | Takes it; `branch` and `slot` replace the one held. A `branch` or `pr` read from a slot whose lease is another Job's is dropped (*Whose slot it is*) |
| `settled` | `attachment`: `kind`, `target`, `state` | Moves it to `spent` or `given_back` |
| `measured` | `usage`: `context_tokens?`, `context_window?`, `cost_micros?`; `plan?`: the plan's windows; `context_breakdown?`: what fills the window | Keeps what was reported. The breakdown is kept whole in `session_context`, one row a job, and replaced by the next; a report without one leaves it. The windows are the account's, so Fleet keeps the newest from any job as `get_plan_usage` (`machine.md`, *Quota*) and keeps nothing of them on the job |
| `tuned` | `model?`, `effort?`, `mode?`, `commands?`, `mod_version?` | Held in memory as the terminal's `terminal` facts: model, effort, mode and the commands it lists |
| `turn_started` | nothing | A prompt went in (the mod, never for a slash command, which most of the time ends without a turn): the job is working, and the reply the turn before left is over (*What a Job is doing*) |
| `turn_completed` | `said?`: the first line of what the turn ended on | Stamps the last turn, lets go a hold for the plan's limit, and keeps `said` as the reply (*What a Job is doing*). Marks every window the job showed ready, unless a `subagent` of it still stands (*Showing a window*). The mod does not tell a turn that died on an API error |
| `limited` | `resets_at` | Holds the job until the plan's limit resets (*A job that hit the plan's limit*) |
| `wrote` | nothing | The job itself wrote a file (the mod, never for a subagent): marks every window it showed as still changing |
| `permission_asked` | `tool`, `detail?` | A terminal's harness opened its own permission prompt (the mod, from Claude Code's `PermissionRequest` hook): a `permission` call until it closes (*A terminal's permission prompt*) |
| `permission_settled` | nothing | The tool call that raised the prompt ended: the call goes |
| `touched` | `paths`: files written, from the job's directory | Checks them against who else writes them (*Write clashes*); a terminal Job's are counted for the timeline as `session.edited` |
| `read` | `path`: a file read, from the job's directory | Counted for the timeline as `session.read`, for a terminal Job; nothing on the ledger changes |
| `ran` | `command`: the first line of a shell command, 300 characters at most | Counted for the timeline as `session.ran`, cut and with what looks like a secret in it hidden; nothing on the ledger changes. A command whose first word is `gh` is entered as `session.called` instead, as its subcommand words only |
| `tasks` | `tasks`: the agent's whole list, each with `id`, `text` and `status` (`pending`, `in_progress` or `completed`) | Keeps the list and when each item was first seen completed (*The build record*). An empty list clears it |
| `plan_left` | nothing | The agent left plan mode (*The build record*) |
| `committed` | `branch?` | A `git commit` printed a commit made. Entered on the timeline as `session.committed` |
| `pushed` | `branch?` | A `git push` printed a ref updated. Entered on the timeline as `session.pushed` |
| `ended` | `reason` | Ends it and gives back everything it still holds. A terminal's slot is parked unless the reason is `clear` or `resume` (*Ending a terminal job*) |

**No instant is reported**: Fleet stamps each fact on arrival. **A repeated fact publishes nothing**, so a harness may report on every turn.

The mod for Claude Code is `plugins/armada/`, and `crates/adapters` is the only Rust that names the harness. It sends no message text and no prompt apart from the first line of real text in the first, as a title, the first line of what a turn ended on, as the reply (*What a Job is doing*), and the first line of each shell command, for the timeline (`docs/practices/protocol.md`, *The Fleet's timeline*): the harness's wrapper tags (`<agent-message …>`, `<command-…>`, `<system-reminder>`) are taken off first, and a prompt with nothing left has no title. A `/rename` in the terminal is reported as the name typed, or, for a bare `/rename`, the name Claude Code wrote to the transcript. **A Job's name can also be changed from Bridge**, by pressing its title in the Job view: `rename_session`, hosted or terminal, which stands until the next terminal `/rename`. A hosted Job's harness is not told, so its own name for itself stays what it was. **The mod also sends the text of each item on the task list the agent wrote**, read from the task tools it already calls (*The build record*).

## What is kept

Two tables, in `crates/store`. The second is not the first's child.

| Table | One row is | Key |
|---|---|---|
| `sessions` | A job: harness, origin, repository, directory, title, `live` or `ended`, when it started, was last seen and last finished a turn, the figures it last reported | `id`, the harness's own |
| `ledger_attachments` | One thing a holder took or did | `holder_kind`, `holder_id`, `kind`, `manifest_id`, `target` |

A holder is `{ kind: session, id }` and points at neither table, so another kind of holder can hold rows without a table of its own. A `kind` is open text:

| `kind` | `target` | Held until |
|---|---|---|
| `slot` | the slot's number | the job leaves it, the pool shows it held by another or free, or the job ends |
| `branch` | the branch's name | the job checks out another, its slot is given back, or it ends |
| `pr` | the pull request's number | it merges (`spent`) or closes (`given_back`) |
| `subagent` | the agent's id | it stops (`spent`) or its process is cut off (`given_back`); more work after either holds it again, *A terminal job's thread* |
| `background` | the task id the harness gave a shell command the agent ran beside its turn (`run_in_background`) or a `Monitor` watch; `detail.form` is `command` or `monitor`, `task` its description, `command` the command, and once it ends `status` (`completed`, `failed` or `killed`), `output` (the file its output went to) and `exit` (the code, for a command that exited on its own) | the task ends (`spent`, with `status`), or the process that ran it is replaced (`given_back`). Several stand at once, and a hosted Job's only; *Background tasks* |
| `message` | `to:<who>` or `from:<who>`, with a count in `detail` | |
| `studio` | the Studio's id | not reported by anything yet |
| `forked_to`, `forked_from` | the other job's id | `spent` when written, so the old job ending does not give the link back. A move's pair carries `detail.moved`, *A moved job* |
| `started_to`, `started_by` | the other job's id | `spent` when written, as a fork's links are. The caller holds `started_to` and the Job its agent started holds `started_by` |
| `unsent` | the words of the first message another job's agent wrote | `standing` until the person's next send, which spends it; ending the job gives it back |
| `artifact` | a page's or document's address, or a file's absolute path; `detail.form` is `page`, `file`, `image`, `doc` or `window`, `detail.title` its name | nothing: it stays on the ledger after the job ends |

**An artifact is something a person would open, and a code edit is never one.** Edits are Branches and Pull requests. Five forms:

| Form | What counts | The press opens |
|---|---|---|
| `page` | A page published with Claude Code's `Artifact` tool (a publish, not a list, read or asset upload) | the address, in the browser |
| `file` | A file the `Write` tool **created** with a document extension: `md`, `mdx`, `txt`, `pdf`, `png`, `jpg`, `jpeg`, `gif`, `webp`, `svg`, `csv`, `doc`, `docx`, `xls`, `xlsx`, `ppt`, `pptx`. Not under `node_modules`, `.git`, `.claude`, `.armada`, `target`, `dist`, `build` or a temporary directory. `html` is left out, because a page's source appears as the page once published; configuration (`json`, `yaml`, `toml`) is not a document | the file, on this machine |
| `image` | A picture the job **looked at**: a `Read` of a `png`, `jpg`, `jpeg`, `gif` or `webp`, in any directory (a screenshot is usually in `/tmp`). The same path twice is one row, and a picture it wrote stays `file` | the file, on this machine |
| `doc` | A Claude Docs document made or edited (`mcp__claude_ai_Claude_Docs__create`, `batch`, `update`), by the address its answer carries or the document's id | the address, in the browser |
| `window` | A page the job showed with `show_window` | the address, in Bridge's own window |
| `sketch` | A diagram the job drew with `show_sketch`; `target` is `sketch:<title>` and `detail.by` is `agent` | the sketch pad |

The terminal mod tells the first five (`plugins/armada/hooks/facts.ts`, `artifactOf`). **A hosted job tells files and pictures looked at**, because its stream carries a `Write` call but not its result, so a published page's address and a Docs link are not seen; and it cannot tell a new file from a rewrite, so a rewritten document counts. The extension lists are the same rule written twice, in `facts.ts` and `adapters::written_document` / `adapters::viewed_image`. Files made by other tools, such as a screenshot a command saved, are not seen.

### Whose slot it is

**The lease in the pool says which Job works a slot. A ledger row is a claim, believed while the pool agrees.** A harness reports the directory a Job is in, and a slot it left stays in that directory for whoever leases it next, so the directory says where a Job was and the lease says who works there. `armada worktree lease` records the agent job it ran under beside the holder; a hosted Job holds by its own id; a lease taken by hand, or before leases named a job, names none.

| What the pool says of a slot, for a Job | Fleet |
|---|---|
| The lease is written as the Job's | Credits the `slot`, its checked-out `branch`, and what is read from it |
| The lease names no live Job: a person's own terminal, an older lease, a Job that ended (`/clear` starts a new id in the same terminal) | Credits it, as before the lease named anyone |
| Held for another live Job, or free | Credits no `slot` and drops a `branch` or `pr` read from it |
| Any of these, for a `pr` the mod reports with `created` in its `detail` | Credits it. The mod read its address off the job's own `gh pr create`, so no directory is being trusted, and a job that leased a second slot and left its first still owns what it opened |

**A release is settled by the pull sweep, for hosted and terminal Jobs alike.** Each interval Fleet gives back the `slot` and `branch` rows of a live Job whose lease is no longer its own, by whatever road the slot went: its own `armada worktree release`, Bridge's release, a lease taken back from a gone holder. A hosted Job is also followed at its next tool call. **A `pr` row stays**: the Job that opened a pull request is still the one to tell after it leaves the slot, and the row settles when the pull request merges or closes.

**A slot whose branch merged is offered to its holder before Fleet takes it back (the owner, 16 Oct 2026).** Jobs kept slots long after their work landed, and a full pool left others with none. When the pull request from the branch a slot is leased on merges, Fleet tells the Job the lease names, whatever the pull request's owner was told: `slot-3's branch X merged`, to lease it for new work or give it back. The Job may want the slot for follow-up work in its plan, so the notice comes first. A hosted Job's next turn that ends with nothing queued is then the moment Fleet gives the slot back, if HEAD is still that branch or a commit the base has, nothing is uncommitted and every commit is on a remote or the base. A new branch cut there is new work and the slot stays. Uncommitted files or commits on no remote keep it, and the Job is told once for each what keeps it. The branch is kept. `docs/concepts/fleet.md`, *Worktree slots*.

**The ledger draws every slot the Job holds.** It keeps one `slot` row, the slot the Job works in, so a slot its subagent leased has no row of its own. Its record is drawn with a `slot` for each other slot whose lease is the Job's, by Fleet's lease or a subagent's `armada worktree lease` naming it (`crate::slot_holders::slots_of`). A terminal Job that moved out of a slot gave it up though its shell still holds the lease, so a slot the ledger gave back for one is not drawn again. The owner, 15 Oct 2026.

**One reading answers who holds a branch or a pull request.** A Job is confirmed on a branch when it is live, its `slot` row stands, the lease there is its own or names nobody live, the worktree exists, its `branch` row stands and that branch is checked out. The owner of a pull request, the Jobs told which branches a build holds, the write-clash table and the attaching of a pull request nobody reported all ask it (`crate::slot_holders`); the write-clash table adds every other slot the Job holds. Where no Job is confirmed, the one whose standing row is oldest stands, and a branch a Job gave back is nobody's. A merge is attributed to whoever held the pull request or its branch in any state, since the question there is who made it.

**State is one vocabulary for every kind.** `standing` is in force, `spent` is used and done with, and `given_back` was let go before it was used. A released slot and a spent need read the same way.

### Releasing what a Job holds

**The person gives back everything a Job holds, or hands it to another, with `release_session_holds`** (`session_id`, optional `to_session_id`), from Release on the Job's flight controls ([Cockpit](cockpit.md)). The owner, 16 Oct 2026: "Give me a button in the flight control of a session that is gated by a confirmation dialog. This button will let me force release everything in the ledger of that session. Agents sometimes can't do this but from Bridge we should be able to make it happen. It would be cool if I could select another session to transfer what it holds to." It acts on the `standing` `slot`, `branch` and `pr` rows named in `rows`, each a `kind` and `target`, and on every one where `rows` is absent, and leaves the rest. **Bridge draws each as a checkbox, all ticked when the confirmation opens, and sends the ones ticked.** The owner, 16 Oct 2026: "If we're building this, I should be able to check what I want to release. By default everything is checked." A row the Job does not hold standing is refused and nothing moves. A slot takes the branch it has checked out with it. Where a slot is among the rows, a hosted Job's running turn is stopped first, as `stop_session` stops it, and the Job stays live. With no target each row is given back, and each slot the pool holds for the Job is released as Bridge's slot Release does it: its uncommitted files are committed to its branch, then the slot is freed. With a target each row is given back and written standing for it, `detail.handed` reading `session <id>`, and each slot's lease moves to it as it stands, held by its id as a hosted Job holds its own. A target that is the Job itself, has ended or works in another repository is refused and nothing moves. Nothing is deleted. A slot the pool would not move stays with its branch, and the answer names each that stayed and why. `agent_access` is `No`: it is the person's act. **Each held row of the Job's ledger also carries its own Release**, shown on hover or focus, which confirms the one row and sends it alone with no target; the row then reads given back, or says why Fleet kept it. The owner, 16 Oct 2026: "I should also be able to release items from the ledger panel one by one."

### Which machine owned a pull request

**A registry names the Job that claimed a pull request, so a second machine under the same GitHub account can tell who had it.** It is a small central store, set up by [signing in](account.md), and Fleet writes a row when a Job claims a pull request. Nothing is written to the pull request or the repository, so a label, a comment and a body trailer are all refused.

| Field | Holds |
|---|---|
| key | The sha256, in hex, of `owner/name#number`, the name lowercased so every machine makes the same one |
| kind | `session` |
| id | The holder's short id |
| machine | The machine's name |
| since | When the claim was made |

**A row never holds a title, a repository name or code.** The machine that owns the pull request draws its title from its own ledger, and any other machine draws only the stored facts. A repository registers nothing until its Manifest turns it on, and the switch is off by default. With it off, the repository registers none and shows no owners at all, those another machine registered included.

| When | Fleet |
|---|---|
| A Job claims a pull request (`claim_pull_request`) | Writes the row |
| The reading of a repository's open pull requests | Asks for the rows of all of them in one call, and puts each owner on the merge line's pull request and on the panel's page as `owner` (kind, id, machine, `this_machine`) |
| A pull request is in the last reading and not in this one | Deletes its row |

**None of it can fail the claim or the delivery.** A store that cannot be reached, a repository bound to no profile (or to one signed out) and a switch that is off leave the row unwritten and the field absent, and a claim is made all the same. The panel's page carries what the last reading found and makes no call of its own.

**Bridge draws a row as a chip.** The `square-terminal` glyph and the Job's short id in mono lead, and the machine's mark follows: `laptop` for this machine, `monitor` for another, its tooltip the machine's name. A pull request with no row draws no chip, in the merge line's open pull requests and in the Pull request panel. The chip is drawn from the wire's `owner`. The switch is *Pull request owners* in the Policy section of the Manifest form.

## What Fleet answers

| Operation | Answers |
|---|---|
| `list_sessions` | Every job, most recently seen first, narrowed by repository, `live` or `ended`, and `q`: a title, a branch, a pull request number or a slot |
| `who_owns` | Every holder of a branch, a pull request or a slot, the ones still holding it first. Empty is an answer |
| `session.changed` | One job whole, whenever a fact changed something |

Both reads are on the agent's door, so a job can ask who else holds the branch it is about to take.

## A job Fleet hosts

A person can start a job from Bridge. Fleet runs it, keeps its thread, and
puts it on the same ledger with `origin` `bridge`. It is the person at
work, in their own configuration. [Helm](helm.md) is the same
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
> follow. Where the person's style and the rules conflict on how a need or
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
send of. `settings.session-quiet-timeout` is how long a process may sit idle.
[Spike 26](../spikes/026-does-a-message-from-another-session-wake-a-live-process.md).

**The lease is taken by a hook, and nothing writes the main checkout before it.**
A write tool, or a shell line that is not a read, is refused until the job
holds a slot; after it, a file tool naming a path in the main checkout is
refused with the slot's path. A shell line is read by its words, so one Fleet
does not recognise as a read leases first. `|`, `;` and `&` inside quotes do not cut
a line, and a redirect to `/dev/null` or onto another descriptor (`2>&1`) writes nothing;
any other `>`, a `$(` or a backtick, or a quote left open is a write. `sed -n` with an
address and `p` (`sed -n 10,40p file`) reads; every other `sed` is a write.
[Spike 24](../spikes/024-how-does-a-session-move-into-a-slot-at-its-first-write.md).

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
repository with no `armada.yml`, it has no pool to lease from, so until that root's
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
writes by itself. [Spike 29](../spikes/029-why-a-sessions-auto-asked-and-how-a-question-is-answered.md).

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
[Spike 25](../spikes/025-is-auto-mode-reachable-in-a-spawned-session-now.md).
A change applies from the next process: an idle one is ended at once, a running
one when its turn is over.

**A question is an ask too.** The agent's `AskUserQuestion` reaches the same door as any call, and
in a hosted job nobody could answer it. Fleet now holds it whatever the mode and the thread
draws it as a form: each question with its options, a multi-select where the agent said so, and
*Other* for the person's own words. *Answer* hands back one entry per question, *Skip* tells the
agent so. Fleet returns the tool's input with its `answers` map filled in, keyed by question text,
a multi-select's labels joined with `, `. [Spike 29](../spikes/029-why-a-sessions-auto-asked-and-how-a-question-is-answered.md).

| Operation | Does |
|---|---|
| `start_session` | A row for one repository, with optional title, model, effort and mode |
| `rename_session` | A person's name for a Job, hosted or in a terminal; stands until the next terminal `/rename` |
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

### What a restart does to a job

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

### Background tasks

**A hosted Job's ledger shows what its agent runs beside its turn, and Fleet detects it.** A `Bash` call run in the background and a `Monitor` watch each become one `background` row, read off the Job's own stream, so no tool call of the agent's is involved (the owner, 14 Oct 2026: Fleet detects state and does not rely on a tool call). The call is remembered when it is heard; the harness's `task_started` line names the task, the call that asked for it and whether it runs beside the turn, which is true for a call a hook moved to the background as it is for one that asked. A task whose call was not a shell command or a watch (a subagent) has no row, a command that ran in the foreground has none, and an end for an id that never started here settles nothing. A subagent's backgrounded command is the Job's row, heard the same way.

| Line | Gives the row |
|---|---|
| `task_started` | `target` (the task id), `detail.task` (its description), and from the call, `detail.form` and `detail.command` (the whole command) |
| `task_updated` with a final status | `spent`, and `detail.status`: `completed`, `failed` or `killed` |
| `task_notification` | `detail.output` (the file its output went to), `detail.exit`, and the same `status` (`stopped` is `killed`) |

The two ending lines each fill in what they have, in either order. **The exit code is the one fact read out of prose:** no key carries it, so only the end of the notice's `summary` is read, in the two forms the harness writes (`completed (exit code 0)`, `failed with exit code 3`). A summary in neither form has no `exit`. `detail.output` is only known once the task ends. A terminal Job's `background` rows are not kept: its mod does not report them.

## Showing a window

**A job shows the person a web page with `show_window`** (`url`, optional `title`, optional `variations`, optional `steps`, optional `origins`, optional `sign_in`), a tool on the agent's door, and Bridge opens it in a capture window by itself. Any `http` or `https` address; any other scheme is refused with its reason. Fleet places a hosted job by its connection, as for its asks, so the call names no job; a terminal job names its own id, which the mod puts in the model's first-message context. It writes one `window` artifact on the ledger however often the address is shown, and a quiet `window` row on the thread each time, which is what makes Bridge open (or raise) the window. Pressing the ledger row or the thread row opens it again, only for an address the ledger shows as a window. **A note taken in the window goes to the job as a message from the person**, with the element it pointed at, and wakes it. The window is pinned to the origin it opened on, as a server's is, and to the `origins` the call names (*A deployed page*). **`variations`, a list of `{name, url}`, offers several takes of one change in the one window**, `url` the first's and each held to the same `http`/`https` rule. The bar draws a letter and name per take, a press swaps the page, and the pin moves to the origin of the take chosen and to no address the job did not send. A note names the take it was taken on (`On B, Needs you first: …`), Approve names the one on screen (`Approved B, Needs you first: <url>`) and is per take, and the window row and the ledger's `window` artifact keep the takes so a reopened window brings them back. **`steps` walks the person through any page, on any repository's app**: a local dev server or a deployed staging address. Each step is a `say` caption and one verb (`press`, `look`, `hover`, `later`, `type`, `key`, `select`, `paste` or `drag`) aimed at a target found by role and accessible name, or by the words it draws: the mock walks' own shape, as JSON, so a page needs nothing of Armada's in it. Bridge plays them from outside the page. It rings each target and draws the caption over the page, and acts on the ringed target with real mouse and keyboard input when the person presses Next, so the page hears what it would from him. The bar's walk row (Back, Next, Autoplay, Replay and the keys) drives them as it drives a mock's walk. **A target Bridge cannot find holds the walk on that step**, the bar marks it, and Fleet tells the job in its own voice which step and which target, once (`window_failed`), so it fixes its steps and shows the window again. Fleet refuses steps of the wrong shape with the first one that is wrong. They are kept on the `window` artifact and row beside the takes, and a window shown with other steps is another window. A page that plays its own walk (Armada's mock) is passed none and plays as it did. `docs/practices/capture-window.md`, *A walk in the page*. **The bar says whether the page is ready to walk**, a Job mark at its start (*Ready to walk*, or *Still changing*) and the bar's hue, and Approve reads *Not ready* while it changes. **It is detected, not declared:** `show_window` says ready, the mod's report of a file the job itself wrote (`wrote`; a subagent's does not count) or a note from the window marks every window the job showed as working (`detail.working` on its artifact), and showing the page again or the end of the turn (`turn_completed`) marks it ready. **A turn that ends while a `subagent` of the job still stands leaves the windows working**, since a job that hands a note to a background subagent and ends its turn has not finished changing the page; the subagent settling marks nothing, and the turn that takes its report does. Only subagents count: a server or a shell left running holds no window. **The window draws the job beside the page**, its thread and the box to answer it, so the walk is read and answered without going back to Bridge's window. `docs/practices/capture-window.md`, *The panel beside the page*.

**A deployed page** can be walked as a local one is. A staging environment behind single sign-on redirects to a provider and back, so `show_window` takes `origins`, a list of other `http` or `https` origins (a host and an optional port, no path) the window may pass through. A navigation or redirect to one of them is followed; anything else is refused as before, and the page stays pinned to the shown origin and those. A call whose `origins` or `sign_in` break the rule is refused with 422 and nothing written. The window keeps **one cookie jar per repository**, read off the Job's `manifest_id`, and Jobs in no repository share one unnamed jar. So a sign-in holds for later walks of that repository, and two repositories share none. How the window gets signed in depends on the repository:

| The repository | The job passes | Who signs in |
|---|---|---|
| Gives no way in | `url` and `origins` | The person, on the provider's page in the window, once. The jar keeps it |
| Has a dev magic link or a token login address | `url`, `origins` where it redirects, and `sign_in`, a list of addresses on the shown origin or a named one | The window, before the page loads: each address in order, in the repository's jar |

**`sign_in` is a secret.** Fleet holds it in memory from the call until Bridge takes it (`take_window_sign_in`, once), the job shows another window, or the job ends, and writes it nowhere: not the thread, the ledger, a `window` row, the timeline, a note, the receipt or an event. It is held for as long as Bridge is away, so a Bridge that connects after the page was shown opens the window of each live job whose newest shown page it never opened and was shown in the last 15 minutes, and takes the sign-in then. Bridge holds it for as long as the window takes to load it. A load of one that fails is told to nobody. A window reopened from the ledger or the thread signs in with none, so it relies on the jar. A popup onto the shown origin or a named one opens as a child of the window and closes on `window.close()`; every other popup is refused. The window's user agent is Chrome's, without the Electron token. While the page is at a provider or its popup is open, a walk with `steps` waits on its step and tells the job nothing (`docs/practices/capture-window.md`, *A popup a Job's page opened*).

**A question asked while the person walks a page is refused, not held.** A message starts a turn only once the current one ends, and an open `AskUserQuestion` keeps it open, so every note and the Approve would queue behind the question until he answered it. [Spike 27](../spikes/027-how-does-fleet-deliver-text-to-a-live-terminal-session.md).

| | |
|---|---|
| A walk opens | At `show_window`: the newest `window` row on the thread. A Bridge that was not connected opens it when it connects, if the walk has not ended and was shown in the last 15 minutes |
| A walk ends | The person approves the page (`Approved: <url>`, or of several takes `Approved B, Needs you first: <url>`), Bridge says its window closed (`window_closed`), or a newer `show_window` replaces it. The row gets `ended_at` and is published again |
| A note in the window | Ends nothing, though it settles the `walk:` waiting item |
| The window cannot show the page | Ends nothing. Bridge says so (`window_failed`) and Fleet tells the job in its own voice, with the address and what went wrong: a failed load, a status of 400 or more, a crashed page, Vite's error overlay or a page with nothing drawn. Not while the page is still changing, and once until it is seen working |
| A step cannot be played | Ends nothing. The walk holds on the step, and Fleet tells the job in its own voice which step and which target Bridge could not find (`window_failed` again, with a reason that begins `step`), once until the step is found |
| A question meanwhile | Refused with a reason the agent reads, and **kept whole**: Fleet writes the ask row with every question, header, option and description, settled `kept`, so it draws in the Needs you list as the question that lapsed does, to be answered after the walk (the answer goes to the agent as his message). The agent is told not to add it to `waiting_for`, and to end the turn. A question about the page is asked in the window with `ask_in_window` instead (*Asking in the window*). Since 17 Oct 2026, the owner: "why can't we keep all of the options and everything about the question? Turning it into a reply box makes no sense." A `kept` question, unlike an `unanswered` one, is not cleared by his notes or his Approve, which are what he sends during a walk; a newer question or his answer to it clears it A hosted job's is refused at its permission door; a terminal job's at `ask_from_terminal`, which the mod asks before the terminal's own prompt opens |

The mod's first-message note and `show_window`'s description tell the agent the same. Closing the window ends the walk and does not settle its `walk:` item, which still waits. A Bridge that quits with a window open may not reach Fleet to say so. The walk then reads open until it is approved or replaced, and a question meanwhile is kept as above, which blocks nothing.

## Drawing a sketch

**A session draws the person a diagram with `show_sketch`** (`title`, `scene`, optional `against`), a tool on the agent's door, in place of ASCII art. The scene is the format of `docs/concepts/sketch-pane.md`: `nodes`, `edges` and optional `strokes`. Fleet places a hosted session by its connection, as for `show_window`; a terminal session names its own id.

**A scene that `parseScene` would draw as an empty frame is refused first.** `ipc` (`sketch.rs`) checks it against the same rules and answers 422 with up to twelve problems, each naming the part (`nodes[1] `b``), what it holds and what is allowed. It is stricter in one way: a field the format has no name for is refused, where `parseScene` drops it. A scene with no nodes is refused. The kinds, wire parts and icons are listed in the tool's description, and `crates/ipc/src/tests/sketch.rs` fails when they differ from `scene.ts`.

**A `screen` node is a wireframe of an app's screen, for any product.** Its `screen` field is a layout tree of rows, columns and grids that size themselves, holding generic parts (window, nav, panel, tabs, list, table, button, field, text, chat, modal and the rest), so the agent says what is on the screen and never where. `crates/ipc/src/sketch/screen.rs` checks the tree against `readScreen` in `packages/components/src/compositions/SketchScene/screen.ts` and refuses each fault by its path, such as `nodes[2] `team` screen.children[1].rows[0].icon`, with what it holds and what is allowed. A tree nests at most 12 levels. The test reads the element types, their fields, the enums and the glyph list out of `screen.ts`. The tool's description teaches the types, and a refused glyph is answered with the whole list.

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

**A question about the walk opens in the walk.** A job that wants the person's answer about something on a page it showed calls `ask_in_window`, a tool on the agent's door, with the `question`, its `options` (each a `label` and an optional `description`), the page's `url` and a CSS `selector` for the place, with an optional `text` the element shows that picks one where the selector finds several. The owner, 11 Oct 2026: "a question about the walk should open in the walk. In fact it should take me where the agent is asking me about. Highlight it like an annotation and show a card with the question and my options to answer. If the question is not related to the walk it should wait."

| | |
|---|---|
| What it may ask about | An address on the origin of a window the job showed, or of any of its takes (the window swaps to that take), whether or not its walk is still open: a closed window is opened again. Anything else is refused with a reason that names `AskUserQuestion`, which Fleet keeps whole while he walks |
| What the call does | Keeps a `window_ask` row on the thread and answers at once. **It does not wait**: the job ends its turn, and the answer arrives as a message from the person |
| What Bridge does | Brings the job's walk window forward, opening it if it is closed, moves the page to the address, scrolls the place into view, outlines it, and draws the question on a card of the window's own bar. `docs/practices/capture-window.md` |
| How it waits | `window_ask:<row>` in the Needs you panel, with its options and its `place`. Its act is a walk of the window it was asked in, so a press there opens the window with the card |
| How it is answered | An option, words of the person's own, or both, through `answer_waiting`. The job gets `Re: <question>` with the option and the words beneath it, and the row keeps the answer. Being a message, it also settles the walk it was asked in. `dismiss_waiting` drops it with nothing sent |
| While sleep is on | Recorded as a walk is and never decided for him: it is about what is on the page |

Capture and Approve keep working while the card is open. The card folds into the bar so the page takes presses again, and the window keeps showing it until it is answered or dismissed.

## What a Job is doing

**Nothing the person sees depends on the agent making a tool call.** Fleet reads what a Job is doing from what it observed, and `waiting_for` and `all_done` only add detail to it. `SessionRecord.standing` says it, for a live Job; an ended one has none.

| `standing` | Read from | Item in `waiting_for` |
|---|---|---|
| `working` | a turn is running and something was heard from it within the bound, or, between turns, a `subagent` row of it stands: a parent that ended its turn to wait on its subagents read as idle while they ran (14 Oct 2026). A `background` row does not count: the agent is woken when its command ends, and the person has nothing to wait on | none |
| `stuck` | a turn is running, nothing has been heard from it for **ten minutes**, and it is not held on a question or a permission | none |
| `done` | `done_since` stands (the agent said nothing is left) | `done:<done_since>`, text `Done` |
| `replied` | the last turn ended on a message to the person that only reports, and he has not written since | none: the unread mark |
| `idle` | none of the above, which includes a reply that asks (it is an item) | none |

A reply whose close asks him something is an item whatever the standing: `replied:<turn ended at>`, source `replied`, text the sentence that asks.

**The reply is read off the turn.** For a hosted Job the turn is on its stream: the last thing the agent said before the turn ended. For a terminal Job the mod reports `turn_started` when a prompt goes in and `turn_completed` with `said`, the first line of the last assistant message, when the turn ends; the agent is never asked to. Fleet keeps the turn's end and the end of what was said (the last 1,500 characters, from a line start) in `session_turn_end` (one row a Job), so a reply survives a restart. A terminal Job's mod sends only a first line, so that line is all there is to read for an ask. It stops standing when the person writes to the Job, when another turn begins by any route and when the Job ends. A turn that ended on tools alone, with nothing said, leaves no reply, and a Job whose turn ended before Fleet observed this has none: nothing is read back from old turns. A Helm Job has no reply, since its replies are the dock's. A `done` outranks a reply.

**A reply that only tells is no item; a reply that asks is one.** The owner, 14 Oct 2026: "When a session replies to me, it shouldn't show up in needs me. Needs me is when it needs something from me. Replied should be idle or unread." And 17 Oct: "Part of me wonders if we could use a cheap rust natural language processor to detect if the latest message from an agent in a thread is something that needs me. Then we can classify replies that dont need me as unread and give them more weight. Then replies that need me to respond can show as something that needs me." A reply that only reports is `replied`, an idle Job with an unread mark, which Bridge draws and clears once he has been in the Job (`cockpit/viewed.ts`); Fleet does not yet say that he has.

**Whether the close asks is read by rules** (`session_host/asks.rs`; `docs/spikes/reply-asks-classifier.md` has the measurement and why no model). The reading leans toward yes: a wrong call costs a dismiss, and a missed ask leaves a Job waiting unseen. The item is not raised where the agent already asked through a tool (a turn held on a card or permission, or an agent `waiting_for` item, question card, permission or window question open), where a `done` stands, or while a subagent works. A dismissed id never returns, and dismissing it sends the agent nothing; answering it with text is a message to the Job. A `done` is an item too: it asks whether to end the Job.

**Stuck is its own bound**, `STUCK_AFTER` in `session_host/standing.rs`, ten minutes of nothing heard from a running turn. It is not `settings.session-quiet-timeout`, which is how long an idle process is kept and says nothing about a turn that went quiet. A Job goes stuck by time passing, which no event says, so the notice loop's tick looks at every running turn and publishes the Jobs whose stuck state moved, in both directions (`standing_checked`). A terminal turn's running state is kept in memory only: a Fleet restart forgets it, and the Job reads as idle until its next turn starts. A mod from before `0.3.20` reports neither `turn_started` nor `said`, so its Job is never working or stuck by this reading and has no reply; a `done` still stands.

**A terminal's permission prompt.** Claude Code in a terminal asks its own permission for a tool call, and nobody may be at the terminal. The mod forwards what the harness announced: its `PermissionRequest` hook reports `permission_asked` with the tool and its command or path, where no hook beneath decided it, and the tool call ending reports `permission_settled`. The agent is never asked. The mod also puts the prompt to Fleet as it puts a question, and races the terminal against the answer: **the call is answered from Bridge**, Allow once or Refuse, as a hosted Job's permission is, or at the terminal, whichever comes first. An allow from Bridge runs the call again past its own check, and the terminal's dialog closes (`docs/spikes/031-can-bridge-answer-a-terminal-permission-prompt.md`). In the Job the command is a block to run, and its ▶ is Allow once. Allow and remember is not offered, because remembering is the harness's own. Until the ask arrives Fleet raises the call from the fact alone, with no options and the `message` act naming the Job. While it is open the turn is waiting on the person, never stuck. A turn starting or ending, and the Job ending, closes it too. A mod from before `0.3.22` reports neither, so its prompt raises no call and the turn reads stuck after ten minutes.

**Bridge reads the state from `standing` and from nothing else**: not from an event's `live` flag or its age, and not from whether a tool was called. `Timeline::session_turn` marks no entry live, because a turn that finished is not moving. An answer sent from a card counts as him being in the Job, so the items he has answered stop being calls (*Waiting on the person*).

## Waiting on the person

**A job says what it is waiting on the person for with `waiting_for`**, a tool on the agent's door that replaces the whole list each call (`items`: `id`, `text`, an optional `act` of `walk`, `answer`, `approve_pr` or `run` with a `target`, optional `options`, and optional `steps`, a list of short lines); an empty list clears it. Fleet keeps the agent's items in `session_waiting`, stamps `since` and keeps it for an unchanged `id`, and **adds its own on every read** into `SessionRecord.waiting_for`: `ask:<call>` for an open question, `perm:<call>` for an open permission, `walk:<url>` for a window the job showed that no `Approved: <url>` message (of any of its takes) has followed, `window_ask:<row>` for a question asked in the window and not yet answered (*Asking in the window*). Where an agent item names the same act and target as one of Fleet's, Fleet's stands, and an agent `walk` item for any address the job has shown in a window (trailing slash aside, approved or not) is left off: the window is the one record of that page. An ended job holds nothing. **Bridge shows the list on the composer**, as a **Needs you** band folded to one glyph an item, named on hover; a press rides it up as the header of a panel and dims the thread. The panel opens as a list, a row each and no act on the row; a press on a row opens that one to do its act, and Back or Esc returns to the list. The owner, 12 Oct 2026: "when the session is not completed, the 'needs you' items in the right panel move to a panel in the thread like this that is locked to the bottom. Maybe it's collapsed by default?" A question or permission is left out of the band, since its own card shows it. The band stays when the agent says it is done, above the done callout; the ledger holds no section for them. The owner, 16 Oct 2026: "I really don't like the waiting for me being in the ledger. its already in the thread. We dont need it in the ledger." Bridge puts a Job with items under Needs you with a bell-ring mark whose tooltip lists them. **The Cockpit raises a call for an item only while he has not been in the Job since it began waiting** (*Cockpit*, the call deck): Bridge keeps when he last viewed each Job (`localStorage`, per id), his latest message counts as being in it, and an item whose `since` is older than the later of the two is no call, though it stays in the list. **`answer_waiting`** (`session_id`, `item_id`, and a `choice` index into the item's `options`, `text`, or `mode` of `best` or `quick`) settles one: a card item goes through `answer_session_ask`, a walk item sends the approval as the person's message, an agent item becomes a message to the job, and once that is delivered the item is dismissed as `dismiss_waiting` does, so it leaves the list without the agent restating it. A `choice` with `text` says both: the words go with the pick, to the agent as a second line and to an ask card as its note. `best` tells the agent to decide for itself and think it through, `quick` to just get it done: finish the work and land it, skipping the formalities. **`dismiss_waiting`** (`session_id`, `item_id`, and an optional `reason`) drops an item for good: the id is kept in `session_waiting_dismissed`, a derived item with it never comes back, and an agent item with it is removed and left off if the agent states it again. **An item the agent stated tells it so**: the Job is sent `Owner removed "<item text>"`, with `: <reason>` after it where the person gave a reason that is not blank, so the agent does not take the silence for an answer. The owner, 14 Oct 2026: "when there is a 'needs me' item on a job, I should be able to delete it, which lets the agent know that I removed it. Maybe I can give a reason when I do." An item Fleet derived (a card, a permission, a walk, a window question, the first message, a done) is only dropped, and the agent is told nothing, since it never stated it. The message goes before the item is dropped, so a send that fails leaves the item standing to try again. A nothing-held id is refused as for `answer_waiting`. **Only the newest walk window of a job waits**, and it stops waiting once the person has sent the job a message after it opened; closing the window does not settle it. The mod's first-message note tells the agent to keep the list current, to give each item an act, and never to list a page it showed with `show_window`, which waits by itself, and puts these tools ahead of the person's own rules for reporting actions and questions in a reply. It also says a decision is asked one way, in the prompt or in `waiting_for` and never both, that a shell command for the owner goes in a block tagged `bash` so Bridge draws its run button, and that a page is shown as a link and never a screenshot. **An agent item that carries `steps` is a procedure**: its glyph is Steps, and its row reads its `text`, and its press opens the steps beside the thread in place of its `act`. A `walk` stays as the page they are about, one key at the top and a second that opens it in the browser; a `run` stays as its command, drawn whole at the top with a copy key in the block's corner, and left off where a step names it. A step's `backticked` command is set as code in its line and drawn again under it, whole, with its own copy key. The owner, 15 Oct 2026, on a run item whose steps hid its command: "The copy button should be embedded in the code block instead of outside of it." Each step has a tick that is the person's own place-keeping, a step the agent got wrong is edited in place, dropped (struck through, kept) or added at the end, and the key under the reply box reads Done while the box is empty and Send once something is typed. Done answers the item with `Done`, ticked or not; either key sends the corrections as plain lines after the reply (`Step 2 changed: "was" → "now"`, `Step 4 dropped: "was"`, `Added after step 6: "now"`). Fleet trims each step and drops a blank one; an empty list is no steps. The agent keeps `text` to one short line. An agent item with none, or with `answer` and no `options` while no question is open in the thread, gets a Reply glyph, and its press opens a box under the row whose text goes to `answer_waiting` as `text`.

**A job says it has nothing left to work on with `all_done`** (`session_id` only; the hosted job holding the connection wins, as for `waiting_for`). Fleet stamps `SessionRecord.done_since`, keeps it in `session_done`, and publishes `SessionChanged`; a second call keeps the first instant, and an ended job is refused. It is a statement and ends nothing: Bridge draws a callout at the foot of the thread, **End job** (the ordinary close) or **Keep going** (local to Bridge, it hides that `done_since` only). `done_since` stops standing when the person sends the job a message (`send_session_message` or an `answer_waiting` that becomes one), when the agent calls `waiting_for` with a non-empty list, and when a hosted job's turn starts again, except a turn that Fleet's own news starts (`told_news_by_fleet`: a pull request merged, a build moved), which asks the agent for nothing. A word that asks for work, such as a failed check or a stranded slot, clears it. A terminal job's mod reports a prompt going in as `turn_started`, which is the same turn-start signal; a mod from before it has none, and for that one only the first two apply. The mod's first-message note tells the agent to clear `waiting_for`, then call `all_done` only when nothing is owed and none of its background work is running.

**A job gives back what it is finished with using `finish_up`** (`session_id` only; the hosted job holding the connection wins). It releases each slot the pool holds for the job as `armada worktree release` does, which refuses a tree with uncommitted files or commits that neither its branch's remote nor the base has; stops the servers running in each slot it released; and deletes the local branches those slots were on, the one the job was leased and the head branch of each pull request it holds or held, where the base (the local branch or `origin`'s) already holds every commit. Answered with `released` (slot numbers), `deleted_branches` and `kept`, a sentence for each thing that stayed and why. It commits nothing for the agent. **It is a call on Fleet's door because a command cannot do the same:** once a slot is released the next write in the job takes a new lease, and the git guard refuses `git branch -D` and `kill`. A terminal job is refused; its slot is parked when its process ends. The mod's first-message note tells the agent to call it before `all_done`, and to start servers with `start_server` so it can stop them.

**A job marks done a note Bridge sent it using `mark_annotation_done`** (`note_id`, and `session_id` placed as `finish_up`'s is). A note id is letters, digits and `-`, so it names no path outside an annotations directory. Fleet looks for `<note_id>.json` under the served root's `.armada/annotations/`, then the preview's `.armada/preview/.armada/annotations/`, then each slot's and each `.claude/worktrees/` checkout's; none found is refused, naming where it looked. Only a note whose `sent.sessionId` is the calling job is marked: `status` becomes `done` and `updatedAt` Fleet's now, every other field stays in Bridge's order, and the file is written through a rename. Answered with the `path` written. **It is a call on Fleet's door because the note may sit in a checkout the job holds no slot for**, such as the preview's, and the slot guard refuses that overwrite; the guard is not widened for it.

**An agent that calls `show_window`, `ask_in_window`, `waiting_for` or `all_done` through the door is answered a receipt, not the row.** The row grows with every attachment (36 to 62 KB for a long job, 10 KB at the median, measured 7 to 13 Oct), and an agent keeps each answer in its context for good. The receipt is `session_id` and what the act set: `window` (with `title` and the `takes` names), `asked` (the question's `window_ask:<row>`), `waiting_for` (the agent's items as Fleet kept them, `id` and `text`), or `done` and `done_since`. It weighs the same on the first call and the thousandth. Bridge's routes and the mod's hook, which are not the door, are answered the row. Nothing reads these four answers today.

## Working on

**A hosted Job shows what it is doing, and Fleet detects it.** `SessionRecord.activity` (`SessionActivity`: `now?`, `todos[]`) is built from the tool calls Fleet's PreToolUse gate sees, which is every tool call of a Job Fleet hosts: the hook's matcher is `*`. Until 14 Oct 2026 the matcher named only the five tools a lease is taken on, so a read, a search, a subagent or a `TodoWrite` never reached Fleet and `now` and `todos` stayed empty for a Job that was not writing. A call to any other tool is noted and let through before the pool or the Manifest is read. No tool asks the agent to say it, because Fleet does not rely on an agent calling a tool to know its state (the owner, 14 Oct 2026). The one thing the agent does say is its own to-do list, and Fleet reads that off the call too.

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

**It is kept with the rest of a Job's runtime state, in memory.** It is not in the store, a Fleet restart starts it empty, and it is dropped when the Job ends. The gate notes a call only for a Job that is hosted and open, after the check that already passes an unknown or ended one, and does so whatever the gate then decides about the call.

**A call does not publish a row.** The row is large and goes out whole with every `session.changed`, and a busy Job makes many calls. A change to `now` or `todos` publishes at once when the Job's row has not gone out for what it is doing within the last two seconds. Otherwise one publish follows when those two seconds are up, carrying the latest, so the last change is never lost and a burst costs two rows. A call that changes neither publishes nothing, so a long run of identical calls costs no rows. The gate takes a lock for a moment and awaits nothing, and the publish is a task of its own, so the answer to the call is not held for it.

**A turn starting clears `now` and keeps `todos`.** The line of the last turn is not the new turn's, and the agent's list is still true across turns. It is the same moment `done_since` stands down, and a turn Fleet's own notice begins counts: a new turn is a new turn. The row that announces the turn carries the clear. A turn ending clears nothing.

**Bridge draws nothing for a Job whose turn is not running.** Whether a line or a band is drawn is Bridge's rule, read off `hosted.turn` and these fields.

**Silence is not here.** A running turn nothing has been heard from is the `stuck` state, from the activity of the process Fleet hears, whatever it calls. `activity` says what a Job is doing, and carries no time to read a silence from.

**Only a hosted Job is covered.** A Job run in a terminal makes its tool calls in a process Fleet does not host. Its mod reports a few tools through `report_session` and does not call the gate, so such a row carries no `activity`.

## The build record

**`SessionRecord.build` says how far a Job has got, read from what Fleet observed.** It asks the model for nothing: there is no tool for the agent to call and no line in any prompt. (That was the rule of 14 Oct 2026. The owner, 15 Oct 2026, wanted a plan he approves before the work, so a Job now has `record_plan`; *The plan* has it. The ring still asks nothing: it is read from the same observations.) The agent states a plan with the task tools it already uses, the mod hears those calls, and Fleet checks the plan against the edits it saw. Bridge draws the record as a ring under the Job's drone (the decision of 14 Oct 2026, *A Job builds a ring*). The field is absent for a Job that has done nothing Fleet observed.

**What the mod tells.** `TodoWrite`, `TaskCreate` and `TaskUpdate` become one fact, `tasks`, holding the whole list after the call. `ExitPlanMode` becomes `plan_left`. A successful `git commit` and `git push` become `committed` and `pushed`, read from what git printed and not from the command alone, so a commit with nothing staged, a push with nothing to send, a rejected push and a `--dry-run` tell nothing. **A subagent's `TaskCreate` and `TaskUpdate` are the Job's list too** (the owner, 15 Oct 2026: *"An entire session contributes to the workload for that session from a scope/plan to implementation"*). The task tools are shared: a subagent's `TaskCreate` takes the next id of the parent's list and its `TaskList` shows the parent's tasks (measured on claude 2.1.300, headless with the task tools on), so the mod hears them into the one list and marks each task with the `agentId` of the subagent that made it. Whoever updates a task, it stays its maker's. **A new process starts from the harness's list, not from nothing** (mod 0.3.27): a hosted Job's process is replaced when Fleet restarts or moves it into a slot, and a terminal Job can be resumed, and either leaves the mod with no list in memory, so a `TaskUpdate` named a task nothing there had made and was never told (Job 7c5094db, 16 Oct 2026: nothing after its process was replaced was heard). At its first task tool the mod reads `~/.claude/tasks/<list>/*.json`, the list being the job's id or `CLAUDE_CODE_TASK_LIST_ID`, takes it as the list, and applies the call on top, so a rename of a subject is heard as well as a status. The harness's files are the harness's own record of what its tools did, which is why they are read rather than the change being trusted to arrive whole. A store that cannot be read tells nothing, and is tried again at the next task tool. A subagent's `TodoWrite` is a list of its own (the older generation) and is not told. **Its reads, edits, commands, commits and pushes are the Job's work**: in a terminal Job the mod's hooks fire inside subagents and report under the parent, and a hosted Job's process stream carries a subagent's tool calls, which Fleet hears without drawing its words. A hosted Job feeds the same path from its gate (`TodoWrite`, `ExitPlanMode`) and from its process's tool events. **A hosted Job is started with `CLAUDE_CODE_ENABLE_TODO_TOOLS=1`**: a headless process has no task tools without it, so until 15 Oct 2026 its agent could write no list and its build record never had pieces.

**Subagents on the ring.** Fleet turns a task's `agentId` into the label `workers` uses, `Type: description`, from the Job's `subagent` attachment, and keeps it on the task (`session_build_tasks.made_by`) because the attachment is spent by the time a finished subagent's pieces are last read. Each piece carries it as `by`. Bridge draws a running subagent as a worker inside the ring (`workers`), and the pieces whose `by` is its label are the segments it wrote, so the ring fills as subagents tick their own steps. Piece kinds follow the same rules whoever wrote the task: an edit the Job saw while it was worked is what makes it solid.

**What Fleet keeps.** Two tables (`session_build`, `session_build_tasks`): the instants of the first edit, the last edit, the last tool activity, plan mode left and the list's first write, and the list with the instant each item was first seen completed and whether an edit was seen while it was worked. Items are matched by id and text, else by text, so a reordered list keeps what was seen of each. An item that goes back to not completed loses its instant. Everything else is derived on read. A walk shown, a commit and a push are timeline entries (`session.walked`, `session.committed`, `session.pushed`), and a pull request's stops are the entries it already has (`pr.opened`, `pr.checks`, `pr.queued`, `pr.merged`); the row is published again when one is written.

**A row with no first edit is filled from the timeline.** When a Job's row is first made, and on each fact heard while the row still has no first edit, Fleet reads that Job's `session.edited`, `session.committed` and `session.pushed` entries in one query, from the later of the Job's start and the last merge of a pull request it held. The earliest of the three is the first edit, the last `session.edited` entry is the last edit, and the build's start moves back to the earliest. This covers work heard before this Fleet started or before the row existed, which the timeline keeps for 30 days. A row that has a first edit is not read again, and no row is made from nothing kept. An entry is dated by its burst's first call, so a seeded last edit is when the last burst began.

**A commit or a push counts as the first edit** where none was heard, since it proves code changed even when a script made the edits through a shell. A read or a command does not. A commit or a push does not start a new build after a merge, as a read does not.

**A build ends where its pull request merged.** The first edit, plan or task list heard after the merge starts another build with no list and no road. A read or a command after the merge does not.

| Field | Read from |
|---|---|
| `phase` | `landed` when the pull request merged. `road` when a stop was reached at or after the first edit, or any stop where no edit, commit or push was ever observed. `plan` when a task list was written or plan mode left and no edit followed. `implement` when edits, a commit or a push were seen. Otherwise `scope` |
| `pieces` | One per task, in list order. `solid`: completed, and an edit was seen between the previous completion (or the list's first write) and this one. `hollow`: completed with no edit seen in that window. `scaffold`: the first task not completed, while an edit has landed since the last completion. `ghost`: every other. With no list there are none: progress is never invented. `working` marks a task the agent's tool has in progress, whatever piece it is |
| `stops` | The road as drawn: the stops reached in the order they were, then the forecast ones not yet reached, each with no `at` |
| `forecast` | The stops the repository's recent landed Jobs usually reached |
| `landed_at` | When the pull request merged, as the forge dated it |
| `quiet_since` | The last read, edit, command or task list. It is as of the last time the row went out, which a call after a minute of quiet does |
| `stalled` | The Job's `standing` is `stuck` |
| `workers` | The subagents still standing, each as `Type: description` |

**The stops.** `walk` is the first walk shown after the first edit. `pushed` is a push that updated a ref. `pr` is the pull request opened, which is never later than its first check, queue entry or merge. `checks` is its first CI state. `auto_merge` is its first queue state. Merging is not a stop: it closes the road and is `landed_at`. The pull request is the newest one the Job holds that is not closed and did not merge before this build began.

**The forecast rule.** For the Job's repository, read the pull requests that merged most recently and that a Job held, up to ten. For each, take the stops reached between that Job's previous landing and this one, in the order they were reached. The forecast is the most common whole sequence among them, and a tie goes to the more recent. A landing with no stop observed takes no part, and a repository with no history has no forecast. A stop already reached is not drawn faint twice.

**What is not observed, and left absent.** A task made by an earlier process of the Job is read from Claude Code's own list on disk (see *What the mod tells*), so it has no maker: a subagent's task from before a restart reads as the Job's own. Edits made through a shell command (`sed`, a redirect) are not edits, though the commit that follows them is the first edit if none was heard, and nor are files outside the Job's directory. A task completed before its edits were filled in from the timeline stays hollow, and a seeded last edit is only as exact as the timeline's bursts. A Job with no new fact since a Fleet restart keeps the row it had until its next tool call. A task the agent never marks completed is never solid or hollow. Auto-merge set through the forge's page, and checks Fleet's merge line read only after the merge, may have no stop.

## The plan

Since 16 Oct 2026. The owner, 15 Oct 2026: a Job plans with him first, and what he approves is what gets built. **Nothing in a plan is the agent's word about progress**: the plan is its statement of the work, and how far it got is Fleet's observation, as for the build record.

**`record_plan`** is a tool on the agent's door. It carries `groups` in order, each with a `title`, an `agent` (the subagent type that takes it, one word, free text such as `rust-engineer`), `paths` (from the repository's root; an edit under one marks the group worked) and `tasks` (titles), and an optional `sketch` (`title` and `scene`), checked by the code `show_sketch` uses, so a scene that would draw as an empty frame is refused with the part at fault. A group or task may send back the `id` Fleet gave it. Fleet refuses a plan with every reason at once and says to call `record_plan` again; nothing is kept from a refused plan. The agent gets back the version and each id Fleet gave.

**Recording again replaces the plan as a new version.** Fleet keeps the current version and the diff from the one before: a group or task is matched to the old one by its id, then by its text with case and spacing ignored, and what differs is `added`, `renamed`, `moved` (to another group) or `removed`. An id the old version never held is not an id. The sketch's scene is a ledger artifact `sketch:<title>`, as `show_sketch` keeps it, and the plan names it.

**A recorded plan is pending until the owner approves it, and pending is one of the Job's calls.** It is a waiting item, id `plan:<version>`, source `plan`, where `ask:` and `perm:` items go. Nothing blocks the agent's tools meanwhile; the wait is the agent's to keep, and the Job rules say to keep it (`docs/contracts/agent-prompt.md`, 5f). Sleep mode leaves it for the morning, and `answer_waiting` refuses it: a plan is approved, or sent changes, from its own card. **`approve_plan`** (`session_id`, `version`) is a Bridge route. It sets `approved` and sends the agent one message, before it settles, so a failed send leaves the plan to approve again. A version that is not the current one is refused (409, `fleet.plan_version_stale`), a Job with no plan too (`fleet.plan_none`). A second press tells the agent nothing.

**His changes are proposals.** **`send_plan_changes`** (`session_id`, `version`, `marks`, `approve`) takes marks against the version he read: rename, remove, add task, add group, move task, change subagent, note. Fleet checks each against that version (a mark may name a group or task another mark of the same batch adds), words them as **one message** to the agent, with the ids so it can answer by them, and writes one `plan_changes` row to the thread in place of a Fleet message beside it. **The plan does not change**, whether it is pending or approved; it changes when the agent records it again. `approve: true` approves the version with them.

**How far it got is observed.** A task is `open`, `working`, `handed_in`, `done` or `failed`. The agent's task list (the `tasks` fact, from the mod and from the hosted gate, through the one funnel `build_heard`) is matched to plan tasks by text. **The same words first; otherwise reworded close enough** (the task's words in a row inside the item's or the item's inside the task's, filler such as `the` and `to` left out, or at least 0.6 of the longer line's words shared, two words at least), **and an item says at most one task and a task is said by at most one item**, the best pairs taken first and an item no task wants ignored. **The agent is told its list is the plan**: the `record_plan` receipt (`task_list`), and the message for an approval (with or without marks), which lists the tasks verbatim, say `TASK_LIST_IS_THE_PLAN`, and the Planning prompt says it before the plan is recorded. A task the list calls in progress is `working`; one it calls completed is `done` if an edit was seen under its group's paths while it was worked, else `handed_in`, claimed and not observed. `failed` is not set by anything Fleet observes yet. An edit under a group's paths marks the group worked (`worked_at`); a group that names no paths takes every edit.

**With no plan, the agent's own task list is what Progress shows.** The list is kept whether or not a plan exists (*The build record*; `session_build_tasks`), so a Job that never records a plan still has one. Bridge lists it in the order the agent made the tasks, each with a state mark: `done` for a completed task, `working` for the one the tool has in progress, `open` for the rest. The running subagents are listed under it. The row is published when the list changes, so Progress follows live. **A recorded plan takes over**: with one, Progress shows the plan's groups and tasks and none of the agent's list, and the list goes on moving the plan's tasks by text as above. A Job with neither a list nor a subagent at work, and no workflow to follow, stands folded to its band.

**Who touched which file.** `SessionRecord.touches` is a bounded window of the Job's recent file touches for the Field: the subagent (type and the call that started it, absent for the Job's own agent), the `path` from the repository's root, the `act` (`write`, `edit` or `remove`) and `at`. The window holds 60; the same file by the same hand as the latest touch moves its instant and is not a new one. A hosted Job's touches are read off its process stream: the `Agent` call (once `Task`) names the type, and every line a subagent makes carries that call as `parent_tool_use_id`. A call whose start Fleet never heard (a Fleet that restarted) is named `subagent`. A terminal Job's come from the mod's `touched` fact, which carries the subagent's `agentId` (`by`) and the `act`; the type is read from the `subagent` attachment. A hosted Job loads the mod too, so only the stream is recorded for it. A removal is read from a plain `rm`, `git rm` or `unlink`; a command that removes by a pattern or a variable names nothing.

## The rail

Since 16 Oct 2026. A Job that follows a workflow has a **current step**, and it is **read off what Fleet observed, never off the agent's word**. `SessionRecord.workflow_step` is its index in the workflow's steps; it is derived each time the row is drawn (`rail.rs`, `Fleet::rail_drawn`), so nothing stores it beside the facts it comes from. It is absent for a custom Job, once the delivering step's pull request has merged with no step after it, and while `features.workflows` is off.

**Three anchors are exact.** A `records_plan` step is current until a plan is approved. A `follows_plan` step is current from approval, and stays until a later step is entered. A `delivers` step is current from the first push until its pull request merges.

**The steps between are read loosely**, each entered on the fact that fits it, only after every anchor before it has been passed:

| Step declares | Entered on |
|---|---|
| evidence `diff` | the first edit |
| evidence `test_suite_run` or `failing_test`, or a Manifest Check | a Check run the Job asked for, or the pull request's checks |
| the `delivery` phase | a commit |
| evidence `review` | a pull request opened |
| anything else | never by itself |

**Where a step cannot be told apart from the one before it, the earlier is kept**: a step that fits the same fact as its neighbour is not entered. A workflow edited under a running Job moves the indexes; the rail does not follow that.

**Every move is entered on the timeline** as `session.step`, `says` `step: <old> → <new>`, a step by its label, `start` before the first and `done` past the last.

**A skipped step draws one nudge, and nothing blocks.** Two skips are told, each once for each step in a Job's life: files edited before the `records_plan` step's plan was approved, and a push while the plan's tasks are open. Fleet sends the agent one short message from Fleet naming the step and what it asks (the step's own `about`, cut at 200 characters, else a default), and ends it with *Nothing is blocked*. A nudge that cannot be delivered because nobody is there is asked again later and not counted.

**With `features.workflows` off** (Settings, live; the owner's decision of 16 Oct 2026) there is no step, no nudge and no boundary judge, even for a Job that already has a workflow, and the first turn does not tell the agent the steps. The Job keeps the workflow it has. The proposer is shown none and never picks one, and `set_session_workflow` refuses to set one (`fleet.session_workflows_off`) but still clears one.

## The judges

Since 16 Oct 2026. **A judge is a stateless call**, on the Judge model setting (`judge.md`, *A Job's judge*), put about a Job's work. It is shown the request the person dispatched with (the Job's first message from him), the plan with the state Fleet observed for each task, the repository's standing rules and the diff against the base, and **never the transcript**. It is advisory: nothing it says blocks the Job.

**Two things run one.** The **tile** (`judge_session`, `POST /sessions/judge`) runs a judge now: it asks whether the work carries out the plan and stays inside the request, and beside that the `judge_checks` of the step the Job is on. It answers the row at once with `judging` true, and the reading lands on it as `judge` `{at, concerns}`; zero is how Bridge says *No concerns*. A **boundary judge** runs in the background when the current step moves past a step that has `judge_checks`, with those checks as the questions, once for each step. Only the step just left is read, not steps passed over. **Boundary judges are off since 16 Oct 2026** (`BOUNDARY_JUDGES` in `rail_watching.rs`), the owner's decision: most concerns were useless or too technical, and how a judge reads the work is to be reworked before any judge runs on its own. Only the tile reads.

**A concern is a call in the thread.** Each refusal the judge read becomes a waiting item, id `judge:<n>`, source `judge`, with its text (what was expected, produced and what follows, joined in one string), `concern.from` (the tile, or the step it came from), and `cites`: the files and lines of the diff its quotation landed on. A concern worded as one still waiting is not raised twice. **A call that fails or cannot be read raises nothing**, leaves `judge` as it was, and says why as an envelope line on Fleet's stderr.

**A concern is a claim, and its parts are kept apart.** The refusal's `expected` (what should be seen), `produced` (what was seen instead) and `consequence` (what the difference does) are carried on the wire, in `concern`, and in the store (`session_concerns`, three columns, empty where the concern was raised without them), beside `text`. **`text` is unchanged**: the one string `send_concern` sends the agent is still the joined refusal, so what he sends is what the judge wrote. A concern kept before the parts existed has none, and the card draws its `text` on its lines. The card heads with the consequence, then a labelled row each for Expected and Found, each clamped to two lines with a More press for the rest, which reads Less once open. **The Job's judge is asked to write each of the three as one short sentence** and not to restate the task, the plan or the diff (`BREVITY` in `session_work.rs`).

**The person decides what the agent hears.** `send_concern` sends it to the agent as his message (naming where it came from, the text and what it cites) and settles it; the message goes first, so a send that fails leaves the concern waiting. `dismiss_concern` settles it and tells the agent nothing. A settled concern leaves `waiting_for` and stays on the record in `settled`, a bounded window of the last 40, for the thread to draw as one line. `answer_waiting` and `dismiss_waiting` refuse a `judge` item, and sleep mode leaves it for the morning.

**Held in memory:** `judging`, as a Fleet that restarts reads nothing as reading. Kept in the store: the rail's last step, the steps nudged and judged, the concerns and the last reading (`session_rail`, `session_concerns`, `session_judge_reading`).

## What fills the context window

**A Job's context tank opens onto what fills it, the way `/context` shows it.** The mod asks the engine for a `summary` breakdown, estimated locally and never `full`, which would call the token-count API for every tool and file. It asks when the throttle lets a measurement through and the context moved, and sends `SessionRecord.context_breakdown`: `categories` (`name`, `tokens`, and a `kind` of `used`, `free`, `buffer` or `deferred` that Bridge branches on, never the name), `memory_files` (`path`, `tokens`), the ten `mcp_tools` that weigh most (`name`, `server`, `tokens`), `total_tokens`, `max_tokens` and `autocompact_threshold?`. **Names, paths and counts only**: no message text, prompt or file contents leave the mod. The live token count is the `context_tokens` already on `usage`.

## Where the tokens went

**Fleet keeps the usage of every turn, as well as the latest figures.** `usage` on the record stays the last `context_tokens`, `context_window` and `cost_micros`. Beside it, `session_turn_usage` holds a row for each answered message: its id, when, the model, input, cache-creation, cache-read and output tokens, and the names of the tools the turn called. A message reported twice is one row, and a repeat replaces it only with more output, because a harness writes the same message as it streams. `get_session_usage` returns a job's turns for a window of time, oldest first; adding the rows by model or by tool is the reader's. Names and counts only: no tool input or message text. `docs/practices/protocol.md`, *Usage a turn at a time*, has the wire, and says nothing reports a turn yet.

## A job that hit the plan's limit

**A job the plan's limit stopped is woken when the limit resets, and nobody has to poke it.** A turn that ends on `error: rate_limit` is reported by the mod as `limited`, with the instant the full window opens again: the latest reset among the windows at or near full, so a weekly window spent beside a five-hour one is waited out. A limit no window shows full leaves nothing to wait for, and is not reported. Fleet keeps the instant in `session_limit`, one row a job, and serves it as `SessionRecord.held_until` for Bridge to draw as `battery-low` reading "Limit reached · resumes 6:40 PM" (`hourglass` is banned; `packages/icons/icons/battery-low.toml`).

A minute after `held_until`, a tick of the notice loop sends the job `Continue` through `told_by_fleet`, the word Fleet already has for its own messages, and lets the hold go. The row is persisted, so a Fleet that restarts meanwhile still wakes it.

| Job | The wake |
|---|---|
| Hosted | A turn on its process, started again if it was ended for being quiet. The thread draws `Continue` as Fleet's |
| Terminal | Held for its mod as `Fleet: Continue` and submitted when the mod next asks. A mod that is not asking is tried again each tick, and given up on 30 minutes past the wake |
| Ended | Nothing is sent, and the hold goes |

**A job that took a turn in the meantime is not woken.** The hold goes with a finished turn, with a message from the person, with a hosted turn starting and with the job ending. The turn the limit ended is not a finished one: the mod does not report a turn that died on an API error as `turn_completed`.

**The mod keeps no timer of its own.** A terminal job is woken by Fleet through the path it already asks on; a mod timer would wake it again after a Fleet restart that had already done so.

## What Fleet tells a Job

**A Job never has to be told to watch its pull request.** When a pull request a Job holds fails its checks, conflicts with the base, leaves the merge queue, is marked unmergeable, or merges, Fleet sends that Job one message, once. It names the pull request, what happened and the branch. For failed checks it lists each failing check with its log address, and the last lines of the first two logs. A merge tells the Job to call `finish_up`, which releases its slot and deletes the merged branch, and never to delete the branch by hand or ask the owner to: ending the Job parks its slot and keeps the branch. `docs/concepts/fleet.md`, *Telling the owner of a pull request*, has the triggers.

| Job | What happens |
|---|---|
| Hosted | The message is a turn: it wakes the Job and starts its process if it was ended for being quiet. The thread draws it as a message from **Fleet**, the voice another Job's message has, named `Fleet`, so it is never the person's |
| Terminal | Fleet cannot push into one. The message is held for its mod, prefixed `Fleet:`, and handed over when the mod next asks. A mod that is not asking leaves it unsent and Fleet tries again at the next reading |
| Ended | Nothing is sent |

**A Job is told when what it wrote would conflict.** When Fleet finds that the files a Job wrote conflict with another Job's, or with the base after it moved, it sends that Job one message: the files, who the other is and how to reach it. The Job is not paused and the message is sent once per set of files. `docs/concepts/fleet.md`, *Write clashes*. Bridge marks the Job's row and puts the swords glyph on its Bays key, and the Bays panel names the other holder and the files, from the standing clashes Fleet lists. A file in that list opens as a sheet of the Job's change to it and the other holder's, each from where the two split, with the hunks that collide marked in both; Fleet reads the two patches from git when the file is pressed (`read_clash_file`, `docs/concepts/fleet.md`, *Write clashes*). A Fleet that cannot read one leaves the sheet with a sentence and no diff.

| The other is | The message names |
|---|---|
| A Job | Its address, `s-` and the first eight characters of its id, and its branch, and says to use `SendMessage` to that address |
| The base | The commit on it that changed the first file, and says to merge it into the branch |

A terminal Job reports the files it wrote through the mod's `touched` fact, as paths from its directory.

**A Job that held a slot is told when the slot is stranded.** When a slot a Job held last has work in it and nothing holds it now, Fleet sends that Job one message, once. It names the slot and its branch, what the slot holds, and asks the Job to commit, push and release it. `docs/concepts/fleet.md`, *Rescuing a stranded slot*.

**A Job whose slot is far behind its base is told to merge it.** When the checkout a Job holds is 20 or more commits behind the base, Fleet sends that Job one message: `slot-4 (branch armada/x) is 684 commits behind main. Merge main into your branch before your next push.` The count is the pool's own `behind`, and 20 is the figure Bridge's slot row starts from. It is sent once. The Job is told again only after the slot has fallen below 20 and then behind again, and a Fleet that restarts may say it once more. A Job's slot, a free slot, a stranded slot and a subagent's bay are not told, and the owner is not called: merging the base in is the agent's work.

## Acts on a pull request

A pull request a job holds is a `pr` row, and a person can act on it without leaving the job. The pull request is named by its repository and number.

| Operation | Does | Refuses |
|---|---|---|
| `get_pull_request` | Reads it: draft, open, merged or closed, auto-merge, the forge's checks (`pending`, `passed`, or `failed` with the failing names), title, branch, address | Where the forge would not answer |
| `ready_pull_request` | Takes a draft out of draft | Anything that is not a draft |
| `merge_pull_request_by_number` | Merges it | While the checks are running or failed, a draft, a closed one, and the forge's own refusals |
| `enable_auto_merge` | Asks the forge to merge when the checks pass | Once they have passed or failed, a draft, a closed one |
| `get_pull_request_detail` | Reads the page the panel draws: base, author, mergeability, review state, the conversation, the forge's checks with their run time, files and commits, and the head, the description and the diff's review threads | Where the forge would not answer, and where it would not give the threads |
| `get_pull_request_file` | One file's patch, read when a person presses a row of the Files tab. Bridge draws it in a panel to the left of the pull request's | Where the forge would not answer, and for a file the pull request does not list. A file the forge sent no patch for answers with Fleet's sentence |
| `write_pull_request` | Comments, approves or requests changes, then reads the page again. With line, file and reply comments it is one review | A comment or a request with no body, any comment with a blank body, a Job the ledger never heard of, and the forge's own refusals |
| `resolve_review_thread` | Resolves a conversation on the diff, or opens it again. Bridge only, see `docs/concepts/pull-review.md` | The forge's own refusals |
| `get_review_lists` | Reads the pull requests waiting on a person, as four lists. Bridge only, see `docs/concepts/pull-review.md` | A forge that would not answer any list |

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
> Why: a Job's `pr` row is what the harness reported, and `detail` (`state`, `auto_merge`, `checks`, `failing`, `title`, `branch`, `address`, and `body` where the author wrote one) is Fleet's reading of the same pull request, so one writer is not two.

**Only a failed required check is `checks: failed`.** Where a repository names a gate (`ci`), a check outside it that is red while the gate runs reads `pending`, and one red after the gate passed reads `passed`, with its name still in `failing`. Bridge draws those two as a tinted chip and leaves the Job's row alone; only `failed` puts a Job under Idle with a red mark, and a Job that waits on the person is under Needs you whatever its checks say. Fleet refuses a merge on all three.

**The three writes are a person's ask, `pushes to shared`.** The operations are `Helm only`, and Helm calls them on a person's ask. Bridge's own buttons are the person's press and reach the routes directly. `get_pull_request` is open to any agent.

**A press is drawn as underway until Fleet has answered and the pull request has been read again.** The pressed button takes Button's pending rendering with a turning ring before its label, the other acts are disabled so a second cannot start, and a refusal is said above them and the button returns. Merge, Enable auto-merge and Ready for review show the state the re-read brings.

## A forked job

A job that is **ended or dead** offers Fork, in place of its message box. `start_session` takes `fork { session_id }` (23.69) and starts a new job hosted by Bridge as a copy of that conversation. Spike 28 measured the agent side.

| | |
|---|---|
| Dead | The ledger says `ended`, or it is a terminal job whose mod has not asked for ten seconds (`terminal.listening` is absent). A hosted job is dead only once it is closed |
| Refused | A live job, 409 `fleet.session_fork_live`. An unknown id, 422 `fleet.no_such_session`. Nothing is written |
| The fork | Its own id, ledger and slot. It takes the old title unless one is given, starts in the repository's main checkout, and leases a slot at its first write like any other job. It holds none of the old job's slot, branch or pull requests |
| First process | Resumes the old id as a fork under the new one, so the conversation is copied and the new id is Fleet's. Once it has run, the job resumes by its own id |
| Ledger | A `forked_from` row on the fork and a `forked_to` row on the old job, both `spent` |
| Told | **Every process of a fork has a note in its system prompt** (`--append-system-prompt`): forked from <title> (<id>) at <time>, that job is live or has ended, and nothing it started is the fork's. Its subagents, background tasks and shells, slots, branches, windows, pull requests and peer jobs stay its own, and a task-notification about its background work is not acted on, relaunched or reported. A fork launch cannot be built without the note (`HostedLaunch::forking` takes it) |

> **Rule.** The whole of a job is forked only once it is ended or dead. A live one is forked at a message, below.
> Why: a copy of the whole of a conversation still being written is two jobs racing to the same next message, and a job that can still be spoken to needs no copy.

### A fork at a message

**Words selected inside one message offer Fork from here**, on a live job as well as a dead one. `fork { session_id, at }` (23.79) copies the conversation up to and including the message whose thread row id is `at`, and nothing after it. The old job is not touched: it keeps its process, its slot and its next turn, and gains only the `forked_to` row. The fork is hosted by Bridge as any fork is.

| | |
|---|---|
| Refused | An `at` no message of the thread has, or one the agent's conversation does not hold, 422 `fleet.fork_point_unwritten`. Nothing is written, and the line is looked for before the fork is made |
| Ledger | The `forked_from` row's detail carries `at`, the row Bridge cuts the folded old rows at, and `cut`, the transcript line Fleet gives the agent. The detail is the ledger's, so a Fleet that restarts before the fork's first process still has the cut |
| First process | As above, with `--resume-session-at <cut>`. Only that process is told it; the fork's own file holds the cut after |
| The line | A terminal job's row ids are its transcript lines' `uuid`s (`<uuid>.<n>` for a later row of one line), so the cut is the row id without its suffix. **A hosted job's rows carry no `uuid`**: Fleet minted their ids and the `Said` events the stream is read into drop the line's. Fleet lays the thread's messages over the transcript's from the end, matching the agent's by its words and a person's by being inside the line Fleet wrote (it adds a mention line and attachment paths), and the cut is where `at` lands. A message with no words, or none that lands, is refused: it is not guessed |

The agent side is spike 30. A cut at a message the agent is part way through a tool call on is repaired by the CLI.

**Why a note, and not a cleaner copy.** The CLI copies the conversation, and the conversation says the old job launched subagents, shells and a slot; the CLI then replays a notification for each of them that had not finished ("didn't finish before the previous job ended", "failed", "stopped") into the fork. Stripping the notifications from a copy of the transcript would leave the launches that caused them in the words the fork reads, and the copy would be a file Fleet writes into the CLI's own folder in a format Fleet does not own. The fork is told instead, in the one place it cannot read past. Found 13 Oct 2026: a fork of a live job relaunched the old job's builder into the old job's slot, and showed the owner the old job's windows as its own.

**Bridge** keeps every job that ended in the last seven days (by when it was last seen) beside the live ones, under an Ended heading (a terminal job that has not ended but whose mod is not asking is Quiet, its own heading above it, and still offers Fork), and finds an older one by search, and draws a dead job without a message box and with Fork in its head. It owns nothing: its slots, branches and pull requests are not offered to a chip, and it is not offered to `@`. A press on Fork asks Fleet, and Bridge opens the new job once it holds it. The ledger draws a Forks section only where a job has a row for it.

**A terminal that comes back is live again.** The mod asking again flips `listening`, which Fleet publishes as `session.changed` while a window has the thread open. Fork on a job whose terminal woke between the press and Fleet is refused as live. After a Fleet restart every terminal job reads dead until its mod's next ask, about two seconds.

## A moved job

**A Job started in the wrong repository is moved, not started again.** `move_session { session_id, manifest_id }` (`POST /sessions/move`) ends the Job and starts its whole conversation again in another served repository, under the same title, and answers the new Job's record. It is a fork across repositories, in one call: `start_session`'s `fork` with no `at`, after the old Job is ended.

| | |
|---|---|
| Refused | Before anything is ended or written. An unknown Job, 404 `fleet.no_such_session`. A repository Fleet does not serve, 404 `fleet.no_such_manifest`. The Job's own repository, 409 `fleet.session_move_same`. A terminal Job whose mod is still asking, 409 `fleet.session_move_listening`: it runs in the owner's terminal, which Fleet cannot move |
| The old Job | A live hosted one is closed as Close closes it: its process ends, its slot is parked (what is in it is committed to its own branch, nothing is pushed) and its branch is kept. An ended one, or a terminal one whose mod stopped asking, is left as it is |
| The new Job | A fork: its own id, ledger and slot, in the target repository's main checkout, leasing a slot at its first write. It keeps the old model, effort and mode |
| Ledger | The `forked_from` and `forked_to` rows of *A forked job*, `spent`, with `detail.moved` on both: the repository the Job came from on `forked_from`, the one it went to on `forked_to`. `forked_from` also carries `detail.branch`, the old Job's branch or branches, where it had any. A fork has neither, which is how Bridge tells a move from a fork |
| Told | **Every process of the new Job has a move note in its system prompt** in place of the fork note: moved from repository <root> to repository <root> at <time>, its conversation is a copy of <title> (<id>) which has ended, it now works in the second. Its slot, its branch (<name>) and the files it had changed stayed in the first. What the old Job started is not its own, a task-notification about it is not acted on, relaunched or reported, and it does not write in the first repository. The note is read from the ledger at each start, so it survives a Fleet restart |

**The conversation follows the directory.** The CLI keeps a conversation under the folder of the directory it ran in. The new Job's first process starts in the target repository's checkout and `bring_conversation_to` links the old transcript, found by id wherever it is, into that folder before `--resume <old> --fork-session` runs, so the copy is found across repositories.

**What stays behind is not moved.** The old repository's slot, branch and changed files are the old Job's, and the old Job is ended; the branch is still there for a person or a later Job to pick up. Moving a Job does not carry its pull requests.

**An agent may move its own Job** (`agent_access` Yes). Its process is the one that ends, so the answer never reaches it; the new Job is listed and Bridge opens it. The call names the Job, so an agent could name another's; it is held to its own by the instruction, not by Fleet.

**Bridge draws it from the ledger alone.** A Job's repository is a chip in its head, on the flight deck's stage, and where Fleet serves `move_session` it opens the other served repositories; a pick asks in the same layer, as a caution, what stays (the branch and its files) and, for a live Job, that it ends. A `forked_to` with `moved` is "Moved to" and a `forked_from` with `moved` is "Moved from", each a row that opens the other Job: the first ends the old thread and the second starts the new one, whose conversation is the old one's rows read in place and not folded as a fork's is. A Job with a `forked_to` that carries `moved` shows its repository as text, so it is moved once. A terminal still listening has every repository drawn off with the fact on its tooltip; Fleet's refusals are the composer's toast (`fleet.session_move_same`, `fleet.session_move_listening`, and the two 404s).

## A Job an agent started

A Job's agent may start a Job, or fork its own conversation into one, through two tools on its door (23.80). The owner approved it on 12 Oct 2026 on the condition that **the new Job waits for him**.

| | |
|---|---|
| `start_another_session` | `manifest_id`, `title`, `message`. A new Job in any repository Fleet serves |
| `fork_this_session` | `title`, `message`, optional `at`. A copy of the caller's own conversation, to the row `at` or to its latest message of its own that the transcript holds. Allowed while the caller is live, since it is a fork at a message |
| The caller | Placed by the connection of its hosted process, as `show_window` places it. **No call names a Job**, so an agent cannot fork another's conversation, and a call from no hosted Job is refused 422 `fleet.agent_start_unplaced` |
| Not offered | Tune, close, send and every other hosted-session command stay `agent_access` No |

> **Rule.** An agent's Job is created and listed at once, and nothing runs until the owner sends its first message.
> Why: a Job that started itself is a model spending tokens in a repository nobody chose to open. Holding the message makes the start a proposal he reads and may edit.

**The first message is kept as an `unsent` attachment** on the new Job, and read back on every read as a waiting item: id `unsent:<session id>`, text *First message*, source `first_message`, act `message` on the Job. It has its own source and act so the Cockpit draws no reply box for it and a press opens the Job. Bridge puts the words in the new Job's message box once, where the box is empty. His ordinary send (`send_session_message`) sends what the box holds, starts the first turn, and spends the attachment and so the item; editing first is the same call. `answer_waiting` refuses the item, since sending is the answer. Ending the Job discards both. While sleep is on, the item is left on the list and not decided for him.

**The brief is read at creation**, so the proposal (title kept as the agent gave it, workflow, model, effort) is on the new Job before he sends, and his send goes out on it. A fork is not read (*A Job's proposal*).

**Both ledgers record it.** A start writes `started_by` on the new Job and `started_to` on the caller; a fork writes the `forked_from` and `forked_to` of *A forked job*, with `at` and `cut`. The caller's thread gets a `started` row naming the Job and whether it is a fork, which Bridge draws as the press that opens it. The new Job's thread opens under a line naming the Job whose agent started it.

**Nothing bounds how many an agent starts.** Each waits for him, so none spends anything, but a loop would fill the list. A limit waits for one being seen.

## Linked Jobs

A Job splits its work into **linked Jobs it follows**, one for each group of its plan. The owner approved the design on 16 Oct 2026 (walk `linkedSessions`), with the groups on the Plan and a Linked section in the Ledger both drawn. The Plan is Progress now and the Ledger is gone from a Job's view; the same Jobs are a group in the Artifacts layer (*A Job open*).

| | |
|---|---|
| Handing a group | `start_another_session` with `group` (the agent's, held for the owner like any start) or `hand_to_session` (the owner's press on **Hand to a Job**, which sends the brief). The new Job is in the same repository, titled with the group's title |
| The brief | The group's tasks word for word, its paths, then the parent's handoff: its title and id, slot, branches, open pull requests and the last six messages of its thread. The agent's own `message` follows when it gave one |
| The link | `started_by` on the child and `started_to` on the parent, as for any started Job, with `detail.group` on both. A group is handed once (409 `fleet.group_handed`), and a group the plan lacks is refused (422 `fleet.group_unknown`) |
| The parent keeps track | When a child **needs the owner** (a waiting item other than its unsent first message, a question or permission held, or stuck), **ends**, or has a **pull request merged**, Fleet sends the parent's agent a message, which wakes it when idle, and writes a `linked` row (`event` `needs`, `done` or `merged`) in the parent's thread. The message is news, so it leaves the agent's `all_done` standing. Each is told once |
| Every Job started from another | Is followed in that way and stands under its parent in Work, whether or not it was handed a group. Progress and the Artifacts layer draw only those with a group |

> **Rule.** A Job follows every Job its agent starts, and the owner hears of a child through the parent's thread where it happened.
> Why: the parent is the one holding the plan, and a child that waited on him with nobody saying so was how work stalled.

Bridge draws it from the wire, with no state of its own:

| Where | What |
|---|---|
| Progress | Under a group of the plan handed to a Job, that Job's row of two lines: the `link` glyph and its title, then in the mono muted line its state mark and its pull request chip with the checks mark. A group not handed has **Hand to a Job** in its open body |
| The Artifacts layer | A Jobs group: each child's title, the group it came from after it, its state mark and its pull request with the checks mark. A press opens the child, and Esc is Back to the parent |
| The parent's thread | A `linked` row where it happened, `eye` for needs, `power` for done, `git-merge` for merged, the press that opens the child |
| Work | `docs/concepts/cockpit.md` |
| Keys | Enter on a row opens the child; Esc is Back, to the parent's thread and open groups as they were |

A Job that is idle when Fleet starts and already needs the owner is not told: `needs` is read as a move. A notice for a parent that ended is dropped.

## A Job that waits on another

A Job can be dispatched to **start when another Job reaches a point**, with that Job's handoff first. The owner approved the design on 16 Oct 2026 (walk `sessionAfterAnother`).

| | |
|---|---|
| What waits | A Job with no message sent: it holds its brief, takes **no slot and runs no process**. The wait is a fact on its record (`waits`), kept in `session_waits` and absent once it has started. It stands in Work under the Job it waits on, in the state `after` |
| The condition | `ends` (the default), `group_done` (a group of the plan of the Job waited on), `task_done` (one task), `pr_pushed`, `pr_lands`. Fleet **reads it from what it observes**: the Job's state, its plan's progress and its pull request events, never from an agent's call. The first pull request seen is the one `pr_pushed` and `pr_lands` mean, and it is kept |
| Where it starts from | `main` (the default) or `branch`, the branch of that pull request. Dispatch defaults `pr_pushed` to `branch` |
| The start | Once, when the condition is met: the first message is the **handoff of the Job waited on** (the builder `hand_to_session` uses) followed by the brief. A restart cannot start it twice |
| Who sets it | Dispatch, from a brief that tags a Job with `@` and the words after it, read by the proposer (`read_session_wait`); or an agent's `start_another_session` with `after`. A reading that cannot be had is the defaults: it ends, from `main`, asking nothing |
| The Job waited on | Is told what the waiting one needs, once, and woken if idle, as any news. It may change the condition or the base with `change_waiting`, which is allowed only for a Job that waits on it; the owner may change any. Each change writes a `waiter` row in both threads. The owner can start a waiting Job at once (`start_waiting_now`) |
| Refused before anything is made | The Job waited on is unknown, is the Job itself, has ended, or has no such group or task in its plan |

> **Rule.** A waiting Job starts on what Fleet observed, and what it waits for can be changed by the Job it waits on.
> Why: the Job doing the work knows when the point is reached and what is needed of it, and a hand that cannot move the condition leaves a Job waiting on a point that will not come.

The owner's Job sends that message when the condition is met. An agent's waiting Job holds it for the owner instead, as any start an agent makes, so nothing runs until he sends it.

Bridge draws it from the wire, with no state of its own:

| Where | What |
|---|---|
| Dispatch | The sentence over the request, `Starts when [Session] [condition] · from [base] · needs [..]`, filled by the proposer's reading: the defaults at once, its answer when it arrives, and a send waits for the answer. `⌥A` opens Dispatch with `@` typed |
| Work | The waiting row's state line says when it starts. `docs/concepts/cockpit.md` |
| The waiting Job | Its brief, who it waits on, the condition (changeable), what it needs and where it starts from, and **Start now** |
| The threads | A `waiter` row: in the thread of the Job waited on when it is dispatched, changed or started; in the waiting Job's own thread when its condition is changed |

## In Bridge

Bridge reads every live job from `list_sessions` once per connection and keeps it by `session.changed`, whole, so a chip anywhere asks the same list who owns its branch, slot or pull request, across every repository. A thread is read with `get_session` when a window first opens it, then followed by `session.row`. The renderer reaches both through the draft seam (`SessionsDraft`): the mock fills it from fixtures and a real window from what main publishes, so a screen cannot tell which.

> **Rule.** A job started in Bridge starts in the repository the window has picked, or the one repository Fleet serves.
> Why: Dispatch names its repository the same way, and a job started somewhere the person is not looking is one they cannot find.

| Bridge draws | From |
|---|---|
| The ledger's slot and branch as one pair | A branch row ends in the key chip of its slot, so a slot a branch is on has no section or row of its own. A slot no branch is on keeps a row under Branches, after the branches, with the key glyph. A press on either opens the slot on Cleanup. A `slot` of 0 on a branch is unknown and draws no chip |
| A pull request, its Checks and its acts | The `pr` row's `detail`; each press is one of the pull request operations above, and `read` brings the row current when a job is opened |
| The ledger's sections (a Job with no `work`, drawn as it was) | Pull requests, Branches, then the rest; only a kind that holds a row is drawn. What waits on the person is not among them: the Needs you panel on the composer holds it in every state. A ledger with nothing on it draws one small picture and no words |
| The ledger's bar | Four icon buttons across its top: Search, Pull requests, Branches and slots, Artifacts. A kind narrows the ledger to its section, one at a time, and a kind with no row is disabled. Search narrows every section to the rows whose name or text holds the words, and its draft is kept per Job. An empty ledger draws no bar |
| Artifacts | The `artifact` rows, one section with a glyph per form (`globe`, `files`, `image`, `notebook-text`) and a tooltip naming it (Published page, File written, Looked at, Doc). A file opens through main, which opens only a path the job's own ledger names as a file it wrote or a picture it looked at |
| The ledger beside the thread | The ledger is its own panel beside the conversation, headed "Ledger" with a button at its trailing edge that hides it. While hidden, the conversation's header holds the button that shows it again; the choice is the window's, kept in its storage. Below the breakpoint the header's one button opens the ledger as a sheet instead |
| A sketch the person drew | The picture it was sent as, and the drawing Bridge kept for the ledger. The wire holds only the picture |
| The context tank | The head reads `context_tokens` and `context_window` from `usage` as a pill of live tokens and the percent, `53.8k · 5%`, filled to that percent and coloured normal below 80, warning from 80, danger from 95, with a faint dotted line at `autocompact_threshold` where the breakdown holds one. Where `context_breakdown` is held it is a button: hover or focus opens a card, a press keeps it open, Esc or a press outside closes it. The card is an inspect card (`docs/contracts/design-system.md`, Inspect card): a state band `5% used`, the tokens over the window, a slim bar of the categories in the window, one fact a category, then the three heaviest memory files and MCP servers (a server's tools summed). Where none is held it is a figure with a tooltip and no button |
| The Loadout | `loadout_summary` draws a quiet `layers` mark after the context tank, absent until the Job reports one. Hover or focus opens an inspect card: Knows and Told counted, and what the latest turn used, each with where it came from (`Kit`, `Armada`, `Repository`, the plugin's or MCP server's own name, `Built-in`, `Managed`) and a subagent's model. A use repeated in its turn is one row with `× 40` after its name; it is left off a use heard once. A press, or Show all on the card, opens a floating sheet read from `get_session_loadout`: Knows and Told by kind, then Used by turn, latest first and open, one turn open at a time. A Source select narrows all three. Only an MCP server that failed (`unplug`) or is connecting (`clock`) draws a state mark. It is read again whenever the summary moves. **Every row is a button**, under Knows, Told and each turn's uses: a press opens that item in a second floating sheet to the sheet's left, placed as a pull request's file is beside its panel, its name over its source and the path it was read from. It is read with `read_loadout_item`; markdown draws as Prose, a SKILL.md's front matter as a fenced block, and text in monospace. A body Fleet cut says so in one line, and an item Fleet did not read shows its `why`. An MCP server reads nothing and lists its tools from the Loadout, each Loaded or Deferred. Another press replaces it, the open row is `aria-current`, and Esc or Close closes it with the Loadout still open. The walk is `session-loadout` |
| A running Job's line in the list | `activity.now`, one line under the title. Nothing is drawn for a Job whose turn is not running, however old its last `now` is |
| The thread's Working on band | `activity.todos`, over the message box, in the order doing, to do, done. Folded, a task mark a todo, each named on hover with its words (`TaskMark`'s); a press opens the rows. Left off when the Job says it is done, and when the list is empty |
| A Cockpit tile's line | The todo being done, else `activity.now` |
| What waits on him | **The Needs you panel**, a band on the composer that opens as a list over the thread: the question or permission (and the question that lapsed) first, then a judge's concerns, then what the agent and Fleet say waits. A row opens its one thing to answer. A plan to approve stays the thread's last card, with a chip over the composer (`Plan`) that jumps back to it when scrolled away. Nothing is pinned and nothing moves as the thread scrolls. Working on stays under the thread; the Job end band takes the composer's edge only when nothing waits, and a press opens it to what was made and End job or Keep going. The owner, 16 Oct 2026: "I am really not liking these new scrolling question panes"; 17 Oct 2026, the band, on `?walk=needs-you-panel` and `?walk=session-end-band`. `.claude/decisions/2026-10-17-what-waits-on-him-is-a-band-on-the-composer.md` |
| A job held for the plan's limit | `held_until` draws `battery-low` in place of the live state mark, on its list row, its Cockpit tile and its header, with the tooltip `Limit reached · resumes 6:40 PM`. Nothing animates |
| An ask from the phone | A hosted job's held ask is answered through Pocket, with the same `answer_session_ask`. A terminal job shows there as waiting, and while Fleet holds its question the phone answers it the same way; after the hold it is answered in its terminal. [Pocket](pocket.md) |

## A Job open

Since 16 Oct 2026. The owner, 16 Oct 2026, short of room: Progress, no Ledger, more Flight control keys, and an Artifacts view for "everything that was created or used during a session". Approved on `?walk=compactSessionView`; the calls were the thread's last card (`?walk=compactCallsInThread`) until 17 Oct 2026, when they became the Needs you panel on the composer. `docs/concepts/cockpit.md` has the layout.

| On screen | From |
|---|---|
| The thread, and its last card | The plan awaiting approval. A question, a judge's concern and the rest are rows in the Needs you panel on the composer |
| The Field | `work.field`, only the Job's own drone and the subagents at work: one whose `subagent` row is `spent` has left it |
| Progress | The workflow's steps as a line of glyphs, each a hover card (`Build · current`), and the plan's groups and tasks under it. It is the old Status and Plan as one panel |
| Flight control | The Job's own tiles; the last four open a layer over the thread |

| Layer | Key | Reads |
|---|---|---|
| Artifacts | `8` | The Job's rows as they stand: `pr`, `artifact` (pages, files, docs, images), `sketch`, `subagent`, `background`, `studio` and `started_to` (the Jobs it handed a plan group to, each with the group and its state), the files its drones touched (`touches`), and from the thread the commands it ran and the commits it made. **The wire holds no commit record, so a commit is the thread's `git commit -m` line**, not Fleet's observation,. A field at the top narrows every group as it is typed in |
| Subagents | `9` | Every `subagent` row, the running ones first. A press opens its own thread |
| Checks | `0` | The `check` rows of the thread, newest first. A press opens the run on the Checks page |
| Bays | none | Every bay the Job holds, or held last before it stranded, each with its lease and how far it is behind its base, and the Job's write clashes: `slot-trouble.ts`. **Lit only when a bay is stranded or 20 or more commits behind.** A write clash puts the swords glyph at the key's corner and does not light it. A stranded bay is also a call in the thread's last card. A bay behind and a write clash are not: Fleet tells the agent, so they leave the owner nothing to do. Off, with its reason, where the Job holds no bay |

A layer covers the thread and the composer, which stay mounted under it: Back (the key again, the button, or Esc) returns to the thread at the same offset with focus where it was. The arrows walk its rows, and focus goes in when it opens, to the search field in Artifacts and to the first row elsewhere. Esc is the layer's while one is open, and a sheet over it keeps its own. The layer is dropped when another Job is opened.

## Which model, and a fresh start

**A Job's composer names the model it runs on.** Fleet launches `claude` with `--model` only where a person picked one, and the CLI's own default applies otherwise, so the pick alone says nothing. Fleet reads the model off each `init` line of the stream (`DroneEvent::Started`, one per turn) and carries it as `HostedFacts.resolved_model` (23.85): `claude-opus-5-5`, which Bridge names `Opus 5.5`. **It is cleared when the process is let go** and absent before the first turn, because the next process has not said. The composer's Model select reads it: the selected option, the picked alias or the default, is labelled `Opus 5.5` once the resolved model is of that option's family. Until it has, or after another model is picked, the select reads the alias or the default as it did, **never a guess**. The head draws no model. A terminal Job is named only where its mod reported a model.

**Past 300k tokens of context or a day since it started, a quiet mark (`move-right`) sits after the context tank.** Hover or focus opens its card and a press keeps it open, as the context tank's does: a fact `Context` of `612k tokens`, `Started 3h ago` in its head, and two acts. **Fork** forks the conversation at its latest row (`start_session` with `fork.at`, 23.79). **New Job** asks the old Job for its handoff, which is a message of the person's sent with `send_session_message`: the goal, where it stands, open pull requests and branches, and what is next. Its next reply, once the turn is over, is the handoff, and the new Job opens under the same title with `Handoff from <title>` and that reply as its first message, sent for the person. While the old Job writes it the mark pulses and New Job waits. **If it ended, the send was refused, or no reply came within 90 seconds**, the first message is the ledger's facts instead: the title, its slot, its branches and its open pull requests. Nothing is typed. Both are Bridge's alone (`session-handoff.ts`), over operations that already exist; the cost is one turn of the old Job, which re-reads its whole context once.

## A Job's proposal

**A Job is read once**, by the model call the proposer makes (`job-proposer.md`, *How a Job's request is read*), and what the call settles is the Job's own. `StartSession.request` carries the words. The press never waits: the start answers at once with the row, `proposal.reading` true and no process, and **the process starts when the answer lands**, on the title, workflow, model and effort it settled. A first message sent meanwhile is written to the thread at once and its turn is held, in the order the messages came. A failed, late or unreadable call starts the Job on the machine's defaults.

| On the record | Meaning |
|---|---|
| `workflow` | The one workflow the Job follows, or absent: **a custom Job**. Never a default |
| `proposal.reading` | The call is still out |
| `proposal.picked` | The fields the proposer filled (`title`, `workflow`, `model`, `effort`), each dropped when the person sets that field |

**Which starts are read.** Every Job is read once, at the first message that is its brief. A start with a `request` (the dispatch sheet) is read at the start. Any other start is read at the first message it is sent or given, and that turn is held behind the reading exactly as a dispatch's is.

| Start | Read | When |
|---|---|---|
| Dispatch sheet (`request`) | Yes | At the start |
| Annotations, Lessons, Hand to Job, any Bridge start with no `request` | Yes | At the person's first `send_session_message` |
| `start_another_session` (an agent's start, with a plan group or a wait) | Yes | At creation, from the agent's brief, so the proposal is in the dispatcher before he sends. His send does not read it again |
| A fork (`fork_this_session`, a fork in `start_session`, red main's fork) | No | It carries its source's conversation, model and settings |
| Helm, and Setup's Job | No | Helm is a conversation with fixed values, not a brief; Setup has no Manifest to read against |
| Autopilot, Rescue's continuation | No | Fleet's own dispatch: its brief is fixed, and Autopilot has picked the workflow |
| Fleet's words to a Job (`told_by_fleet`) | No | A notice is not a request |

**Only the first.** A Job is read when it has not run, holds no proposal and has not been asked; the mark is in memory, so a Fleet that restarts before the first turn may read once more. What the Job already carries (a title, a model, an effort, a workflow) stands over the answer, as it does for a dispatch. No proposer, or a failed one, starts the Job as it is.

**What a person sets wins, and an answer never overwrites it.** A field given on the start, or changed in a panel while the call was out, stays as set.

**A change reaches the agent, because it cannot read the panel.**

| Changed | What the agent is told | When | Timeline |
|---|---|---|---|
| The workflow (`set_session_workflow`) | A message from Fleet naming the workflow and its step labels, or saying it is now a custom Job | At once: mid-turn at the next tool call, and an idle Job wakes. A Job that has not run is told with its first turn | `workflow: <old> → <new>` |
| The model or effort (`tune_session`) | A line ahead of the turn that starts on the new values: `from this turn your model is X, not Y` | On the next turn, with no wake | `model: <old> → <new>`, `effort: <old> → <new>`, entered when that turn starts |

**An unchanged model tells nothing.** Fleet keeps the model and effort each turn ran on, and the first turn only sets the baseline, so a restart does not tell again. The three events are `session.workflow`, `session.model` and `session.effort` on the timeline, where committed and pushed rows come from.

**Held turns are in memory.** A Fleet that restarts while a Job is still being read loses them; the message row stays on the thread.

## A queued idea

**A Job can be kept as an idea that does not start.** The dispatcher's Job kind offers Queue for later beside Start (`queue_session`, `⇧⌘↵`, rebindable), and its Model and Effort selects set what the idea will start on. Pressing it clears the draft and keeps `{ text, root, repository, model?, effort? }`. It is not a Job: no row on Fleet, no slot, no process, no proposal call.

**It lives on this machine, in Bridge.** The ideas are kept in `localStorage` (`armada.bridge.queued-ideas`, `queued-ideas.ts`), oldest first, since Fleet's preference set is closed and holds nothing of the kind. Another Bridge window hears the change through the `storage` event. A restart of Bridge or of Fleet keeps them.

**A queued idea sits at the head of its repository's band on the Cockpit deck's list**, dashed, with two acts and no other: Start and Remove. A click on its words starts nothing; `↵` on the row starts it.

**Start sends the idea through `StartSession` like any start**, with `request` the idea's words and `model` and `effort` only where the idea has them, so Fleet's proposer fills what is unset and never overwrites what was picked. The idea is removed once the Job's id exists, and a start that fails keeps it. The words go to the new Job as its first message.

## Stopping a turn

**A Stop sits beside the working mark at the end of a hosted Job's thread**, and the same press is available wherever that mark is drawn with a turn running. It sends `stop_session` (`POST /sessions/stop`, 23.84). Fleet ends the process as it does after a quiet spell, withdraws a question the agent was held on, sets the turn to `idle` and publishes the row. **The row stays live and the slot stays held**: nothing is parked or committed, which is what separates it from `close_session`. The next message resumes the conversation. The thread keeps every row the agent had written, and Stop draws no row of its own.

**A terminal Job has no Stop.** Fleet does not host its process and refuses the act.

**Debug Bridge starts a Job with no turn.** `start_session` writes the row and runs nothing, so Bridge starts one for the note and leaves the note and its picture in the message box as a draft (`annotate/debug-draft.ts`). The first turn is the person's Enter, with whatever he added. `docs/practices/running-locally.md`, *Annotating Bridge*.

## Pausing a Job

**A pause gives the slot back and keeps the Job.** `pause_session` (`POST /sessions/pause`, 23.92) ends the running turn as `stop_session` does, commits what is in the Job's slot to the branch the checkout is on as a `WIP:` commit (nothing is pushed; a checkout on no branch or on the base gets a `kept/` branch), and frees the slot for the pool. The row stays live in the list and carries `hosted.paused` (`at`, `resuming`). It is not a state: `state` is `live` and `hosted.turn` is `idle`. The ledger's `slot` row is given back and its `branch` row stays, since the Job still owns the branch. It is the Job pause (`job.md`, *Pausing a Job*) for a Job, and it reuses the same park.

**A resume takes a slot at the branch tip.** `resume_session` (`POST /sessions/resume`) leases the branch into a free slot and clears the marker. It starts no turn. **It never forces another holder out**: where every slot is held, or Jobs already wait, the Job joins the repository's line and the row reads `hosted.paused.resuming` until the first slot to free is leased to it. A pause while it waits calls the wait off.

**A message to a paused Job resumes it**, as a stopped Job's next message resumes its conversation: the lease is taken before the process starts, and a full pool leaves the Job paused and waiting.

| Refused | Code |
|---|---|
| Ended, a Helm, or holding no slot | 409 `fleet.session_not_pausable` |
| Paused already | 409 `fleet.session_already_paused` |
| Not paused (resume) | 409 `fleet.session_not_paused` |
| The pool or git will not park the work or seat the branch | 409 `fleet.session_pause_refused` |
| A terminal Job | 422 `fleet.session_command_refused` |

**What it does not do.** Only the slot the ledger records for the Job is parked; one a subagent of the Job leased stays held. The marker is kept in `hosted_sessions` (`paused_at`, `paused_resuming`), so it survives a Fleet restart; a restart forgets the line, and the Job's next message joins it again.

## What a message names

A message to a hosted Job can name things with `@` and `#`, sent as `mentions` and kept on its `message` row as `tags`. **Fleet writes the line that tells the agent** (`The person named: ...`), ahead of the words, and the thread keeps the words as typed.

| Tag `kind` | `id` | The agent is told |
|---|---|---|
| `session` | The Job's id | Its title, and its address to write to with `SendMessage` where Fleet hosts it |
| `pull_request` | The number | `pull request #219 "title"`, and its address where the tag has one |
| `issue` | The number | `issue #214 "title"` and its address. A tag's title begins with `#` and the number, and the line drops that so the number is said once |
| `branch` | The name | The branch |

**`@` offers the other Jobs, and once letters follow it the repository's files that match them.** A file is written into the words as `@path` and is not a tag: the agent reads the path itself. Pull requests are `#`'s, and no trigger offers a branch, though the kind is still accepted.

**`#` at the start of a word offers the repository's open issues and open pull requests**, matched on the number, or on the key where the tracker names issues by one (`#ops` offers `OPS-7` and `OPS-12`, whatever the case), and reads one number that is neither (a merged pull request, a closed issue) when it is typed whole. The repository is the Job's own. Helm's box offers the same, for the repository its conversation answers for (`helm.md`, *The message box*). `url` on a tag is optional and only the two forge kinds carry it.

## A Job's Checks

**An `armada check` run in a slot a Job holds is the Job's run.** The CLI keeps no record of its own, so it tells Fleet when the run begins and when it ends with the log (`report_session_check`, `POST /sessions/checks`, 23.75). Fleet places the Job by the slot the directory is in, never by what the call says, and **the pool's lease answers first**: the live Job the slot's lease names (a Job or its subagent leases more slots than the one its ledger `slot` row names), else the Job whose standing `slot` row the pool does not deny (`slot_holders::session_in_slot`). A directory no Job holds is a person's own run and is not recorded, and neither is a Fleet that is not running.

| Where it shows | What |
|---|---|
| A hosted Job's thread | One `check` row per run: the Check's key, its run id, `running`, `passed` or `failed`, and what it was narrowed to (`target`: the test filter, or `--changed` and the packages the change reached; absent where it ran whole). The row is replaced by id as the run ends, and the Job is published whole each time |
| The Job's header | The running glyph and a count of the `check` rows that are `running`, named `2 checks running`. A press lists them, each with its key, its `target` and how long it has run, and a press on one opens its run's log over the Job. Nothing is drawn at none, and the list closes with the last |
| The Checks page | A row with the `session` requester (`session_id`, `slot`), drawn with the slot's owner chip. Pressing the thread row opens the run's log over the Job |
| The log | `get_session_check_output`, `GET /sessions/checks/:run/output`, answered as `get_check_output` answers a run's. Kept up to 256 KiB, the newest end when cut |

**While it runs, its log is live and a person can stop it.** `armada check` tells Fleet what it printed in `chunk` calls on the same route, and the Check log panel follows it (`observe_session_check`). The panel's head holds Stop, which sends `SIGTERM` to the Check's process group and `SIGKILL` after a grace period, and Force stop, which sends `SIGKILL`. Fleet holds the ask and answers it to the next `chunk`; the process ends its own group and tells `end` as for any run.

A terminal Job's thread is its transcript and carries no `check` row; its runs are on the Checks page. A run whose `armada check` process is killed stays `running` until its Job ends, when Fleet ends it as failed with "the Job ended" as its log.

## A Job's retro in its thread

**A retro being written is a `retro` row on a hosted Job's thread** (23.78). Fleet adds it when it accepts a press, replaces it by id as the writing goes (`stage`, `stage_since`, `characters`, the `items` that have settled) and leaves it when the retro is kept (`retro`, the number `get_session_retro` takes) or has failed (`why`). `docs/concepts/retro.md`, *A Job's retro*, has the stages.

| Where it shows | What |
|---|---|
| A hosted Job's thread | One `retro` row per retro, in the order the presses came. It is a progress report: it holds no retro, and `has_news` and the retro's own record never read it |
| `get_session` | The row as it last stood. One left going by a Fleet that has restarted is returned and kept as `failed`, "Fleet restarted while it was written" |
| A terminal Job | No row. Its thread is its transcript |

## Running a block

A fenced shell block in a Job's thread shows Run when the block is hovered or focused. Pressing it opens a terminal docked in the Job's view, between the thread and the message box, and runs the command in the Job's working directory, its slot if it has one. A Run key on a Needs-you item opens the same pane, and so does a message that is one line starting with `!` (`! rbenv install 4.0.6`) sent from the Job's box: it is run, not sent to the agent, and when it ends Fleet tells the Job as for any run. A bare `!` or a message of several lines is sent as written. There is one pane and one run at a time: Run again stops the old run first and starts afresh. The pane's header carries the command and Stop while it runs; after, a mark with the exit code on its tooltip (*Exit N*, or *Stopped*), Run again and Close. Close puts the pane away, and the block's mark brings the same run back. Keys typed go to the program while it runs, and its size follows the pane's.

**It is the person's own act, and the command runs on a terminal.** It is the job leader of a pseudo-terminal with `TERM=xterm-256color`, so a program sees a tty, keys written with `write_session_run` reach it (Ctrl-C as an interrupt) and `resize_session_run` changes the size it sees. Output is the terminal's bytes with the escapes intact, held as the last 256 KiB. The pane is xterm.js (`@xterm/xterm`), drawing those bytes as they arrive; its keys go out as `write_session_run`, its fitted size as `resize_session_run`, and the run starts at the size the pane first fits (`cols` and `rows` on `start_session_run`, `POST /sessions/run/start`). Bridge reads what it prints on `observe_session_run`, a socket of its own, until `exited`. A Job that has ended, or a run Fleet does not know, is refused in the same words as any other act on a Job.

**When the run ends, Fleet tells the Job** as it tells it anything: the command, `Exit <code>.` or `Stopped.`, and the last 40 lines as a person would have read them, escapes and carriage-return overwrites resolved. **A hosted Job's thread keeps that as one `ran` row** (command, exit code or stopped, the plain tail), and the agent is sent the same words once, with no message from Fleet beside the row. A terminal Job cannot be written to, so its mod is handed the words as a message (`told_by_fleet`, held for it). **A run that exits 0 also takes the waiting item whose `run` act names that command off the Job's list**, from the thread or from the Needs you band, in the same moment, and the message says so; Bridge answers nothing. A Job that has ended by then is not told.

The screen is not kept past the run. Closing the window, or the Job, ends the reading; a reopened thread shows the block without it, and a hosted thread keeps the run's `ran` row with its tail.

**A block with no language is a shell block when it reads as one** (`readsAsShell`): every line opens like a command, after an optional `$ `, and none reads as code, JSON, prose or a diagram. A doubt is no Run. In a **terminal** Job, inline code that is exactly a command the terminal lists (`` `/update-armada` ``, `` `/model opus` ``) is pressable instead: it is run in the terminal as typed with `run_session_command` (`POST /sessions/command`), no output comes back, and a check says Sent. A Fleet-hosted Job has no terminal, so the code stays plain. A waiting item whose `run` target is such a command takes the same path: Run in the Needs-you list sends it to the terminal instead of a shell, and a Sent settles the item.

## A terminal job's thread

Open a job a person runs in a terminal and Bridge draws its conversation and a message box. `get_session` reads the transcript file the agent CLI keeps for it (`adapters::terminal_thread`, found by job id under the person's home) and answers rows in the shape a hosted thread has: what the person typed, what the agent said, and one line per tool call. Fleet then watches the file for as long as the job is live and publishes each new line as `session.row`, so the thread follows the terminal. A row's id is the transcript line's own, so a line read twice replaces itself.

A subagent has a thread of its own. `get_session_subagent` reads `<project>/<session id>/subagents/agent-<subagent id>.jsonl`, the file the CLI keeps beside the job's, and answers it in the same rows with whether the subagent `finished` (a turn ended with nothing more to run, a hand-back, or the CLI's interrupt line where its parent's `TaskStop` or the person cut it off) and its `report`, the last thing it said. **It also says the `model` its last answer was written under and `context_tokens`, the input side of that answer's usage (input, cache write and cache read), against `context_window`**, which Bridge draws in the sheet's head as the Job's own figure is drawn; the type of agent is not read here, it is the `type` the spawning call's attachment already holds. The transcript names the model and never its window, so the window is the parent Job's reported one where it runs on the same model id, else the usual 200k until the tokens outgrow it and 1M after, a guess made in one place (`window_for`). It reads that one path and no other, for hosted and terminal jobs alike. Bridge reads it again while the subagent runs. **The engine's stop settles it**: the mod's `SubagentStop` hook makes the row `spent` and stamps when, even on a row already spent, because a transcript can end on a line that reads as still working (13 Oct 2026). Where no stop came, a `standing` row is shown `spent` once its transcript shows a turn that ended, and the first `get_session_subagent` that sees it finished settles it. **A parent can send a settled subagent more work**, which is no new spawn and no hook, so Fleet looks every two seconds (`subagents_followed`) at the transcripts of the settled subagents of live jobs, a `stat` each until one grows: one whose last line is work stamped after the row settled is `standing` again, and the timeline enters `Subagent resumed`. Work stamped before the stop opens nothing (14 Oct 2026).

> **Rule.** The thread is drawn and never read as a claim. Nothing Fleet knows about a terminal job, its title or its state, comes from its rows.

A slash command the person ran is one `command` row, as typed, and the summary the CLI writes where it compacted a long conversation is one `compaction` row, so neither reads as the person's words. What is not drawn: the agent's reasoning, a tool's answer, a command's output, a subagent's own turns, the CLI's bookkeeping (a caveat, a reminder, a task notification) and a message from another job. No wrapper tag the CLI writes for itself reaches a row.

**A message reaches a terminal by its own mod.** Fleet cannot push into a terminal. `send_session_message` to a terminal job holds the text, and the `armada` mod in that job holds an ask for `take_held_messages` open (`wait_ms`, up to 25 seconds, one at a time), which Fleet answers the moment something is held, and submits each text as the person's own prompt, which starts a turn whether the job is idle or busy (spike 27). The ask is also how Fleet knows the job is listening: a send to one with no ask open that has not asked within ten seconds is refused as `fleet.terminal_session_unreachable`, so the person is told at once. A file or picture is saved by Fleet as for a hosted job and sent as `Attached file: <path>` in the text. The mod reports what the terminal runs on (`tuned`: its model, effort, permission mode and the commands it lists) and Bridge draws those in the composer. A model or effort chosen in Bridge is held as a command, and the mod runs it as `/model <x>` or `/effort <x>`; the engine refuses a slash command submitted as text, so it goes through the mods API's command call (spike 27). **The permission mode is shown and not set**: nothing in the mods API switches a live job's, and it is read from the settings-hook inputs at each turn, so it is stale between a change in the terminal and the next turn. End job works on a terminal job too: *Ending a terminal job*, below. The sent text shows in the thread when the transcript has it, which is when the turn starts.

### Ending a terminal job

**End job ends a terminal job everywhere**: the agent's process in the
person's terminal ends, the slot it stood in is parked, and the row ends as a
hosted close does, `end_reason` `closed`. Fleet cannot end a process it does not
host, so `close_session` on a terminal job holds an `end` for its mod, in
the answer to the ask it already holds open (`take_held_messages`). The mod runs
it as `/exit` through the mods API's command call, behind any message held with
it, and the engine runs a command only once the job is idle, so a turn
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
| The slot | Parked once the process holding its lease has gone, as a hosted close parks (the branch it is on, what still runs in it ended first). The mod reports from inside the exit, so Fleet asks the pool again for up to ten seconds. A holder still running then (a person's own terminal, another job's lease) is left alone; a park the pool refuses for another reason is said on Fleet's log. The ledger's rows were given back when the terminal reported its end, so unlike a hosted close they do not wait on the park |

**A terminal that ends on its own is parked too**, the same way and with the
same wait for its process, and its row keeps the terminal's reason:

| Reason the mod reports | Slot |
|---|---|
| `prompt_input_exit`: `/exit`, ctrl+c, ctrl+d | Parked |
| `other`: the window closed (a hangup), another signal, a `-p` run done | Parked |
| `logout` | Parked, once the process has gone |
| `clear`, `resume` | Kept. The same process goes on under another job id, in the same directory |
| None: a crash or a kill no hook outlives | Kept. The job reads Quiet and its slot is held for a resume |

What Fleet remembers about a press is held in memory only, so a Fleet restarted
between the press and the end ends the row with the terminal's reason, and parks
by the table above.

**A terminal's question is an ask in its thread too.** The `armada` mod hooks `AskUserQuestion` and posts the question once with `ask_from_terminal`, which Fleet keeps in its store as the same `ask` row a hosted job writes, so Bridge draws the same card, and answers with the call's id. The terminal's own prompt opens once Fleet has answered that post, or after a second and a half without an answer, and not at all where Fleet refused the question because a walk is open (*Showing a window*). The mod then polls the same route with `wait`, each poll held about 25 seconds, so no one request has to outlast the question: a poll that fails or times out is asked again, and a Fleet that restarts finds the question where it was. A person answering in Bridge has the answer kept until the mod's next poll collects it, and the mod returns it as the tool's result, which drops the terminal's prompt. A person answering in the terminal first makes the mod send `settled`, which closes the card. A card whose job ended, or whose mod has not polled for three minutes, is closed too, so Bridge never offers one that cannot be answered. Fleet out of reach leaves the terminal's prompt alone.

## Not built

**The Manifest switch is drawn on mock data only.** The chip is read off the wire, but the form's *Pull request owners* switch still runs on `draft.prOwners`, so a real Bridge draws none and the key is set in `armada.yml`. A row is deleted by a Fleet that saw the pull request open and then gone: one that merged while no Fleet was reading keeps its row, and so does a claim that was given back.

**The `/` list is empty until an agent has started.** `hosted.commands` is the names in the stream's `init` line, slash commands then skills, and a job whose own process has not started is given the last one any job read. It is held in memory, so after a Fleet restart it is empty until a process starts, and the harness says nothing of what each command does, so the list draws names only.

**A job that dies without ending stays `live`.** Its `last_seen_at` is what says it has gone quiet; nothing yet checks the process.

**A message another job sent a terminal job is not drawn.** A hosted
job's thread carries what another hosted job wrote to it; a terminal
thread skips the line, and the mod reports the message as a count and never
its text.

**Hosted jobs in different permission modes may not wake each other.** The
tool says a job in another mode holds a cross-session message for approval.
Not measured.

**A message names where it went and not which job**, because the harness does not say: a delivery says whether it came from a peer or a teammate, and never a job id.

## Sleep mode

The owner turns on Sleep from the menu in Bridge's title row before bed (`set_sleep`), and the menu button carries the moon while it is on. Jobs keep going until only he can unblock them. `get_sleep` reads the switch and the night's rows; turning it on starts a new night and clears the last one's rows.

While it is on, a pass over every live job's waiting items (`crates/fleet/src/sleeping.rs`, riding the pull-request notice tick) works like this:

| Item | What the night does |
|---|---|
| `ask:` or agent item with a "(Recommended)" option, waiting under two minutes | Nothing. The agent carries on by itself first. |
| `ask:` or agent item with an option labelled "(Recommended)", past two minutes | Answers with that option. Recorded under decided, with the option as what it chose and the job it came from. |
| `ask:` or agent item with no such option: a question with other options, an open question, an errand with an act | Nothing, unless it reads as destructive (next row). It stays in the job's Needs you panel for the owner, and is not recorded under decided or blocked. |
| Destructive or irreversible (delete, drop, remove, force, reset, rm, overwrite, discard, close PR and the like, read from the text and the options; unsure holds) | Never answered. Recorded under blocked. |
| `perm:` permission | Never allowed. Recorded under blocked. |
| `walk:` | Recorded under walks. |
| A pull request the pull watch sees merge | Recorded under landed. |

**The review is cards on the stage, opened with the call key**, not a panel of its own: each decided row is a call, headed by who decided it. The card says what Sleep chose and keeps its acts on the rail: **Agree** keeps it (`review_sleep`), **Change** sends the owner's words to the agent (`override_sleep`) and reviews the row, **Open the Job** goes to where it was decided, **Later** puts it at the back, and **Dismiss**, under a cover, reviews it for good. A decision made in a Job ends in that Job's own box, which is how it is changed, and `session_id` on the wire row is the Job it opens. A correction Fleet refused leaves the row where it is. Either act marks the row reviewed, and `get_sleep` leaves a reviewed row out of `decided`. **The first use of Bridge after Sleep was turned on turns it off** (`set_sleep` off). Blocked and walk rows are already calls: waiting items.

`override_sleep` sends the job `Re: <asked>` and the owner's words, marks the row corrected and reviewed. Every change publishes `sleep.changed` with the night whole.

## A mod that is out of date

A mod loaded before the plugin was updated keeps running the old code until the person runs `/reload-plugins` in that job. **The mod reports its version** in `started` and in the first `tuned` (`MOD_VERSION` in `plugins/armada/hooks/facts.ts`, held equal to `version` in `plugins/armada/.claude-plugin/plugin.json` by a Fleet test, so bump both together), and Fleet keeps it on the job's row. `SessionRecord.mod_out_of_date` is true for a terminal job whose reported version is older than the `version` in the same file in the repository it stands in, or that reported none. Bridge marks it on the row and in the header with "Mod out of date: run /reload-plugins". Fleet reads the repository's file when it builds the record, so the mark moves with the next fact the job reports and not when the file changes.

## Installing the Claude Code mod

```
claude plugin marketplace add "$HOME/Library/Application Support/Armada/mod"
claude plugin install armada@armada-local
```

**Fleet keeps its own copy of the mod** in that folder. `scripts/restart` copies
`plugins/` into it from the tree it builds, so the mod is as new as the Fleet
and a pull on the checkout changes nothing a job loads. After a restart,
`/reload-plugins` in an open job picks the new copy up. The copy is made
beside the folder and renamed over it, so a job never reads half of one.
A tree with no `plugins/` leaves the old copy and warns. Whatever compares a
job's reported mod version with the installed one reads
`Armada/mod/armada/.claude-plugin/plugin.json`.

**The mod ships an output style, `Armada`** (`plugins/armada/output-styles/armada.md`), found from the folder with no entry in `plugin.json`, and picked with `/output-style` as `armada:Armada`. It keeps the coding instructions (`keep-coding-instructions: true`), states the job rules and sets plain, concise replies. A hosted job needs it no more than it needs the rules in its prompt; it is for a terminal job of a person who has no style of their own.

**The mod ships a fleet of subagents** (`plugins/armada/agents/`), named `armada:` and the type, so a job in any repository can hand a decided change to `armada:mechanic` on `sonnet` or a test run to `armada:test-runner` on `haiku`. The job rules say to prefer the repository's own types first. Each preloads the mod's skills its work needs (`plugins/armada/skills/`): `armada:reading-a-brief`, `armada:verify-before-done` and `armada:root-cause`, so a rule like "reproduce first" is written once. `docs/practices/subagents.md` has the fleet, the model each runs on and the skills each preloads.

It loads in a Judge call and a scout too, since they read the person's user settings. Fleet starts each with `ARMADA_DRONE=1`; the mod reads it and reports nothing, so only the person's own jobs reach the ledger ([spike 23](../spikes/023-does-a-user-installed-mod-load-in-a-drone.md)).
