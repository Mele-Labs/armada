// One Drone running: its live output is open without a press, and a press folds it.

import { button, inside, region, text, walk } from "../walk";

const now = region("Now");

const w = walk("job-now-drone", [
  { look: inside(now, text(/Read crates\/store\/src\/writer.rs/)), say: "One Drone, so its live output is already open" },
  { press: inside(now, button("Output of Implement Drone")), say: "A press folds it" },
]);

export { w as "job-now-drone-live" };
