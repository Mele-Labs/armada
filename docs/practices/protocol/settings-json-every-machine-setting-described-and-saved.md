# settings.json: every Machine setting, described and saved

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/machine.md`, *settings.json*.
**Additive only**: two operations and one event; `get_limits`, `save_limits`, `get_preferences` and `save_preferences` keep their shape and read and write the same file.

| Where | Carries | Notes |
| --- | --- | --- |
| `get_settings` (`GET /settings`) | `SettingsList`: `path`, `refused?` (`key?`, `reason`), `settings[]` | Each `Setting`: `key`, `group`, `section`, `title`, `description`, `tip` (what the control's tooltip names where `description` is empty, else `null`), `kind` (tagged on `kind`: `integer` `min` `max` `unit`, `seconds` `min` `max`, `boolean`, `choice` `options`, `text`, `prompt`, `text_list`, `json`), `default`, `value`, `saved` (`null` where absent), `applies` (`live` or `at_restart`), `pending_restart`, `overridden_by_env` (`null` where none) |
| `save_settings` (`POST /settings/save`) | `SaveSettings`: `changes`, a key to a value or `null`, which removes it | 422 `fleet.unacceptable_settings` naming the key, and nothing written; 500 `fleet.settings_not_written`. `agent_access` is `Helm only` |
| `settings.changed` (event) | `SettingsList`, whole | After a save, a hand edit, or a hand edit refused. A resync does not carry it |

Bridge's half is `packages/protocol/src/settings.ts`, written by hand like the rest.
