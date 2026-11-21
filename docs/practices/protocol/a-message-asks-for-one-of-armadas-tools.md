# A message asks for one of Armada's tools: `TagKind::Tool`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 19 Nov 2026. The native Bridge's message box lists Armada's own tools under its + button and `ask_for_tool` (⌘/), and the one picked goes in at the caret as a chip.
**Additive only**: one `TagKind`; no operation, route, event kind or field.
There is no number to bump: the protocol ID moves with `crates/ipc/src` and `packages/protocol/src`.
Bridge's half is `packages/protocol/src/hosted-sessions.ts`, written by hand; Pocket's `apps/pocket/src/wire/hosted-sessions.ts` is generated.

| Where | Side | Carries |
| --- | --- | --- |
| `TagKind::Tool` (`tool`) on a `SessionTag` | Bridge sends, Fleet reads | `id` is `sketch`, `diagram`, `takes`, `document`, `browser`, `terminal` or `features`, and `title` the tool's name. Fleet adds one line for each after the line that names the rest, `The person asked for Takes: put the options to them with add_part of kind takes, one scene each, your pick first.` An `id` Fleet does not know adds no line; the person's words still say what they meant |

The thread keeps the tag on his row, so the native Bridge draws the chip again in its hue.
Classic and Pocket read the tag and draw no chip for it, so the row shows the words as typed.
