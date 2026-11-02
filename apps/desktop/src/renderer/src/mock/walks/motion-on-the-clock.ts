// A drone's loop and the console's lamps, moving again on the loop clock: 5 moves a second, sharing its frames, and still after a minute with no input.
// Over the `one-session-view` scenario, which has a working Job.

import { region, role, walk } from "../walk";

const motionOnTheClock = walk("one-session-view", [
  { look: region("Flight controls"), say: "The console's lamps move again, 5 times a second" },
  { press: role("button", /^Rotate the audit log/), say: "A working Job" },
  { look: region("Jobs", { exact: true }), say: "Its drone moves on the Field under this list, on the same 5 moves. Is that smooth enough to keep?" },
]);

export { motionOnTheClock as "motion-on-the-clock" };
