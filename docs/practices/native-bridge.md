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

Every screen goes through the same five steps, in order.

1. An HTML mock with the real tokens.
2. The owner walks the mock.
3. The gpui build.
4. A native walk on mock data, in the screen's own window with a walk bar.
5. Land.

**Nothing he sees lands before his walk.**

## Native walks

A native walk is a screen on mock data in its own window with a walk bar.
Its steps and scenario live in `crates/bridge-native/src/walk/`, and each step aims at a `{role, name}` that `labels::labelled` put on.

| Step | What to do |
|---|---|
| Build | `cargo build` in `crates/bridge-native`, in the Job's own slot |
| Play it first | `bridge-native --walk <name> --check`, which exits 1 naming the first step it could not play. `--shots <dir>` writes each step as a picture, which is how a Job looks at a native screen |
| Show it | `show_window` on `armada://walk/<name>`, with a `title` and a `brief` and no `steps` |
| He opens it | He presses Open on the Job's tab. Fleet starts the slot's own build, and refuses a build older than the slot's HEAD |

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
| One native build at a time | gpui and Chrome are large builds on a Mac that also runs Fleet |
| A heavy cold build | If the foundation measures one, only one native build Job runs at a time |
| The second slot then | Takes Fleet or docs work |
| Tests | Only through `armada check test <name>` or `--changed` |
| Formatting | Never `rustfmt` or `cargo fmt` |

## The overlap rule

**An Electron feature that lands gets a task in the matching step of the build order in [Native Bridge](../concepts/native-bridge.md).**
Electron takes only fixes and small features that help the owner daily.

## Targets

**A long thread opens, and a Job switch lands, in under 50 ms.**
The figures tune the build and gate nothing.

## Open questions

- **[native-cold-build-weight]** Is a cold native build heavy enough to limit native build Jobs to one at a time?
  The foundation measures it. Until then the one-build-at-a-time rule above holds.
