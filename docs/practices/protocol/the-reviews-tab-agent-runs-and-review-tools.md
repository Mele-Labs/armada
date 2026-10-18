# The Reviews tab: agent runs and review tools

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 16 Oct 2026. `docs/concepts/pull-review.md`, *Agent runs*. **Additive only**: five operations, new DTOs in `ipc::review_runs`, one event, one optional field on a row, and a setting. Bridge's half is `packages/protocol/src/review-runs.ts` and `events.ts`, written by hand like the rest. There is no number to take: the protocol ID moves with the wire files, and `armada need --status` showed nothing standing on these paths.

| Where | Side | Change |
| --- | --- | --- |
| `start_review_run` (`POST /review_runs/:repository`) | Bridge writes, Fleet answers | `StartReviewRun` (`number`, `kind` `tool`, `explanation` or `ask`, `name`, `again`) answers `ReviewRun`. **Answers at once**: a kept result as it stands, the run already going, or a new one `queued`. 422 `fleet.review_unknown_tool`, 422 `fleet.review_blank`, 500 `fleet.review_pull_request_unreadable`. `Bridge only` |
| `get_review_runs` (`GET /review_runs/:repository/:number`) | Bridge reads | `ReviewRuns`: every run kept or going for the pull request, one for each kind, name and head it ran on, newest first. `Bridge only` |
| `list_review_tools` (`GET /review_tools/:repository`) | Bridge reads | `ReviewTools`: `tools[]` of `ReviewTool` (`id`, `kind`, `name`, `prompt`, `source`, `replaces?`) and `left_out[]` (`file`, `source`, `said`), left out when empty. `Bridge only` |
| `save_review_tool` (`POST /review_tools/:repository`) | Bridge writes, Fleet checks | `SaveReviewTool` (`scope` `kit` or `repository`, `kind`, `name`, `prompt`, `overwrite`) answers `ReviewTools` read again. 422 `fleet.review_tool_blank`, `fleet.review_tool_not_a_name`, `fleet.review_tool_exists`; 500 `fleet.review_tool_unwritable`. `Helm only` for an agent |
| `review_run.changed` | Fleet publishes, Bridge reads | `{ kind, run }`: the run whole, **nested under `run`** because the run has a `kind` of its own that a flattened event would overwrite. Not replayed |
| `ReviewRun` | Fleet writes, Bridge reads | `manifest_id`, `number`, `kind`, `name`, `head` (the whole commit it ran on), `state` (`queued`, `running`, `done`, `failed`), `started_at`, and when set `finished_at`, `failed` (Fleet's reason), `result` and `cost_micros` |
| `ReviewResult` | Fleet writes, Bridge reads | Tagged by `shape`: `themes`, `impact`, `steps`, `reason`, `text` or `findings`. A place is `ReviewPlace` (`path`, `side`, `start`, `end`) and a step's source is `description` or `doc` with a `path` |
| `PullReviewRow.holder?` | Fleet writes, Bridge reads | `PullReviewHolder` (`id`, `title`): the Session of the person's that holds the pull request, from the pull request owners. Left out when none does, and never a Job |

**An answer is validated before it is kept**, and one that is not valid is a `failed` run with `failed` set and no `result`. Fleet checks that every place names a file the pull request changes, that a range starts at line 1 or later and does not end before it starts, that a step's doc exists in the checkout at the head, and that a link is http or https.

**A kept run is keyed by repository, pull request, head, kind and name** (`review_runs` in the store, migration `20261016T1700Z-review_runs.kept.sql`, a new table and nothing altered). A reader marks a run out of date when it holds a head newer than the run's `head`. `name` is a definition's id, or for `ask` the question's words lower cased and single spaced.

**One setting.** `models.review` (Model section, `live`, no environment variable) ships as the Job model.

**Skew.** The IDs differ, so a Bridge and a Fleet that differ are refused regardless, and the lifeboat serves. Nothing here touches the four `/v0/` routes.

**The event stream: neither better nor worse.** A run adds three small events, `queued`, `running` and `done` or `failed`, on the one bounded drop-oldest broadcast every event shares. No text is streamed and no queue is added, so a run costs three events however long its answer. The message `review_run.changed` carries is not replayed, which is why a Bridge that missed some, or reconnected, reads `get_review_runs` again when it opens a pull request and on Refresh, and never believes the events alone are complete. The unmeasured risk above is unchanged: nobody has a number for how many events a real fleet produces against a minimised Bridge.
