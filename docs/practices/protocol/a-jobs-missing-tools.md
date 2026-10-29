# A Job's missing tools

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`SessionRecord.tools_missing`, optional and additive: `{ harness_version: string, missing: ("mod" | "door" | "task_tools")[] }`.
It is on a hosted Job whose agent came up without some of its Armada pieces, as Fleet read the CLI's `init` line, and absent when nothing is missing.
`missing` is never empty. Bridge reads it to say what the Job lacks and which CLI it ran on.

**Held in memory and lost on a Fleet restart**, as the sign-in mark is: the next `init` sets it again, and a surface treats it as how the last process came up, never as a history.
It rides the record Fleet already serves and adds no event, so it reaches a Board with the next `session.changed`.
Fleet restarts the process once for a missing door or mod (*A Job that came up without its tools*), and the field is the mark that stays when the restart does not mend it.

**`mend_session_tools` is the person's restart of it**: `POST /sessions/tools/mend`, body `{ session_id }`, answered with the `SessionRecord` once the restart is under way, `Bridge only`.
It fills Fleet's copy of the mod, restarts the process without the once-only guard and clears nothing itself.
Refused with 422 `fleet.session_tools_unmendable` (a terminal Job), 422 `fleet.session_mod_not_filled` (the script's words), 409 `fleet.session_closed` and 422 `fleet.no_such_session`.
