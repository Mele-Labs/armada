# A write clash's file is read as two patches

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 17 Oct 2026. `docs/concepts/fleet.md`, *Write clashes*. **Additive only**: one operation, one DTO and one route, and no field of an existing type changes. The protocol ID moves with `crates/ipc/src`, the operation file, the route table and `packages/protocol/src`, and there is no number to take.

| Where | Side | Change |
| --- | --- | --- |
| `read_clash_file` (`GET /clashes/file?session_id=&other_kind=&other_id=&path=`, `Bridge only`) | Bridge asks, Fleet answers | `ClashFile`: `ours` and `theirs`, each the unified diff `git diff` prints for that one path from the merge base of the two sides, empty where a side left the file alone. `other_kind` is a `ClashSide`, `session` or `main`; `other_id` is the other Job's id and `main` ignores it |
| `fleet.clash_not_standing` | Fleet refuses | A 409: the pair no longer clashes, or either has left the table |
| `fleet.clash_path_not_in_clash` | Fleet refuses | A 422: `path` is not one of the clash's `paths` |
| `fleet.clash_unreadable` | Fleet refuses | A 500: git would not answer |
| `ClashFile` in `packages/protocol/src/clashes.ts` | Bridge reads | The hand-written mirror. Bridge reads the operation through one preload entry, `readClashFile`, which names no Manifest: Fleet finds the clash by the Job (`apps/desktop/src/main/clashes.ts`) |

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves. Were a pair to meet anyway, a Fleet without the route answers 404, which no refusal here is, and Bridge words it as "This Fleet cannot show the files of a clash." and draws no diff.

**The event stream.** Untouched: a request-response read, no event kind, and nothing published. The open risk below is neither better nor worse.
