# Protocol 23.27: a retro item says where it stands

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

The owner, 5 Oct 2026. The Retro sheet in Bridge offered Agree and Disagree on an item already
answered from the Lessons list, because `get_job_retro` carried no answer state.

**Additive.** Two optional fields, with the names and types `Lesson` already has.

| Change | Where | Absent or older |
| --- | --- | --- |
| `state` | `RetroItem` | Only on an item whose row has no answer record, read as before |
| `job_proposed` | `RetroItem` | Absent until agreeing proposed a Job for the item |

Both are read from the rows `list_lessons` reads. A 23.26 Bridge ignores them.
