# Layout mods

**What it is:** A mod whose `layout.json` reorders, hides and sets the opening choice of Bridge's Dashboard tabs and panels, Job tabs and rail rows, by id, and nothing else.

---

**Kind:** Concept.

You ask a session to put the merge line above the fleet on the Dashboard. It writes `mods/tidy/layout.json`, and the Dashboard re-lays out with no restart. A month later you promote the mod to a branch, as a theme is.

## The registry

> **Rule.** Every tab, panel and rail row that can be arranged registers one entry, and a layout can name only entries that are registered.
> Why: a layout is then ids and nothing else, so it cannot reach into a screen's code.

The registry is `LAYOUT` in `packages/shell/src/layout.tsx`. A panel written in code, tier 3, registers through the same entries.

| Field | Holds |
|---|---|
| `id` | A slug, unique in its region |
| `label` | The name drawn |
| `icon` | A glyph from `packages/icons/icons/` |
| `hideable` | False where the owner has to decide on it, or has to reach it to undo a layout |
| `slot` | Dashboard panels only: `top`, above the tab strip, or `board` |
| order | Position in the region's list, which is the default order |

Every entry is drawn until a layout hides it.

### The regions

| Region | Entries | Never hidden | Orders | Opens on |
|---|---|---|---|---|
| `dashboard.tabs` | `command-central`, `running`, `done` | `command-central` | yes | `first` |
| `dashboard.panels` | `quick-dispatch`, `fleet`, `merge-line` | `fleet` | yes, within a slot | |
| `job.tabs` | `overview`, `workflow`, `plan`, `record`, `checks`, `drones`, `pulse`, `settings` | `overview`, `plan` | yes | `first` |
| `rail` | The rows of Work and Machine | `overview`, `mods`, `settings` | no | |

A hidden entry leaves the strip or the rail and nothing else. It stays in the palette, in links, and in the strip while it is the one open, so an act that jumps to it still lands.

## layout.json

```json
{
  "version": 1,
  "dashboard.panels": { "order": ["merge-line", "fleet"] },
  "rail": { "hidden": ["lessons"] },
  "job.tabs": { "order": ["overview", "workflow", "record", "plan"], "hidden": ["pulse"], "first": "plan" }
}
```

| field | type | required | default | meaning |
|---|---|---|---|---|
| `version` | number | yes | | `1` |
| `<region>` | object | no | none | One of the four regions |
| `order` | list of ids | no | registry order | Named first, in this order |
| `hidden` | list of ids | no | none | Not drawn |
| `first` | id | no | first entry | What opens first |

An entry a list does not name keeps its default place after the named ones.

## Validation

> **Rule.** A file is accepted only when every key, type and id has the shape above, and one wrong shape refuses the whole file.
> Why: the folder is written by anything on the machine, so the accepted language is a whitelist.

| Input | Result |
|---|---|
| Over 4096 bytes, not JSON, not an object | Invalid |
| `version` other than `1` | Invalid |
| A field other than `order`, `hidden`, `first` | Invalid |
| A list over 32 items, or an item that is not a slug of 40 characters or fewer | Invalid |
| A region this build does not have | Ignored |
| An id this build does not have | Ignored |
| `order` on the rail, `first` on panels | Ignored |
| A `hidden` id that is not hideable | Ignored when resolved |

`parseLayout` returns the problems and what it ignored. An invalid layout mod is a row with `valid` false, as for a theme.

## Precedence

> **Rule.** The shipped layout is the base, each enabled layout mod applies over it in name order, and the owner's own choice applies last.
> Why: choosing in Settings is the act that changes his screen, and no mod outranks it.

```
registry defaults
      |  each layout mod that is on and valid, by name
      v
 order / hidden / first  ──  a field a layer sets replaces the field below it
      |
      v  the owner's own choice (Settings → Layout), same shape
 what is drawn
```

| Case | Result |
|---|---|
| Two mods set `order` | The later mod's list |
| A mod hides what the owner showed | Shown |
| `first` names a hidden entry | The first entry that is shown |
| `hidden` names an entry that is not hideable | Drawn |

## When an id goes

> **Rule.** An id that is not registered is skipped everywhere it is named, and never fails a layout.
> Why: a panel is removed or renamed in a release, and a mod written before it must keep working.

The mod row does not turn invalid and the Layout section lists the entries that exist. The owner's own choice keeps the dead id until he resets, and it does nothing.

## Safe mode

> **Rule.** With `--no-mods`, or with no layout mod on, Bridge draws the registry defaults.
> Why: a layout can hide navigation, and a window that will not show its own controls must always be recoverable.

Mods and Settings are never hideable, and Settings → Layout, Reset to defaults, clears the owner's choices and switches every layout mod off. The mods stay installed.

## Where it sits

| Piece | Home |
|---|---|
| Registry, `parseLayout`, `resolveLayout`, `LayoutSource` | `packages/shell/src/layout.tsx` |
| Dashboard tabs, Job tabs, rail | Read `useLayout(region)` |
| Settings → Layout, Mods row | `packages/surfaces/settings/src/` |
| The mock | `apps/desktop/src/renderer/src/mock/layout.ts`, walk `?walk=modsLayout` |

Fleet's list, validation and promotion for a `layout` kind, and `kind = "layout"` in `mod.toml`, are not built; the mock holds the mods in memory.

## Promotion and code panels

| Later | Cost |
|---|---|
| Promotion | Copy `mod.toml` and `layout.json` to `packages/mods/<name>/`, as a theme's two files |
| A panel written in code | Registers an entry in the same registry, so `layout.json` already orders it |
| A new region | One `LAYOUT` entry and one `useLayout` call |

## Open questions

- **[layout-reset-scope]** Does Reset to defaults switch layout mods off, or clear only the owner's own choices? The mock switches them off, so the screen is the shipped one in one press. What decides it: whether an owner who resets expects his mods to survive it.
- **[layout-rail-region]** Is the rail a region in tier 2? The owner's example hid the Lessons tab, which is the Retros row of the rail, since the Dashboard has no such tab. What decides it: whether he means the rail, or a tab Bridge does not have.
- **[layout-per-repository]** Is a layout machine-wide or per repository? Mods are machine-local today. What decides it: whether he arranges Bridge differently for different repositories.
- **[layout-job-kinds]** Does a Job's tab layout apply to every kind of Job? A Job at its approval gate draws the proposal where Overview is, and an epic Job draws a wave. What decides it: whether the same order serves those.
- **[layout-dashboard-reflow]** Which Dashboard entries survive the tile rework on `dashboard/cockpit` and `dashboard/dispatch-rows`? The registry names `fleet` and `merge-line`, and both branches keep the merge line and replace the lanes with tiles. What decides it: what those branches land.
