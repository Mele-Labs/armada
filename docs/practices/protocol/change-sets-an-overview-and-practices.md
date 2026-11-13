# Change sets, an overview and practices: three run kinds and a catalogue

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 13 Nov 2026. `docs/concepts/pull-review.md`, *Change sets*.
**Additive only**: three values of `ReviewRunKind`, three shapes of `ReviewResult`, new types in `ipc::review_change_sets`, and two error codes; no operation, route or event kind.
Bridge's half is `packages/protocol/src/review-runs.ts`, written by hand, field for field.
The protocol ID moves with `crates/ipc/src` and the operation files, and there is no number to take.

| Where | Side | Change |
| --- | --- | --- |
| `start_review_run` (`POST /review_runs/:repository`) | Bridge writes, Fleet answers | `StartReviewRun.kind` takes `change_sets`, `overview` and `practices`. `name` is the change set's id for `practices`, and is not read for the other two, which are kept as `change-sets` and `overview` |
| `ReviewResult` `change_sets` | Fleet writes, Bridge reads | `sets[]` of `ChangeSet`: `id` (`set-1` on, Fleet's), `title`, `kind` (`migration`, `data`, `logic`, `api`, `ui`, `tests`, `config`, `other`), `purpose`, `area` and `hunks[]` of `HunkRef` (`path`, `old_start`, `old_lines`, `new_start`, `new_lines`) |
| `ReviewResult` `overview` | Fleet writes, Bridge reads | `findings[]` of `OverviewFinding`: `severity` (`blocker`, `risk`, `bug`), `set`, `title`, `reason` and `at`, a `ReviewPlace`. Blockers first |
| `ReviewResult` `practices` | Fleet writes, Bridge reads | `set`, `kind`, `area`, `read_at`, `reused`, `sources[]` of `PracticeSource` (`path`, `blob?`) and `checks[]` of `PracticeCheck` (`item`, `source`, `verdict` `met`, `missed` or `not_applicable`, `note`) |
| The explanation `what-changed` | Fleet carries, Bridge asks | One plain paragraph, asked with `since` as his seen head when a pull request that changed since he looked is opened |

**Refusals.**

| Code | When |
| --- | --- |
| 409 `fleet.review_change_sets_needed` | `overview` or `practices` with no `change_sets` result kept at the same head and range. Nothing is spent |
| 422 `fleet.review_change_set_unknown` | `practices` whose `name` is no kept set's id |

**The practice catalogue** is `practice_catalogue` in the store, migration `20261113T1800Z-practice_catalogue.table.sql`, a new table and nothing altered.
It is keyed by Manifest id, area and kind, and holds the items, their sources with each blob at the base, and when they were read.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
A Bridge that met a Fleet with these kinds anyway draws nothing for a shape it does not know.

**The event stream.** Untouched: each run is three `review_run.changed` events, as any run is.
