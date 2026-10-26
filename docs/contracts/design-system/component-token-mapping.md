# Component → token mapping

**Part of:** [Design System — UI & Voice](../design-system.md).

Tokens alone don't determine a screen. Without this section a design
tool infers which token each primitive uses — plausibly, and differently
every session, which is a slower version of the drift this contract
exists to prevent. Below is the binding for the primitives a job list
needs. Anything not listed follows the same logic: surfaces from Ground,
text from Foreground, interaction from Accent, and status **only** from
the status tokens.

**Global.** Focus is a 2px `--accent` ring at 2px offset, no glow. It
was a 1px `--border-strong` ring; once a secondary button took
`--border-strong` as its resting edge, the two were the same colour,
width and position and focus rendered as nothing — a resting edge and a
focus ring must differ on all three. Control focus now matches row
focus, and the accent already carried the keyboard focus edge. Disabled
is `--fg-subtle` text with hover suppressed — never reduced opacity,
which muddies status colors. **Dimming is a token, not an alpha** — a
de-emphasised row steps down to `--border-subtle` and `--fg-subtle`.
Every interactive element transitions on `--duration-fast`.

## Table — the Job Board row

The densest thing in the app and the reason the spacing scale is tight.

```
header row   32px · --bg-base · --text-2xs · --fg-subtle
             uppercase, 0.04em tracking (ALL CAPS here and on an eyebrow)
             bottom rule --border-default
body row     36px · --bg-raised · 12px horizontal padding
             row rule --border-subtle
hover        --bg-hover
selected     --accent-muted
primary cell --fg-default · --text-sm
secondary    --fg-muted · --text-sm
metadata     --fg-subtle · --text-2xs   (timestamps, elapsed)
mono cell    --text-xs mono              (job id, path, branch, duration, cost)
```

No zebra striping. At 36px rows it reads as noise, and the row rule
already separates. Status appears as a badge in its own column **and** as
a row tint of its own hue at `--row-tint` (5%), so a person scanning the
Board sorts rows by state without reading a badge. This read "never as a
row-background tint" until 2026-09-16: that was true at the badge's 12%,
where eight tinted rows were unreadable. At 5% it is a wash. Hover and
selection **replace** the tint rather than mixing with it, so
`--accent-muted` still owns selection, and the row's left edge stays the
focus and hover edge — status takes no edge of its own.

**An identifier copies on click.** A job id, a drone id, a branch
name — a value whose whole use is being quoted somewhere else — copies
to the clipboard when clicked and goes to `--accent` on hover.

**A path or a command is mono and does not copy by default.** Both have
somewhere to go: a path opens where it lives, a command opens what it
did. A surface with nowhere to open to may fall back to copying, but the
gesture belongs to the destination first, and a file list whose paths
copy instead of opening is the case this rule was narrowed for. It carries no `copy` glyph: the
affordance token is the affordance, and a 12px icon repeated down
fourteen rows is the noise Iconography's default-to-no-icon rule exists
to prevent. A toast confirms, because a clipboard write is silent by
nature and a failed one is otherwise indistinguishable from a dead
element. A value that copies does not also get a button that copies it.

## Badge — status

The one place status tokens are used directly. `{state}` is the enum
variant, and the label comes from the enum→verb table in the Voice &
Copy section, never hand-written.

```
background  --status-{state}-bg   (12% opacity variant)
text        --status-{state}
border      none
height      20px · 6px horizontal padding · --radius-sm
type        --text-2xs · weight 500 · sentence case
icon        required, 12px lucide, strokeWidth 2, leading, inherits text color
```

Icons are how the escalation sub-reasons and the `not_started` axis
values differentiate, since they share one hue each. Every badge state
carries an icon, so the column never reads as ragged and hue is never
the only channel. Full specification on [Iconography](../iconography.md).

12px rather than 11px is deliberate: lucide draws on a 24px grid, so
12px is an exact half-scale and a stroke of 2 lands on exactly 1px. 11px
scales to 0.917px and antialiases into fuzz on a dark ground.

