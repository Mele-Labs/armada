# Protocol 23.81: write clashes between agents

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/fleet.md`, *Write clashes*. **Additive only**: one operation, one fact a terminal reports, and one new reach for `leave_note`.

| Where | Carries | Notes |
| --- | --- | --- |
| `list_write_clashes` (`GET /clashes?manifest_id=&holder=`) | `WriteClashes`: `clashes[]` of `WriteClash`: `one`, `other`, `paths[]`, `since` | `one` and `other` are `ClashHolder`: `kind` (`job`, `session` or `main`), `id`, `label`. `other` is `main` for a holder against the base, with the base's name as `id`. `holder` is a Job's or a Session's id and keeps the clashes it is in. In memory in Fleet: a restart answers none until the holders write again. `Bridge only` |
| `SessionFact` `touched` (`POST /sessions/report`) | `paths[]`, relative to the job's `cwd` | Sent by the `armada` mod from 0.3.13 for each write tool call. Paths only; one that climbs out of the directory is dropped. Fleet reads it for a Job standing in a pool slot and ignores it elsewhere |
| `leave_note` (a Drone's tool) | `to` may be a Session's address, `s-` and the first eight characters of its id | Refused where Fleet has found no clash between that Session and the Job, and once per two minutes per Session |

**Skew.** A Fleet before 23.81 answers 404 to the read, so a Bridge that marks rows takes the 404 as no clashes and draws none, without a message. It also rejects a `touched` report as an unknown fact, and the mod drops it and goes on. A mod before 0.3.13 reports no writes, so its Job is never a holder in a clash. Bridge's half is `packages/protocol/src/clashes.ts` and `sessions.ts`, written by hand like the rest. Bridge reads `list_write_clashes` through one preload entry, `getWriteClashes`, which names no Manifest: main asks once per served repository and merges the answers (`apps/desktop/src/main/clashes.ts`).
