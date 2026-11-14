# Artifact

**What it is:** A sketch, a draft or a page a Job made for the person, read as one record with its revisions and where its newest stands.

---

**Kind:** Concept.

**Part of:** [Job](session.md).

Fleet serves a Job's artifacts with `get_job_artifacts` (`crates/ipc/operations/get_job_artifacts.toml`), built in `crates/fleet/src/session_host/artifacts.rs`.
The owner approved the design on 13 Nov 2026.

## Status

| Part | State |
|---|---|
| The record and `get_job_artifacts` | Built in Fleet; native draws it later |
| Drafts kept as artifacts keyed by title, with revisions | Built |
| Archive and restore, by the agent or by him | Built in Fleet; native draws the fold later |
| The For line, the agent's and Fleet's links | Built in Fleet; native draws it later |
| His Edit of a page's or a draft's words, kept as his revision | Built in Fleet; native draws the toggle later |
| A plan's sketches, settled by its one Approve | Built in Fleet; native draws the page later |
| Approve all and Dismiss all across Jobs, `answer_items` and `dismiss_items` | Built in Fleet; native's review panel calls them later |
| A page's actions, `press_page_action` | Built in Fleet; native draws the buttons later |
| A page's votes, `cast_page_vote` | Built in Fleet; native draws the votes later |

## What an artifact is

**Rule.** An artifact is something a Job made for the person to read or answer: a sketch from `show_sketch`, a draft from `propose_draft`, or a page under `.armada/documents/` ([Document](document.md)).
A file, an image, a window or a note shown to him is not one.

**Rule.** An artifact's id is the ledger target it is kept under, and it stays the same across revisions: `sketch:<title>`, the page's path, or `draft:<title>`.

**Rule.** Its form is `sketch`, `draft` or `page`.

**Rule.** Each artifact keeps every revision, numbered from 1, oldest first, as files in Fleet's attachments folder outside every worktree.
A sketch keeps `r<n>.json`, a page `r<n>.html` and a draft `r<n>.md`.
An identical write keeps no new revision.

**Rule.** Every revision is the agent's, except one he wrote with his Edit, which is his (`by` `owner`).

## Where it stands

**Rule.** The state is for the newest revision: `reading`, `asking`, `approved`, `sent_back`, `withdrawn` or `dismissed`.
A page that does not ask reads `reading`; one that asks and whose file is gone reads `withdrawn`.
`dismissed` is his Dismiss, with no verdict.
`withdrawn` is the Job taking it back with `withdraw_sketch`, `withdraw_document` or `withdraw_draft`, or a draft replaced under its title.

**Rule.** `withdraw_sketch` and `withdraw_document` write the dismissal his Dismiss writes, and mark the attachment `withdrawn_by` `agent`.
The mark is what tells the Job's withdrawal from his Dismiss.
A sketch or page withdrawn before the mark existed reads `dismissed`.

**Rule.** An artifact that asks names the Needs you item that answers it: `sketch:<title>`, `document:<path>` or `draft:<row id>`.
A sketch the pending plan names names the plan's item, `plan:<version>`.

## A plan's sketches

The owner ruled this on 13 Nov 2026: one approval for one decision.
A plan carries the sketches drawn for it, and its Approve settles them together, so they do not arrive as separate cards.

**Rule.** `record_plan` names sketches by title in `sketches`, on the plan or on a group, and each must be one the Job drew with `show_sketch`.
A title it holds no sketch for is refused, naming up to twenty it holds.
A group's sketches are drawn beside that group's section of the plan.

**Rule.** While the version that names a sketch is pending, the sketch does not wait on its own.
It has no `sketch:<title>` item, its `item` is `plan:<version>`, and `in_plan` is that version.

**Rule.** His Approve of the plan settles each named sketch still asking `approved`, with no words, and the Job is told `With it: "<title>", "<title>".` on one line.
A sketch already answered, taken back or dismissed is left as it is.
`in_plan` then names the version whose Approve settled it.

