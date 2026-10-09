// One Drone at work: its live mark, its step and its last action, under a header for the kind of row.

import { region, inside, role, walk } from "../walk";

const now = region("Now");

const w = walk("job-now-drone", [
  { look: inside(now, role("heading", "Running")), say: "Running, with a header for the kind of row" },
  { look: inside(now, role("img", "Drone")), say: "The row's kind is its icon" },
  { look: inside(now, role("img", "Running")), say: "The mark breathes, and its tooltip names it" },
  { press: inside(now, role("button", /Open Implement Drone/)), say: "A press opens the Drone's panel" },
]);

export { w as "job-now-drone-running" };
