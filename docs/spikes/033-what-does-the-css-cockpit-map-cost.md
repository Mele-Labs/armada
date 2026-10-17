# Spike 33: What does the CSS Cockpit map cost to draw?

**Answer: with nothing happening the map keeps the GPU process busy, about 0.75 of a core, and
the renderer's main thread is near idle. Busy, the main thread is the cost: 1.5 seconds of
script in 10.** This is the baseline the Pixi rebuild of the map (#2301) is measured against. It
changes nothing.

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
| idle, 10 s | Nothing. The loops that never stop (bob, ping, breathe) are what runs |
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
