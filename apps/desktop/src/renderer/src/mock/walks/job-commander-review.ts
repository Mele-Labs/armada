// A Job at its review gate. What the lead held, the review's Approve and Send back, is an ask drawn as the
// Cockpit's call cards are. While the work awaits review it takes the board's full width under the Commander,
// and the canvas and the Now panel keep their row below it.
// Over `job-commander-review`.

import { button, inside, region, role, walk } from "../walk";

const now = region("Now");
const review = role("region", /^Review:/);
const commander = role("region", "Commander", { exact: true });

const w = walk("job-commander-review", [
  { look: commander, say: "The Commander leads, over the canvas" },
  { look: review, say: "The review takes the full width under it, in the call card's frame" },
  { look: inside(review, button(/^Approve/)), say: "Approve the work, where the lead had it" },
  { look: inside(review, button(/Request changes|Send back/)), say: "And sending it back with a note" },
  { look: now, say: "The canvas and the Now panel keep their row below" },
]);

export { w as "job-commander-review" };
