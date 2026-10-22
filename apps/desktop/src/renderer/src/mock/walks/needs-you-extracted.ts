// A reply's close, read sentence by sentence: each ask is its own Needs you item, the approval the agent also stated is shown once, and the
// follow-ups it kept for itself are listed in Progress, folded under Later. Over the `needs-you-extracted` scenario.

import { button, deckRow, inside, region, role, walk } from "../walk";

const needs = region("Needs you");
const progress = region("Progress");
const later = inside(progress, button("Later"));

const needsYouExtracted = walk("needs-you-extracted", [
  { press: deckRow("Read the Studio registry on start"), say: "Its last reply closes with three asks and two follow-ups" },
  { press: inside(needs, button(/^Needs you/)), say: "Three items, one per ask" },
  { press: inside(needs, button("Open Should the registry be read once at start, or on every Studio open?")), say: "A question of the shape X, or Y? opens with its two choices, and a reply box under them" },
  { key: "Escape", on: inside(needs, button("Back", { exact: true })), say: "Esc returns to the three items" },
  { look: inside(needs, role("group", "pnpm vitest run packages/studios")), say: "The command is drawn whole, with its Run key" },
  { look: inside(needs, button("Open Approve #2714 and it lands.")), say: "The agent stated this approval itself. The reply's copy is folded into it" },
  { look: later, say: "The follow-ups the agent kept are under Later, folded" },
  { press: later, say: "A press opens it" },
  { look: inside(progress, role("list")), say: "Two rows, as the reply wrote them" },
]);

export { needsYouExtracted as "needs-you-extracted" };
