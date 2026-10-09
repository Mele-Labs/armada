// Sleep mode: the moon beside Helm goes on, a question is answered for the owner while he sleeps, and
// waking opens the Morning review — what was decided, what still needs him, what landed, the walk
// held — with one answer overridden. Over the `session-question` scenario; the night itself is the
// mock's (`mock/sleep.ts`).

import { button, inside, region, role, text, walk } from "../walk";

const moon = button("Sleep mode");
const review = role("dialog", "Morning review");
const decided = inside(review, region("Decided for you"));

const sleepMode = walk("session-question", [
  { look: moon, say: "Sleep mode, beside Helm" },
  { press: moon, say: "On: the work carries on until it is blocked on the owner" },
  { look: button("Morning review"), say: "A question was answered for him; the review is there to read" },
  { press: moon, say: "Off, and the review opens" },
  { look: decided, say: "Decided for you" },
  { look: inside(decided, text("Large")), say: "What the agent chose" },
  { press: inside(decided, button("Override")), say: "Override" },
  { type: "Medium", into: inside(decided, role("textbox", "Correction")), say: "The correction" },
  { press: inside(decided, button("Send")), say: "Sent to the agent" },
  { look: inside(decided, text("Medium")), say: "The row shows what was sent" },
  { look: inside(review, region("Still needs you")), say: "Still needs you" },
  { look: inside(review, region("Landed overnight")), say: "Landed overnight" },
  { look: inside(review, region("Walks waiting")), say: "Walks waiting" },
]);

export { sleepMode };
