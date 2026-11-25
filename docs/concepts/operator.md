# Operator

**What it is:** One agent on your Mac that watches every project, puts one decision at a time in front of you, and does the rest.

> One long-running agent session per Mac, with Fleet's tools, across every project Fleet serves.
> It puts one card in front of you at a time: one decision, two to four options, its pick first, a cost on each.
> Everything else goes to a feed of what happened since you last looked.
> How much it does without asking is a value for each project.
> When its context fills it hands on to a fresh copy.

---

**Kind:** A hosted agent session that Fleet starts and keeps.
Its rules are `crates/adapters/src/operator_rules.md`.
Its page is in the native Bridge, and [Native Bridge](native-bridge.md) owns how the page is drawn.

## What it is for

**One agent watches what you are doing, and you talk to it.**
What needs a decision reaches you as a card with its read and its pick.
What only happened goes to the feed.
Nothing waits in a list to be found.

## How it is shaped

**It is one agent session, not a call per arrival.**
It runs for as long as its context lasts, holds Fleet's tools, and keeps what it has learned of the week in that context.
It reads code and pull requests, starts, messages and ends Jobs, and lands pull requests.

**There is one per Mac, across every project.**
Order and usage are shared across repositories, so it sees every project's Jobs at once.
What it may do unasked is set for each project.

**It hands on when its context fills.**
A fresh copy takes over with the open cards, the saved level of each project, what it waits on and what you told it in the talk.
Its `session_id` is on the page, so the page follows the copy in front.

**It is not a Job.**
It has no Needs you list, no plan, no preview and no end, so the door offers it none of a Job's own acts: `waiting_for`, `all_done`, `finish_up`, `record_plan`, `offer_preview`, the Later items, `write_feature_sections`, and the asks a Job sends it.
A call to one is refused, whatever it was told.
Its prompt carries its own rules and no Job's id, no Logbook and none of how you work with Jobs.
The mod sends none of its replies back to a list, and it runs with the mod's `Operator` output style whatever style you picked.

**Detection stays Fleet's.**
The Operator is told what Fleet saw and is never asked whether something happened.

## The page

**One card is in front at a time.**
A card is one decision: a title, its read, two to four options with the Operator's pick first, and a cost on each option.
Keys 1 to 4 pick an option.
A card never holds a count or a status report.

| You do | What happens |
|---|---|
| Press an option, or its key | The card leaves the front for the top of the feed at once and the next one comes up, before Fleet has answered. If Fleet refuses, the card comes back in front with Fleet's reason. The Operator is told which card and which option, and does it now |
| Press Review on a card for a Job's plan, sketch, document or draft, or its key 1 | The Review panel opens over the page with that item chosen, and the Operator's read and pick above it. The card stays in front until he answers in the panel. Esc shuts the panel and gives the keys back to the card |
| Answer a Job's build to try | The Job's own Ready to try card stands in the card's place, as the Job's thread draws it, with no Operator frame: Looks good, Needs rework, Show me how, Remind me and Refresh preview, answered as in the thread. Keys 1 to 3 are Looks good, Needs rework and Show me how. The card keeps its whole height, and the talk gives up room for it. Its chip in Next up reads Ready to try |
| Discuss (`/`) | The composer under the card takes focus, with a tag naming the card. The card stays in sight |
| Send in the composer with the tag | His words join the talk under the card as a talk about that card, after a row `About:` and the card's title where they turn to a card. The talk runs until the item settles or the Operator puts up a new card |
| Send in the composer with no tag and no kind chip | His words are a talk to the Operator about no card (`talk_to_operator` with no `about`). They join the same talk with no `About:` row, and the Operator answers there. No card settles for them. A talk carries words only |
| Send in the composer with Idea, Feature or Bug picked | His words are work for the Operator: a dispatch with that kind, which the Operator may start, fold into a live Job, ask about or keep as an idea. A toast says which |
| Press the folded card | It opens above the talk, and stays open until the talk grows or you send again |
| Show (`⌘Enter`) on the new-card strip | The newest card that came comes in front, open, and the strip goes |
| Snooze (`S`) | The card goes back until its time, hidden until then |
| Open Next up, at the top of the right side | The cards queued behind the one in front, then the ideas the Operator kept, newest first. Pressing a card brings it to the front. The keys go into it: ↓ and ↑ move, Enter brings, Esc folds it |
| Press a card working at the top of the feed | The card comes to the front to be read: its read, every option with your pick marked and none to press, and its steps. It has no Snooze and no Discuss. Esc puts the card that was in front back |
| Press a kept idea in Next up | The idea comes to the front as a card with two options: Start it, which sends it as a dispatch, and Remove, which drops it. It has no Discuss and no Snooze. Esc puts the Operator's own card back |
| Press See what it is doing on the status card | The Operator's own session opens as a Job's view does, with its thread. Esc, or the Operator place in the strip, comes back to the page |
| Press Restart on a stopped status card | The Operator is switched off and on again. The card reads Starting until Fleet has started a new session |

