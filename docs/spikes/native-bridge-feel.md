# Spike: How does the native Bridge feel, and what does it cost to run, next to the Electron Bridge?

**Answer: the native Jobs screen answers a switch or a keystroke in under 5 ms even in a debug build, and about 0.4 ms in release, so the 50 ms target is far from the part measured.**
What is measured is the CPU half of a frame, on a Jobs list that draws only a highlight when a Job opens and no thread at all.
The target is therefore not tested yet: a thread open cannot be measured until the native app draws one, and the part of a real frame that waits on the display (vsync, present, compositor) was not reachable without putting a window on screen.
The figures tune the build and gate nothing.

On memory, a headless release process holds 39 MB with the Jobs list drawn, and 73 MB as a real (hidden, off-screen) window.
The owner's running Electron Bridge held about 1.0 GB in its main window's five processes when read, and 1.5 GB with the four more renderers that were open beside it.
The two are not like for like: Electron was the whole Bridge in use on a busy machine, native was one list on mock data.

Measured on 9 Nov 2026 on an Apple M5 Pro, 18 cores, 48 GB, macOS 27.0.1, arm64, rustc 1.98.1, `gpui-pre =0.3.11`.
Load average was 1.7 to 2.1 for the native runs, and 12 to 19 when Electron was read, from other Jobs.

## How it was taken

`crates/bridge-native/examples/feel.rs`, on the branch `spike/native-bridge-feel` because it copies parts of `main.rs` that would drift, is an example binary, not a test, because gpui's Mac platform must be made on the main thread and the test harness runs tests on another.
It re-declares the crate's modules from `src/` and carries copies of `Shell` and `init_app` from `main.rs`, so a change to those must be mirrored there.
Nothing in the shipped binary depends on it.
`cargo build --example feel` (add `--release`), then `feel <measure>`: `job_switch_14`, `job_switch_200`, `frame_14`, `frame_200`, `typing`, `process`, `window`.

The headless setup is the walk check's: `HeadlessAppContext::with_platform` with `gpui_platform::current_headless_renderer()`, a 1100 by 760 window at scale 2.
A frame is `Window::draw`: layout, prepaint and paint into a scene, on the CPU.
Every figure is a wall-clock `Instant` around the call, over the count in the table.
The 200-Job screen is the fixture's Jobs repeated under new ids in the example, not a committed fixture.
Each switch and press was checked to change the drawn pixels, and the typed text was checked in a screenshot, so none of the figures times a no-op.

## The numbers

Times are milliseconds, median then 95th percentile.
Release is `--release` with the crate's own profile (none set, so cargo's default); debug is `cargo build`.

| Measure | Release | Debug | How |
|---|---|---|---|
| Job switch, Enter on the focused row, 14 Jobs | 0.25 / 0.39 | 2.20 / 2.50 | n=104. Key, keymap, action and effects, then `draw` |
| Job switch, Enter, 200 Jobs | 0.38 / 0.42 | 3.55 / 3.79 | n=104 |
| Job switch, a press on a row, 14 Jobs | 0.24 / 0.27 | 2.12 / 2.40 | n=112. Mouse down and up, `on_click`, then `draw` |
| Job switch, a press, 200 Jobs | 0.36 / 0.39 | 3.48 / 3.83 | n=208 |
| Typing in the walk window's note box | 0.43 / 0.46 | 4.46 / 4.69 | n=78. `InputState` through gpui's input handler, then `draw` of the whole walk window |
| Frame, Jobs screen, 14 Jobs | 0.10 / 0.11 | 1.04 / 1.11 | n=300. `refresh` then `draw`, so the whole tree is rendered |
| Frame, Jobs screen, 200 Jobs | 0.17 / 0.18 | 1.80 / 1.84 | n=300. The list is virtual; why 200 Jobs cost more than 14 was not isolated |
| First frame, 14 / 200 Jobs | 0.11 / 0.17 | 1.07 / 1.83 | Fonts and the Metal device are up by then |
| App, assets and renderer up | 60 | 72 to 88 | Before the window opens |
| GPU raster and read back of one frame, 2200 by 1520 px | 2.9 / 3.6 | 114 / 117 | `render_to_image`, n=60. Readback is not part of a window's present; the debug figure is gpui's debug Metal path |
| First GPU raster | 8.5 | 120 | Includes the first Metal pipeline use |

Of the switch figures, about half is the dispatch and half the `draw`, in both builds.
The headless press does not move the pointer, so a hover state is not drawn.

### Memory and CPU of the process

`ps -o rss=,time=` on this process, read at the start of each phase.
CPU is the change in cumulative CPU time over the phase, so its resolution is 10 ms.

| Process | Release | Debug |
|---|---|---|
| RSS before gpui starts | 10.5 MB | 13.8 MB |
| Headless, 14 Jobs drawn, idle | 38.7 MB, 0.0% of a core | 52.9 MB, 0.0 to 0.1% |
| Headless, 200 Jobs drawn, idle | 39.3 MB, 0.0% | 53.4 MB, 0.1% |
| Headless, redrawing the 14-Job screen 60 times a second for 10 s | 5.3% of a core, RSS +0.2 MB | 30.5 to 31.4%, RSS +0.2 MB |
| Same with 200 Jobs | 6.2% | 35.1% |
| Same, and rasterising each frame on the GPU with readback | 39%, RSS 66.8 MB | 99%, RSS 80.9 MB |
| Real window, hidden, off screen, 14 Jobs drawn, idle | 72.7 MB, 0.1% | 86.7 MB, 0.1% |
| Real window, 200 Jobs | 73.4 MB, 0.2% | not run |
| Real window, `draw` in a loop | 0.35 ms a frame, 0.6% | 2.1 ms a frame, 2.7% |

