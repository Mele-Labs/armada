// Which calls carry the two standing answers. **Only a question an agent asked and could have
// answered itself**: a Plan question, a Drone's or a Judge's, a Session's ask. A call Fleet raises
// about a state (a failing pull request, main red, a failed Check, a stuck Drone) keeps its own acts
// and nothing more (owner, 9 Oct 2026). Pure, so the rule is checked without drawing a card.

import type { Item } from "../Dashboard";

// **A command a Drone is held on is not one**: it is answered by one of its offers, and best and quick
// are not among them.
export function asksAnAgent(item: Pick<Item, "decisions" | "kind" | "key" | "waiting" | "asked">): boolean {
  if (item.asked?.kind === "command") return false;
  return item.decisions !== undefined || item.kind === "Drone question" || item.kind === "Judge question" || (item.waiting !== undefined && item.waiting.item.source !== "walk");
}
