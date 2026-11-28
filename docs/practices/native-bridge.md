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

**A native screen is a port of its Classic screen, never a redesign.**
It keeps the layout, controls, states and keys Classic had, read at the tag `classic-final`.
A screen the owner has asked to come back with a new layout is not a port: Mods, Doctor, Cleanup and Lessons (armada-gmue, armada-c7ra, armada-ua7n, armada-sgrk).
Every feature goes through the same five steps, in order.

1. An inventory of the Classic screens: their regions, sizes, spacing and type tokens, controls, states and keys, each with `path:line`.
2. The gpui build, matching the inventory.
3. A parity check against the inventory, by the Job and then by a checker that did not build it.
4. The screen shown as a native walk; on his Approve, the branch offered with `offer_preview`, which he tries in his preview after Refresh preview.
   He takes notes on it there with ⌥⌘A, or approves, and changes go into the native screens only.
5. Land.

**The parity check passes before any walk is shown.**
It compares pictures, not code: each step of `--shots` against what the inventory says Classic drew.
The Job looks at every picture itself, and a subagent's own table never signs off.
Then the orchestrating Job runs a checker agent that did not build the feature, and looks at the pictures before the walk is shown.
At least these match Classic:

| Area | Matches |
|---|---|
| Keys | Every shortcut, working from the same places, with focus returned after a popup or menu closes |
| Right-click | Every element's menu, with the same items in the same order |
| Layout | Tabs, panels, resizing and focus behave the same |

The offer's `about` carries the result as a short table, with any known gpui gap.

**One offer per feature, never one per piece.**
Decided 9 Nov 2026: partial walks made the owner do the comparing, and the rebuild slowed.

**Every screen he approves is the real native Bridge, built by Fleet from the Job's own branch on its own mock data.**
Decided 27 Nov 2026: approving a prototype meant trusting an agent to turn it into the app's look, and he wants to approve the exact look.

It is a native walk shown with `show_window` on `armada://walk/<name>`, as *Native walks* below says.
It opens in one press, and a branch that moves rebuilds by itself.
The walk bar has two answers:

| Answer | His words | What follows |
|---|---|---|
| **Ship it** | "Approve the walk and land the change without previewing in the app" | Closes the walk's gate and the first open `you_approved` gate, so the Job lands it |
| **Approve** | "Approve the walk and preview it in the app before landing it" | The Job calls `offer_preview`, and **Skip** on the Ready to try card lands it without trying |

He chooses by the change's size: a small change he ships from the walk, and a larger one he walks and then tries in the preview.

Fleet keeps pictures of the look he approved, from the build he approved.
Before the pull request opens and on every push it plays the walk again and compares.
A drift goes back to the Job with side-by-side pictures, and never to him.

A prototype keeps its Approve: he chose on 28 Nov 2026 to keep prototypes approvable.
They are never the look gate of a screen he would see.
A walk still building reaches him at once, marked Building, with Open greyed until it is built (28 Nov 2026).
This holds in every repo.

## Native walks

A native walk is a screen on mock data in its own window with a walk bar.
Its steps and scenario live in `crates/bridge-native/src/walk/`, and each step aims at a `{role, name}` that `labels::labelled` put on.

A Job plays its walks itself first, as its own check of a screen.

| Step | What to do |
|---|---|
| Build | `cargo build` in `crates/bridge-native`, in the Job's own slot, for the Job's own check only |
| Play it first | `bridge-native --walk <name> --check`, which exits 1 naming the first step it could not play. `--shots <dir>` writes each step as a picture, which is how a Job looks at a native screen |
| Commit it | The screen and its walk file, in the Job's slot: Fleet builds the walk from the commit, so only committed work is shown |
| Show it | `show_window` on `armada://walk/<name>`, with a `title` and a `brief` and no `steps`. Alternatives are `variations`, each another native walk's address, the first the `url` itself. The row says Building while Fleet builds it from the Job's commit and asks him nothing until the build plays every walk named (`--list-walks`). A failure is told to the Job and never to him |
| He opens it | One press on the walk's row. Fleet starts the native Bridge it built from the Job's commit, with `--take <name>=<walk>` for each variation |
| He picks a take | The bar's takes menu, or ⌥⌘T, swaps to that take's walk from its first step. Approve and each note name the take on screen |

**The check never opens a window, and Fleet never starts a walk he did not open.**

**A walk talks to Fleet on three routes only**: the message that carries Approve, the note, and the window closed.
It skips the protocol check, because a Job's branch may change the wire and its own walk must still reach his Fleet.
Fleet pins those three request shapes in `crates/fleet/src/tests/native_walk.rs`, so a change to one fails there.

## Labels

**Every element he can press, read or focus carries a role and a name.**
VoiceOver reads them, walks and notes aim at them, and tests find elements by them.

## Web content

**Web content goes through the one web view host and Armada's own sign-in store.**
That covers documents, sketch mocks, walks of web pages and the Browser tool.
The host is Apple's WKWebView, laid over its pane's frame, with the page's picture drawn in its place while a sheet covers it or a walk plays with no window.

## Load

| Rule | Detail |
|---|---|
| A native build | Runs like any other Rust build Job: a cold one is about a minute and 2 GB, measured in [the feel spike](../spikes/native-bridge-feel.md) |
| Tests | Only through `armada check crates/bridge-native:test <name>` or `armada check crates/bridge-native:test --changed`: the crate is a workspace of its own, so the root's `armada check test` does not reach it. `bridge-native --walk <name> --check` stays the walk's own test. CI runs the whole suite in the `native` job on any pull request whose change touches the crate, and `ci` waits on it |
| Formatting | Never `rustfmt` or `cargo fmt` |

## What gpui cannot match

**A screen is drawn as Classic drew it, and these are the known gaps.**
Say the ones a screen shows in the brief of its walk.
A native walk is shown alone, never beside a picture of the Classic screen (9 Nov 2026).

| Gap | Where it shows | Why |
|---|---|---|
| No letter-spacing | The Jobs list's group heads, set at 0.18em in Classic | gpui's text style has no tracking |
| No weight 600 in Plex Mono | The same heads, which ask for 600 | `packages/tokens/fonts` ships Mono at 400 and 500, and a browser fakes the bold where gpui does not |

## Targets

**A long thread opens, and a Job switch lands, in under 50 ms.**
The figures tune the build and gate nothing.
