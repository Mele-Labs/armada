// The owner asks a Job's Commander why T7's Drone stopped, reads the answer in the same thread, and
// tells it to fix it. Over `job-commander-asked`, where the Drone sits stuck and Fleet has woken nothing.

import { button, inside, region, role, text, walk } from "../walk";

const commander = role("region", "Commander", { exact: true });
const now = region("Now");
const box = role("textbox", "Tell the Commander");

const w = walk("job-commander-asked", [
  { look: inside(now, text("No output for 14m")), say: "T7's Drone is stuck" },
  { type: "why did T7's Drone stop?", into: box, say: "The owner asks the Commander" },
  { press: inside(commander, button("Send", { exact: true })), say: "Send" },
  { look: inside(commander, text(/turn 288 of 300/)), say: "It reads the Drone and answers in the thread" },
  { type: "fix it", into: box, say: "Then tells it to fix it" },
  { press: inside(commander, button("Send", { exact: true })), say: "Send" },
  { look: inside(commander, text("Turn cap")), say: "The cap raised, as a row" },
  { look: inside(commander, text("Task restarted")), say: "T7 restarted" },
  { look: inside(now, role("img", "Commander")), say: "The Drone on it is marked the Commander's" },
]);

export { w as "job-commander-asked" };
