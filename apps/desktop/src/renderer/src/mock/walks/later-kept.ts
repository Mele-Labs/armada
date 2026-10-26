// A Job that said it was done with follow-ups open: Fleet's refusal sits in its thread, and the follow-up left is under Deferred in Delivery with
// a Remind and a Drop. Over the `later-kept` scenario.

import { button, deckRow, inside, region, text, walk } from "../walk";

const landing = region("Delivery", { exact: true });
const later = inside(landing, button("Deferred"));
const remind = inside(landing, button(/^Remind/));
const drop = inside(landing, button(/^Drop/));

const laterKept = walk("later-kept", [
  { press: deckRow("Read the Studio registry on start"), say: "The Job said it was done" },
  { press: inside(region("Fleet turn"), button(/Fleet notice/)), say: "Fleet's notice is folded in the thread. A press opens it" },
  { look: text("all_done refused. Still open under Later:"), say: "Fleet refused. Two items were open" },
  { press: inside(landing, button(/^Delivery/)), say: "Delivery is folded to its band while the Job is still being built, and a press opens it" },
  { press: later, say: "One is settled. One is left" },
  { hover: remind, say: "Remind" },
  { press: remind, say: "Remind sends it to the Job" },
  { look: text(/You said: .The Studio icon tooltip is still owed\./), say: "It arrives in the thread" },
  { press: drop, say: "Drop takes it off the list" },
  { look: landing, say: "Deferred is gone" },
]);

export { laterKept as "later-kept" };
