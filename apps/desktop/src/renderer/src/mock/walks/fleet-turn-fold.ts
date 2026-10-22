// A turn Fleet started folds into one dim row: the notice, the agent's reply and its tool runs. A press opens the notice and the
// reply under it, the same press folds it again. A turn that ends in a question for the owner is not folded.
// Over the `fleet-turn-fold` scenario.

import { kit, toSessions } from "../sessions/walk-kit";
import { button, deckRow, inside, role, text, walk } from "../walk";

const { thread } = kit(false);
const folded = inside(thread, button(/Merge main notice/));
const ask = role("article", "Waiting on you");

const fleetTurnFold = walk("fleet-turn-fold", [
  toSessions,
  { press: deckRow("Move the store to the new clock"), say: "A Job Fleet told twice" },
  { look: folded, say: "Fleet's notice and the agent's reply, folded into one row" },
  { hover: folded, say: "The notice, as Fleet sent it" },
  { press: folded, say: "Press to open the notice and the reply" },
  { press: folded, say: "Press again to fold it" },
  { look: inside(ask, text("Rebase the branch or merge main into it?")), say: "A turn that asks you stays open, with its question" },
]);

export { fleetTurnFold as "fleet-turn-fold" };
