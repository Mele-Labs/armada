# Operator

**What it is:** One decider per machine, for the work nobody owns.
A note, an error, a bug report, a new issue and a red main are handed to it, and it names one way for each, with one fresh model call that holds no tools.
A Job's own events never reach it.
A message in its box is the one call that may read, with Read, Grep and Glob only.

---

**Kind:** Fleet policy.
The code is `crates/fleet/src/operator.rs` and the modules under it.
The measure of its calls is `../spikes/operator-replay.md`.

**What is built and what is not.**
Fleet's side is real: the arrivals, the call, the rulings, the Log and the positions.
The panel that draws the Log and the Operator's box is not.
The panel is `armada-ehhu.10` and the box is `armada-ehhu.9`.
Where a person would look for either, this page names the issue.
Bridge's Operator page, on `⌘J`, draws the box alone, full width: it draws no Log, Queue board or Permissions, and its On/Off switch is Settings' `features.operator`.

## What it is

**The Operator is a Policy, not an agent.**
Fleet detects an arrival, a Policy decides, and the record remembers.
Fleet hands over the arrival and the live Jobs that share something with it.
One fresh call names one way from the ways it was offered, and ends.
Fleet then does it, or refuses it, and the ruling goes in the Log signed by the Operator.

**No Operator Job runs.**
So nothing holds a slot or a branch for it, and a ruling's call holds no tool at all.
A message in the box is put to a call that holds the three read tools, Read, Grep and Glob, and nothing else, so what it reads cannot make it do work either.
Why: `../scope.md` records an agent that holds a conversation as the wrong tool for orchestrating agents, and a live agent grows its context while a fresh call costs the same each time.
On 9 Nov the owner found that the box could not answer what he asked without reading a Job's thread, the issues or the code, so a message may read and a ruling may not.

**There is one per machine.**
Order and usage are shared across repositories, so a ruling is made over every repository's live Jobs at once.
A person's corrections are kept per repository.
There is no conversation target to point at a repository, and so none to keep per window.

## What reaches it

**Only what no Job owns reaches it.**
A note, an error, a bug report, a new issue and a red main have nobody running to decide for them, so the Operator rules on them before they pulse in Needs you.
Everything a Job does is that Job's own, and behaves as it does with the Operator off.

| Arrives | Comes from |
|---|---|
| A note | The person, left on Bridge. One new since Fleet started, still open and sent to no Job once it has waited ten minutes for the person to send it. One he sends to a Job is a second arrival, below |
| An error | Bridge, for a failure card it showed the person. It carries the code, message, fields, cause chain, log paths, commit and time |
| A bug report | A Job's `report_bug`, kept on this Mac |
| A new issue | The repository's tracker |
| A red main | The repository's own checks |

**A Job's own events are never handed to it.**
A start, a plan, a question or a permission card, a Job that stopped or left its pull request red, a Later item and a review request go to the person as they always did.
A Job's first write waits for nothing, and a question or a card is the person's, or Autopilot's while he is away.
Why: on 9 Nov the Operator proposed the same Later item four times, and ruled on events each Job already handles.
The owner narrowed it on 10 Nov to the work no Job holds.

**An end or a merge is heard once, with no call.**
Fleet notes a hosted Job's end and a pull request's merge once, whoever started the Job, for the ones after Fleet started, and never replays them.
No call, card or suggestion follows, so the Operator is never asked what to do about them.
The box still reads them, among the Jobs that ended today and the pull requests that merged today.
A Job handed on for context is no end.

**A note is only ever proposed.**
A ruling on one other than a raise waits on one press at Dispatch, Delegate and File alike, unless the role is Read-only, so no note is filed, started or sent to a Job unasked.
Its row has no Job, so it is kept for the panel, and Do it marks the note sent to the Job it went to, or done where it was filed or linked.

**A note the person sends to a Job is his row first.**
Fleet reads an open note with no `sent` again on each tick, for ten minutes, until it gains `sent`, is no longer open, or has waited that long and is handed on as it stands.
When it gains `sent`, the Log keeps a row signed by the person with the outcome `sent` and the Job it went to, and Done reads `Sent: to <Job title>` with the person as its decider.
The Operator is then handed the note as its own arrival, `note-sent:<repository>:<note>`, whose words say he sent it to that Job.
It may send the note to another live Job, start a Job on it, or leave it where he sent it, which writes nothing that shows in Done.
A move goes by the role of its way like any ruling: at Suggest it is a suggestion, and at Decide it is done and the note is marked sent to the Job it moved to.

