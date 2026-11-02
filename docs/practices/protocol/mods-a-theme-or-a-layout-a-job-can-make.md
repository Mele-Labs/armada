# Mods: a theme or a layout a job can make

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/mods.md`, and `docs/concepts/layout-mods.md` for the second kind.
Five operations, one event and two optional fields on the preferences, and a layout mod adds one field to each of three of them.

| Where | Carries | Notes |
| --- | --- | --- |
| `list_mods` (`GET /mods`) | `ModList`: `ModSummary` `name`, `origin?` (`built_in`, left out where local), `kind?` (`theme`, `layout`, `words`, `fleet` or `tweak`), `version?`, `description?`, `taught_on?`, `entry?`, `enabled`, `valid`, `reason?`, `changed_at?`, `pull_request?` (`ModPullRequest` `address`, `number`, `title`, `session_id`) | A broken mod is a row with `valid` false; a field that could not be read is left out. A built-in mod is named `built-in:<folder>`. `pull_request` is left out once the promote's pull request merges or closes |
| `scaffold_mod` (`POST /mods/scaffold`) | `ScaffoldMod`: `name`, `description?`, `kind?` (`theme`, `layout`, `words`, `fleet` or `tweak`, absent is a theme); answers `ModScaffolded`: `name`, `path` | 422 `fleet.unacceptable_mod` for a name that is not a slug, or a mod that exists. 500 `fleet.mod_not_written` |
| `set_mod_enabled` (`POST /mods/enable`) | `SetModEnabled`: `name`, `enabled`; answers the `ModSummary` | 404 `fleet.no_such_mod`. `agent_access` is `Helm only` |
| `validate_mod` (`GET /mods/validate?name=`) | `ModChecked`: `name`, `valid`, `problems[]`, `css?`, `layout?`, `words?`, `fleet?` | `css` is a theme's or a tweak's checked text and `layout` a layout mod's; `words` is a Words mod's `WordsFile` (`name`, `pattern`, `example`, `address[]` of `{repository, template, found}`) and `fleet` a Fleet mod's `FleetFile` (`rule[]` of `{notice, does, where}`). Each is present only when `valid`, and only the one for the mod's kind |
| `promote_mod` (`POST /mods/promote`) | `PromoteMod`: `name`, `manifest_id?`; answers `ModPromoted`: `name`, `session_id`, `title`, `branch`, `commit`, `pull_request?`, `pull_request_not_opened?` | 409 `fleet.mod_not_promotable`, also for a built-in mod and while an earlier promote's pull request is open. `agent_access` is `No` |
| `mods.changed` (event) | `ModList`, whole | Names no Job. A resync does not carry it |
| `get_preferences` | `Preferences.theme?`, `Preferences.layout_choices?`, `Preferences.key_bindings?`, `Preferences.time_format?` | `theme` is left out while `dark`, and absent is `dark`. `layout_choices` is the text of the owner's `layout.json`, left out while he has made none. `key_bindings` is the owner's own keys, left out while he has changed none. `time_format` is the three switches `hour12`, `seconds` and `date`, left out while he has chosen none |
| `save_preferences` | `SavePreference.text?`, read for the names `theme`, `layout_choices`, `key_bindings` and `time_format` | 422 `fleet.unacceptable_theme`, 422 `fleet.unacceptable_layout` for text `layout.json`'s rules refuse, 422 `fleet.unacceptable_key_bindings`, and 422 `fleet.unacceptable_time_format`. An empty `layout_choices`, `key_bindings` or `time_format` takes the choices back |

**The event stream gets no new queue.** `mods.changed` is one message per change, found by a rescan every two seconds, and goes through the shared drop-oldest backlog; a Bridge that missed it reads `list_mods` after the resync.
It does not touch the unbounded-sink risk above.

**Since 23 Oct 2026**, `words`, `fleet` and `tweak` are kinds, and `ModSummary.taught_on` says what Teach was pressed on.
Both are additive: a peer that predates them fails to decode a row of a new kind, and the IDs differ, so the lifeboat serves.

**Since 23 Oct 2026, again**, `ModSummary.entry` says which Logbook entry made a mod, and `ModChecked.words` and `ModChecked.fleet` hand back what a taught mod holds, as `css` and `layout` do.
All three are optional and left out when absent, so an older peer reads the rest; the IDs differ and the lifeboat serves.

**Since 23 Oct 2026, a tracker mod's edit after its try**, `ModSummary.stale` is true where a try is kept of a `tracker.toml` that has changed, and `tried` is absent then.
`TryTrackerMod.manifest_id` is optional, and absent is the first served repository whose Manifest names the mod as its tracker, else the first served, so Bridge's Try names none.
Both are additive and left out when absent; the IDs differ and the lifeboat serves.

**Since 31 Oct 2026, a tab mod**, `ModKind` has `tab`, whose file is `tab.js`.
`ModSummary.glyph` is a tab mod's glyph by registry name, `ModChecked.tab` (`TabModule`: `glyph?` and `code`) is the module as it was checked, `ModAim` has `tab`, and `StartModSession.aim` names the kind a Mod Job is aimed at from its first message.
All three fields are optional and left out when absent, and `tab` is a new spelling of two closed sets, so a peer that predates it fails to decode a row of that kind; the IDs differ and the lifeboat serves.
The event stream gets no new queue, and the unbounded-sink risk above is untouched: a rewritten `tab.js` moves `changed_at` and rides the one `mods.changed` the rescan already publishes.

**Since 2 Nov 2026, an instrument mod**, `ModKind` has `instrument`, whose file is `instrument.js`, held to the limit and the checks `tab.js` is.
`ModSummary.glyph` is an instrument mod's too, `ModChecked.instrument` (`InstrumentModule`: `glyph?` and `code`) is the module as it was checked, and `ModAim` has `instrument`.
A scaffold names `glyph = "gauge"` and writes a starter that calls `host.face` with a paint in `ink` and returns a small panel body, and it passes at once.
A promote commits `mod.toml` and `instrument.js`, and `crates/fleet/build.rs` compiles `instrument.js` into a built-in mod.
Nothing in Fleet's list filters by kind, so an instrument is a row of `list_mods` and `mods.changed` from the moment it is scaffolded.
The field is optional and left out when absent, and `instrument` is a new spelling of two closed sets, so a peer that predates it fails to decode a row of that kind; the IDs differ and the lifeboat serves.
The event stream gets no new queue, and the unbounded-sink risk above is untouched: a rewritten `instrument.js` moves `changed_at` and rides the one `mods.changed` the rescan already publishes.
