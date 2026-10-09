// Add a box puts one of his on a Drone's sketch, in his colour, with a field for its words.

import { role, walk } from "../walk";

const w = walk("job-sketch-plan", [
  { press: role("button", "Add a box", { exact: true }), say: "Add a box" },
  { type: "Flush here", into: role("textbox", "The words in your box"), say: "Say what it is" },
  { look: role("group", "Your box: Flush here"), say: "His box, in his colour, beside the Drone's" },
]);

export { w as "job-sketch-add" };
