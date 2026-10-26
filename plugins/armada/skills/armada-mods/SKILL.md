---
name: armada-mods
description: How a job makes a mod end to end — a theme (colours), a layout (which tabs, panels and places are drawn, and in what order), or a tracker (an issue tracker Armada was not built for). Scaffold it, edit its one file, check it, and tell the owner where to see it. Load when the owner asks for a different look, colours, a theme, to move, hide or reorder something in Bridge, or to use an issue tracker Armada does not support.
---

# Making a mod

**Three kinds.** A theme changes colours, in `theme.css`; a layout changes what
Bridge arranges, in `layout.json`; a tracker teaches Fleet an issue tracker, in
`tracker.toml`. Colours are the first section below, layouts the second and
trackers the last. Pick the kind from the ask: "warmer", "darker" and "higher
contrast" are themes; "move", "hide", "put first" and "reorder" are layouts.

# In a Mod Job

**A Mod Job's first message is the owner's note on what he pointed at.**
`docs/concepts/mods.md`, *A Mod Job*, is what one is.

| Do | Because |
|---|---|
| Reach for a mod first | A mod needs no source change and applies live once valid |
| Pass your `session_id` to `scaffold_mod` | The mod lists you as `made_by`, and *Mods made* shows it with its switch, which is his undo |
| Where a mod cannot say it, tell him so | He should not wait on a mod that cannot do the ask |
| On Armada's repository, change the source on the Job's branch, then put `scripts/preview --restart` in `waiting_for` as a `run` item | The change reaches him through the preview |
| In the mods folder, with no repository, say a mod cannot do it and stop | There is no source to change and no preview to restart |

# Making a theme mod

**A mod is a folder on this machine, and a local one never touches Bridge's
source.** You make it with Armada's tools, edit one CSS file, check it, and the
owner picks it in Settings. `docs/concepts/mods.md` is what a mod is and what
Fleet refuses.

## The steps

| # | Do | Tool |
|---|---|---|
| 1 | See what exists, so the name is new | `list_mods` |
| 2 | Make the folder; it answers with its `path` | `scaffold_mod` with `name`, a `description` and your `session_id` |
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
| Edit anything under `apps/desktop` or `packages/` for a local mod | A mod exists so a look needs no source change. In a Mod Job on Armada's repository, the source is for what a mod cannot say |
| Edit `mod.toml` beyond `description` and `version` | `name` must equal the folder, and `kind` is `theme` |
| Add a second file or a link in the folder | Only `mod.toml` and `theme.css` are read |
| Promote a mod unprompted | It starts a Job that opens a pull request in the repository, and that is his ask |

**Promotion is the owner's act.** `promote_mod` is not a tool you have: Bridge
calls it when he asks, and Fleet starts a Job of its own that writes the mod to
`packages/mods/<name>/`, opens the pull request and sees it merged. Do not copy
the files yourself. A mod that is merged there ships built in as
`built-in:<folder>`, and cannot be promoted again.

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

# Making a layout mod

**A layout names panels and places by id and nothing else.** It can hide the
Cockpit's panels and places, and reorder and hide an open Job's panels. It
cannot add a panel, change a label or reach any code. `docs/concepts/layout-mods.md` is what Fleet
accepts and how layers combine.

## The steps

| # | Do | Tool |
|---|---|---|
| 1 | See what exists, so the name is new | `list_mods` |
| 2 | Make the folder; it answers with its `path` | `scaffold_mod` with `name`, `description`, your `session_id` and `kind` set to `layout` |
| 3 | Edit `layout.json` in that path | Your editor |
| 4 | Check it; read `problems`, fix, check again | `validate_mod` with `name` |
| 5 | Tell the owner where to see it | Mods lists it; Settings, Layout shows what it did |

**The scaffold holds `{ "version": 1 }`, which changes nothing**, so it is valid
before you edit it. **You are not done until `valid` is true**, and Bridge
applies only the `layout` text `validate_mod` returned. You cannot switch the mod
on or off: `set_mod_enabled` is his. A mod nobody switched is on, so it applies
as soon as it is valid, with no restart.

## The regions and ids

An id that is not in this table is ignored, never an error, so a typo does
nothing: check the id against the table.

