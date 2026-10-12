// A Job at its review gate. What the lead held, the review's Approve and Send back, is an ask in the Now
// panel, drawn as the Cockpit's call cards are, and the Commander has the lead's place over the canvas.
// Over `job-commander-review`.

import { button, inside, region, role, walk } from "../walk";

const now = region("Now");
const commander = role("region", "Commander", { exact: true });

const w = walk("job-commander-review", [
  { look: commander, say: "The Commander leads, over the canvas" },
  { look: inside(now, role("region", /^Review:/)), say: "The review gate is an ask in Now, in the call card's frame" },
  { look: inside(now, button(/^Approve/)), say: "Approve the work, where the lead had it" },
  { look: inside(now, button(/Request changes|Send back/)), say: "And sending it back with a note" },
]);

export { w as "job-commander-review" };