**A bordered pill is a Job state and nothing else.** An origin tag and
provenance are plain sans in `--fg-muted`; drift states are plain sans in
`--notice-caution`. Two chips in
one row separated only by colour makes a reader learn a rule the screen
never states.

**The badge carries no leading dot.** Its job was telling a status chip
from a bordered pill that is not one; origin no longer carries a chip,
and with an icon mandatory on every state the dot is a second marker for
one claim.

## Button

| Variant | Rest | Hover | Use |
| --- | --- | --- | --- |
| Primary | `--accent` fill, `--fg-inverse` text, `--shadow-primary` | `--accent-hover` | One per view. Approve, Dispatch |
| Secondary | `--bg-sunken`, `--border-strong`, `--fg-default` | `--bg-hover` | Everything ordinary |
| Ghost | transparent, `--fg-muted` | `--bg-hover` • `--fg-default` | Row actions, icon buttons, toolbars |
| Destructive | transparent, `--status-completed-failed` text and border | fill at 12% | Kill, and Send it back on a step the gaming check holds — it sends work back rather than forward (owner, 2 Oct 2026). Never a filled red button |
| Tonal | `--accent-muted` fill, `--accent-hover` text, no border | 24% `--accent` mixed into `--accent-muted` | Chrome present on every screen, carrying the app's main entry. Never the one solid accent of a view |

```
height   36px default · 32px sm (use sm inside table rows)
padding  16px default · 8px sm
type     --text-sm · weight 500 · --radius-md
```

**Emphasis comes from fill, not size.** A primary action is `--accent`
fill at the normal 36px control height; a call to action is never scaled
up to make it matter. **A list row never takes one** — every row carries
one secondary control, because fourteen rows offering a decision would
be fourteen accent blocks. Urgency on a list is carried by the badge and
the ordering, and the accent is spent on the detail screen, where the
object of attention is one thing.

**Every button in a group is the same height.** A ghost action recedes
by losing its fill and dropping to `--fg-muted`, never by shrinking.
Mixed heights in one row read as a rendering bug.

**A secondary is filled one surface step from its ground** —
`--bg-sunken` on a card, `--bg-raised` on a sunken or overlay row. A
button filled the colour of the surface behind it shows only its text,
so it looks shorter than the primary beside it even where the boxes
match exactly.

Destructive stays outlined because a solid red button reads as an error
state rather than an action, and `--status-completed-failed` is already
spoken for as a *status*. Kill is deliberate, not alarming.

**Pending is the pressed control, and only that one.** Every variant
collapses onto one pending rendering — `--bg-sunken`, `--border-strong`,
`--fg-default` text, and the bar under Motion — so the button a person
pressed is the one control in its group that is not greyed out. It stays
focusable and refuses a second press. Disabling alone was the treatment
before, and it drew the pressed control exactly like its siblings.

## Tabs

Sections of one object, drawn as a segmented control: one track, and the
chosen tab filled inside it.

```
track      --bg-raised · --border-default · --radius-md · 2px inner padding
tab        32px · 12px horizontal padding · --text-sm · --fg-muted
chosen     --accent-muted fill · --fg-default · weight 500 · --radius-sm
hover      --fg-default
```

**The chosen tab is filled, not underlined.** The rail's own selected row is
`--accent-muted` filled, and a panel's tabs sit in the same eyeline as it — an
underline made one selection read as a different idea from the other, which is
what the owner saw on Manifest on 17 Sep 2026. The fill is the contract's own
for a selected row, with `--fg-default` on it because `--accent` measures
4.06:1 there and would not clear as body text.

**Neither a destination nor a panel's own filters.** Two filled tracks stacked
— a Job's destinations over the Record panel's filters — carry no hierarchy
between them, which the owner hit on 28 Sep 2026; the destinations are the
strip below. **A panel's filters are a dropdown in the panel's head**: the
trigger reads the chosen filter and its count, the menu lists every filter with
its count on this section's rule, and the chosen one carries the check. The
owner, 29 Sep 2026: nine segmented filters overflowed and wrapped at laptop
width.

