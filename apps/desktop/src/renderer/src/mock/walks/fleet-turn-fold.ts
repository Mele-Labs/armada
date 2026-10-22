// A turn Fleet started folds into one dim row: the notice, the agent's reply and its tool runs. A press opens the notice and the
// reply under it, the same press folds it again. The turn's last agent message, and the owner's reply, draw outside the fold.
// A turn that ends in a question for the owner is not folded.
// Over the `fleet-turn-fold` scenario.

import { kit, toSessions } from "../sessions/walk-kit";
import { button, deckRow, inside, region, role, text, walk } from "../walk";

const { thread } = kit(false);
const folded = inside(thread, button(/commits behind main/));
const needsYou = role("button", "Needs you");

const fleetTurnFold = walk("fleet-turn-fold", [
  toSessions,
  { press: deckRow("Move the store to the new clock"), say: "A Job Fleet told twice" },
  { press: needsYou, say: "Fold the Needs you panel so the thread shows" },
  { look: folded, say: "Fleet's notice and the agent's work on it, folded into one row" },
  { hover: folded, say: "The notice, as Fleet sent it" },
  { press: folded, say: "Press to open the notice, the tool runs and the agent's words on the way" },
  { press: folded, say: "Press again to fold it" },
  { look: inside(thread, text(/I'll merge main right before the next push/)), say: "The agent's last words in the turn draw outside the fold" },
  { look: inside(thread, text("Merge it now, the push is close.")), say: "Your reply after a notice draws in the thread, never in the fold" },
  { look: inside(thread, button(/#2734 merged into main/)), say: "The first sentence names each notice, and a check marks it handled" },
  { look: inside(thread, text(/The only open item is in Needs you/)), say: "Every turn's closing words draw outside the fold" },
  { look: inside(thread, region("Message from Fleet")), say: "A turn that asks you stays open, notice and reply in full" },
]);

export { fleetTurnFold as "fleet-turn-fold" };
