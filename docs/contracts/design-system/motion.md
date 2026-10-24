# Motion

**Part of:** [Design System — UI & Voice](../design-system.md).

```
--duration-press   90ms     a press answering
--duration-fast    120ms    hover, focus, a body crossfade
--duration-base    180ms    menus, dialogs, toasts, a tab bar travelling
--duration-sheet   220ms    a sheet from the trailing edge
--duration-travel  300ms    a row re-sorting to its new place
--duration-pulse   1600ms   a loop that says still working
--duration-answer  1600ms   how long a control shows Fleet's answer
--duration-hold    900ms    holding a destructive control to commit
--duration-decay   45s      a changed row's mark fading
--ease             cubic-bezier(0.2, 0, 0, 1)
```

**Three clocks, for three audiences.** A *transition* (press to travel,
under 300ms) is for a person watching. A *loop* (pulse) says something
is still working. A *decay* (45s) is for a person returning: a change
away from the focus of attention is missed, and a one-shot flash is only
seen by someone already looking, so a changed row keeps a mark that
fades slowly enough to still be there when they look back. A decay is
neither a loop nor an entrance, so neither rule below governs it.

**No entrance animations on data.** A Job Board that animates rows in on
every poll is unusable. The rule is about data. **Chrome that a person
summoned may enter**: a menu rises from its trigger, a dialog scales from
0.97 with its scrim fading, a sheet travels in from the trailing edge,
and a toast rises, because each fires once, on an act, and says where it
came from. A row re-sorting under a person **travels** to its new place
rather than teleporting, because animation measurably improves tracking
an object across a change (Heer & Robertson, InfoVis 2007); a new row
does not travel, so nothing enters. "Placement resolves before paint"
still holds: a layer never moves after it lands.

**Two exceptions.** Nothing else on a board, row or tile enters, and a
reading that merely changes still decays.

**A call that needs the owner comes forward.** On Command
Central a Plan question, a failed Check or a Session waiting on a command
rises over the fleet at `--duration-travel`, and a call put off for later
flies back up to the waiting calls at the edge. Why: what needs the owner is
the one event the screen exists to raise, and a call that arrived
unannounced among the tiles could wait unseen (owner, 9 Oct 2026).

**The Cockpit powers on.** While Fleet starts or comes back, each of the
deck's panels comes online in turn: its frame traces, its label and glyph
light, and a scan line passes down it until Fleet answers. Then each
panel's data wipes in, in the same order, and the battlefield's
territories arrive one at a time and glide when the layout changes. It
plays on every launch, for at least a second, and on every restart. It
never plays on a reading that merely changes. Reduced motion draws each
state at once, with no scan line. Why: a restart used to read as a run
of faults and a map that jumped into place, and the owner asked for the
deck to come online instead (owner, 23 Oct 2026).

**A changed row decays.** When a Job's status changes, its row takes its
new status hue at `--row-tint-recent`, falling to the resting
`--row-tint` across `--duration-decay`, and a mono line in the same hue
says what changed and how long ago, fading with it. It takes no edge:
the left edge is focus and hover's. The decay rides on the resting tint
and ends at it, so there is one tint channel and never two computed on
top of each other. Every changed row decays, with no cap:
`[decay-cap]`.

**Press moves colour, never geometry.** A pressed control walks one step
down its ground ladder at `--duration-press`: `--accent` to
`--accent-muted`, a secondary or ghost to `--bg-sunken`; tonal drops its
hover lift back to its `--accent-muted` rest, because its only caller
sits on `--bg-sunken`. Nothing transforms and nothing reflows. A focused field's ring grows from its
edge and its ground lifts one step at `--duration-fast`.

**Nothing may carry information by motion alone.** Under
`prefers-reduced-motion` every transition duration is zero and each
state still reads from colour and label.

**A loop that never ends animates `transform` and `opacity` only.** A loop runs for as long
as the window is open, so what it animates is what Bridge costs idle. The compositor draws
those two with no repaint; `box-shadow`, `background-position`, `stroke-dashoffset` and the
rest repaint their element every frame. A glow is a pseudo-element with a static shadow that
animates `opacity` or `scale`; a dash that marches is a layer that slides one period. Use
`will-change` on the element that animates, never on a container.
`apps/desktop/src/main/compositor-motion.test.ts` refuses a new infinite `@keyframes`
that animates anything else, and holds the allow-list: each exception names why it cannot move.

| Measured against the mock in headless Chromium, ms per second | Before: `box-shadow` ring | After: `transform` and `opacity` ring |
|---|---|---|
| Main thread, window idle (`data-idle`) | 141.6 to 168.1 | 12.6 to 15.6 |
| Main thread, map moving | 73.2 | 30.6 to 37.2 |

What a loop still costs on the compositor does not depend on what it animates: any running
animation, even one small ring, holds frame production at the display's rate (about 150 to
210 ms per second of compositor and Viz time, against about 2 when everything is paused).
The floor is why a window nobody is looking at stills its loops (`styles/index.css`).

**A layer nobody summoned does not enter.** The licence above is for chrome a
person asked for, and it is the press that pays for it. A guide card opened by
a `?` scales from 0.97 like any dialog; the same card opened because somebody
met a piece for the first time is simply there. #1603.

**Teaching borrows a game's motion and never its scoring.** Motion may show how
pieces relate, where the relation is what is being learned — a task joining its
group, a group's work becoming a commit, members landing in order. **There are
no points, no streaks, no badges and no progress bar for its own sake**, and
this sentence is here in as many words because the next person to read *learn
from games* will reach for a badge. Whatever a relation is drawn as, it is
drawn once: three animations of one relation, drawn three ways, is worse than
none.

