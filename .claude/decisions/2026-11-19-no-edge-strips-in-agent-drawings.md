# Should agents mark a card with a coloured stripe down one side?

**Said 2026-11-19.** Pointing at a card in an agent-drawn page with a coloured strip along its top: "I hate these little edge borders. Please nudge agents to not use that style."

## What he chose

| Asked | He chose | The cost he took |
|---|---|---|
| How far the ban reaches | Only what agents draw: pages, sketches and mocks | Bridge's own leading edges stay (fenced blocks, blockquotes, sketch node kinds), so the app and agents' drawings differ on this |

The ban covers a coloured stripe on one side of a box, card, callout, table cell or diagram node: `border-left: 3px solid`, `border-top: 3px solid`, `box-shadow: inset 4px 0 0`, or a strip down a node's side in SVG.
Use a tint fill of the hue, a full hairline outline in the hue, a coloured glyph or dot, a chip, or a header band across the whole top.

## Where it landed

- `.claude/skills/session-habits/SKILL.md`, the Cockpit bar bullet.
- `docs/contracts/design-system/the-product.md`, after the Cockpit bar table.
- `plugins/armada/skills/writing-a-document/SKILL.md`, under the kit variables.
- `crates/ipc/operations/show_sketch.toml`, the mock guidance.
- No house pattern in `.armada/catalogue/patterns.html` or `plugins/armada/` used a strip.
