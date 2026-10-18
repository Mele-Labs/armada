// A judge's concern as a claim to read: the consequence as its headline, then what was expected and what was found, each clamped to two
// lines with More for the rest. Over the `judge-concern-card` scenario.

import { button, deckRow, inside, region, role, walk } from "../walk";

const card = role("group", /^Judge: /);

export const judgeConcernCard = walk("judge-concern-card", [
  { press: deckRow("Name the store migrations"), say: "The Job is open, with a judge's concern waiting on the Needs you band" },
  { press: inside(region("Needs you"), button("Needs you", { exact: true })), say: "The band opens to a list, a row for the concern" },
  { press: inside(region("Needs you"), button(/^Open /)), say: "A press opens it" },
  { look: card, say: "The consequence is the headline. Under it, what was expected and what was found, each on a labelled row" },
  { press: inside(card, button(/^More of expected/)), say: "More opens the rest of a row" },
  { look: inside(card, button(/^Less of expected/)), say: "The whole sentence, and Less puts it back to two lines" },
]);
