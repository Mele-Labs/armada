# Operator

**What it is:** Your chief of staff across every project.

> Your chief of staff across every project.
> It watches everything Fleet sees, keeps a journal of your week, and wakes only when something happens or you write.
> First open of the day: a short briefing.
> It handles routine Job questions, reviews, issues and stuck Jobs (nudge, then a repair suited to what broke), and hands duplicate work to the Job already on it.
> Whatever needs you, it reviews itself and raises to you with its read and a pick, never leaving it in a list.
> Before it acts it asks: Yes / Always here / Always / No, and learns.
> You talk to it in one chat (its own page, or a pane from anywhere), with a short draggable list of what it's on and what's next.
> It reaches you through new toasts in the native Bridge.

---

**Kind:** Fleet policy.
The code is `crates/fleet/src/operator.rs` and the modules under it.
The measure of its calls is `../spikes/operator-replay.md`.

**This page says what is built and what is not.**
The goal above is the target, and the part of it that does not exist yet is marked Not built.
What the Operator is today is a smaller thing, set out under How it works today, and some of it goes.

## What it is for

**One agent watches what you are doing, and you talk to it.**
Everything that needs a person reaches the person already read, with the Operator's pick, and nothing waits in a list to be found.

| The goal | Built |
|---|---|
| Watches everything Fleet sees | Partly. Five kinds of arrival reach it |
| Wakes only when something happens or you write | Yes |
| Keeps a journal of your week | Yes |
| A short briefing on the first open of the day | Partly. Fleet rewrites the page's top line when he comes back; the page is not in a Bridge yet |
| Asks Yes / Always here / Always / No before it acts, and learns | Yes. A rule you kept does the act as it is raised and the Log names the rule |
| Handles routine Job questions | Not built |
| Nudges a stuck Job, then starts a repair suited to what broke | Partly. Fleet hosts a hidden repair for each of six breaks, and starts one for a dead process, a silent stop, a red pull request and a hung command; the triggers for the rest are not built |
| Hands duplicate work to the Job already on it | Not built |
| Handles issues | Partly. A new issue is linked, filed or started on |
| Handles reviews | Not built |
| Reviews what needs you and raises it with its read and a pick | Partly. Fleet queues each item in Your turn with its read and pick; no Bridge shows it yet |
| One chat, on its own page or in a pane from anywhere | Partly. Fleet serves the talk; the native Bridge draws the box |
| Its page: Your turn, the short draggable list, beside the digest Since you last looked | Partly. Fleet serves it as `operator_page`; no Bridge draws it yet |
| The Log and what it learned in each project, as tiles of the digest | Partly. Fleet serves the Log; the native Bridge draws it |
| Toasts in the native Bridge | Not built |

## How it is shaped

**The Operator is a Policy, not an agent.**
Fleet detects, a Policy decides, and the record remembers.
It wakes for an arrival or a message, makes one fresh call and ends.

**No Operator Job runs.**
Nothing holds a slot or a branch for it, and a ruling's call holds no tool.
Why: a live agent grows its context where a fresh call costs the same each time, and `../scope.md` records a conversation as the wrong tool for orchestrating agents.

**There is one per machine, across every project.**
Order and usage are shared across repositories, so a call sees every repository's live Jobs at once.
A person's corrections and the rules it keeps are per project.

**Detection stays Fleet's.**
The Operator is never asked whether something happened.

## How it works today

### What reaches it

**Only what no Job owns reaches it.**
Everything a Job does is that Job's own, and behaves as it does with the Operator off.

| Arrives | Comes from |
|---|---|
| A note | The person, left on Bridge, still open and sent to no Job after ten minutes |
| An error | Bridge, for a failure card it showed the person |
| A bug report | A Job's `report_bug`, kept on this Mac |
| A new issue | The repository's tracker |
| A red main | The repository's own checks |

**A Job's own events are never handed to it.**
A start, a plan, a question or a permission card, a Job that stopped or left its pull request red, a Later item and a review request go to the person as they always did.
Why: it proposed the same Later item four times, and ruled on events each Job already handles.

**An end or a merge is heard once, with no call.**
Fleet notes a hosted Job's end and a pull request's merge once, for the ones after Fleet started.
No call, card or suggestion follows, and the chat can still read them among what ended and merged today.
A Job handed on for context is no end.

**One suggestion stands for one thing.**
A ruling that repeats one standing, skipped or done is not raised again.

