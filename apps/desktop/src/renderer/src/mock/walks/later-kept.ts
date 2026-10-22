// A Job that said it was done with follow-ups open: Fleet's refusal sits in its thread, and the follow-up left is under Later in Progress with
// a Remind and a Drop. Over the `later-kept` scenario.

import { button, deckRow, inside, region, text, walk } from "../walk";

const progress = region("Progress");
const later = inside(progress, button("Later"));
const remind = inside(progress, button(/^Remind/));
const drop = inside(progress, button(/^Drop/));

const laterKept = walk("later-kept", [
  { press: deckRow("Read the Studio registry on start"), say: "The Job said it was done" },
  { look: text("all_done refused. Still open under Later:"), say: "Fleet refused. Two items were open" },
  { press: later, say: "One is settled. One is left" },
  { hover: remind, say: "Remind" },
  { press: remind, say: "Remind sends it to the Job" },
  { look: text(/You said: .The Studio icon tooltip is still owed\./), say: "It arrives in the thread" },
  { press: drop, say: "Drop takes it off the list" },
  { look: progress, say: "Later is gone" },
]);

export { laterKept as "later-kept" };
