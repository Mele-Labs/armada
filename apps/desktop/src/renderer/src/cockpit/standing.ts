// Which calls carry the two standing answers. **Only a question an agent asked and could have
// answered itself**: a Session's permission or question with options. A call Fleet raises about a
// state (a failing pull request, main red) keeps its own acts and nothing more (owner, 9 Oct 2026).
// **A Session's plain line is an errand for him, not a question** (owner, 11 Oct 2026): a command to
// run, a pull request to approve, a page to open, or a word to the agent, so it gets its own act and
// a text box. Pure, so the rule is checked without drawing a card.

import type { Item } from "../Dashboard";

export function asksAnAgent(item: Pick<Item, "waiting">): boolean {
  if (item.waiting === undefined) return false;
  const { source, options, act } = item.waiting.item;
  // A terminal's own permission prompt is answered at the terminal, so its one act opens the Session.
  if (source === "permission" && act?.kind === "message") return false;
  // A draft waits on his OK: leaving it to the agent would have the agent approve its own text. A sketch and a document are the same, and so is an act a rule would do.
  if (source === "draft" || source === "sketch" || source === "document" || source === "rule_act") return false;
  return source === "permission" || source === "ask_card" || source === "window_ask" || (options?.length ?? 0) > 0;
}
