# Window and layout model

**Part of:** [Design System — UI & Voice](../design-system.md).

One responsive prototype covers all widths — not separate comps per
breakpoint.

## Window chrome

Frameless, `titleBarStyle: 'hiddenInset'`, macOS traffic lights inset over
the **title row** — the bar across the window's top carrying the one bar,
the plan's limits, Helm's reopen control, Fleet's status dot and one menu
holding the account, Sleep, Settings and Doctor. A wordmark starts the row, and the
window's tabs are drawn in its centre (Navigation, below). The menu button carries a moon while Sleep is on and an
amber dot while signed out. **The one bar searches** (the owner, 10 Oct 2026): it opens the palette, centred
across the window and not on the bar, and the palette offers what was typed as the work too, so the row draws no
Dispatch button. Dispatching is its own sheet, centred across the window and not hung from the bar, and `n` opens it without the palette; the Cockpit's deck holds a
Dispatch tile for the same sheet. The row sits flush on
`--bg-base` with no rule under it, so the panels below carry the frames. **The dot is
permanent chrome**, corrected on 18 Sep 2026 from the conditional one #1438
shipped — see Navigation, below. Costs a custom drag region: the whole row is
a drag region, and every control inside it opts out by hand — the dot included,
since a drag region swallows the pointer and a tooltip inside one never
opens.

## Navigation

**There is no left column.** Bridge/1088 replaced the rail and the status bar with a
column of Work, Machine and Fleet panels, and #2301 replaced that column with the
Cockpit window's own chrome: a tab bar in the title row and, under System, a sidebar of
entries. The deck's own left column holds the repository picker above the Jobs and
Sessions list, and it is the Cockpit's, not the shell's. `⌘\` (`toggle_sidebar`), the
48px rail and the rail's collapse rules went with the column; the decision that
governed them is `.claude/decisions/2026-09-22-the-rail-is-a-choice.md`.

```
title row   wordmark · tabs · search · plan limits · Helm · Fleet dot · menu
tabs        Cockpit · Jobs · Sessions · Studios · Retros · System        ⌘1 to ⌘6
System      one page, a sidebar of entries beside the picked entry
            Kit (kinds, Workflows, Models, Prompts, Harness) · Fleet (settings) · Bridge (Theme, Layout, Keyboard shortcuts, Guides, and the rest)
