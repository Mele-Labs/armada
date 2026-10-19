# Tokens

**Part of:** [Design System — UI & Voice](../design-system.md).

The token set below is mirrored row by row in `packages/tokens/src/*.css`,
the authority on each value; each token's role, source file, contrast
measurements and revision history are tracked in the Armada Tokens
database. Components that consume a token are related to it from Armada
Components, which also records what is still missing and which journey
first needs it.

Reference as Tailwind classes mapped to CSS variables (`bg-surface-raised`,
`text-fg-muted`, `text-status-running`). Never raw hex.

## Ground

Deep desaturated blue-slate, not near-black. Reads as instrument panel
rather than terminal, and gives status color room to sit without
vibrating.

```
--bg-base        #0F1419   canvas
--bg-sunken      #0B0F13   wells, code blocks, log panes
--bg-raised      #161C23   table rows, and a flat card (see Depth)
--bg-overlay     #1D242D   dialogs, popovers, dropdowns
--bg-hover       #212A34   row and control hover
--border-subtle  #232B35   table rules, dividers
--border-default #2E3946   flat card and input edges
--border-strong  #3D4A5A   focus rings, active edges
```

## Depth

One look, drawn by `packages/components/src/depth.css`, `pages.css` and the roles in
`packages/tokens/takes.css`. The root carries `data-look="new"`, which
`@armada/components/src/look` sets in each window that loads the stylesheet. There is no flag to turn it off.

**Restraint is the rule.** The model is Linear, Vercel Geist, Raycast and the macOS HIG.
Elevation is a solid lightness step, a 1px hairline and a small neutral shadow. **No
gradients, no glows, no sheen and no texture** on a surface, a control or a state. Colour is the theme's: nothing in these
files names a hue, and a shade is a `color-mix` of a theme token.

**Four levels of lift**, each a shadow and the edge that goes with it:

```
rest      --shadow-rest      a card or panel on the canvas, a button at rest
hover     --shadow-hover     the same under the pointer
lifted    --shadow-lifted    picked up, or a panel with focus inside it
overlay   --shadow-overlay   a layer over everything: menu, popover, dialog, sheet
```

Edges are `--edge-rest`, `--edge-hover`, `--edge-lifted` and `--edge-overlay`, steps of
`--border-subtle` to `--border-strong`. A shadow is black on a dark theme and slate on a
light one. Nothing inside a panel takes a shadow.

**A panel is glass.** Every Cockpit panel and pane, the Studios page's cards, and the layers that float over
them share one material:

```
--glass-fill   56%   --bg-sunken over the canvas, translucent
blur           --blur-glass (1px), saturated 170%, so the canvas grid reads through
edge           1px rgb(255 255 255 / 0.11), 0.26 while focus is inside
highlight      1px rgb(255 255 255 / 0.05) along the top inner edge
menu, popover, palette, inspect card, combobox list
               --bg-overlay at 78%, --blur-overlay (28px), edge 0.12, --shadow-overlay
```

The fill is the sunken charcoal rather than the raised one so that the canvas shows through without
lifting the ground under text; `--fg-subtle` is held at 4.5:1 where the pool is strongest. Rows, fields
and wells inside a panel stay flat on their Ground token. The Flight map is not a panel: no frame, edge,
shadow, fill or header, and its canvas is transparent, so the grid shows round its floor and the panels sit
around it; its zoom and recentre buttons are round glass. No blur while a scrim covers
the panel: a Sheet or Dialog and what is inside it stay flat on `--bg-raised`.

**Takes of the material**, on `data-glass` (`takes.css`): none is `current`, the dark sheet above. `vision`
is a light frosted sheet: `--fg-default` at 10% over a 32px blur saturated 180%, an edge of 15% and a 24%
top highlight, a `0 12px 32px` shadow, 20px panels, 14px chips (8% fills for tiles, fields and selects),
pill controls and toolbars, and a sentence-case medium-weight header with no rule or fill. `vision-tight` is
the same material and chips at 8px panels, 6px controls and the tracked-caps header with its rule. Both lift
`--fg-muted` and `--fg-subtle` so `--fg-subtle` holds 4.5:1 on the sheet and `--fg-muted` on a chip.

