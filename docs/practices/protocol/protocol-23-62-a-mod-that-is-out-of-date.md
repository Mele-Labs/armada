# Protocol 23.62: a mod that is out of date

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/session.md`, *A mod that is out of date*. **Additive only**: one optional field on two facts and one on the record.

| Where | Carries | Notes |
| --- | --- | --- |
| `report_session` | `SessionFact` `started.mod_version?`, `tuned.mod_version?` | The version of the `armada` mod reporting. Kept on the job's row, and a fact without one leaves it |
| `session.changed`, `list_sessions`, `get_session` | `SessionRecord.mod_out_of_date?` | `true` for a terminal job whose mod is older than its repository's `plugins/armada/.claude-plugin/plugin.json`, or reported none. Absent otherwise |

**An older Bridge ignores the field, and an older mod sends none**, so a job running one is marked as soon as Fleet and the repository carry a version.
