# Protocol 16.1: Kit's MCP servers, and what a Drone here is handed

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#1275`. `get_kit_servers`, `add_kit_server`, `forget_kit_server`, `set_kit_server_reach` and
`set_manifest_server_reach` — five routes under `/kit/servers`, the Manifest riding as
`?manifest_id=` the way `get_repository_allowed_commands` already carries it. Additive: no existing
field moved, and an older Bridge reads none of them.

`KitServerRow` carries **both tiers and the answer they come to**. `drones` is Kit's default,
`manifest` is this repository's word — left out where it has none, which is not a third spelling but
Kit's default answering — and `resolves` is `core_model::a_drone_resolves` over the two. It crosses
rather than being computed on the far side because the same call writes the `--mcp-config` document
a Drone is spawned against, and a surface that recomputed it could draw a server as reaching a Drone
that no Drone is handed.

`ServerAddress` is tagged by `transport`, so a reader matches one field rather than guessing which
of two optional keys turned up.

`SetManifestServerReach.reach` is `Option`, **and the key must be present**: `null` is the
take-back, and `SetModel` is the precedent. A Bridge that dropped the field would throw away a
person's word about who may reach a server and be answered 200.

Every one of the five is `agent_access = "No"`. What a Drone may reach is the owner's decision, and
a tool that read the set is a step toward one that changes it.
