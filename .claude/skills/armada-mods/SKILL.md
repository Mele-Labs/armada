---
name: armada-mods
description: How a session makes a theme mod for Bridge end to end — scaffold it, edit its stylesheet, check it, and tell the owner where to pick it. Load when the owner asks for a different look, colours, or a theme.
---

# Making a theme mod

**A mod is a folder on this machine, and a local one never touches Bridge's
source.** You make it with Armada's tools, edit one CSS file, check it, and the
owner picks it in Settings. `docs/concepts/mods.md` is what a mod is and what
Fleet refuses.

## The steps

| # | Do | Tool |
|---|---|---|
| 1 | See what exists, so the name is new | `list_mods` |
| 2 | Make the folder; it answers with its `path` | `scaffold_mod` with `name` and a `description` |
| 3 | Edit `theme.css` in that path | Your editor |
| 4 | Check it; read `problems`, fix, check again | `validate_mod` with `name` |
| 5 | Tell the owner | Settings, Theme, then the mod's name |

**Step 4 is not optional, and you are not done until `valid` is true.** Bridge
applies only what `validate_mod` returned, so a stylesheet with a problem does
nothing and shows nothing.

**Say it plainly when you finish:** the mod's name, that it is valid, and where
to pick it. You cannot pick it for him: `set_mod_enabled` and the theme
preference are his.

## What you must not do

| Never | Because |
|---|---|
| Edit anything under `apps/desktop` or `packages/` for a local mod | A mod exists so a look needs no source change |
| Edit `mod.toml` beyond `description` and `version` | `name` must equal the folder, and `kind` is `theme` |
| Add a second file or a link in the folder | Only `mod.toml` and `theme.css` are read |
| Promote a mod unprompted | It puts the mod on a branch of the repository, and that is his ask |

**Promotion is the owner's act.** `promote_mod` is not a tool you have: Bridge
calls it when he asks. If he asks you to do it, copy `mod.toml` and `theme.css`,
and not `.git`, to `packages/mods/<name>/` in your own slot and land it like any
other change.

## What theme.css may say

`:root { --token: value; }` and nothing else, at most 32 KiB. No `url()`, no
`@import`, no `@media`, no selector but `:root` and `[data-theme]`, no property
that is not a design token. `validate_mod` names the line of each problem.

## Token reference

**Change colours and leave sizes alone.** Every token in
`packages/tokens/tokens.json` is accepted, including spacing, row heights and
widths, and moving those breaks layout.

| To change | Set | Notes |
|---|---|---|
| The canvas and its layers | `--bg-base`, `--bg-sunken`, `--bg-raised`, `--bg-overlay`, `--bg-hover` | Darkest to lightest in the shipped theme; keep that order |
| Rules and edges | `--border-subtle`, `--border-default`, `--border-strong` | |
| Text | `--fg-default`, `--fg-muted`, `--fg-subtle`, `--fg-inverse` | `--fg-inverse` sits on `--accent` |
| The interactive colour | `--accent`, `--accent-hover`, `--accent-muted`, `--accent-faint` | Never a status colour |
| Job and step status | `--status-running`, `--status-escalated`, `--status-completed-success`, `--status-completed-failed`, and the other `--status-*` | Each has a `-bg` tint beside it |
| Diffs | `--diff-add-bg`, `--diff-add-fg`, `--diff-del-bg`, `--diff-del-fg` | |
| Type | `--font-sans`, `--font-mono` | Installed fonts only; nothing is loaded |

**The `--surface-*`, `--text-*` and `--edge-*` tokens are aliases** of the ones
above, so setting `--bg-raised` moves `--surface-card` with it. Set the
primitive, not the alias. Keep body text at 4.5 to 1 against the layer it sits on.

## Example: a warmer dark

```css
:root {
  --bg-base: #17130F;
  --bg-sunken: #110E0B;
  --bg-raised: #1F1A15;
  --bg-overlay: #29231C;
  --bg-hover: #322B23;
  --border-subtle: #2E2820;
  --border-default: #40372C;
  --fg-default: #EFE6DA;
  --fg-muted: #B1A292;
  --accent: #E0954A;
  --accent-hover: #EBA763;
}
```

## Example: higher contrast

```css
:root {
  --bg-base: #000000;
  --bg-raised: #0B0B0B;
  --border-default: #6B7684;
  --border-strong: #9AA6B5;
  --fg-default: #FFFFFF;
  --fg-muted: #D0D7E0;
  --accent: #6CB8F0;
}
```