**What animates on a loop is what is still working**, because a hue or a
label can say *which* and only motion says *still*. Three things do: the
running mark, a control waiting on Fleet, and the live phase's top edge.
There was a rule that one thing animates per screen; it was retired on
2026-09-14, #1117.

**One loop per card, and that rule is what the third one bought.** A bar
travelling and a mark breathing in the same header are two things saying
one word, and a reader has to decide which of them they were watching. So
where a card sweeps, every mark inside its header holds still.

**A running step's card carries a phase track instead, and then the track is
its one loop.** Along its bottom edge, the step's parts in order — its
Drones, its gate's Checks, its Judge, and a person where one holds it — done
filled, now lit, next hollow. The part now breathes, a person's turn as much
as a machine's, so a step waiting on him is as easy to spot; the sweep does not run beside it, because
a bar along the top would say *still working* a second time about the same
card. The step panel's header draws the same track. The owner chose it on
3 Oct 2026, over a mark on the state pill that was too easy to miss.
`StepPhaseTrack`. A stopped step whose Checks a person sent to run again
draws it too, Checks the part now, for as long as the gate's live set
stands — its state still says stopped, and nothing else on the card would
say the Checks are running.

**A control waiting on Fleet sweeps a bar along its bottom edge**, from
the press until Fleet answers or refuses. Its label says what it is
doing — *Request changes* reads *Requesting changes…* — and the rest of
its group is disabled. The bar is `--status-running`, 2px, travelling at
`--duration-pulse`; under `prefers-reduced-motion` it holds still at full
width and the label carries the reading. A person who pressed it and has
waited five seconds is told under the group that Fleet is still on it.
The Job itself does not move until Fleet says so — a refused act would
otherwise have to snap back.

**The bar is one line across four states**, so a press, the wait and the
answer read as one object rather than a bar appearing from nothing and
vanishing to nothing:

| State | The 2px line |
|---|---|
| press | opens from the centre to full width at `--duration-press` |
| pending | travels in `--status-running` at `--duration-pulse` |
| accepted | fills the edge in `--status-completed-success`, held for `--duration-answer` |
| refused | retracts to nothing in `--status-escalated`, and nothing else moves |

A refusal is told from an acceptance by the line's colour and direction
before the label is read. The control never moves in any of the four.

**The Cockpit's call answers are such controls too.** The answer pressed,
Poke, Attach or Dismiss sweeps the same bar along its edge until Fleet
answers, with the tooltip *Waiting on Fleet* and no label of its own; every
other control on the card is held, and a second press is not a second send.
A refusal stops the line and the card stays, with Fleet's words in the toast.
The five-second sentence is not drawn there: past it nothing new appears and
the line keeps going (owner, 9 Oct 2026). **An answered or dismissed call is
held out of the deck until Fleet stops carrying it**, because Fleet takes the
POST before it has re-derived what the Session waits on; the hold lets go
after thirty seconds if Fleet still carries it, with nothing said.

**The live phase sweeps a bar along its top edge**, the same line on the
same clock: `--pending-bar` tall, `--status-running`, one segment
travelling at `--duration-pulse`, linear rather than `--ease` for the
control's own reason. It runs while the Drone is in that phase and stops
when the phase does. Under `prefers-reduced-motion` it holds still at
full width, and nothing is lost — `--phase-live-edge` and the wash
already say which phase is live.

**Under the phase name, what the Drone is doing right now.** The call
still in flight — one with no answer yet, which is the one thing the log
beneath cannot show, because its `answered` row does not exist. Between
calls it is the Drone's own last sentence, with no verb: nothing is in
flight to conjugate, and a verb invented for the gap would claim a call
nobody is making. The verb takes `--status-running` and nothing else on
the line does.

**The running mark animates continuously.** A hue says which step is
current; only motion says it is still working, and that is the reading
a static rail cannot give — it matters most on the step that has been
running for nine minutes.

**Every working mark pulses.** A list pulses every running row's Running
badge. Job detail pulses the rail's current step, and the header's Running
badge stays static, because the rail names *which* step is working and the
badge only names the Job's state. A sheet open over Job detail does not stop
the rail, and the sheet's own live mark pulses beside it. A
[Studio](../../concepts/studio.md) pulses every node that is still working. The
step bar never pulses — its job is where the work got to, which is a static
fact, and the badge sits in a fixed column on every row so the motion appears
in one predictable place rather than moving with the workflow's length.

**The live phase's header carries no mark at all.** It is the one place a
running mark was dropped rather than made to pulse, and the sweep is why:
a mark says *which*, and on the one card already edged, washed and swept
in running there is no *which* left to say — what a second mark would add
is a second loop. **Marks stay wherever one row among many is the live
one**: the plan's task rows, and the live narration row inside that
phase's own body. The word the mark owed a screen reader is still said,
off `enum-verbs.toml` rather than retyped, because hue, wash and motion
are three channels a reader may have none of.

Opacity and scale only, at `--duration-pulse`. The ring holds still, so
no row shifts and nothing reflows. Hue says *which* state, unchanged on
every running row; the pulse says *still working*, which is true of every
working thing on screen, so the pulse follows status rather than focus.
Under `prefers-reduced-motion` the pulse stops and `--step-running`
carries the reading alone. #1276 carries this to the screens.
