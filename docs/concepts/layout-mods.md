# Layout mods

**What it is:** A mod whose `layout.json` reorders, hides and sets the opening choice of Bridge's Cockpit panels and places, by id, and nothing else, which Fleet checks and Bridge applies.

---

**Kind:** Concept.

You ask a job to hide the merge queue on the Cockpit and the Retros page.
It writes `mods/tidy/layout.json`, and the window re-lays out with no restart.
A month later you promote the mod to a branch, as a theme is.

## The registry

> **Rule.** Every tab, panel and place that can be arranged registers one entry, and a layout can name only entries that are registered.
> Why: a layout is then ids and nothing else, so it cannot reach into a screen's code.

The registry is `LAYOUT` in `packages/shell/src/layout.tsx`. A panel written in code, tier 3, registers through the same entries.

| Field | Holds |
|---|---|
| `id` | A slug, unique in its region |
| `label` | The name drawn |
| `icon` | A glyph from `packages/icons/icons/` |
| `hideable` | False where the owner has to decide on it, or has to reach it to undo a layout |
| `slot` | Cockpit panels only: `top`, above the panel, or `board` |
| order | Position in the region's list, which is the default order |

Every entry is drawn until a layout hides it.

### The regions

| Region | Entries | Never hidden | Orders | Opens on |
|---|---|---|---|---|
| `dashboard.panels` | `fleet`, `merge-line`, `field` | `fleet` | yes, within a slot | |
| `rail` | The places: `overview`, `sessions`, `kit`, `mods`, `settings`, `guides`, `workflows`, `studios`, `lessons`, `reviews` | `overview`, `mods`, `settings` | no | |

**`rail` is the region of places, and it hides them and never orders them.** The key keeps its name and its label is Places.
An entry is a tab or a System strip page: Cockpit, Jobs, Studios, Logbook and Reviews (`reviews`) are tabs, and Kit, Mods, Settings, Guides and Workflows are strip pages.
Hiding one takes that tab or strip page off the bar.
The bar's order is the shell's own, and `⌘1` to `⌘6` follow it.
System is never hidden.
Doctor is a row of the title menu with no entry, and so is every place with no tab and no strip page (Worktree Slots, Merge line, Manifest, Checks), so a layout cannot name them.

**`dashboard.panels` has the merge queue as its one counterpart on the deck.** A hidden `merge-line` hides the deck's merge queue.
`field` is the Field panel of an open Session, and a hidden `field` is not drawn.
`fleet` has no panel on the deck, and `order` has no effect there.
A layout row with no counterpart is ignored, and `dashboard.tabs` went with the old Cockpit (#2301), so a layout that names it is ignored like any region this build does not have.
A layout is machine-wide, with no repository in it.

A hidden entry leaves the strip or the bar and nothing else. It stays in the palette and in links, so an act that jumps to it still lands.

## layout.json

```json
{
  "version": 1,
  "dashboard.panels": { "hidden": ["merge-line"] },
  "rail": { "hidden": ["logbook"] }
}
```

| field | type | required | default | meaning |
|---|---|---|---|---|
| `version` | number | yes | | `1` |
| `<region>` | object | no | none | One of the regions above |
| `order` | list of ids | no | registry order | Named first, in this order |
| `hidden` | list of ids | no | none | Not drawn |
| `first` | id | no | first entry | What opens first |

An entry a list does not name keeps its default place after the named ones.

## Validation

> **Rule.** A file is accepted only when every key, type and id has the shape above, and one wrong shape refuses the whole file.
> Why: the folder is written by anything on the machine, so the accepted language is a whitelist.

Fleet checks the file with `ipc::layout` and Bridge reads the text it returns with `parseLayout`, so the two apply one set of rules written twice.
`packages/shell/layout-registry.json` lists the regions, ids, which can be hidden and which can be ordered; Fleet reads its regions from it, and a test on the Bridge side fails when it and `LAYOUT` disagree.

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

`parseLayout` returns the problems and what it ignored. An invalid layout mod is a row with `valid` false, as for a theme, and stays listed with its reason.

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

The mod row does not turn invalid and the Layout section lists the entries that exist.
The owner's own choice is read the same way, so a dead id in it does nothing, and it is dropped from the saved text the next time he changes that region.

## Safe mode

> **Rule.** With `--no-mods`, or with no layout mod on, Bridge draws the registry defaults.
> Why: a layout can hide navigation, and a window that will not show its own controls must always be recoverable.

The Cockpit, Mods and Settings are never hideable, and Settings → Layout, Reset to defaults, clears the owner's choices and switches every layout mod off.
The mods stay installed.
`?nomods` gives the defaults whatever Fleet holds, and Settings → Layout then changes nothing that is kept.

## Where it sits

| Piece | Home |
|---|---|
| Registry, `parseLayout`, `serializeLayout`, `resolveLayout`, `LayoutSource` | `packages/shell/src/layout.tsx` |
| The regions and ids both sides check | `packages/shell/layout-registry.json` |
| Fleet's check of `layout.json` | `crates/ipc/src/layout.rs` |
| Fleet's list, scaffold, validation and promotion for `kind = "layout"` | `crates/fleet/src/mods/` |
| Cockpit panels and places | Read `useLayout(region)` |
| Settings → Layout, Mods row | `packages/surfaces/settings/src/` |
| The source over Fleet | `apps/desktop/src/renderer/src/fleet-layout.ts` |
| The mock | `apps/desktop/src/renderer/src/mock/layout.ts`, walk `?walk=modsLayout` |

## What is kept

| What | Where | Default |
|---|---|---|
| A layout mod | `~/Library/Application Support/Armada/mods/<name>/`, `mod.toml` with `kind = "layout"` and `layout.json` | |
| A mod's switch | `mod_switches`, as for a theme | On |
| The owner's own choices | `layout_choices` in `get_preferences`, saved by `save_preferences` with `text` holding a `layout.json`, and empty when none | None |

`validate_mod` answers a layout mod with `layout`, the text as checked, where a theme gets `css`.
Bridge applies that text and no other.
`save_preferences` refuses a `layout_choices` that `layout.json`'s rules would refuse, with 422 `fleet.unacceptable_layout`.
`scaffold_mod` takes a `kind`, and a layout scaffold holds `{ "version": 1 }`, which changes nothing.
Promotion copies `mod.toml` and `layout.json` to `packages/mods/<name>/`.

## Promotion and code panels

| Later | Cost |
|---|---|
| Promotion | Copy `mod.toml` and `layout.json` to `packages/mods/<name>/`, as a theme's two files |
| A panel written in code | Registers an entry in the same registry, so `layout.json` already orders it |
| A rework of the Cockpit | Registers each panel it draws as an entry of `dashboard.panels`, and an entry it removes is an id that is gone, which every layout skips |
| A new region | One `LAYOUT` entry, one `useLayout` call, and its entries in `layout-registry.json` |
| A new entry | One `LAYOUT` line and the same in `layout-registry.json` |
