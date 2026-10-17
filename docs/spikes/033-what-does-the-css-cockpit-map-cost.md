# Spike 33: What does the CSS Cockpit map cost to draw?

**Answer: with nothing happening the map keeps the GPU process busy, about 0.75 of a core, and
the renderer's main thread is near idle. Busy, the main thread is the cost: 1.5 seconds of
script in 10.** This is the baseline the Pixi rebuild of the map (#2301) is measured against. The
section "After" below is the same measurement on the canvas map that replaced it.

Measured 17 Oct 2026 on macOS 27, headless Chromium from Playwright 1.62.1 (the headless shell,
no window), 1440 by 900, load average 6 to 19 from other sessions, `taskpolicy -c utility`. One
run. The CSS map as of `armada/session-0814bfed` before the rebuild.

## What was run

`apps/desktop/scripts/trace-map.mjs` (`pnpm -C apps/desktop trace-map`) serves the mock and opens
the `battlefield-progress` scenario, one repository with a few drones, then drives three phases.
Each phase is measured twice on a fresh page, since a trace perturbs what it measures: once
from CDP counters (`Performance.getMetrics`, `SystemInfo.getProcessInfo`, `LayerTree`), once from
a trace for frames.

| Phase | What happens |
|---|---|
| idle, 10 s | Nothing. On the CSS map the loops that never stop (bob, ping, breathe) are what ran |
| busy, 10 s | A moment of the scenario passes every second: edits land, subagents fly, the pull request opens, queues and merges |
| pan, 5 s | Sixty wheel events a second pan the camera, `+` and `-` zoom it |

## The baseline

Seconds of CPU over the phase. Renderer main thread from `Performance.getMetrics`; process CPU
from `SystemInfo.getProcessInfo`, so it takes in every thread of the process.

| | idle | busy | pan |
|---|---|---|---|
| Wall seconds | 10.0 | 10.2 | 5.1 |
| Main thread, `TaskDuration` | 0.18 | 2.01 | 1.55 |
| Main thread, `ScriptDuration` | 0.06 | 1.47 | 1.11 |
| Main thread, `LayoutDuration` | 0.00 | 0.01 | 0.00 |
| Main thread, `RecalcStyleDuration` | 0.01 | 0.12 | 0.05 |
| Renderer process CPU | 0.95 | 2.68 | 1.71 |
| GPU process CPU | 7.46 | 7.29 | 4.43 |
| Browser process CPU | 0.01 | 0.09 | 0.14 |
| Compositor frames drawn | 318 | 390 | 119 |
| Main thread paints | 190 | 2,500 | 3,880 |
| Composited layers | 128 | 128 | 122 |
| DOM nodes at the end | 1,315 | 1,662 | n/a |

Raw numbers: `033-baseline-trace.json`. The paint counts are from the trace pass, where a trace's own
cost raises them (the counter pass saw none at idle).

## What it says

- **Idle is not free.** 318 frames in 10 seconds with nothing moving is the CSS loops. The
  compositor draws them, and the GPU process spends 7.5 CPU seconds in 10. In this browser that
  is software GL, so it is higher than a real GPU would be; read it as relative.
- **Busy costs the main thread.** Script is 1.5 seconds in 10, over 2,500 paints, from React
  re-rendering the plane as the moments change.
- **Panning costs more per second than anything else.** 1.1 seconds of script in 5, since each
  pointer step sets state and re-renders the whole `Battlefield`.
- **A hundred or so composited layers** for one repository and a few drones. Each billboard and
  each slab face is its own.

## What it does not say

One run on a machine other sessions were using. A rerun can differ by a fifth. A real GPU would
move the GPU row and leave the main thread's. The scenario has one repository; the owner's
field has more.

## After: the canvas map

Measured the same evening on the same scenario, size and tool, after the CSS map was deleted
(`033-after-trace.json`; `033-after-quiet-trace.json` with `--quiet`, which sets `data-idle` as the
window does after a minute with no input). One run of each. The machine was busier than for the
baseline: load average 8 for the first and 13 for the second.

| | idle | busy | pan |
|---|---|---|---|
| Wall seconds | 10.0 | 10.6 | 5.1 |
| Main thread, `TaskDuration` | 0.06 | 1.99 | 0.58 |
| Main thread, `ScriptDuration` | 0.05 | 1.76 | 0.37 |
| Renderer process CPU | 0.29 | 2.14 | 0.72 |
| GPU process CPU | 87.2 | 81.6 | 38.8 |
| Compositor frames drawn | 705 | 440 | 162 |
| Main thread paints | 50 | 1,039 | 3,537 |
| Composited layers | 31 | 30 | 30 |
| DOM nodes at the end | 811 | 849 | 850 |

With `--quiet`:

| | idle | busy | pan |
|---|---|---|---|
| Main thread, `TaskDuration` | 0.02 | 0.82 | 1.19 |
| Main thread, `ScriptDuration` | 0.02 | 0.73 | 0.78 |
| Renderer process CPU | 0.03 | 0.99 | 1.42 |
| GPU process CPU | 2.0 | 5.2 | 1.9 |
| Compositor frames drawn | 0 | 9 | 43 |
| Main thread paints | 50 | 390 | 9,426 |
| Composited layers | 31 | 30 | 30 |

## What changed, and what did not

- **The renderer is idle when the map is idle.** Script at rest is 0.05 seconds in 10 against 0.06,
  and composited layers are about 30 against 128, since the map is one canvas and the DOM over it
  is a few hundred boxes with no art in them.
- **Panning costs the main thread a third of what it did**: 0.37 seconds of script in 5 against
  1.11, in the run without `--quiet`.
- **Busy costs about what it did.** Script is 1.76 seconds in 10 against 1.47. React still reads
  the whole field again each moment (the layout, the frame and the scene); this run did not
  separate that from the canvas's own work.
- **With the window held, the map draws nothing.** Under `data-idle` no frame is drawn in 10 seconds
  and the GPU process uses 2 CPU seconds, against 7.5 on the CSS map at rest.
- **Without it, the page still draws.** At rest 705 frames were drawn in 10 seconds, more than the
  CSS map's 318. A probe of the same page found four infinite CSS animations running, all
  `deck-bay-pulse`, the Bays pulsing while active, and none once `data-idle` was set. This run did
  not separate how many of the frames are those and how many are the canvas's own loops for the
  Sessions that are working.
- **Panning still paints a great deal**: 3,537 main thread paints in 5 seconds, close to the 3,880
  of the baseline. This run did not find which element repaints.

## What the after run does not say

One run of each, on a machine other sessions were using, and GPU rows moved a lot between runs of
this same code: earlier traces of the canvas map at rest on this branch read 2 to 13 GPU CPU
seconds, and the CSS map's read 7 to 73, depending on the load. Do not compare the GPU rows
across runs; the frame counts and the main thread rows moved far less. The GL is software
(SwiftShader), so the canvas is drawn on the CPU inside the GPU process; a real GPU would lower
those rows and leave the main thread's. The `--quiet` pan row is not a user's pan, since a real
pointer ends the hold at once. The scenario has one repository.
