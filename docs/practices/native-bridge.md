# Native Bridge practices

How a Job works on `crates/bridge-native/`, the native Bridge.
What it is and why it exists is in [Native Bridge](../concepts/native-bridge.md).

## Who does the work

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
Every screen goes through the same four steps, in order.

1. An inventory of the Electron screen: its regions, sizes, spacing and type tokens, controls, states and keys, each with `path:line`.
2. The gpui build, matching the inventory.
3. A native walk on mock data, in the screen's own window with a walk bar, beside the Electron screen.
   The owner names his spacing and font tweaks there, and they go into the native screen only.
4. Land.

**Nothing he sees lands before his walk.**

## Native walks

A native walk is a screen on mock data in its own window with a walk bar.
Its steps and scenario live in `crates/bridge-native/src/walk/`, and each step aims at a `{role, name}` that `labels::labelled` put on.

| Step | What to do |
|---|---|
| Build | `cargo build` in `crates/bridge-native`, in the Job's own slot |
| Play it first | `bridge-native --walk <name> --check`, which exits 1 naming the first step it could not play. `--shots <dir>` writes each step as a picture, which is how a Job looks at a native screen |
| Show it | `show_window` on `armada://walk/<name>`, with a `title` and a `brief` and no `steps`. Alternatives are `variations`, each another native walk's address, the first the `url` itself |
| He opens it | He presses the walk's row or Open on the Job's tab. Fleet starts the slot's own build with `--take <name>=<walk>` for each variation, and refuses a build older than the slot's HEAD |
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
