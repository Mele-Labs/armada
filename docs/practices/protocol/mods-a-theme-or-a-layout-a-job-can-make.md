# Mods: a theme or a layout a job can make

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/mods.md`, and `docs/concepts/layout-mods.md` for the second kind.
Five operations, one event and two optional fields on the preferences, and a layout mod adds one field to each of three of them.

| Where | Carries | Notes |
| --- | --- | --- |
| `list_mods` (`GET /mods`) | `ModList`: `ModSummary` `name`, `kind?` (`theme`, `layout`, `words`, `fleet` or `tweak`), `version?`, `description?`, `taught_on?`, `enabled`, `valid`, `reason?`, `changed_at?` | A broken mod is a row with `valid` false; a field that could not be read is left out |
| `scaffold_mod` (`POST /mods/scaffold`) | `ScaffoldMod`: `name`, `description?`, `kind?` (`theme`, `layout`, `words`, `fleet` or `tweak`, absent is a theme); answers `ModScaffolded`: `name`, `path` | 422 `fleet.unacceptable_mod` for a name that is not a slug, or a mod that exists. 500 `fleet.mod_not_written` |
| `set_mod_enabled` (`POST /mods/enable`) | `SetModEnabled`: `name`, `enabled`; answers the `ModSummary` | 404 `fleet.no_such_mod`. `agent_access` is `Helm only` |
| `validate_mod` (`GET /mods/validate?name=`) | `ModChecked`: `name`, `valid`, `problems[]`, `css?`, `layout?` | `css` is a theme's or a tweak's checked text and `layout` a layout mod's, each present only when `valid` and never both; a Words or Fleet mod has no text to hand back |
| `promote_mod` (`POST /mods/promote`) | `PromoteMod`: `name`, `manifest_id?`; answers `ModPromoted`: `name`, `branch`, `commit` | 409 `fleet.mod_not_promotable`. `agent_access` is `No` |
| `mods.changed` (event) | `ModList`, whole | Names no Job. A resync does not carry it |
| `get_preferences` | `Preferences.theme?`, `Preferences.layout_choices?`, `Preferences.key_bindings?`, `Preferences.time_format?` | `theme` is left out while `dark`, and absent is `dark`. `layout_choices` is the text of the owner's `layout.json`, left out while he has made none. `key_bindings` is the owner's own keys, left out while he has changed none. `time_format` is the three switches `hour12`, `seconds` and `date`, left out while he has chosen none |
| `save_preferences` | `SavePreference.text?`, read for the names `theme`, `layout_choices`, `key_bindings` and `time_format` | 422 `fleet.unacceptable_theme`, 422 `fleet.unacceptable_layout` for text `layout.json`'s rules refuse, 422 `fleet.unacceptable_key_bindings`, and 422 `fleet.unacceptable_time_format`. An empty `layout_choices`, `key_bindings` or `time_format` takes the choices back |

**The event stream gets no new queue.** `mods.changed` is one message per change, found by a rescan every two seconds, and goes through the shared drop-oldest backlog; a Bridge that missed it reads `list_mods` after the resync.
It does not touch the unbounded-sink risk above.

**Since 23 Oct 2026**, `words`, `fleet` and `tweak` are kinds, and `ModSummary.taught_on` says what Teach was pressed on. Both are additive: a peer that predates them fails to decode a row of a new kind, and the IDs differ, so the lifeboat serves.
