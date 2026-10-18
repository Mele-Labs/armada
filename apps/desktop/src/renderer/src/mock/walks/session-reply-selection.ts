// Replying to words in an earlier message of a live Session. Selecting them raises the small bar;
// Reply puts the words in the Job's own box as a quote, behind anything already typed, and the box
// takes focus with the caret under them. What is typed there goes to the Job as one message.
// Over the scenario `session-fork-selection`, which holds the same Session.

import { kit, toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, text, walk } from "../walk";

const SAID = "When the second rolls over between the two reads, the values differ by one second.";

const { message, thread } = kit(false);
const sessions = region("Jobs", { exact: true });

const wide = walk("session-fork-selection", [
  toSessions,
  { press: inside(sessions, button("Fix the flaky store test")), say: "A live Job opens" },
  { select: SAID, in: inside(thread, text("The test builds its expected timestamp")), say: "Words in an earlier message, selected" },
  { look: role("toolbar", "Selection"), say: "A bar sits just over the selection" },
  { hover: button("Reply"), say: "Reply is named on hover" },
  { press: button("Reply"), say: "Reply quotes the words" },
  { look: message, say: "They wait in the Job's own box, caret below" },
  { type: "Pin the clock in the store, and check sweep.rs too.\n", into: message, say: "A reply typed under the quote" },
  { look: inside(thread, text("and check sweep.rs too")), say: "Sent, in the thread" },
]);

export { wide as "session-reply-selection" };
