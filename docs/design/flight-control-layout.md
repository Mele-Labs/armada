# Flight control sorts what it holds by what each one does

**With a Job open, Flight control mixes three kinds of thing in one grid: words to the agent, views over the thread, and switches on the Job.**
Sorting them by kind, and drawing a switch as a switch, is what makes a control findable.
Two takes do it, and the owner picks one from their walks.

## The failure this answers

On 25 Oct 2026, walking a Job's right column, the owner pointed at Flight control: "I still feel like there is a better way to layout flight control. I dont know why it feels so hard to find anything."

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
- The deck's own tiles with no Job open. Moving Map and Timeline to the stage and Checks and Calls to the Jobs list follows B, if he picks it.

## What overlaps

Branch `armada/session-4d27072d`, not yet merged, opens Artifacts over the right column from the composer's ⇧⌘A.
Take B draws Artifacts as a tab over the thread, as main does.
Whichever take he picks, Artifacts has one home, settled when the take is wired.

## Order of work

1. A mock of each take, played on `?take=grouped` and `?take=tabs`, with a walk each. No tests.
2. He walks both and picks one, or a mix.
3. The picked take is wired as the only layout. `?take=` goes, with the other take and Pages, and `docs/concepts/cockpit.md` says what is drawn.
4. Tests hold what he approved, then the branch lands.
