// Answering a held Drone from the card the plan graph hangs beside its task:
// the card and its edge go once the answer is sent. `held-drone-asks.test.tsx`
// holds the claims.

import { button, card, inside, role, tab, text, walk } from "../walk";

const ASK = role("group", "Needs you");

export const allowingAHeldCommandFromTheCanvas = walk("arc/executing-held", [
  { press: tab("Plan"), say: "The plan, as its groups and tasks" },
  { look: inside(card("Draw what is running, in four lists"), text("Needs you")), say: "T5's card says a Drone under it needs you" },
  { look: ASK, say: "A card beside it asks, joined to T5 by an edge" },
  { press: inside(ASK, role("radio", "Allow for this job")), say: "Allow it for this Job" },
  { press: inside(ASK, button("Send this answer")), say: "Sent from the canvas, to the same call Overview sends" },
  { look: card("Draw what is running, in four lists"), say: "The card and its edge are gone" },
]);
