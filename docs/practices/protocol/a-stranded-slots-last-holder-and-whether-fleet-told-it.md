# A stranded slot's last holder, and whether Fleet told it

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 13 Oct 2026. `docs/concepts/fleet.md`, *Rescuing a stranded slot*. **Additive only**: two optional fields on `SlotStranded`, one new type and one new plain enum. Bridge's half is `packages/protocol/src/holding.ts`, written by hand. There is no number to take: the protocol ID moves with `crates/ipc/src/holding.rs`.

| Where | Carries | Absent |
| --- | --- | --- |
| `SlotStranded.last_holder` | `SlotLastHolder`: `kind` (`session` or `job`), `id`, `title?`, `open` | A holder the lease cannot name: a process outside any Job Fleet hosts, or a record written before Fleet noted it |
| `SlotStranded.told` | An instant: when Fleet told `last_holder` to commit, push and release | Not told: the holder is not an open Job, or Fleet has not yet read the slot since it was stranded |
| `SlotHolderKind` | `session`, `job` | |

**`open` is what decides.** A Session is open while the ledger holds it live, a Job until it ends. Fleet tells only an open Session, and a slot whose last holder is not open, or is absent, is read by a Scout without a press, so `rescue` appears on a slot nobody pressed.

**`SlotHolderKind` is a plain enum Bridge matches on**, as `CommitHome` is, because no registry declares the set; a third kind is an act Bridge has to learn.

**`stranded` is where these ride**, so they are present on a slot a Job's kept release holds as well as a stranded one, and absent on the slot a live process holds with only uncommitted files. Nothing about `rescue_slot` changes: `start` is still accepted from a person, and refused with `fleet.rescue_reading` while Fleet's own Scout reads.

**Skew.** A Bridge that predates this reads no `last_holder` or `told` and draws a stranded bay as before. A Fleet that predates it sends neither, and Bridge treats both as absent.
