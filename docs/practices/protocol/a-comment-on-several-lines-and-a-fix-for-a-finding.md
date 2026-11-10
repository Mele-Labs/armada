# A comment on several lines, and a fix for a finding

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 10 Nov 2026. `docs/concepts/pull-review.md`, *A review* and *Agent runs*.
**Additive only**: one optional field on a line comment, one optional field on a run's start, one kind of run and one shape of result.
Bridge's half is `packages/protocol/src/pull-reviews.ts` and `review-runs.ts`, written by hand like the rest.
There is no number to take: the protocol ID moves with the wire files.

| Where | Side | Change |
| --- | --- | --- |
| `PullReviewComment::Line.start_line?` | Bridge writes, Fleet posts | The first line of a range on `side`, before `line`. Absent, or equal to `line`, is one line, posted as today. A `start_line` after `line` is a 422 `fleet.pull_request_range_backwards`, refused before the forge is reached |
| `StartReviewRun.kind` `fix` | Bridge writes, Fleet answers | A fix for one finding's lines. `name` is `<path>:<start>-<end> <title>`, which Bridge composes and Fleet keeps as given |
| `StartReviewRun.finding?` | Bridge writes, Fleet checks | The `ReviewFinding` a `fix` fixes, ignored on any other kind. 422 `fleet.review_fix_needs_a_finding` without it, 422 `fleet.review_fix_on_old_lines` for a `left` place, 422 `fleet.review_fix_off_the_change` for a file the pull request does not change or no range from line 1 |
| `ReviewResult::Fix` | Fleet writes, Bridge reads | Tagged `fix`: `at` (the finding's place), `before` (its lines at the run's head), `after` (the agent's lines that replace them) and `why` (one line) |

**A range posts as a range on the forge.** Fleet sends `startLine` and `startSide`, the same side as `side`, only where `start_line` is before `line`, so one line posts exactly as it did.
`ReviewComment::Line` in `adapter-traits` carries `start_line` through to the adapter.

**`at` and `before` are Fleet's, never the agent's.** Fleet reads lines `start` to `end` of the file in the bay the run reads, at the head, before the model is asked, and fills `at` from the finding.
Anything else the agent writes beside `after` and `why` is dropped.
A finding past the end of its file, an empty `after`, or an `after` equal to `before` is a `failed` run with Fleet's reason.
`after` is kept with no trailing newline, as `before` is.

**A fix is keyed like every run**, by repository, pull request, head, kind and name.
The store keeps `kind` as text with no check on it, so `review_runs` takes `fix` with no migration.
A Fleet built before this, moved back onto the same store, reads a kept `fix` row as an `ask` whose result no longer reads, and serves it as a failed run.

**Bridge's mirror matches on `shape` with a default.** `accountOf` in `apps/desktop/src/renderer/src/pr-review/runs.ts` answers nothing for a shape it does not draw, so a `fix` result draws nothing until a surface draws it, and breaks nothing.

**Skew.** The IDs differ, so a Bridge and a Fleet that differ are refused regardless, and the lifeboat serves. Nothing here touches the four `/v0/` routes.

**The event stream: neither better nor worse.** A fix is a run, so it adds the same three `review_run.changed` events, on the one bounded drop-oldest broadcast every event shares.
`before` and `after` ride the last of them, sized by the finding's lines and not by the file.
The unmeasured risk is unchanged: nobody has a number for how many events a real fleet produces against a minimised Bridge.
