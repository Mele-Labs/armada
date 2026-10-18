# Mods: a theme or a layout a job can make

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/mods.md`, and `docs/concepts/layout-mods.md` for the second kind. Five operations, one event and two optional fields on the preferences, and a layout mod adds one field to each of three of them.

| Where | Carries | Notes |
| --- | --- | --- |
| `list_mods` (`GET /mods`) | `ModList`: `ModSummary` `name`, `kind?` (`theme` or `layout`), `version?`, `description?`, `enabled`, `valid`, `reason?`, `changed_at?` | A broken mod is a row with `valid` false; a field that could not be read is left out |
| `scaffold_mod` (`POST /mods/scaffold`) | `ScaffoldMod`: `name`, `description?`, `kind?` (`theme` or `layout`, absent is a theme); answers `ModScaffolded`: `name`, `path` | 422 `fleet.unacceptable_mod` for a name that is not a slug, or a mod that exists. 500 `fleet.mod_not_written` |
| `set_mod_enabled` (`POST /mods/enable`) | `SetModEnabled`: `name`, `enabled`; answers the `ModSummary` | 404 `fleet.no_such_mod`. `agent_access` is `Helm only` |
| `validate_mod` (`GET /mods/validate?name=`) | `ModChecked`: `name`, `valid`, `problems[]`, `css?`, `layout?` | `css` is a theme's checked text and `layout` a layout mod's, each present only when `valid` and never both |
| `promote_mod` (`POST /mods/promote`) | `PromoteMod`: `name`, `manifest_id?`; answers `ModPromoted`: `name`, `branch`, `commit` | 409 `fleet.mod_not_promotable`. `agent_access` is `No` |
| `mods.changed` (event) | `ModList`, whole | Names no Job. A resync does not carry it |
| `get_preferences` | `Preferences.theme?`, `Preferences.layout_choices?`, `Preferences.key_bindings?` | `theme` is left out while `dark`, and absent is `dark`. `layout_choices` is the text of the owner's `layout.json`, left out while he has made none. `key_bindings` is the owner's own keys, left out while he has changed none |
| `save_preferences` | `SavePreference.text?`, read for the names `theme`, `layout_choices` and `key_bindings` | 422 `fleet.unacceptable_theme`, 422 `fleet.unacceptable_layout` for text `layout.json`'s rules refuse, and 422 `fleet.unacceptable_key_bindings`. An empty `layout_choices` or `key_bindings` takes the choices back |

**The event stream gets no new queue.** `mods.changed` is one message per change, found by a rescan every two seconds, and goes through the shared drop-oldest backlog; a Bridge that missed it reads `list_mods` after the resync. It does not touch the unbounded-sink risk above.
