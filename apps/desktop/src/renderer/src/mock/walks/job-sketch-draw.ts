// The owner draws on top of a Drone's sketch with the pad's own pen, and Undo takes the last line back.

import { role, walk } from "../walk";

const w = walk("job-sketch-judge", [
  { look: role("button", "Draw"), say: "The pad's pen, on the sketch's rail" },
  { press: role("button", "Draw"), say: "Pick it up and drag on the sketch" },
  { look: role("button", "Undo"), say: "Undo takes the last line back" },
]);

export { w as "job-sketch-draw" };