**The track is `--bg-raised`, not `--bg-sunken`, and that is Button's own
rule.** A filled control steps one surface from its ground, and the grounds a
tab strip sits on are the canvas and a Sheet's overlay. `--bg-sunken` is 18
values below `--bg-overlay` and cut a visible hole in the activity log's
header; `--bg-raised` is 7 below it, and on the canvas the two differ by 7
either side of `--bg-base` — so one value serves both rather than a rule
scoped to a layer. Measured on the built app, 18 Sep 2026.

**The title row's places never wrap; they fall back to glyphs.** A strip in a
panel wraps, but the places share one row with the title bar's sides, and a
track on two or three lines read as broken there (the owner, 15 Oct 2026).
`TitleBar` measures the room its sides and the search trigger leave, and where
the labels are wider than that the places draw `iconsOnly`: each tab its
surface's registry glyph, its name in the tooltip. Where even the glyphs do not
fit, the row's start gives way, the repository's name truncating first; the
places are never drawn over it.

**Still the `tabs` primitive.** A segmented control is how the strip is
painted, not a new base component — hard rule 2 stands.

**The run sheet's scope control is segmented too, and deliberately different.**
*Changed / All* is a choice made before pressing Run, so it is built from
`radio` and fills the chosen segment solid `--accent` with `--fg-inverse`, the
way a pressed control reads; hairlines divide its segments because nothing
travels across them. A tab is a view already open, which is the rail's own
`--accent-muted`, and the fill travels, so it carries no dividers to slide
over. **Two renderings of one shape, told apart by whether the press has
happened yet** — and said here so the next strip does not pick one at random.

**One fill that travels**, at `--duration-base`: the eye follows the selection
from the tab it left, where an edge per tab can only swap. It travels by
transform and width so nothing reflows, does not travel on first placement,
and under reduced motion it simply lands.

## Destinations

Where inside one object a person is, drawn as underline tabs: plain text on the
surface, and a rule under the chosen one.

```
strip      no track · 1px --border-subtle under the whole strip · 4px gap
tab        --h-control-sm · 12px horizontal padding · --text-sm · --fg-muted
chosen     --edge-active --accent rule beneath · --fg-default · no fill
hover      --fg-default
figure     trailing mono --text-2xs in --fg-subtle, never a filled pill
```

**Two kinds of control at two levels is what buys the hierarchy**, and spacing
is not. A destination is a place you go; a filter narrows what is already on
screen. Drawn as one control they read as two of the same thing, which is what
the owner refused on 28 Sep 2026 — `.claude/decisions/` holds the exchange.

**The figure beside a name is only ever one this screen has read.** Zero draws
nothing, and the value is not always a count: the boards draw `Workflow 1 / 5`
and `Record 21` in the same slot.

**Job detail is the one surface that draws these**, its seven destinations.
`Tabs` is unchanged and is what everything else takes.

## Input

```
background  --bg-sunken        (recessed, opposite of raised)
border      --border-default
text        --fg-default · placeholder --fg-subtle
focus       2px --accent ring at 2px offset
invalid     --status-completed-failed border, message below in --text-xs
height      36px · 8px horizontal padding · --radius-md · --text-sm
```

Select, checkbox, and radio inherit the same border, focus, and height rules.

A switch is a recessed pill with a raised knob, and the state is written in the half of the track the knob leaves.
On lights the track in `--accent` with ON on it; off is the sunken ground with OFF.
A switch whose two states have names of their own passes `words`, such as 24H and 12H, and the track widens to the longer word.
A switch in a band or a bar is `size="sm"`, one badge tall (`--h-badge`); a row's is the default, a badge and `--space-2` tall.
Depth is the track's inset shadow and the knob's rim and drop, with no gradient, glow or sheen.

In a list row whose text opens something, the switch is the bare `SwitchControl`, named by its `label`, and only the control toggles.

## Dropdown menu

A floating layer, so it takes `--bg-overlay` and is the one place a
shadow is legal.

```
surface    --bg-overlay · --border-default · --radius-lg · shadow
item       32px · 8px padding · --text-sm · --fg-default
hover      --bg-hover
danger     --status-completed-failed text, --bg-hover on hover
separator  --border-subtle
label      --text-2xs · --fg-subtle
```

