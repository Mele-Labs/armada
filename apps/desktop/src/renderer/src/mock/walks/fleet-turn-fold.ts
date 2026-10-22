// A turn Fleet started folds into one dim row: the notice, the agent's reply and its tool runs. A press opens the notice and the
// reply under it, the same press folds it again. A turn that ends in a question for the owner is not folded.
// Over the `fleet-turn-fold` scenario.

import { kit, toSessions } from "../sessions/walk-kit";
import { button, deckRow, inside, region, role, text, walk } from "../walk";

const { thread } = kit(false);
const folded = inside(thread, button(/Merge main notice/));
const needsYou = role("button", "Needs you");

const fleetTurnFold = walk("fleet-turn-fold", [
  toSessions,
  { press: deckRow("Move the store to the new clock"), say: "A Job Fleet told twice" },
  { press: needsYou, say: "Fold the Needs you panel so the thread shows" },
  { look: folded, say: "Fleet's notice and the agent's reply, folded into one row" },
  { hover: folded, say: "The notice, as Fleet sent it" },
  { press: folded, say: "Press to open the notice and the reply" },
  { press: folded, say: "Press again to fold it" },
  { look: inside(thread, region("Message from Fleet")), say: "A turn that asks you stays open, notice and reply in full" },
]);

export { fleetTurnFold as "fleet-turn-fold" };
