# What is kept

**Part of:** [Job](../session.md).

Three tables, in `crates/store`. The second is not the first's child, and the third is the second's history.

| Table | One row is | Key |
|---|---|---|
| `sessions` | A job: harness, origin, repository, directory, title, `live` or `ended`, when it started, was last seen and last finished a turn, the figures it last reported | `id`, the harness's own |
| `ledger_attachments` | One thing a holder took or did | `holder_kind`, `holder_id`, `kind`, `manifest_id`, `target` |
| `ledger_attachment_versions` | One version of one of those rows, from `at` | `seq`, in the order written |

A holder is `{ kind: session, id }` and points at neither table, so another kind of holder can hold rows without a table of its own. A `kind` is open text:

| `kind` | `target` | Held until |
|---|---|---|
| `slot` | the slot's number | the job leaves it, the pool shows it held by another or free, or the job ends |
| `branch` | the branch's name | the job checks out another, its slot is given back, or it ends |
| `pr` | the pull request's number | it merges (`spent`) or closes (`given_back`) |
| `issue` | the issue's reference: the forge's number as digits, or a tracker's key such as `ABC-12`; `detail` is `title`, `state` (`open` or `closed`) and `address`, which the pull reading reads through the Manifest's tracker each pass, one read an issue, as it reads each `pr` row, and `pull_requests`, the numbers of the pull requests that work it, comma-joined ascending, which only grows. Taken when the person names it to the job, by an issue tag or a link into the job's own repository, in a message or the request it was dispatched with; or when the job's agent files it in that repository, which the mod reads off the address `gh issue create` printed; or when a pull request it holds closes it, as the `pr` row's `closes` says; or when a pull request it holds names its key in its title or branch, as the Manifest's tracker links one, only for the tracker's own projects and with that pull request on the row; or when a group of its recorded plan names it as `issue`; or when the agent or the person claims it with `claim_issue`, which may name its pull requests. Taking the issue off the group in a later recording does not give the row back. A bare `#123` is not naming it | a terminal job ends, which gives back all it holds. A closed issue's row stands, and naming it again takes nothing a second time |
| `subagent` | the agent's id | it stops (`spent`) or its process is cut off (`given_back`); more work after either holds it again, *A terminal job's thread* |
| `background` | the task id the harness gave a shell command the agent ran beside its turn (`run_in_background`) or a `Monitor` watch; `detail.form` is `command` or `monitor`, `task` its description, `command` the command, and once it ends `status` (`completed`, `failed` or `killed`), `output` (the file its output went to) and `exit` (the code, for a command that exited on its own) | the task ends (`spent`, with `status`), or the process that ran it is replaced (`given_back`). Several stand at once, and a hosted Job's only; *Background tasks* |
| `message` | `to:<who>` or `from:<who>`, with a count in `detail` | |
| `studio` | the Studio's id | not reported by anything yet |
| `forked_to`, `forked_from` | the other job's id | `spent` when written, so the old job ending does not give the link back. A move's pair carries `detail.moved`, *A moved job* |
| `started_to`, `started_by` | the other job's id | `spent` when written, as a fork's links are. The caller holds `started_to` and the Job its agent started holds `started_by` |
| `unsent` | the words of the first message another job's agent wrote | `standing` until the person's next send, which spends it; ending the job gives it back |
| `artifact` | a page's or document's address, or a file's absolute path; `detail.form` is `page`, `file`, `image`, `doc` or `window`, `detail.title` its name | nothing: it stays on the ledger after the job ends |
| `link` | a web address (`http` or `https`) the person wrote in a message or the request the job was dispatched with, or the agent wrote in its reply; `detail.by` is `you` or `agent`, the first to write it. Not one to this machine (`localhost`, `127.0.0.1`, `[::1]`, `0.0.0.0`, `*.localhost`) or to the job's own repository's issues and pull requests, which the `issue` and `pr` rows hold. A sentence's closing punctuation is not part of it | nothing: `spent` when written, so it stays after the job ends, as a fork's link does. Writing it again changes nothing |

**An artifact is something a person would open, and a code edit is never one.** Edits are Branches and Pull requests. Five forms:

| Form | What counts | The press opens |
|---|---|---|
| `page` | A page published with Claude Code's `Artifact` tool (a publish, not a list, read or asset upload) | the address, in the browser |
| `file` | A file the `Write` tool **created** with a document extension: `md`, `mdx`, `txt`, `pdf`, `png`, `jpg`, `jpeg`, `gif`, `webp`, `svg`, `csv`, `doc`, `docx`, `xls`, `xlsx`, `ppt`, `pptx`. Not under `node_modules`, `.git`, `.claude`, `.armada`, `target`, `dist`, `build` or a temporary directory. `html` is left out, because a page's source appears as the page once published; configuration (`json`, `yaml`, `toml`) is not a document | the file, on this machine |
| `image` | A picture the job **looked at**: a `Read` of a `png`, `jpg`, `jpeg`, `gif` or `webp`, in any directory (a screenshot is usually in `/tmp`). The same path twice is one row, and a picture it wrote stays `file` | the file, on this machine |
| `doc` | A Claude Docs document made or edited (`mcp__claude_ai_Claude_Docs__create`, `batch`, `update`), by the address its answer carries or the document's id | the address, in the browser |
| `window` | A page the job showed with `show_window` | the address, in Bridge's own window |
| `sketch` | A diagram the job drew with `show_sketch`; `target` is `sketch:<title>` and `detail.by` is `agent` | the sketch pad |

