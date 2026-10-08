// A picked option keeps its sketch shown, and picking another replaces it.

import { inside, region, role, walk } from "../walk";

const now = region("Now");

const w = walk("job-sketch-plan", [
  { press: inside(now, role("radio", "Wrap it in place")), say: "Pick an option" },
  { look: role("img", "Sketch"), say: "Its sketch stays after the pointer leaves" },
  { press: inside(now, role("radio", "Split it out")), say: "Pick another" },
  { look: role("img", "Sketch"), say: "and the sketch follows" },
]);

export { w as "job-sketch-select" };
