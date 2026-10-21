# How should the System tab, and Kit inside it, be laid out?

**Decided 2026-10-20.** He pointed at the Kit page, a grid of equal tiles, and asked for a cleaner, more intuitive layout: *"Honestly lets think about the entire system tab and how its laid out. Then maybe focus on Kit."*

What he was shown first: five System pages in five layouts (tile grid, card column, side tabs, two list and detail pages), and pages split by where a thing is stored rather than what it changes, so Theme, Layout and Guides each sat in two places and Kit's own workflow files had a page of their own.

**Chosen: one sidebar grouped by what it changes, and Kit as kind, then item, then detail.** Walked as `systemSidebar` and approved.

| Asked | He chose | The cost he took |
|---|---|---|
| How System is laid out | One sidebar, groups Kit, Fleet and Bridge, one search; the five tabs go | The Mods page goes: mods sit under Bridge › Theme and Layout. Settings' Model, Effort, Prompts and Harness move under Kit |
| How Kit lays out its items | Pick a kind, then an item; the detail carries the full text, the path, Open in editor and Show in Finder | A kind is read one at a time, not the whole Kit at a glance |
| Whether a plugin's skills, sub agents and commands show beside his own | Yes, each marked with the plugin's name | Fleet reads every installed plugin's folders, not only the harness home |
| Where Kit's models come from | Claude Code's own list, with a switch per model for whether Jobs may use it; the typed roster goes | Fleet asks the harness at start and when Kit opens; with no answer it falls back to its built-in names and says so |
| Whether a model shows its version | Named without one (Opus, Fable, Sonnet, Haiku), the version beside it as a fact; pinned older versions not listed | A Job cannot be held on an older version from Kit |

**What he changed on the walk.** The group headers blended into the list; he asked for the Job panel's repository-group style, mono caps with a hairline to the edge, and got it.

**Decided for him, as plumbing.** Switching a model off moves every setting that names it to a model still on in the same save, the judge and proposer to the last one on, because Fleet refuses a roster that a setting still names. Effort takes every level Claude Code lists, `xhigh` and `max` included.

**Where it landed:** Fleet reading plugin items, #2635. Fleet reading models from Claude Code, #2638. Bridge's sidebar and Kit, branch `bridge/system-sidebar-mock`.