The redraw load is a ceiling: it renders the whole tree every 16.7 ms, and the app draws only when something changed.
The per-frame time in that loop is longer than the tight-loop figures above (0.87 ms against 0.25 in release), because the core clocks down between frames.
The real window loop ran about 13 times a second, not 60, with 124 to 127 frames in 10 s in all three runs, so its CPU percentage is of 13 frames a second.
Why was not isolated; a hidden window's timers are the likely cause.

### The Electron Bridge, read and not touched

Processes under `~/Library/Application Support/Armada/electron/40.10.6/Armada.app`, started from the build at `builds/current`, read with `ps` twice, 20 and 30 s apart.
CPU is the change in cumulative CPU time over that interval.
`ps`'s own `%CPU` is a lifetime average and was not used.

| Process | RSS | CPU, 30 s |
|---|---|---|
| Main (pid 15175) | 276 to 295 MB | 6 to 9% |
| GPU | 113 to 114 MB | 17 to 22% |
| Network utility | 83 to 114 MB | 0 to 2% |
| Audio utility | 39 to 40 MB | 1% |
| Renderer, the window up since launch (2 days 10 h) | 483 to 491 MB | 55 to 63% |
| Sum of those five | about 1.0 GB | about 92% |
| Four more renderers, 1 to 2 minutes old | 99 to 974 MB each | 0 to 12% each |
| All nine | about 1.5 GB | about 95% |

I could not tell, without touching it, which window each of the four newer renderers draws.
`armada serve` (Fleet, 310 to 346 MB) is not Bridge and is not counted.
The Bridge was open and doing work during both reads, so the CPU is not the Bridge idle.

| Electron figure | Value | Source |
|---|---|---|
| Thread open | 132 to 239 ms | Given in the brief. No page in `docs/spikes/` or `docs/` carries it |
| Job switch | not found | `grep` of `docs/` finds none |
| Typing | not found | `grep` of `docs/` finds none |

`docs/concepts/native-bridge.md` says opening a thread was mostly React drawing, and that #3356 cut per-update redraws from 2,211 to 561.

## Thread open

**The native app draws no thread yet, so there is no thread-open figure.**
Opening a Job today sets a highlight on its row in the list; the pane to its right is empty.
When a thread is drawn, measure:

- From the open act (Enter, or a press on a row) to the end of the first frame that draws the thread's tail, for a long thread (the Electron 132 to 239 ms was a long one) and a short one, on the same fixture in both apps.
- The same with the thread's rows already held, to split Fleet's reply from the draw.
- The tail frame with markdown, code and diff rows drawn, since those are the costly rows.
- A burst of Fleet events folded into one notify, and the frame after it.
- Scrolling up through the virtual list, as frame cost per frame and the longest frame.

## What was not measured

| Not measured | Why it matters |
|---|---|
| Vsync wait, present, compositor, display scan-out | A real frame ends there. At 60 Hz the wait for the next vsync is up to 16.7 ms, at 120 Hz up to 8.3 ms, on top of the figures above |
| Present of a real window | A hidden window gets no display link, so gpui asks it for no frame. `Window::draw` does not present, and the present path is not public |
| The input path through AppKit | Keystrokes were sent to gpui's dispatcher, not through `NSTextInputClient`, so IME and the event queue are missing |
| Thread open, and a Job's content | Not drawn yet |
| A Job switch or typing in Electron | No figure exists, and he was not to be driven |
| Fleet traffic | The mock is read from a file, no WebSocket, no event bursts |
| Scrolling, resizing, light and dark switching | Not exercised |
| More than one machine, more than one run of each | One Mac, one session |
| Electron idle | The Bridge was open and active when read |

**The headless number differs from a real window's in kind, in five ways.**
There is no display link, so nothing paces frames and none waits on a vsync.
`draw` stops before the present, so the GPU's submit and the layer's drawable are missing.
The scheduler is gpui's test dispatcher, not the run loop, so no event ever queues behind another.
The window is not on a display, so no compositor and no scale or colour-space handling.
RSS is lower: 39 MB headless against 73 MB for a real hidden window, because the headless run holds no AppKit window and no Metal layer until a frame is rasterised.

## Build weight

As measured before this spike, for the native app's cold debug build: 58 s, peaking at 2.1 GB in one process, with the load average going from about 5.3 to 5.5 (light).
Measured here, a release build of the example with `sccache` on and its hit count not read: 1 min 40 s wall, 65.7 s of user CPU and 9.3 s of system, peak 1.42 GB in one process, load average 2.05 before and 6.66 after, other Jobs running.
Rebuilding the example after an edit took 4 to 11 s in debug.
This slot's `target/` holds 11 GB for debug and 1.8 GB for release.
The Chrome spike's build figures, which add CEF, are in [`native-bridge-chrome.md`](native-bridge-chrome.md).

On these figures native build Jobs are not limited to one at a time.
They say a cold native build is the size of one ordinary Rust compile on this Mac: well under two minutes, one process near 2 GB of 48, and a load that stayed in the single digits.

## Reading the timings

| If you see | It means |
|---|---|
| Switch under 5 ms in debug, under 0.5 ms in release | The CPU half of the frame is far inside 50 ms, and the Jobs screen is not what will cost the budget |
| 200 Jobs 1.7 times the cost of 14 | Still far inside the budget. The list draws only the visible rows, so the cause of the extra cost was not isolated |
| GPU raster 115 ms in debug, 3 ms in release | Do not read gpui's debug rendering as the app's speed; judge on release |
| Real window 73 MB against headless 39 MB | The platform layer is about 34 MB, before any thread or Chrome |
