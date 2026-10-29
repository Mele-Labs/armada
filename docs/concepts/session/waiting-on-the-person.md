# Waiting on the person

**Part of:** [Job](../session.md).

**A job says what it is waiting on the person for with `waiting_for`**, a tool on the agent's door that replaces the whole list each call (`items`: `id`, `text`, an optional `act` of `answer`, `approve_pr` or `run` with a `target`, optional `options`, and optional `steps`, a list of short lines); an empty list clears it.
Fleet keeps the agent's items in `session_waiting`, stamps `since` and keeps it for an unchanged `id`, and **adds its own on every read** into `SessionRecord.waiting_for`: `ask:<call>` for an open question, `perm:<call>` for an open permission, `walk:<url>` for a window the job showed that no `Approved: <url>` message (of any of its takes) has followed, `window_ask:<row>` for a question asked in the window and not yet answered (*Asking in the window*), `draft:<row>` for a draft proposed with `propose_draft` and not yet settled (*Proposing a draft*), `task_done:<project>` for tasks of a project the job held that it or a pull request has finished ([Project](../project.md)).
**`task_done:<project>` is answered with Mark done or Keep open**, through `answer_waiting`, and the choice settles every task the job asked in that project without sending the agent a message.
Where an agent item names the same act and target as one of Fleet's, Fleet's stands.
**`waiting_for` refuses an item that asks the person to walk**, by a `walk` act or in its words, and tells the agent to call `show_window`: the window is the one record of a walk, and its item leaves at his Approve.
On 19 Oct 2026 Jobs listed walks in their own words, one with no window behind it and one that stayed after he approved.
Phrase rules read the act, the id and the text (`walk_asks.rs`; `docs/spikes/walk-item-classifier.md` has them measured), and an item kept before the rule is left off every read.
**It refuses an item that repeats the plan he has not approved**, such as "Approve plan v4" beside Fleet's own "Plan to approve": once the language model is loaded and while a plan is pending, `plan_asks.rs` compares each item with that text by cosine and names the plan version in the refusal.
**It refuses an item that repeats a question that lapsed** (`fleet.session_waiting_asked`): a question nobody answered inside the hold, or one kept for a walk, stays in his Needs you whole and his answer comes to the agent as a message.
The reply to a timed-out question says so, and tells the agent not to ask it again or list it; one that does anyway is refused when its option labels are the question's or its text is the question's, and an item kept before the rule is left off every read while the question stands (`lapsed_asks.rs`).
Both his answer to a kept question and to a timed-out one start with "Answering your earlier question", which `handoff_answered_later` reads for a handoff card in either case.
Until the model loads the list is accepted as it was; the walk rules read no model, since it caught no walk they miss.
**It refuses an item the owner could not act on** (*An item he cannot act on*).
**Two items that are one ask by two doors are folded to one row on every read**, once the model is loaded (*Items that are one ask*).
An ended job holds nothing.
**Bridge shows the list on the composer**, as a **Needs you** band folded to one glyph an item, named on hover; a press rides it up as the header of a panel and dims the thread.
The panel opens as a list, a row each and no act on the row; a press on a row opens that one to do its act, and Back or Esc returns to the list.
The owner, 12 Oct 2026: "when the session is not completed, the 'needs you' items in the right panel move to a panel in the thread like this that is locked to the bottom.
Maybe it's collapsed by default?" A question or permission is left out of the band, since its own card shows it.
The band stays when the agent says it is done, above the done callout; the ledger holds no section for them.
The owner, 16 Oct 2026: "I really don't like the waiting for me being in the ledger. its already in the thread.
We dont need it in the ledger." Bridge puts a Job with items under Needs you with a bell-ring mark whose tooltip lists them.
**The Cockpit raises a call for an item only while he has not been in the Job since it began waiting** (*Cockpit*, the call deck): Bridge keeps when he last viewed each Job (`localStorage`, per id), his latest message counts as being in it, and an item whose `since` is older than the later of the two is no call, though it stays in the list.
**`answer_waiting`** (`session_id`, `item_id`, and a `choice` index into the item's `options`, `text`, or `mode` of `best` or `quick`) settles one: a card item goes through `answer_session_ask`, a walk item sends the approval as the person's message, an agent item becomes a message to the job, and once that is delivered the item is dismissed as `dismiss_waiting` does, so it leaves the list without the agent restating it.
A `choice` with `text` says both: the words go with the pick, to the agent as a second line and to an ask card as its note.
`best` tells the agent to decide for itself and think it through, `quick` to just get it done: finish the work and land it, skipping the formalities.
**`dismiss_waiting`** (`session_id`, `item_id`, and an optional `reason`) drops an item for good: the id is kept in `session_waiting_dismissed`, a derived item with it never comes back, save a walk the job shows again, and an agent item with it is removed and left off if the agent states it again.
**An item the agent stated tells it so**: the Job is sent `Owner removed "<item text>"`, with `: <reason>` after it where the person gave a reason that is not blank, so the agent does not take the silence for an answer.
The owner, 14 Oct 2026: "when there is a 'needs me' item on a job, I should be able to delete it, which lets the agent know that I removed it.
Maybe I can give a reason when I do." **A walk, or a sentence Fleet read from the agent's reply, tells it too**: the Job is sent `Owner dismissed "<item text>"`, so it neither shows nor asks again for want of knowing; it comes back only if the agent shows or asks again.
The owner, 28 Oct 2026: "I should be able to dismiss any needs you item, even walks and PRs. It should let the agent know I dismissed them."
An item Fleet derived in any other way (a card, a permission, a window question, a task asked about, the first message, a done) is only dropped, and the agent is told nothing, since it never stated it.
**A closed item is kept whole.** `dismiss_waiting` keeps the item as it stood (text, source, act, options, steps, when it began) and when it was closed, beside the dismissed id in `session_waiting_dismissed`, whatever the source.
The Job's record lists them as `closed`, newest first, at most 50: each `{ item, at, ended? }`.
**`reopen_waiting`** (`session_id`, `item_id`) lets one wait again: an agent's own item goes back at the end of its list as it was stated, and the agent is told nothing.
`ended` marks one that cannot be reopened because what it stood for has ended (a permission whose call ended, a walk that was withdrawn, a done the person's message cleared, anything on a Job that has ended); reopening it is refused.
An agent reads the same list with `list_closed_waiting`.
A dismissal a Job's own process makes (a walk it withdrew, a pull request that merged) is not a close and is not listed.
**A draft item is withdrawn, not only dropped**: the Withdraw (Dismiss) on its card is `dismiss_waiting`, and the Job is sent `Withdrawn: <title>.` and `Do not propose it again.`, with the reason on the next line where one was given, and the row settles `withdrawn`.
The job takes back a draft of its own with `withdraw_draft`, and is sent nothing; both are in [Proposing a draft](proposing-a-draft.md).
The message goes before the item is dropped, so a send that fails leaves the item standing to try again.
A nothing-held id is refused as for `answer_waiting`.
**Every walk window of a job waits until the person approves it.** One item per page, `walk:<url>`, for each standing window he has not approved since it was last shown: an Approve at or after the newest `window` row showing the page or one of its takes.
Approving a newer walk leaves an older one waiting.
A message, a note and closing the window settle nothing, and a page shown again waits again.
A window the job is changing raises no item while it is changing.
A page changed after it was shown (`detail.changed` on its artifact, set with `working`) raises none even when it is ready, however often the turn ends between fixes.
It waits again, with its `since` at that instant, only when the job shows it again with `show_window`.
`dismiss_waiting` drops one, and an ended job has none.
**The job takes a walk back with `withdraw_window`** (`url`, `session_id`) when he has moved past it, as when he asked for a bigger change instead of approving.
It writes the dismissal `dismiss_waiting` writes for `walk:<url>`, so the item leaves the list and the window holds no pending plan, and it ends the walk, so the job's questions are not refused for it.
Only a window the job showed can be withdrawn.
Showing the page again with `show_window` lifts the dismissal, so a withdrawn walk waits again, and so does one he dismissed himself.
The mod's first-message note tells the agent to keep the list current, to give each item an act, and never to list a page it showed with `show_window`, which waits by itself, and puts these tools ahead of the person's own rules for reporting actions and questions in a reply.
It also says a decision is asked one way, in the prompt or in `waiting_for` and never both, that a shell command for the owner goes in a block tagged `bash` so Bridge draws its run button, and that a page is shown as a link and never a screenshot.
**An agent item that carries `steps` is a procedure**: its glyph is Steps, and its row reads its `text`, and its press opens the steps beside the thread in place of its `act`.
A `walk` stays as the page they are about, one key at the top and a second that opens it in the browser; a `run` stays as its command, drawn whole at the top with its run key in the block's corner and its run on a terminal under it that the person types into ([Running a block](running-a-block.md)), and left off where a step names it.
A step's `backticked` command is set as code in its line and drawn again under it, whole, with its own run key beside the copy key, the same run key a fenced block has, and it runs where the Job works.
A command that exits clean ticks its step, and Fleet keeps an item that has steps on the list where it would take off one that has none.
Fleet tells the Job how every run ended, so Bridge sends nothing of its own.
The owner, 15 Oct 2026, on a run item whose steps hid its command: "The copy button should be embedded in the code block instead of outside of it." Each step has a tick that is the person's own place-keeping, a step the agent got wrong is edited in place, dropped (struck through, kept) or added at the end, and the key under the reply box reads Done while the box is empty and Send once something is typed.
Done answers the item with `Done`, ticked or not; either key sends the corrections as plain lines after the reply (`Step 2 changed: "was" → "now"`, `Step 4 dropped: "was"`, `Added after step 6: "now"`).
Fleet trims each step and drops a blank one; an empty list is no steps.
The agent keeps `text` to one short line.
An agent item with none, or with `answer` and no `options` while no question is open in the thread, gets a Reply glyph, and its press opens a box under the row whose text goes to `answer_waiting` as `text`.

**A job says it has nothing left to work on with `all_done`** (`session_id` only; the hosted job holding the connection wins, as for `waiting_for`).
Fleet stamps `SessionRecord.done_since`, keeps it in `session_done`, and publishes `SessionChanged`; a second call keeps the first instant, and an ended job is refused.
It is a statement and ends nothing: Bridge draws a callout at the foot of the thread, **End job** (the ordinary close) or **Keep going** (local to Bridge, it hides that `done_since` only).
`done_since` stops standing when the person sends the job a message (`send_session_message` or an `answer_waiting` that becomes one), when the agent calls `waiting_for` with a non-empty list, and when a hosted job's turn starts again, except a turn that Fleet's own news starts (`told_news_by_fleet`: a pull request merged, a build moved), which asks the agent for nothing.
A word that asks for work, such as a failed check or a stranded slot, clears it.
The person's message also tells the agent so: where a done stood as they wrote, the agent reads one more line after their words, saying the message took back its `all_done` and to call it again if nothing is left once it has dealt with the message.
Only the agent reads the line, and the person's row keeps their words.
Without it the agent answers in prose, Fleet reads the job as replied, and the Done item never comes back.
A terminal job's mod reports a prompt going in as `turn_started`, which is the same turn-start signal; a mod from before it has none, and for that one only the first two apply.
The mod's first-message note tells the agent to clear `waiting_for`, then call `all_done` only when nothing is owed and none of its background work is running.
**`all_done` is refused while a follow-up is open** (*Follow-ups kept for Later*).

**A reply that claims a done or a walk without its call is answered with one reminder.** Fleet takes a job's word only through `all_done` and `show_window`, so where a turn ends it reads the reply's close (`reply_claims.rs`).
A close that says the work or the job is done, finished or complete, or that nothing is left, while no `done_since` stands and nothing is on the job's waiting list, sends the agent: "Your reply says the work is done, and all_done is not called.
If nothing is left, call all_done; if something is, say what."
A sentence that defers or negates it, with "once", "after", "when", "until", "if", "before", "not" or "n't", and a question, claim nothing.
A reply whose close, or a table cell near its end, asks the owner to walk, approve or leave notes on something, while no walk window of the job waits on him, sends: "Your reply asks the owner to walk something, and no walk window of this job is open.
Build each walk and call show_window for it."
A reply that claims both is reminded of the walk only, since the work is not done while a walk is owed.
The reminder goes through `told_by_fleet` and starts a turn, in which the agent makes the call or says what is left.
It is kept as the job's tell (`reminded_done` or `reminded_walk`), and the reply that first follows it is not read, so a reminder is never answered with another.
Helm and an ended job are not read, and a terminal job is reminded only while its mod is listening, as any word Fleet holds for it.
The owner reported two jobs on 21 Oct 2026: one wrote "PR 2681 is merged and the job is done" and never called `all_done`, so no Done item came, and one listed walks for him in a table with no window open, until he asked "Please open the walks".
These are phrase rules; #2692 is the language model reader that would replace them.
They learn from disputes in the meantime: a reminder the Job disputed and a cheap model agreed was wrong keeps the sentences that set it off, and a near sentence is not reminded again in any Job (*What Fleet tells a Job*).

**A turn that leaves the person's message unanswered is sent back once** (`unanswered.rs`).
He reads only what the agent writes, never its reasoning.
From his message until the agent writes him something other than a lone line starting "Status:", the job owes him a reply.
The first `AskUserQuestion` while it does is refused with: "Not asked: since his last message you have written him nothing but a status line, and he reads only what you write, never your reasoning.
Write your answer to his message, then ask this question again."
The door waits up to two seconds for the stream to reach the question first, so an answer written just before it counts.
A turn that ends still owing it sends the agent, through `told_by_fleet` and unless another turn is waiting to start, the same words without "Not asked:" and with "Write your answer to his message." to close.
Either one settles the debt, so the question asked again is put to him and the reminder's turn is not read.
A turn Fleet's own notice began owes nothing, and Helm is not read.
Job 7f7f4ae4 did this twice on 24 Oct 2026: asked why commands were recorded, it worked the answer out in its reasoning and went straight to its question, so he saw a status line and the question and wrote "you never answered me or explained".

**A job gives back what it is finished with using `finish_up`** (`session_id` only; the hosted job holding the connection wins).
It releases each slot the pool holds for the job as `armada worktree release` does, which refuses a tree with uncommitted files or commits that neither its branch's remote nor the base has; stops the servers running in each slot it released; and deletes the local branches those slots were on, the one the job was leased and the head branch of each pull request it holds or held, where the base (the local branch or `origin`'s) already holds every commit.
Answered with `released` (slot numbers), `deleted_branches` and `kept`, a sentence for each thing that stayed and why.
It commits nothing for the agent.
**It is a call on Fleet's door because a command cannot do the same:** once a slot is released the next write in the job takes a new lease, and the git guard refuses `git branch -D` and `kill`.
A terminal job is refused; its slot is parked when its process ends.
The mod's first-message note tells the agent to call it before `all_done`, and to start servers with `start_server` so it can stop them.

**A job marks done a note Bridge sent it using `mark_annotation_done`** (`note_id`, and `session_id` placed as `finish_up`'s is).
A note id is letters, digits and `-`, so it names no path outside an annotations directory.
Fleet looks for `<note_id>.json` under the served root's `.armada/annotations/`, then the preview's `.armada/preview/.armada/annotations/`, then each slot's and each `.claude/worktrees/` checkout's; none found is refused, naming where it looked.
Only a note whose `sent.sessionId` is the calling job is marked: `status` becomes `done` and `updatedAt` Fleet's now, every other field stays in Bridge's order, and the file is written through a rename.
Answered with the `path` written.
**It is a call on Fleet's door because the note may sit in a checkout the job holds no slot for**, such as the preview's, and the slot guard refuses that overwrite; the guard is not widened for it.

**A note sent to a job is kept on it as an artifact of form `note`**, target `note:<id>`.
Fleet keeps one when `send_session_note` or `send_session_notes` carries a `note_id`, and when a plain message names a window-scope note by its picture `annotation-<id>.png` or its line `- Note file: .armada/annotations/<id>.json`, whose file must exist.
Its detail is `note`, `title`, `text`, `tag`, `component`, `screen`, `address`, `file` and `picture`, the last two absolute paths and the picture never inline, with `status` and `sent`.
`status` is `open`, `done` once the note is marked, or `returned` where the job closed unfinished.

**A job's close settles each open note sent to it.**
Where its `all_done` still stands, nothing having been sent and no turn begun since, each becomes `done`.
Otherwise `sent` is removed and the note stays `open`, so Bridge lists it under no job.
Both close paths do this, a hosted close and a terminal job's end, and each rewrite is written through a rename and moves the note's artifact.
That artifact's `session.changed` is Bridge's word that a note file changed: main tells the annotation layer, which reads its notes again.

**An agent that calls `show_window`, `withdraw_window`, `ask_in_window`, `waiting_for` or `all_done` through the door is answered a receipt, not the row.** The row grows with every attachment (36 to 62 KB for a long job, 10 KB at the median, measured 7 to 13 Oct), and an agent keeps each answer in its context for good.
The receipt is `session_id` and what the act set: `window` (with `title` and the `takes` names), `asked` (the question's `window_ask:<row>`), `waiting_for` (the agent's items as Fleet kept them, `id` and `text`), or `done` and `done_since`.
It weighs the same on the first call and the thousandth.
Bridge's routes and the mod's hook, which are not the door, are answered the row.
Nothing reads these four answers today.

## A reply that asks

**A reply raises one Needs you item for each sentence of its close that asks the person.**
Fleet reads the close of the turn's last message (`crates/fleet/src/session_host/extracting.rs`) with its headings left out.
A heading is a `#` line, a line wholly in bold, or a short line alone before the next paragraph that ends in `:` or `?`: it introduces what follows and asks nothing.
A line with a fenced command right after it is kept whatever its shape.

Each remaining sentence is read alone by two heads of the local language model, one for what asks the person and one for what the agent keeps for itself.
The sentence the asks rules find is always the first ask, so nothing the rules caught is lost.
Every ask is the item `replied:<turn ended>:<n>`, with `n` its place among the reply's asks, its text the sentence, and its `since` the instant the reply ended.
`dismiss_waiting` drops one ask by its id and leaves the others.

An ask's act is read from its words:

| Sentence | Act | Options |
|---|---|---|
| A fenced command follows it | `run`, the command as target | none |
| Names approve, merge or land, and a `#N` | `approve_pr`, `#N` | none |
| Anything else | `answer` | the alternatives of an "X, or Y?", or the short list it closes |

A reply that asks nothing raises nothing, and the Job reads `replied` ([What a job is doing](what-a-job-is-doing.md)).
A reply is read even beside an item already listed, and a repeat of that item is folded instead.
The readers are in force (*What is in force*), and each ask a reply makes is its own item with the act and options its words name.

## A question that asks nothing

**Bridge marks a question read from a reply, and he can say it asks nothing.**
Its row in Needs you draws `ScanText`, named `Read from the reply`, and ends in two thumbs; a right press on the row, or the ContextMenu key, offers the same two as **A question** and **Not a question**.
Thumbs down is `dismiss_waiting` with `misread`: the item goes, and Fleet keeps its sentence in the store's `reply_misreads` for good, of no one Session.
Thumbs up is `confirm_waiting`: the item stays, Fleet keeps its sentence in `reply_confirmed` and reads the item `confirmed` from then on, and the row drops its thumbs while its menu still offers **Not a question**.
Fleet refuses both on an item of any other source, since only a read from a reply can be misread.

A sentence a later reply says that is within `FOLDS_AT` of a kept one raises no item, in any Session (`dedup::left_off_as_misread`).
The check uses the vectors the fold below already asks the model for, so a sentence not embedded yet stays this read and goes on the next.
An item the agent stated stands however near it is.
The asks head learns from them only when it is trained again: `tools/lm-train` reads every misread sentence as one that asks nothing and every confirmed one as one that asks ([Spike: reply sentence readers](../../spikes/reply-sentence-readers.md)).

## Items that are one ask

**Fleet folds items that are one ask by two doors into the best of them** (`crates/fleet/src/session_host/dedup.rs`).
The doors are an agent item beside the reply's own sentence, and an agent item restating a walk or the plan that Fleet lists in its own words.
`waiting.rs` already drops an agent item that names the same act and target as Fleet's, and the rules know walks; the rest is worded many ways, so the model reads each pair of texts by cosine.

Items are sorted by precedence and each is dropped where it is within `FOLDS_AT` of one already kept.
Fleet's own item stands over the agent's, and the agent's over the reply's.
Within one of those, an item with an act stands over one without, then the earlier.
A chain of near items folds into its best member, not into each other, and the kept item is kept as it was.
An item the model has not embedded yet stays this read, and the Session is published again once it is.
The fold runs on every read, after the derived items are added, and only once the model is loaded.

## An item he cannot act on

**A clear item holds a verb aimed at him and the thing it names, reads alone, and is not a status report** (`crates/fleet/src/session_host/clarity.rs`).
A fenced command handed over is the verb and the thing.
"Bigger than it looked?" is none of these.

| Why unclear | What it is |
|---|---|
| Pointer | Points at something he would have to find: "see above", "three options" |
| Status | Reports where things stand and asks nothing |
| No ask | Holds no verb aimed at him, or none with a thing it names |

The rules meet an item twice:

- **`waiting_for` refuses the call** naming the first unclear item and why, and asks the agent to word it again as one line that reads alone.
  Nothing of the list is kept.
  Only the rules read here, since the head cannot say why.
- **A reply's unclear ask is worded as the clearest of the agent's sentences near it.** The candidates are its own sentence and the rest of its paragraph or list item, or the paragraph a heading heads.
  The rules rank them first and the `item_clear` head breaks a tie; a clear ask keeps its own words.
  "Bigger than it looked?" followed by "The parser grew from 300 lines to 900. Split it into two files or leave it as one." is raised as the last sentence.
  An ask with no clear candidate is left out, the asks that stay are numbered again among themselves, and a reply whose asks are all unclear raises no item.

The head does not refuse: it cannot say why, and it caught little the rules missed.
Beside the rules it ranks.
It reads a sentence of a close here, not an item, and that was not measured.

## Follow-ups kept for Later

**A sentence of a reply's close in which the job says it will do something later is kept for that job until it is settled.**
The second head of *A reply that asks* finds it, and a sentence the first head calls an ask is not one.
Fleet keeps it per job across replies (`session_later`, `SessionRecord.later`, `crates/fleet/src/session_host/later.rs`).
The same sentence said again is the same item, and a settled one is not kept again when a reply is read again.
Bridge shows the open ones in Delivery, folded under a Deferred label that a press opens, with the issues the Job filed for later.

The job settles one with `settle_later`, and the person has two presses on its row:

| Act | By | Needs |
|---|---|---|
| `settle_later` `done` | The job | Nothing |
| `settle_later` `filed` | The job | The issue, a forge number or a tracker key |
| `settle_later` `dropped` | The job | A reason in a sentence |
| Drop | The person | Nothing: the same call as `dropped`, placed by no connection |
| Remind | The person | Nothing: messages the job to do it, file it or drop it |

**`all_done` is refused while any follow-up is open.**
The refusal is 409 `fleet.later_open`, names each by id and sentence, and says how to settle.
**End job is `close_session`, which never reads them.**
Only the agent's call is held, and the follow-ups of an ended job are not kept.
The wire is in `docs/practices/protocol/a-jobs-kept-follow-ups-and-settling-one.md`.

## What is in force

Every reader here is built, measured and in force: the owner picked all three on 22 Oct 2026.
Each constant is `true` in its module, and a spike holds what it measured.

| Reader | Constant | In force means | Measured in |
|---|---|---|---|
| Sentence readers | `extracting::IN_FORCE` | Each ask of a reply is an item, and follow-ups are kept for Later | `docs/spikes/reply-sentence-readers.md` |
| Fold | `dedup::IN_FORCE` | Items that are one ask are one row, once the model is loaded | `docs/spikes/needs-you-dedup.md` |
| Clarity | `clarity::IN_FORCE` | `waiting_for` refuses an item he could not act on, and a reply's unclear ask is left out | `docs/spikes/item-clarity.md` |

The filter on a reply's asks needs both the sentence readers and the clarity rules in force.
The `waiting_for` refusal needs only the clarity rules.
A running Fleet keeps no follow-up until the model is loaded.