**One sheet, whatever the panel holds.** The Job pane is the thread's own frame: the card inside it draws no
edge, shadow or glass, its head is the same band every panel has, and the message box is a flat `--bg-step`
field with a hairline. The Field and Progress beside an open Job, the repository picker and its button over
Jobs, and the bays strip are the same material at the same radius; the Field keeps an opaque fill, since its drawing animates.
A focused panel lifts its edge and shadow and fills no header. A state is the colour of a panel's edge,
never a glow. The repository field inside Autopilot is a flat step at the control radius.

**The canvas** is `--bg-base` under a static grid and one radial pool of `--accent-faint`, in a 760 x 480px ellipse on the top
leading corner, which the title row does not cut off (it has no ground of its own). `--helm-faint` pools on the
bottom trailing corner behind Helm's dock, only while the dock is open. The radial pools are the only
gradients, they are the light the glass shows, and they carry no information.

**The grid** is 1px dots on the shell, under every panel, from `--grid-pitch` and `--grid-line` in `takes.css`:
16px apart, 16% of `--fg-default`, under a 0.5px blur. Dots, not lines, so the map's own lines never cross a second
set behind it. The strength is set by contrast: `--fg-subtle` holds 4.5:1 on a sheet where the pool is at its
strongest, with no blur to soften it. Static, no fade, no animation.

**A `Card` on the canvas outside the Cockpit and Studios** (Overview, Reports, Settings) still draws the
tokens `--bg-glass`, `--bg-glass-end`, `--border-glass`, `--border-highlight` and `--glass-blur`, in
`tokens/colors.css`, until it takes the panel material above.

**Three look tokens in `takes.css`:** `--bar` (2px), the bar on a selected row's leading edge, `--blur-glass`
(1px), the backdrop blur under a panel, and `--blur-overlay` (28px), the blur under a menu, popover or palette,
which sits over text. The look's size for meta lines is `--size-meta`; `--text-meta` stays the
colour token it is.

**Radius is two values.** `--radius-lg` 8px for panels, cards, menus and the palette; `--radius-md` 6px for
controls, rows and tiles; `--radius-sm` 3px for badges. No pill except an avatar.

**Selection is flat.** A selected row is a low-alpha accent wash, 16%, and a 2px accent bar on the leading
edge. The keyboard cursor is the hover step and a hairline. Hover is one flat background step,
`--bg-step`. **A row never lifts, moves or grows a shadow under the pointer**; only a card that is itself a
control rises one pixel. An active segment of a tab strip is a solid step over its track with a hairline.

**A panel header has no icon.** It is small tracked caps over one hairline, with a keycap at the far end.
A count is never drawn beside what it counts.

**The Work row** is two lines on one left edge, with no icon for its kind. Line 1 is the title and a fixed
column of three slots: the clash mark, the phase glyph and the state mark, in that order, which lines up down the
list. Line 2 is the last thing the Job or Session said, quiet and cut to the width, and its age in mono under the marks.
A linked Session alone has a glyph, `link`, before its title. A queued row keeps the single line it had.

**The repository field** at the head of the Cockpit's left column is a combobox
(WAI-ARIA: a text input with a listbox popup). Pressing or focusing it turns it into a search box in place,
showing the current pick as its placeholder; the list opens under it as a glass layer and narrows as he types.
ArrowDown and ArrowUp move, Enter picks, Esc puts the previous pick back and closes. All repositories leads, a rule under
it; the repositories follow; Add a repository is a footer row outside the filter, reachable by arrow like any row.

**Studios on All repositories** draws one glass panel for each repository, headed by its name, and never
asks for a repository first. Nothing else about a list changes with the scope.

## Foreground

