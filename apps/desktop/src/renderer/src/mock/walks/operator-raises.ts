// The Operator raising what needs him: the day's briefing in Discussion, then a line for each Needs you item with what it
// read and what it would pick, one it could not read, and one he already settled. Over `operator-raises`.

import { button, inside, role, text, walk } from "../walk";
import type { Target } from "../walk";

const panel: Target = role("region", "Operator", {
  exact: true,
});
const discussion = inside(
  panel,
  role("tabpanel", "Discussion", {
    exact: true,
  }),
);

export const operatorRaises = walk("operator-raises", [
  {
    key: "Meta+j",
    on: button("Menu", {
      exact: true,
    }),
    say: "⌘J opens the Operator. Bridge came to the front for the first time today, so it wrote you a briefing",
  },
  {
    look: inside(discussion, text(/^Good morning/)),
    say: "The briefing: what moved since last night, and what waits on you",
  },
  {
    look: inside(discussion, text(/^Quick dispatch needs you/)),
    say: "Each thing that needs you gets its own line. It read the Job first: what it is, what it makes of it, and what it would pick",
  },
  {
    look: inside(discussion, text(/^Rework the Map legend needs you/)),
    say: "A walk gets the same: what the walk shows you, and its pick",
  },
  {
    look: inside(discussion, text(/^Store reports its size needs you/)),
    say: "When the read fails, or the week's plan has it paused, the item is still raised, just with no read",
  },
  {
    look: inside(discussion, text(/^Time each build step needs you/)),
    say: "Something it raised earlier that you have since answered says it's settled. The native Bridge will draw each of these as a card with its acts",
  },
]);
