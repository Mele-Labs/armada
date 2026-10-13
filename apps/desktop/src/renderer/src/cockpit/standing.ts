// Which calls carry the two standing answers. **Only a question an agent asked and could have
// answered itself**: a Plan question, a Drone's or a Judge's, and a Session's permission or question
// with options. A call Fleet raises about a state (a failing pull request, main red, a failed Check,
// a stuck Drone) keeps its own acts and nothing more (owner, 9 Oct 2026). **A Session's plain line is
// an errand for him, not a question** (owner, 11 Oct 2026): a command to run, a pull request to
// approve, a page to open, or a word to the agent, so it gets its own act and a text box. **A
// Commander's card is neither**: it asks what is out of its reach, so there is no deciding left to
// hand back to it. Pure, so the rule is checked without drawing a card.

import type { Item } from "../Dashboard";

export function asksAnAgent(item: Pick<Item, "decisions" | "kind" | "key" | "waiting">): boolean {
  if (item.kind === "Commander asks") return false;
  if (item.waiting !== undefined) {
    const { source, options } = item.waiting.item;
    return source === "permission" || source === "ask_card" || source === "window_ask" || (options?.length ?? 0) > 0;
  }
  return item.decisions !== undefined || item.kind === "Drone question" || item.kind === "Judge question";
}
