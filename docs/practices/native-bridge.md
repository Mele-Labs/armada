# Native Bridge practices

How a Job works on `crates/bridge-native/`, the native Bridge.
What it is and why it exists is in [Native Bridge](../concepts/native-bridge.md).

## Who does the work

**One Job owns `crates/bridge-native/` at a time.**
Other Jobs in the rebuild do Fleet views and inventories, and never edit the crate.
Decided 10 Nov 2026: two Jobs built halves that never met in one build, so walks showed one half with stand-ins for the other.

**Dispatch `gpui-engineer` for any work under `crates/bridge-native/`.**
Its brief is `.claude/agents/gpui-engineer.md`.

## Read the pinned source

**Read the pinned gpui and gpui-component source before using an API.**
Memory of gpui is older than the pin.

## The wire

| Rule | Reason |
|---|---|
| Link `crates/ipc` directly | The protocol ID covers `ipc`. A copied type drifts silently |
| Get the ID from `ProtocolId::current()` | One build speaks one protocol |
| Send no `Origin` header to Fleet | Fleet refuses a request with one, as it came from a page |

Never copy a wire type.
The seam is described in `protocol.md`.

## Logic

**A screen draws a Fleet view.**
Missing logic is a Fleet change, so Pocket gets it too.

## The design loop

**A native screen is a port of its Electron screen, never a redesign.**
It keeps the layout, controls, states and keys it has today.
Every feature goes through the same five steps, in order.

1. An inventory of the Electron screens: their regions, sizes, spacing and type tokens, controls, states and keys, each with `path:line`.
2. The gpui build, matching the inventory.
3. A parity check against running Electron, by the Job and then by a checker that did not build it.
4. The branch offered with `offer_preview`, which he tries in his preview after Refresh preview.
   He takes notes on it there with ⌥⌘A, or approves, and changes go into the native screens only.
5. Land.

**The parity check passes before any walk is shown.**
It compares pictures, not code: Electron's renderer on the mock Fleet in Playwright, at the walk window's size and on the same scenarios, beside each step of `--shots`.
The Job looks at every picture itself, and a subagent's own table never signs off.
Then the orchestrating Job runs a checker agent that did not build the feature, and looks at the pictures before the walk is shown.
At least these match Electron:

| Area | Matches |
|---|---|
| Keys | Every shortcut, working from the same places, with focus returned after a popup or menu closes |
| Right-click | Every element's menu, with the same items in the same order |
| Layout | Tabs, panels, resizing and focus behave the same |

The offer's `about` carries the result as a short table, with any known gpui gap.

**One offer per feature, never one per piece.**
Decided 9 Nov 2026: partial walks made the owner do the comparing, and the rebuild slowed.

**Nothing he sees lands before he has tried it in his preview.**
Decided 13 Nov 2026: a walk from a Job's slot build took him four presses and a refusal to open.

## Native walks

A native walk is a screen on mock data in its own window with a walk bar.
Its steps and scenario live in `crates/bridge-native/src/walk/`, and each step aims at a `{role, name}` that `labels::labelled` put on.

**A walk is shown to him only for a state a live Fleet cannot reach**, such as Fleet down or a refused send.
Everything else he sees in his preview.
A Job still plays its walks itself, as its own check of a screen.

| Step | What to do |
|---|---|
| Build | `cargo build` in `crates/bridge-native`, in the Job's own slot, for the Job's own check only |
| Play it first | `bridge-native --walk <name> --check`, which exits 1 naming the first step it could not play. `--shots <dir>` writes each step as a picture, which is how a Job looks at a native screen |
| Reach his build | `offer_preview` the branch; the walk can be shown once he has refreshed onto a preview holding it |
| Show it | `show_window` on `armada://walk/<name>`, with a `title` and a `brief` and no `steps`. Alternatives are `variations`, each another native walk's address, the first the `url` itself. Fleet refuses a walk the build he runs does not play (`--list-walks`), so the Job sees it and he never does |
| He opens it | One press on the walk's row. Fleet starts the native Bridge of the build he runs, `Armada/builds/current/Armada.app`, with `--take <name>=<walk>` for each variation |
| He picks a take | The bar's takes menu, or ⌥⌘T, swaps to that take's walk from its first step. Approve and each note name the take on screen |

**The check never opens a window, and Fleet never starts a walk he did not open.**

**A walk talks to Fleet on three routes only**: the message that carries Approve, the note, and the window closed.
It skips the protocol check, because a Job's branch may change the wire and its own walk must still reach his Fleet.
Fleet pins those three request shapes in `crates/fleet/src/tests/native_walk.rs`, so a change to one fails there.

## Labels

**Every element he can press, read or focus carries a role and a name.**
VoiceOver reads them, walks and notes aim at them, and tests find elements by them.

## Web content

**Web content goes through the one Chrome host and the Armada profile.**
That covers documents, sketch mocks, walks of web pages and the Browser tool.

## Load

| Rule | Detail |
|---|---|
| A native build | Runs like any other Rust build Job: a cold one is about a minute and 2 GB, measured in [the feel spike](../spikes/native-bridge-feel.md) |
| A build with Chrome in it | Measured heavier in [the Chrome spike](../spikes/native-bridge-chrome.md): look again before two run at once |
| Tests | Only through `armada check crates/bridge-native:test <name>` or `armada check crates/bridge-native:test --changed`: the crate is a workspace of its own, so the root's `armada check test` does not reach it. `bridge-native --walk <name> --check` stays the walk's own test. CI runs the whole suite in the `native` job on any pull request whose change touches the crate, and `ci` waits on it |
| Formatting | Never `rustfmt` or `cargo fmt` |

## What gpui cannot match

**A screen is drawn as Electron draws it, and these are the known gaps.**
Say the ones a screen shows in the brief of its walk.
A native walk is shown alone, never with the Electron screen in a second tab: he opens Electron himself to compare (9 Nov 2026).

| Gap | Where it shows | Why |
|---|---|---|
| No letter-spacing | The Jobs list's group heads, set at 0.18em in Electron | gpui's text style has no tracking |
| No weight 600 in Plex Mono | The same heads, which ask for 600 | `packages/tokens/fonts` ships Mono at 400 and 500, and a browser fakes the bold where gpui does not |

## The overlap rule

**An Electron feature that lands gets a task in the matching step of the build order in [Native Bridge](../concepts/native-bridge.md).**
Electron takes only fixes and small features that help the owner daily.

## Targets

**A long thread opens, and a Job switch lands, in under 50 ms.**
The figures tune the build and gate nothing.
