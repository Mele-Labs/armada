# Handing the title bar, dispatch and strip to Jobs screens

On 10 Nov 2026 the owner put all native code under one Job, Jobs screens.
This is what the Dispatch and Title Bar Job leaves it, on `native/dispatch` at `8613d93d9c` and this note's commit.
That commit holds all three of its branches: `native/title-bar` (`c06c0fdc75`) is an ancestor, and `native/strip-marks` is the same commit.
Nothing of it is on `main`, nothing has been approved, and no pull request was opened.

The two inventories beside this note are the checklist: [title bar, rings, build](title-bar-rings-build.md) and [dispatch, notes, strip, clashes](dispatch-merge-clash.md).
Section 6 of the second says where each piece mounts.

## What is only mine

These files are new and touch nothing of Jobs screens, so the merge takes them whole.

| Piece | Files |
|---|---|
| Title bar, picker, Places strip, usage rings, build switcher, swap hold, Fleet down | `titlebar/` (`mod.rs`, `face.rs`, `parts.rs`, `usage.rs`, `switcher.rs`, `swap_hold.rs`, `fleet_down.rs`, `scenes.rs`), `chrome.rs` |
| Dispatch sheet | `dispatch/` (`mod.rs`, `screen.rs`, `sheet.rs`, `view.rs`, `face.rs`, `scenes.rs`), `drafts.rs` |
| Annotation layer on ⌥⌘A | `annotate.rs`, `annotate/` (`panel.rs`, `store.rs`, `tools.rs`), `inspect.rs` |
| Capture, shared by the walk bar and the layer | `capture.rs`, moved out of `walk/capture.rs`, which is deleted |
| Merge queue strip | `merge_strip.rs`, `merge_strip/` (`fix.rs`, `words.rs`, `scenes.rs`) |
| Context ring and Compact | `context_ring.rs` |
| Write-clash mark | `clash_mark.rs`, `clash_mark/scenes.rs` |
| Walks | `walk/all.rs`, `walk/title_bar.rs`, `walk/dispatch.rs`, `walk/notes.rs`, `walk/strip_marks.rs`, `walk/stand_in.rs` |

## Shared files, and what the merge must keep