**Rule.** A version he sets aside, or one that no longer names a sketch, lets that sketch wait on its own again if it still asks.
A sketch drawn again after its plan was approved asks on its own, as any redraw does.

**Rule.** Only his Approve of the plan settles its sketches.
A version that stands on an earlier approval, or that his Approve of a walk settles, settles none, and each still asking waits on its own.

The single `sketch` a plan draws inline is approved with the plan as before, and never asks on its own.

## Drafts are keyed by title

**Rule.** A draft proposed under an earlier draft's title is the next revision of that draft.
The owner ruled this on 13 Nov 2026, so a draft revises as pages and sketches do.

**Rule.** Where the earlier draft is still open, it settles `withdrawn` and the Job is sent nothing, since it replaced it.
His Needs you list holds one item for the title.

**Rule.** A draft Fleet raises for a workflow change is not an artifact.

## Archive

**Rule.** The agent or he archives a stale artifact with `archive_artifact`: one a newer artifact replaced, or one that no longer bears on the work.
Fleet never archives one itself, since it cannot tell whether one artifact replaces another.
The owner ruled this on 13 Nov 2026.

**Rule.** An archived artifact folds in the Artifacts tab, and `restore_artifact` brings it back.
It says who archived it, `agent` or `owner`, and when.

**Rule.** An archived artifact that waited on him leaves his Needs you list with no verdict, by the dismissal his Dismiss writes.
Its state reads as it stood before, never `dismissed`, and it names no item.

**Rule.** Where he archives one that waited on him, the Job is sent `He archived "<title>". It is off his list with no verdict.`
Where the agent archives it, nothing is sent.

**Rule.** A new revision keeps the archive, so only a restore lifts it.
A sketch drawn again or a page revised asks him again all the same.

**Rule.** A restore lifts the archive and the dismissal the archive wrote, and nothing else.
An item answered meanwhile stays answered, so a restore never asks him again what he settled.
The Job is sent nothing.

**Rule.** The archive is kept in Fleet's store apart from the artifact's attachment, which a new revision rewrites whole.

## His Edit

**Rule.** He edits a page's or a draft's words in place, with Edit on, and Save keeps them as the artifact's next revision, his.
The owner decided this on 13 Nov 2026; native draws the toggle, the faint underline under editable words and the typing in place.
Fleet's half is `save_artifact_edit` (`crates/ipc/operations/save_artifact_edit.toml`), in `crates/fleet/src/session_host/artifact_editing.rs`.

**Rule.** An Edit reaches words only: text, headings, lists and table cells.
A page he saves holds the same elements with the same attributes in the same order as the revision he edited, or it is refused naming the first element that differs.
A sketch or a mock takes notes, not edits.

**Rule.** Only the newest revision takes an Edit, and the refusal names the newest.
An archived artifact, a settled draft and one the Job no longer holds take none.

**Rule.** The Job is sent each Edit as his message before anything is kept, so a send that fails keeps nothing:
`He edited "<title>": revision <n> is his, from revision <n-1>.`, then one line a change, then what to do.
A page's change is a block whose words changed, as `- <section>: "<old>" → "<new>"`, the section the `h2` it comes under or `Top`.
A draft's is a line diff of `-` and `+` lines.
At most 40 changes are listed, then `and <k> more`, and each quote is cut at 300 characters.

**Rule.** A page he edited is written to its file in the worktree, so the agent's next write builds on his words.
The turn's end reads the same bytes as the newest revision and keeps nothing more.

**Rule.** A draft he edited takes his text on its open row, so his Approve sends his text for the Job to apply.

**Rule.** An Edit settles nothing: a page or a draft that asked still asks on his revision, and he may then Approve it.
An Edit that changes no words keeps nothing and sends nothing.

**Rule.** Who wrote each revision is kept in Fleet's store apart from the artifact's attachment, as the archive is.
A Job that takes another's place carries his revisions of its pages.

## The For line

**Rule.** An artifact may say what it is for: one sentence from the agent on what it serves and what his answer changes, shown under the title.
It is optional.
The owner decided this on 13 Nov 2026.