**The talk is the Operator's own session, under the card.**
The card, the talk and the composer share one width, with Next up and the feed in a panel of their own beside them.
The talk is the session's thread, drawn as a Job's thread is, and follows its end unless you scrolled up.
While it overflows its room, the card in front folds to one line: its title, the Operator's last act and a chevron.
A card too tall for its room scrolls its read in its own place and keeps a line of it in sight.
Its answers and its foot, Snooze and Discuss, stay whole at its bottom edge, and the talk gives up room for them.
The talk starts below the card's edge on a whole row, and the box under it is always whole.

**A card that comes while the card is folded, or the talk is in use, waits in a strip.**
The strip stands over the talk: a bell, how many cards came, the newest one's title and Show.
It glows once as a card arrives, then holds.
Without the strip, a card that comes where nothing was in front would take the front from under you.

**With nothing in front, a status card stands in the card's place.**
It reads Nothing needs you, with a chip for where the Operator's session stands, and a line for what it is doing.
The state comes from the Operator's own session record, never from the page, so a quiet page tells a working Operator from a broken one.

| Chip | When | The line |
|---|---|---|
| Working, in green | A turn is running | What Fleet saw it doing, such as `Editing gate.rs`, and for how long. With nothing seen, `In a turn`, and for how long |
| Watching, in the accent | Between turns | `Between turns. Last heard from 5m ago`, from when Fleet last heard from it, else its last turn, else when it last saw the session |
| Stopped, in red, with a red frame | Its standing is stopped, its session ended, or Fleet recorded a stop | Why, in plain words. The title reads `The Operator stopped 25m ago` and the button reads Restart |
| Starting, neutral | The switch is on and no session exists yet | `Fleet starts its session on the next tick`, with no button |
| Off, muted | The switch is off | `Every Job asks you directly`. The title reads `The Operator is off` and the button reads Switch on |

**A stopped Operator comes in front of any queued card.**
Its card is red on its frame and edge, with the reason and Restart.
The talk and the composer dim, and the keys that answer a card wait, until it is back.

Under the line, `Last act 5m ago` names the newest entry of the feed.
It is left out when the feed is empty.
The head's button, See what it is doing, opens the session as a Job.
**Restart goes through the switch.**
Fleet has no act that starts a stopped Operator's session again: it ends the session when `features.operator` goes off and starts one on the next tick once it is on, so Restart saves the setting off and then on.

**A card you picked is at the top of the feed until it is settled.**
It shows the option you picked as its title, the card's own title under it, and a line for each step the Operator reports, such as a Job started, a pull request opened or a merge.
A turning ring marks it as live, a check marks each step done, and the newest step keeps the ring.
When the Operator settles it, the feed shows it settled.

**A picked card comes back to the front when it cannot be done.**
The Operator reports that it failed, with the reason, or nothing settles it within 15 minutes of your press.
It comes back ahead of every other card with the reason on it, and you pick again.
Its kind chip reads Did not settle in red, its frame is red, and its read starts with one line: what you picked, then why it came back.
Its options are live, and Next up shows the same chip on its row.
A picked card is never lost: until it is settled it is at the top of the feed or back in front.

**A talk about a card ends in one of two ways.**
The Operator settles the card, or it puts up a new one.
It may revise the card in front while you talk.

**Next up is what is queued behind the card in front, and the ideas the Operator kept.**
It is a section at the top of the right side, above the feed, folded until you open it, with its count beside its name.
The count includes the kept ideas.
A snoozed Job raises nothing until it wakes.

**The right side is a feed of what happened since you last looked.**
It holds what the Operator dispatched, merges, Jobs that ended, and what Jobs asked or told it.
A line in the feed is information.
A decision is never a line in the feed.

**The pages the Operator keeps for him are read in Review, under Operator.**
A backlog report and any other page the Operator offers stand in the Review panel as a group named Operator, beside the Jobs' groups.
The Operator is listed nowhere else as a Job.