Sheet and dialog use the same surface treatment at `--radius-lg`.
Where it opens, which edge it aligns to and what it does when it does not
fit are under Floating layers.

## Tooltip

```
surface  --bg-overlay · --border-subtle · --radius-sm · shadow
type     --text-xs · --fg-default · 8px / 4px padding
timing   400ms delay in, --duration-fast
group    300ms grace: inside it the next tooltip opens on arrival
```

**A tooltip carries one of three things, and never a fourth.** A path,
a branch name or a full timestamp — the value behind an abbreviated
one. What pressing a control does, written at the control. Or what a
word naming an Armada concept *is*, from the one place those sentences
live. It never restates the text it sits on, and it never carries an
explanation the row should have made plain, per the briefing-register
rule. A criterion sentence, a Judge's grounds, a log line and a step's
own name already read as themselves.

**An act names the control it is on, and never the one beside it.** A
reader who presses on the strength of one that describes a neighbour
has learnt that the surface lies, which costs more than every correct
hover on the screen was worth — so this is worse than no tooltip, not
merely less good. The run tree's chevron nearly took *Click to open
details in the right panel*: true of the step name one column over, and
false of the chevron, which opens that step's facts in place.

**The delay is per group, not per tooltip.** Crossing a row of eight
annotated chips is one wait: the first waits out `--tooltip-delay`, and
for `--tooltip-grace` after any one closes the next opens on arrival.
A delay held per instance is eight waits, which is why nobody reads the
second chip.

**A concept sentence has exactly one home.** The three gate tiers are
`phaseSaid`; every other word is `concepts.ts` beside it, keyed by the
word a reader sees. Two explanations of what a Check is would be two
things that can disagree, which is the distinction the gate rests on.

Placement, alignment and collision are under Floating layers.

**The tooltip is for one line of words.** Anything with a head, a state
and facts is an inspect card, below.

## Stamp

```
type     the text around it · a native button with no chrome of its own
reading  the format set in Settings → Time (24-hour or 12-hour, seconds, date)
tooltip  the other reading
```

**A time is a `Stamp`, never a string a screen formats.** It starts as a
clock, or as an age ("4m ago") with `first="age"`, which is for the
places that read ages today. A press turns it to the other reading, and
the tooltip says what that is. With Date off, a clock from an earlier
day still carries its day ("Oct 12 14:02"), so a bare time is always
today's. `still` draws it as plain text inside a button or a summary,
where a button of its own would be a nested one.

**A surface turns its stamps together.** `StampSurface` wraps a list,
a panel or a page: a press on any stamp inside turns every one, and a
second press turns them back. A stamp outside a surface turns only
itself. Where a stamp cannot sit, as in a tooltip or an accessible name,
`timeSaid` writes the same clock as text.

## Inspect card

**One card opens on every rich hover, the same over a Session as over a
pull request.** A reader who has seen one has seen them all, and only what
is said in it changes with the kind of thing. It is `Inspect` and
`InspectCard` in `packages/components/src/primitives/Inspect/Inspect.tsx`. It replaced
`StopCard`, the Session ledger's preview bubble, the owner chip's card, the
merge queue light's card, the plan usage card and the map's callout, each
of which drew its own surface and kept its own timing.

```
head     kind glyph · kind in small caps · item id or place, mono, trailing
name     --text-sm · --weight-heading
state    glyph + words in the item's status tone, on a 14% wash of it
facts    label --fg-muted · value --fg-default, values right-aligned
section  a muted heading over more facts, or the caller's own body
keys     key chip + verb for each key that works on it now, ending in Esc
```

**The material is one solid surface.** `--bg-overlay`, a hairline
`--border-default`, `--radius-lg`, `--shadow-overlay`, and a 2px top edge in
the item's status tone. No glow, no blur and no translucency.

**The state band is the only other place the tone is drawn**, and the tone
is the status token the item already maps to, never a colour chosen for the
card. A card with no state has a top edge of no colour.

Facts are `--text-2xs` in tabular figures, so a column of figures lines up
on the right edge, and a value the system made (a branch, a path, an id, a
duration) is mono. The card is narrow or wide and nothing between.

