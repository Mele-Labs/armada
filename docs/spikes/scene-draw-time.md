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

## Shot check tolerance

The shot check (`docs/concepts/scenes.md`, *Shot check before an offer*) sets a scene's picture of a state against the built screen's, and needs a threshold for "the same".
This measures it on the sample scene, drawn at 960 by 720 points at scale 2, in a debug build.

Each picture is averaged to a grid 32 cells across, 60 pixels square here.
A cell differs when its mean colour is over 0.02 from the other picture's, as a share of full scale.
A pair has drifted when the mean difference over the grid passes 0.003, or when more than 0.005 of the cells differ.

The variants were made by editing the sample's markup and set against it with `bridge-native --shot-check <sample> --against <variant>`.

| Variant of the sample | State | Mean | Cells over | Result |
|---|---|---|---|---|
| The sample against itself | list, detail | 0 | 0 | matched |
| The four chips' tone, running to danger | list | 0.00095 | 0.0156 | drifted |
| The card's tone, accent to warning | detail | 0.00443 | 0.0833 | drifted |
| The two buttons swapped | detail | 0.00330 | 0.0260 | drifted |
| The list padded by one space step | list | 0.00702 | 0.0951 | drifted |
| The Open button removed | detail | 0.00049 | 0.0104 | drifted |
| The list's limit from 4 rows to 3 | list | 0.00126 | 0.0104 | drifted |

A spacer in the markup moves the list about 12 points, not one pixel, so the small shifts were made on the sample's own picture, in a script that moves it by whole pixels.
The grid and thresholds are the same as above.

| Shift in pixels, across and down | List mean | List cells over | Detail mean | Detail cells over |
|---|---|---|---|---|
| 1, 0 | 0.00012 | 0 | 0.00009 | 0 |
| 1, 1 | 0.00031 | 0 | 0.00043 | 0 |
| 2, 2 | 0.00062 | 0.0013 | 0.00085 | 0 |
| 4, 4 | 0.00125 | 0.0169 | 0.00232 | 0.0221 |

The threshold is 0.003 on the mean and 0.005 on the cells over.
A shift of up to two pixels each way matches, and one of four drifts, on the share of cells.
The mean alone cannot tell a missing button (0.00049) from a two pixel shift (0.00085), so the share carries it.
The coarser grids tried before, 64 and 128 cells across, scored a one pixel shift higher and a missing button lower, so they separate the two less.
At 64 cells and a cell threshold of 0.04, the first tried, a changed chip tone, a missing button and a missing row all passed.

A walk's picture is never pixel-equal to a scene's, since the scene is a sketch of the screen.
These thresholds were set on pictures drawn by the same code, so they say what a drift of a part looks like and not how close a real build gets.
Run one real pair, and read its difference picture, before trusting a threshold on it.
A scene drawn against a real walk, with the walk's step ids added by hand, scored 0.016 and 0.024 on the mean, which is the distance between the sample and the Jobs screen, and drifted in both.