| Region | Ids, in the order shipped | Cannot be hidden | `order` | `first` |
|---|---|---|---|---|
| `dashboard.panels` | `list`, `merge-line`, `activity`, `bays`, `controls`, `dispatch`, `autopilot` | `list` | no | no |
| `job.panels` | `workflow`, `blueprint`, `field`, `landing` | `blueprint` | yes | no |
| `rail` | `overview`, `sessions`, `kit`, `mods`, `settings`, `guides`, `workflows`, `studios`, `logbook`, `reviews`, `issues` | `overview`, `mods`, `settings` | no | no |

`packages/shell/layout-registry.json` is the list Fleet checks. **`rail`
is the places: the Cockpit, the Jobs tab (`sessions`) and the System strip's pages.
The Logbook page is the place `logbook`.** `dashboard.panels` is the Cockpit's
panels: `list` the Jobs list, `merge-line` the merge queue, `controls` Flight
control. A right column the layout empties gives its room to the stage.
`job.panels` is the panels an open Job stacks in the right column, in that
order, telling the Job as a story: `workflow`, `blueprint` (the plan), `field`
and `landing` (what the workflow ends in, the mods a Mod Job made among it).
Hiding a place takes it off the tab bar or the strip and nothing else:
it stays in the palette and in links.

| Field | Is | Notes |
|---|---|---|
| `version` | `1` | Required |
| `order` | List of ids | Named first, in this order; the rest keep their place after |
| `hidden` | List of ids | Not drawn. A non-hideable id is ignored |
| `first` | One id | No region takes it today; it is ignored |

A list is at most 32 ids, an id is lowercase letters, digits and `-` of at most
40 characters, the file is at most 4 KiB, and any other key inside a region makes
the whole file invalid.

## Example: the merge queue and Activity hidden, the Logbook off the tabs

```json
{
  "version": 1,
  "dashboard.panels": { "hidden": ["merge-line", "activity"] },
  "rail": { "hidden": ["logbook"] }
}
```

## Example: Blueprint first in a Job, then Workflow, the Field hidden

```json
{
  "version": 1,
  "job.panels": { "order": ["blueprint", "workflow"], "hidden": ["field"] }
}
```

## What you must not do

| Never | Because |
|---|---|
| Invent a region or an id | It is ignored, and the owner sees no change |
| Try to hide Mods or Settings | They stay drawn so a bad layout can always be undone |
| Edit `mod.toml` beyond `description` and `version` | `name` must equal the folder, and `kind` is `layout` |
| Add a second file or a link in the folder | Only `mod.toml` and `layout.json` are read |
| Write the owner's own choices | `layout_choices` is Settings, Layout, and it outranks every mod |

**What he sees.** Layout mods apply in name order, a later name's `order` or
`hidden` replacing an earlier one's for that region, and his own choices in
Settings, Layout apply over all of them. Reset to defaults there clears his
choices and switches layout mods off, so tell him the mod is still installed
and can be switched back on in Mods. **Promotion is his act**, as for a theme:
Bridge calls `promote_mod`, and a Job of its own opens the pull request.

# Making a tracker mod

**A tracker mod teaches Fleet a tracker it has no adapter for**, over the
tracker's command line, its HTTP API or an MCP server. Fleet then lists, searches and reads its
issues like the built-in ones. `docs/concepts/mods.md`, *Tracker mods*, is the
whole file format.

## The steps

| # | Do | Tool |
|---|---|---|
| 1 | Find how the tracker is read: a CLI that prints JSON, an HTTP API, or an MCP server. Run it by hand once and keep a real answer | Your shell |
| 2 | See what exists, so the name is new | `list_mods` |
| 3 | Make the folder | `scaffold_mod` with `name`, `description`, your `session_id` and `kind` set to `tracker` |
| 4 | Write `tracker.toml` in that path. Test each `map` against the answer you kept: `jq '<map>' answer.json` | Your editor |
| 5 | Check it; read `problems`, fix, check again | `validate_mod` with `name` |
| 6 | Try it against the repository; read `reason`, fix, try again | `try_tracker_mod` with `name` and `manifest_id` |
| 7 | Name it in the repository's Manifest, `tracker:` then `provider: <name>` | A Manifest edit, in the repository's `armada.yml` or in Kit's copy of it (`docs/concepts/kit.md`, *What a repository runs*) |