**One rule set opens, pins, closes and places every card.**

| | |
|---|---|
| Opens | On hover after `--tooltip-delay`, and on keyboard focus. A press focuses a button too, and it is the press that decides then |
| Switches | While one is open, or within `--tooltip-grace` of one closing, the next opens on arrival, and the first is put away. One card is open at a time |
| Pointer | It can cross into the card; the card closes `150ms` after the pointer leaves both |
| Pins | A press pins it, where the trigger's press is not already an act. A pinned card is a dialog named for the item, takes focus, can be pressed in, and stays when the pointer leaves. A row that opens on a press, or a chip that selects, does not pin (`pin={false}`) |
| Closes | Esc, or a press outside. Esc on a card the reader holds is taken whole and returns focus to the trigger; on a card only hovered it goes on to whatever else answers it |
| Placed | Below the trigger, or beside it on the inline-start side for a row in a column whose card below would cover the rows under it. Anchor positioning with a flip where there is no room, in the top layer, so it clears every sheet and is never clipped by what it hangs off |
| Reads | Hovered it is a `tooltip` that describes its trigger; pinned it is a `dialog` |
| Motion | It eases in for `--duration-fast` only where motion is allowed, and draws still under reduced motion |

**The keys are read from the registry.** A key that works on the item is
an action of the shortcut registry, so Settings → Keyboard moves it in the
card too; a key only the browser answers, such as Enter on a focused
button, is written by hand. The footer always ends in Esc.

**The activity graph keeps its own `Hour` card, by the owner's choice
(16 Oct 2026).** It is a `Tooltip` in `card` mode and the only one left.
A new rich hover is an inspect card; a second card of its own would be a
second answer to the same question.

## Prose — model-written markdown

Free text a model wrote — a Judge's consequence, a flag's citation, a
Drone's turn, a Helm reply — is drawn by `Prose` as GitHub-flavoured
markdown, through `react-markdown`, `remark-gfm` and `remark-breaks`.
**The owner chose full markdown on 1 Oct 2026**, over a hand-written
subset that refused links, tables and blockquotes. A single newline is a
line break, the way a forge draws a review comment.

**The body inherits the surface's size**, so one component reads at
`--text-sm` in a dialog and `--text-2xs` in a rail. Only headings and the
table header declare a size: the heading carve-out under Typography, and
the Table primitive's own header step.

**Block flow, never flex.** `-webkit-line-clamp` counts no lines inside a
flex box, and Prose is drawn inside `Clamped`. Every gap is a margin
between siblings.

| Written | Drawn |
|---|---|
| `#`, `##` | the line at `--text-lg`, `--weight-heading` and `--fg-default` over a `--border-default` rule, as a paragraph with `--space-6` above. It gets no entry in the page's outline |
| `###` | the same at `--text-base`, with no rule |
| `####` and deeper | the same at body size |
| `**bold**`, `*italic*`, `~~struck~~` | `--weight-heading` at `--fg-default` (the carve-out under Typography), slant, a line through in `--fg-subtle` |
| `` `code` `` | mono in a `--bg-overlay` well edged in `--border-default` |
| Fenced block | mono on `--bg-sunken` at `--radius-md`, edged in `--accent-muted` with a two-unit `--accent` leading edge (the carve-out under Accent). It wraps rather than clipping |
| `-` list, `1.` list | discs, and mono numbers at `--weight-heading`, in `--accent` |
| `- [x]` task | a disabled checkbox in `--accent`, in place of the marker |
| Table | the Table primitive's header: a `--bg-overlay` band, `--text-2xs` caps at `--tracking-caps` in `--fg-muted`, `--border-default` beneath. `--border-subtle` between rows and around the table. Cells wrap; column alignment is dropped |
| Blockquote | a callout on `--accent-faint` with a two-unit `--accent` leading edge, words at `--fg-default` |
| `---` | a `--border-subtle` rule |
| Link | `--accent`, underlined on hover (hard rule 2). Only `http:` and `https:` are links; anything else draws as its text |
| Image | a link to the address, labelled by its alt text. The CSP's `img-src 'self' blob:` would draw a remote image broken |
| Raw HTML | its characters. No `rehype-raw` |

