# Spike: How long does a saved prototype scene take to draw natively?

**Answer: from a saved scene's markup to a drawn frame took a median of 17 ms in a debug build, of which reading the markup and its fixtures took a median of 3 ms.**
The slowest of 27 runs was 22 ms, so a save redraws well inside one second and the draw is not what an agent waits on.
Release was not measured, and the debug figure is the upper bound: the earlier spike on the native Jobs screen found debug about ten times slower than release for the CPU half of a frame ([the feel spike](native-bridge-feel.md)).
The machine was busy, with a load average of 19 to 24 from other Jobs, so these figures lean slow.
What is not measured is the wait on the display (vsync, present, compositor), which a headless frame does not have.

Measured on 23 Nov 2026 on an Apple M5 Pro, macOS 27.0.1, arm64, rustc 1.98.1, `gpui-pre =0.3.11`.

## What was drawn

The mock Job's sample scene, `crates/bridge-native/fixtures/prototype-scene.html`, with its fixture `prototype-jobs.json`.
It has two states, a list of four Jobs in one and a detail card in the other, with tabs, chips, buttons and a bound text on each row.
The window is 960 by 720 points at scale 2, the pop-out window's own size.

## How it was taken

`bridge-native --prototype-shots <dir>` draws the sample headless and prints the times.
It is a function in `crates/bridge-native/src/prototype/shots.rs` and not a test, because gpui's Mac platform must be made on the main thread and the test harness runs tests on another.
The setup is the walk check's: `HeadlessAppContext::with_platform` with `gpui_platform::current_headless_renderer()`.

Each run does what a save does.
It hands `PrototypeView::set_source` a new `Source`, then draws one frame with `Window::draw`.
A comment appended to the markup makes each run a different save, because an identical source returns early, and it changes nothing drawn.

| Figure | What it spans |
|---|---|
| Read | `set_source` only: the markup parsed by html5ever, the fixtures decoded through `ipc::decode` and again as values for the bindings, and every part and token in the scene checked |
| Markup to a drawn frame | The read, then `run_until_parked`, then one `Window::draw`: the scene turned into elements, laid out, prepainted and painted into a scene |

Every figure is a wall-clock `Instant` around the span.
The window had already drawn the scene, so fonts, glyphs and the look are warm.
The first draw of a cold window is not in these figures.

## The numbers

Times are milliseconds, three processes of nine runs each, debug build.

| Figure | Min | Median | Max |
|---|---|---|---|
| Read | 1.3 | 3.0 | 5.6 |
| Markup to a drawn frame | 9.0 | 16.9 | 22.4 |

The remaining 14 ms of the median is the frame.
Most of it is a debug build's unoptimised layout, and a release build is expected to take a tenth of it by the feel spike's ratio, which is not measured here.

## What it says

A saved scene is on screen in about two frames at 60 Hz in the slowest build, and the sample is small.
The sample holds 4 list rows, so a scene with a few hundred rows is not covered, and `<list limit>` exists to keep one in bounds.
The agent's own wait is Fleet's part send and the artifact listing that carries it to Bridge, which this does not measure.