```
--fg-default  #E4E9EF   primary text
--fg-muted    #93A1B1   labels, secondary text
--fg-subtle   #7E8CA0   timestamps, metadata, placeholders
--fg-inverse  #0F1419   text on solid accent fills
```

## Accent

One accent, used for interactive affordance only — never for status.

```
--accent        #4A9EDB   primary buttons, links, selected state
--accent-hover  #5FB0E8
--accent-muted  #1C3A52   subtle fills, selected row backgrounds
```

**One carve-out: the structure of a model's text.** `Prose` draws its
list markers, its quote rule (over an `--accent-faint` callout) and the
leading edge of a fenced block in `--accent`, though none of them can be
pressed. The owner chose this on 1 Oct 2026 ("Widen Accent for Prose").
The marks sit on structure a reader scans, not on a status, so the "never
for status" half holds without exception.

**A status mark never sits on an accent fill.** `--status-running` is
1.28:1 against `--accent`, so a status line drawn on a primary button
cannot be seen. A control that has to carry a status mark first steps
down off the accent — to `--accent-muted` for a press, to the pending
rendering under Button while Fleet has not answered — and the mark reads
against that.

## Helm

```
--helm        #8E95FF   Helm's chip icon, Send's text
--helm-muted  14%       chip and Send fill
--helm-edge   38%       the dock's top edge, Send's border
--helm-faint  6%        the dock's wash, the canvas's pool behind it
```

Helm answers questions about whatever is on screen, so the dock needs to
read as Helm at a glance and not as another panel. **Indigo sits between
`--accent` and `--status-rejected`**, so it appears on Helm's own chrome only,
never on or beside a Job row. `--helm` reads 6.09:1 on `--bg-glass` and 4.82:1
on its own chip.

## You

```
--you        #E8C547   the message band's fill
                       light: #8F7000
--you-edge   55%       the message card's border
```

The owner's own messages in a Session thread need to read as his at a glance,
apart from the agent's row, another Session's band and the lease. **Gold sits at
hue 47**, brighter and more yellow than `--status-awaiting-review` amber at 39,
which is a state and reads as one. `--you` is only ever the band of his message
in a Session thread, never on or beside a Job row, so the two do not meet. His
acts, Approved and Answered, stay the inverse neutral and do not take it.
`--fg-inverse` on `--you` as the band's text reads 11.03:1 in dark and 4.68:1 in
light.

## Status — derived from the state machine

One token per Job state, and the set is the state machine's — not a
palette. The critical semantic distinction: `rejected` and `killed` are
**deliberate human decisions**, not system failures, and must not read as
errors.

```
--status-not-started        #8C97A6   dormant, queued
--status-running            #4FB8D9   active, in flight
--status-awaiting-review    #E5A93D   needs you, not urgent
--status-escalated          #EE8450   needs you, urgent
--status-completed-success  #4FAF7C   landed
--status-completed-failed   #E97878   system failure
--status-rejected           #B489DA   you declined it
--status-killed             #9BA3AC   you stopped it
--status-handed-in          #4FB4AA   no task state takes it now
```

`--status-handed-in` is the midpoint of running and completed-success. It was
added on the owner's call so a submitted task would read neither as pending nor
as done; on 17 Oct 2026 the owner had that task read as done, and no task state
takes it. The pull request review's custom tools still do.

Each has a `-bg` variant at ~12% opacity for badge fills. A Job Board
row takes the same hue at `--row-tint` (5%) — see Table.

**Contrast pass, 2026-08-20.** Five values were lifted in lightness so
badge text clears 4.5:1 as 12px text on its own 12% tint over
`--bg-raised`. Hue and semantic assignment are unchanged — nothing was
reassigned, only brightened.

| Token | Was | Now | Badge contrast, before → after |
| --- | --- | --- | --- |
| `--fg-subtle` | #5D6B7C | #7E8CA0 | 3.15 → 5.02 on `--bg-raised` |
| `--status-not-started` | #5D6B7C | #8C97A6 | 2.83 → 4.83 |
| `--status-escalated` | #E8763D | #EE8450 | 4.93 → 5.52 |
| `--status-completed-failed` | #DC5B5B | #E97878 | 4.06 → 5.12 |
| `--status-rejected` | #A97BD1 | #B489DA | 4.48 → 5.12 |
| `--status-killed` | #6B7684 | #9BA3AC | 3.27 → 5.52 |