**A link hands its address to the shell and the window goes nowhere.**
Bridge opens it in the system browser through `openLink`, which takes the
address the text carried and checks only its scheme. Every other opener reads
its address in main, but that is not possible here: a link in model text has
no id main could look it up by. Without an opener, a link draws as its text.
A refused open is said in a toast, in `whyNotOpenedLink`'s words.

## Overview summary tiles

Overview's four counts, each its own card, two by two.

```
tile     the card treatment under Depth, --radius-lg, and a radial wash of
         the tile's hue at --status-wash from the top leading corner
label    --text-xs, weight 600, in the tile's hue, after a --dot of it
count    --font-mono --text-2xl, --fg-muted; past zero, the tone the
         caller passes
```

| Tile | Hue |
| --- | --- |
| Needs you | `--status-awaiting-review` |
| Running | `--status-running` |
| Queued | `--status-not-started` |
| Recently ended | `--status-completed-success` |

**The label takes the hue at every count, and the count only past zero.** An
amber "0" reads as something waiting. The label reads 4.39:1 or better on its
own wash, `not_started` the lowest.

## Overview empty state

```
card     the card treatment under Depth, centred content
line     "No jobs." --text-base weight 600, then "Propose one." --fg-muted
action   Dispatch, Primary — the view's one solid accent
```

**No icon.** Iconography's empty-state rule holds. The title row carries no
Dispatch button since its one bar took the act (10 Oct 2026), so this is the
view's one solid accent, beside the empty space.

## Helm dock

```
frame    a 1px --helm-edge top edge fading to --border-glass, over the card
         treatment, with --helm-faint washed in from the top trailing corner
chip     --helm-muted fill, --helm icon, beside "Helm"
folded   the sheet the dock folds to below the breakpoint takes the same frame
         and chip, over the sheet's --bg-overlay: --helm reads 4.65:1 on its
         chip there, against the 3:1 a non-text mark takes
composer --bg-sunken, --border-glass
Send     --helm-muted fill, --helm text, --helm-edge border
```

## Instruments

**A mark that draws a measurement is an instrument, not an icon or an
illustration.** Twelve minutes of a Drone doing nothing is a flat line no
figure can show, and a footprint's shape says where the work went before a
path is read. The ban on decorative iconography stands; an instrument is
exempt only while every mark in it is a value from the wire.

| Instrument | Where | Draws |
|---|---|---|
| **Activity** | A running Job | Tool calls per 30s window over the last twelve minutes, one bar per window |
| **Footprint** | A finished Job's footprint | Each file as a column, width by lines changed, split into added over deleted |

```
activity bar      --instrument-activity at 75%; an empty window --border-subtle at 1.5px, never absent
activity axis     1px --border-default along the base
footprint added   --diff-add-fg at 55%
footprint deleted --diff-del-fg at 45%
outside the plan  1.5px --instrument-outside-plan outline, and the words beside the key
labels, key       --font-mono --text-2xs --fg-subtle
```

**An instrument takes the hue of what it measures.** Tool calls are the
running step's work, so activity draws in the running hue, aliased once in
`tokens/status.css`. A file outside every declared plan is drift, which a
badge already draws amber, so its outline is amber too. A file with no line
count is left out of the footprint and counted in its key, because a guessed
width is a wrong measurement.

**An instrument carries its own reading in words.** Each has a label naming
what it measures and an accessible description stating what it shows, such as
*no tool calls for the last 12 minutes*. It never animates in, and a new
reading replaces the drawing rather than moving it.

## Fleet panel

Fleet's panel, a popover on the title row's dot — what the status bar used to read.

```
state    --dot (6px) in the panel's head, beside the "Fleet" label — the
         trailing slot, so it is the same mark at every width
rows     pid / port / protocol / up, one row each: label --text-xs
         --text-label on the left, value --font-mono --text-xs --text-body
         right-aligned to the panel's edge —
         FigureList, `column="fit"`
detail   --font-mono --text-2xs --fg-subtle, the sentence a state carries
doctor   border-top --border-subtle above it; a dot, "Doctor", the outcome
         (--status-completed-success / --status-awaiting-review /
         --status-escalated) and the modules checked, in --fg-muted
```

