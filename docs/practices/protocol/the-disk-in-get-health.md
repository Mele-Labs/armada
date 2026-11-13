# The disk in `get_health`: free space, the two floors, the last pass and what the next would drop

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Fleet drops build output when the disk runs short, and holds new Jobs under a lower floor: `.armada/documents/disk-reclaim-policy.html`, `limits.diskReclaimFloorGib` and `limits.diskFloorGib`.
A surface that draws the disk reads it from `get_health`.
**Additive**: one optional field on an existing type and three new types; no operation, no route, no error code, no event.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/health.ts`, written by hand, field for field.

| Where | Side | Carries |
| --- | --- | --- |
| `FleetHealth.disk?` | Fleet writes, Bridge reads | `get_health`: a `DiskHealth` `{ free_bytes, reclaim_floor_bytes, hold_floor_bytes, last_pass?, next_to_drop? }`. Absent from a Fleet older than this, from one serving no repository, and where the volume would not answer |
| `DiskHealth.free_bytes` | Fleet writes, Bridge reads | Free on the volume of the first served repository, read when the answer is made |
| `DiskHealth.reclaim_floor_bytes` | Fleet writes, Bridge reads | `limits.diskReclaimFloorGib` in bytes, as Fleet holds it now. Under it Fleet drops build output. `0` is off |
| `DiskHealth.hold_floor_bytes` | Fleet writes, Bridge reads | `limits.diskFloorGib` in bytes, as admission holds it now. Under it no new Job starts |
| `DiskHealth.last_pass?` | Fleet writes, Bridge reads | A `ReclaimPass` `{ at, freed_bytes }` for the last pass since Fleet started that dropped anything: `at` RFC 3339, `freed_bytes` what the volume gained over the pass. Absent where none has |
| `DiskHealth.next_to_drop?` | Fleet writes, Bridge reads | Build folder paths in the order the next pass would drop them, at most ten. Set by the last look, and absent when that look ended over the reclaim floor, since nothing would be dropped |

**Of the first served repository only.** Every served repository is looked at, and each short one raises its own card in Your turn; the wire carries the first's volume, the one admission reads.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
Were a pair to meet anyway, the field is optional on both sides: an older Bridge ignores it, and a newer one reads an older Fleet's answer as no disk.

**The event stream.** Untouched. The disk is read with `get_health`; the card it raises in Your turn is `operator_page_changed`, which exists.

**No migration and no store change.** The last pass and the folders to drop are held in Fleet's memory and are empty after a restart until the first look, which runs at start.