**Rule.** `show_sketch` and `propose_draft` take an optional `for` of at most 200 characters, and optional `links` of `{label, url}`, at most 5.
A `for` over 200 characters, more than 5 links, or a `url` that is not `http://` or `https://` refuses the call and names the limit or the link.
Nothing is kept.

**Rule.** A page says it in its head: `<meta name="armada-for" content="...">` and any number of `<link rel="armada-for" href="..." title="...">`.
A page is never refused, so Fleet keeps the first 200 characters and the first 5 `http(s)` links.
Each link it drops is named to the Job in the notice that names a sketch that will not draw.

**Rule.** When a revision is kept, Fleet adds the links it knows after the agent's own, never one the agent already gave:
the plan task the Job has in progress (`task`), that task's plan group (`plan_step`), and each open pull request the Job holds (`pull_request`, `#<n> <title>`).
A pull request the agent linked by its address is not linked again.

**Rule.** Each revision keeps its own For line, with Fleet's links as they stood when it was kept, so the record reads the same later.
The artifact shows the newest revision's.
A sketch drawn again with no `for` has only Fleet's links.

**Rule.** The For line rides on the artifact, the latest sketch card, the document card and the draft row, so native draws it without a second read.

## Several at once

The owner decided this on 13 Nov 2026.
His review panel answers several items in one press: Approve all, Ask for changes on each, and Dismiss all.
Fleet's half is `answer_items` and `dismiss_items` (`crates/fleet/src/session_host/answering_items.rs`).

**Rule.** Both reach every item with an Approve, across Jobs: a sketch, a page that asks, a plan, a draft, a walk, and an item a Job raised with `approves: true`.
A question, a permission prompt and every other item keep their own answers, and either call refuses them, `fleet.items_not_approvable`.

**Rule.** Each item settles on its own and in order, and one refusal never stops the rest.
The answer names each item with `settled`, or `refused` with a code and a message, and a refused item stands as it was.

**Rule.** Each item settles the way its own answer settles it, with the same message to its Job.
A sketch is answered at the drawing it holds now, and a page at its newest revision.
A plan is approved as `approve_plan` approves it, and goes as `send_plan_changes` sends its marks where it carries notes.
A draft, a walk and an agent's item take their Approve or Ask for changes.

**Rule.** His notes stay on their item and go with that item's verdict, numbered `1.`, `2.`, one a line.
They are the words of an approval, or the note of a send back, and a send back with no note is refused for that item, `fleet.items_send_back_empty`.
A plan's notes are each a note on the plan as a whole.
A walk approved with notes keeps `Approved: <address>` as its first line, with the notes under it.

**Rule.** Dismiss all takes each item off his list with no verdict.
Each Job is sent one message, a line `He passed on "<title>" with no verdict.` for each of its items, in place of each kind's own dismiss text.
A plan's title is `plan v<version>`.

**Rule.** The message goes first, so a send that fails leaves every item of that Job standing.
Then each closes as his own Dismiss closes it: a plan is set aside, a draft settles `withdrawn` with no reason, and a sketch reads `dismissed`.

## Page actions

The owner decided this on 13 Nov 2026.
An action is a button the agent puts on any part of a page, such as each row of a table of things to review, with words like Investigate, Discuss or Approve.
His press calls back to Fleet, which does it in the Job.
Fleet's half is `press_page_action` (`crates/ipc/operations/press_page_action.toml`), in `crates/fleet/src/session_host/page_actions.rs`.

**Rule.** The agent declares one with `<armada-action id="..." act="..." label="..." item="...">`, anywhere on the page, inside a `<tr>` too.
`item` names the row or part it sits on.
`start` also takes `brief`, and `repository` where the Job is not in the page's Job's repository.
`answer` takes `ask`, the ask's waiting id, and `option`, one of its labels.

**Rule.** Fleet reads an action off the kept revision he pressed, never off a call or the file as it is now.
The first action with an id is the one pressed.
Each carries the text of the `<tr>`, else the `<li>`, it sits in, cut to 300 characters.