**The build sits above the rows**, in a section edged below by `--border-subtle`:
a Select naming the build Fleet runs on (Main or Preview), one Button, and the
build's position against main. The owner approved the look on 8 Oct 2026.

```
select    Main | Preview. Choosing the other restarts Fleet and Bridge onto it
button    secondary, full width, a glyph and a label that follow the build in use
          Preview   RotateCw   Refresh preview    "Merge in-flight branches and restart on the preview"
          Main      RotateCw   Update to main      "Restart on latest main"
          Main, level with main: disabled, tooltip "On latest main"
          restarting: LoaderCircle, the label's -ing form, pending, disabled
position  "main" in --text-label, then Check (--status-completed-success) when
          level, ArrowUp and a figure (--fg-muted) for commits the build holds
          that main lacks, ArrowDown and a figure (--status-awaiting-review)
          for commits main holds that it lacks. The words are the tooltip
          while restarting: the "main" label gives way to a turning LoaderCircle and the
          stage in words (--fg-muted, sans): Merging branches (preview), Fetching main
          (main), Building Fleet, Building Bridge, Restarting Fleet, Reopening Bridge,
          Retrying build. The owner asked for words here, 8 Oct 2026; the ring alone
          shows until the restart has named a stage
failed    one line in --status-escalated under the button, Fleet's own words
```

**The button follows the selection, and on Main it is never left disabled
while main has moved.** The owner, 8 Oct 2026: *"if main is selected the app
should make sure its on the latest main"*. A position that cannot be counted
draws no mark and leaves the button enabled.

**A restart past a working Drone asks first.** The Dialog is neutral, titled
"Drones are working", lists the Jobs that would be adopted in mono, and says
"Cannot be redirected until it finishes", with the rest of what adoption costs
on the line's tooltip. No Drone working asks nothing.

**The rows were settled on 2026-09-17**, replacing two `·`-joined mono lines,
at the cost of two lines in a column that was already tall. **A state draws
only the rows it has a value for**, never a label beside a blank: a Fleet
being connected to or not answering has pid and port, and protocol and up
arrive with the connection.

**The same rows run across in a destination, `column="strip"`.** The right
edge is what a reader of a 160 to 380px panel runs down; a destination is the
width of the window, and there the same rows put a label and its figure a
hand's width apart. Pulse draws what a Job is running and what it is taking
that way. #1538.

```
cell     label over figure, content-sized, packed from the leading edge;
         they wrap as whole cells
label    --text-xs --text-meta, nowrap. Dimmer than a panel row's, which
         is --text-label
figure   --font-mono --text-lg --text-body, one line
cap      its own line under the figure: sans --text-2xs --fg-subtle,
         `of $30.00` under `~$9.20`. `Figure.detail`
gap      calc(--space-8 + --space-2) across, --space-3 down
group    one --border-width rule in --edge-rule, a gap either side, before
         the first figure a `Figure.apart` marks
```

**The contrast is size and hue and nothing else**, the rule this document sets
out under *Emphasis on the dark ground*. The figure climbs two steps to
`--text-lg` and the label drops one to `--text-meta`, so the band reads as
numbers with words under them rather than as pairs of equal weight.

**The board's own artboard drew 20px**, which is not on the type ladder, so
the figure is the step under it. `Pulse · the destination`.

**A strip carries no right edge and a panel carries no rule.** A column of
rows is read down one edge and a rule across it would cut that edge in half; a
strip runs along a band where nothing separates one reading from the next, so
the rule is what says the band changed subject. Pulse's falls between what the
Job is running and what it is taking.

> **Rule.** The label on a band is dimmer than the figure and the figure is two
> steps larger. A cap, a total or anything else the figure is measured against
> goes under it on its own line, never beside it.
> Why: a figure and its cap on one line double the length of the thing a person
> is trying to land on, and the pair read as a phrase. Measured on Pulse, where
> `~$1.80 of $3.00` at `--text-base` beside `1 running` and `none out` was the
> band the owner could not scan (28 Sep 2026).

