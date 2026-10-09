// Which calls carry the two standing answers. **Only a question an agent asked and could have
// answered itself**: a Plan question, a Drone's or a Judge's, a Session's ask. A call Fleet raises
// about a state (a failing pull request, main red, a failed Check, a stuck Drone) keeps its own acts
// and nothing more (owner, 9 Oct 2026). Pure, so the rule is checked without drawing a card.

import type { Item } from "../Dashboard";

export function asksAnAgent(item: Pick<Item, "decisions" | "kind" | "key">): boolean {
  return item.decisions !== undefined || item.kind === "Drone question" || item.kind === "Judge question" || item.key.startsWith("session:");
}