`--fg-subtle` no longer equals `--status-not-started`, which closes the
contrast item Iconography flagged. `running`, `awaiting-review` and
`completed-success` already passed and were left alone. One shortfall
remains: `not_started` badge text on `--bg-overlay` reads 4.38:1, and
badges appear on floating layers rarely enough to accept it. `--accent`
as text on `--accent-muted` is 4.06:1, so selected rows keep
`--fg-default` text (9.68:1) rather than accent text.

**Escalation sub-reasons** all use `--status-escalated`, differentiated
by **label and icon**, never by hue — a column of oranges would be
unreadable. **The trigger list is not enumerated here.** It lives on
Workflow, it has grown twice, and several triggers are not yet in
`core-model`'s enum. What this document owes is the rule, not the
roster; the enum→verb test is what catches a trigger shipping with no
label.

**The approval axis is a status, not a reason.** `awaiting_approval` and
`queued` are statuses of their own, and what remains as `queued`'s reason
names the resource — with ready becoming the null rather than a value.
`queued` and its reasons share `--status-not-started` and differ by
label and icon; `awaiting_approval` left grey for amber. See Job.

**The symptom that forced this is a rendering bug, which is why this
document carries it.** A sub-dispatched Job inherited its parent's
approval before 23.13. Under a single four-value field, one out of
headroom computed as `pre_approved_queued` and never rendered on the Job
Board at all. Under the current set a `queued` Job carries its reason
naming the resource, so no combination computes to an unrenderable label.

> Long-term intent: generate these token names from `core-model`'s Rust
> enum via the same codegen step that emits the `ipc` TypeScript types,
> so adding a ninth state fails the build until a token exists for it.

## Below Job level

Drawn from the workflow rail, which broke the rule that hue stops at the
Job — a done step, a running step, and a Judge criterion verdict all
wanted it. **Hue below Job level exists only where `tokens/status.css`
declares it.** That file is the list; this section carries the
reasoning, and deliberately does not restate the roster, because an
enumeration here went stale twice.

Every value **aliases** its Job counterpart rather than introducing a
new one. The mapping is declared, so it is read rather than inferred.

```
--step-advanced    var(--status-completed-success)
--step-running     var(--status-running)
--step-waiting     var(--status-awaiting-review)
--step-failed      var(--status-completed-failed)
--step-failed-bg   var(--status-completed-failed-bg)
--step-stopped-bg  var(--status-escalated-bg)
--verdict-met      var(--status-completed-success)
--verdict-not-met  var(--status-completed-failed)
--run-running      var(--status-running)
--run-passed       var(--status-completed-success)
--run-failed       var(--status-completed-failed)
--run-stopped      var(--status-killed)
--phase-live-edge  --status-running 45% into --border-default
--phase-live-bg    --status-running 6% into transparent
--phase-live-rule  --status-running 25% into --border-subtle
--slot-held        var(--status-running)
--slot-free        var(--status-completed-success)
--slot-stranded    var(--status-awaiting-review)
--slot-stranded-bg var(--status-awaiting-review-bg)
--slot-warm        var(--status-escalated)
--slot-cold        var(--status-not-started)
--slot-closed      var(--status-rejected)
--slot-closed-bg   var(--status-rejected-bg)
```

**A worktree slot's bay takes its slot's hue.** On Worktree Slots a held bay is a
filled card under a `--slot-held` band, a free one an open dashed outline in
`--slot-free`, a stranded one hatched from `--slot-stranded-bg`, and the build
mark `--slot-warm` or `--slot-cold`. A slot not made, or not a checkout, takes
none. A slot a person closed is shuttered in `--slot-closed`, rejected's
violet, because closing is a person declining to lease it; a held one closed
keeps its band and holder and takes the closed mark beside its state. The owner asked for it on 4 Oct 2026: icons and a row tint alone did not
say which slots were available. Like a verdict, it never sums onto a Job.