**The state is the head's dot, not a row under it.** A row saying "Running"
sat at the top of the body with its own dot beside the words, under a head
already naming Fleet; the owner deleted it on 28 Sep 2026 — *"Delete this row
and just put the status dot next to the 'Fleet' title in the panel header."*

> **Rule.** Fleet's state is the dot in the panel's head, carrying the state in
> words as an accessible name and a tooltip. The body holds only what the head
> cannot: the figures, the sentence, and Doctor's line.
> Why: the head is where a person looks for a panel's own subject, and a second
> mark two lines under the first said the same fact twice in one glance. The
> words are not lost — the dot is a named graphic, as the title row's is.

**A state with nothing for the body draws the head alone.** One does: the
moment before Bridge has read the runtime file has no figure, no sentence and
no Doctor reading. The head keeps its label and its dot and offers no chevron,
because a chevron onto an empty box is a control that does nothing.

**Fleet's state is one of three, and the panel's dot names which** — the
same three the status bar used to carry, on the same grounds Doctor's pass,
warn and fail reuse the Job values rather than inventing a third set. A
connection reading none of the three — reading the runtime file, connecting,
a refused runtime file, a protocol Bridge does not speak — keeps a neutral
dot and names itself in the label instead. It is not a glyph, so "this panel
carries no icons" is unaffected; see [Iconography](../iconography.md).

**The dot has a second home and the mapping does not.** The title row draws
this dot at every width, from the same four-state map and the same word — see
Navigation, above. It read "while the left column is folded" until 18 Sep
2026, when the owner made the title row's dot permanent; the two are on screen
together, which is a duplicated reading and never a second mapping. Two tables
would be two things that can disagree, which is the whole reason the hue is
mapped rather than chosen.

```
running       --status-completed-success dot
              "Fleet running" · rows: pid, port, protocol, up
not running   --status-escalated dot
              "Fleet is not running" · mono: what the runtime file says —
              no file at its path, a pid held by nothing, or a pid held by
              something else — plus what to do, since Bridge cannot start it
unreachable   --status-awaiting-review dot
              "Fleet unreachable" · rows: pid, port · mono: alive, no
              answer for N, plus how stale the last read is
```

**This panel's rows carry pid, port, protocol and up only**; the running
count is Overview's own arithmetic.

The two failure states differ on the runtime file, which is the fact
that separates them: Fleet writes port, pid and protocol ID on
startup and removes them on a clean exit, so a missing file is a Fleet
that is not there and a live pid with no answer is a Fleet that is
wedged. Two different things to do about it, so two sentences rather
than one timeout message.

**A protocol mismatch is a degraded notice, not a fourth dot.** Where Fleet and
this Bridge carry different protocol IDs, Bridge opens no socket and the notice
reads "Fleet and Bridge do not match", with the first eight digits of each ID in
its detail row and a Restart Fleet button (rotate-cw, spinning while it works;
the tooltip names what a working Drone costs). Fleet is alive and its Jobs keep
running, so the edge is the degraded one and the headline is not red. See
`../practices/protocol.md`, What Bridge does with the ID it reads.

## Closed

- **~~Icon set for the escalation sub-reasons and the `not_started` axis
  values.~~** **Closed.** Specified in full on
  [Iconography](../iconography.md) — every badge state, navigation,
  actions, Doctor, and the rule for anything unlisted. lucide-react
  confirmed against Phosphor, Tabler, Radix and Heroicons; hard rule 5
  stands. The enum→verb test asserts an icon entry in the same pass.
- **~~Window and layout model~~** **Closed.** Specified in full under
  Window and layout model above — frameless `hiddenInset` chrome insetting
  the traffic lights over the title row, the window's tabs in the title row and
  a System strip under it (#2301 replaced the left column), full-width routes with no
  inspector but for Helm's dock, no page head and no status bar, and the
  floors and breakpoint under Responsive behaviour. Delivered as one
  responsive prototype rather than per-width comps.