A job can also put a file on the ledger itself with `show_file`: [Showing a file](showing-a-file.md).

The terminal mod tells the first five (`plugins/armada/hooks/facts.ts`, `artifactOf`).
**A hosted job tells files and pictures looked at**, because its stream carries a `Write` call but not its result, so a published page's address and a Docs link are not seen; and it cannot tell a new file from a rewrite, so a rewritten document counts.
The extension lists are the same rule written twice, in `facts.ts` and `adapters::written_document` / `adapters::viewed_image`.
Files made by other tools, such as a screenshot a command saved, are not seen.

## Every version

**A row of `ledger_attachments` is replaced in place, and every version it stood in is kept.**
A trigger on the table writes one to `ledger_attachment_versions` when a row is inserted and whenever its `state` or `detail` changes, whichever of Fleet's writers made the change, stamped with the row's `changed_at`.
A write that moves neither, such as a subagent stopping a second time, adds none.
A row kept before the table existed has one version, the state it was in, at its `since`.
Bridge reads a Job's versions whole, oldest first, with `get_attachment_history`; they ride no `session.changed`, which carries the latest row only.
A session Fleet forgets as blank takes its versions with it, and an ended Job's stay with its rows.

**A version also says what it followed.** Fleet keeps, for each Job, the thread row id of the person's latest message and the plan task in progress (`session_focus`), and a trigger on `ledger_attachment_versions` copies both into `ledger_attachment_version_origins` as the version is written.
The message is the one a person's send to a hosted Job wrote, set before anything else in the send can return, or, for a Job that waited on another, the brief it is sent when its wait is met; the task is the one the agent's task list put in progress most recently, and none once that task is completed or nothing is in progress.
Fleet works both out from what it saw, never from what the agent says.
A version written before this, by a holder that is not a session, or before the Job had a message or a task, follows nothing known.
It is a second table because a migration that adds only cannot change the triggers that write the version, and a new trigger on the version cannot update its row.
The same two ids ride `AttachmentVersion` as `from_message` and `from_task`, so Bridge can hang an artifact off the plan group of its task, or else off the message it followed.

## Whose slot it is

**The lease in the pool says which Job works a slot. A ledger row is a claim, believed while the pool agrees.** A harness reports the directory a Job is in, and a slot it left stays in that directory for whoever leases it next, so the directory says where a Job was and the lease says who works there.
`armada worktree lease` records the agent job it ran under beside the holder; a hosted Job holds by its own id; a lease taken by hand, or before leases named a job, names none.

| What the pool says of a slot, for a Job | Fleet |
|---|---|
| The lease is written as the Job's | Credits the `slot`, its checked-out `branch`, and what is read from it |
| The lease names no live Job: a person's own terminal, an older lease, a Job that ended (`/clear` starts a new id in the same terminal) | Credits it, as before the lease named anyone |
| Held for another live Job, or free | Credits no `slot` and drops a `branch` or `pr` read from it |
| Any of these, for a `pr` the mod reports with `created` in its `detail` | Credits it. The mod read its address off the job's own `gh pr create`, so no directory is being trusted, and a job that leased a second slot and left its first still owns what it opened |

**A release is settled by the pull sweep, for hosted and terminal Jobs alike.** Each interval Fleet gives back the `slot` and `branch` rows of a live Job whose lease is no longer its own, by whatever road the slot went: its own `armada worktree release`, Bridge's release, a lease taken back from a gone holder.
A hosted Job is also followed at its next tool call.
**A `pr` row stays**: the Job that opened a pull request is still the one to tell after it leaves the slot, and the row settles when the pull request merges or closes.

