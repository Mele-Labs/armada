# A Job walks any page: `steps` on `show_window`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/session.md`, *Showing a window*, and `docs/practices/capture-window.md`, *A walk in the page*.
A Job in any repository passes the mock walks' own steps with the page it shows, and Bridge plays them from outside the page.
The protocol ID moves with the files below, and there is no number to take.

| Where | Side | Change |
| --- | --- | --- |
| `ShowWindow.steps` (`show_window`, `POST /sessions/window`) | Job writes, Fleet checks | `Vec<WalkStep>`, empty and left out when there are none. A step is `say` and exactly one verb of `press`, `look`, `hover`, `later`, `type` (with `into`), `key` (with `on`), `select` (with `in`), `paste` or `drag` (with `by`); a target is `{role, name?, exact?, within?}`. `ipc::walk_steps`, strict: a key it does not know is a 400 from the decode, naming it |
| Fleet's check (`sessioning.rs`, `steps_shown`) | Fleet refuses | 422 `fleet.session_unnamed`, nothing written, naming the first step that is wrong: no or more than one verb, a verb without its own target, a target with no role, a caption missing or too long, nesting too deep, steps that weigh more than the bound written out. The bounds are the constants beside it |
| `SessionRow::Window.steps` (`session.row`) | Fleet writes, Bridge reads | The steps the window carries, kept when the walk ends as the takes are. Bridge opens the tab with them when the row arrives |
| `Attachment.detail.steps` on the `window` artifact | Fleet writes, Bridge reads | The same steps as a JSON string, because `detail` holds strings only; `openSessionWindow` reads them back from the ledger. Anything that is not a list of steps in Fleet's shape reads as none in Bridge |
| The door's receipt for `show_window` | Fleet writes | `steps`, the captions in order, left out where there are none |
| `window_failed` (`POST /sessions/window/failed`) | Bridge writes | Unchanged in shape. A `why` that begins `step` is a step Bridge could not play, and Fleet tells the Job to fix its steps rather than that its page failed to load |
| `ShowWindow.brief` (`show_window`, `POST /sessions/window`) | Job writes, Fleet checks | `Option<String>`, left out when there is none: the Job's line or two on what changed and what to look at. Trimmed; blank is none. At most 400 characters (`MOST_BRIEF` in `sessioning/shown.rs`); longer is 422 `fleet.session_unnamed`, nothing written |
| `SessionRow::Window.brief` (`session.row`) | Fleet writes, Bridge reads | The brief the window carries, kept when the walk ends as the takes are. A row written before it has none and reads as before |
| `Attachment.detail.brief` on the `window` artifact | Fleet writes, Bridge reads | The same text, beside `title` and `variations`; this is where Bridge reads it from the ledger. Absent where the walk has none |
| `WalkTarget`, `WalkStep`, `ShowWindow.steps`, `SessionRow` `window` `steps`, `ShowWindow.brief`, `SessionRow` `window` `brief` | Bridge reads | `packages/protocol/src/sessions.ts` and `hosted-sessions.ts`, written by hand like the rest |

**Additive.** Every new field is optional and left out when empty.
A Fleet before this one ignores `steps`, and a Bridge before it ignores the field on the row.
The IDs differ, so a Bridge and a Fleet that differ are refused regardless.
There is no number to take: `armada need --status` showed nothing standing on these paths, and no migration is needed since the steps ride rows and an artifact's open `detail`.
**The event stream is untouched** beyond the `session.row` a window row already publishes, so the back-pressure risk below is neither better nor worse.
The lifeboat is untouched.
The steps ride the record on the window artifact, which is published whole on every change, so they are bounded the way a sketch's scene is.

**`brief` was added 3 Nov 2026**, when a Job's walk became a tab beside its thread rather than a window.
It is additive and optional, as `steps` is.
The receipt the door gives the Job does not echo it.
The event stream is untouched beyond the `session.row` a window row already publishes, which grows by at most 400 characters.
