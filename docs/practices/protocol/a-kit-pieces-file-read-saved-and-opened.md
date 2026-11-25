# A Kit piece's file: `get_kit_file`, `save_kit_file`, `open_kit_piece`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 16 Nov 2026. `docs/concepts/kit.md`.
**Additive**: three operations, three routes, the types they carry and ten error codes; no event kind and no change to a type already on the wire.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `classic-final:packages/protocol/src`, and there is no number to take.
Bridge's half is `classic-final:packages/protocol/src/kit.ts`, written by hand, field for field.

| Where | Side | Carries |
| --- | --- | --- |
| `get_kit_file` (`GET /kit/file?kind=&name=`, `agent_access = "No"`) | Bridge sends, Fleet answers 200 | `GetKitFile` `{ kind, name, plugin? }`; answers `KitFile` `{ path, text, at, plugin? }` |
| `save_kit_file` (`POST /kit/file/save`, `Bridge only`) | Bridge sends, Fleet answers 200 | `SaveKitFile` `{ kind, name, plugin?, read, text }`; answers `KitFile`, the file as it now stands |
| `open_kit_piece` (`POST /kit/pieces/open`, `Bridge only`) | Bridge sends, Fleet answers 200 | `OpenKitPiece` `{ kind, name, plugin?, place }`; answers `KitPieceOpened` `{ path }` |

`kind` is `skills`, `sub_agents`, `commands`, `agent_file` or `output_styles`, and `open_kit_piece` also takes `plugins`.
`name` is `SetupItem.name`.
`plugin` is `SetupItem.plugin`, and absent means the person's own or a repository's piece, so a plugin's piece of the same name is told apart.
`place` is `editor` or `finder`.

**Fleet resolves every path.** It reads the inventory again for each call and takes the file from the one row that names the piece, so a caller can reach no file the inventory does not list.
A skill's file is its `SKILL.md`. Finder shows a skill's folder and a plugin's install path.

**A save over a file that moved is refused**, as `save_manifest_file`'s is: `read` is required, whole, and compared against the disk at the moment of the save.
It is a 409 `fleet.kit_file_moved` carrying `on_disk`, absent where the file is gone.

**Refusals** are a 422 each, by code: `fleet.kit_file_not_listed`, `fleet.kit_file_ambiguous` (several rows of the same plugin, or of none, share the name), `fleet.kit_file_none` (a built in output style, Armada's job rules), `fleet.kit_file_from_plugin` (save only), `fleet.kit_open_plugin_in_editor` and `fleet.kit_open_no_editor`.
A 500 is `fleet.kit_file_unreadable`, `fleet.kit_file_unwritable` or `fleet.kit_open_failed`.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
Were a pair to meet anyway, a Fleet without the routes answers 404.

**The event stream.** Untouched: none of the three publishes an event, so the open risk below is no better and no worse.