| File | What my change does | What the merge keeps |
|---|---|---|
| `jobs.rs` | In `render_job`, the empty Conflicts spacer becomes `ClashMark::in_row(row.id.clone())`. In `JobsScreen::render`, `title_row(look)` becomes `let title = crate::chrome::title_row(window, cx);`, the strip goes in as the last child, `.child(crate::merge_strip::strip(window, cx))`, and the root's `on_key_down` calls `keymap::traverse` so Tab reaches every tab stop | All four, beside Jobs screens' own changes. The title row's Job slot is Jobs screens' to fill, and `ContextRing` goes in the Job head it builds |
| `main.rs` | Declares `annotate`, `capture`, `clash_mark`, `context_ring`, `dispatch`, `drafts`, `inspect`, `merge_strip` and `titlebar`. `init_app` calls `titlebar::bind`, `merge_strip::bind`, `annotate::bind` and `dispatch::screen::bind` before `keymap::install` | Both sides' `mod` lines and both sides' `bind` calls, every bind before `keymap::install` |
| `keymap.rs` | Adds `traverse` (Tab and Shift+Tab to the next and previous tab stop), `cap`, `shown` and `shown_digit` (an act's key as its keycap reads, from the registry) | All four, beside Jobs screens' additions |
| `labels.rs` | Adds role words: dialog, alert, listbox, option, radiogroup, radio, tab, tablist, group, combobox, checkbox, tooltip | The union of both sides' words. Radio, checkbox, group and radiogroup are on both sides: keep each once |
| `dropdown.rs` | Adds `Entry::Search { label, to_rule }`, a search field heading a menu, and `Entry::Row(Row)`, a row with icon, detail, key, tick, member, accent, attention, gear and `trail` (the picker's place glyph, reached with →). Typing filters the rows while a Search heads the menu | The agreed shape below |
| `walk/mod.rs` | Adds the walks `title-bar`, `dispatch`, `notes`, `strip-marks` and `native-title-bar`, the verbs `Keys`, `RightPressOn`, `Scene`, `PressOn`, `Type`, `Drag`, `Answer`, `Marks`, `PausedScope` and `Fresh`, their `const fn` helpers, and the screens `Dispatch`, `Marks` and `All` | Both sides' walks, verbs and screens. `walk/capture.rs` stays deleted: capture lives in `src/capture.rs` |
| `walk/stage.rs` | Builds `Screen::Dispatch`, `Screen::Marks` and `Screen::All` around `JobsScreen::new`, and plays each new verb in `perform`. `Verb::Fresh` does nothing there, because the walk window rebuilds the screen first | All of it. If `JobsScreen::new` changes its arguments, the three builds follow |
| `walk/window.rs` | The note box and the region, pointer and press handling move into `crate::capture::Capture`, an entity that emits `Noted` and `Closed`. The walk window subscribes to it, and on a `Fresh` step builds the screen again before playing on | Mine, plus `.key_context(crate::keymap::TEXT_BOX)` on the capture's note box, so keys typed in a note are not taken as bindings. That lands in `capture.rs` |
| `walk/check.rs` | Activates the check's window so focus listeners hear, and draws three frames before each step so a step's callbacks run | Both lines |
| `Cargo.toml` and `Cargo.lock` | Adds `serde_json = "1"` for the notes kept as files | Both sides' dependencies |

### The agreed dropdown shape

Plain keyed acts use Jobs screens' variant: `Entry::Item { id, label, icon, danger, key, off }`.
`Row.disabled: bool` becomes `off: Option<SharedString>`, the reason a row cannot be picked, said in its tooltip.
`pickable()` skips `Search`, `Rule`, `Label`, `Off`, an `Item` with `off` set, and a `Row` with `off` set.
`Entry::Search` and `Entry::Row` stay as mine for the repository picker, the Fleet panel and the chip menus.

## Open bugs and gaps

None of these is fixed, and the owner marked none in a walk.

| What | Where | Why |
|---|---|---|
| ⌘K and the search field do nothing | `titlebar/mod.rs` emits `Asked::Palette` | Nothing takes it: the palette is on Jobs screens' branch. Map it to `Palette::open` |
| ⌘J and ⌘, do nothing | The same `Asked` | No live sink for Operator or Settings |
| Nothing is live | Every piece | Each draws mock scenes. The live Shell does not mount the dispatch screen, and no live `AskedSink` maps the `Asked` events. Fleet's views are on `main` (#3394, #3396): `RingBand`, `context_percent`, `compact`, `switcher`, `PullView` with `held_by` and `tone`, and `/clashes/all` |
| The context ring has no home | `context_ring.rs` | Only `walk/stand_in.rs` draws it. It goes in the Job head |
| The Places strip lists Jobs only | `titlebar/face.rs` | The other places come as their screens land |
| Controls are 32px tall where Electron's are 24px | Picker, Manifest button | My reading of the owner's 9 Nov ask for more room. He has not confirmed it |
| The ContextMenu key does nothing | Bar, places, strip chips | Said to be a gpui gap on macOS, not verified. Shift+F10 works |
| Ring ends are square | Usage and context rings | gpui draws no round caps here |
| Picker search takes typed letters only | `dropdown.rs` | No paste, no IME |
| The narrow-bar scene draws the 720px bar inside the full window | `titlebar/scenes.rs` | The walk does not resize its window |
| The dispatch drawer does not grow in | `dispatch/sheet.rs` | The animation was skipped |
| Hover cards open at once in a walk, and a pulse does not show in a step | Strip, rings, marks | A step is one frame |
| No letter-spacing, no 600 weight in Plex Mono | Group heads | Known gpui gaps, `docs/practices/native-bridge.md` |
| No live restyle for Tools in a note | `annotate/tools.rs` | A change is recorded, not drawn |
| Not checked by running | Everything | Parity was checked by reading Electron's code against the native code in two audits, then by `--check` and its pictures. Neither app was run beside the other, and no one pressed keys in a live native window |
| A focused element's press on Enter or Space | Clash marks, pins | Handled element by element (`e1c630f81c`). Not checked for gpui as a whole |

## The walk steps that cover each piece

`native-title-bar` plays 124 steps in four parts, each starting from a rebuilt screen (`walk/all.rs`).

| Steps | Piece |
|---|---|
| 1–7 | Places: ⌘1, the right-press menu (Hide, View), Shift+F10, focus going back |
| 8–9 | Usage meter on Tab, its card |
| 10–11 | Teach on a right press elsewhere on the bar |
| 12–13 | The repository picker and its place glyphs |
| 14–17 | Usage rings in each band, the step-down offer |
| 18–25 | The Fleet panel: Main, Preview, Refresh preview, a failed restart, Update to main and its ask |
| 26–30 | Keep, the swap hold, Swapped back |
| 31–35 | The narrow bar, signed out, the menu of what the row gave up |
| 36–40 | Events dropped, Fleet down, Restart Fleet |
| 41–59 | The dispatch sheet: ⌘N, the repository read from the words, Autopilot and Read it, refusals, the strip after sending, ⌥A tags, waits, no repository, More, Fleet unreachable |
| 60–93 | The annotation layer: ⌥⌘A, aiming at the bar, a held element and a box, Size and Changes, pictures, pins, reload, Tab and Enter on a pin, ⌥⌘S, sending to a Job, a refusal, Start job, All notes, Mark done, Delete |
| 94–108 | The merge strip: chips, hover card, right-press menu, q, Tab, main red, Send back, Take over, a taken red main, All |
| 109–115 | The context ring: its card, red, the popover, the tooltip, Compact |
| 116–124 | Write-clash marks: the pair, files and their paths, several clashes, the Paused scope |

The four walks it condenses, `title-bar`, `dispatch`, `notes` and `strip-marks`, still play on their own.
