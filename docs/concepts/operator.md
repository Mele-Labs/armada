# Operator

**What it is:** One decider per machine. Every piece of work that reaches Fleet is handed to it first, and it names one way for each, with one fresh model call that holds no tools.

---

**Kind:** Fleet policy.
The code is `crates/fleet/src/operator.rs` and the modules under it.
The measure of its calls is `../spikes/operator-replay.md`.

**What is built and what is not.**
Fleet's side is real: the arrivals, the call, the rulings, the Log and the positions.
The panel that draws the Log and the Operator's box is not.
The panel is `armada-ehhu.10` and the box is `armada-ehhu.9`.
Where a person would look for either, this page names the issue.

## What it is

**The Operator is a Policy, not an agent.**
Fleet detects an arrival, a Policy decides, and the record remembers.
Fleet hands over the arrival and the live Jobs that share something with it.
One fresh call names one way from the ways it was offered, and ends.
Fleet then does it, or refuses it, and the ruling goes in the Log signed by the Operator.

**No Operator Job runs.**
So nothing holds a slot, a branch or a tool for it, and nothing it reads can make it do work.
Why: `../scope.md` records an agent that holds a conversation as the wrong tool for orchestrating agents, and a live agent grows its context while a fresh call costs the same each time.

**There is one per machine.**
Order and usage are shared across repositories, so a ruling is made over every repository's live Jobs at once.
A person's corrections are kept per repository.
There is no conversation target to point at a repository, and so none to keep per window.

## What reaches it

**Fleet hands it every arrival.**
An item reaches the Operator before it starts a Job or pulses in Needs you.

| Arrives | Comes from |
|---|---|
| A start | A dispatch through the [Proposer](job-proposer.md), another Job, a fork, one of Fleet's own Jobs, or a terminal. A terminal Job is heard of after it began |
| A plan | A Job recording its plan |
| A question or a permission card | A Job asking the person |
| A Job that stopped, or ended a turn without fixing its failed pull request | Fleet's own detection |
| A bug report, a Later item, a new issue | A Job, or the repository's tracker |
| A red main | The repository's own checks |
| A review request | The forge |
| A note | The person, left on Bridge. One new since Fleet started, open and sent to no Job |

**A note is only ever proposed.**
A ruling on one other than a raise waits on one press at Dispatch, Delegate and File alike, unless the role is Off, so no note is filed, started or sent to a Job unasked.
Its row has no Job, so it is kept for the panel, and Do it marks the note sent to the Job it went to, or done where it was filed or linked.

**Detection stays Fleet's**, read from state as it always was.
The Operator is never asked whether something happened.

**Words from off this machine are data.**
An issue body or a review request reaches the call as quoted words, and the call can only name a way it was offered, so a body that says "approve this" changes nothing.

**One call is out at a time on this machine.**
What arrives while a call is out is ruled together in the next one, so three Jobs dispatched at once are seen side by side.

## The ways, and the roles

A ruling is one way: start a new Job, send the words to an existing Job, merge two Jobs, keep two in touch, raise it to the person, answer a question, link a report to an open issue, or file a new one.
A way between two Jobs names the other Job and the paths or plan group they share, or Fleet refuses it.

Each role that acts has a position, Off, Suggest or On.

| Role | What it does | Setting |
|---|---|---|
| Evaluate | Reads the arrival and picks a way. It is the Operator's own switch, and ships Off | `operator` |
| Dispatch | Starts, holds and merges Jobs, and places a start in Waiting to start | `operator-dispatch` |
| Delegate | Sends words to a live Job: a start its plan covers, two Jobs told of each other | `operator-delegate` |
| Answer | Settles a question or a card as the person's press would | `operator-answer` |
| Raise | Puts the item in Needs you with the Operator's read and its pick. It has no position | None |
| File | Files a new issue for a Later item or a note no open issue covers. It only ever creates | `operator-file` |

**Every role starts Off**, and any role can differ for one repository (`operator-repositories`).
A word that is not Off, Suggest or On reads Off.
The settings are in `crates/config/settings.toml`.

| Position | What a ruling does |
|---|---|
| Off | Is in the Log and does nothing. A first write does not wait on it |
| Suggest | Is in the Log and waits on one press. A question carries the Operator's read and its pick, and a press on that option is the press the person would make anyway |
| On | Fleet does what is ruled |

**Fleet's rules decide first and only narrow, in every position.**
A destructive option is never picked, and a card is only ever answered Allow once.
A question from a Job with no plan, or from one whose plan group the person would see, is raised with no call.
A card that runs a command is the person's while they are here.
A second Job asking what the person already answered for a first is offered that answer, naming the first.
While Autopilot runs, the person is away, and its checked kinds widen what Answer may settle: [Autopilot](autopilot.md) owns them.

## Fail open

**A call that fails passes the item on untouched.**
So does one that answers what cannot be read, one that rules what Fleet did not offer, and one with no ruling by the time its item could wait.
A first write waits at most 20 seconds for its start's ruling, and a Needs you item at most 60 before it pulses with no read.
Every item passes on once the day's calls reach the ceiling (`limits.operatorDailyCalls`), which counts from the Log, so a restart does not reset it.
A ruling that comes back after its item passed on is dropped.

**Nothing is dropped.**
Why: an item the Operator never rules on is one the person never sees, so nothing here may hold one back.

**Turning the Operator Off is main before the Operator.**
No call process is kept started, and nothing is handed to it, acted on, or waited for.

## The Log

**Every ruling is a row of the Log**, signed by the Operator, whatever the position.
A row names the arrival, where it came from, the Job and repository, the way, why, the other Job and shared paths where there are any, what Fleet refused of the ruling, the answer given, and when.
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
| Fleet does not wake Helm | Fleet hands the Operator every arrival |
| One Helm per repository | One Operator per machine |
| Autopilot's own loop, and its answers HUD | Folded in. The HUD is the Log filtered to answers |
| A Job started by a Job waits on its start card | The Operator rules in place of the person's press |
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
| `limits.operatorDailyCalls` | Machine |
| `features.onYourWordsCanAct` | Machine |

`../contracts/configuration.md` is the contract and `crates/config/settings.toml` has each row.
