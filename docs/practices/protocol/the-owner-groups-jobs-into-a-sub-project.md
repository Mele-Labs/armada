# The owner groups Jobs into a sub project: `start_sub_project`, `list_sub_projects` and `ungroup_sub_project`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 26 Nov 2026. `docs/concepts/feature-catalog.md`, *Sub projects*.
**Additive only**: three operations and their routes, and the types in `crates/ipc/src/sub_projects.rs`; no field on an existing type and no event kind.
There is no number to bump: the protocol ID moves with `crates/ipc/src`, the operation files and the route table.
Only the native Bridge reads it.

| Where | Side | Carries |
| --- | --- | --- |
| `start_sub_project`, `POST /sub_projects` | Bridge sends `StartSubProject`, Fleet answers `SubProject` | `project`, `name` and `jobs`, the session ids in his order. The answer adds `id`, a ulid, `features`, the slugs read at the press, and `started_at` |
| `list_sub_projects`, `GET /sub_projects?project=` | Fleet answers `SubProjects` | `sub_projects`, oldest first; every project's where `project` is left out |
| `ungroup_sub_project`, `POST /sub_projects/ungroup` | Bridge sends `UngroupSubProject` | `id`. Answered 204 with no body |

**`features` is read once, at the press**, from the same `jobs_now` that `list_features` answers.
Fleet adds no Job to a sub project after it.
The Jobs list adds the live Jobs that write to one of `features` itself, from the `features` on each Job's row.

The press is a 422 `fleet.sub_project_no_name` or `fleet.sub_project_no_jobs`, a 404 `fleet.no_such_session` for a Job Fleet does not hold, and a 409 `fleet.sub_project_across_projects` for a Job not filed in `project`.
Ungroup is a 404 `fleet.no_such_sub_project` for an unknown id.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The event stream.** No new event: Bridge reads the list again after a press.