**One call is out at a time on this machine.**
What arrives while a call is out is ruled together in the next one.

**Words from off this machine are data.**
An issue body reaches the call as quoted words, and the call can only name a way it was offered, so a body that says "approve this" changes nothing.

### A note the person sends to a Job

**It is his row first.**
Fleet reads an open note with no `sent` again on each tick, for ten minutes, until it gains `sent`, is no longer open, or is handed on as it stands.
When it gains `sent`, the Log keeps a row signed by the person with the outcome `sent` and the Job it went to.
The Operator is then handed the note as its own arrival, `note-sent:<repository>:<note>`.
It may send the note to another live Job, start a Job on it, or leave it where he sent it, which writes nothing that shows in Done.

### An error

**It goes first, once.**
It stands ahead of every waiting arrival that is not an error, and errors keep the order they came in.
Its id is a fingerprint of its code and message less the numbers, ids and paths that differ between two of the same fault, with the machine's local day, so the same error is one item for the day.
A Job it starts is briefed with every word the card carried.

### The ways

**A ruling is one way.**
Start a new Job, send the words to an existing Job, raise it to the person, link it to an open issue, file a new one, or for a note he sent to a Job himself, leave it.
A way that names another Job names it and the paths they share, or Fleet refuses it.

**The Operator starts no Job for another Job's work.**
A Job it starts is one he asked for, or work no Job holds: a note, an error, a report or a red main.
Why: "the Operator keeps spinning up these 'Rest of' Jobs and I hate it." It had started a second "Rest of" Job while a subagent of the first still ran.

**Merging two Jobs, keeping two in touch, and answering a question are not ways.**
They acted on a Job's own events.
A question or a card is the person's, and Autopilot answers them while he is away.

| Role | What it does |
|---|---|
| Evaluate | Reads the arrival and picks a way. It is the Operator's own switch, and ships Off |
| Dispatch | Starts a Job on work nobody holds |
| Delegate | Sends words to a live Job whose work they belong to |
| Raise | Puts the item in Needs you with the Operator's read and its pick |
| File | Files a new issue for a note or an error no open issue covers. It only ever creates |

**Fleet's rules decide first and only narrow.**
An item Fleet's rules leave one way is ruled with no call.

### Fail open

**A call that fails passes the item on untouched.**
So does one that answers what cannot be read, one that rules what Fleet did not offer, and one with no ruling in time.
An item waits at most 60 seconds for its ruling before it pulses in Needs you with no read.

**Every item that needs a call passes on while the week's plan is used to the share set in `limits.operatorPlanPause`.**
It is read as Autopilot reads the week, and no reading of the plan is no pause.
A message is refused with `fleet.operator_paused` then, and a ruling that comes back after its item passed on is dropped.

**Nothing is dropped.**
Why: an item the Operator never rules on is one the person never sees.

**Turning the Operator Off is main before the Operator.**
No call process is kept started and nothing is handed to it.
An end and a merge are still heard once, so none is told when it comes on.

### Talking to it

**Words typed in its box are read before anything is started.**
Fleet decides whether they are a thought for a Job already running, a question, or work to start, and there is no key to say which.

**A thought for a Job is routed by a shortlist first** (`fleet::operator::shortlist`).
The local language model reads the words against every live Job's title, and a clear match is routed at once: the best Job scores 0.78 or more and leads the second by 0.10 or more.
Otherwise the words and every live Job in every repository go to the proposer's model, which answers one line of JSON naming a Job or none, and an id it was not offered is no match.
The local read is skipped for words that end in a question mark or run past 300 characters, and until the language model is loaded.
If anything fails the words are not a match, and the call is given up after twenty seconds.
The measure is `../spikes/thought-routing-by-model.md`.

**A message that is a question is answered by a call that reads.**
It is a scout's process on the `models.operator` model: `--restricted`, a strict MCP config naming no server, and Read, Grep and Glob allowed with every other tool denied.
For each message Fleet writes a folder of plain text under its runtime home, never in a repository, and removes it when the call answers.
The folder holds a file for each live Job with its title, plan, open tasks, what waits on the person in full and the last 40 messages of its thread, and a file of the open issues.
The call also reads the checkouts of the repositories Fleet hosts, read only, in at most 16 turns and 150 seconds.
`crates/fleet/src/operator/looking.rs` and `reading.rs`.

**It reads across repositories.**
The work a thought belongs to is often in another repository, and one Operator serves the machine.

