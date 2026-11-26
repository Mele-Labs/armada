# Waiting on the person

**Part of:** [Job](../session.md).

**A job says what it is waiting on the person for with `waiting_for`**, a tool on the agent's door that replaces the whole list each call (`items`: `id`, `text`, an optional `act` of `answer`, `approve_pr` or `run` with a `target`, optional `options`, optional `steps`, a list of short lines, and an optional `artifact`); an empty list clears it.
Fleet keeps the agent's items in `session_waiting`, stamps `since` and keeps it for an unchanged `id`, and **adds its own on every read** into `SessionRecord.waiting_for`: `ask:<call>` for an open question, `perm:<call>` for an open permission, `walk:<url>` for a window the job showed that no `Approved: <url>` message (of any of its takes) has followed, `window_ask:<row>` for a question asked in the window and not yet answered (*Asking in the window*), `draft:<row>` for a draft proposed with `propose_draft` and not yet settled (*Proposing a draft*), `sketch:<title>` for a sketch shown with `show_sketch` and not yet answered (*Drawing a sketch*), `task_done:<project>` for tasks of a project the job held that it or a pull request has finished ([Project](../project.md)).
**`task_done:<project>` is answered with Mark done or Keep open**, through `answer_waiting`, and the choice settles every task the job asked in that project without sending the agent a message.
Where an agent item names the same act and target as one of Fleet's, Fleet's stands.
**An item about one of the Job's artifacts names it** with `artifact`, the artifact's id or its title.
Fleet resolves it against what the Job holds, refuses one it holds none of (`fleet.artifact_not_held`, naming the ids it does hold), and keeps and serves the id as `WaitingItem.artifact`, on the open item and the closed one, through a restart and a reopen.
Bridge shows the item with the artifact's name and opens that artifact on a press.
The owner ruled on 14 Nov 2026 that a Job presents one clean thing, so a need about an artifact points at it and does not describe it again.
Native's press is the Jobs screens' work (`docs/concepts/artifact.md`).
**`waiting_for` refuses an item with a `walk` act**, and tells the agent to call `show_window`: the window is the one record of a walk, and its item leaves at his Approve.
On 19 Oct 2026 Jobs listed walks in their own words, one with no window behind it and one that stayed after he approved.
Only the act says it (`walk_asks.rs`); Fleet does not read the item's words, and an item kept before the rule is left off every read.
**It refuses an item that is a merge wait** (`fleet.session_waiting_merging`): its `approve_pr` act names one of the Job's pull requests that is queued, has auto-merge on or has merged, or its words name such a pull request's number or link (`merge_waits.rs`).
A merge wait is not a Needs you item, since Fleet follows the merge and tells the Job when it lands; the agent rewords.
An item kept already, or written again as it was, is never taken off for it.
**In a repository with a preview it refuses an item that asks him to try the Job's own change** (`fleet.session_waiting_look`): an `approve_pr` act on one of the Job's pull requests, `approves: true` whose words name one of them or a branch it offered, or words opening with *Try*, *Look at* or *Test* that name one (`look_asks.rs`).
The Ready to try card of `offer_preview` asks him that, and his Looks good closes the `you_approved` gate, so the refusal tells the agent to offer the branch; a decision with options and an item about another Job's work stand.
On 17 Nov 2026 Jobs raised such items above the card that asked the same thing, though the job note said not to; one kept before the rule comes off every read once the Job's branch is on the preview list.
**It refuses an `approves` item whose `artifact` names one of the Job's artifacts that already asks him** (`fleet.session_waiting_artifact_asks`, `approval_items.rs`): the sketch or page has its own Approve.
His Approve of an artifact takes off the `approves` items naming it, as an answer to one would; an item naming no artifact stands.
On 19 Nov 2026 a Job drew a sketch and also asked him to approve it, and the second ask stayed after he approved the first.
**A `run` item meant for another of his Macs names it in `act.on`**, and Bridge offers no Run for it on any other Mac (`run_macs.rs`, `job_view/run_mac.rs`).
On 18 Nov 2026 a Job asked him to run a command on his other Mac, and Run ran it on the one he sat at.
Only the act says it; the item's words are never read for a Mac.
Fleet refuses a name that is neither this Mac nor a paired one (`fleet.session_waiting_mac`), naming the Macs it knows.
**It refuses an item that repeats a question that lapsed** (`fleet.session_waiting_asked`): a question nobody answered inside the hold, or one kept for a walk, stays in his Needs you whole and his answer comes to the agent as a message.
The reply to a timed-out question says so, and tells the agent not to ask it again or list it; one that does anyway is refused when its option labels are the question's or its text is the question's, and an item kept before the rule is left off every read while the question stands (`lapsed_asks.rs`).
**Where he answers by buttons it refuses a question with no options, or more than four** (`fleet.session_waiting_options`), naming the item and telling the agent to ask it with two to four options, its pick first and a cost on each.
A question is an item with no `approves`, no `steps` and no act but `answer`; a run, a pull request, a message and a procedure need no options (`ways_checking.rs`).
**It refuses an item that asks and is not worded as a question** (`fleet.session_waiting_question`), naming the item (the owner, 19 Nov 2026).
An item asks where it has options, an `answer` act, or `approves`, and its text is its title: the question in plain words, one line of at most 200 characters, with no backticks, ending in `?`.
Something that only informs him is not an item: the Job says it in its reply, or shows it with `asks` false.
A reply that asks in words is not sent back to be asked with options: on 13 Nov 40 of the 51 sent were disputed, read off replies that asked nothing, quoted a question, handed takes to another Job or pointed at an item already in Needs you.
Both his answer to a kept question and to a timed-out one start with "Answering your earlier question".
**Two items are never folded to one row**: whether two wordings are one ask is a guess, so each stands (*Items that are one ask*).
An ended job holds nothing.
**Bridge shows the list on the composer**, as a **Needs you** band folded to one glyph an item, named on hover; a press rides it up as the header of a panel and dims the thread.
The panel opens as a list, a row each and no act on the row; a press on a row opens that one to do its act, and Back or Esc returns to the list.
**What he reads rather than answers is on a sheet, and the rest stays on the card.**
A plan, sketch, draft and document are the review items: they are one **Review** row in the panel's list, with a glyph for each, named on hover, with no count, and it opens one sheet over the thread.
A question, command, pull request, message and Fleet's own cards stay on the card, answered where they stand, as above.
A Job with both keeps the question on the card and lists only the review items on the sheet; a Job with no review items has no Review row and never opens the sheet.
A Job with only review items has nothing to fold, so a press anywhere on the band opens the sheet.
**The sheet takes two or more.** One review item stays where it lives: one draft is a row on the card, and one plan, sketch or document is its card in the thread.
The sheet lists the items oldest first, the list on the left and the selected item on the right, and each item has its own answer, with Approve naming the item, such as `Approve Plan v2`.
`⌘↵` approves, `⌥R` sends back with a note, `↑` and `↓` move, and Esc closes the sheet and returns focus to what opened it.
After an answer the next open item comes up, and the last answer closes the sheet.
A walk is never on the sheet. It is a row on the card, whose press brings its tab forward, and a tab of the Job, where its bar holds the only Approve.
`⇧⌘Y` (`needs_you`, `crates/core-model/domain/actions.toml`) opens the sheet when the Job has review items, and otherwise folds the card open or shut.
The owner, 12 Oct 2026: "when the session is not completed, the 'needs you' items in the right panel move to a panel in the thread like this that is locked to the bottom.
Maybe it's collapsed by default?" A question or permission is left out of the band, since its own card shows it.
The band stays when the agent says it is done, above the done callout; the ledger holds no section for them.
The owner, 16 Oct 2026: "I really don't like the waiting for me being in the ledger. its already in the thread.
We dont need it in the ledger." Bridge puts a Job with items under Needs you with a bell-ring mark whose tooltip lists them.
**The Cockpit raises a call for an item only while he has not been in the Job since it began waiting** (*Cockpit*, the call deck): Bridge keeps when he last viewed each Job (`localStorage`, per id), his latest message counts as being in it, and an item whose `since` is older than the later of the two is no call, though it stays in the list.
**`answer_waiting`** (`session_id`, `item_id`, and a `choice` index into the item's `options`, `text`, or `mode` of `best` or `quick`) settles one: a card item goes through `answer_session_ask`, a walk item sends the approval as the person's message, an agent item becomes a message to the job, and once that is delivered the item is dismissed as `dismiss_waiting` does, so it leaves the list without the agent restating it.
A `choice` with `text` says both: the words go with the pick, to the agent as a second line and to an ask card as its note.
`best` tells the agent to decide for itself and think it through, `quick` to just get it done: finish the work and land it, skipping the formalities.
**A card of several questions is one item a question**: `ask:<call>` for the first and `ask:<call>#<n>` for each after it, each titled with its question and carrying its choices, a multi-select one too.
Answering one keeps his pick and takes it off the list, and the call is answered once every question has been, with every answer in the order asked.
**His answer is kept for its thread card**, apart from what the Job is told: the item, its question, how he answered, his words and his pick, on `SessionRecord.answers` ([Artifact](../artifact.md), *Parts*).
His dismissal is kept too, as `dismissed` with his reason, and a permission prompt keeps nothing.
**`answer_items` and `dismiss_items`** answer or dismiss several items with an Approve at once, across Jobs, each as its own answer would: [Artifact](../artifact.md), *Several at once*.
**`dismiss_waiting`** (`session_id`, `item_id`, and an optional `reason`) drops an item for good: the id is kept in `session_waiting_dismissed`, a derived item with it never comes back, save a walk the job shows again, and an agent item with it is removed and left off if the agent states it again.
**An item the agent stated tells it so**: the Job is sent `Owner removed "<item text>"`, with `: <reason>` after it where the person gave a reason that is not blank, so the agent does not take the silence for an answer.
The owner, 14 Oct 2026: "when there is a 'needs me' item on a job, I should be able to delete it, which lets the agent know that I removed it.
Maybe I can give a reason when I do." **A walk, or a sentence Fleet read from the agent's reply, tells it too**: the Job is sent `Owner dismissed "<item text>"`, so it neither shows nor asks again for want of knowing; it comes back only if the agent shows or asks again.
The owner, 28 Oct 2026: "I should be able to dismiss any needs you item, even walks and PRs. It should let the agent know I dismissed them."
An item Fleet derived in any other way (a card, a permission, a window question, a task asked about, the first message, a done) is only dropped, and the agent is told nothing, since it never stated it.
**A closed item is kept whole.** `dismiss_waiting` keeps the item as it stood (text, source, act, options, steps, when it began) and when it was closed, beside the dismissed id in `session_waiting_dismissed`, whatever the source.
The Job's record lists them as `closed`, newest first, at most 50: each `{ item, at, ended? }`.
**`reopen_waiting`** (`session_id`, `item_id`) lets one wait again: an agent's own item goes back at the end of its list as it was stated, and the agent is sent `The owner reopened "<item text>"; it is in your waiting_for list again, so keep it when you next call waiting_for.`, since its next `waiting_for` replaces the whole list.
Any other item tells the agent nothing.
`ended` marks one that cannot be reopened because what it stood for has ended (a permission whose call ended, a walk that was withdrawn, a done the person's message cleared, anything on a Job that has ended); reopening it is refused.
An agent reads the same list with `list_closed_waiting`.
A dismissal a Job's own process makes (a walk it withdrew, a pull request that merged) is not a close and is not listed.
**A draft item is withdrawn, not only dropped**: the Withdraw (Dismiss) on its card is `dismiss_waiting`, and the Job is sent `Withdrawn: <title>.` and `Do not propose it again.`, with the reason on the next line where one was given, and the row settles `withdrawn`.
The job takes back a draft of its own with `withdraw_draft`, and is sent nothing; both are in [Proposing a draft](proposing-a-draft.md).
The message goes before the item is dropped, so a send that fails leaves the item standing to try again.
A nothing-held id is refused as for `answer_waiting`.
**Every walk of a job waits until the person approves it.** One item per page, `walk:<url>`, for each standing window he has not approved since it was last shown: an Approve at or after the newest `window` row showing the page or one of its takes.
Approving a newer walk leaves an older one waiting.
A message, a note and closing the tab settle nothing, and a page shown again waits again.
A window the job is changing raises no item while it is changing.
A page changed after it was shown (`detail.changed` on its artifact, set with `working`) raises none even when it is ready, however often the turn ends between fixes.
It waits again, with its `since` at that instant, only when the job shows it again with `show_window`.
`dismiss_waiting` drops one, and an ended job has none.
**The job takes a walk back with `withdraw_window`** (`url`, `session_id`) when he has moved past it, as when he asked for a bigger change instead of approving.
It writes the dismissal `dismiss_waiting` writes for `walk:<url>`, so the item leaves the list and the window holds no pending plan, and it ends the walk.
Only a window the job showed can be withdrawn.
Showing the page again with `show_window` lifts the dismissal, so a withdrawn walk waits again, and so does one he dismissed himself.
**Every sketch a job shows waits until the person answers it, except one shown with `asks: false`, which waits on no one** ([Drawing a sketch](drawing-a-sketch.md)).
One item per artifact, `sketch:<title>`, source `sketch`, its act `message` on the job, while a sketch or its part is put to him and unanswered.
Its text is the question its one ask puts to him; an artifact kept before every ask was a question reads as its title, with the parts still open, never a count ([Artifact](../artifact.md), *Parts*).
His Approve or Send back on its card settles it, and so does `dismiss_waiting`, which tells the job `Owner dismissed "<title>"`.
`answer_waiting` on it is refused: the card answers it.
Drawing the title again lifts the dismissal, so the redrawn sketch waits again.
**The job takes a sketch back with `withdraw_sketch`** (`title`, `session_id`) when he has moved past it.
It writes the dismissal `dismiss_waiting` writes for `sketch:<title>` and marks the attachment `detail.withdrawn` = `"true"`, so the card shows no Approve or Send back and the item is not raised.
His own Withdraw ends the same way.
A sketch holds no plan, so none is lifted.
A title the job showed no sketch under is refused.
A new drawing of the title does not carry the mark.
A plan's sketch raises none, since it is approved with the plan.
**The sketch a result leads with rides on the done card and never waits on its own.**
`all_done` writes the dismissal `dismiss_waiting` writes for the lead picture where it is a sketch still put to him, with no verdict and no mark, so the done card is the one place he sees it and his End the one press (`result_sketch` in `picture_shown.rs`).
A sketch the pending plan names, and one beside a question of the job's own still open, still wait, and so does every sketch while a gate holds the call or the step the job is on ends on `sketch_approved`.
**A sketch drawn in the turn still running waits only once that turn ends**, so a result sketch `all_done` settles in the same turn never shows and goes.
His answer or dismissal on its card is taken meanwhile, and a plan gate or step that waits on his approval of a sketch raises it at once.
The thread's pinned sketch reads the same way.
A job moved into a slot keeps the same id, so its sketches and their items stay.
**Every revision Fleet keeps of a page a job wrote under `.armada/documents/` waits until the person answers it.** One item per page, `document:<path>`, source `document`, its text the page's title and its act `message` on the job, while the newest revision's `detail.asked` is `true` and `detail.settled` is unset.
It is read off the page Fleet kept, never off the tool call ([Document](../document.md)).
His Approve or Send back (`answer_document`) settles that revision, and so does `dismiss_waiting`, which tells the job `Owner dismissed "<title>"`.
So does a reply of his in the thread that a model call reads as approving the page ([Document](../document.md)).
`answer_waiting` on it is refused.
A new revision asks again and lifts the dismissal; the same bytes written again ask nothing.

**A page that asks nothing raises no item** (the owner, 19 Nov 2026: something that only informs him goes to the thread and his Artifacts).
Fleet still writes `detail.reads` on the job's own revision of such a page, and `mark_artifact_seen` still keeps the revision he last opened, but neither raises nor closes an item.
Until then it waited as `read:<path>`, text `Read: <title>`, until he opened its newest revision.

**The job takes a Document back with `withdraw_document`** (`path`, `session_id`) when he has moved past it.
`path` is the page's path as its item names it, the end of it (`lease.html`) or its title.
It writes the dismissal `dismiss_waiting` writes for `document:<path>`, so the item leaves the list, the card stops asking and the plan held behind the page is no longer held by it.
The file is not deleted, the job is told nothing, and a new revision asks again.
A page the job holds no Document at is refused, as is a name that fits two, and another job's page is not this job's.

**Fleet groups the list once, so every Bridge draws the same band** (`get_needs_you`, `GET /sessions/needs_you?session_id=`).
A Bridge draws it and derives nothing from `waiting_for`: it is not carried on the record, which would give every list a second copy of each item, so a Bridge reads it again when the Job changes.
Every item is one row of exactly one section, and a row says its glyph, what pressing it does (`act`) and how it is put away (`dismiss`).
The sections are in the order the band draws them: Questions, To do, To review, Walks, Commands, PRs, Restart and Done, none drawn empty.
**To do is a thing the agent asked him to do**, such as "Restart the Mac, then check the rows and tell me how many remain": an agent item with no choices and no answer asked, and no walk, command or pull request to press.
Its act is Done, with a note box (`2 remain`), and `done_waiting` closes it as a dismissed item is closed and sends the job one turn, `Done: <text>` and the note under it.
Only an item the agent stated is done: a page is approved, a call dismissed.
`fold` is false when only review pages, a restart key or a done are owed, so there is no list to open.
`review_in_band` is the sheet's rule: more than one page owed, or a Document the only thing owed; a lone plan, sketch or draft stays on the thread's card.
The native Bridge names a lone plan, sketch or draft in the band all the same, since its card can sit far up a long thread.
An open Ready to try card is always in the band, alone or not: `fold` and `review_in_band` are true, and `glyphs` carries its mark, `Ready to try`.
A restart (`scripts/preview --restart`, `scripts/restart --main`) is the band's key in Restart, steps or none, except the preview's Refresh, which Bridge draws from `get_preview_offers` and is no item here.
`dismiss` is `none` for what is answered and never dismissed, `plain`, `skip` for what only drops, `reason` where a reason goes to the agent, and `not_a_question` for a sentence read from a reply.
Where it is not `none`, `dismiss_waiting` takes the item.
The rule per source, held to the code by a test (`crates/fleet/src/session_host/needs_you.rs`):

| Source | Section | Dismiss |
|---|---|---|
| `agent` | by act: to do unless it asks or runs | reason |
| `ask_card` | questions | none: its card answers it |
| `walk` | walks | plain |
| `permission` | questions | none: its card answers it |
| `window_ask` | questions | none: its card answers it |
| `draft` | to_review | reason |
| `handoff` | questions | plain: Stay, and the Job is told nothing |
| `sketch` | to_review | plain |
| `document` | to_review | plain |
| `first_message` | questions | none: Start answers it |
| `replied` | by act: a pull request to open is plain, any other is a sentence | not_a_question |
| `done` | done | plain |
| `plan` | to_review | plain |
| `slot_choice` | questions | none: Fleet refuses it, and no longer raises it |
| `delivery` | questions | none: Fleet refuses it |
| `words_address` | questions | plain |
| `fleet_call` | questions | plain |
| `task_done` | questions | none: Fleet refuses it |
| `watch_not_ours` | questions | plain |
| `held_child` | questions | none: Fleet refuses it |
| `preview_offer` | by act: commands | skip; Fleet no longer raises it |
| `rule_act` | questions | skip |
| `operator_suggestion` | questions | skip |
| `browser_ask` | questions | plain |
| `gate` | questions | none: Fleet refuses it |
| `operator_undone` | by act: a question | plain; the agent is told nothing, and an answer goes to it as `Re: <question>` |
| `job_conflict` | questions | none: Fleet refuses it |
| `job_held` | questions | plain; the ask is settled with no answer and the Job that asked is sent `held? <pull or branch>: He dismissed it with no answer.` |
| `ready_to_try` | to_review | plain; one row for the Job while any card is open, its cards answer it, and a dismissal covers only the cards open then |

**Every end a step waits on the person for raises an item, read off what the job showed and never off its `waiting_for`.**
`rail_ends.rs` names, for each end, who ends it: the job, the merge, or the person through the item of one source, in a match with no fallback, so an end added with no wait does not build.

| The step ends when | The item |
|---|---|
| He approves the plan | `plan:<version>` |
| He approves a walk | `walk:<url>` |
| He approves a sketch | `sketch:<title>` |
| He approves a document's newest revision | `document:<path>` |
| He approves a draft | `draft:<row>` |
| He picks an idea | the open question, `ask:<call>` |
| He replies | `replied:<turn ended>` |
| The pull request merges | none: the merge is nobody's to press |

**A step that ends on his reply waits on him at every turn end**, a reply that only reports included.
Where nothing else waits and the turn is over, the row carries `replied:<turn ended>`, text `<step>: you reply`, until he writes or dismisses it.
A reply that asks raises its own items under the same id, so dismissing either settles both.
A paused or done job raises none.

The mod's first-message note tells the agent to keep the list current, to give each item an act, and never to list a page it showed with `show_window`, which waits by itself, and puts these tools ahead of the person's own rules for reporting actions and questions in a reply.
It also says a decision is asked one way, in the prompt or in `waiting_for` and never both, that a shell command for the owner goes in a block tagged `bash` so Bridge draws its run button, and that a page is shown as a link and never a screenshot.
**An agent item that carries `steps` is a procedure**: its glyph is Steps, and its row reads its `text`, and its press opens the steps beside the thread in place of its `act`.
A restart with steps is the exception: it stays the band's restart key.
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
**It is refused too, 409 `fleet.picture_owed`, where he looks and the Job has shown no picture** since it started: no `show_sketch`, no `show_window` and no image through `show_file`.
`picture_screens` is not checked.

**A question is never refused for want of words before it.** Fleet checks nothing about what the agent wrote since his message; the question check was deleted on 6 Nov 2026.
Fleet sends nothing at a turn's end, and does not read a reply's words for a done or a walk the agent did not call: it takes a job's word only through `all_done` and `show_window`.
Job 7f7f4ae4 did this twice on 24 Oct 2026: asked why commands were recorded, it worked the answer out in its reasoning and went straight to its question, so he saw the question alone and wrote "you never answered me or explained".
Job 20b4e00a did it again on 10 Nov 2026, so the thread now draws the reasoning before such a question open above it (*A terminal job's thread*).

**A job gives back what it is finished with using `finish_up`** (`session_id` only; the hosted job holding the connection wins).
`all_done` does the same first, so a job that is finished calls only `all_done`; `finish_up` is for giving things back early.
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

**An agent that calls `show_window`, `withdraw_window`, `withdraw_sketch`, `withdraw_document`, `ask_in_window`, `waiting_for` or `all_done` through the door is answered a receipt, not the row.** The row grows with every attachment (36 to 62 KB for a long job, 10 KB at the median, measured 7 to 13 Oct), and an agent keeps each answer in its context for good.
The receipt is `session_id` and what the act set: `window` (with `title` and the `takes` names), `asked` (the question's `window_ask:<row>`), `waiting_for` (the agent's items as Fleet kept them, `id` and `text`) with `fleet` beside it (Fleet's own items, `id`, `kind` and `text`, read-only to the agent and never counted as its own, so its count matches Bridge's), or `done` and `done_since`.
It weighs the same on the first call and the thousandth.
Bridge's routes and the mod's hook, which are not the door, are answered the row.
Nothing reads these four answers today.

## Needs you and Ready for you

**A waiting Job is one of two things.**
Needs you means the Job cannot go on without him.
Ready for you means built work waits for him to look at it, and nothing is blocked on his answer.
`owed_kind` in `crates/ipc/src/job_mark.rs` is the one function that says which, with a match on the source that has no fallback, so a new source is not built until it is placed.
The Job's mark, the standing calls and Pocket's push all read it.

An item is Ready for you only when it shows built work.
Every other item is Needs you, and `done` is neither, since it never counted as owed.

**A Job that said it is done is neither either.**
Once it calls `all_done` its mark reads Done in the Done group, with no dock count, banner, sound or push, until he ends it.
The owner, 23 Nov 2026: "Jobs where the agent has said they are done and its ready to end should be in its own group."
Native draws them as their own Done group, last, behind a Done Jobs switch in the list's view menu.

| Source | State |
|---|---|
| `ready_to_try` | Ready for you |
| `walk` | Ready for you |
| `agent` | Needs you, and Ready for you when its act is a walk |
| `document` | Needs you, and Ready for you when it is only to read (`read:<path>`) |
| `replied` | Needs you, and Ready for you when it names a pull request to open |
| `ask_card` | Needs you |
| `permission` | Needs you |
| `window_ask` | Needs you |
| `browser_ask` | Needs you |
| `handoff` | Needs you |
| `words_address` | Needs you |
| `first_message` | Needs you |
| `plan` | Needs you |
| `sketch` | Needs you |
| `draft` | Needs you |
| `fleet_call` | Needs you |
| `slot_choice` | Needs you |
| `delivery` | Needs you |
| `task_done` | Needs you |
| `watch_not_ours` | Needs you |
| `held_child` | Needs you |
| `job_held` | Needs you |
| `job_conflict` | Needs you |
| `gate` | Needs you |
| `rule_act` | Needs you |
| `operator_suggestion` | Needs you |
| `operator_undone` | Needs you |
| `preview_offer` | Needs you: Fleet no longer raises it |
| `done` | Neither |

An agent item with a run, steps, answer, message, `approve_pr` or done act, and a restart, is Needs you.

**The Job's mark carries it.**
`JobState::Ready`, `JobLamp::Ready` and `JobGroup::Ready` mark a Job whose every owed item is Ready for you, once it rests and its question has not lapsed.
A Job with one Needs you item beside any number of Ready for you ones is Waiting, with the Needs lamp and group.
A Job still at work keeps its working state, and a walk it owes marks its lamp and group Ready.
A failing pull request is his to look at before any built work, so it keeps a Job in Needs.

**Only Needs you counts, sounds and pushes.**
A Ready for you item raises no standing call, so the dock count and the banner sound leave it out, and Pocket sends no push for it.
The row still lists it in the Job's band.

## A reply that asks

**A reply raises one Needs you item for each sentence of its close that asks the person.**
Fleet reads the close of the turn's last message (`crates/fleet/src/session_host/extracting.rs`) with its headings left out.
A heading is a `#` line, a line wholly in bold, or a short line alone before the next paragraph that ends in `:` or `?`: it introduces what follows and asks nothing.
A line with a fenced command right after it is kept whatever its shape.

Each remaining sentence is read alone by two heads of the local language model, one for what asks the person and one for what the agent keeps for itself.
The sentence the asks rules find is always the first ask, so nothing the rules caught is lost.
Every ask is the item `replied:<turn ended>:<n>`, with `n` its place among the reply's asks, its text the sentence, and its `since` the instant the reply ended.
None is raised when the agent listed its own `waiting_for` items that turn, or when `features.readRepliesForNeeds` is off.
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

**Fleet does not fold items into one row.** An agent item beside the reply's own sentence, or restating a walk or the plan Fleet lists, stands beside it: reading two wordings as one ask was a guess, and the person sees both.
`waiting.rs` still drops an agent item that names the same act and target as Fleet's, which is a fact.
The model's vectors (`crates/fleet/src/session_host/dedup.rs`) serve only what he marked Not a question, above.

## What he needs first

**Every open ask of a Job carries its place and why it stands there**, so Bridge and Pocket can show him one at a time.
Each `WaitingItem` in `SessionRecord.waiting_for` has `order`, from 1, and `why`, one plain sentence, and the list is served in that order.
**The Job sets the order with `order_waiting`**: the ids of its asks, its own and Fleet's, first to last, each with a `why`.
The asks it leaves out follow in Fleet's order, and an id that is not an open ask refuses the call, naming it (`crates/ipc/operations/order_waiting.toml`).

**Where the Job set no order, Fleet's is by what the ask is**, and the list's own order within each:

| Place | Asks | Why |
|---|---|---|
| First | What the work waits on: a question, a vote, an approve of a sketch, plan or draft, a walk | The work waits on your answer |
| Then | A page to read or approve | A page to read |
| Then | A build to try | Try it once the asks above are answered |
| Last | The Job's done | The work is done |

**A refresh comes before a prototype or a walk that needs it.**
Where the Job has a preview to refresh and also a prototype or a walk, the build to try comes first, with the why *The prototype needs this build*, or *The walk needs this build*.
The owner's own example, 25 Nov 2026: the prototype could not be tried until the build was refreshed.
The words are in one place, `crates/fleet/src/session_host/needs_first.rs`.

## Follow-ups kept for Later

**A follow-up the job puts to `defer_later` is kept for that job until it is settled.** Fleet does not read follow-ups off a reply.
Fleet keeps it per job across replies (`session_later`, `SessionRecord.later`, `crates/fleet/src/session_host/later.rs`).
The same sentence said again is the same item, and a settled one is not kept again.
Bridge shows the open ones in Delivery, folded under a Deferred label that a press opens, with the issues the Job filed for later.

The job settles one with `settle_later`, and the person has two presses on its row:

| Act | By | Needs |
|---|---|---|
| `settle_later` `done` | The job | Nothing |
| `settle_later` `filed` | The job | The issue, a forge number or a tracker key |
| `settle_later` `dropped` | The job | A reason in a sentence |
| `settle_later` `handed` | The job, while the Operator is on | Nothing: the Operator links it to an open issue that covers it, or, with Operator files on, files it in the repository's beads |
| Drop | The person | Nothing: the same call as `dropped`, placed by no connection |
| Remind | The person | Nothing: messages the job to do it, file it or drop it |

**`all_done` is refused while any follow-up is open.**
The refusal is 409 `fleet.later_open`, names each by id and sentence, and says how to settle.
**End job is `close_session`, which never reads them.**
Only the agent's call is held, and the follow-ups of an ended job are not kept.
The wire is in `docs/practices/protocol/a-jobs-kept-follow-ups-and-settling-one.md`.

## What is in force

| Reader | Constant | In force means | Measured in |
|---|---|---|---|
| Sentence reader | `extracting::IN_FORCE` | Each ask of a reply is an item | `docs/spikes/reply-sentence-readers.md` |