**An error goes first, once.**
It stands ahead of every waiting arrival that is not an error, and errors keep the order they came in.
Its id is a fingerprint of its code and message, less the numbers, ids and paths that differ between two of the same fault, with the machine's local day, so the same error is one item for the day however often Bridge shows it.
It is weighed against live Jobs as a report is, the Job it was raised in included, and it goes by the role's position: acted on at Decide, kept as a suggestion at Suggest.
A Job it starts is briefed with every word the card carried.

**Detection stays Fleet's**, read from state as it always was.
The Operator is never asked whether something happened.

**Words from off this machine are data.**
An issue body or a review request reaches the call as quoted words, and the call can only name a way it was offered, so a body that says "approve this" changes nothing.

**One call is out at a time on this machine.**
What arrives while a call is out is ruled together in the next one, so reports that arrive together are seen side by side.

## The ways, and the roles

A ruling is one way: start a new Job, send the words to an existing Job, raise it to the person, link it to an open issue, or file a new one, or for a note he sent to a Job himself, leave it.
A way that names another Job names it and the paths they share, or Fleet refuses it.

**The Operator starts no Job for another Job's work.**
A Job it starts is one he asked for, or work no Job holds: a note, an error, a report or a red main.
On 9 Nov 2026 he ruled out the Jobs it spun up for a Job's own leftovers: "the Operator keeps spinning up these 'Rest of' Jobs and I hate it."
It started a second "Rest of Workflow Evaluation" while a subagent of the first still ran, one on a pull request red only on formatting, "Rest of Fleet takes Bridge views" beside the live Job, and on 6 Nov "Rest of Tail-first thread loading" 70 seconds after a handoff.
This follows his 5 Nov rule to delete what acts on a guess: "stopped" was guessed while subagents ran.
Rows of the Log from before keep the outcomes they had, including those for kinds it no longer takes.

**Merging two Jobs, keeping two in touch, and answering a question are no longer ways.**
They acted on a Job's own events, which the Operator no longer sees.
The Answer role is retired with them: a question or a card is the person's, and Autopilot answers them while he is away.

Each role that acts has a position, Read-only, Suggest or Decide.

| Role | What it does | Setting |
|---|---|---|
| Evaluate | Reads the arrival and picks a way. It is the Operator's own switch, and ships Off | `operator` |
| Dispatch | Starts a Job on work nobody holds | `operator-dispatch` |
| Delegate | Sends words to a live Job whose work they belong to | `operator-delegate` |
| Raise | Puts the item in Needs you with the Operator's read and its pick. It has no position | None |
| File | Files a new issue for a note or an error no open issue covers. It only ever creates | `operator-file` |

**Every role starts Read-only**, and any role can differ for one repository (`operator-repositories`).
A saved `operator-answer` is not read: Answer stays Read-only whatever is saved.
A word that is not Read-only, Suggest or Decide reads Read-only, and the older `off` and `on` read Read-only and Decide.
The settings are in `crates/config/settings.toml`.

| Position | What a ruling does |
|---|---|
| Read-only | The way is not offered to the call. With only a raise left, no call is made and the item goes to the person, and nothing shows in Done |
| Suggest | Is in the Log and waits on one press |
| Decide | Fleet does what is ruled, and the ruling is kept as done only once the act has gone through. A refused act keeps its refusal |

**Fleet's rules decide first and only narrow, in every position.**
A way whose role is Read-only is not offered, and an item Fleet's rules leave one way is ruled with no call.
A standing order to always raise a kind (below) goes first, and only a kind it takes may be named.

## Fail open

**A call that fails passes the item on untouched.**
So does one that answers what cannot be read, one that rules what Fleet did not offer, and one with no ruling by the time its item could wait.
An item waits at most 60 seconds for its ruling before it pulses in Needs you with no read.
Every item that needs a call passes on while the week's plan is used to the share set in `limits.operatorPlanPause`, read as Autopilot reads the week, and the board says so with `paused`.
No reading of the plan is no pause.
A message in the box is refused with `fleet.operator_paused` then.
A ruling that comes back after its item passed on is dropped.