**The acts in a tile's panel take the hue of what they do, from tokens that exist.** A destructive act (Clear, Delete branch, Forget Job, Remove slot) is `--status-completed-failed`, as the destructive Button is; a rescue is `--slot-stranded`; Close slot, Reopen slot and Stop take `--fg-default`.
Each has its label beside a 16px glyph, a tint of `calc(var(--row-tint) * 2)` and `--row-tint-recent` on hover.
**The tint stops at 14%**: worked on `--bg-overlay`, error red as text is 4.54:1 there and 3.53:1 at 28%, so the deeper hover step the bays take fails in a panel.
No alias was added.

**Step activity answers where the work is.** `retrying` and
`not_started` take no hue — `--fg-muted` and `--fg-subtle`. A **killed**
step takes none either, and that exclusion is load-bearing: killing is a
human decision rather than a system failure and must not read as an
error. The rail's current row keeps its `--accent-muted` tint and 2px
`--accent` left edge, which is emphasis and not status.

**`failed` is the one value that reports an outcome rather than a
position.** A step whose Check refused takes `--step-failed` with a bare
`x` glyph, following `advanced` taking `check` — the same mark as the
`completed_failed` badge, meaning the same thing one level down. It was
drawn neutral first, on the grounds that a Check result is measured and
measured facts render flatly. That was reversed: where a failed Check
ends the Job with no retry and no triage, that row is the entire reason
a person opened the screen, and making them find it by weight in a rail
of muted rows is the frustration the surface exists to prevent. **The
gate row beneath stays neutral** — the step's state is hued, the Check's
exit code is measured.

**Two step values carry a surface rather than a glyph hue alone:
`stopped` and `failed`.** A step whose retries are spent is its own
activity value — not retrying, and not waiting on you either, since
folding it into `waiting` would render a designed human gate and a dead
stop alike. Both take a surface for the same reason: a glyph only holds
while its row is selected, and the row that ended the Job has to stay
findable while you read the Check output beside it. **In a rail,
background states what the row is and the accent left edge states which
row you are on** — the surface is constant, selection adds the edge. One
of each per rail, because a Job stops or fails in exactly one place.
They differ in the glyph: `stopped`'s `flag` stays `--fg-default`,
because with the surface carrying the warning a hued flag would say it
twice, while `failed`'s `x` is hued, since failed is an outcome and
states it in both channels.

**Criterion verdicts are measured facts and render as flatly as one.** A
criterion is met or it is not. The red does not claim the Job failed: a
Judge refusal is the gate working, and the row's copy names which
criterion and why. **Verdict hue is per criterion and never sums onto
the step or the Job** — that is the rule that lets a red cross sit under
a running step beneath an escalated badge without any of the three
contradicting the others.

**A run started by hand takes Job colours, and is still no verdict.** A
Check or Command run from a Job's run sheet or the Manifest surface hues its
result: the code it expected is `--run-passed`, any other ending
`--run-failed`, one in flight `--run-running`, and one somebody stopped
`--run-stopped` — killed's grey, so it never reads as an error. The hue sits on the result's chip and the
elapsed figure, and it counts for nothing — a run writes no Evidence and no
`check_runs` row, and on Job detail the run sheet keeps its own heading so a
rehearsal never reads as the gate's result.

**Refusals sort first, and every criterion row carries its number.** A
card that reorders breaks correspondence with the frozen
`acceptance_criteria[]` order, so a citation to "criterion 4" would no
longer sit fourth on screen. Explicit numbering is what lets both hold:
the rows a person needs are at the top, and the citation still resolves.
See How are criterion verdicts encoded without status hue?