**A slot whose branch merged is offered to its holder before Fleet takes it back (the owner, 16 Oct 2026).** Jobs kept slots long after their work landed, and a full pool left others with none.
When the pull request from the branch a slot is leased on merges, Fleet tells the Job the lease names, whatever the pull request's owner was told: `slot-3's branch X merged`, to lease it for new work or give it back.
The Job may want the slot for follow-up work in its plan, so the notice comes first.
A hosted Job's next turn that ends with nothing queued is then the moment Fleet gives the slot back, if HEAD is still that branch or a commit the base has, nothing is uncommitted and every commit is on a remote or the base.
A new branch cut there is new work and the slot stays.
Uncommitted files or commits on no remote keep it, and the Job is told once for each what keeps it.
The branch is kept.
`docs/concepts/fleet.md`, *Worktree slots*.

**The ledger draws every slot the Job holds.** It keeps one `slot` row, the slot the Job works in, so a slot its subagent leased has no row of its own.
Its record is drawn with a `slot` for each other slot whose lease is the Job's, by Fleet's lease or a subagent's `armada worktree lease` naming it (`crate::slot_holders::slots_of`).
A terminal Job that moved out of a slot gave it up though its shell still holds the lease, so a slot the ledger gave back for one is not drawn again.
The owner, 15 Oct 2026.

**One reading answers who holds a branch or a pull request.** A Job is confirmed on a branch when it is live, its `slot` row stands, the lease there is its own or names nobody live, the worktree exists, its `branch` row stands and that branch is checked out.
The owner of a pull request, the Jobs told which branches a build holds, the write-clash table and the attaching of a pull request nobody reported all ask it (`crate::slot_holders`); the write-clash table adds every other slot the Job holds.
Where no Job is confirmed, the one whose standing row is oldest stands, and a branch a Job gave back is nobody's.
A merge is attributed to whoever held the pull request or its branch in any state, since the question there is who made it.

**State is one vocabulary for every kind.** `standing` is in force, `spent` is used and done with, and `given_back` was let go before it was used. A released slot and a spent need read the same way.

## Releasing what a Job holds

**The person gives back everything a Job holds, or hands it to another, with `release_session_holds`** (`session_id`, optional `to_session_id`), from Release on the Job's flight controls ([Cockpit](../cockpit.md)).
The owner, 16 Oct 2026: "Give me a button in the flight control of a session that is gated by a confirmation dialog.
This button will let me force release everything in the ledger of that session.
Agents sometimes can't do this but from Bridge we should be able to make it happen.
It would be cool if I could select another session to transfer what it holds to." It acts on the `standing` `slot`, `branch` and `pr` rows named in `rows`, each a `kind` and `target`, and on every one where `rows` is absent, and leaves the rest.
**Bridge draws each as a checkbox, all ticked when the confirmation opens, and sends the ones ticked.** The owner, 16 Oct 2026: "If we're building this, I should be able to check what I want to release.
By default everything is checked." A row the Job does not hold standing is refused and nothing moves.
A slot takes the branch it has checked out with it.
Where a slot is among the rows, a hosted Job's running turn is stopped first, as `stop_session` stops it, and the Job stays live.
With no target each row is given back, and each slot the pool holds for the Job is released as Bridge's slot Release does it: its uncommitted files are committed to its branch, then the slot is freed.
With a target each row is given back and written standing for it, `detail.handed` reading `session <id>`, and each slot's lease moves to it as it stands, held by its id as a hosted Job holds its own.
A target that is the Job itself, has ended or works in another repository is refused and nothing moves.
Each moved pull request is registered to the target in the central registry, as a claim registers it, and a store that cannot be reached leaves the hand made.
[A handed-off Job](a-handed-off-job.md) is this hand with a new Job as the target.
Nothing is deleted.
A slot the pool would not move stays with its branch, and the answer names each that stayed and why.
`agent_access` is `No`: it is the person's act.
**Each held row of the Job's ledger also carries its own Release**, shown on hover or focus, which confirms the one row and sends it alone with no target; the row then reads given back, or says why Fleet kept it.
The owner, 16 Oct 2026: "I should also be able to release items from the ledger panel one by one."

## Which machine owned a pull request

**A registry names the Job that claimed a pull request, so a second machine under the same GitHub account can tell who had it.** It is a small central store, set up by [signing in](../account.md), and Fleet writes a row when a Job claims a pull request.
Nothing is written to the pull request or the repository, so a label, a comment and a body trailer are all refused.

| Field | Holds |
|---|---|
| key | The sha256, in hex, of `owner/name#number`, the name lowercased so every machine makes the same one |
| kind | `session` |
| id | The holder's short id |
| machine | The machine's name |
| since | When the claim was made |

**A row never holds a title, a repository name or code.** The machine that owns the pull request draws its title from its own ledger, and any other machine draws only the stored facts.
A repository registers nothing until its Manifest turns it on, and the switch is off by default.
With it off, the repository registers none and shows no owners at all, those another machine registered included.

| When | Fleet |
|---|---|
| A Job claims a pull request (`claim_pull_request`) | Writes the row |
| The reading of a repository's open pull requests | Asks for the rows of all of them in one call, and puts each owner on the merge line's pull request and on the panel's page as `owner` (kind, id, machine, `this_machine`) |
| A pull request is in the last reading and not in this one | Deletes its row |

**None of it can fail the claim or the delivery.** A store that cannot be reached, a repository bound to no profile (or to one signed out) and a switch that is off leave the row unwritten and the field absent, and a claim is made all the same.
The panel's page carries what the last reading found and makes no call of its own.

**Bridge draws a row as a chip.** The `square-terminal` glyph and the Job's short id in mono lead, and the machine's mark follows: `laptop` for this machine, `monitor` for another, its tooltip the machine's name.
A pull request with no row draws no chip, in the merge line's open pull requests and in the Pull request panel.
The chip is drawn from the wire's `owner`.
The switch is *Pull request owners* in the Policy section of the Manifest form.