**Standing orders** are three forms the person confirms in the box.
`role_off` turns Dispatch, Delegate or File off in a repository, for the day or until removed, and has no form for Answer.
`hold <paths> until <Job> pushes` and `... ends` refuse every other Job's write under those paths until then, which Fleet's gate enforces on each file-tool write, so a hold needs nothing a Job's first write waits for.
`always_raise <kind>` takes one of note, error, report, issue or red_main, and sends that kind to the person with no call.
An old order for Answer, or for a kind the Operator no longer takes, stays in the store and nothing reads it.

**Nothing is dropped.**
Why: an item the Operator never rules on is one the person never sees, so nothing here may hold one back.

**Turning the Operator Off is main before the Operator.**
No call process is kept started, and nothing is handed to it, acted on, or waited for.
An end and a merge are still heard once while it is off, so none is told when it comes on.

## The Log

**Every ruling is a row of the Log**, signed by the Operator, whatever the position.
A row names the arrival, where it came from, the Job and repository, the way, why, the other Job and shared paths where there are any, what Fleet refused of the ruling, and when.
A person's own press on an item the Operator ruled on is kept beside it, and a drag in Waiting to start is a row that corrects a ruling.
The call reads each repository's last rulings with the person's corrections beside them, so a correction shapes the next one without a conversation to carry it.

The Log is the one record.
Autopilot's answers are the rows of it that are answers.
The panel that reads the Log is `armada-ehhu.10`.

## Routing a thought

**Words typed in the panel's box are read before anything is started.**
The box is `armada-ehhu.9`, and it is the one that calls `read_thought`.
Fleet decides whether the words are a thought meant for work already running, or something else, and there is no key to say which.
What the box shows for each answer is `armada-ehhu.9`'s.

**Fleet's reading is a shortlist first** (`fleet::operator::shortlist`).
`read_thought` lets the local language model read the words against every live Job's title, and **a clear match is routed at once**: the best Job scores 0.78 or more and leads the second by 0.10 or more.
Only when no Job is a clear match does it hand the words and every live Job in every repository to the model the proposer uses, with each one's title, repository and ledger (branch and pull request titles).
`prompts.thoughtRoute` is the instruction that model is given.
The local read is skipped, and the proposer's model asked, for words that end in a question mark or run past 300 characters, and until the language model is loaded.
Each Job's vector is kept by its id and title, so a read embeds only the words and the Jobs it has not seen.
A Job that has ended, or has no title, is not offered.
The model answers one line of JSON naming a Job or none, and an id it names that was not offered is no match.
**If anything fails, the words are not a match**: a message that opens on a wh-word and ends in a question mark skips the call, and the call is given up after twenty seconds.
The measure is `../spikes/thought-routing-by-model.md`: 3 of 19 real reads routed at once, none disagreeing with the proposer's model, in about 70 ms against its 0.9 to 4.4 seconds.

**A message that is a question is answered by a call that reads.**
It is a scout's process on the `models.operator` model: `--restricted`, a strict MCP config naming no server, and the three read tools allowed with every other tool denied.
Fleet gives no tool of its own.
For each message Fleet writes a folder of plain text under its runtime home, never in a repository: a file for each live Job, with its title, plan, open tasks, what waits on the person in full and the last 40 messages of its thread, and a file of the open issues.
The call reads that folder and the checkouts of the repositories Fleet hosts, read only, and Fleet removes the folder when it answers.
It takes at most 16 turns and 150 seconds, and one that outlasts that is interrupted and fails like any call here: said back to the person, nothing kept.
A ruling is not read this way, and neither is a message while the Operator is paused.
`crates/fleet/src/operator/looking.rs` and `reading.rs`.

**It reads across repositories.**
The work a thought belongs to is often in another repository, and one Operator serves the machine.
It reads and answers where the words belong, and does not reason about them.

