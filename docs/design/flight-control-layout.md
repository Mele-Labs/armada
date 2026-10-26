# Flight control sorts what it holds by what each one does

**With a Job open, Flight control mixes three kinds of thing in one grid: words to the agent, views over the thread, and switches on the Job.**
Sorting them by kind, and drawing a switch as a switch, is what makes a control findable.
Two takes did it, and the owner walked more after them; *Approved, 26 Oct 2026* is the one he picked.

## The failure this answers

On 25 Oct 2026, walking a Job's right column, the owner pointed at Flight control: "I still feel like there is a better way to layout flight control.
I dont know why it feels so hard to find anything."

What a Job open draws today (`cockpit/deck/SessionTiles.tsx`, `docs/concepts/cockpit.md`, *The Job's flight controls*):

| Cause | What he sees |
|---|---|
| The order is the order the tiles were built in | Update, Auto land, Compact, Report, Map, Judge, Release, Artifacts, Subagents, Checks, Bays, Requests: talking, viewing and switching side by side |
| Twelve tiles alike | One size, a small word, glyphs that do not say their act (Map an orbit, Bays three bars, Release a plug) |
| An off tile keeps its seat | A fresh Job draws Compact, Judge, Release, Subagents, Checks and Bays grey, half the grid |
| Pages | Terminal and Pause are on page 2, behind a dot (Pages, approved 23 Oct 2026) |
| Two sets in one panel | With no Job open the same panel is Checks, Calls, Map and Timeline, on other keys |

The first is the cause the rest follow from.
Five of the tiles are views that open over the thread, two are switches with a state, and the rest are acts.
A grid of equal tiles gives the eye no kind to look in, so it reads every tile.

## Take A, grouped

The same panel, in three groups with a caption each, and no pages.

| Group | Holds |
|---|---|
| Job | Auto land and Pause as switches that read on or off, then Release |
| Tell the agent | Update, Compact, Judge, Report, then his own Words and Slash command tiles |
| Open | Artifacts, Checks, Subagents, Bays, Terminal, Requests, then his own Shell and Page tiles |

An off tile moves to the end of its group as a glyph alone, its reason the tooltip.
Map is not drawn: it is the stage's own switch, and returns with the deck's tiles when the Job closes.

## Take B, views as tabs

The views move to where they open.
The Job's thread gets a tab row, Thread, Artifacts, Checks, Subagents and Bays, each tab the layer the tile opens today (`session-view/layers.ts`).
A tab with nothing in it is not drawn.
Flight control keeps one row: the Auto land and Pause switches, then Update, Compact, Judge, Report, Release, Terminal and Requests.
Terminal stays a button, since it opens a shell in its own place and not a view over the thread.
The panel is one row tall, so the thread gets the height.

## Kept in both

- Every fact a tile carries: Compact's meter, Checks pulsing, the Subagents number, Bays lit or clashing, Judge reading, the sending mark.
- Every key, by tile, with its cap.
- Edit, and his own tiles. A tile he adds joins the group, or the row, of its kind.
- The deck's own tiles with no Job open. The approved take keeps Checks, Calls, Map and Timeline there, drawn as keys.

## The takes after A and B

Each was walked in the mock on a query switch: `?take=`, `&tabs=`, `&tabpos=`, `&views=` and, for the density rounds, `&density=`, `&heads=`, `&places=`, `&autopilot=` and `&jobname=`.

| Take | What it drew | The owner's verdict |
|---|---|---|
| A, grouped | Three captioned groups: Job, Tell the agent, Open | Not picked |
| B, tabs under the head | Thread, Artifacts, Checks, Subagents and Bays as a tab row under the Job's title | Not picked |
| B, tabs in the footer | The same tabs as the Job panel's last row, under the composer | Not picked |
| B, segmented tabs | The same tabs as a segmented control in the title row | Not picked |
| Views in the composer | The four views as icon buttons in the message box's row | "looked worse" |
| Icons in the Job head | The four views as icon buttons in the Job's title row | Picked |
| A Views panel, lower left | A panel under the Jobs list with a row for each view, key `i` | Not picked |
| Density: `tight` | Gutters, bands, rows and controls tighter, the type unchanged | Not picked |
| Density: `tight-type` | `tight`, and the Cockpit's type one step smaller | Picked, with the choices below |
| `heads=legend` | The panel's name set into its top hairline | Not picked |
| `heads=none` | No name band; the acts appear in the corner on hover or focus | Picked |
| `places=small` | The Places as a glyph and small text | Not picked |
| `places=icons` | The Places as glyphs, the name in a tooltip | Picked |
| `jobname=notch` | The open Job's state and name in the title bar's centre notch | Picked |
| `autopilot=hidden` | Autopilot not drawn | Picked, as a setting |

The density rounds began with the owner's ask: "I am looking at various mission control / cockpit user interfaces and they seem to be able to fit more on the screen.
Its like our font sizes, weights or spacing or something is restricting us".

## Approved, 26 Oct 2026

The owner approved a combination: the Job's views as icons in its head, Flight control as one row of keys, and `tight-type` with `heads=none`, `places=icons`, `jobname=notch` and Autopilot behind a setting.

| Changed | Reads |
|---|---|
| The views | Icons in the Job's head, as in B, where they open, and not tabs or a row in the composer |
| Flight control | One row of the deck's own tiles drawn as keys: Auto land, Pause, Judge, Report, Release, Terminal, Requests, then his own |
| Gone from the row | Update, Compact (now in the context figure), Map (the stage's own switch) and Pages |
| The Cockpit's type | One step smaller than the app's, as the ask above wanted |

The reasons are the two the owner gave: Flight control was hard to find things in, and the Cockpit did not fit enough.
Take B's reason stands for the head: the views sit where they open, and the thread gets the height.
`docs/concepts/cockpit.md` says what the Cockpit draws, and is the owner of every figure and name; this page keeps the takes and the reasons.

## What overlaps

Branch `armada/session-4d27072d`, not yet merged, opens Artifacts over the right column from the composer's ⇧⌘A.
Take B draws Artifacts as a tab over the thread, as main does.
Whichever take he picks, Artifacts has one home, settled when the take is wired.

## Order of work

1. The approved combination is wired as the only layout. The query switches go, with the takes that were not picked.
2. Tests hold what he approved, then the branch lands.
