# Protocol 23.35: Accept changes the Kit

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

The owner, 5 Oct 2026: Accept on a Kit item in Retros should change the Kit, starting with allowed
commands. `../concepts/kit.md`, *Kit's allowlist*; `../concepts/retro.md`, *Items*.

**Additive only.** Optional fields, one DTO set and one route; a 23.32 Bridge sends none of the new
fields and reads past the ones it is sent.

| Change | Where | Carries | Absent |
| --- | --- | --- | --- |
| `scope` | `AnswerCommand` | `repository` or `kit`, read by an Always allow only. `kit` keeps the command in `~/.armada/allowed-commands` for every Job on this machine | The repository, which is what Always allow has meant since `#836` |
| `change` | `RetroItem`, `Lesson` | `{ "kind": "allow_command", "command": "…" }`, on a Kit item whose command Fleet copied off a refusal in the record | An item with no change, and every item kept before 23.35 |
| `applied` | `RetroItem`, `Lesson` | The change `agree_lesson` applied, on every later read | Nothing applied |
| `change` | `RetroAnswered` | What the retro model may write: `{ "kind", "refusal", "command"? }`. Read leniently, so one that will not read costs the change and not the item | Not asked for by a Fleet before 23.35 |
| `allowlist` row | `KitInventory` | Each command as `name` and where it came from as `source` (`retro item <id>`, `always allow`, `written by hand`), in place of *not read* | `not_read` with why, where the file will not read |
| `remove_kit_allowed_command` | `POST /kit/allowed_commands/remove` | Body `{ "run" }`; answers `KitAllowedCommands`, each row `{ "run", "source", "lesson_id"? }`. Helm only | A 409 `fleet.kit_allowlist_refused` where no line is spelled `run` |

**The Always allow value is a field and not a fourth `CommandAnswer`.** `CommandAnswer` is one Bridge
matches on, so a variant added there is a major bump by the table above; a `scope` on the answer is
read by Fleet alone. `offers` is unchanged, and a Fleet at 23.35 or later takes both scopes whenever
it offers Always allow.

**One migration, V110**, on `job_retro_items`: `change_kind`, `change_command` and `applied`, all
null or false on every item kept before. **No event.** Agreeing is the one place a retro acts and its
answer is the `Lesson`.

**One Fleet fitting, `Host::kit_home`**, which is not on the wire.

**Bridge's half**, mirrored by hand in `packages/protocol`: `RetroChange` and the `change` and `applied` fields on `RetroItem` and `Lesson` (`retro.ts`), `AlwaysAllowScope` and `AnswerCommand.scope` (`commanding.ts`), and `KitAllowedCommand`, `KitAllowedCommands` and `RemoveKitAllowedCommand` (`kit.ts`). Bridge sends `scope: kit` only with a `rule`, and the allowlist rows it draws are the `allowlist` kind of `get_kit_inventory`, read in place of *not read*.