**Job questions stay in each Job's own Needs you.**
The Operator is not handed them, so only you answer them, or Autopilot while you are away.

## What it may do

**It is a value for each project, `operator_acts`.**

| Value | The Operator |
|---|---|
| `act_then_tell` | Acts, then says so in the feed |
| `ask_before_starting` | Raises a card before it starts any Job, and acts on the rest as `act_then_tell` does |
| `only_when_told` | Does nothing unasked |

**A project with no value asks once.**
The Operator raises a card with the three levels, its pick first, and keeps your answer as that project's value.
Armada starts at `act_then_tell`.

**Whatever the level, a change you would see or a decision that is yours is always a card.**
The level moves how much it does unasked and never lowers this.

## What reaches it

**Only what no Job owns reaches it.**
Everything a Job does is that Job's own.

| Arrives | Comes from |
|---|---|
| A note | You, from Bridge |
| A press, Snooze or a message in the composer | You, on a card. Each reaches the session as a message that names the card |
| A bug report | A Job's `report_bug`, kept on this Mac |
| A conflict | A Job's `raise_conflict`, between its work and another Job's |
| A landing question | A Job's `ask_held`, whether a pull request or a branch should land |
| A message from a Job | Any Job's message to it. It may answer in that Job's thread |
| A new issue | The repository's tracker |
| A red main | The repository's own checks |

**A Job's own events go to you as they always did.**
A start, a plan, a question, a permission card, a Job that stopped and a review request are not handed to it.

**Words from off this machine are data.**
An issue body reaches it as quoted words and never as an instruction.

### A Job's two asks

**A Job may send the Operator two typed things.**
`raise_conflict` says two Jobs' work cannot both stand: `with`, the other Job's address, `files` and `wire` (at least one), `mine`, `theirs` and `breaks`, one sentence each.
`ask_held` asks whether a pull request or a branch should land: exactly one of `pull` and `branch`.
`withdraw_held` takes the question back, with the same one of `pull` and `branch`: the row goes and the Job is sent nothing.
He may dismiss a landing question instead, and the Job is sent that he dismissed it with no answer.
Fleet checks every part and refuses with `fleet.conflict_refused` or `fleet.held_refused`, in a sentence that names the part.

**Both arrive in the Operator's session.**
It may answer in the Job's thread, or put up a card.

## How it acts

**It acts through Fleet's tools and its own acts.**

| Act | Does |
|---|---|
| `raise_card` | Puts a card in the queue: `kind` (`deal` or `raise`), `title`, `read`, two to four `options` each with a `label` and a `cost`, the first its pick, and optionally `sessions`, `repository` and `talk_of` |
| `revise_card` | Changes a standing card's read or options |
| `report_card` | On a card you picked: a `step` it took, or `failed` with why, which puts the card back in front of you |
| `settle_card` | Closes a card with what was said |

Only the Operator's session may call these.
A card's id is what the press, Snooze and the composer name.

**It writes to a forge only to land a pull request.**
It never comments on, labels or edits an issue or a pull request for you.

**It reads across repositories.**
The work a thought belongs to is often in another repository, and one Operator serves the Mac.

## A backlog report

**It reads a repository's issues and writes none.**
`list_issues`, `get_issue` and `search_issues` are offered to its session alone, each naming its repository.
Nothing that writes an issue is: only your press of a page's `close` button closes one.

**It writes one page you review and revise with it.**
The page is `.armada/documents/<slug>.html` in its own folder, kept and waiting on you as a Job's page is.
It groups the issues by screen or area, with a verdict and the evidence for each.
Your notes revise the same page.

**Its buttons start the work.**
A `start` button names its repository, since the Operator stands in none.
For the issues it found already done in native, it puts one `close` button on the report, listing them.
Your press closes them in that repository's tracker, and the Operator is told what was closed and what was not.
The Operator never closes an issue itself.
Anything else is a `message` button, whose press tells the Operator.

**It is an extra path.**
The backlog stays readable on the Issues screen without the Operator.

## With the session not running

**Fleet then does what it did with the Operator off.**
The switch, a session that has not started yet and one that has ended all read the same.

| Arrives | What happens |
|---|---|
| A dispatch | The Job starts as he asked, with its sends |
| A conflict or a landing question | It stands in the sending Job's own Needs you, as `job_conflict` or `job_held` |
| A report, a note, an error, an issue and a red main | Nobody is told, and each stays where it is kept |
| His words in the composer | Refused with `fleet.operator_session_not_running`, since nothing would hear them |
| A press on a card | Refused with the same code, and the card stands |

