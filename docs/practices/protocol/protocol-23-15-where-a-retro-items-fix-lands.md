# Protocol 23.15: where a retro item's fix lands

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

The owner, 3 Oct 2026: every retro item says where its fix lands, apart from whose way it got in.
`../concepts/retro.md`, *Where the fix lands*, is the concept.

**One optional field, one query parameter, one enum, all additive.** `RetroItem` and `Lesson`
gain `lands_in`, exactly one of `armada`, `kit` or `manifest`. `GET /lessons` takes
`?lands_in=` to narrow to one, and absent is all three; any other value is a 400.

| Field | On | Absent |
| --- | --- | --- |
| `lands_in` | `RetroItem`, `Lesson` | An item kept before 23.15, which no `?lands_in=` matches |

**Never defaulted.** The retro call is told the three places and must name one per item; an item
naming none, or one that is not one of the three, is dropped the way an item citing nothing the
record holds already was. A fix that spans two places is two items. **An old item is not
migrated**: a place chosen after the fact would be a guess, so it reads with the field absent and
Bridge lists it under All alone.

**Store V102** adds the nullable `lands_in` column to `job_retro_items`.