menu        account · Sleep · Settings (⌘,) · Doctor
```

> **Rule.** The window's places are declared, not derived from the surface roster.
> A page is a tab or a System entry because it was put there, and a page that is
> neither has no tab and stays reachable from the palette by name.
> Why: the roster is every destination Bridge can reach and the palette lists all
> of them. Worktree Slots, Merge line, Manifest and Checks are reached by name,
> and a navigation derived from the roster could not express that without
> deleting the destination. The declaration is `DECK_TABS`, `SYSTEM_PAGES` and
> `PLACES` in `packages/shell/src/surfaces.ts`.

> **Rule.** No page is reachable only by a place the window does not draw.
> Why: a page with no tab and no System entry was a row in the rail, and the rail
> is gone. The palette lists every surface the roster holds, and a layout that
> hides a tab or a System entry leaves it there.

**System opens the entry last picked, Skills where none has been.** Its tab is never
hidden, so the System sidebar is always one press away. A tab carries no count: the Cockpit's
tab carries a pip while a call waits and the window is somewhere else, and nothing
else on the bar carries a mark. Navigation tabs do not carry escalation or approval
counts, because the Cockpit's list carries both and two places to check are two
chances to disagree.

**Workflows is a System entry.** The definitions a Job runs come from this machine's
Kit and from each repository, so the surface that edits them sits beside Kit. Its
glyph is `workflow`, and its sidebar entry carries a pip while Fleet has left a workflow
file out. The pip takes no count, and `triangle-alert` stays Doctor's. It takes no
digit.

**Checks is reached by name.** Every Check the Manifest has had requested or run is
read there, beside the Jobs and the merge line that asked for them. Its glyph is
`shield-check`, the Check family's, drawn at 16px; the 12px result marks keep their
meaning in a table. It takes no digit.

**Fleet's liveness is one dot in the title row, at every width.** #1438 drew that dot
only while a left column was absent, which was the right shape for a column that
disappeared and the wrong one for a column that collapsed. The owner settled it on 18
Sep 2026: keep it, drawn always. A press opens Fleet's panel in a popover, so the
panel is one press from every tab.

> **Rule.** A status dot sits at the title row's trailing edge at every width,
> carrying Fleet's own state, its own colour, and its state in words as both an
> accessible name and a tooltip. It says liveness and nothing else.
> Why: chrome that appears and disappears on a resize is the cost, and the
> owner weighed it against the two alternatives and rejected both — removing
> the dot, and leaving it conditional. A mark that appeared only when something
> was wrong stays rejected, because silence would mean healthy and broken alike.

The colour is the Fleet panel's, from the one map — the dot and the panel
cannot disagree, because neither holds a table of its own. Status colour is
mapped, never chosen; see [Iconography](../iconography.md).

**The Session head is one row, and carries the context as a tank.** The terminal glyph (its tooltip is the address), the title on one line with the whole of it in its tooltip, Retro and End session as icons, the tank, the fresh, mod and clash marks, and the state mark last behind a rule.
The head draws no address and no model: the composer's Model select reads the model.
The tank is a mono pill of the live tokens and the percent of the window, `53.8k · 5%`, filled from its leading edge to that percent in a translucent tone with a crisp edge at the level, and a faint dotted line where the breakdown holds an autocompact threshold.
The tone is the plan ring's levels: normal below 80, warning from 80, danger from 95. The fill eases when the value moves, and does not under reduced motion.
Where Fleet holds a breakdown it is a button that opens an inspect card (hover or focus, kept by a press, closed by Esc or a press outside): kind `Context`, the tokens over the window, a state band `5% used`, the window as one slim bar of its categories, one fact a category with tokens in mono, and the heaviest memory files and MCP servers.
No dial, and nothing drawn where no window is held.
A Session or a Job held for the plan's limit wears `battery-low` and `Limit reached · resumes 6:40 PM` in place of its live mark.

**pid, port, protocol and uptime do not follow it.** They stay in the panel.
A dot cannot carry a figure; Fleet's own figures are read in the panel.

## Content area

**Full-width routes. No inspector pane over a route, no modal for Job detail.**
Board and detail are separate destinations, and job detail's own inspector is
inside the route rather than over it — below `--layout-breakpoint` it folds to
a sheet flush to that route's edge, never to the window's. **Helm's dock is the
one exception**: at `--layout-breakpoint` and wider it draws over the content
when open, because it answers questions about whatever is on screen rather
than inspecting one Job, and draws nothing at all when closed. The title row's
own Helm button is the one way back at every width, since no edge strip is
drawn at any; below the breakpoint it opens the dock as a sheet.

> **Rule.** Helm's dock is a layer over the content and never a column in it,
> and Bridge opens with it shut. Opening it changes nothing about the width of
> what is behind it, at any window width.
> Why: the owner, 22 September 2026 — *"I like the panel it's in, but I think
> it should overlay the contents of the screen. By default it's closed, and
> when it opens it overlays instead of shrinking everything."* A dock that
> took 380px made every rule written against the window wrong by that much
> while it was open, and three screens each grew a private answer to it in one
> day (#1583). It is the fold's own mechanism generalised: below
> `--layout-breakpoint` the dock has always been a layer.

> **Rule.** The workflow canvas's own inspector takes that same arrangement. A
> press on a node opens it on a layer over the canvas; nothing is open until a
> press; the canvas keeps the destination's full width at every state of it; and
> Close, in the panel's own head, is the way back. It is a layer inside a route
> and not one over a route, so the clause above stands.
> Why: the owner, 25 September 2026 — *"This panel should overlay the canvas. By
> default it is not open, so the canvas spans the width of the page and this
> panel is opened when you click a node."* It reads as a layer because it takes
> the card treatment every panel on the canvas takes; before that it carried the
> canvas's own ground and blended into it. **Not Helm's indigo** — `--helm`
> marks Helm's own chrome and nothing else (Tokens → Helm), and this panel sits
> inside one Job.

**It takes no scrim and no modality**, unlike the sheet it folds to. A person
reads a Job and asks Helm about it, so the screen underneath stays live and
reachable, and one press on Close or `⌘J` puts it away. `--w-work-min` is what
keeps a dock dragged wide from covering the thing being discussed.
See Responsive behaviour, below, and [Helm](../../concepts/helm.md).

This follows from what a detail view actually holds: the escalation
payload, the full attempt history including every prior Judge summary
rather than the latest, per-step evidence, and a diff. That is not
inspector content, and a split pane would cramp the thing the page
exists to show.

**If triage speed suffers in practice**, the fix is prev/next navigation
within the detail view — staying in the queue without splitting the
layout. Not an inspector.

### Job detail's own head

**Job detail is the one screen that keeps its own head**, because it has to
say which Job rather than repeat what Navigation already says — #1090 ended
the page head every other screen used to spend on its own name. Its head
carries the Job's name, its badge, the acts that end or replace it, and one
line of facts beneath.

| | |
|---|---|
| Placement | Job detail's own head, leading edge, before the status badge |
| Type | The Job's title `--text-base`, `--fg-default`, body weight |
| Facts | One line beneath, `--text-2xs`, `--fg-muted`, labels set back |

> **Rule.** No screen carries a breadcrumb trail.
> Why: Navigation is already the answer to where you are. Job detail carried a
> two-segment one — `Overview ›` and then the Job — and #1484 took it out: the
> first segment named the screen Bridge opens on whichever screen you actually
> came from, and it was never wired, so it read as a way back and was not one.

> **Rule.** Every fact on the head's line is a reading, never a value a person
> came to fetch.
> Why: *Where things are* is the region for a value you want — the repository,
> the branch, the worktree, the Drone — and a value drawn in both is two values
> that can disagree. The head is what you read on the way past. *Where things
> are* was retired with Overview's reframe of 29 Sep 2026 and its code is
> deleted; the rule stands.

> **Rule.** A fact carrying a machine value carries a word naming it.
> Why: the run was four bare strings, two of them the same string, and the
> owner read his own branch off it and asked whether it was a branch, a
> worktree or an id.

> **Rule.** How long a Job ran is `Run time`, wherever it is drawn, and it
> stops at the instant the Job stopped.
> Why: the Board column and job detail's head answer the same question off the
> same two instants. Two spellings read as two figures, and a figure still
> climbing on a Job that died reads as a Job still working.

## Responsive behaviour

**The desktop window has no minimum width.** A person keeps Bridge in a slim
window beside the work they are doing, and a floor refused them that (the
owner, 30 Sep 2026). `--window-floor`, 768px, is the narrowest tier a layout
is drawn for: at it and below, sheets go flush and closes go icon-only. The
touch client floors at 390px, which leaves 358px between its gutters.

> **Rule.** The main process sets no `minWidth`. `--window-floor` is a layout
> tier, and the touch client never reads it.
> Why: a floor on the window decides for the person how much of their screen
> Bridge takes. `packages/shell/src/floor.ts` answers whether the window is at
> or under the tier, and a touch client has no window to bound.

**One breakpoint at ~1100px, and one client boundary at the desktop floor:**

| | ≥ 1100px | < 1100px | Touch client |
| --- | --- | --- | --- |
| Tabs | In the title row's centre, the one bar an icon and its key beside them; the System sidebar beside the page while System is the place | The same. At the title row's narrow width its other controls are listed in its menu and the tabs stay | A bottom tab bar |
| The deck's left column | The repository picker over the Jobs and Sessions list, the feed under them | The same; the deck draws its panels at every width and has no narrow layout of its own | Not built |
| Job row | One shape at every width — a stacked row carrying the badge, the headline sentence and the labelled field run beneath | The same row. Nothing reshapes | The same row, field run wrapped |
| Helm's dock | A layer over the content when open, taking none of its width; closed draws nothing, and the title row's Helm button opens it | Closed draws nothing; the title row's Helm button opens it as a sheet over the content | Not built |
| Job detail's Overview inspector | A column beside the run | **A sheet over the run**, opened by pressing a step and closed by `Esc`; flush to both edges at the floor | Not built |
| The Workflow canvas's inspector | A layer over the canvas, opened by pressing a node and closed by Close; nothing until a press | The same layer, at the width of the destination | Not built |

**There were three columns here until #1583**, the middle one being 1100–1280
with the dock open — the band where the dock's 380px and job detail's two
floors could not both be paid for. The
dock costs the layout nothing now, so that band reads as the first and the
table is shorter by a column.

The last column is a client and not a window width. A desktop window under
768px keeps the < 1100px column's layout, with the floor tier's flush sheets,
and each surface that crowds there fits itself: under about 924px the title row's other controls are listed in its one menu.

The stacked row is the status grammar's own shape: headline sentence on
line one (`Job 12 stalled at step 3`), labelled field run on line two
(`api · 3 pokes · auth/session.rs · 12m · ~$1.80`). The badge stays
leading on line one so status is still the first thing caught.

**Every field is drawn wherever the row has the width for it**, and which
fields those are is what changes with width — never their order, and never
which of them is first.

This said *no field is dropped at any width* until the row could not honour
it. The Board grew a column for the Job's handle and a column for Tasks, and
the drawn floors then wanted 1085px on a four-fact row — measured at the
frame — against a 720px content column at the desktop floor. Shrinking every
other track to nothing still left that row 71px too wide at 954. The rule was
written when the run was shorter, and something had to give: `JobRowStacked`
had been dropping the trailing facts below 1100px for some time while this
page still forbade it.

**P2 is what decides which ones go, and it is why this is not a free hand.**
The facts a person decides from are the leading ones — the badge, the
headline, the identifier, the workflow and the step — and those never give
way. What gives way is the tail: how long it ran, who sent it, its repository
and its tasks, in that order, which is the order the Board draws them in. A
decision that needs one of those is a decision made on the Job itself, one
press away.

> **Rule.** Above `--layout-breakpoint` every fact the row carries is drawn,
> on one line, and a secondary value truncates with a tooltip carrying the
> full string.
> Why: a fixed shape is what lets the run read down a list, and a pointer can
> always reach what truncation hid.

> **Rule.** Below `--layout-breakpoint` the trailing facts give way, in the
> order the Board draws them, and the identifier never does.
> Why: a row is quoted by its handle, so the one field a person carries out of
> the row is the one field that cannot be the one to go.

> **Rule.** A count of facts the row can carry has a track list of its own.
> Why: a count with no list falls through to another count's, and the facts
> past it land in the slack track or in the action's column — measured at
> three widths, and each looked like a different bug: crushed to 16px, wrapped
> onto a second row, or printed under the action's button.

> **Rule.** Below the desktop floor the field run wraps to as many lines as the
> field set needs, and nothing truncates.
> Why: a touch width has no room to truncate into and no hover to open a
> tooltip, so the row grows taller rather than thinner.

**Honest cost:** the stacked row is taller than a table row, so fewer
jobs are visible at once. That is a real loss on a monitoring surface,
and it was accepted deliberately: the Job Board and Alerts disagreeing
about what a job looks like is what retired the two-shape version, and
the row is the most repeated element in the app.

**Validation:** the field set needs revisiting rather than the row where its
leading facts cannot carry themselves at 720px on one line, or at 358px
wrapped. A trailing fact giving way above those widths is the row working; a
leading one giving way is the field set being too long.
