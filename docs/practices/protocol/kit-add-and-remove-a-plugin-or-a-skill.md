# Kit's Add and Remove: a plugin or a skill

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 21 Oct 2026. `docs/concepts/kit.md`.
**Additive except one field**: four operations, four routes, one error code and the types they carry, and `uninstall` on `SetupItem`; no event kind.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/kit.ts`, written by hand, field for field.

| Where | Side | Carries |
| --- | --- | --- |
| `look_at_kit_source` (`POST /kit/pieces/look`, `agent_access = "No"`) | Bridge sends, Fleet answers 200 | `LookAtKitSource` `{ source }`; answers `KitSourceFound`, tagged by `found`: `plugin` `{ name, marketplace, repository, skills, commands }`, `skill` `{ name, repository, commands }`, or `unknown` `{ why }` |
| `install_kit_piece` (`POST /kit/pieces/install`, `Bridge only`) | Bridge sends, Fleet answers 200 | `InstallKitPiece` `{ source, kind, manifest }`; answers `KitPieceInstalled`, tagged by `state`: `installed` `{ setup }`, or `failed` `{ command, exit_code, error, taken_back }` |
| `remove_kit_piece` (`POST /kit/pieces/remove`, `Bridge only`) | Bridge sends, Fleet answers 200 | `RemoveKitPiece` `{ kind, name }`; answers `KitPieceRemoved` `{ setup, trashed }`, `trashed` `null` for a plugin and always sent |
| `restore_kit_piece` (`POST /kit/pieces/restore`, `Bridge only`) | Bridge sends, Fleet answers 200 | `RestoreKitPiece` `{ trashed }`; answers `KitInventory` |
| `SetupItem.uninstall` | Fleet sends | On an installed plugin's row only: the command Remove runs, `claude plugin uninstall <name>@<marketplace>` |

**Fleet runs only what it found.** Install takes a repository and a kind, looks again and runs that look's commands, so no command line crosses from Bridge.
Remove names a skill by the name the inventory gives it, and Fleet finds its folder in an inventory read for the call.
Restore names a place Fleet answered with, and Fleet keeps the way back in `~/.armada/kit-trash`.

**A failed command is an answer.** `exit_code` is `-1` where the command would not start or ran past its time limit.
`taken_back` names a marketplace this install added and removed again when the plugin then failed; one the person already had is never removed.

**Refusals** are a 422 `fleet.kit_piece_refused`: a source that is no longer a piece of `kind`, `manifest` `true` (Kit's armada.yml has no place for a plugin or a skill), a skill that is not the person's own, a plugin not installed or whose uninstall failed, and a place Fleet did not trash.

**Bridge's call** is not wired yet: main and preload carry none of the four, and `packages/surfaces/manifest/src/fake/kit-pieces.ts` answers them in the mock.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
Were a pair to meet anyway, a Fleet without the routes answers 404, and an older Bridge leaves `uninstall` unread.

**The event stream.** Untouched: none of the four publishes an event, so the open risk below is no better and no worse.
