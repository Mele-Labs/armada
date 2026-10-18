// Replying to words in an earlier message of a live Session. Selecting them raises the small bar;
// Reply puts the words in the Job's own box as a quote, behind anything already typed, and the box
// takes focus with the caret under them. The box draws what the markdown says: the quote as a block,
// `- ` lines as bullets, an address and a path as links. What is typed there goes to the Job as one message.
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
  { look: message, say: "The words stand as a quote block" },
  { type: "Pin the clock in the store, then check:\n- the store clock\n- the sweep timer", into: message, say: "A reply line and two bullets, typed under the quote" },
  { look: message, say: "Bullets are drawn as typed" },
  { type: "See https://example.com/clock and crates/fleet/src/servers.rs:60 first.", into: message, say: "An address and a path, typed under the bullets" },
  { look: inside(message, text("https://example.com/clock")), say: "The address and the path are drawn as links" },
  { key: "Enter", on: message, say: "Enter sends" },
  { look: inside(thread, text("Pin the clock in the store, then check:")), say: "Sent, in the thread" },
]);

export { wide as "session-reply-selection" };
