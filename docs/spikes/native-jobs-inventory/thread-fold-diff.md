# Where Fleet's thread fold and Electron's differ

The native Bridge draws a Job's thread from `get_job_thread`, so Fleet's fold has to give the items Electron Bridge draws.
Electron folds in TypeScript and is the spec: `itemsOf` in `packages/components/src/compositions/SessionThread/SessionThread.tsx`, fed by `rowsOfThread` and `withChanges` (`packages/screens/src/sessions-wire.ts`), `threadRowsOf` (`apps/desktop/src/renderer/src/session-thread-rows.ts`), the host's rows in `apps/desktop/src/renderer/src/sessions.tsx` and `useJudgedRows`.
Fleet's fold is `crates/fleet/src/thread_view.rs`, its wire `crates/ipc/src/job_thread.rs`.
The inventory of what Electron draws is `job-view.md` beside this note.
Read on 9 Nov 2026, against the spacious take, which the shipped app always draws.
Fleet now matches it on branch `fleet/thread-matches-electron`.

## Row by row

| What | Electron | Fleet before | Fleet now |
| --- | --- | --- | --- |
| Run of tool calls | one `calls` item; any other row ends it | same | same |
| `TaskCreate`, `TaskUpdate`, `TaskList`, `TaskGet`, `ToolSearch`, `show_window` calls | dropped before the fold (`UNDRAWN`), so they neither join nor break a run, nor handle a Fleet turn | kept in runs | dropped before the fold |
| Settled `ask` | a `tool` line, `<command>: <word>`, so it joins a run and a Fleet turn's reply | a `row`, which broke the run | joins the run as its `ask` row; no words |
| `window_ask` | a `tool` line, so it joins a run | a `row` | joins the run as its `window_ask` row |
| Waiting `ask`, `browser_reminder` | dropped | dropped | dropped |
| `plan` row | dropped; it only anchors the approved plan | a `row` | dropped; the `plan` item stands where Electron draws the card |
| `delivery` row | dropped | a `row` | dropped |
| `linked` row that `replaces` | dropped | a `row` | dropped |
| Fleet notice | folds with the work after it, unless kept | always folded | folds unless kept |
| Kept notice | disputed (a `disputed` row names it), and the last Fleet turn while the Job asks: `asked` on the record, a plan put to him or set aside, or a pinned sketch | none | same as Electron |
| Last agent message after a Fleet turn | its own item | same | same |
| Call with a subagent after a notice | swallowed into the turn's reply | same | same |
| Call with a subagent elsewhere | `subagent` item, alone | same | same |
| Check runs | one item per run of one name | same | same |
| Sketch cards | replaced, against, answer on the latest card | same | same |
| The newest sketch put to him, unanswered | pinned after the thread; its latest card leaves it, so the runs either side join | a card | `head.sketch`; its latest card leaves the items |
| Sketch revisions | every kept drawing on the latest card, once all have read | none | `revisions` on the latest card and on the pin |
| Document | one card per page, its newest revision | one card per page already; the wire's comment said one per revision | comment corrected; a test holds it |
| `change` | placed before the first row written later | same | same |
| `not_started` | a row before the fold | an item after the fold | an entry before the fold, so a turn before it is not the last |
| `judged` | a row after the last row written by then | none | `judged` item, same place |
| Pending or set-aside plan | the card after the thread | none | `head.plan` |
| Approved plan | after the item of the last row drawn before its `plan` row, **only where that item is a `row`**; else not drawn | none | `plan` item, same rule |

## Not ported, and why

| What | Why |
| --- | --- |
| `batchNotices` | It merges Fleet notices by rules taught in Bridge and read from lessons, matched by a pattern; it is the client's rule, not the thread's. |
| An earlier Job's rows over a divider, a move's carried rows and `moved` rows | They are other Jobs' threads; the native app reads those itself. |
| `withWorkflowCards`, a `started` row's Start | They decorate a row with a lookup or a press, and change no item. |
| `going` on the plan card | A press Bridge sent and Fleet has not answered; Fleet cannot know of it, so the client keeps it. |
| The offer's words | The wire already sends the parts: `head.asked.call.offers` ids, `reaches`, `reaches_to`. `Reject once` and each label are Electron's copy. |
| Mod cards and the Job browser's card (`inline`) | Drawn by the client, placed by time. |
| `names` of an answered ask | Electron reads the first word of its own line; Fleet names the tool asked about, and a window question none. |
| A pinned sketch whose kept scene will not draw | Electron pins an empty scene; Fleet refuses such a sketch, so it pins none. |

## Kept as Electron does it, though it looks wrong

An approved plan is drawn only where the row just before its `plan` row is a plain row.
That row is usually the `record_plan` call inside a run of calls, so Electron mostly draws no approved plan card.
Fleet copies the rule, and a test holds it, so the native thread matches; changing it is a follow-up for both apps together.

## What the record already serves

The thread carries where a card stands and its state; what the card says stays on `SessionRecord`.

| Electron reads | From | Thread needs it? |
| --- | --- | --- |
| The plan's groups, summary, changes, sketch | `record.plan` | No: the record serves it |
| The plan's Judge notes and press | `record.judge_readings` | No |
| Whether the plan is put to him | `record.waiting_for` source `plan` | Only as `head.plan`'s state |
| The ask card's offers and folder | `record.hosted.asked` | No: `head.asked` carries the call |
| A group's own Job card | the child Jobs | No |
| The lapsed deck | rows | Already `head.lapsed`; Electron draws it in Needs you, not the thread |