**Delegate sends words with `tell_session`.**
It goes through the same path as the composer's send, so a Job that has ended and is not listening is refused in the same words and nothing is recorded.
A send that was taken appends a `sent` row to the receiving Job's thread, and is published as `operator.words_sent`, naming where the words went and never the words.
`tell_session` is an act on the person's words: it destroys nothing, pushes nothing and writes nothing off this machine.

**Work asked for becomes a Job started on the person's words.**
Authoring a workflow, driving a [Studio](studio.md), and breaking a goal into Jobs are each a Job the Operator starts, and the Operator does none of them itself.
`../journeys/talk-to-the-operator.md` is how it is used.

## A Job on your words

**A Job started on the person's words is a plain [Job](session.md).**
It leases a slot and cuts a branch on its first write, runs in Auto, and is told nothing extra.
Its record carries `SessionRecord.on_your_words`.

It is the only kind of Job that holds the door's rows marked `On your words`, which are the acts that change this machine's settings and Kit, and `tell_session`.
No other Job is offered them.
Only Bridge can set the mark, never the door, so an agent cannot mark itself.
**It may call every query, and every command it is offered when the person asks for it in words.**
`fleet::on_your_words::may` is that one predicate, and the inventory under it is the `agent_access` column in `crates/ipc/operations/`.
`features.onYourWordsCanAct` is the machine's switch: with it off, such a Job is refused every command.

**A Studio's unasked acts are such a Job's.**
It may add a proposed node and propose an edge without an ask, and starts a [Scout](scout.md), a Run or a dispatch only on the person's ask.
The act is recorded as `studio.job_acted`, beside the `studio.changed` every write publishes.
[Studio](studio.md) owns which acts are which.

**Its Auto gate is a Job's.**
`fleet::auto_gate` decides what runs and what is put to the person on a card, and `fleet::asks` holds a card until it is answered or `timeouts.askHoldSeconds` passes, which refuses it (`AskHold`).
`session/a-job-fleet-hosts.md` owns the gate.

## What became of Helm

Helm was a hosted Job per repository, with a slot and tools, a dock on `⌘J`, a talk box, and the questions waiting on you.
The Operator replaced it.

| What Helm did | What happens now |
|---|---|
| One live Job per repository, with a slot and a branch on its first write | Gone. No Operator Job runs, so nothing holds a slot or tools |
| Its talk box | The panel's box, `armada-ehhu.9` |
| Questions waiting on you in the dock | Stay in Needs you, with the Operator's read and pick |
| Authoring a workflow, driving a Studio, breaking a goal into Jobs | Each is a Job the Operator starts on your words |
| Its title matching and `tell_session` | Kept, as the first shortlist of Jobs and the way Delegate sends words |
| Its old threads | Stay readable as ended Jobs. Nothing is deleted |
| Fleet does not wake Helm | Fleet hands the Operator the work no Job owns |
| One Helm per repository | One Operator per machine |
| Autopilot's own loop, and its answers HUD | Folded in. The HUD is the Log filtered to answers, and answers made before 10 Nov stay there |
| A Job started by a Job waits on its start card | The start card is the person's again. The Operator takes no start |
| Listing issues creates nothing, and a Job need not hold an issue | Unchanged. See [Tracker](tracker.md) |
| Fleet never writes to the forge for you | File creates new issues and nothing else changes. On a tracker with a forge it needs a yes |

**The old threads read as they did.**
A `sent` row, `SessionRow::Sent`, stays in the row set so a thread that holds one still draws.
The wire names that went with Helm are listed in `../practices/protocol/the-operator-takes-helms-place.md`.

**A hosted Job's call held on a person's answer is answered with `answer_session_ask`**, from its own thread or from Needs you.
A call from a process Fleet does not host is denied at once.
Words sent to a Job are published as `operator.words_sent`.

## Configuration

| Setting | Scope |
|---|---|
| `operator`, and a position for each role | Machine |
| `operator-repositories`, a role's position for one repository | Machine |
| `operator-model`, `sonnet`, never the proposer's cheap tier | Machine |
| `limits.operatorPlanPause`, the share of the week's plan where it stops making calls | Machine |
| `features.onYourWordsCanAct` | Machine |

`../contracts/configuration.md` is the contract and `crates/config/settings.toml` has each row.