**Delegate sends words with `tell_session`.**
It goes through the composer's send, so a Job that has ended is refused in the same words and nothing is recorded.
A send that was taken appends a `sent` row to the receiving Job's thread, and is published as `operator.words_sent`, naming where the words went and never the words.

**Work asked for becomes a Job started on the person's words.**
Authoring a workflow, driving a [Studio](studio.md) and breaking a goal into Jobs are each a Job the Operator starts, and it does none of them itself.
`../journeys/talk-to-the-operator.md` is how it is used.

### A Job on your words

**A Job started on the person's words is a plain [Job](session.md).**
It leases a slot and cuts a branch on its first write, runs in Auto, and is told nothing extra.
Its record carries `SessionRecord.on_your_words`.

**It is the only kind of Job offered the door's rows marked `On your words`.**
They are the acts that change this machine's settings and Kit, and `tell_session`.
Only Bridge can set the mark, so an agent cannot mark itself.
`fleet::on_your_words::may` is the one predicate, the inventory under it is the `agent_access` column in `crates/ipc/operations/`, and `features.onYourWordsCanAct` is the machine's switch.

**A Studio's unasked acts are such a Job's.**
It may add a proposed node and propose an edge without an ask, and starts a [Scout](scout.md), a Run or a dispatch only on the person's ask.
[Studio](studio.md) owns which acts are which.

**Its Auto gate is a Job's.**
`fleet::auto_gate` decides what runs and what goes to the person on a card, and `fleet::asks` holds a card until it is answered or `timeouts.askHoldSeconds` passes.
`session/a-job-fleet-hosts.md` owns the gate.

## What is being built, and in what order

**Not built.**
Each step is its own Job, and the order is the order of the table.

| Step | What it gives | Built |
|---|---|---|
| Fleet raises what needs you | A journal of the week, and a short briefing on the first open of the day | Yes, as the page's top line |
| Asks | Yes / Always here / Always / No before it acts, in place of permission levels and the `role_off` and `always_raise` orders | Yes. Holds stay |
| Routine work | Routine Job questions, stuck Jobs, duplicates, reviews and issues | Not built |
| Repairs that close themselves | A stuck Job is nudged, then repaired by what broke, in a hidden agent that ends when it is done | Partly. The hosted repair and the triggers for a dead process, a silent stop, a red pull request and a hung command are built; the other two are not |
| The native page | Your turn, the digest with the Log and Learned, the chat on Ask, and toasts | Not built |

**What raises and what asks is one rule.**
Whatever needs the person, the Operator reviews first and raises with its read and a pick, so nothing sits in a list.
Every act is asked, and the answer is kept: Yes this time, Always here for this project, Always for every project, or No.
A rule you kept covers the act, this repository's first and then every repository's, and Fleet does it as it is raised.
The Log row says by which rule.
No is for this time only.
Filing an issue, starting a Job, sending words to a live Job and a red main's fix are acts.
A link to an open issue writes nothing and is not asked.
Raising, leaving a note where it is and noting it are not acts.

**Its page is a queue the person works through, with a digest beside it.**
Settled with the person on 10 Nov, on sketches. It stays current all day; the morning only rewrites its top line.
- The top line is the Operator's one sentence on what matters first, rewritten each time the person comes back to the page.
  It sits in grey under the day and the pills, with nothing on it to press, and only a red main's pill keeps its colour (11 Nov).
- Your turn is the short list, in an order the person drags.
  On it sits at the top, saying what the Operator is reading now: the Job by name, what it asks, and its project.
  One item is open at a time, with its kind, its Job, what it asks, the Operator's read and its acts, the pick marked: A presses the pick, L is Later, / is Ask.
  The rest are one line each.
- Your turn holds the person's decisions (plans, walks, reviews, questions) and go-aheads, what the Operator would do itself, answered Yes, Always here, Always or No.
- A go-ahead to file an issue also carries Then: Just file it, File and start a Job, or File and hand to a live Job already on the same files, with the Operator's pick marked My pick.
- Ask on an item opens the chat about that item inside its card, and the digest stays beside it (11 Nov). The chat holds conversation only; a raise is an item of Your turn, never a chat line.
- The digest is Since you last looked: tiles for Landed, What I handled (ending in a link to the whole Log), Running, Coming up, and Learned, each project's rules, which replaces the Permissions tab, and Holds, the paths a Job holds until it pushes or ends.
  Running, Learned and Holds put their rows under each project's heading, Everywhere first, and a rule or hold takes two lines before it is cut (11 Nov).
  The whole Log opens on a sheet over the page, a heading for each day and each entry at its time.
