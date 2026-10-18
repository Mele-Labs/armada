# A Job's retro in its thread

**Part of:** [Job](../session.md).

**A retro being written is a `retro` row on a hosted Job's thread** (23.78).
Fleet adds it when it accepts a press, replaces it by id as the writing goes (`stage`, `stage_since`, `characters`, the `items` that have settled) and leaves it when the retro is kept (`retro`, the number `get_session_retro` takes) or has failed (`why`).
`docs/concepts/retro.md`, *A Job's retro*, has the stages.

| Where it shows | What |
|---|---|
| A hosted Job's thread | One `retro` row per retro, in the order the presses came. It is a progress report: it holds no retro, and `has_news` and the retro's own record never read it |
| `get_session` | The row as it last stood. One left going by a Fleet that has restarted is returned and kept as `failed`, "Fleet restarted while it was written" |
| A terminal Job | No row. Its thread is its transcript |
