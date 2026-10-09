// Zoom in, zoom out and fit sit on the sketch's rail; the wheel, a drag and the keys move it too.

import { role, walk } from "../walk";

const w = walk("job-sketch-judge", [
  { press: role("button", "Zoom in"), say: "Zoom in" },
  { press: role("button", "Zoom in"), say: "and again" },
  { press: role("button", "Fit"), say: "Fit brings the whole sketch back" },
]);

export { w as "job-sketch-zoom" };
