// A judge's concern as a claim to read: the consequence as its headline, then what was expected and what was found, each clamped to two
// lines with More for the rest. Over the `judge-concern-card` scenario.

import { button, deckRow, inside, role, walk } from "../walk";

const card = role("group", /^Judge: /);

export const judgeConcernCard = walk("judge-concern-card", [
  { press: deckRow("Name the store migrations"), say: "The Session is open, with a judge's concern pinned under its thread" },
  { press: inside(role("region", "Pinned calls"), button("Judge")), say: "The pinned region is short on a thread this size, so a press on its band opens it over the thread" },
  { look: card, say: "The consequence is the headline. Under it, what was expected and what was found, each on a labelled row" },
  { press: inside(card, button(/^More of expected/)), say: "More opens the rest of a row" },
  { look: inside(card, button(/^Less of expected/)), say: "The whole sentence, and Less puts it back to two lines" },
]);