**The live phase takes running on its edge, with a faint wash.** One row
of a step's timeline is the one the Drone is inside, and on a card that
size a mark alone does not carry it across a desk: `--phase-live-edge`
lifts the card's own border 45% toward running, `--phase-live-bg` washes
it at 6% — below a summary tile's 9%, because a phase card is larger
again and its body is a log — and `--phase-live-rule` carries the same
lift to the line under its header. All three alias `--status-running`,
which is what a running phase is one level down. **The mark leaves that
header**; see Motion.

**Everything else below Job level stays neutral, with one exception.** An
origin tag and the retry marker carry position, surface, weight and glyph.
Drift is the exception: `gone` and `current` render in `--notice-caution`,
because a drifted file is behind rather than broken. Adding a value to
`tokens/status.css` is a contract change, not a design decision.

> **Rule.** A caution notice is the one Alert tone below Job level that
> takes hue, and it aliases `--status-awaiting-review` as
> `--notice-caution`.
> Why: amber already means a person is on this, and a run that lands in
> a Drone's worktree is that.

It carries no glyph, because `triangle-alert` is Doctor's and the
contract has no generic alarm glyph.

## Diff

```
--diff-add-bg     #14301F
--diff-add-fg     #6FD196
--diff-del-bg     #351A1D
--diff-del-fg     #E88A8A
--diff-context    #93A1B1
```

**The diff colours are the patch view's and every other place a count of
lines appears**: a log row's `+2 −2`, the files under a plan task, and
Produced's per-file counts and its size bar. What the numbers *are* still
differs by surface — a task's are the sizes of the Drone's own edits, the
diff's are the patch — and the label beside them is what says which.
Colour says added and removed; it never says which kind of number.

## Tool families

What a Drone's call *does*, in three hues, declared in
`tokens/tools.css`.

```
--tool-look    #8FA3D9   Read, Grep, Glob
--tool-change  #D98BB5   Edit, MultiEdit, Write, NotebookEdit
--tool-run     #C2B48A   Bash
```

**Its own file, and that is the rule rather than the filing.** Every
value in `status.css` aliases a Job status, so the mapping is declared
rather than inferred; these alias nothing, because they answer a
different question — not where the Job stands, but what the call in front
of you reaches for. **A tool colour never carries status, and a status
colour never carries a family.**

**Why it is worth a hue at all.** A Drone's hour is a long stretch of
looking, a burst of changing, then a run, and in a mono column where
every tool name is the same colour that shape is invisible — which is the
reading a person opens the log for.

**They sit near two statuses and are told apart by where they are**,
which is the risk hard rule 3 guards. `--tool-change` reads near
`completed_failed`'s red and `--tool-run` near `awaiting_review`'s amber.
Nothing in a log row is a status: a status is a badge, a mark or a row
surface, and a family is one word at the head of a mono line, so the two
never appear in the same slot.

**A tool the roster does not name takes no colour.** Hue is scarce and an
unclassified tool is not a fourth family — the name draws in the line's
own foreground. Adding one is a decision about what a family means.

## Node kinds

What a node on the approval canvas *is*, in four hues, declared in
`tokens/kinds.css`. The owner's call of 4 Oct 2026, scoped to that
canvas's nodes and nothing else.

```
--kind-setup     #7DB8AE   the brief, its base branch
--kind-step      #8B95E8   a Drone step, and the groups or Jobs it fans into
--kind-gate      #D2A6C8   a gate the work passes: Checks, a Judge, a person
--kind-delivery  #A8C07F   Done when, the pull request, Land
```

**A kind is a tint or an edge, never a solid band; a status is always the
solid band.** So the two never share a slot, the rule Tool families keeps
for the same risk. A node carries its kind as a `--space-1` leading edge,
always. At the gate, where nothing has a status yet, its band is the
kind's tint, 22% into `--bg-raised`, under `--fg-default`. On a running
Job the step machine's status takes the band, solid, and the edge keeps
the kind.

**Soft, and told apart from status by treatment as much as hue.** Each
sits near a status — setup near `completed_success`, the gate near
`rejected` — and a tint or a 4px edge is never read as a status, which is
a mark, a badge or a solid band.