**Rule.** Fleet keeps the list of acts a page may run, and refuses anything off it: `message`, `start` and `answer`.
Anything that lands, merges, deletes or stops goes to the Job as a `message`, whose label says what the Job will do.

**Rule.** The page is never refused.
When it is kept, each action with no id, an id an earlier action holds, or an act off the list is named to the Job in the notice that names a sketch that will not draw:
`In the page <path>, action "<label>" will not run: <why>.`

**Rule.** A `message` press sends the Job, as his message, `He pressed "<label>" on <item> in "<title>".`, the row's text on the next line, and his words after.

**Rule.** A `start` press starts a Job at once, as his own start from the dispatch sheet does.
It is held on no start card, and the Operator does not rule on it, since the button is his press.
Its brief is the action's `brief`, then `From "<title>", <item>: <row text>`, then his words.
In the page's Job's repository it takes that Job's project.
The page's Job is then sent `He started Job <short id> from "<label>" on <item>.`

**Rule.** An `answer` press answers the Job's ask with the option, his words as the answer's words, exactly as his own pick would.
An ask the Job no longer waits on, or an option not one of its options, is refused, `fleet.page_ask_not_open`.

**Rule.** Every press is kept in Fleet's store, apart from the page's attachment, and served on the page as `presses`, so native turns a pressed row green with the act under its title.
A press that is refused keeps nothing, and an archived page takes no press.

## Page votes

The owner decided this on 13 Nov 2026: every form of vote is available, the agent picks the one that suits each vote, and it hears the result with every note.
A vote puts two or more takes of one thing side by side, such as two sketches of a screen, and asks him to choose between them.
Fleet's half is `cast_page_vote` (`crates/ipc/operations/cast_page_vote.toml`), in `crates/fleet/src/session_host/page_votes.rs`.

| Form | He sees | He gives | Fits |
| --- | --- | --- | --- |
| `pick` | The takes side by side, a radio on each | One take, and a note on any of the rest | One winner |
| `rank` | The takes as a list he reorders | An order of every take, a number and a note on each | An order |
| `score` | One row per take, a check or a cross on each named part | A keep or a drop per part, and a part left bare is no vote | Parts of different takes that may be combined |

**Rule.** The agent declares one with `<armada-vote id="..." form="pick|rank|score" question="...">` holding two or more `<armada-take id="..." title="...">`.
A take holds ordinary page content, such as a sketch or a mock.
A `score` vote also names its parts, `parts="header,list,footer"`, and is the only form that takes them.

**Rule.** Fleet reads a vote off the kept revision he voted on, never off a call or the file as it is now.
The first vote with an id is the one cast, and the first take with an id in it is that take.

**Rule.** The page is never refused.
When it is kept, each vote with no id, an id an earlier vote holds, a form off the three, fewer than two takes, a take with no id or an id an earlier take of it holds, or a `score` with no parts is named to the Job in the notice that names a sketch that will not draw:
`In the page <path>, vote "<question or id>" will not count: <why>.`

**Rule.** A cast whose form is not the vote's is refused, `fleet.page_vote_form`.
A take the vote does not hold, a part it does not name, or a rank that skips or repeats a take is refused, `fleet.page_vote_takes`.

**Rule.** The Job is sent, as his message, `He voted on "<question or id>" in "<title>".`, then the result, every note with it:
for a pick, `Picked: <take>.` then a line per note, `<take>: <words>`;
for a rank, a line per place, `<n>. <take>`, with `: <words>` where there is a note;
for a score, a line per take, `<take>: keeps <parts>; drops <parts>`, leaving out a half with no parts, then a line per note.
The message goes first, so a send that fails keeps nothing.

**Rule.** Every cast is kept in Fleet's store, apart from the page's attachment, and the newest per vote is served on the page as `votes`, so native shows the result he gave.
A recast replaces the earlier result and is sent to the Job again, and an archived page takes no vote.
