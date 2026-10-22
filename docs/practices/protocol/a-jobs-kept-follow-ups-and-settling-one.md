# A Job's kept follow-ups, and settling one

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 21 Oct 2026. `docs/concepts/session/waiting-on-the-person.md`, *all_done is refused while a follow-up is open*.
**Additive.** No protocol number moves; the protocol ID changes with the wire it hashes.
Types live in `crates/ipc/src/sessions.rs` and `packages/protocol/src/sessions.ts`.

| Where | Side | Change |
| --- | --- | --- |
| `SessionRecord.later` | Fleet writes, Bridge draws | `SessionLater[]`, oldest first, the open ones: `id` (`later-<n>`), `sentence`, `said_at`. Left out while empty and on an ended Job |
| `settle_later` (`POST /sessions/settle_later`, on the agent's door) | The agent and Bridge call | `SettleLater`: `id`, `how` (`done`, `filed`, `dropped`), `issue?`, `reason?`, `session_id?`. Answers `LaterSettled`: `id`, `sentence`, `open[]`. 422 `fleet.later_not_settleable` for a `filed` with no issue Fleet reads or an agent's `dropped` with no reason; 409 `fleet.later_not_open` |
| `all_done` | The agent calls | Refused 409 `fleet.later_open` while any follow-up is open, naming each |

**Optional members are left out, never sent as `null`.** A Bridge that predates this ignores `later`; a Fleet that predates sends none.
**The agent's call and Bridge's Drop are one operation.** A call placed by a hosted Job's connection must give a reason to drop; Bridge's call places none and gives none. A terminal Job calls with `session_id` and is read as the person's, so its drop is not held to a reason.
Migration `20261021T1500Z-session_later`: one table, read by nothing else.
