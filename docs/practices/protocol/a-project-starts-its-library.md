# A project starts its library: `get_library_start` and `draft_module_features`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 24 Nov 2026. `docs/concepts/feature-catalog.md`, *Starting a library*.
**Additive only**: two operations and routes; no field on an existing type and no event kind.
There is no number to bump: the protocol ID moves with `crates/ipc/src`, the operation files and the route table.
Only the native Bridge reads it.

| Where | Side | Carries |
| --- | --- | --- |
| `get_library_start`, `GET /features/start?project=` | Fleet answers `LibraryStart` | `project`; `modules`, each a `StartModule`: `name`, `slug`, `path` and `held`, present where the library holds the slug; `docs`, repository-relative, a folder with a trailing `/` |
| `draft_module_features`, `POST /features/start/draft` | Bridge sends `DraftModules`, Fleet answers `ModulesDrafted` | `project`. The answer: `drafted`, the slugs made, and `held`, the slugs passed over |

**Fleet reads the modules again for the press**, so Bridge sends only the project, and what it drafts is what Fleet read, never what Bridge drew.
The press is a 409 `fleet.library_started` where the project holds a confirmed or seeded feature, and a 422 `fleet.feature_project`, `fleet.start_no_repository` or `fleet.no_module`; the read refuses the first two of those 422s.

**The press is the person's**, as `answer_retire` is: the Operator through the door, Bridge on the port, and a 403 `api.the_persons_own` from inside a Job.
The read is Bridge's alone.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The event stream.** No new event: Bridge reads the library again after a press, and the drafted rows are in it.