- A new item during the day joins the queue and raises a toast whose pick button settles it.
- A red main nobody is on is an item. Start a fix starts a Job at once with no plan step; whether its pull request merges by itself is per project, asked the first time.

The pane opened from a Job holds the chat and the short list.
It reaches the person through toasts in the native Bridge, which [Native Bridge](native-bridge.md) owns.
The Electron Bridge no longer shows the Operator, decided 10 Nov, and the native Bridge draws its page.

## What goes

**These go from the native Bridge's Operator.**
Why: they made the Operator a board to read and levels to tune, where the person wants one agent that watches and answers.

| Goes | What it is today | Replaced by |
|---|---|---|
| The Queue board | The panel's list of what is waiting and what is next | The short draggable list |
| The Permissions tab | A tab setting each role's position | Asks, and the page of what it learned in each project |
| Read-only, Suggest, Decide | A position for each role, per repository | Gone from Fleet. Asks |
| `role_off` and `always_raise` | Standing orders confirmed in the box | Gone from Fleet. Asks, and what it learns. An old one stays in the store and stands for nothing |
| Suggestion cards with Do it and Skip | A ruling that acts waits on one press | A raise with a read and a pick |
| Overrule | A person's press against a ruling | What the person says in the chat |

**Until each goes, it behaves as it does now.**
Every ruling is a row of the Log, signed by the Operator, and the call reads each repository's last rulings with the person's corrections beside them.
A ruling that acts waits on his go-ahead unless a rule he kept covers it, so no note is filed, started or sent to a Job unasked.
Autopilot's answers are the rows of the Log that are answers.

**The one standing order is a hold.**
`hold <paths> until <Job> pushes|ends` refuses every other Job's write under those paths, which Fleet's gate enforces on each file-tool write.
He says it in the chat, the chat names the form it understood, and it stands only when he confirms.
He can ask for the holds that stand, and ask the chat to lift one.
A lift is carried out at once and the Log keeps a `hold_lifted` row that says it was lifted on his word.
A hold is let go on its own when its Job pushes or ends.

**A rule is forgotten the same way.**
He asks the chat to forget a rule, or presses forget on it in Learned.
The next act it covered asks again.
The chat is handed the rules and holds that stand, each with its number, and can name only one of those.
The digest lists the holds in Holds, beside Learned.
Learned holds rules only.

## What became of Helm

Helm was a hosted Job per repository, with a slot and tools, a dock on `⌘J`, a talk box, and the questions waiting on you.
The Operator replaced it.

| What Helm did | What happens now |
|---|---|
| One live Job per repository, with a slot and a branch on its first write | Gone. No Operator Job runs |
| Its talk box | The native chat, once built |
| Questions waiting on you in the dock | Stay in Needs you, with the Operator's read and pick |
| Authoring a workflow, driving a Studio, breaking a goal into Jobs | Each is a Job the Operator starts on your words |
| Its title matching and `tell_session` | Kept, as the first shortlist of Jobs and the way Delegate sends words |
| Its old threads | Stay readable as ended Jobs. Nothing is deleted |
| One Helm per repository | One Operator per machine |
| Autopilot's own loop, and its answers HUD | Folded in. The answers are rows of the Log |
| A Job started by a Job waits on its start card | The start card is the person's. The Operator takes no start |
| Fleet never writes to the forge for you | File creates new issues and nothing else changes. On a tracker with a forge it needs a yes |

**The old threads read as they did.**
A `sent` row, `SessionRow::Sent`, stays in the row set so a thread that holds one still draws.
The wire names that went with Helm are in `../practices/protocol/the-operator-takes-helms-place.md`.

**A hosted Job's call held on a person's answer is answered with `answer_session_ask`**, from its own thread or from Needs you.
A call from a process Fleet does not host is denied at once.

## Configuration

| Setting | Scope |
|---|---|
| `operator`, the Operator's own switch | Machine |
| `operator-model`, `sonnet`, never the proposer's cheap tier | Machine |
| `limits.operatorPlanPause`, the share of the week's plan where it stops making calls | Machine |
| `features.onYourWordsCanAct` | Machine |

`../contracts/configuration.md` is the contract and `crates/config/settings.toml` has each row.