**You are not done until a try has `passed` true.** Fleet uses the mod only
while its last try of the file as it stands passed, so every edit after a try
needs another. `valid` true alone is not enough.

## What the file must say

`[guide]`, `[search]` and `[read]` are required; `[credential]` and `[list]` are
optional. Each call names `transport = "cli"` (with `argv`),
`transport = "http"` (with `url`, and optional `method`, `headers`, `body`) or
`transport = "mcp"` (with `server`, `tool`, and optional `env`, `arguments`), and a
`map`: a `jq` filter from the answer to a list of issues with Armada's keys.
`reference`, `title`, `url` and `closed` are required in every issue; a row
without one fails the call. `{query}` goes in the search, `{reference}` in the
read, and `{credential}` anywhere once `[credential]` names a command that
prints the secret.

Add `[links]` with `reference`, a regular expression for how a ticket is named in
a dispatch's or a message's words, so a Job whose request names one holds that ticket. Write
`reference = '\bLIN-\d+\b'` for keys in the words, or put the key in a group,
`'linear\.app/acme/issue/(LIN-\d+)'`, to link by address: the first group is the
reference where there is one. It must read as a pattern and not match the empty
text, or the file is refused with the reason. Without it nothing is linked.

| Never | Because |
|---|---|
| Write a token into `tracker.toml` | The file is read by anything on the machine. Read it from the keychain in `[credential]` |
| Use `sh -c` in `argv` | Arguments are passed whole and never through a shell; a shell would reopen that |
| Name the mod `github` or `jira` | Those words choose the built-in trackers, and the mod would never be reached |

An `mcp` call starts its `server` for the one call. Put the token in `env`, as
`{credential}`, never in `server`, which is refused a placeholder. `{query}` and
`{reference}` go in `arguments`. The `map` reads the result's `structuredContent`,
or else its first text part parsed as JSON; a server that gives neither fails the
call, and so does `isError`.

## Example: a tracker read over its HTTP API

```toml
[guide]
language = "Tracker filter"
guide = """
A filter is `field:value` pairs joined by spaces. Fields: state (open, closed),
assignee (a login, or me), label. Example: `state:open label:bug`.
"""
open = "state:open"
assigned_to_me = "state:open assignee:me"
fields = ["State", "Priority"]

[credential]
argv = ["security", "find-generic-password", "-s", "tracker-token", "-w"]

[search]
transport = "http"
method = "POST"
url = "https://tracker.example.com/api/search"
body = '{"filter": "{query}", "limit": 100}'
map = """
.issues | map({reference: .key, title, url: .link, closed: (.state == "closed"),
  assignees: [.assignee], labels, updated_at: .updated, body: .description,
  fields: [{name: "State", values: [.state]}, {name: "Priority", values: [.priority]}]})
"""

[search.headers]
Authorization = "Bearer {credential}"
Content-Type = "application/json"

[read]
transport = "http"
url = "https://tracker.example.com/api/issues/{reference}"
map = """
{reference: .key, title, url: .link, closed: (.state == "closed"), body: .description,
 author: .reporter, opened_at: .created,
 comments: [.comments[] | {author: .by, at: .created, body: .text}]}
"""

[read.headers]
Authorization = "Bearer {credential}"
```

## Example: a tracker read over an MCP server

```toml
[guide]
language = "Tracker filter"
guide = "A filter is `field:value` pairs joined by spaces. Fields: state, assignee, label."
open = "state:open"
assigned_to_me = "state:open assignee:me"

[credential]
argv = ["security", "find-generic-password", "-s", "tracker-token", "-w"]

[search]
transport = "mcp"
server = ["npx", "-y", "@example/tracker-mcp"]
env = { TRACKER_TOKEN = "{credential}" }
tool = "search_issues"
arguments = { filter = "{query}", limit = 100 }
map = """
.issues | map({reference: .key, title, url: .link, closed: (.state == "closed"),
  updated_at: .updated, body: .description})
"""

[read]
transport = "mcp"
server = ["npx", "-y", "@example/tracker-mcp"]
env = { TRACKER_TOKEN = "{credential}" }
tool = "get_issue"
arguments = { key = "{reference}" }
map = """
{reference: .key, title, url: .link, closed: (.state == "closed"), body: .description}
"""
```
