// Arrows pulse along their direction; Play the steps lights the nodes in the order the Drone gave them.

import { role, walk } from "../walk";

const w = walk("job-sketch-judge", [
  { look: role("group", "Sketch", { exact: true }), say: "Arrows pulse from where they start to where they end" },
  { press: role("button", "Play the steps"), say: "Play the steps" },
  { look: role("button", "Stop the steps"), say: "Each node takes the light in turn, the rest stand back" },
]);

export { w as "job-sketch-flow" };