**Fleet's own items stay.**
A red main nobody is on and a disk hold are in Your turn whether or not the session runs.

## Holds

**The one standing order is a hold.**
`hold <paths> until <Job> pushes|ends` refuses every other Job's write under those paths, which Fleet's gate enforces on each file-tool write.
He says it in the talk, the Operator names the form it understood, and it stands only when he confirms.
He can ask for the holds that stand, and ask it to lift one.
A lift is carried out at once and the Log keeps a `hold_lifted` row.
A hold is let go on its own when its Job pushes or ends.

## A Job on your words

**A Job started on a person's words is a plain [Job](session.md).**
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

## What went

**These go from the Operator.**
Why: they made it a Policy that made one call per arrival and a board of levels to tune, where the person wants one agent that watches and answers.
The code that served them is removed, and Fleet rules nothing now.

| Went | What it was | Replaced by |
|---|---|---|
| The Policy and its one-shot call | A fresh call for each arrival, with no tools | One long-running session with Fleet's tools |
| Your turn rulings | Each arrival ruled into a way: start, send, raise, file, close, group, leave | The Operator acts by the project's `operator_acts` and raises a card for what is yours |
| Rulings from precedent | A question answered by following one of his earlier answers, with Undo | None. A question stays in its Job's Needs you |
| Kind chips | Idea, feature or bug marked on a dispatch as a hint to route it | A chip picked makes a send a dispatch, and nothing routes by its kind |
| Auto-starts from reports | A bug report ruled into a Job at once | The report reaches the Operator as a message |
| The per-item review | A reading call on each plan, pull request and walk before it reached Your turn | None. A plan, a pull request and a walk wait in their Job's own Needs you |
| Yes, Always here, Always and No | Four answers to each go-ahead, and the rules kept from them | Options on a card, and `operator_acts` for each project |
| Learned rules and the digest's tiles | Each project's kept rules, and Landed, What I handled, Running and Coming up | The feed, for what happened, and `operator_acts` for what it keeps |
| The queue board, Permissions tab, Read-only, Suggest and Decide, `role_off` and `always_raise` | Positions and standing orders for each role | `operator_acts` |
| Suggestion cards with Do it and Skip, and Overrule | A ruling that acts waited on one press | A card with options, and the talk |
| The Your turn list, dragged into order | The short list he ordered | One card in front, and Next up behind it |

The measure of the one-shot rulings is `../spikes/operator-replay.md`.

## What became of Helm

Helm was a hosted Job per repository, with a slot and tools, a dock on `⌘J`, a talk box, and the questions waiting on you.
The Operator replaced it.

| What Helm did | What happens now |
|---|---|
| One live Job per repository, with a slot and a branch on its first write | One Operator session per Mac |
| Its talk box | The composer under the card |
| Questions waiting on you in the dock | Stay in Needs you |
| Authoring a workflow, driving a Studio, breaking a goal into Jobs | The Operator starts a Job for it, or does it |
| Its old threads | Stay readable as ended Jobs. Nothing is deleted |

**The old threads read as they did.**
A `sent` row, `SessionRow::Sent`, stays in the row set so a thread that holds one still draws.
The wire names that went with Helm are in `../practices/protocol/the-operator-takes-helms-place.md`.

**A hosted Job's call held on a person's answer is answered with `answer_session_ask`**, from its own thread or from Needs you.
A call from a process Fleet does not host is denied at once.

## Configuration

| Setting | Scope |
|---|---|
| `operator_acts`, what it may do unasked, one of `act_then_tell`, `ask_before_starting` and `only_when_told` | Project, read and set from the page's wire |
| `operator`, the Operator's own switch | Machine |
| `operator-model`, `sonnet`, the model of its own session, never the proposer's cheap tier | Machine |
| `features.onYourWordsCanAct` | Machine |

`../contracts/configuration.md` is the contract and `crates/config/settings.toml` has each row.
The wire is `../practices/protocol/the-operators-page-fleet-keeps.md`, and `../journeys/talk-to-the-operator.md` is how it is used.

## The feature catalog

Fleet tells the Operator what the project's feature library needs from it: a feature a Job named that the library lacks, an area that grew, and his words and notes that are asks.
Each is kept and carried out with the Operator's own tools, and every decision is a card.
Fleet keeps Armada's own seed itself, so the library is not empty while the Operator is off.
[Feature catalog](feature-catalog.md) says what each holds and what the Operator does with it.
