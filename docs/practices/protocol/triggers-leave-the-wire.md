# Triggers leave the wire

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 27 Oct 2026. `.claude/decisions/2026-10-27-a-feature-job-walks-before-it-tests.md`, *On the wired walk*.
**Removal only**: no operation, event kind, route or field is added.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and a pair that disagrees is refused.
`crates/ipc` and `packages/protocol` are edited together.

No Trigger has fired since Jobs left the wire on 17 Oct, and the Workflow editor that alone saved one went with the Workflows page.
Fleet no longer reads `.armada/triggers/` or `machine/triggers/`, and a file left there is inert.
The store keeps the Trigger and added-step tables, unread, for the migration that drops the Job tables.

**Operations removed (4).** Each loses its route, and the two reads lose their MCP tools:

| Operation | Route | Agents |
| --- | --- | --- |
| `list_triggers` | `GET /triggers` | Yes, now gone |
| `get_trigger` | `GET /triggers/definition` | Yes, now gone |
| `save_trigger` | `POST /triggers/save` | Bridge only |
| `remove_trigger` | `POST /triggers/remove` | Bridge only |

**Wire types removed.** `ipc::triggers` and its mirror (`packages/protocol/src/triggers.ts (deleted)`): `TriggerMoment`, `TriggerLevel`, `TriggerScope`, `TriggerRuns`, the skip types, `TriggerSummary`, `OverriddenTrigger`, `LeftOutTrigger`, `TriggerList`, `TriggerDefinition`, `SaveTrigger`, `TriggerSaved`, `RemoveTrigger` and `TriggerRemoved`.

**Errors removed.** The `fleet.trigger_*` codes the save and remove answered with.

The earlier notes on Triggers, 23.58 to 23.73, stay as the history of what the wire carried.
