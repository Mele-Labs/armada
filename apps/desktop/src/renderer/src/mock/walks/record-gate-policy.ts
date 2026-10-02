// The Record says why a step held, where the repository's policy decided it.
// #1683: Fleet records what both policies resolved to on a run that reached a
// gate. A run whose Checks failed says the rule never got to decide, and an
// older run that recorded nothing says nothing.

import { row, tab, text, walk } from "../walk";

export const recordGatePolicy = walk("job/reviewHeldByPolicy", [
  { press: tab("Record"), say: "The Job's Record, newest first" },
  {
    look: text("The repository said a person answers"),
    say: "The last run held because the repository said a person answers",
  },
  {
    look: text("but the run ended before that gate"),
    say: "The run before it failed its Checks, so the rule never decided it",
  },
  {
    look: row("regression_verify, running to awaiting_human"),
    say: "The first ran before Fleet recorded the policy, so its row says nothing",
  },
]);
