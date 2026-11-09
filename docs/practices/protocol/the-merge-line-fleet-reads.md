# The merge line, read by Fleet

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Fleet reads the merge line once, and every client draws it.
Bridge read it in TypeScript from `get_merge_lines` (`mergeLineViews`); the native Bridge and Pocket read this instead.
**Additive only**: one new operation and one new event, and no existing one changes.
Electron Bridge keeps its own TypeScript reading, so `get_merge_lines` and `merge_lines.changed` are as they were.
The Rust half is `crates/ipc/src/merge_line_view.rs` and the reading is `crates/fleet/src/merge_line_view.rs`.
Bridge's half is `packages/protocol/src/merge-line-view.ts`, written by hand.

| Operation | Kind | Carries |
| --- | --- | --- |
| `get_merge_line_view` (`GET /merge_lines/view`) | query | `MergeLineViews`: one `MergeLineView` for each line `get_merge_lines` serves, in its order |
| `merge_line_view.changed` | event | `MergeLineViews`, whole |

## What Fleet reads

| Field | Read from the hub as |
| --- | --- |
| `name` | The repository's label among the served ones: its Manifest's name, else the root's last segment, widened to `parent/folder` and then the whole root only where another served repository reads alike. A root the list does not hold is its own name |
| `pulls` | The forge's merge queue by position, then those waiting for ci to join it, then the rest as the forge lists them |
| `main` | `green`, or `red` with the first failed job as `failed` and the others as `also`; absent otherwise |
| `landed`, `checkout`, `read_at`, `quiet_since` | The hub's, as they are |

**Someone else's pull request is listed only while it is in the queue, and then quietly.**
It keeps its number, title, branch, address and author, and its place; its queue state is `in_queue`, and `mine` stays false so a client can tell.
Every other field is absent and `draft` and `merge_by_hand` are false, so nothing downstream raises a card for it.
A pull request of your own passes through whole.
A pull request waiting for ci to join the queue is not in it, so another author's is left out.

**Main has a mark only when it is green, or red with a failed job.**
A main not read yet, one still running, one the forge ran nothing on, and a red that names no failed job have none.
A failed job is its Manifest Check where one maps and its own name where none does, with `unmapped` set, the first test its log names and its log address.
The Job that merged the red and the Job taking it always carry a `title`: the ledger's, else the branch for the merge, else the Job's id.

## What is left out

The land runner's lists are always empty since `armada land` was retired, so `line`, `off`, `landed` and `sent_back` of a `MergeLine` are not here.
**A pull request's place in the queue is its `queue.position` in `pulls`**, where Bridge worked it out as `placeOf`.
**The person's repository pick stays a client filter**: every served line is here, and the client keeps the lines it draws.
Two departures from the TypeScript: your own pull request passes through whole, where the TypeScript copied the fields a panel drew, and a repository with no hub is still a view, with its name and nothing else.

## The event

Fleet reads the lines every two seconds for `merge_lines.changed`, and reads the view on the same tick.
**It publishes `merge_line_view.changed` only when the view differs from the last one it sent**, and the first read sets the baseline.
A repository's name moving because the served list changed is a change here and none in `merge_lines.changed`.
A hub's `read_at` moves with each forge visit and is in the view, so the two events usually go together.

## Skew and the event stream

A client that missed an event reads `get_merge_line_view` again on the resync after `Missed`, because a reconnecting Bridge must not believe it holds the line it has not heard.
The event is whole, so a missed one costs one read and no gap.
**This makes the unbounded sink slightly worse, and bounds how much.**
It adds one event beside each `merge_lines.changed`, and only on a change, about once a forge visit for a repository whose forge answers.
The sink is still unbounded from the application side, and the recovery is the resync.
A Bridge from before this does not call the operation and ignores the event.
A Fleet from before it refuses `/merge_lines/view` as an unknown route.
